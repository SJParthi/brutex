# PAUSE 2026-10-06 08:07 UTC — group rest (branch pr74/g18-rest)

Head: **aed23ec** (pushed, clean, no cargo-mutants marker). Earlier pause note: PAUSE-rest.md.

## Done since RESUME (07:14)
- Static gates (all except 1e) PASS at d170a21.
- Non-root (uid 65534) tests of costs/indicators/lake/telemetry/vocab/pull/store at d170a21: 75 binaries, 2476 passed. The only failure is store `cited_commits`, 5 tests; it is environmental (git and target tmpdir not readable by nobody) and passes as root.
- Mutation runs in /home/user/wt-g18 at d170a21, with CI flags in place plus --build-timeout 300:
  - store file.rs remaining survivors: 14 tested, 14 caught.
  - store format.rs Overlay::same_bytes: 3 tested, 3 caught.
  - lake footer.rs, ALL mutants (incl. Cursor::byte Ok(1)): 84 tested, 80 caught, 4 unviable, 0 timeout.
  - pull masters holds_exactly (shard 10 timeout): 5 tested, 5 caught.
- aed23ec: the coordinator's untested store survivors. refuse_if_sealed guard and missing_below `&&` now have tests (D-2086, G18-rest-33/34). The remove_index guard true/false was already covered by G18-rest-22.

## In progress
- `r-mine` (cargo mutants --in-diff of 969493e1..d170a21, crates only) is still running in /home/user/wt-g18. Output: /tmp/claude-0/mut/batch4.out and /tmp/claude-0/mut/r-mine/.

## Next steps after RESUME
1. Read /tmp/claude-0/mut/batch4.out (r-mine line). Kill any missed or timeout.
2. In /home/user/wt-g18: `git checkout -q --detach aed23ec`, then prove the D-2086 kills:
   `cargo mutants -p store --file crates/store/src/file.rs --re 'in remove_index|why.kind\(\) == ErrorKind::NotFound with true in refuse_if_sealed|replace && with \|\| in missing_below' --in-place --baseline run --build-timeout 300 --cap-lints true --cargo-arg=--locked --test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle`
3. Timing p50/p99/max: `cd /tmp/claude-0/tbench-{base,head} && CARGO_TARGET_DIR=/tmp/claude-0/tbench-target-{base,head} cargo run --release` (built; not yet run: UNVERIFIED).
4. Merge newest origin/final/all-fixes (merge commit). Then fmt, clippy -D warnings, tests of touched crates as non-root, static gates; push.
5. Write RESULT-rest.md (table item | fixed? | commit | evidence).

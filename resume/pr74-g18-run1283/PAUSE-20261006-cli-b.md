# PAUSE 2026-10-06 09:23 UTC — cli-b (branch pr74/g18-cli-b)

Head: fd7e3a2 (pushed; tree clean; no cargo-mutants marker in the tree).

## Done
- All 80 survivors and 10 timeouts have a test or a restructure. Decisions D-2020..D-2033; invariants G18-cli-b-01..26.
- Targeted cargo-mutants over the whole list, all with --timeout 300; no timeouts:
  - Run 1, earlier: 29 done (caught or unviable).
  - Run m4: 83 more; 79 caught, 3 unviable (expected: a turn with no Default; a let-chain where && became || does not compile), 1 missed (stored.rs:3590 '|| last').
  - That miss is fixed in fd7e3a2 with a past-the-dated-close test.
- cli lib suite as uid 65534: 1970/0 at 9f4914d. The modules touched since then were re-run green.
- fmt clean. Static gates exit 0 at 9f4914d.
- origin/final/all-fixes is still 969493e, already contained, so there is nothing to merge.

## In progress (detached)
- m5 proves the stored.rs condition block, 9 mutants; log /tmp/g18bin/m5.log. At pause: caught 0, missed 0.
- clippy --workspace --all-targets -D warnings; log /tmp/g18bin/clippy.log. Last line at pause:     Checking api v0.1.0 (/home/user/brutex/crates/api).

## Next on RESUME
1. Check m5: expect 0 missed. Check clippy: expect EXIT 0. Fix anything that fails.
2. Re-run the full cli lib tests as non-root and the static gates (/tmp/g18bin/gates/*.sh, skipping 03) at head.
3. Re-check origin/final/all-fixes and merge if it moved. Push.
4. Write RESULT-cli-b.md (item | fixed? | commit | evidence).

## Update during pause
- clippy --workspace --all-targets --locked -D warnings at fd7e3a2: EXIT 101 (3 error lines). Log /tmp/g18bin/clippy.log.
- m5 last log line: Found 9 mutants to test

- m5 (stored.rs condition block, 9 mutants) finished:
```
MISSED   crates/cli/src/stored.rs:3587:23: replace < with <= in exact_minute_context_from_span in 29s build + 0s test
9 mutants tested in 8m: 2 missed, 7 caught
EXIT 2
crates/cli/src/stored.rs:3590:9: replace || with && in exact_minute_context_from_span
crates/cli/src/stored.rs:3589:9: replace || with && in exact_minute_context_from_span
crates/cli/src/stored.rs:3588:9: replace || with && in exact_minute_context_from_span
crates/cli/src/stored.rs:3587:23: replace < with > in exact_minute_context_from_span
crates/cli/src/stored.rs:3588:23: replace != with == in exact_minute_context_from_span
crates/cli/src/stored.rs:3589:24: replace != with == in exact_minute_context_from_span
crates/cli/src/stored.rs:3590:17: replace != with == in exact_minute_context_from_span
crates/cli/src/stored.rs:3587:23: replace < with == in exact_minute_context_from_span
crates/cli/src/stored.rs:3587:23: replace < with <= in exact_minute_context_from_span

[exited with code 0]
```
- Clippy failed on 2 missing-backticks doc lints in test files (pool_oos_tests.rs:491 `CATALOG_OUT`, selection_v6_tests.rs:631 `AlreadyExists`/`ENAMETOOLONG`). Fix on RESUME, then re-run clippy.

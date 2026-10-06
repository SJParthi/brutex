# PAUSE 2026-10-06 08:07 UTC — cli-b (branch pr74/g18-cli-b)

Head: d85fa9d (pushed; tree clean; `git grep "changed by cargo-mutants"` prints nothing).

## Done
- Every listed survivor (80) and every listed TIMEOUT (10, timeouts-cli-b.md) has a killing test or a restructure. Decisions D-2020..D-2033; invariants G18-cli-b-01..26.
- Earlier targeted runs: 14 CAUGHT, 1 UNVIABLE (readonly_file::regular -> Ok(Default::default())), 0 missed, 0 timeout.
- cli lib suite as uid 65534: 1970 passed / 0 failed at 9f4914d. At d85fa9d only the ordered, admission_v4 and finalization_v4 modules were re-run (25 passed).
- fmt clean. Static gates all exit 0 at 9f4914d (1e skipped).

## In progress (running detached, no usage cost)
- cargo-mutants over the remaining 97 targets (survivors + timeouts + rewritten V4 loops), jobs 2, --timeout 300. Script /tmp/g18bin/run4.sh; output /tmp/g18bin/m4/mutants.out; log /tmp/g18bin/m4.log.
- At pause: caught 13, missed 0, timeout 0, unviable 0.

## Next steps on RESUME
1. Read /tmp/g18bin/m4/mutants.out/{missed,timeout}.txt. Fix anything listed and re-run only those mutants.
2. cargo clippy --workspace --all-targets -- -D warnings (NOT RUN yet).
3. Merge origin/final/all-fixes with a merge commit, keeping both sides at the docs tails. Then fmt, clippy, the cli lib tests as non-root (build the cli lib tests, copy the binary to /tmp/g18bin, run it under setpriv as 65534), and the static gates (/tmp/g18bin/gates/*.sh, skipping 03). Push.
4. Write RESULT-cli-b.md.

## Restart
If the run died: `setsid nohup /tmp/g18bin/run4.sh &`, after `rm -rf /tmp/g18bin/m4`. Add the already-caught names to /tmp/g18bin/ex4.txt first.

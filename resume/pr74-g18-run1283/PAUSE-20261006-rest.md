# PAUSE 2026-10-06 08:28 UTC — group rest (branch pr74/g18-rest)

Head: **445ad02** (pushed, clean, no cargo-mutants marker). origin/final/all-fixes is still 969493e1, so there is nothing to merge.

## Done since the 08:16 RESUME
- D-2086 store kills (remove_index ×2, refuse_if_sealed, missing_below): CI flags plus build-timeout in /home/user/wt-g18 at aed23ec, 5 tested, 5 caught.
- The own-diff run 969493e1..d170a21 found 1 MISSED, `ingest.rs check_day → ()`. Fixed in 445ad02 (check_day returns the emitted level, D-2087, G18-rest-35).
- `cargo clippy --workspace --all-targets -D warnings` clean at aed23ec; `cargo fmt --check` clean; static gates PASS at 445ad02.
- Timing p50/p99/max measured, base against head. The numbers are in the RESULT draft at /tmp/claude-0/RESULT-rest.draft.md.

## In progress
- `r-mine2`: cargo mutants `--in-diff` of 969493e1..445ad02 (crates), in /home/user/wt-g18 at 445ad02, CI flags plus build-timeout. Output in /tmp/claude-0/mut/r-mine2/ and r-mine2.log.

## Next steps after RESUME
1. Read the r-mine2 result: `grep 'mutants tested' /tmp/claude-0/mut/r-mine2.log; cat /tmp/claude-0/mut/r-mine2/mutants.out/{missed,timeout}.txt`. Kill anything listed.
2. Re-fetch origin/final/all-fixes; if it moved, merge, re-validate (fmt, clippy, non-root tests, gates) and push.
3. Fill HEADSHA and MINE2 in /tmp/claude-0/RESULT-rest.draft.md, publish it as resume/pr74-g18-run1283/RESULT-rest.md on fix-queue (merge commit, never force), and send the final table.

## r-mine2 result (finished during pause)
```
exit 0
66 mutants tested in 16m: 37 caught, 29 unviable

[exited with code 0]
```

## PAUSE 09:23 UTC (api untested cases)
Head of pr74/g18-rest: **fea3659** (clean, pushed). RESULT-rest.md published at 70eca74.

In progress, detached so they keep running: the 126 untested api mutants of run 1283 (every `crates/api` line in untested-mutants-run1283.md, which includes the shard-138 cases), split 63+63, in place with CI flags and `--build-timeout 1500`:
- api-a in /home/user/wt-g18 (fea3659): log /tmp/claude-0/mut/api-a.log, results /tmp/claude-0/mut/api-a/mutants.out/
- api-b in /home/user/wt-g18b (fea3659): log /tmp/claude-0/mut/api-b.log, results /tmp/claude-0/mut/api-b/mutants.out/
Name lists: /tmp/claude-0/api-names.txt; regex args in /tmp/claude-0/api-re-{a,b}.txt.

Next after RESUME:
1. `grep 'mutants tested' /tmp/claude-0/mut/api-{a,b}.log; cat /tmp/claude-0/mut/api-{a,b}/mutants.out/{missed,timeout}.txt`.
2. Kill each MISSED/TIMEOUT with a test or restructure (D-3650..3659, then D-3670..3679; rows G18-rest-37 onward). Prove each in place in a wt-g18 worktree, never in /tmp/claude-0 (its root-only permissions break the uid-65534 tests).
3. Add an "api untested cases" section to RESULT-rest.md; validate, then push.
Restart a half: `/tmp/claude-0/mut/api-run.sh /home/user/wt-g18 /tmp/claude-0/api-re-a.txt api-a` (and the b variant with wt-g18b).

## api progress at 11:06Z (2 h wait loop stopped; runs continue detached)
```
api-a: running | caught 7 missed 0 timeout 0 unviable 1
api-b: running | caught 7 missed 0 timeout 0 unviable 0
processes: 4
```

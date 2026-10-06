# PAUSE — api group (pr74/g18-api), 2026-10-06 ~04:50 UTC

Branch `pr74/g18-api`, head **91cbd56** (pushed). It is based on origin/final/all-fixes 969493e1 and does not yet carry a merge of the newest final/all-fixes.

## Done (committed and pushed, validated)
- All 50 survivors from the first list and the 4 late ones (shards 183/184) have a fix in source: tests, or a restructure where the mutant was equivalent.
  - D-2040..D-2046.
  - Invariant rows G18-api-01..27.
- `cargo fmt --check` passes.
- `cargo clippy -p api --all-targets --locked -- -D warnings` is clean.
- api lib tests as non-root (setpriv 65534): 1502 passed, 0 failed.
- api tests/binary.rs passes as non-root.
- Static gates (language-purity job, all steps except 1e) all PASS, including gate 10 after 91cbd56.
- Differential check of the ingest `pad` rewrite against the old code: 279,930 strings, 0 differences (rustc -O scratch program).
- Microbenchmark of copied old/new shapes, ns/op p50/p99/max, taken while a mutation run loaded the box:

  | Shape | Old p50 / p99 / max | New p50 / p99 / max |
  |---|---|---|
  | peak | 2.43 / 4.26 / 369 | 2.44 / 2.61 / 317 |
  | frontier clamp | 2.32 / 2.42 / 186 | 2.24 / 2.87 / 4135 |
  | pad | 26.8 / 35.6 / 4015 | 31.9 / 51.2 / 1253 |

## Mutation proof — PARTIAL (UNVERIFIED beyond this)
- Kills recorded so far:
  - First run (main tree, in-place; since stopped): `audit.rs peak -> 0` and `-> 1`, `booleanevidencejson render_with_budget -> Ok(Default)`, `logs note_request -> Default` all caught. The 3 `frontier`/YearMonth `Default` mutants are unviable.
  - Worktree --re run: `bars.rs:860 > with >= in earlier_in_time` caught.
- Not yet run: the remaining targeted survivors (/tmp list: 51 via --re) and the 62 in-diff mutants.

## In progress: the 3 TIMEOUT cases (timeouts-api.md)
- `calendar_of.rs:213` Landing::drop -> (). Plan: a test that builds `Cache::default()`, inserts one flight, drops a `Landing` and asserts with no wait that `landed` is `Abandoned` and the flight key was removed.
- `server.rs` `Slots::try_take` `<` -> `>`, and `LimitedListener::accept` delete `!`. Bounded tests ALREADY exist: `head_deadline_tests::a_slot_is_taken_below_the_cap_and_refused_at_it` and `accept_takes_a_free_slot_at_once_and_waits_only_when_full`. The timeout comes from an EARLIER-ordered test that serves HTTP with an unbounded wait and hangs under `--test-threads=1`. Next step: find that test.
  - A mutated binary is built at /home/user/nr/mut-trytake. It is box-local and will be gone after the container restarts; rebuild it by applying `(n > self.cap)` at `server.rs:17703`.
  - Run it with `--test-threads=1` and see which test hangs. Bound that wait, or make the catching test run first, then re-prove with CI flags plus `--build-timeout 300`.

## Next steps after RESUME
1. Re-create the worktree if the container restarted: `git worktree add /home/user/wt-mut origin/pr74/g18-api`.
2. Mutation proof in the worktree (never in the main checkout):
   ```
   cargo mutants --baseline skip --in-place --timeout 900 --build-timeout 300 --cap-lints true -p api --test-tool nextest --cargo-arg=--locked --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle --re '<regexes listed in RESULT plan>'
   ```
   Then repeat with `--in-diff <(git diff origin/final/all-fixes HEAD)`.
3. Fix the 3 timeouts as above and prove them.
4. Merge the newest origin/final/all-fixes (merge commit). Run fmt, clippy, api tests as non-root and the static gates, then push.
5. Write RESULT-api.md on fix-queue.

STATUS: IN PROGRESS

# Cloud mutation and coverage pass: store (2026-10-18)

Branch `cloud/mut-store-20261018`, cut from `origin/cloud/mut-store-20261017`
at `e0c7f5da`, with the newest `origin/feat/db-fix` (`67c4e58d`) merged in
(`318a3555`). It continues `reports/cloud-mut-store-20261017.md` (on this
branch, unchanged), whose `## Left` list is this pass's task.

Scope, as that report sets it:
- `crates/store/src/{emits, file, flock, lib, path, safe_open, sealed,
  sealed_codec, sort_index_writer, staging, time_index, verdict}.rs`;
- `crates/telemetry/src/sink.rs`.

## Summary

- **The `## Left` list is done.**
  - Runs B and C were already done on 17 Oct.
  - Every carried survivor was re-tested on the final tree with the full
    suite: **50 tested, 38 caught, 12 missed, 0 timeouts**. The 12 missed
    are exactly the 12 argued equivalents, each argued again against the
    current code.
- **The three 17 Oct timeouts are now caught** under CI's gate flags.
  - Measured: timeout before, caught after.
  - The fix is two tests under a deadline (`24ca118c`).
  - CI's verifier never credits a timeout, so these were real gaps.
- **CI's own plan for this branch** (`--in-diff` against `feat/db-fix`):
  **38 mutants, 22 caught, 16 unviable, 0 missed, 0 timeouts**.
- **Coverage, the final run.** No production line or branch this branch
  adds is uncovered. Every remaining gap in scope is listed with its
  reason, and they are r5's set unchanged.
- **Gates.** fmt clean, clippy clean, deny green, and
  `cargo test -p store --locked` green (810/0) as an ordinary user runs it.
  - Run as plain root, 3 `settings` tests fail, as on the base.
  - `cargo test --workspace --locked`: 9,411 passed. Its 5 failures are
    `api`'s slow-reader tests, red identically on the base (not this pass's
    code).
  - The CI `language-purity` job: 32 of 32 green.
  - **CI gate 20 would be red with this branch** (`sink.rs` 25 against 20,
    `lib.rs` 0 against 6), measured on Linux. Its counts are not this
    pass's files: see NEEDS THE MAC.
- **No production line changed this session.**

## The merge

`feat/db-fix` moved 38 commits past this work's base (`f9cdb584`). Two
conflicts, both appends at the end of a document, both resolved by keeping
both sides whole:
- `docs/04-invariants.md`: CMS-01..CMS-09 beside R4-06's and AG-04's rows;
- `docs/05-decisions.md`: D-6017 beside D-4990 onward. D-6017 is still the
  highest D-number in the ledger.

In the scoped files the merge brought only test changes: `safe_open.rs`'s
test module became `pub(crate)` so `staging_tests.rs` can share
`socket_at` (GF-1). No production line of a scoped file moved, so every
mutant name and line below is the same on both sides of the merge.

## Measured in this session

### Baseline, the merged tree (`318a3555`)

`cargo test -p store --locked`, workspace profile (opt-level 3), as root with
`CAP_DAC_OVERRIDE` and `CAP_DAC_READ_SEARCH` dropped from the bounding set,
`TMPDIR` a sticky world-writable directory so the uid-65534 permission
children can make their scratch:
- 809 passed, 0 failed, 4 ignored, 37 test targets, 81 s, with
  `cited_commits`' history test skipped;
- `cited_commits` alone: 6 passed, that test included. BASE-CITED's fix is
  on `feat/db-fix`, so this test is no longer red here.

The 4 ignored tests read real months or are release-mode measurements (see
NEEDS THE MAC).

### How the mutation runs are made

CI's mutation gate (`ci.yml`, job `mutants`) runs cargo-mutants 26.2.0 with
nextest 0.9.145. Its flags are:
- `--minimum-test-timeout 900 --timeout-multiplier 2 --cap-lints true`;
- `--test-tool nextest`, with `--max-fail=1:immediate --test-threads=1`
  (one test at a time, stopping at the first failure);
- `--no-shuffle`.

Its verifier (`.github/mutation_gate.rs`) never credits a timeout as a
catch. Every mutation run of this session uses those two tools at those
versions and those flags, with three differences:
- `--in-place`, in a git worktree under `/tmp` (mode 755). The container
  runs as root, so the store's `where_permission_binds` tests re-run
  themselves as uid 65534. A cargo-mutants copy sits in a 0700 temporary
  directory, which that child cannot enter, so in a copy those tests fail
  for the setup's reason and not the mutant's.
- `CAP_DAC_OVERRIDE` and `CAP_DAC_READ_SEARCH` dropped, and `TMPDIR` a
  sticky world-writable directory, as for the baseline.
- `-p store` (and `-p telemetry` for `sink.rs`) with `-F` patterns or
  `--in-diff` naming the mutants.

One more difference, and only for the timeouts' before-and-after runs: a
fixed `--timeout 300`. That is three times the 101 s suite, so a timeout
there costs 5 minutes and not 15.

The workspace profile is unchanged (opt-level 3). A sweeper removed test
scratch older than 15 minutes during the mutation runs, so the disk could
not fill as it did on 17 Oct. No store test runs that long: the whole suite
takes 101 s one test at a time.

### The three timeouts of 17 Oct are now caught (`24ca118c`)

The 17 Oct report left three mutants as timeouts, "detected, not
survivors":
- `file.rs:4318:15` `<` → `<=` in `first_at_or_after`;
- `file.rs:4324:63` `/` → `%` in `first_at_or_after`;
- `sealed.rs:2466:21` `<` → `<=` in `SealedMonth::audited`.

CI's gate does not credit a timeout, so these were gaps by the repository's
own rule. Measured this session:

**Every test run alone under each mutant.** Each of the 802 store tests ran
in its own process with a 60 s limit (`cited_commits`' history test aside).
The clean tree passes all 802 this way.

| mutant | pass | fail fast | loop (killed at 60 s) |
|---|---:|---:|---:|
| `first_at_or_after` `<=` | 756 | 9 | 37 |
| `first_at_or_after` `%` | 762 | 15 | 25 |
| `audited` `<=` | 786 | 0 | 16 |

- **`first_at_or_after`, both mutants.** The fast failures include
  `first_at_or_after_never_probes_more_than_the_bisection_height` and
  `locating_a_batch_costs_a_bisection_and_not_a_scan`. The loops are lookups
  and re-offers that bisect a month with no time index: `tests/tix*.rs`,
  `tests/write.rs`, the `file` unit tests and others.
- **`audited` `<=`.** Every audit loops at its end: 15 `sealed_tests` and
  `cloud_mutants_store::a_sealed_months_audit_walks_every_block_once_and_ends`.
  Three of them were already under this branch's one-minute deadline.

**Why each was a timeout under CI's gate.** In nextest's order (one test
at a time, stopping at the first failure), the first test to fail or loop
was:
- for both `first_at_or_after` mutants, this branch's own
  `file::cloud_mutant_tests::an_overlap_that_agrees_with_everything_it_meets_has_no_diagnosis`
  (28th of 799). It loops, before the first fast failure (38th and 39th);
- for `audited`, `sealed_tests::a_file_on_disk_reads_the_same_as_its_bytes`
  (172nd). It loops, and no test fails fast.

**The fix (`24ca118c`), tests only.** Those two tests now run their bodies
on a worker with a deadline:
- the overlap test, 20 s, through `file_fault_tests::within`, now
  `pub(super)`. It keeps the sink held on its own thread;
- the sealed test, one minute, through `within_a_minute`, the helper the
  17 Oct pass gave three of its neighbours.

Clean, they take 0.004 s and 0.017 s.

**Before and after, under CI's gate flags (cargo-mutants 26.2.0, nextest,
`--timeout 300`):**

| mutant | before (`318a3555`) | after (`24ca118c`) |
|---|---|---|
| `first_at_or_after` `<=` | timeout: SIGTERM at 296 s on the overlap test, 33rd of 799 | **caught**: the overlap test fails at its 20.2 s deadline, 33rd of 799 |
| `first_at_or_after` `%` | timeout: SIGTERM at 296 s, the same test | **caught**: the same test, 20.2 s, 33rd |
| `audited` `<=` | timeout: SIGTERM at 280 s on `a_file_on_disk_reads_the_same_as_its_bytes`, 172nd | **caught**: that test fails at its 60.2 s deadline, 172nd |

Each side's unmutated baseline passed first (105 s and 104 s of tests).

Under plain `cargo test` (libtest, which waits for every test in a binary)
all three stay timeouts, because 36, 24 and 11 tests still loop with no
deadline. CI does not run them that way. A nextest `priority` override in
`.config/nextest.toml` would not depend on the order of test names.
`first_at_or_after_never_probes_more_than_the_bisection_height` would catch
the two lookup mutants at once, and `an_empty_month_is_a_head_alone` the
audit one at its deadline. That file is outside this pass's scope, so
this is a follow-up for its owner.

### What CI's gate will plan for this branch: the `--in-diff` run

The gate mutates only the lines a change touches. The plan here is
`git diff origin/feat/db-fix HEAD -- crates/store/src crates/telemetry/src`,
the production lines this branch adds over the base it merges into. Test
modules are not mutated. Both 26.2.0 and 27.1.0 list the same 38 mutants
(all `file`, `flock`, `safe_open`, `staging`, `verdict`, `sort_index_writer`
and `sink`). Run on `85713b2f` with the gate's flags, the timeout auto-set
to 900 s as in CI, in 48 min:

| | count |
|---|---:|
| mutants | 38 |
| caught | 22 |
| unviable | 16 |
| missed | **0** |
| timeout | **0** |

The 16 unviable mutants each replace a body with `Default::default()` of a
type that has none: `File`, `Metadata`, `Staged`, `Seen`, `ReadRefusal`,
`Flock`, `Sink`, `Published`, `Reindexed`, `Walked`, `Why`, and the generic
`T` of `fault`. The 22 caught are:
- the bodies of `seal_committed`, `initialise`, `barrier_in`, the
  `Positional` seam's `put` and `get` (`Ok(0)`, `Ok(1)`),
  `write_durable`, `remove_tmp`, `Reader::index_handle` (`Ok(None)`),
  `keep_sort_index`, `append_sort_run`, `write_whole`, `read_index_at`,
  `damaged` (`Ok(None)`), `clear_staging`, `walk_sidecar` (`Ok(None)`) and
  `Audit::retire`;
- `staging::create`'s `links != 1` guard, forced true or false, and `!=`
  made `==`.

`24ca118c` changes only test code. Listed again on the final tree, the plan
is the same 38 mutants, line for line.

### Every carried survivor, re-tested on the final tree (`24ca118c`)

**The set: 50 mutants.**
- the 12 that the 17 Oct report left missed or argued equivalent: run B's 8,
  run C's 3, run D's 1;
- its 3 timeouts;
- `Head::check`'s two, which run C missed with two tests skipped;
- the 7 whose 17 Oct logs held the full disk's `os error 28`;
- `Series::feed_byte`'s two;
- every mutant the 16 Oct report argued equivalent, at its current line:
  `open_regular`, `staging::create`, `clear_staging`, `walk_sidecar`,
  `write_durable`, `remove_tmp`, `genesis`, `publish`, `missing_below`,
  `Gathering::drop` and `refuse_link`.

That is 41 mutants. The patterns also matched 9 neighbours, which ran too:
- `first_at_or_after`'s `4325:31`;
- `missing_below`'s `4766:55` and `4767:12`;
- `publish`'s guard forced false and its `==` made `!=` (`3285`);
- `publish`'s four `+=` mutants (`3269:17`, `3287:30`).

**How.** The gate's flags, in place on `24ca118c`, with nothing skipped,
`cited_commits` included (in a worktree its history check runs):
- the timeout auto-set to 900 s;
- each unmutated baseline passed first (166 s and 170 s of tests).

The container restarted at 08:45 UTC during leg 1, after 9 of the 50, and
left one mutant written into the worktree's `file.rs`. It was reverted
(`git checkout`) and leg 2 tested the other 41.

| | leg 1 | leg 2 | total |
|---|---:|---:|---:|
| tested | 9 | 41 | **50** |
| caught | 6 | 32 | **38** |
| missed | 3 | 9 | **12** |
| timeout | 0 | 0 | **0** |
| unviable | 0 | 0 | 0 |

**The 12 missed are exactly the 12 argued equivalents** (next section):
- `MAX_ROW_LEN` ×3;
- `O_SYNC.is_none()` → `false`;
- `flush_all` `<` → `<=`;
- `writer_flags` ×2;
- `Code::worse`;
- `Fold::push`;
- `last_before` ×2;
- `mark_coverage`.

So no test in the full suite tells any of them apart.

**The 38 caught, and the test that caught each.** Every one fails in
0.10 to 0.24 s, except the three timeout mutants, which fail at their
deadline as shown above.
- `first_at_or_after` ×3: the agreeing-overlap test (20 s; `4325:31` in
  0.16 s).
- `missing_below` ×3: `the_missing_directories_are_strictly_below_the_root`.
- `Gathering::drop` ×2:
  `a_gathering_dropped_unfinished_restores_the_state_before_it`.
- `refuse_link` ×2: `refuse_link_refuses_a_link_and_passes_everything_else`.
- `open_regular` guard:
  `an_absent_name_is_refused_without_a_look_and_a_link_with_one`.
- `Head::check` ×2: `a_full_disk_at_any_byte_is_…` and
  `a_misfiled_month_is_refused`.
- `frames`: `a_device_error_is_named_with_what_was_being_read`.
- `audited` `<=`: `a_file_on_disk_reads_the_same_as_its_bytes` (60 s).
- `clear_staging`, `walk_sidecar`:
  `a_refused_look_at_the_sort_index_or_its_staging_name_refuses_the_door`.
- `staging::create` ×2: `a_look_the_host_refuses_is_a_stat_refusal_…` and
  `a_second_create_the_host_refuses_is_a_create_refusal_…`.
- `feed_byte` ×2: `a_series_feed_byte_is_its_vendors_own`.
- The verdict writer's 17:
  - `write_durable` ×3: `a_refused_writes_temporary_gone_or_held_is_said_as_it_is`;
  - `remove_tmp` → `Ok(())`:
    `a_day_at_the_last_generation_is_refused_never_wrapped`;
  - `remove_tmp`'s guard ×3:
    `a_linked_temporary_gone_is_done_and_one_held_refuses_as_a_remove`;
  - `genesis` ×3: `a_year_index_linked_first_is_read_and_any_other_refused_link_is_said`;
  - `publish`'s `3269:17` ×2: `a_hard_link_at_a_staging_name_is_refused_…`
    (`tests/verdict.rs`);
  - `publish`'s guard forced true:
    `an_image_link_refused_otherwise_than_exists_is_never_stepped_past`;
  - `publish`'s guard forced false, its `==`, and `3287:30` ×2:
    `a_crash_between_link_and_commit_never_overwrites_an_image_unit`.

The 7 full-disk suspects of 17 Oct (`verdict.rs` 3081:5, 3084:21 ×2,
3084:32, 3108:21 ×2, and `sealed.rs` 2156:9) are all among the caught.

The 16 Oct pass argued 34 mutants equivalent. 22 of them are now caught by
the 17 Oct pass's seams and tests. **The other 12 are the 12 missed above.
No other mutant in scope needs an argument.**

### Coverage, the final run (on `24ca118c`)

**How.** `cargo llvm-cov` 0.8.4's own wrapper, with `--branch`
(`RUSTC_BOOTSTRAP=1`, since `-Zcoverage-options=branch` is unstable),
builds the store and telemetry test binaries in the workspace profile.
- Each of the 39 binaries runs alone from its package root, with the
  capabilities dropped and its profiles in its own world-writable folder,
  so the uid-65534 children's profiles are kept too.
- Each binary's profiles are merged on their own, and `llvm-cov export
  -format=lcov` reads them against that binary.
- A line counts as covered when any binary ran it, and a branch arm when
  any binary took it. This is the per-binary union the 16 and 17 Oct passes
  used: one merged profile keeps one copy of a function built differently in
  two binaries, and undercounts.

All 39 binaries passed (`cited_commits`' history test skipped, as in every
run).

| file | lines | missed | branches | missed |
|---|---:|---:|---:|---:|
| `store/src/emits.rs` | 285 | 6 | 2 | 0 |
| `store/src/file.rs` | 4,440 | 64 | 358 | 20 |
| `store/src/flock.rs` | 75 | 0 | 8 | 0 |
| `store/src/path.rs` | 206 | 0 | 30 | 0 |
| `store/src/safe_open.rs` | 177 | 8 | 22 | 2 |
| `store/src/sealed.rs` | 1,434 | 0 | 274 | 4 |
| `store/src/sealed_codec.rs` | 646 | 0 | 90 | 0 |
| `store/src/sort_index_writer.rs` | 388 | 0 | 62 | 0 |
| `store/src/staging.rs` | 109 | 0 | 14 | 0 |
| `store/src/time_index.rs` | 744 | 0 | 80 | 9 |
| `store/src/verdict.rs` | 2,685 | 6 | 310 | 14 |
| `telemetry/src/sink.rs` | 4,463 | 25 | 340 | 20 |

This is the 17 Oct r5 table, line for line, with one difference: `file.rs`
has three more instrumented lines, `24ca118c`'s wrapper, all covered. `lib.rs`
has no instrumented line. Every uncovered line and branch is where r5 put
it, except one test-code branch, which `24ca118c` moved from 10095 to 10107.

Each one was looked at again in the code at this commit. The production
code ends where each file's first `#[cfg(test)]` begins: `file.rs` 5990,
`verdict.rs` 4178, `sink.rs` 3511.

**Uncovered production lines, each unreachable:**
- `file.rs` 3349-3350, 3363-3364, 3467-3468, 3546-3547, 3557-3558,
  3574-3575, 3577-3578: `let Some(tix) = self.tix.as_ref() else`, a
  re-borrow after the caller read the same `Some`.
- `file.rs` 3375-3385 and 3493-3507: `time_index::extend` refusing
  `OffGrid` or `Outside`. Admission refused those bars first, on the same
  grid and month bounds. `Err(other)` cannot come, because the stamps
  iterator never errs.
- `file.rs` 3488-3491: `SharedSlot` at the threshold. An intraday slot holds
  one bar, and a daily month never reaches 1,024 bars.
- `file.rs` 2582, 2588, 3796, 3806-3808: a block range refused when it was
  computed from the same layout, and a buffer that a `const` assertion
  sizes.
- `file.rs` 3941-3942: the overlap walk past `held_through`.
- `file.rs` 3993-3994: an empty month at `suffix_that_follows`, which
  admission refuses first.
- `file.rs` 4424-4427: `index_refused`'s non-`Unreadable` arm, which only
  the arms above reach.
- `safe_open.rs` 104/107: a file type Linux does not have.
- `safe_open.rs` 299-306: the arm for targets without verified open flags.
- `verdict.rs` 716-717: a vendor table that disagrees with its declaration.
- `verdict.rs` 3923-3924, 4075-4076, 4136-4137: `handle` answering `None`
  right after `prefix` cached it.
- `sink.rs` 3410/3413: a posting path with no parent.

**Uncovered production branches with no uncovered line:**
- `file.rs` 3256: `set_tix_state` on a handle with no `tix`.
- `file.rs` 3679: the sort cache's second key. Every commit sets the cached
  block to `NO_BLOCK`, so a changed count always comes with a changed block.
- `file.rs` 5154: `O_SYNC.is_none()`, true only on a target without one.
- `file.rs` 5248: the parent walk ending by `None`. It ends at the root's
  parent, which `starts_with(root)` refuses.
- `sealed.rs` 1499, 1525: `get_mut(5)` on arrays of six.
- `sealed.rs` 1504: `Fold::push`'s table-length guard (see its equivalent
  mutant below).
- `sealed.rs` 2352: `block > 0` with block 0, which the arm cannot see (see
  its equivalent mutant below).
- `verdict.rs` 2278, 2312, 2478, 3417: `get_mut` on tables sized by the
  value that indexes them.
- `verdict.rs` 3143: `make_dirs`' walk ending by `None`. It breaks at the
  series root first.
- `verdict.rs` 3853: the eviction's `min_by_key` on a full cache, which is
  never empty.

**Test code**, run only when a test fails:
- `file.rs` 6220 to 9776 and branches 6367 to 10107;
- `verdict.rs` 4219, 4227, 4637, 4974;
- `time_index.rs` 921, 1057, 1058, 1105;
- `emits.rs` 788-792 and 801;
- `sink.rs` 3604-3607, 3995, 4032-4035, 4365, 4584, 4968, 6552, 6882, 6941
  and 7792, the doubles' methods at 3706-3716, and the branches after 3511.

These are `panic!` arms of helpers, `assert!` messages and short-circuits,
deadlines that pass only when a body never returns, and doubles' methods a
test does not call.

**Intersected with this branch's own lines** (every line it adds over
`feat/db-fix`, 4,367 of them), the uncovered set is test code alone:
- in the scoped files: `verdict.rs` 4219, 4227, 4637, 4974, `file.rs`
  branch 10107 and `sink.rs` branch 6934;
- in the branch's test files:
  - `file_fault_tests.rs` lines 320, 657, 696, 869;
  - `file_fault_tests.rs` branches 315, 334, 650-661, 755, 845, 879, 912,
    949;
  - `verdict_tests.rs` 568, 813, 879, 950-951;
  - `safe_open_tests.rs` 459;
  - `staging_tests.rs` 133, 196, 228.

Each of those is a `panic!` arm, an `assert!` short-circuit, or a match
guard that is false only when the test fails. **No production line or branch
this branch added is uncovered.**

## Equivalent mutants, argued against the code at `24ca118c`

Each mutant below survives the full suite on the final tree (see the
re-test above). Each argument was re-derived from the code as it stands,
not carried.

| mutant | why no test can tell it from the original |
|---|---|
| `file.rs:215:32` `>` → `==`, `>` → `>=`; `file.rs:220:15` `>` → `>=` (`MAX_ROW_LEN`) | A `const` over the shipped widths: `RECORD_LEN` 56, `GREEK_LEN` 80, `OVERLAY_LEN` 24. `56 > 80`, `56 == 80` and `56 >= 80` are all false, so `widest` is 80. `80 > 24` and `80 >= 24` are both true, so the answer is 80. A width change that could tell them apart trips `const _: () = assert!(MAX_ROW_LEN == 80)` at compile time. The other operators there pick 56 or 24, and are unviable for the same assertion. |
| `file.rs:5154:42` guard `O_SYNC.is_none()` → `false` (`DurableBatch::defer_file`) | On this target `O_SYNC` is `const Some(..)` (`file.rs:5064`), so `is_none()` is already `false`. Only a build for a host without a verified `O_SYNC` (`file.rs:5076`) tells them apart. The `→ true` mutant is caught. |
| `file.rs:5365:67` `<` → `<=` (`flush_all`) | `at` comes from `next.fetch_add(1)`, so each index is handed to one worker once, and a failure is recorded at most once per index. `at == *earlier` never holds, so `<` and `<=` choose alike. `==` and `>` there are caught. |
| `file.rs:5579:27` guard `through` → `true` / `false` (`writer_flags`) | The arm sits in `#[cfg(not(any(macos, linux or android on x86_64 or aarch64)))]`. It is not compiled on this target, so the mutated text never reaches a binary. |
| `verdict.rs:410:29` `<` → `<=` (`Code::worse`) | `severity` maps the 18 codes to 1..=18, each once (`verdict.rs:378-398`). Equal severities mean the same code, and either answer is that code. |
| `sealed.rs:1504:32` `<` → `<=` (`Fold::push`) | At most one extra time-table entry can be pushed once the table holds `table_rows`. `finish` does `table.resize(self.table_rows, n_blocks)`, which cuts it off, and nothing reads the table before `finish`. |
| `sealed.rs:2352:35` guard `block > 0` → `true`, `2352:41` `>` → `>=` (`last_before`) | The guard is reached only in the `Some(previous)` arm, so `index = skip + at ≥ 1`. `skip` is 0 when `lo` is 0 and 1 otherwise, and `block = lo + at`. With `lo = 0`, `at = index ≥ 1`; with `lo ≥ 1`, `block ≥ 1` already. So `block > 0`, `true` and `block >= 0` are all true there. |
| `sink.rs:3272:32` `>` → `>=` (`Audit::mark_coverage`) | `self.bytes` is a `u64`, so the mutant makes `older` always true. It differs only when `bytes == 0` and the generation folder does not exist. There `older` only chooses between `self.bytes` (0) and `0` as the start of the later dimensions' coverage: 0 either way. |

## Gates, on the final tree

The code is final at `24ca118c`; every later commit changes this report
alone.

| gate | result |
|---|---|
| `cargo fmt --all --check` | **clean** (exit 0) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **clean**: exit 0, no warning, 2 min 31 s |
| `cargo test -p store --locked` | **green: 810 passed, 0 failed, 4 ignored, nothing skipped, 37 targets** (exit 0, 2 min 58 s), with `CAP_DAC_OVERRIDE` and `CAP_DAC_READ_SEARCH` dropped, as an ordinary user runs it. See the root caveat below |
| `cargo deny check` | **green**: `advisories ok, bans ok, licenses ok, sources ok`, exit 0, cargo-deny 0.20.2. `Cargo.toml`, `Cargo.lock` and `deny.toml` are identical to `feat/db-fix` (DENY-1's bumps) |
| CI job `language-purity`: every `run:` step but 1e (it moves `web/` aside) and 14 (it re-measures bounds), run through `.github/checks_all.rs`'s plan as the 17 Oct pass ran them | **32 of 32 green** on `0c1c19e2` (144 s), and 32 of 32 on the base `67c4e58d` (143 s), each in its own worktree. The 8 that were red on 17 Oct, on the base and the branch alike, have been fixed on `feat/db-fix` since |

**The workspace** (CLAUDE.md §9's `cargo test --workspace --locked`, the
17 Oct `## Left` list's "workspace test run"). Run on `0c1c19e2`, whose code
is `24ca118c`'s: uninstrumented, `--no-fail-fast`, nothing skipped, the
capabilities dropped, in 40 min.
- 152 test targets and 13 doc-test runs;
- **9,411 passed, 5 failed, 48 ignored**;
- the one red target is `api --lib`: the 5
  `server::head_deadline_body_tests` below, which fail identically on the
  base `67c4e58d`. Every other target is green, `cli`'s pool test and
  `store`'s `cited_commits` among them.

**Run as plain root**, with every capability, `cargo test -p store --locked`
fails 3 tests, all in `settings::tests`, in code outside this pass:
- `a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`;
- `a_disk_that_refuses_the_write_is_refused_with_nothing_visible`;
- `a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`.

Their own message says why: "this host reads a 000 directory (root?): the
property cannot be shown here". Unlike the store's other permission tests,
they do not run through `where_permission_binds`.

**The base `67c4e58d`, run the same way, fails the same 3**, plus
`staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`.
That is the ext4 inode-reuse failure, which the 17 Oct pass fixed on this
branch. Neither the Mac nor CI runs as root. Not edited here:
`settings.rs` is not this pass's file.

### The workspace, measured for CI gate 20

`cargo llvm-cov --workspace --locked --no-fail-fast` on `24ca118c`, then
`cargo llvm-cov report --text`, then gate 20's own tool
(`.github/gates_jobs.rs gate-20`, its 26 tests green), as the coverage job
does. Linux is the platform CI measures on.

- **Run as root, the profile is short.** The uid-65534 children cannot
  write their profiles into the root-owned target folder. A first reading
  gave `sink.rs` 199 and `lib.rs` 2, every one of the 199 a base line in a
  `where_permission_binds` body.
- **With the children's profiles.** The test binaries of the four crates
  whose tests start such children (store 36, telemetry 3, pull 23, api 4)
  were run again, from the same instrumented build, with their profiles in
  a world-writable folder. Those 117 profiles were added to the workspace's
  and the report taken again.
  - Running again adds only coverage that CI's run as an ordinary user also
    gets.
  - The 25 lines `sink.rs` then leaves uncovered are exactly the 25 of the
    per-binary union above.

| file | declared in `ci.yml` | measured, this branch |
|---|---:|---:|
| `clock.rs` | 1 | 1 |
| `level.rs` | 1 | 1 |
| `lib.rs` | 6 | **0** |
| `record.rs` | 1 | 1 |
| `sink.rs` | 20 | **25** |
| `tail.rs` | 3 | 3 |

**So gate 20 would be red with this branch**: `sink.rs` 25 against 20, and
`lib.rs` 0 against 6.
- The 25 `sink.rs` lines are 3413 (production, a posting path with no
  parent; see Coverage) and test code: 3604, 3606-3607, 3706-3716 (9
  lines), 3995, 4032-4035, 4365, 4584, 4968, 6552, 6882, 6941, 7792.
- None of them is a line this branch adds.
- The declared counts live in `.github/workflows/ci.yml`, and their prose in
  `docs/06-limits.md` §54. Both are outside this pass's files (see NEEDS THE
  MAC).

(The base, measured the same way, is in progress: below when it lands.)

**What else the workspace run showed.** It ran 152 test targets in 142
min, instrumented, beside a mutation run. Two targets failed:
- **`api` lib: 5 tests of `server::head_deadline_body_tests`**, AH-02's from
  `feat/db-fix`:
  - `a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole`;
  - `a_chunked_body_reaches_a_slow_reader_whole`;
  - `a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole`;
  - `pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is`;
  - `slow_readers_at_the_cap_are_served_whole_and_the_next_waits`.

  Run again alone, uninstrumented, on an idle machine: the same 5 fail on
  this branch and **identically on the base `67c4e58d`**. Each is cut after
  about 5.37 MB at about 1.28 s ("cut after 5374637 of 16777216 bytes"), so
  it is deterministic and not load. This branch does not touch
  `crates/api`. Not this pass's code; for AH-02's owner.
- **`cli` lib: `pool::tests::the_pool_verb_prints_its_whole_page_on_a_generated_store`**:
  "candidate detail refused: No such file or directory" after 1,058 s.
  - The cause was this session's own scratch sweeper, which removed
    `TMPDIR` entries older than 15 minutes, so it deleted this one
    unusually long test's store under it.
  - Run alone, uninstrumented: **passes** (227 s).
  - The sweeper's age was then raised to 180 minutes.

## What this session changed

| commit | what |
|---|---|
| `318a3555` | merge of `feat/db-fix` `67c4e58d`: two document conflicts, both sides kept |
| `24ca118c` | **tests only**: the agreeing-overlap test (`file.rs`, this branch's own) and `a_file_on_disk_reads_the_same_as_its_bytes` (`sealed_tests.rs`) run their bodies under a deadline; `file_fault_tests::within` becomes `pub(super)` so the first can share it |
| the rest | this report |

**No production line changed.** The rule "every source change gets its own
test that fails before and passes after" therefore has nothing to bind.
The test change's own before and after is the timeouts table above:
timeout before, caught after, on CI's flags.

No new invariant, so `docs/04-invariants.md` is unchanged past the merge.
No new locked choice, so there is no new D-number. The deadline is the
17 Oct pass's own pattern (D-6017).

## Carried from the 17 Oct report, not re-measured here

- **Mutation runs A to D**, and their legs and counts. They mutated the same
  production code: no production line of a scoped file has changed since,
  and the merge brought only tests. Their survivors and timeouts are all in
  the re-test above, and their catches cannot be lost, because no test was
  removed or weakened.
- **The four real defects found and fixed on 17 Oct** (D-6017), and the
  seams and tests of that pass (CMS-01 to CMS-09).
- **The fork-gate flake fix** in the sink tests.

## NEEDS THE MAC

- **CI gate 20's declared counts** (`.github/workflows/ci.yml`) and
  `docs/06-limits.md` §54. Measured here on Linux, as CI measures: with this
  branch, `sink.rs` 25 (declared 20) and `lib.rs` 0 (declared 6).
  - The comparison against the base is in the section above.
  - Re-measure on the merged head, set the counts, and correct §54's prose
    in the same commit, as the gate's own message asks.
  - These are not this pass's files.
- **The 4 ignored store tests** need real data or a release build, so
  they did not run here:
  - `the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving`
    (real months, `BRUTEX_SEAL_SAMPLES`);
  - `real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form`
    (`BRUTEX_SEALED_ROOT`, `_MONTHS`, `_OUT`);
  - `small_month_rule_latency` and `time_and_row_lookup_latency`, which are
    release-mode measurements.
- **The macOS rows.** This container is Linux on ext4, run as root with the
  capabilities dropped.
  - The `cfg(target_os = "macos")` arms of `file.rs`, `safe_open.rs` and
    `open_flags.rs` are other targets' arms here, so they are neither covered
    nor mutated on this host. Among them are `writer_flags`, an equivalent
    here.
  - GF-1's `macos` CI job runs the suite there.

## Follow-ups for other owners

- **`.config/nextest.toml`** (not this pass's file). A `priority` override
  would make the three former timeout mutants' catches independent of the
  order of test names:
  - `first_at_or_after_never_probes_more_than_the_bisection_height` and
    `an_empty_month_is_a_head_alone`;
  - the same shape as D-0693's entries.

  Today they are caught because the first test each meets in the gate's
  order is under a deadline (`24ca118c`).
- **`api`'s five `head_deadline_body_tests`** fail on Linux (this
  container) on `feat/db-fix` itself. AH-02's owner should know before CI
  runs them on `ubuntu-24.04`.
- **`settings_tests`' three root-only failures** could run through
  `where_permission_binds` as the store's other permission tests do. Then
  they would hold as root too, where they now fail on their own premise.

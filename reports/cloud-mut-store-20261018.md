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
- 809 passed, 0 failed, 4 ignored, 37 test targets, 81 s;
- `cited_commits` alone: 6 passed. BASE-CITED's fix is on `feat/db-fix`,
  so this test is no longer red here.

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

The workspace profile is unchanged (opt-level 3). A sweeper removes test
scratch older than 15 minutes, so the disk cannot fill as it did on
17 Oct.

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
test does not call. **No production line or branch this branch added is
uncovered.**

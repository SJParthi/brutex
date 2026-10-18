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
| `first_at_or_after` `<=` | (below) | **caught**: the overlap test fails at its 20.2 s deadline, 33rd of 799 |
| `first_at_or_after` `%` | (below) | **caught**: the same test, 20.2 s, 33rd |
| `audited` `<=` | (below) | **caught**: `a_file_on_disk_reads_the_same_as_its_bytes` fails at its 60.2 s deadline, 172nd |

Under plain `cargo test` (libtest, which waits for every test in a binary)
all three stay timeouts, because 36, 24 and 11 tests still loop with no
deadline. CI does not run them that way. A nextest `priority` override in
`.config/nextest.toml` would not depend on the order of test names.
`first_at_or_after_never_probes_more_than_the_bisection_height` would catch
the two lookup mutants at once, and `an_empty_month_is_a_head_alone` the
audit one at its deadline. That file is outside this pass's scope, so
this is a follow-up for its owner.

(More to follow: the full-suite re-test of every survivor, the CI-shaped
`--in-diff` run, coverage and gates.)

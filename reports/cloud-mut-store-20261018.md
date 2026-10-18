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

(More to follow: the timeout mutants, the full-suite re-test of every
survivor, the CI-shaped `--in-diff` run, coverage and gates.)

STATUS: IN PROGRESS

# R3-17 — one upkeep runner (cloud build report)

Branch `cloud/db-R3-17-20261017`, cut from `origin/feat/db-fix` at `50093fa3`; the newest
`origin/feat/db-fix`, `f9cdb584` (`Merge R3-14: …`), merged in on 17 Oct 2026 once R3-06 landed.

## Prerequisites

All three merged into `origin/feat/db-fix` (re-checked 17 Oct 2026 after the operator's CONTINUE):

```
$ git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-06|R3-16|R3-36):"
Merge R3-06: Month summary kept beside each month (time-of-day index, extremes, day counts, gap result), and the gap audit reads it
Merge R3-36: Store steps as one checked list: every pending run with its command, its prerequisite and its proof
Merge R3-16: Shared safe file helpers: one way to create a temporary file, one way to open a file for reading
```

The first push of this branch (`406bf6d0`, `986106ee`) reported the part blocked on R3-06 and
recorded the operator's choice to wait for it; nothing was built then.

## Owned files, as the tree spells them

The spec's names drifted from the tree's. R3-17 takes, as the operator's defaults say:

| Spec name | File in the tree |
|---|---|
| `crates/cli/src/srt_build.rs`, `_tests.rs` | `crates/cli/src/srt_builder.rs`, `srt_builder_tests.rs` |
| `crates/cli/src/tix_rebuild.rs`, `_tests.rs` | `crates/cli/src/tix_rebuilder.rs`, `tix_rebuilder_tests.rs` |
| `crates/cli/src/summary_build.rs` | `crates/cli/src/msum_builder.rs` (R3-06 named it after the spec was written) |
| `crates/cli/src/census_recount.rs`, `_tests.rs` | same |
| `crates/cli/src/gdfl_rekey.rs`, `_tests.rs` | same |
| `crates/pull/src/upkeep.rs`, `_tests.rs`, `crates/cli/src/upkeep_command.rs` | new |

## Done

- Prerequisites confirmed; newest `feat/db-fix` merged in.
- Gate tools installed in the cloud: cargo-llvm-cov 0.9.1, cargo-deny 0.20.2, cargo-mutants
  27.1.0 (prebuilt release binaries).
- Survey of the builders, the store's per-month lock and the census and re-key doors.

## Left

Everything of the work package: `pull::upkeep`, `cli upkeep`, the four named tests, DB-027 and
DB-N14, docs entries, every gate.

## Gate results

Not run yet.

## NEEDS THE MAC

To be filled as the build goes; every real-data measurement in the goal is a Mac check.

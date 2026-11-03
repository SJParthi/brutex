STATUS: IN PROGRESS

# BASE-COV-LIBS-LS1 (crate `store`), 3 Nov 2026 cloud wave

TOKEN `BASE-COV-LIBS-LS1-20261103-1`; branch `cloud/cov-libs-ls1-20261103`, cut from
`origin/cloud/base-kit-libs-20261103` (`eeec369be7dc`, START `bd04fb299d23`).

## Host

Linux x86_64, root, 4 cores, 15 GiB memory, 29 GiB free disk. Stable `rustc 1.97.1
(8bab26f4f 2026-07-14)` (the repository's `rust-toolchain.toml` pin). Pinned nightly installed
as `nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13
2026-08-07)`; `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
`TMPDIR=/dev/shm/ls1tmp` (tmpfs, mode 1777 so a child test re-run as `nobody` can write).
Kit scripts checked against `MANIFEST.md`: `d3-route-b-cloud.sh` f032b525..., `d3-covset.mjs`
ecef3bdc..., `d3-route-b.sh` f3d70ac2..., `replay-minus-reports.sh` f223185d... (all match).

## Step 1: `cargo test -p store --lib --locked --no-fail-fast`

- On START (as root, `TMPDIR` mode 0755): compiles with no warning; 466 passed, 9 failed,
  2 ignored. Two of the nine re-run themselves as `nobody` through
  `crate::support::where_permission_binds` and failed only because the child could not write
  `TMPDIR` (my host setup); with `TMPDIR` mode 1777 they pass.
- The other seven fail only as root. Proof: each passes run alone as uid 65534 with
  `setpriv --reuid=65534 --regid=65534 --clear-groups <store lib test binary> --exact <name>`
  (`test result: ok. 1 passed`):
  - attempt 2-4 tests: `catalog::cover_tests::a_bars_root_that_cannot_be_listed_is_refused_not_empty`,
    `quarantine::cover_tests::an_index_the_host_will_not_open_is_refused_with_the_hosts_kind`;
  - base tests (present at `f01d7a0e4`):
    `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`,
    `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`,
    `settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
    `settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
    `settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`.
- Fix (test code only, the crate's own D-0995 pattern): each of the seven now calls
  `crate::support::where_permission_binds("<harness name>", <name>_body)`, which re-runs the
  unchanged body as `nobody` when the process is root and in-process-uid otherwise (the Mac and
  CI run it as before). After: 475 passed, 0 failed, 2 ignored (as root).

## NEXT

Integration tests of `store`; Route B on the pinned nightly for `store` (whole crate); gaps and
(F)/(U) lists; close GAP rows.

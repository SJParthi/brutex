STATUS: IN PROGRESS
GREEN-TIP: 2e7a0b6d3b568190024e2e9b405e803805f3fdca

# report-BASE-COV-PULL-PS1 (TOKEN BASE-COV-PULL-PS1-20261103-1)

Session BASE-COV-PULL-PS1 of the 3 Nov 2026 cloud wave. Branch `cloud/cov-pull-ps1-20261103`, cut from
`origin/cloud/base-kit-pull-20261103` (kit commit `5789f435`, START `f98a899398f6`). Owner file
`reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PS1.md` (commit `79a5eb68`).

## Kit check

Every one of the 26 manifest rows matches `MANIFEST.md` (`sha256sum` of the whole file, or of the fenced block
extracted with the manifest's own `awk`), and `MANIFEST.md` hashes to `078a849c...c6479`, the figure in the kit
commit's message. `jq -e .` accepts `verify-BASE-COV-PULL-r2.json`, `fix-BASE-COV-PULL.json` and `packages.json`.

## Host

Linux x86_64 cloud container, uid 0 (root), 4 cores, 15 GiB memory, 28-29 GiB free disk on `/`.
Stable `rustc 1.97.1 (8bab26f4f 2026-07-14)` from `rust-toolchain.toml`. One target dir `/home/user/ps1-tgt`,
`CARGO_PROFILE_DEV_DEBUG=0` (allowed for room, CONTEXT.md), `TMPDIR=/dev/shm/ps1-tmp` (a 16 GiB tmpfs).

**A host note for PS2 and PS3 (and the Mac's cloud kit):** the TMPDIR must be mode 1777 (sticky, world-writable,
like `/tmp`) on a root host. `support::where_permission_binds` (D-0995) re-runs a permission test as uid 65534
when the parent is root, and that child makes its scratch folder under TMPDIR. With my first TMPDIR (mode 0755,
owned by root) 13 more lib tests went red (`a scratch dir: Os { code: 13, kind: PermissionDenied }` inside the
child), every one of them green once the folder was 1777: run 1, 2393 passed / 23 failed; run 2, the same binary,
2406 passed / 10 failed. Route B on a root host has the same trap one level down: the profile folder must be
writable by uid 65534 too, or those children write no profile and their bodies read as uncovered.

## Step (1): repair 2's tests compile, lint and pass

`cargo check -p pull --lib --tests --locked`: no error and no warning (rustc found nothing to fix in repair 2's
2,524 lines). The compile faults were lints:

`cargo clippy -p pull --all-targets --locked -- -D warnings` on START: 3 errors, all in repair 2's new tests:

| file | group | lint | edit |
|---|---|---|---|
| `crates/pull/src/gdfl_dropbox_cov_tests.rs:56` | **outside G1** (G3: tests `gdfl_dropbox.rs`) | `manual_assert_eq` | `assert!(read == secret(..))` is `assert_eq!(read, secret(..))` (`Secret`'s `Debug` is redacted) |
| `crates/pull/src/gdfl_run_start_cov_tests.rs:32` | **outside G1** (G3: tests `gdfl_run_start.rs`) | `single_match_else` | `match std::env::var(CASE).ok()` is `if let Ok(case) = std::env::var(CASE)` |
| `crates/pull/src/ssm_cov_tests.rs:195` | G1 | `single_match_else` | the same |

Commit `91de72cb`. No behaviour changed.

`cargo test -p pull --lib --locked` (with `--no-fail-fast`; one target, so the result is the same): 2406 passed,
10 failed, 48 ignored. All ten fail as root and pass as uid 65534 (proof below). Nine are old tests; the tenth,
`master_snapshots::cov_tests::a_root_that_will_not_list_under_the_lock_is_refused_on_both_counts`, is repair 2's
own and sits in a G1 file: it chmods its root to 0o300, which root lists anyway, so the refusal under test never
ran. Its body now runs through `crate::support::where_permission_binds` (D-0995), as the sink test beside it in
the same file already did (commit `2e7a0b6d`, G1 file `crates/pull/src/master_snapshots_cov_tests.rs`). As root it
now passes, its body run as uid 65534.

### Gates on the GREEN-TIP `2e7a0b6d3b568190024e2e9b405e803805f3fdca`

- `cargo fmt --all --check`: rc 0.
- `cargo clippy -p pull --all-targets --locked -- -D warnings`: rc 0.
- `cargo test -p pull --locked --no-fail-fast`: 32 binaries (the lib, 30 integration test files, the doc tests);
  **2940 passed, 9 failed, 54 ignored**; build (the lib rebuilt, every integration binary new) and run 3 min 59 s wall (opt-level 3, 4 cores). Every integration
  binary and the doc tests are green (doc tests 39 passed; `tests/unit.rs` 163 passed, ...). The 9 failures are
  the lib's, all root-only, listed under NEEDS THE MAC with their proof. 2940 + 9 = 2949 run, 48 more than the
  2901 of c9d653d5f914, which are repair 2's new tests; 54 ignored is L-BCP-3's count.
- The same lib binary run whole as uid 65534 (`runuser -u nobody -- env TMPDIR=/dev/shm/ps1-tmp
  /home/user/ps1-tgt/debug/deps/pull-208c1d9e4a98b91d`, cwd `crates/pull`): **2416 passed, 0 failed, 48 ignored**.

### Files changed in step (1)

`crates/pull/src/gdfl_dropbox_cov_tests.rs` (outside G1), `crates/pull/src/gdfl_run_start_cov_tests.rs`
(outside G1), `crates/pull/src/ssm_cov_tests.rs` (G1), `crates/pull/src/master_snapshots_cov_tests.rs` (G1). All
four are among the 34 files repair 2 changed (`git diff --name-only c9d653d5f914 f98a899398f6 -- crates/pull/src`).

## NEEDS THE MAC

**Host reds (kit CLOUD.md item 8), each a red that appears only as root, each proved by the same test passing as
a non-root user.** Command as root: `cargo test -p pull --lib --locked` on `2e7a0b6d`. Proof: the same lib binary
`/home/user/ps1-tgt/debug/deps/pull-208c1d9e4a98b91d` run as uid 65534 with `--exact <the 9 names>`: `test
result: ok. 9 passed; 0 failed`; and the whole binary as uid 65534: 2416 passed, 0 failed. None of their nine test
files is among the 34 repair 2 changed, so step (1) could not touch them:

1. `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`
   (`alerts_tests.rs:2331`, G2): `left: Ok(()) right: Err(PermissionDenied)`.
2. `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`
   (`gdfl_dropbox_tests.rs:2778`, G3): `left: "not_refused" right: "journal_unwritable"`.
3. `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`
   (`gdfl_dropbox_tests.rs:2866`, G3): `left: "not_refused" right: "store_not_writable"`.
4. `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name` (`gdfl_ia16_tests.rs:2717`, G3):
   `unwrap_err()` on an `Ok` report.
5. `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice` (`gdfl_run_ia2_tests.rs:351`, G3):
   `unwrap_err()` on `Ok(1790812801)`.
6. `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`
   (`census_store_link_tests.rs:359`, G2): the compaction succeeded on a read-only directory.
7. `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`
   (`census_store_link_tests.rs:690`, G2): `a line still to write on a read-only file: true`.
8. `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words` (`zerodha_verify_tests.rs:5105`, G3):
   `unwrap_err()` on an `Ok` spool read.
9. `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named` (`zerodha_verify_tests.rs:3712`, G3):
   the alert write met a non-file where the test made a read-only one.

They are the 3 Nov FNO-LEVERAGE host's kind (9 pull tests red as root, all green as `nobody`). Each could be made
green as root the way D-0995 does it (`where_permission_binds`), by the group that owns its file.

Also for the Mac: Route B whole:pull on its pin, the shadow GET re-check, macOS rows, non-root 14-core tests, the
54 ignored real-data tests (L-BCP-3), the D1 landing.

## Step (2): G1 coverage

In progress (next entries below).

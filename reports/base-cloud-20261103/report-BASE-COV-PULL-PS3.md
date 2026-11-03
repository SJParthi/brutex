STATUS: IN PROGRESS

# report-BASE-COV-PULL-PS3 (TOKEN BASE-COV-PULL-PS3-20261103-1)

Session BASE-COV-PULL-PS3 of the 3 Nov 2026 cloud wave: group G3 of `SHARDS.md` (BASE-COV-PULL, operator decision
D2 part A). Branch `cloud/cov-pull-ps3-20261103`, cut from `2e7a0b6d3b568190024e2e9b405e803805f3fdca` (PS1's
GREEN-TIP, line 2 of PS1's report). Owner file `reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PS3.md` (commit
`5699dd48`). This report is rewritten as the work goes; the last push holds the current state.

## Kit check

Every kit file I use matches `MANIFEST.md` (`sha256sum` of the whole file, or of the fenced block extracted with the
manifest's own `awk`): CLOUD.md, CONTEXT.md, DECISIONS.md, SHARDS.md, repair-BASE-COV-PULL-progress.md,
d3-recipe.txt, d3-route-b-cloud.sh, d3-covset.mjs, deep-BASE-COV-PULL-c9d653d5f914-gaps.txt,
deep-BASE-COV-PULL-f98a899398f6-set.txt, base-cov-pull-fu.txt, verify-BASE-COV-PULL-r2.json, fix-BASE-COV-PULL.json;
`MANIFEST.md` hashes to `078a849c...c6479`; `jq -e .` accepts both json files.

## Host

Linux x86_64 cloud container, uid 0 (root), 4 cores, 15 GiB memory, 27-30 GiB free on `/`. Stable
`rustc 1.97.1 (8bab26f4f 2026-07-14)` from `rust-toolchain.toml` for the gates. The pin for Route B installed as
`nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; its own
`llvm-cov --version` and `llvm-profdata --version` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
`TMPDIR=/dev/shm/ps3-tmp` (tmpfs, mode 1777, PS1's host note); `CARGO_PROFILE_DEV_DEBUG=0` (CONTEXT.md allows it).

## G3's files

The 55 files `SHARDS.md` lists under G3, plus the six test files repair 2 added for G3 production files (a new file
belongs to the group of the file it tests): `gdfl_dropbox_cov_tests.rs`, `gdfl_run_start_cov_tests.rs`,
`http_cov_tests.rs`, `kite_cov_tests.rs`, `masters_cov_tests.rs`, `zerodha_verify_cov_tests.rs`. 61 files, 90,685
lines. (The other four files repair 2 added, `census_store_cov_tests.rs`, `fno_membership_cov_tests.rs`,
`gdfl_rekey_cov_tests.rs`, `manifest_cov_tests.rs`, test G1/G2 files. The benches under `crates/pull/benches/` are
not in SHARDS.md, so G1's by its rule; `cargo test` builds no bench, so no Route B figure reads them.)

## Step 1: six root-only reds in G3's test files (commit `d418a3ec`)

PS1's report lists nine pull lib tests that fail as root and pass as uid 65534. Six are in G3 files. Each body now
runs through `crate::support::where_permission_binds` (D-0995), so as root it runs in a child as uid 65534 and the
refusal under test runs:

- `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`
- `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`
- `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`
- `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice`
- `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`
- `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named` (its doc line now says what its body does: a
  read-only spool folder, not a directory at the log's name)

As root, the six pass (`--exact` the six names: `test result: ok. 6 passed; 0 failed`). The ia16 body run directly as
root (`BRUTEX_PERMISSION_BINDS_CHILD=1`) still fails at `gdfl_ia16_tests.rs:2726`, the `unwrap_err()` PS1 recorded;
run as `nobody` it passes. `cargo fmt --all --check` rc 0; `cargo clippy -p pull --all-targets --locked -- -D
warnings` rc 0 (opt-level 0 target). The other three root-only reds (`alerts_tests.rs`, two in
`census_store_link_tests.rs`) are G2's.

## Step 2: Route B baseline of G3 on the start tip

`d3-route-b-cloud.sh` (kit copy, unchanged) on a detached worktree at `2e7a0b6d`, set = `reading whole:pull` with
one `coverage <file> whole` line for each of G3's 61 files, package `pull`, every test of the crate (no filter):
`D3_W=/home/user/ps3-w D3_DIR=<kit> D3_NIGHTLY=<nightly-2026-08-08> D3_LLVM_BIN=<its llvm-tools> D3_CARGO=<its
cargo> TMPDIR=/dev/shm/ps3-tmp CARGO_PROFILE_DEV_DEBUG=0`. Root host: a watcher made the per-package profile folder
mode 1777 within 50 ms of the script creating it, so `where_permission_binds` children (uid 65534) write their
profiles: 17 profiles written by uid 65534, 55 by root.

- build 16:05:13-16:06:20 IST (1 min; 166 artifacts, 31 test executables); tests 16:06:20-16:10:08 (3 min): 2940
  passed, 9 failed (PS1's nine root-only reds, six fixed in step 1), 54 ignored; shows to 16:16:20. `ROUTE-B DONE
  ... every package measured and read`, exit 0.
- **G3 at `2e7a0b6d`: 61 files, 54 with a gap; lines 49,318/51,591 (2,273 uncovered), branches 3,865/4,792 (927
  uncovered sides).**
- Classified by where each item sits (a node reader of the source, then checked by hand): production code 1,259
  items; test code that is no failing arm 354; failing-arm candidates in tests ((F) to be checked) 739; inside the
  bodies of the 14 `#[ignore]` real-data tests of G3's files (L-BCP-3, the operator's) 848.

## Step 3: closing the gaps (in progress)

Six helpers (subagents of this session) work in detached worktrees of this branch with their own target dirs, one
file family each, under one brief (G3's files only, no git writes, red first, 8 extreme cases a file, root-host and
telemetry notes): zerodha_verify; gdfl_dropbox; ingest and ingest_failkind; tickstore_write and http; the GDFL
family (gdfl_import_lineage, gdfl_import_journal, gdfl_lineage, gdfl_tickstore, gdfl_archive, gdfl_run_scope,
gdfl_run_start); the small files (gap_backfill, rolling, resolve, gaps, archive, session, totp, tenor,
day_universe). I take masters and the test-only files, check each helper's diff against the rules, run its tests,
fmt and clippy here, and commit it. Builds share two slots (flock), about 2.7 GB each.

Landed so far (each commit pushed):

- `c0230d8d` masters: 16 new tests (13 in the new `masters_land_tests.rs`), every refusal of the landing path;
  quick instrumented run (`masters::`) masters.rs lines 1742/1765, branches 181/202 with every item left an (F),
  the hostless-URL (U) or one plant side closed by `plants_for_two_sources_at_once_meet_only_their_own`.
- `9341c620` gdfl_ia16_tests (no test-side gap left but (F)) and gdfl_nfo_attack_tests (oracles tested directly).
- `8e306353` tests/no_scheduler_delete.rs: 318/318 lines, 112/112 branches.
- `e11fc694` tests/derive.rs and tests/pipeline.rs: only (F) left.
- `454a6797` gdfl_dropbox (helper db): gdfl_dropbox.rs lines 1977/2133 -> 2145/2145, branches 370/450 -> 447/448;
  **a real defect fixed**: the staging lock outlived its press in a forked child's copy of the description
  (the press-2 `run_busy` that made `a_duplicate_refused_by_its_sha256_stays_refused_on_every_later_press` red once
  under load); and gdfl_cm_admission_tests' fixture predicate tested directly.

## NEEDS THE MAC

Route B whole:pull on its pin, the shadow GET re-check, macOS rows, non-root 14-core tests, the 54 ignored real-data
tests (L-BCP-3), the D1 landing.

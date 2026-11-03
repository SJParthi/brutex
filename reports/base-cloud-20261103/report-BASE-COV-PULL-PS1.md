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

## Step (2): G1 coverage (interim; the final figures replace this section)

### The instrument on this host

The pinned nightly installs here and every pin matches: `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`
(`nightly-2026-08-08`), `llvm-cov` and `llvm-profdata` `LLVM version 23.1.0-rust-1.99.0-nightly`. Route B is the
kit's `d3-route-b-cloud.sh` unchanged (sha256 `f032b525...`), called with `D3_NIGHTLY`, `D3_LLVM_BIN`, `D3_DIR` (the
kit's `d3-covset.mjs`) and `D3_CARGO` set to a wrapper that runs the nightly cargo under umask 0022, the script's
own folders being made under umask 0000. `CARGO_TARGET_DIR=$K/cov` is a link to the one target dir.

**Two host traps, both of which silently drop coverage on a root host** (for PS2, PS3 and the cloud kit):

1. `where_permission_binds` children run as uid 65534 and write their profiles into the same folder as the
   parent. Under the scratchpad (`/tmp/claude-0`, mode 0700) they could not reach it and wrote nothing: run 1
   (stopped) had 10 root-owned profiles and none of uid 65534. The profile folder must be on a path uid 65534
   can traverse and write (here `/home/user/ps1-prof`, mode 0777).
2. umask 0000 for the whole run (my first fix for 1) made a test's config file 0666 and turned
   `gdfl_dropbox::tests::a_missing_link_is_a_named_wait` red (`dropbox_config_too_open`); hence the wrapper.

### Route B on the GREEN-TIP (run 2, `2e7a0b6d3b56`)

`routeb.sh /home/user/ps1-deep 2e7a0b6d3b56-2` (a worktree at the GREEN-TIP; set = the kit's f98a899398f6 set's
206 coverage lines, every pull file the tool reports, which are this tip's files exactly). Build 1 min, tests 4
min (2939 passed, 10 failed: the 9 host reds and the umask one above, 54 ignored), 73 profiles, read 30 min.
`ROUTE-B DONE ... EXIT 0`.

- **whole:pull: lines 153,352/160,118 (6,766 uncovered), branches 11,561/14,244 (2,683 uncovered sides);**
  173 of 206 files with a gap.
- **G1 (66 files: SHARDS.md's 64 plus repair 2's `fno_membership_cov_tests.rs` and `manifest_cov_tests.rs`):
  lines 51,738/53,838 (2,100 uncovered), branches 3,896/4,830 (934 sides);** 62 files with a gap. The same 64
  files held 2,258 lines + 957 sides at c9d653d5f914 (SHARDS.md).
- G1's 3,034 rows by kind (a node classifier, scratchpad `classify.mjs` + `assertcond.mjs`, read-only): 1,306
  inside the bodies of G1's 17 `#[ignore]` real-data tests (L-BCP-3, the operator's); 639 (F) by pattern (the
  failing side of an assert's condition decided by enumerating every truth assignment under short-circuit
  evaluation, panic lines, message lines, let-else elses); 48 branches inside `matches!` guards and closures
  of assert conditions, checked by hand; **777 production rows and 264 other test-code rows: the work.**

### Done so far (each batch committed and pushed; every new test passes on stable and was shown red)

| batch | commit | G1 files | new tests | rows reached / left |
|---|---|---|---|---|
| 1 | `24b53762` | fetch, nse, capture, request_minutes, pricing | 10 | the drivable rows; the rest (U) |
| 2 | `e944865d` | vendor, gdfl_import causes, gdfl_run stop (+ `tests/gdfl_run_stop.rs`) | 7 | idem |
| 3 | `b16623fc` | manifest, master_snapshots first_listed | 23 | 38/39 and 50/53; 4 (U) |
| 4 | `8d6731f1` | fno_membership (+ the defect below) | 25 | 107/128; 17 (U), 4 seam |
| 5 | `20f67de4` | gdfl_import day loop | 26 | 124/132; 7 (U), 1 seam |
| 6 | `ba28378c` | reference_data, gdfl_run locks | 33 | 66/85 and 60/77; (U) and seams |

Batches 3-6 were drafted by read-only helper agents (no repository write, no cargo; brief in the scratchpad)
and integrated, linted, run and red-checked here. **Red checks** (one production line changed by hand per test,
all applied at once, `git checkout` after): batch 1 10/10 red, each at its own assertion; batch 2 7/7; batch 3
23/23; batch 4 24/24 (plus the defect test, red on the old code); batch 5 26/26; batch 6 33/33.

Left out on purpose: three drafted tests that won a timing race by retrying it (up to 32 or 16 rounds), because
a retried race is not deterministic; their 9 rows (`reference_data.rs:2915, 2933, 2933:21 both, 2934, 2936`,
`gdfl_run_locks.rs:433-435`) are seams. A macOS branch of a flock-refusal fixture that nobody ran: the test is
Linux-only.

### DEFECT FIXED (D-7335 draft below)

`fno_membership::MonthTable` decode checked a day row with no listing by `row.names + row.refused > 0`, two `u32`
fields of the month file. A row holding `names = u32::MAX, refused = 1` overflowed; with overflow checks on in
every profile the decoder panicked on the file's bytes (and the release build, `panic = "abort"`, aborts).
Each count is now asked on its own (`row.names > 0 || row.refused > 0`), the same refusal. Test
`fno_membership::ps1_tests::a_row_with_no_listing_whose_counts_pass_the_field_is_refused_never_a_panic`, red
first: `panicked at crates/pull/src/fno_membership.rs:1461:63: attempt to add with overflow`.

### Follow-ups found (not changed)

- `master_snapshots::first_listed::listing` has no `carried(route)` guard where `absorb_held` has one: a route
  whose exchange holds `|` writes commit lines no reader parses and damages the next generations. No live route
  reaches it (the one route is `ZERODHA_CONTINUOUS`, NFO/FUT); a guard would turn its test into a refusal.
- `fno_membership`'s reader-thread-panic arm (the "refused, named, never absent" promise) only runs in an
  unwinding build: release aborts on the panic before `join` returns.

### Still to do in step (2)

ssm, master_snapshots, gdfl_import, gdfl_cm_auction, cash_session_cache, rehearsal, gdfl_nfo, refusal and five
small files (helpers drafting), the 264 test-code rows (two helpers on 217 of them), seams, and the final Route B
over G1 with the (F)/(U) list.

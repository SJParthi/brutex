# Cloud mutation and coverage pass: store and telemetry sink (2026-10-16)

**STATUS: IN PROGRESS.** This draft is committed as results land, so nothing
is lost to a container restart. Sections marked *pending* are not measured
yet. The final version replaces this line.

Branch `cloud/mut-store-20261016`, from `cloud/base-20261016` (`c9e3472`, a
one-commit snapshot of `feat/db-fix` at `4756cce0`).

Scope (CLAUDE.md §9 target: no surviving mutant, 100% line coverage, branch
coverage where measurable): `crates/store/src/{emits, file, lib, path,
safe_open, sealed, sealed_codec, sort_index_writer, staging, time_index,
verdict}.rs` and `crates/telemetry/src/sink.rs`.

Every new test lives in a new file or a new module: public items in
`crates/store/tests/cloud_mutants_store.rs` and
`crates/telemetry/tests/cloud_mutants_sink.rs`; private items in a
`#[cfg(test)] mod cloud_mutant_tests` appended at the end of `sink.rs` and
`verdict.rs`. No existing test was changed or removed. No production line was
changed.

## How it was measured

- `cargo-mutants 27.1.0`, `cargo-llvm-cov` (branch coverage through
  `RUSTC_BOOTSTRAP=1`, as `--branch` is unstable), Rust 1.97.1.
- **Pass 1** ran on a pristine `git worktree` of the base commit, so the
  "before" numbers are not affected by the tests this session added.
- **The workspace `dev` profile is `opt-level = 3`, `incremental = false`.**
  A mutant's rebuild of `store` took 68 to 121 s under it. Pass 1 overrode it
  through cargo's environment configuration (no tracked file changed):
  `CARGO_PROFILE_DEV_OPT_LEVEL=0`, `CARGO_PROFILE_DEV_INCREMENTAL=true`,
  `CARGO_PROFILE_DEV_DEBUG=0`. A rebuild of the whole store test suite then
  took about 5 s.
- At opt-level 0 one test, `sealed::sealed_tests::a_flipped_bit_anywhere_is_refused_by_name`,
  takes about 500 s (22 s at opt 3, 41 s at opt 1); three others queue behind
  its sink lock. Pass 1 skipped the tests over 5 s at opt 0 and an
  environment-only failure (listed below), and, for the files no other module
  reaches, other modules' unit tests (`--skip <module>::`; integration-test
  names carry no `::`, so no integration test is skipped by it).
- **Pass 2** (*pending*) re-tests every pass-1 survivor on the base tree with
  the full suite that can reach the file, heavy tests included, at
  opt-level 1.
- **After**: survivors re-tested on this branch with `--iterate`.
- The container runs as root. Tests ran with `CAP_DAC_OVERRIDE` and
  `CAP_DAC_READ_SEARCH` dropped (`setpriv --bounding-set`), so mode bits bind
  as they do for an ordinary user; the store's own `where_permission_binds`
  children (uid 65534) ran as designed. Test scratch directories were on
  tmpfs (`TMPDIR=/dev/shm/...`).
- Shards were time-boxed at 25 minutes each.

## Results per file

*Pass 2 and the store "after" columns are pending.*

| file | mutants | pass 1 caught | pass 1 missed | timeout | unviable | after: missed |
|---|---:|---:|---:|---:|---:|---:|
| telemetry `sink.rs` | 433 | 341 | 38 | 9 | 45 | 1 (equivalent) |
| store `staging.rs` + `safe_open.rs` | 67 | 37 | 13 | 0 | 17 | 3 (equivalent) |
| store `sealed_codec.rs` | 272 | 258 | 0 | 1 | 13 | 0 |
| store `time_index.rs` | 135 | 127 | 0 | 2 | 6 | 0 |
| store `sort_index_writer.rs` | 74 | 58 | 6 | 0 | 10 | 2 (equivalent in practice) |
| store `path.rs` | 130 | 118 | 1 | 0 | 11 | 0 |
| store `verdict.rs` | 840 | 714 | 59 | 5 | 62 | pending |
| store `sealed.rs` | 638 | pending | | | | |
| store `file.rs` | 407 | pending | | | | |
| store `emits.rs`, `lib.rs` | 0 | no mutable code | | | | |

`sink.rs` pass 1 ran the whole telemetry suite, so it is also its pass 2.
After the new tests, `--iterate` re-tested its 47 missed or timed-out mutants:
37 caught, 1 missed (the equivalent below), 9 still timeouts. The 9 time out
because the pre-existing tests hang on them; re-run with only the new tests,
all 9 are **caught** (fail fast).

## Equivalent mutants (so far)

| mutant | why no test can kill it |
|---|---|
| `sink.rs:3190:32` `>` → `>=` in `Audit::mark_coverage` (`older = self.bytes > 0 \|\| generation.exists()`) | `older` only chooses `self.bytes` over `0` as the start of the newer dimensions' coverage; when `bytes == 0` both are 0. |
| `safe_open.rs:310:21` NotFound guard → `false` in `open_regular` | `ENOENT` then goes through `name_the_refusal`, whose `lstat` sees the same absence and returns the same `Io { action: "open", source }`. It costs one more `lstat`, and differs only if a name appears between the two calls, which no test can stage. |
| `staging.rs:254:21` NotFound guard → `true` in `create` | Reachable only if `lstat` fails other than `ENOENT` straight after `open` said `EEXIST`: the directory's search permission revoked between two calls. |
| `staging.rs:263:21` AlreadyExists guard → `true` in `create` | Reachable only if the second `O_EXCL` create fails other than `EEXIST` after the unlink succeeded: the directory changed between two calls. |
| `sort_index_writer.rs:651:21` NotFound guard → `true` in `clear_staging`; `sort_index_writer.rs:661:21` the same in `walk_sidecar` | The `.srt` and its `.new.srt` staging name live in the directory of the bar file the door or writer has just opened, so `lstat` there fails other than `ENOENT` only if that directory's permissions change between two calls of one function. |
| `verdict.rs:410:29` `<` → `<=` in `Code::worse` | Severity is injective over the codes, so equal severity means the same code, and either answer is that code. |
| `verdict.rs:839:9` `Series::feed_byte` → `Ok(0)` and → `Ok(1)` | Both callers discard the value (`series.feed_byte()?` in `validate`; the header's feed byte is written from `feed_code` directly); the `Err` arm needs a `Vendor` outside `Vendor::ALL`, which the type cannot hold. |
| `verdict.rs:3016:30` guard → `true` / `false`, `3016:42` `!=` → `==` in `write_durable` | The guard runs only when `remove_file` FAILS after a refused write, on the temporary this call created in the same directory moments before. Failing needs the directory changed, or the file removed, by another actor between two calls of one function. The `cfg(test)` write seam can be armed only from `verdict_tests.rs`, which this session did not edit. |
| `verdict.rs:3080:21` guard → `true` / `false`, `3080:32` `!=` → `==` in `remove_tmp` | The same: every call removes a temporary this writer made, under the commit lock, in a directory it has just written. |
| `verdict.rs:3102:21` guard → `true` / `false`, `3102:32` `==` → `!=` in `genesis` | The link runs under the commit lock straight after `open_existing` found no index, so `EEXIST` needs an actor that does not take the lock to create the index between two calls; any other link error (`EMLINK`, `EPERM`, `ENOSPC`) is a host fault no test-owned substitute produces on the same directory as a file just written. |
| `verdict.rs:3277:25` guard → `true` in `publish` | The same as `genesis`: a link error other than `EEXIST`, under the lock, is a host fault. |

## Findings (not fixed; no production line changed)

- **`staging::create` under a concurrent unlink** refuses `staging_linked`
  with **0** links ("one of 0 names of a file"): the `lstat` caught the inode
  mid-unlink. Safe (nothing removed, truncated or written) but misnamed.
  Observed by `a_leftover_that_vanishes_before_the_look_leaves_the_create_free`.

## Environment failures in the base suite (not caused by this branch)

| test | why it fails here | status |
|---|---|---|
| `store::staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported` | `assert_ne!(ino)` after unlink + create: ext4 reuses the freed inode number (fails 40/40 alone). Passes on tmpfs and APFS. A test defect, not a code defect. | left unchanged; the new race test proves the same branch without inode numbers |
| `store tests/cited_commits.rs every_commit_the_store_records_cite_is_on_main_or_says_why_not` | the cloud base is a one-commit snapshot, so the commits the docs cite are not ancestors | environment only |
| four `telemetry::sink::tests` chmod tests | fail as root (mode bits do not bind); pass with `CAP_DAC_OVERRIDE` dropped. They do not use `where_permission_binds` (D-0995) as the store's do. | environment only |

## Coverage

*Before measured; after pending.* Before (base tree, profiles of the uid-65534
children included):

| file | lines | missed | line % | branches | missed | branch % |
|---|---:|---:|---:|---:|---:|---:|
| `store/src/emits.rs` | 295 | 9 | 96.95 | 2 | 0 | 100.00 |
| `store/src/file.rs` | 4445 | 178 | 96.00 | 346 | 52 | 84.97 |
| `store/src/path.rs` | 207 | 0 | 100.00 | 30 | 0 | 100.00 |
| `store/src/safe_open.rs` | 178 | 11 | 93.82 | 21 | 1 | 95.24 |
| `store/src/sealed.rs` | 1434 | 8 | 99.44 | 272 | 4 | 98.53 |
| `store/src/sealed_codec.rs` | 646 | 0 | 100.00 | 90 | 0 | 100.00 |
| `store/src/sort_index_writer.rs` | 400 | 36 | 91.00 | 60 | 10 | 83.33 |
| `store/src/staging.rs` | 91 | 18 | 80.22 | 12 | 6 | 50.00 |
| `store/src/time_index.rs` | 705 | 6 | 99.15 | 75 | 9 | 88.00 |
| `store/src/verdict.rs` | 2101 | 272 | 87.05 | 298 | 77 | 74.16 |
| `telemetry/src/sink.rs` | 3914 | 86 | 97.80 | 322 | 45 | 86.02 |

`lib.rs` has no instrumented line. These counts include lines inside each
file's own `#[cfg(test)]` modules (an assertion's message arguments run only
when it fails).

**CI gate 20 is an exact-count ratchet** on telemetry's uncovered lines
(`sink.rs 21` in this base's `ci.yml`). Lines this branch newly covers in
`sink.rs` lower that count, and the gate fails a file with FEWER uncovered
lines than declared until the count is lowered. That edit is in
`.github/workflows/ci.yml`, which this session may not touch: **the operator
must re-measure and lower the `sink.rs` count (and the prose in
`docs/06-limits.md` §54) when re-applying this branch.**

## Gates

*Pending.*

## NOT DONE

*Pending; the final version lists everything not reached.*

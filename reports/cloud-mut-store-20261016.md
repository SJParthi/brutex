# Cloud mutation and coverage pass: store and telemetry sink (2026-10-16)

Branch `cloud/mut-store-20261016`, from `cloud/base-20261016` (`c9e3472`, a
one-commit snapshot of `feat/db-fix` at `4756cce0`).

Scope (CLAUDE.md §9 target: no surviving mutant, 100% line coverage, branch
coverage where measurable): `crates/store/src/{emits, file, lib, path,
safe_open, sealed, sealed_codec, sort_index_writer, staging, time_index,
verdict}.rs` and `crates/telemetry/src/sink.rs`.

## Summary

- **2,996 mutants** over the eleven files that have mutable code (`emits.rs`
  and `lib.rs` have none). On the base suite they came out as follows (pass 1
  ran a reduced suite; see "How it was measured"):

  | outcome | count |
  |---|---:|
  | caught | 2,537 |
  | missed | 174 |
  | timeout | 30 |
  | unviable | 255 |

- **After this branch, 34 mutants survive, and every one is argued equivalent
  below.** Of the 204 that survived pass 1 (missed or timed out), 170 are
  killed by the 112 new tests. The 34 equivalents can only be told apart from
  the original code in one of three ways:
  - **through a fault or race between two system calls of one function**,
    which no test-owned stimulus produces;
  - **on another target** (a `cfg` branch not built on Linux x86_64, or a
    constant that differs there);
  - **not at all**, because the program's behaviour is identical.
- **Pass 2** re-ran those 34 on the base tree with the full suite of their
  crate, heavy tests included: *see below*.
- **Production line coverage** rose in every file that had room:

  | file | before | after |
  |---|---:|---:|
  | `verdict.rs` | 89.09% | 99.21% |
  | `staging.rs` | 80.22% | 97.80% |
  | `sort_index_writer.rs` | 93.95% | 97.37% |
  | `file.rs` | 93.64% | 96.87% |
  | `sink.rs` | 97.08% | 99.69% |
  | `time_index.rs` | 99.22% | 100% |

  `path.rs`, `sealed.rs` and `sealed_codec.rs` were already at 100% and stay
  there. Every production line still uncovered is listed with the reason no
  test reaches it. **Line coverage is not 100% in `file.rs`, `verdict.rs`,
  `sort_index_writer.rs`, `safe_open.rs`, `staging.rs` and `sink.rs`, and
  branch coverage is not 100% in nine files.** See NOT DONE.
- **No production line was changed and no real bug was found.** The new tests
  found five defects in the tree, none a code defect in scope: one misnamed
  refusal under a race, two test-tree defects, a doc comment citing a test
  that does not exist, and two CI failures on the base snapshot. See Findings
  and Environment failures.
- **Every new test is in a new file or a new module.**
  - Public items: `crates/store/tests/cloud_mutants_store.rs` (61 tests) and
    `crates/telemetry/tests/cloud_mutants_sink.rs` (12).
  - Private items: a `#[cfg(test)] mod cloud_mutant_tests` appended at the
    end of `sink.rs` (18), `verdict.rs` (16), `file.rs` (1), `sealed.rs` (1),
    `sealed_codec.rs` (1) and `time_index.rs` (2).
  - No existing test was changed, weakened or removed.

## How it was measured

- `cargo-mutants 27.1.0` and `cargo-llvm-cov`, on Rust 1.97.1. Branch
  coverage needs `RUSTC_BOOTSTRAP=1`, because `--branch` is unstable.
- **Pass 1** ran on a pristine `git worktree` of the base commit, so the
  "before" numbers are not affected by the tests this session added. Each
  file was run in shards (`--shard k/n`), each shard time-boxed at 25
  minutes. No shard reached the box.
- **The workspace `dev` profile is `opt-level = 3`, `incremental = false`.**
  - Under it, rebuilding `store` for one mutant took 68 to 121 s.
  - Pass 1 overrode the profile through cargo's environment configuration,
    which changes no tracked file: `CARGO_PROFILE_DEV_OPT_LEVEL=0`,
    `CARGO_PROFILE_DEV_INCREMENTAL=true`, `CARGO_PROFILE_DEV_DEBUG=0`.
  - A rebuild of the whole store test suite then took about 5 s.
- **What pass 1 skipped.** The suite was the one that reaches each file, with
  these tests left out:
  - tests over 5 s at opt-level 0. One of them,
    `sealed::sealed_tests::a_flipped_bit_anywhere_is_refused_by_name`, takes
    about 500 s there (22 s at opt-level 3);
  - the environment-only failure `cited_commits` (see Environment failures);
  - for files no other module reaches, the other modules' unit tests, via
    `--skip <module>::`. Integration-test names carry no `::`, so this skips
    no integration test.

  `sink.rs` ran the whole telemetry suite.
- **After**: each file's pass-1 survivors, all shards merged, were re-tested
  on this branch with `--iterate` against the new tests. A mutant that a
  pre-existing test hangs on still reports TIMEOUT, so those were re-tested
  against their own deadline test alone.
- **Pass 2**: the 34 mutants classed equivalent were re-tested on the base
  tree against the full suite of their crate (`--tests`), heavy tests
  included, at opt-level 1. A catch there would prove the class wrong.
- **Running without root's permission bypass.** The container runs as root.
  Every mutation and coverage run dropped `CAP_DAC_OVERRIDE` and
  `CAP_DAC_READ_SEARCH` (`setpriv --bounding-set`), so mode bits bind as they
  do for an ordinary user. The store's own `where_permission_binds` children
  (uid 65534) ran as designed. Test scratch directories were on tmpfs.
- Every mutant named in "killed" below was also confirmed directly: a
  targeted `cargo mutants --re` run of the mutant against the test that names
  it, or the merged after-run.

## Results per file

| file | mutants | pass 1 caught | missed | timeout | unviable | killed by new tests | left (equivalent) |
|---|---:|---:|---:|---:|---:|---:|---:|
| telemetry `sink.rs` | 433 | 341 | 38 | 9 | 45 | 46 | 1 |
| store `staging.rs` + `safe_open.rs` | 67 | 37 | 13 | 0 | 17 | 10 | 3 |
| store `sealed_codec.rs` | 272 | 258 | 0 | 1 | 13 | 1 | 0 |
| store `time_index.rs` | 135 | 127 | 0 | 2 | 6 | 2 | 0 |
| store `sort_index_writer.rs` | 74 | 58 | 6 | 0 | 10 | 4 | 2 |
| store `path.rs` | 130 | 118 | 1 | 0 | 11 | 1 | 0 |
| store `verdict.rs` | 840 | 714 | 59 | 5 | 62 | 51 | 13 |
| store `sealed.rs` | 638 | 582 | 22 | 10 | 24 | 29 | 3 |
| store `file.rs` | 407 | 302 | 35 | 3 | 67 | 26 | 12 |
| store `emits.rs`, `lib.rs` | 0 | (no mutable code) | | | | | |
| **total** | **2,996** | **2,537** | **174** | **30** | **255** | **170** | **34** |

How each file's "after" was measured:

- **`sink.rs`.** The merged after-run of the 47 survivors gave 37 caught, 1
  missed (equivalent) and 9 timeouts. The 9 timeouts are pre-existing tests
  hanging on them; re-run against the new tests alone, all 9 are caught.
- **`staging.rs` + `safe_open.rs`.** After-run of the 13: 10 caught, 3
  equivalent.
- **`sort_index_writer.rs`.** After-run of the 6: 4 caught, 2 equivalent.
- **`verdict.rs`.** After-run of the 64: 47 caught, 14 missed and 3
  timeouts.
  - Of the 14 missed, 13 are equivalent. The 14th, `2533:28`, was killed by a
    test committed after that run began (confirmed directly).
  - The 3 timeouts are caught by their own deadline tests (`--timeout 60`,
    since a deadline is 20 s).
- **`sealed.rs`.** After-run of the 32: 29 caught, 3 equivalent.
- **`file.rs`.** After-run of the 38: 21 caught, 15 missed and 2 timeouts.
  - Of the 15 missed, 12 are equivalent. The other 3 (`3948`) were killed by
    a unit test of `diagnose_overlap`'s contract, committed after the
    after-run; all 7 `3948` mutants were confirmed caught.
  - The 2 timeouts (`4305`, `4311`) are caught by their deadline test alone.
- **`path.rs`, `sealed_codec.rs`, `time_index.rs`.** After-runs caught all of
  their survivors (1, 1 and 2).

The new test that kills each survivor names that mutant in its doc comment.
The two appendices list all 112 new tests with what they assert.

## Pass 2: the equivalents against the full suite

*Pending: filled in from `store-p2-equiv` and `telemetry-p2-equiv`.*

## Equivalent mutants

Each row is a mutant that no test in this tree can kill, with the reason. "In
practice" means it can be told apart only through a fault or a race between
two system calls of one function, which no test-owned stimulus produces
deterministically.

| mutant | why no test can kill it |
|---|---|
| `sink.rs:3190:32` `>` → `>=` in `Audit::mark_coverage` (`older = self.bytes > 0 \|\| generation.exists()`) | `older` only chooses `self.bytes` over `0` as the start of the newer dimensions' coverage; when `bytes == 0` both are 0. |
| `safe_open.rs:310:21` NotFound guard → `false` in `open_regular` | `ENOENT` then goes through `name_the_refusal`, whose `lstat` sees the same absence and returns the same `Io { action: "open", source }`. It costs one more `lstat`, and differs only if a name appears between the two calls, which no test can stage. |
| `staging.rs:254:21` NotFound guard → `true` in `create` | In practice: reachable only if `lstat` fails other than `ENOENT` straight after `open` said `EEXIST`, i.e. the directory's search permission revoked between two calls. |
| `staging.rs:263:21` AlreadyExists guard → `true` in `create` | In practice: reachable only if the second `O_EXCL` create fails other than `EEXIST` after the unlink succeeded, i.e. the directory changed between two calls. |
| `sort_index_writer.rs:651:21` NotFound guard → `true` in `clear_staging`; `sort_index_writer.rs:661:21` the same in `walk_sidecar` | In practice: the `.srt` and its `.new.srt` staging name live in the directory of the bar file the door or writer has just opened, so `lstat` there fails other than `ENOENT` only if that directory's permissions change between two calls of one function. |
| `verdict.rs:410:29` `<` → `<=` in `Code::worse` | Severity is injective over the codes, so equal severity means the same code, and either answer is that code. |
| `verdict.rs:839:9` `Series::feed_byte` → `Ok(0)` and → `Ok(1)` | Both callers discard the value (`series.feed_byte()?` in `validate`; the header's feed byte is written from `feed_code` directly); the `Err` arm needs a `Vendor` outside `Vendor::ALL`, which the type cannot hold. |
| `verdict.rs:3016:30` guard → `true` / `false`, `3016:42` `!=` → `==` in `write_durable` | In practice: the guard runs only when `remove_file` FAILS after a refused write, on the temporary this call created in the same directory moments before. Failing needs the directory changed, or the file removed, by another actor between two calls. The `cfg(test)` write seam can be armed only from `verdict_tests.rs`, which this session did not edit. |
| `verdict.rs:3080:21` guard → `true` / `false`, `3080:32` `!=` → `==` in `remove_tmp` | In practice, the same: every call removes a temporary this writer made, under the commit lock, in a directory it has just written. |
| `verdict.rs:3102:21` guard → `true` / `false`, `3102:32` `==` → `!=` in `genesis` | In practice: the link runs under the commit lock straight after `open_existing` found no index, so `EEXIST` needs an actor that does not take the lock to create the index between two calls; any other link error (`EMLINK`, `EPERM`, `ENOSPC`) is a host fault no test-owned substitute produces in the directory of a file just written. |
| `verdict.rs:3277:25` guard → `true` in `publish` | In practice, the same as `genesis`: a link error other than `EEXIST`, under the lock, is a host fault. |
| `sealed.rs:1504:32` `<` → `<=` in `Fold::push` (`table.len() < table_rows`) | At most one extra entry is pushed once the table is full, and `finish` resizes the table to exactly `table_rows`, cutting it off; nothing reads the table before `finish`. |
| `sealed.rs:2352:35` guard `block > 0` → `true`, `2352:41` `>` → `>=` in `last_before` | The guard is evaluated only when `index >= 1`. Here `index = skip + at` and `block = lo + at`, where `skip` is 0 when `lo` is 0 and 1 otherwise, so `index >= 1` already implies `block >= 1`, and both forms are always true there. |
| `file.rs:215:32` `>` → `==` and → `>=`, `file.rs:220:15` `>` → `>=` in the `MAX_ROW_LEN` constant | Compile-time comparisons of the shipped widths (`RECORD_LEN` 56, `GREEK_LEN` 80, `OVERLAY_LEN` 24): all forms pick 80, and any width change that could tell them apart trips `const _: () = assert!(MAX_ROW_LEN == 80)`. |
| `file.rs:4748:63` `&&` → `\|\|` in `missing_below` | The open refuses a missing root (`Missing`) or a non-directory root (`NotADirectory`) before `create_below`, and `dir` is `root.join(..)`, always under the root. The mutant's only extra step is the root itself, which exists, so `try_exists` answers `Ok(true)` and the walk ends where the original's does. |
| `file.rs:5123:42` guard `O_SYNC.is_none()` → `false` in `DurableBatch::defer_file` | On this target: `O_SYNC` is a `const Some(..)` on Linux x86_64/aarch64 and macOS (`file.rs:5033`), so `is_none()` is false at compile time; only a build for a host without a verified `O_SYNC` (`file.rs:5045`) tells them apart. |
| `file.rs:5328:67` `<` → `<=` in `flush_all` | Each target index is attempted once, so a failure at an index already recorded never recurs: `at == earlier` is never true, and both forms choose the same. (`==` and `>` there are killed.) |
| `file.rs:5402:9` `Gathering::drop` → `()`, `5402:12` delete `!` | In practice: `with_directories_flushed_once` calls `finish()` on every `Ok` and `Err` path of the open, so a `Gathering` is dropped unfinished only while unwinding from a panic inside the open, which no store input causes. |
| `file.rs:5542:27` guard `through` → `true` / `false` in `writer_flags` | On this target: the arm is in `#[cfg(not(any(macos, linux\|android x86_64\|aarch64)))]`, not compiled on Linux x86_64. |
| `file.rs:5572:5` `refuse_link` → `Ok(())`, `5573:21` `is_symlink` guard → `false` | On this target: every call (the lock path twice, the checksum sidecar once) is followed at once by an `O_NOFOLLOW` open of the same path (verified here), which a final link fails `ELOOP`, and `writer_open` turns `ELOOP` into the same `StoreError::Symlinked { path }`. `refuse_link` is the guard for a host without a verified `O_NOFOLLOW`. |

## Findings (not fixed; no production line changed)

- **`staging::create` under a concurrent unlink** refuses `staging_linked`
  with **0** links ("one of 0 names of a file"): the `lstat` caught the inode
  mid-unlink. Safe (nothing removed, truncated or written) but misnamed.
  Observed by `a_leftover_that_vanishes_before_the_look_leaves_the_create_free`.
- **`sealed::fold_extremes`' doc comment cites a test that does not exist**:
  `sealed::the_head_extremes_are_the_sort_runs`. Nothing in this tree has
  that name. The tie rule it names is now held by
  `a_sealed_heads_extremes_name_the_earliest_bar_of_each_tie`.
- **`staging_tests.rs`'s scratch directories are not hermetic**: `dir()`
  names one `brutex-staging-<tag>-<pid>-<n>` and never clears it, and two of
  its tests never delete it. Once pids wrap (`pid_max` is 32,768 here), a
  later process meets an earlier one's leftovers:
  `a_free_name_is_created_empty_and_nothing_is_cleared` then sees
  `cleared: Some(9)` and
  `a_symbolic_link_or_a_directory_at_a_staging_name_is_refused_and_left`
  fails on `EEXIST`. Both failed this way in one coverage run here, after
  about 10,000 test processes. This is a test defect, not a code defect. This
  session's runs cleared stale scratch directories before each run, and its
  own tests clear their directory first.
- **`llvm-cov`'s merged report undercounts this workspace** ("950 functions
  have mismatched data"). Its merged record keeps one of a function's
  differing copies (a `const fn` or generic codegen'd in several test
  binaries, or the lib built with and without `cfg(test)`), and when the kept
  copy never ran, lines a passing test executes report 0. For example,
  `path.rs:534-537` ran 25 times in `tests/unit.rs` but read 0 merged. The
  coverage below is a per-binary union, which counts a line covered when any
  test binary ran it.

## Environment failures in the base suite (not caused by this branch)

| test | why it fails here | status |
|---|---|---|
| `store::staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported` | `assert_ne!(ino)` after unlink + create: ext4 reuses the freed inode number (fails 40/40 alone). Passes on tmpfs and APFS. A test defect, not a code defect. | left unchanged; the new race test proves the same branch without inode numbers |
| `store tests/cited_commits.rs every_commit_the_store_records_cite_is_on_main_or_says_why_not` | the cloud base is a one-commit snapshot, so the commits the docs cite are not ancestors | environment only |
| GitHub CI on this branch's PR: `Gate W — the browser tree`, step W1 "web/build matches web/src" | the base snapshot's committed `web/build` is not what its own `web/src` builds (18 files differ); this branch changes nothing under `web/`. Gate W fails first and the run cancels the Rust gates, so CI did not run them on most heads of this branch: they were run locally (Gates, below). | not this branch's; needs a `web/build` rebuild on the base |
| GitHub CI, `Gate 1+2 — one language`, step 1e (when Gate W does not cancel it first): `core tests/banned_lists.rs` `the_readers_bite`, `the_three_banned_lists_agree` | both panic "the step declares its list as VARIABLE=\"...\"": the test parses `.github/workflows/ci.yml`, and the snapshot's workflow and test disagree. Reproduced on the pristine base worktree (`c9e3472`). | not this branch's (no change under `crates/core` or `.github/`) |
| four `telemetry::sink::tests` chmod tests | fail as root (mode bits do not bind); pass with `CAP_DAC_OVERRIDE` dropped. They do not use `where_permission_binds` (D-0995) as the store's do. | environment only |

## Coverage

**Production-only, per-binary union.** "Production" means the lines above a
file's first `#[cfg(test)]` test module. Counting the new test modules would
add thousands of lines to `sink.rs` and `verdict.rs`. Before was measured on
the base tree, after on this branch at `a4c0f2c`. The profiles of the
uid-65534 children are included both times.

| file | lines | missed before → after | line % before → after | branches | missed before → after | branch % before → after |
|---|---:|---:|---:|---:|---:|---:|
| `store/src/emits.rs` | 285 | 6 → 6 | 97.89 → 97.89 | 2 | 0 → 0 | 100.00 → 100.00 |
| `store/src/file.rs` | 2,139 | 136 → 67 | 93.64 → 96.87 | 308 | 43 → 21 | 86.04 → 93.18 |
| `store/src/path.rs` | 205 | 0 → 0 | 100.00 → 100.00 | 30 | 0 → 0 | 100.00 → 100.00 |
| `store/src/safe_open.rs` | 176 | 10 → 8 | 94.32 → 95.45 | 22 | 2 → 2 | 90.91 → 90.91 |
| `store/src/sealed.rs` | 1,407 | 0 → 0 | 100.00 → 100.00 | 274 | 6 → 6 | 97.81 → 97.81 |
| `store/src/sealed_codec.rs` | 635 | 0 → 0 | 100.00 → 100.00 | 90 | 0 → 0 | 100.00 → 100.00 |
| `store/src/sort_index_writer.rs` | 380 | 23 → 10 | 93.95 → 97.37 | 60 | 9 → 4 | 85.00 → 93.33 |
| `store/src/staging.rs` | 91 | 18 → 2 | 80.22 → 97.80 | 12 | 6 → 2 | 50.00 → 83.33 |
| `store/src/time_index.rs` | 255 | 2 → 0 | 99.22 → 100.00 | 54 | 3 → 3 | 94.44 → 94.44 |
| `store/src/verdict.rs` | 2,035 | 222 → 16 | 89.09 → 99.21 | 298 | 77 → 29 | 74.16 → 90.27 |
| `telemetry/src/sink.rs` | 1,303 | 38 → 4 | 97.08 → 99.69 | 264 | 31 → 10 | 88.26 → 96.21 |

`lib.rs` has no instrumented line. Branch counts take, per line, the most
branch entries any one binary reports for it.

### Production lines still uncovered, and why

| file:lines | why no test reaches them |
|---|---|
| `emits.rs:788-792, 801` | Not production: the message arguments of an assertion inside a `#[test]` fn written in `emits.rs` itself, evaluated only when it fails. |
| `file.rs:1759, 2571, 2577, 2586` | The `?` of a write or seal of a block the handle has just measured: a host failure only. |
| `file.rs:2620` | `crc_path` absent at the tail check: only a file born without the checksum flag. |
| `file.rs:3157, 4341` | A `.tix` read failing other than short, on a handle this process just opened: a host I/O error. |
| `file.rs:3337, 3545` | `tix` is `None` after a reindex. The only caller (`index_batch`) has already returned when it is `None`. |
| `file.rs:3346, 4408-4416` | `resume` failing on an index this handle has just rebuilt and written (a host fault), and `index_refused`, reached only from such arms. |
| `file.rs:3351, 3455, 3534, 3562, 3565` | The time index is `None` or not `Ready` after the caller found it so. |
| `file.rs:3362-3372, 3475-3494` | `extend` refusing `OffGrid`, `Outside` or another reason after admission has already refused those bars. |
| `file.rs:3411` | `index_note`'s "lookups bisect" arm, evaluated only by the `Debug` `store.append` line. Covered by `each_commits_line_says_what_it_left_the_time_index_as`, committed after the final coverage run (`f1c6492`), and **not re-measured**. |
| `file.rs:3783, 3793-3795` | A covered range no `Layout` produces; a buffer whose length a `const` assertion fixes. |
| `file.rs:3920, 3929, 3933, 3951` | A read failing only during overlap diagnosis, after the same records were read to decide the overlap; and `index >= n_valid`, which a strictly increasing batch past the last held stamp has already passed. |
| `file.rs:3981` | `suffix_that_follows` on an empty month: admission refuses an unordered batch (`BatchNotOrdered`) before `Header::advance`, and with no bar held `advance` refuses only `last < first`. |
| `file.rs:5403-5404` | `Gathering` dropped unfinished: only while unwinding from a panic inside an open. |
| `file.rs:5561-5563` | `ELOOP` from a link swapped in after `refuse_link` looked. |
| `file.rs:5586` | A leaf not under the root: every caller passes one under it. |
| `safe_open.rs:107` | `Found::of` → `Unknown`: every file type Linux reports is one of the six named kinds. |
| `safe_open.rs:300-306` | `cfg`'d for targets without verified open flags; not built on Linux x86_64. |
| `sort_index_writer.rs:334` | `write_whole` with `n_valid == 0`: both callers return before it. |
| `sort_index_writer.rs:509-514` | `Unsortable`: `build_run_as` refuses only `Outside`, `NotWholeSecond` and `Empty`, and the writer refuses `OffGrid` and `OutsideMonth` at append, so only a bar file written outside this writer reaches it. |
| `sort_index_writer.rs:611` | `verify_bodies` failing to read a `.srt` the walk has just read: the file shrinking between two reads. |
| `sort_index_writer.rs:652, 662` | The arms of the two `sort_index_writer` equivalents above. |
| `staging.rs:255, 266` | The arms of the two `staging` equivalents above. |
| `verdict.rs:717` | `feed_code` → `None`: `Vendor::ALL` lists every variant in declaration order. |
| `verdict.rs:1515` | `fmt_asked`'s catch-all: it is called only for the variants it names. |
| `verdict.rs:2932` | `flock` failing other than `EWOULDBLOCK`: a host fault. |
| `verdict.rs:3016, 3080, 3102-3105, 3281-3283` | The arms of the `write_durable`, `remove_tmp`, `genesis` and `publish` equivalents above. |
| `verdict.rs:3916, 4066, 4127` | `handle` answering `None` for an image `prefix` has just opened and cached. |
| `verdict.rs:3972` | An index born between the reader's two looks: a concurrent writer's race. |
| `sink.rs:3146, 3148, 3150` | The roll-aside directory vanishing right after this call created it: a race. |
| `sink.rs:3331` | A posting path with no parent: every posting path is under the audit directory. |

**CI gate 20 is an exact-count ratchet** on telemetry's uncovered lines
(`sink.rs 21` in this base's `ci.yml`). Lines this branch newly covers in
`sink.rs` lower that count, and the gate fails a file with FEWER uncovered
lines than declared until the count is lowered. That edit is in
`.github/workflows/ci.yml`, which this session may not touch: **the operator
must re-measure and lower the `sink.rs` count (and the prose in
`docs/06-limits.md` §54) when re-applying this branch.**

## Gates

*Pending: filled in once measured.*

## NOT DONE

*Pending.*

## Appendix A: new unit tests (private items)

Each module is `#[cfg(test)] mod cloud_mutant_tests`, appended at the end of
its file. The middle column is what the test asserts; the right column is the
first sentence of its doc comment, which names the mutants it kills or the
lines it covers.

| test | asserts | kills or covers |
|---|---|---|
| `verdict::cloud_mutant_tests::every_check_of_an_image_header_refuses_by_name` | every check of an image header refuses by name. | The magic, the version, the stride, the CRC (and a header cut short), the reserved bytes, a timeframe no rung has, a unit this build does not know or that is not the timeframe's, and a feed, exchange, segment or mode byte no table holds. |
| `verdict::cloud_mutant_tests::a_header_whose_geometry_cannot_hold_its_records_is_refused` | a header whose geometry cannot hold its records is refused, and one that can is measured exactly. | The table's capacity (not a power of two, below the floor, above the ceiling), records that overfill it, per-code counts that do not sum to the records or count code 0, and a detail section out of its place or past its cap. |
| `verdict::cloud_mutant_tests::every_check_of_a_day_block_refuses_by_name` | every check of a day block refuses by name. | Its CRC (and a block cut short), its reserved bytes, and each of its five enumerations at a byte this build does not know. |
| `verdict::cloud_mutant_tests::every_check_of_a_record_refuses_by_name` | every check of a record refuses by name, naming the record. | Its CRC (and a record cut short) as `record_corrupt` with its ordinal; a key field that is empty, not UTF-8, or not zero after its key; a code this build does not know; and reserved bytes that are not zero. |
| `verdict::cloud_mutant_tests::every_whole_image_check_refuses_by_name` | every whole-image check refuses by name. | Covers `parse`: an image shorter than a header and a day block, a length the header does not imply, records out of (code, key) order, per-code counts that differ from the records, an empty slot carrying a tag, a slot naming a record past the records, a slot whose tag is not its record's, more occupied slots than records, and a record its probe window does not find. |
| `verdict::cloud_mutant_tests::every_detail_section_check_refuses_by_name` | every detail-section check refuses by name. | Covers `parse_details`: an entry that runs past the section, an entry out of owner order, an entry of kind 0, an entry naming no record, an entry count the header does not state, and a record whose `detail_at` is not its first entry. |
| `verdict::cloud_mutant_tests::the_detail_section_holds_exactly_its_cap_and_caps_whole_owners` | the detail section holds exactly `detail_cap` bytes; the day's details are kept whole or not at all, and so is each record's, and once one owner does not fit every later owner is capped too. | Covers `section`'s cap: day details of exactly the cap are kept and one byte more are not; a record whose details reach the cap exactly is kept, the next owner that would pass it is capped and flagged `DETAILS_CAPPED`, and so is every owner after it even when it would fit. |
| `verdict::cloud_mutant_tests::days_before_year_one_are_counted_the_proleptic_way` | days are counted the proleptic gregorian way before year 1 too. | Kills `replace - with +` and `replace - with /` in the era of a negative year (`(y - 399) / 400`): `Day` begins at year 1, so only a direct call reaches it, and the function's contract is the whole proleptic calendar. |
| `verdict::cloud_mutant_tests::a_detail_section_of_exactly_the_cap_is_in_place` | a detail section of exactly `detail_cap` bytes is in place. | Kills `replace > with >=` in `geometry`'s cap test, which refuses a section `section` itself fills to the cap exactly. |
| `verdict::cloud_mutant_tests::an_image_of_exactly_the_floor_is_read_and_refused_by_its_geometry` | an image of exactly a header, a day block and a trailer is read as one, and refused for what it lacks. | Kills `replace < with <=` in `parse`'s length floor: such an image is long enough to be read, sealed, and refused by its geometry (its header implies a table), never as "shorter than a header and a day block". |
| `verdict::cloud_mutant_tests::a_detail_section_is_walked_to_its_end_in_bounded_time` | a detail section is walked entry by entry to its end, in bounded time. | Kills `replace += with *=` in `parse_details`' step to the next entry (from 0 it never moves; caught before only by a timeout): a whole image with details is parsed on a worker with a deadline, and every detail is read back in place. |
| `verdict::cloud_mutant_tests::a_key_is_found_only_by_its_hash_and_its_text_together` | a key is found only by its hash and its text together. | Kills `replace && with \|\| in Reader::lookup`: a record whose hash is the asked key's but whose text is another's (a 64-bit hash collision, made here by building the record's key by hand) must not answer for it; nor may the record's text asked under another hash with the same tag and home slot (bit 20 changed). |
| `verdict::cloud_mutant_tests::a_record_whose_probe_window_finds_another_first_is_refused` | a record whose probe window finds another record first is refused. | Kills `replace match guard found == ordinal with true` in `parse`, which would accept any record its window finds something for. |
| `verdict::cloud_mutant_tests::a_day_at_the_last_generation_is_refused_never_wrapped` | a day whose generation counter is at its end is refused, never wrapped. | Covers `publish`'s exhausted counter: a year index whose day has committed generation `u32::MAX` (written here as a valid next slot) has no next generation, so the commit is refused `image_corrupt` naming the year index, and no image is linked. |
| `verdict::cloud_mutant_tests::a_whole_image_with_a_damaged_day_block_is_refused_by_it` | a whole image whose day block fails its check is refused by the day block's own refusal. | Covers `parse`'s day-block step: one byte of the day block changed and the image re-sealed, so the trailer and the header pass and the day block's CRC is what refuses; the refusal is exactly what `decode_day` says of those bytes alone. |
| `verdict::cloud_mutant_tests::a_crashed_writer_names_the_point_it_stopped_at` | a writer stopped at a crash point says where. | Covers the text of the test-only `crashed` refusal, which the crash seam's tests match by variant and never print. |
| `sealed::cloud_mutant_tests::a_time_table_has_one_entry_per_slot_of_the_span_rounded_up` | a month's time table has one entry per `table_slots` bars of its span, rounded up, at every timeframe. | Kills `replace * with /` and `replace * with +` in the slot width and `replace - with +` and `replace - with /` in the rounding of `table_entries_for`. |
| `time_index::cloud_mutant_tests::a_slot_is_its_bucket_and_its_bit` | a slot is its bucket and its bit: `slot / 64` and `slot % 64`. | Kills `replace / with *` in `split`, which sends every lookup to an entry 4,096 times further than its own. |
| `time_index::cloud_mutant_tests::a_bar_several_buckets_on_is_reached_in_bounded_time` | a bar several buckets on is reached, each bucket between counting what came before it. | Kills `replace += with *=` in `extend`'s walk to the next bar's bucket (`at` never advances, so the walk never ends): the walk runs on a worker with a deadline, so that mutant fails here at once rather than hanging the suite, and the entries are checked exactly. |
| `sealed_codec::cloud_mutant_tests::values_pack_least_significant_bit_first_into_whole_bytes` | values are packed least significant bit first, whole bytes out as soon as they are whole, the last byte padded with zeros. | Kills `replace / with *` in `pack`'s count of whole bytes (caught before only by the exhaustive codec tests timing out): it pushes eight times too many bytes per value, which this fails on at once. |
| `file::cloud_mutant_tests::an_overlap_that_agrees_with_everything_it_meets_has_no_diagnosis` | an overlap that agrees with everything it meets has no diagnosis. | Kills `replace && with \|\|`, and `replace < with <=` on either side, in `BarFile::diagnose_overlap`'s last check. |
| `sink::cloud_mutant_tests::a_full_flood_table_drops_exactly_the_pairs_with_nothing_to_say` | a full flood table drops exactly the pairs with nothing to say. | When a new pair meets a table of [`FLOOD_PAIRS`], the table keeps every pair that still holds an unreported count (`held > 0`) or whose window is still open (`at < start + FLOOD_WINDOW_MS`), and drops the rest. |
| `sink::cloud_mutant_tests::only_a_ten_byte_dashed_date_is_a_day` | a day is exactly `yyyy-mm-dd`. | Kills `replace is_day_text -> bool with true` and `replace && with \|\|` after the length test: either files a 10-byte text that is not a day, or any text at all, under the `day` dimension of the audit index, so a probe for a real day would share its posting with garbage. |
| `sink::cloud_mutant_tests::a_month_is_posted_only_when_it_is_one` | an event's `month` is posted only when it is `yyyy-mm`; otherwise its day's month is. | Kills `replace == with !=` and `replace && with \|\|` in the month filter of `postings_of`: the first refuses every real month (an event naming only `month = "2026-10"` would carry no month posting), the second posts any seven-byte text as a month in place of the day's own. |
| `sink::cloud_mutant_tests::a_gone_folder_that_cannot_be_asked_about_is_refused` | a name that cannot be asked about is not "nothing there". | Kills `replace match guard e.kind() == NotFound with true` in `audit_gone_is_ours`: it answers `Ok(None)` for every refusal, so a `audit.idx.gone` the host will not describe (here: below a regular file, `ENOTDIR`) would be taken for absent and created or listed past. |
| `sink::cloud_mutant_tests::the_reaper_deletes_what_is_there_and_counts_what_is_gone_as_done` | a retired generation already gone is reaped, not a failure; one that is there is deleted. | Kills `replace reaper with ()` (nothing is ever deleted; caught before only by a test that timed out waiting) and `replace match guard e.kind() == NotFound with false` in `reaper` (an entry something else already removed is counted as a failed delete and the stream turns loud for ever). |
| `sink::cloud_mutant_tests::a_refused_delete_is_counted_and_named` | a delete refused because the folder changed is counted and named. | Kills `replace Reap::fail with ()`: a refusal would leave no count and no words, so `reap_failures` reads 0 while the entry stays on disk. |
| `sink::cloud_mutant_tests::an_open_audit_stream_holds_its_reapers_channel` | an open stream has a reaper to hand retired generations to. | Kills `replace Audit::start_reaper -> Result<(), String> with Ok(())` and the same of `Audit::start_reaper_with`: neither starts the thread nor keeps its channel, so every roll's retired generation waits for ever. |
| `sink::cloud_mutant_tests::an_audit_line_that_fills_the_file_exactly_does_not_roll_it` | the audit file rolls only when a line would pass its bound; a line that fills it exactly stays. | Kills `replace > with >=` in `Audit::append`'s roll test: it rolls a file that the next line fills to exactly `max_file_bytes`, so every file ends one line short of its bound and the stream rolls (and retires an index generation) one line early. |
| `sink::cloud_mutant_tests::the_posting_handle_cache_is_bounded_and_kept_until_it_must_be_emptied` | the posting-handle cache holds up to `audit_handles` files and is emptied only for a new one past them. | Kills the three mutants of `self.handles.len() >= AUDIT_HANDLES && !self.handles.contains_key(&path)` in `Audit::post`: `&&` to `\|\|` and `>=` to `<` empty the cache at every new posting (an open per pointer on the emit path); deleting `!` never bounds it (a handle per posting ever written, up to the process's file limit) and empties it for a posting it already holds. |
| `sink::cloud_mutant_tests::the_audit_health_carries_the_reapers_counts_and_words` | the stream's health carries the reaper's counts and words. | Kills `delete field reap_failures from struct AuditHealth expression in Audit::health` (caught before only by a test that timed out waiting): the page would read zero failed deletes while the reaper counted them. |
| `sink::cloud_mutant_tests::a_full_table_of_owed_pairs_writes_a_new_pair_unfollowed` | a full table of pairs still owed a count writes a new pair unfollowed. | Covers the arm of `Flood::admit` where the eviction frees nothing: no pair may be dropped while its held count is unsaid, so the new pair is written and not followed (it loses nothing), and the table is left as it was. |
| `sink::cloud_mutant_tests::an_audit_stream_that_cannot_open_says_why_and_drops_by_count` | an audit stream that cannot open says why, and counts every line it then cannot keep. | Covers `Audit::open`'s two refusals (a serial that cannot be read, a current file that cannot be opened), `Audit::failed`, and `append`'s arm for a stream with no file. |
| `sink::cloud_mutant_tests::seeding_passes_over_what_it_cannot_decode_or_read` | a new stream seeds only the ring's audited lines it can read. | Covers `Audit::seed`'s two refusals: a line that does not decode is passed over, and a ring file that cannot be read (a directory in its place) ends that file's walk. |
| `sink::cloud_mutant_tests::an_audited_line_the_file_refuses_is_dropped_and_named` | an audited line the file will not take is dropped, counted and named. | Covers `Audit::append`'s failed write. |
| `sink::cloud_mutant_tests::a_posting_that_cannot_be_made_is_an_index_failure` | a posting that cannot be made is an index failure, and the line still lands. | Covers `Audit::post`'s refused `create_dir_all` and `append`'s count of it: a regular file stands where the `target` dimension's directory belongs. |
| `sink::cloud_mutant_tests::a_roll_that_cannot_delete_the_oldest_file_stops_rolling` | a roll that cannot delete the oldest file stops rolling, says so, and the line still lands. | Covers `Audit::roll`'s named failure and `append`'s latch: a directory with a file in it stands at the oldest kept name, so the roll's first step is refused. |
| `sink::cloud_mutant_tests::an_audited_event_neither_file_takes_is_dropped_from_both` | an audited event neither file takes is dropped from both, each counted in its own health. | Covers `Sink::emit`'s report of an audit failure on the path where the ring's append failed too: the ring counts the event dropped and names its own failure, and the audit stream counts and names its own. |
| `sink::cloud_mutant_tests::a_retired_generation_under_a_changed_folder_says_what_changed` | a retired generation whose folder is gone, or is another folder now, is not deleted through, and the reason says which. | Covers `changed_under`'s three answers: the folder handed over is still that folder (nothing to say), another directory has its name, and nothing has its name. |

## Appendix B: new integration tests (public items)

In `crates/store/tests/cloud_mutants_store.rs` and
`crates/telemetry/tests/cloud_mutants_sink.rs`. Tests run as uid 65534 through
`where_permission_binds` show their body's doc comment.

| test | asserts | kills or covers |
|---|---|---|
| `cloud_mutants_store::a_staging_create_the_host_refuses_is_named_a_create` | a staging name the host refuses to create is refused as a create, not looked at. | Kills `replace match guard why.kind() == ErrorKind::AlreadyExists with true` in `staging::create`: every refusal of the first `open` would be taken for "something is there", so a name below a regular file (`ENOTDIR`) is answered by the `lstat` that follows, as a `stat` failure, with a wasted host call, where the host's refusal was the create's. |
| `cloud_mutants_store::every_found_kind_has_its_own_word_in_the_refusal` | every kind a reader refuses has its own word. | Kills `replace Found::word -> &'static str with ""` and `with "xyzzy"`: the word is what a refusal's line and its `found=` field carry, so a blank or invented word would leave an operator unable to tell a planted pipe from a link. |
| `cloud_mutants_store::a_leftover_that_vanishes_before_the_look_leaves_the_create_free` | a leftover that goes between the create and the look leaves the name free, and the create goes on. | Kills the mutants of `staging::create` that only a race between its host calls reaches: the `lstat` guard `why.kind() == ErrorKind::NotFound` to `false` and `==` to `!=` (a name refused by `O_EXCL` and gone by the `lstat` is refused as a `stat` failure, though nothing stands in the writer's way); the `remove` guard `why.kind() != ErrorKind::NotFound` to `true` (a leftover gone by the `unlink` is refused as a `remove` failure); and the second create's `AlreadyExists` guard to `false` (a name taken again after its removal is an `io` failure, not `staging_contended`). |
| `cloud_mutants_store::a_leftover_whose_name_cannot_be_removed_is_refused_as_a_remove` | a leftover whose name the host will not remove is refused as a remove, and left whole. | Kills `replace match guard why.kind() != ErrorKind::NotFound with false` and `replace != with ==` in `staging::create`: both swallow the `unlink` refusal of a read-only directory (`EACCES`), and the create that follows meets the leftover still there and says `staging_contended`, "another writer is using the name", when the truth is that this one cannot clear it. |
| `cloud_mutants_store::every_staging_refusal_names_itself_its_path_and_its_cause` | every staging refusal says what it is, names its path, and carries the host's error only when there is one. | Covers the refusal text, `path` and `source` of the two refusals the other staging tests meet only through a race or a host error: the operator reads these words in the caller's event. |
| `cloud_mutants_store::every_found_kind_has_its_sentence` | every kind a reader refuses is said in words an operator can act on. | Covers [`store::safe_open::Found::sentence`] for every kind, `Unknown` included: no host this crate builds for produces a file type outside the other six, so only a direct call reaches that arm. |
| `cloud_mutants_store::every_rebuild_door_refuses_a_revision_that_moved_after_promotion` | a revision written to after its promotion is refused by every rebuild door, by name. | Covers the D-3350 refusal in `BarFile::sort_index_existing` ("not sorted"), `BarFile::index_existing` ("not indexed") and `BarFile::open_existing_audited` ("not audited"): a promoted revision appended to outside the repair protocol no longer matches the pointer its promotion wrote, and no door sorts, indexes or audits it as if it did. |
| `cloud_mutants_store::the_rebuild_door_names_each_way_an_index_fails_its_month` | the rebuild door names every way an index fails to describe its month, and writes it whole. | Covers each reason `BarFile::sort_index_existing` gives before it writes a month whole: runs that order only the first rows, bytes past the last run, an index that will not walk, and runs that are another month's bars. |
| `cloud_mutants_store::a_small_months_missing_time_index_is_said_as_the_rule` | a small month's missing time index is said as the rule, not as damage. | Covers `time_index::Why::BelowThreshold`'s text: the reason a reader prints for a month under `INDEX_MIN_ROWS` bars names the threshold, the rule and its bound, so an operator does not take it for a lost `.tix`. |
| `cloud_mutants_store::every_verdict_refusal_has_its_word_its_sentence_and_its_cause` | every verdict refusal has its word, its exact sentence, and its cause only when it has one. | The verdict store's refusals are what an operator reads in a `store.verdict` event and on the CLI; each variant's word and text is pinned here, exactly, with the variables it names. |
| `cloud_mutants_store::every_verdict_code_and_byte_enumeration_round_trips_and_says_its_word` | every verdict code and one-byte enumeration round-trips through its byte, and says its word. | The words are what the routes and documents print (D-4809: numbered append-only, never renamed); a byte this build does not know is `None`, never another value. |
| `cloud_mutants_store::a_flag_bit_this_build_does_not_name_is_kept_and_said_by_number` | a flag bit this build does not name is kept, and said by number. | Covers `flags::unnamed`: a reader keeps every bit a later build appended (D-4844) and names it `flag_<n>` beside the named ones. |
| `cloud_mutants_store::every_verdict_filing_byte_round_trips_and_an_unknown_one_names_nothing` | every filing byte round-trips, and a byte no table holds names nothing. | Covers the exchange, segment and feed bytes of a verdict image's series: each value is filed under its own byte and read back as itself, and 0 or a byte past the table is `None`, never another exchange, segment or vendor. |
| `cloud_mutants_store::a_long_stem_is_cut_at_a_character_and_hashed_whole` | a stem longer than the key field is cut at a character boundary, keeps its whole name, and hashes as the whole. | Covers `Key::hash` and the stem's cut: the kept text never splits a character, `whole` and `Display` give the name as written, and the hash a probe uses is the whole name's, so two stems sharing their first 64 bytes are two keys. |
| `cloud_mutants_store::a_verdict_reader_answers_what_is_not_there_as_absent` | a reader answers what is not there as absent, and says why a day holds no record. | Covers the reader's empty answers: a page of a day never committed is `None`, a generation with no detail section has no details, a key the day does not hold is answered by the day's `NoRecord` and its code, and a new reader holds no handle. |
| `cloud_mutants_store::a_verdict_reader_refuses_a_damaged_index_or_image_by_name` | a reader refuses a damaged year index or day image by name. | Covers the reader's own checks, which run without the BLAKE3 trailer a lookup never reads: a year index cut short, a slot naming a record past the records, an image longer than its header implies, and a detail entry that runs past its section. |
| `cloud_mutants_store::a_time_index_damaged_under_a_live_writer_is_rebuilt_before_the_next_commit` | a time index damaged under a live writer is rebuilt before the next commit, and the commit lands. | Covers the writer's mid-life reindex (`BarFile::index_batch` meeting an entry that fails its checksum, then `reindex`): the `.tix` entries around the month's last committed bar are damaged on disk while the writer holds the month, the next append rebuilds the index from the bars and commits, and the index afterwards describes the bars exactly. |
| `cloud_mutants_store::a_small_month_whose_committed_bar_will_not_read_crosses_unindexed` | a small month whose committed bar will not read crosses the threshold unindexed, and the commit lands. | Covers `BarFile::index_at_threshold` when the committed bars cannot be indexed: a block of committed bars is damaged on disk, the commit that brings the month to 1,024 bars commits its own bars, and no `.tix` is written that would claim bars nothing read. |
| `cloud_mutants_store::a_name_at_the_time_index_before_the_crossing_commit_refuses_it` | a name that appears at the time index before the crossing commit refuses that commit, and no bar is written. | Covers the create of `BarFile::index_at_threshold`: the `.tix` is born with `create_new`, so a file put at its name after the writer opened is never written through; the commit that would have crossed is refused and the month still holds what it held. |
| `cloud_mutants_store::the_store_refusal_sentences_name_the_file_and_the_reason` | the store's refusal sentences name the file and the reason. | Covers the text of `StoreError::TimeIndex`, `StoreError::SortIndex` and `StoreError::Generation`, and of every `Conflict` an overlap can name. |
| `cloud_mutants_store::a_batch_that_skips_held_bars_and_runs_past_them_is_refused_where_it_skipped` | a batch that restates the start of the held range, skips the rest and runs past it is refused at the first held bar it skipped. | Covers `BarFile::diagnose_overlap`'s walk past the held range: the overlap's first two bars match what the month holds, the batch then jumps beyond the last held bar, and the refusal names the third held bar as skipped, at the batch position where it should have been. |
| `cloud_mutants_store::a_durable_batch_holds_a_directory_once_and_a_sealed_one_nothing` | a durable batch holds each directory once, and a sealed one holds nothing more. | Covers `DurableBatch::defer_directory`: a directory deferred twice is one flush when the batch is made durable, and after that the batch is sealed, so a later name's flush is the writer's own (`false`) rather than one a finished batch would never issue. |
| `cloud_mutants_store::a_time_index_occupied_at_another_rank_is_stale_and_rebuilt` | a time index whose last slot is occupied at another rank is stale, and is rebuilt. | Covers the second half of `time_index::resume`'s check: the `.tix` of a month with every second from 09:15 is laid beside a month with the same last second and one second missing, so the last bar's slot is occupied but at a rank one past this month's last row. |
| `cloud_mutants_store::the_open_is_555_minutes_past_ist_midnight_and_its_divisors_align` | the nse open is 09:15 ist: 555 minutes past midnight, and the rungs that divide it align with it. | Kills `replace * with /` in `OPEN_MINUTES_PAST_IST_MIDNIGHT = 9 * 60 + 15`, which makes the open 00:15: the rungs that divide 555 and 15 are the same, so no alignment answer tells them apart, but every session offset computed from the constant moves by nine hours. |
| `cloud_mutants_store::a_commit_is_named_near_the_ceiling_at_half_the_measured_placement` | a commit is named near the ceiling at half the measured placement: 68,950 records. | Kills `replace / with *` in `RECORDS_WATCH = RECORDS_PLACED / 2`, which would name nothing until a day held twice the most records ever placed, so the day that halves the margin (RA-261, D-4925) would pass unsaid. |
| `cloud_mutants_store::a_stem_cut_inside_a_character_steps_back_in_bounded_time` | a stem cut inside a character steps back to its start, in bounded time. | Kills `replace -= with /=` in `Key::stem`'s step back to a character boundary (the cut never moves, so the step never ends; caught before only by a timeout): the stem is made on a worker with a deadline, and the cut lands before the three-byte character the field's last byte falls in. |
| `cloud_mutants_store::a_capped_day_keeps_its_capped_flag` | a day already flagged `details_capped` whose details are capped again keeps the flag. | Kills `replace \| with ^` in the day flags of a capped build: a toggle would clear a fact the caller had already stated. |
| `cloud_mutants_store::a_file_of_exactly_the_longest_day_image_is_read_and_one_byte_more_is_not` | a file of exactly the longest day image is read; one byte more is refused unread. | Kills `replace > with >=` in `read_image`'s size gate. |
| `cloud_mutants_store::two_year_index_slots_at_one_commit_are_read_from_slot_zero` | two valid year-index slots at the same commit are read from slot 0. | Kills `replace > with >=` in `read_year`, which would prefer slot 1 on a tie. |
| `cloud_mutants_store::a_store_root_that_cannot_be_listed_refuses_the_first_commit_of_a_series` | the first commit of a series makes every directory it made durable, up to and including the store root, or is refused before anything is published. | Kills `replace sync_dir -> Result<(), VerdictError> with Ok(())` and `replace == with != in make_dirs`: the first answers every directory sync "done" without one, and the second stops after the year directory's own parent, so the entries the commit made in `verify/` and in the root are never made durable. |
| `cloud_mutants_store::a_year_directory_that_cannot_be_listed_is_refused_never_absent` | a year with no index whose directory cannot be listed is the host's refusal, never "no generation". | Kills `replace match guard why.kind() == ErrorKind::NotFound with true` in `lost_index`: every failure to list the year directory would be taken for "no directory", so a reader would answer a year whose images it cannot see as a year never written, the silent absence the lost-index rule exists to refuse. |
| `cloud_mutants_store::an_unreadable_index_or_image_is_refused_never_absent` | an index or an image that is there but cannot be opened is the host's refusal, to the writer and to the reader: never "absent". | Kills three guards that take any `io` refusal for `ENOENT`: `commit_within`'s open of the year index (the writer would call a present index lost, `index_missing`), `Reader::handle` (the reader would say `index_missing` or `image_missing` of files that are there), and `plan`'s read of the image a carrying generation copies from (`image_missing`). |
| `cloud_mutants_store::a_year_directory_with_no_index_and_no_image_costs_one_open` | a year directory with no index and no image costs its one look. | Kills `replace += with *= in lost_index`: the `opendir` that tells a year never written from a lost index is counted as an open, as the function's cost line says, so a lookup of a year whose directory exists empty costs exactly one open and no read; with no directory at all, nothing. |
| `cloud_mutants_store::a_held_commit_lock_is_refused_when_the_wait_ends` | a commit lock held for the whole wait is refused `lock_stalled` when the wait ends, after one try per step, and nothing is published. | Kills `replace match guard step < steps with true` and `replace += with *=` in `lock_within` (caught before only by a timeout: both wait for ever). |
| `cloud_mutants_store::a_page_names_a_damaged_record_by_its_own_ordinal` | a page names a damaged record by its own ordinal. | Kills `replace + with *` in `Reader::page`'s ordinal of each record read: the refusal must name the record that failed its CRC, not the page's first. |
| `cloud_mutants_store::a_records_details_after_another_records_are_read_entry_by_entry` | a record's details are read entry by entry from its own offset. | Kills `replace += with *= in Reader::details` (caught before only by a timeout, as a walk from offset 0 never moves): from a record whose entries follow another record's, the walk must step one entry at a time, so both of its entries come back in order. |
| `cloud_mutants_store::an_image_whose_temporary_cannot_be_created_is_refused_and_nothing_moves` | a next generation whose temporary the host will not create is refused as the create, and the day stays at its committed generation. | Covers `write_durable`'s own reading of `staging::create`'s host refusal (`io`, never `staging`): the year directory may be read and searched but not written (mode 0555), so the image temporary cannot be born. |
| `cloud_mutants_store::a_provenance_image_that_packs_past_its_length_is_sealed_and_read` | a provenance image that packs to more than its length and two, and at most twice its length, is sealed and read back. | Kills `replace * with +` in `Head::check_geometry`'s bound on the packed provenance (`packed <= 2 * raw`, the most `pack_provenance` can write): `raw + 2` would refuse a valid head. |
| `cloud_mutants_store::a_sealed_heads_extremes_name_the_earliest_bar_of_each_tie` | a sealed head's extremes carry the earliest bar that holds each, and move only for a strictly more extreme bar. | Kills `replace > with >=` and `replace < with <=` in `fold_extremes` (D-3326: the earlier bar keeps a tie). |
| `cloud_mutants_store::a_staging_refusal_of_what_is_there_says_it_was_left_alone` | a staging name held by a link or a second name is refused in words that say what is there and that it was left alone. | Covers the text of `staging_not_regular` and `staging_linked`, which the staging tests match by variant and print only when they fail. |
| `cloud_mutants_store::a_frame_entry_at_either_bound_of_its_span_is_read_and_checked_as_a_frame` | a frame's entry is refused for its order only when it runs backwards or past the longest frame, not at either bound. | Kills `replace < with <=` twice in `SealedMonth::span`: an entry ending where it starts (an empty frame) and one exactly `max_frame_len` long are both in order, so the read goes on to the frame and the frame's checksum refuses it (`frame`), where the mutants refuse the entry (`entry_order`). |
| `cloud_mutants_store::a_month_with_no_bar_answers_every_time_lookup_with_row_zero` | a month with no bar answers every time lookup with row 0. | Covers `BarFile::locate`'s empty-month path, which answers before the time index is asked anything, for the earliest, an ordinary and the latest stamp, on a writer that has just created the month. |
| `cloud_mutants_store::a_time_index_that_cannot_be_read_is_named_and_lookups_still_answer` | a time index that is there but cannot be read is said so, and every lookup still answers by the bisection. | Covers a reader's index decision for a `.tix` name holding something other than a file: not "absent", whatever the month's size, but `Unreadable` naming the directory, and the lookup that follows answers exactly as the bars say. |
| `cloud_mutants_store::an_indexed_lookup_whose_bar_will_not_read_is_that_bars_refusal` | an indexed lookup whose bar will not read is that bar's refusal, not a silent fall back to the bisection. | Covers `BarFile::locate`'s bar fault. |
| `cloud_mutants_store::a_tabled_sealed_month_finds_every_stamp_in_three_reads` | a sealed month with a time table finds every stamp in at most three reads, at every rung that can hold one, and none outside its span. | Kills five mutants of `SealedMonth::lookup` and `last_before`: `replace \|\| with &&` in the head's early answer (a stamp at or before the first would read the table; an empty month would compute block `0 - 1`), `replace * with /` in the table's slot width (only a rung other than one second tells `4096 / tf` from `4096 * tf`, and a one-minute month of more than 64 blocks is an MCX month of exchange hours: 17,000 bars around the clock here), and `replace < with ==`, `with >` and `replace - with +` in the choice of the one-read path (a table names at most 17 blocks, so a lookup is one table read, one directory read and one frame read; the bisection those mutants take reads more, and `lo + hi` passes 64 in the month's later blocks). |
| `cloud_mutants_store::a_table_slot_of_sixty_five_blocks_is_bisected_one_entry_a_step` | a table slot that spans 65 blocks is bisected one entry a step, never read whole. | Kills `replace < with <=` in `last_before`'s choice of path: `BISECT_IN_ONE_READ` is the most directory entries a lookup reads in one positional read, and 64 blocks after the first make 65 to consider. |
| `cloud_mutants_store::a_sealed_months_audit_walks_every_block_once_and_ends` | a sealed month's audit walks every block once and ends: empty, one chunk, and more than 256 blocks. | Kills `replace < with ==` and `replace < with <=` in `audited`'s walk over the directory in chunks of 256 entries (caught before only by a timeout: an empty month under the first, and every month under the second, ask for 0 more entries for ever). |
| `cloud_mutants_store::a_damaged_index_over_bars_that_no_longer_index_is_removed_and_the_commit_lands` | a time index damaged under a live writer, over bars that no longer index, is removed, and the commit still lands unindexed. | Covers `BarFile::index_batch` after a mid-life rebuild that cannot build: the `.tix` entries the month's bars touch are damaged, and one committed bar's stamp (row 500) is moved off the one-second grid on disk, so the rebuild the next append triggers refuses the bars, removes the `.tix` and leaves the month bisecting. |
| `cloud_mutants_store::the_sort_rebuild_door_refuses_a_month_that_does_not_validate` | the sort-index rebuild door refuses a month whose bar file does not validate, exactly as a reader does, and writes nothing. | Covers `BarFile::open_to_sort`'s validation refusal: the bar file is cut one record short of what its header commits, the door is refused with the refusal a reader's open gives the same bytes, and no `.srt` is written. |
| `cloud_mutants_store::a_time_index_or_sort_index_refusal_says_which_file_and_why` | a time-index or sort-index refusal says which file and why. | Kills `replace write_time_index -> fmt::Result with Ok(Default::default())` and the same for `write_sort_index`: each would print nothing for its refusal, which an operator reads in the caller's event. |
| `cloud_mutants_store::an_indexed_one_second_lookup_reads_no_bar` | an indexed one-second lookup answers from the index alone and reads no bar. | Kills `replace == with !=` in `BarFile::locate`'s empty-month test (every month with a bar would bisect; caught before only by a timeout, as a large month's lookups then crawl): an intraday slot is exactly one bar wide, so the index alone places a stamp. |
| `cloud_mutants_store::a_small_months_lookups_bisect_to_every_row_and_end` | a small month's lookups bisect to every row, and end. | Kills `replace < with <=` and `replace / with %` in the bisection `first_at_or_after` (caught before only by a timeout: both can stop the range from shrinking). |
| `cloud_mutants_store::a_second_daily_bar_retires_an_index_already_gone_and_commits` | a second daily bar on one ist day retires the month's index, and an index already gone is already retired. | Kills `replace match guard gone.kind() == ErrorKind::NotFound with false` and `replace == with !=` in `remove_index`: both would refuse the append because the `.tix` it must remove is not there. |
| `cloud_mutants_store::a_retired_index_that_cannot_be_removed_refuses_the_append` | a retired index the host will not remove refuses the append that retired it, before any bar is written. | Kills `replace match guard gone.kind() == ErrorKind::NotFound with true` in `remove_index`, which would take any refusal for "already gone" and let the append go on with a `.tix` still claiming the month. |
| `cloud_mutants_store::a_torn_month_whose_sidecar_cannot_be_measured_is_refused` | a torn new month whose checksum sidecar cannot be measured is refused, never re-initialised. | Kills `replace match guard why.kind() == ErrorKind::NotFound with true` in `refuse_if_sealed`, which would take any refusal to measure the sidecar for "no sidecar" and re-initialise a month whose sidecar may prove committed records (D-1520). |
| `cloud_mutants_store::two_writers_making_one_new_directory_chain_at_once_both_open` | two writers making one new directory chain at once both open. | Kills `replace match guard raced.kind() == ErrorKind::AlreadyExists with false` in `create_below`: a level another opener made between this one's walk and its `create_dir` is that opener's, not a refusal. |
| `cloud_mutants_store::a_confirmed_month_is_its_regular_files_and_an_unmeasurable_one_is_refused` | a confirmed month is its regular files alone, and a month whose files cannot be measured is refused, never taken for absent. | Kills `replace match guard meta.is_file() with true` and `replace match guard gone.kind() == ErrorKind::NotFound with true` in `DurableBatch::confirm_month`. |
| `cloud_mutants_store::a_batch_flush_reports_its_earliest_failure_and_a_linked_file_is_one` | a batch's flush reports the earliest target that failed, and a held file that turned into a link is a failure, not a file gone. | Kills `replace < with ==` and `replace < with >` in `flush_all`'s choice of the refusal it reports, and `replace match guard gone.kind() == ErrorKind::NotFound with true` in its open of each target. |
| `cloud_mutants_store::a_batch_asked_for_two_threads_flushes_on_two_at_once` | a batch asked for two threads flushes on two at once. | Kills `replace == with !=` in `flush_all`'s choice of threads, which would flush a two-thread batch on one, in turn. |
| `cloud_mutants_store::a_generation_pointer_refusal_says_which_pointer_and_why` | a generation pointer's refusal says which pointer and why. | Kills `replace write_generation -> fmt::Result with Ok(Default::default())`, which would print nothing for a refused generation pointer. |
| `cloud_mutants_store::each_commits_line_says_what_it_left_the_time_index_as` | each commit's `store.append` line says what it left the month's time index as: written whole at the threshold, and gone when the bars no longer index. | Covers `BarFile::index_note`, which only the `Debug` line evaluates (`emit_if!` builds no field of a filtered event). |
| `cloud_mutants_sink::a_file_target_passes_the_hosts_fsync_refusal_through` | a file target's `sync` is the host's fsync, errors included. | Kills `replace <impl Target for FileTarget>::sync -> std::io::Result<()> with Ok(())`: a sync that answers `Ok` without asking the host reports every event durable when nothing was made so. |
| `cloud_mutants_sink::the_audit_stream_is_bounded_at_the_decided_64_mib_and_1_gib` | the audit stream rolls at the decided 64 mib and holds at most 1 gib. | Kills both `replace * with +` mutants of `AUDIT_MAX_FILE_BYTES = 64 * 1024 * 1024` (to 1,048,640 and to 66,560 bytes): either would roll the durable stream thousands of times sooner than D-3341 item 3 decided and shrink the history its 16 files hold from 1 GiB to under 17 MiB, with nothing saying so. |
| `cloud_mutants_sink::a_posting_name_at_the_ceiling_is_named_and_one_past_it_is_not` | a value of exactly `audit_max_posting_name` bytes is named; one more is not. | Kills `replace > with >=` and `replace > with ==` in `posting_name`'s length guard: both refuse a value at the ceiling, so a run or instrument whose name is exactly 120 safe bytes would go unindexed and a probe for it would find nothing. |
| `cloud_mutants_sink::every_audit_failure_alone_makes_the_stream_loud` | every signal of `audithealth::is_loud` is loud on its own. | Kills `replace > with <` on `dropped`, `rotation_failures` and `index_failures`, and `replace \|\| with &&` before `rotation_failures` and before `index_failures`: each makes one failure, alone, quiet, so a stream losing lines or index records would read healthy on the operator's page. |
| `cloud_mutants_sink::an_unreadable_audit_serial_is_an_error_and_an_absent_one_is_zero` | a serial that is there and cannot be read is an error, not serial 0. | Kills `replace match guard e.kind() == NotFound with true` in `audit_serial`: it answers `Ok(0)` for every refusal, so a stream whose serial file is a directory (or unreadable) would read as never rolled and every pointer into its rolled files would resolve to the wrong file. |
| `cloud_mutants_sink::a_pointer_past_the_kept_files_resolves_to_nothing` | a pointer resolves into the kept files only: `audit_keep_files - 1` back at most. | Kills `replace < with <=` in `audit_path_for`: the stream keeps 16 files, the current one and `audit.1` to `audit.15`, so a pointer 16 generations back names `audit.16.ndjson`, a file the stream never keeps, rather than saying the line rolled off. |
| `cloud_mutants_sink::audit_read_tells_a_rolled_off_line_from_an_unreadable_file` | a line whose file is gone or short has rolled off; one whose file cannot be read is unreadable. | Kills both mutants of the `NotFound \| UnexpectedEof` guard in `audit_read`. |
| `cloud_mutants_sink::a_posting_that_cannot_be_looked_at_is_an_error` | a posting that cannot be looked at is an error; only an absent one is empty. | Kills `replace match guard e.kind() == NotFound with true` in `audit_postings_before`: it passes over every generation whose posting cannot be asked about, so a probe through a generation that is a file (`ENOTDIR`) answers "nothing audited carried that value" where it must say it could not look. |
| `cloud_mutants_sink::a_page_from_a_place_holds_exactly_the_strictly_older_pointers` | a page from a place holds only what is strictly older: newer generations whole are passed over, its own is read below the index. | Kills the four mutants of the `before` guards in `audit_postings_before` (`serial > at.serial` to `false` or `==`, `serial == at.serial` to `false` or `!=`), which repeat or skip pointers between pages of a keyed `/logs.json` walk. |
| `cloud_mutants_sink::an_open_audit_streams_health_names_its_file` | an open stream's health names its file. | Kills `delete field path from struct AuditHealth expression in Audit::open`: the health would carry an empty path, and every "cannot append an audited line" it reports would name no file. |
| `cloud_mutants_sink::an_unindexed_stream_is_covered_by_the_newer_dimensions_from_its_end` | a stream that already holds lines but no index is covered by the newer dimensions only from where it stands. | Kills `replace > with ==`, `replace > with <` and `replace \|\| with &&` in `older = self.bytes > 0 \|\| generation.exists()` of `Audit::mark_coverage`: each takes a file with bytes and no generation for a new one, and records `run` and `code` as covering it from byte 0, so a probe by run would say "nothing carried it" for lines that were never indexed by run. |
| `cloud_mutants_sink::a_reopened_stream_records_no_dimension_twice` | a reopen records no dimension twice. | Kills `replace match guard held.iter().any(..) with false` in `Audit::mark_coverage`: every open would append every dimension again, so the coverage file grows by eight lines at each start, for ever. |

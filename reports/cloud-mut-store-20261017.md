STATUS: IN PROGRESS

# Cloud mutation and coverage pass: store (2026-10-17)

Branch `cloud/mut-store-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584`. Continues `cloud/mut-store-20261016` (its report:
`reports/cloud-mut-store-20261016.md` on that branch).

Scope: the 16 Oct pass's files, plus the two this pass touched.
- `crates/store/src/{emits, file, lib, path, safe_open, sealed,
  sealed_codec, sort_index_writer, staging, time_index, verdict}.rs`.
- `crates/telemetry/src/sink.rs`.
- Added: `crates/store/src/flock.rs`, which gained a test seam.

## Done so far

### Re-application

`git diff c9e3472e5 origin/cloud/mut-store-20261016 -- . ':(exclude)reports' | git apply -3`
applied all eight files cleanly onto the newest `feat/db-fix`. There was no
conflict, and no `feat/db-fix` change was dropped.

### Real defects found and fixed (production, `crates/store`; D-6017)

1. **A durable batch forgot a refused append barrier.**
   - The rule (D-1907): a plain writer remembers a failed barrier, so no
     later append in the process confirms the commit it guarded.
   - Inside a `DurableBatch`, `barrier_in` defers the flush instead. When
     the batch refused the deferral (the device's `fstat`), it returned the
     refusal without remembering it.
   - Measured: the same process reopened the month and appended past the
     refused commit. 1,100 bars the caller had been told were refused
     became rows of a month confirmed at 1,101.
   - Fixed: the refused deferral is now remembered exactly as a failed sync
     is. `a_refused_measure_of_a_deferred_file_refuses_the_step_that_deferred_it`
     fails on the old code and passes on the new.
2. **`staging::create` misnamed a file caught mid-unlink.** A look that
   catches a file with 0 links refused it as `staging_linked`, "one of 0
   names of a file". Such a file has no name to protect, so it is now a free
   name. This was the 16 Oct finding.
3. **`seal_committed` named the wrong file.** When the host refused the
   checksum sidecar's write, the refusal named the bar file. It now names
   the sidecar.
4. **`sort_index_writer::write_whole` had an unreachable guard.** Its
   `n_valid == 0` guard is gone (its only route returns first for an empty
   month), and the function returns `()`.

### Test seams

All are `#[cfg(test)]`, in the codebase's own shape (`sync_hooked`, D-2740;
flock's unlock seam; verdict's crash seam). Each adds a branch under test,
so the shipped call is the call every test runs.
- `file::fault` and `impl Positional for File`: a per-thread host refusal,
  armed for one action on one file.
- `staging`: a stand-in for the look, and an open refusal.
- `sort_index_writer`: `look` and `read_index_at`.
- `verdict`: `tests::interfere`.
- `flock::Flock::try_lock`: a one-shot refusal.
- `safe_open::name_the_refusal`: a look counter.
- `telemetry::sink::Audit::retire`: an actor between making the roll-aside
  folder and looking at it.

### New tests

Every one asserts behaviour, and every doc comment names what it kills or
covers.
- `crates/store/src/file_fault_tests.rs` (new, 18).
- `staging_tests.rs` (+4), `verdict_tests.rs` (+10), `safe_open_tests.rs`
  (+1).
- `verdict`, `sealed` and `time_index` `cloud_mutant_tests` (+2, +1, +1).
- `crates/store/tests/cloud_mutants_store.rs` (+2).
- `telemetry::sink::cloud_mutant_tests` (+2).

### Findings from the 16 Oct report

All are fixed:
- the staging misnaming (item 2 above);
- `fold_extremes`' citation of a test that did not exist: the test
  `the_head_extremes_are_the_sort_runs` now exists and the doc names where
  it is;
- non-hermetic scratch directories in `staging_tests.rs` and
  `safe_open_tests.rs`: they are now cleared first;
- the ext4 inode-reuse failure of the staging leftover test: the test now
  holds the old file open and checks that it keeps its bytes.

### A flaky test this branch exposed, fixed

`telemetry::sink::tests::a_run_is_read_from_every_form_a_line_carries_it_in`
failed 3 of 30 runs on this branch (0 of 30 on the base).
- It drops a sink and reopens its directory without holding
  `crate::tests::no_fork_in_flight()`.
- `FORK_GATE`'s doc asks every such test to hold it. Without it, a child
  forked by a permission test inside that window still holds the first
  sink's lock, and the reopen is refused.
- The base's timing hid the race. This branch's extra tests exposed it.
- The test, and the two other tests that reopen without the gate, now
  hold it. Measured afterwards: 0 failures in 40 runs.

### Docs

- `docs/05-decisions.md` D-6017. The number is above every D-number on any
  branch (the highest is D-5619).
- `docs/04-invariants.md` CMS-01 to CMS-09.

### Mutation run A (`cargo mutants 27.1.0`)

Files: `staging`, `safe_open`, `sort_index_writer`, `path`, `time_index`,
`flock`.

| | count |
|---|---:|
| mutants | 430 |
| caught | 195 |
| unviable | 235 |
| missed | **0** |
| timeout | **0** |

How it was run, and how the catches were checked:
- At opt-level 1, incremental, with the eight tests over 5 s skipped (and
  `cited_commits`).
- One catch is an OOM kill of the non-terminating `+=` → `*=` in
  `time_index::extend`.
- One is `flock::note_unreleased` → `()`, caught by `emits`' own test,
  which is what that test exists for.

### Mutation runs B and D, part way

Run B covers `file.rs` and `verdict.rs` (1,245 mutants). Run D covers
`sink.rs` (437 mutants). Both started before the commits that answer them,
so every survivor below is re-tested on the final tree.
- **Run B** at 290 of 1,245: 242 caught, 42 unviable, 3 missed, 3 timeouts.
  - The 3 missed are `MAX_ROW_LEN` (215:32 twice, 220:15), which are already
    argued equivalent.
  - The `suffix_that_follows` partition timeout (3996:61) is answered by
    `d2c2b19f`: the overlap tests now run under a deadline and fail in 31 s.
  - `first_at_or_after` (4313:15 `<` → `<=`, 4319:63 `/` → `%`) are
    **detected, not survivors**. Measured on each mutant, every lib test run
    alone:
    - `first_at_or_after_never_probes_more_than_the_bisection_height` fails
      at once (exit 101) on both mutants.
    - 15 lookup tests (12 under `%`) loop forever, because the bisection
      never ends or becomes a scan. They hold up the binary, so the
      binary's result is a timeout.
    - These lines are unchanged on this branch, so CI gate 18 (changed
      lines) does not mutate them.
    - Bounding 15 base tests that this pass does not otherwise touch, only
      to turn a timeout into a fast failure, is churn beyond this pass. A
      probe bound inside the production loop would add a panic branch that
      no test can take.
- **Run D** at 357 of 437: 317 caught, 35 unviable, 1 missed, 4 timeouts.
  - Missed `sink.rs` `Sink::open` `resumed.last_at > now` → `>=`: answered
    by `559624ab`, which pins the clock so a floor can equal it. That commit
    landed after run D started.
  - The 4 timeouts are `audit_postings_before`'s two generation guards
    (`serial > at.serial` and `serial == at.serial`, each forced false and
    flipped). Every one of them hangs one test,
    `a_posting_is_paged_from_a_place_without_a_line_twice_or_skipped`, whose
    page loop had no bound. The loop now refuses a fifth page, so all four
    fail in 0.15 s, with the repeated pointers in the message.
  - Run D reached 371 of 437 (329 caught, 36 unviable), then lost its
    scratch tree when run B's restart cleared the shared scratch directory.
    It was resumed in a directory of its own.
  - Its fifth timeout, `Reap::fail` → `()`, is caught by three tests. Two
    more (`a_failed_delete_is_counted_named_and_left_for_the_next_start`
    and `a_link_at_the_logs_gone_folder_is_refused_and_nothing_under_it_is_touched`)
    waited in the `reaped()` helper. It counted 120,000 one-millisecond
    sleeps, which runs well past two minutes on a loaded machine. While it
    waited, one of them held the fork gate, so every permission test queued
    behind it, and the binary passed cargo-mutants' four-minute limit. The
    helper now gives up after 30 s by the clock. Measured under the mutant:
    4 tests fail and the suite ends in 60 s. Clean, it passes in 0.16 s.
  - **Run D, final** (resumed, on `402b9349`'s tree): 354 tested in 58 min,
    315 caught, 33 unviable, 1 missed, 5 timeouts. The other 83 of the 437
    were caught earlier under the same name.
    - The 1 missed is `mark_coverage` `>` → `>=`, confirmed equivalent:
      `older` differs only when `self.bytes == 0`, and then both arms give
      0.
    - The 5 timeouts are the `reaped()` wait again: `Reap::fail` → `()`,
      `reaper` → `()`, `start_reaper` and `start_reaper_with` → `Ok(())`,
      and `reap_failures` deleted from `AuditHealth`. The helper fix landed
      after run D took its tree.
    - **Re-tested on `af6423fe` with the full suite: 12 mutants matched,
      12 caught.** Under `reaper` → `()` the suite now fails in 120 s, 9
      tests red.
- **`Key::stem` `-=` → `/=`** (run B, timeout): the 16 Oct pass bounded one
  test, but two more stemmed a name whose field ends inside a character
  with no deadline. One is this branch's re-applied
  `a_long_stem_is_cut_at_a_character_and_hashed_whole`; the other is
  `tests/verdict.rs`' `a_long_key_is_refused_and_a_long_stem_is_kept_whole`.
  Both now stem through a `stem_within` helper (a worker given 20 s).
  Measured under the mutant: every store test binary ends, and those two
  fail in 21 s (`11d407e7`).

### Coverage (final run r5, on `55956acf`)

Measured with `cargo llvm-cov` (`--branch`, `RUSTC_BOOTSTRAP=1`) over
`cargo test -p store -p telemetry`, as root with the capabilities dropped.
The union is taken per test binary with `llvm-profdata` and
`llvm-cov export`. Every test binary passed (41 results, `cited_commits`
skipped).

| file | lines | missed | branches | missed |
|---|---:|---:|---:|---:|
| `store/src/emits.rs` | 285 | 6 | 2 | 0 |
| `store/src/file.rs` | 4,437 | 64 | 358 | 20 |
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

The base, measured the same way, missed:

| file | lines | branches |
|---|---:|---:|
| `file.rs` | 83 | 28 |
| `verdict.rs` | 18 | 33 |
| `sort_index_writer.rs` | 10 | 4 |
| `staging.rs` | 2 | 2 |
| `sink.rs` | 34 | 29 |

Every uncovered line and branch left is one of these: an arm no input can
reach (each below, with its reason), an arm for another target, or test
code that runs only when a test fails. The production ones, at their r5
line numbers:

**Uncovered production lines, each unreachable, with its reason:**
- **`file.rs` defensive re-borrows**, after the caller checked the same
  value: lines and branches 3349-3350, 3363-3364, 3467-3468, 3546-3547,
  3557-3558, 3574-3575, 3577-3578. Each is a
  `let Some(tix) = self.tix.as_ref() else`.
- **`file.rs` 3375-3385 and 3493-3507**: `time_index::extend` refusing
  `OffGrid` or `Outside`. Admission has already refused those bars on the
  same grid (`Geometry::new` uses `Admission`'s `OPEN_ANCHOR_MICROS` and the
  same month bounds). `Err(other)` there is unreachable, because the stamps
  iterator never errs.
- **`file.rs` 3488-3491**: `SharedSlot` at the threshold. An intraday slot
  holds one bar, and a daily month never reaches 1,024 bars.
- **`file.rs` 2582, 2588, 3796, 3806-3808**: geometry refusals of a block
  range computed from the same layout, and a buffer that a `const`
  assertion sizes.
- **`file.rs` 3941-3942, 3993-3994, 4424-4427**:
  - the overlap walk past `held_through`;
  - an empty month at `suffix_that_follows`, which admission refuses
    first;
  - `index_refused`'s non-`Unreadable` arm, which only the arms above
    reach.
- **`safe_open.rs` 104/107**: a file type Linux does not have.
- **`safe_open.rs` 299-306**: an arm for targets without verified flags.
- **`verdict.rs` 716-717**: a vendor table that disagrees with its
  declaration.
- **`verdict.rs` 3923-3924, 4075-4076, 4136-4137**: `handle` answering
  `None` right after `prefix` cached it.
- **`sink.rs` 3410/3413**: a posting path with no parent.

**Uncovered production branches with no uncovered line:**
- **`file.rs` 3256**: `set_tix_state` on a handle with no `tix`. Every
  caller has read `self.tix` as `Some` first.
- **`file.rs` 3679**, the cache's second key (`cache.n_valid != n_valid`
  with the same block): every commit sets the cached block to `NO_BLOCK`
  (2549), so a changed count always comes with a changed block. The second
  key keeps the two saying the same thing, as its own comment says.
- **`file.rs` 5154**: `O_SYNC.is_none()`, true only on targets without a
  verified `O_SYNC`.
- **`file.rs` 5248**: the parent walk ending by `None`. It ends at the
  root's parent, which `starts_with(root)` refuses, for every root that has
  a parent. Only `/` or an empty root ends it by `None`, and no caller
  passes either.
- **`sealed.rs` 1499, 1525**: `get_mut(5)` on arrays of six (the crate
  denies indexing).
- **`sealed.rs` 1504**: the table-length guard, which never stops the loop
  (the `Fold::push` equivalent below).
- **`sealed.rs` 2352**: `Some(previous) if block > 0` with block 0. Block 0
  needs `lo == 0` and `at == 0`, which make `index` 0, so `checked_sub`
  answers `None` first.
- **`verdict.rs` 2278, 2312, 2478, 3417**: `get_mut` on tables sized by
  the value that indexes them: `first` by the plan, `counts` by
  `CODE_SLOTS` over decoded codes, `days` by the year. The crate denies
  indexing.
- **`verdict.rs` 3143**: `make_dirs`' parent walk ending by `None`. It
  breaks at the series root, and `series.dir()` is always below it.
- **`verdict.rs` 3853**: the eviction's `min_by_key` answering `None`. It
  runs only on a cache holding at least `HANDLE_CACHE` handles, so it is
  never empty.

**Test code.** These lines and branches run only when a test fails:
- the `panic!` arms of test helpers and `assert!` short-circuits:
  - `file.rs` test lines after 5980;
  - `verdict.rs` 4219, 4227, 4637 and 4974;
  - `time_index.rs` 921, 1057, 1058 and 1105;
  - `emits.rs` 788-792 and 801;
  - `sink.rs` test lines after 3511;
- doubles' methods a test does not call (`sink.rs` 3706-3716).

Of all the test code left uncovered, these are this branch's own (its
added lines, intersected with r5's report):
- `verdict.rs` 4219 and 4227: the `other => panic!` arms of the
  `corrupt_why` and `unknown` helpers.
- `verdict.rs` 4637 and 4974: one short-circuit side each of two `.find`
  predicates in the re-applied cloud tests, which no probe of those tests
  takes.
- `file.rs` 10095: an assertion's `matches!` guard, false only when it
  fails.
- `sink.rs` 6934: the `reaped()` deadline, passed only when the reaper
  never finishes.

No production line or branch this branch added is uncovered.

## Left

- **Mutation runs**: finish B and C, then re-test every survivor on the
  final tree with the full suite (no skips but `cited_commits`). At 22:03:
  - **Run B, done.** Its last resume (`--iterate`, on `af6423fe`'s tree)
    tested the 530 not already caught or unviable: 456 caught, 64
    unviable, 8 missed, 2 timeouts.
    - The 8 missed are all argued equivalents: `MAX_ROW_LEN` ×3,
      `O_SYNC.is_none()`, `flush_all`, `writer_flags` ×2 and
      `Code::worse`.
    - The 2 timeouts are the `first_at_or_after` pair above.
    - The other 715 of the 1,245 were caught or unviable in its earlier
      legs.
    - **Re-tested with the full suite** (no skip but `cited_commits`, on
      `1f102a94`): 16 mutants matched, 8 caught and 8 missed.
      - Caught: all 6 that met the full disk (3 of them had been recorded
        unviable), and 2 more of `DayBlock` that the patterns also
        matched.
      - Missed: exactly the 8 argued equivalents, so no skipped test
        catches any of them.
  - **`last_before`'s widened bisection** (run C, timeouts): `a < b` → `<=`
    never ends, and the midpoint's `+` → `*`, `/` → `%` and `-` → `+` end
    late or not at all. Two base tests in `sealed_tests.rs` had no bound:
    `a_widened_superblock_bisects_in_a_bounded_number_of_reads` and
    `a_lookup_at_the_edge_of_a_superblock_and_a_block_is_exact`. They now
    run on a worker given a minute (each takes 0.4 s), so the lib suite
    fails in 64 to 128 s under those mutants (`1f102a94`). Run C was
    restarted on that tree.
  - **`audited`'s loop `while chunk < n_blocks`** (run C, timeouts):
    - Made `==`, an empty month's audit never ends: `count` is 0, so
      `chunk` never moves. Run alone, only `an_empty_month_is_a_head_alone`
      loops, and it held the sink. Its body now runs on a worker given a
      minute, with the sink held on the test's own thread, so the lib
      suite ends in 70 s with 18 tests red (`349a8a34`).
    - Made `<=`, every audit of a non-empty month loops at its end, so
      each of the 14 audits in `sealed_tests.rs` hangs. That is
      **detected, not survived**. These lines are unchanged on this branch
      (gate 18 does not mutate them), and bounding every audit of the base
      tests is beyond this pass. Measuring every hanging test one by one
      under this mutant ended in the container's restart.
  - **The container restarted at about 23:30.** Run C stopped part way and
    resumed with `--iterate`. Nothing committed was lost.
  - **Run C, done** (`sealed.rs`, `sealed_codec.rs`, 910 mutants, five
    legs). The last leg, on `349a8a34`'s tree, tested the 232 not already
    caught or unviable: 217 caught, 9 unviable, 5 missed, 1 timeout.
    - `Head::check` → `Ok(())` and its `||` → `&&` were missed. Run C skips
      `a_misfiled_month_is_refused` and
      `a_length_that_is_not_the_heads_is_refused` for speed, and those two
      exist to kill both. The misfiled test moves the month within its
      year and the year within its month.
    - `Fold::push` `<` → `<=` was missed, an argued equivalent. The guard
      never stops the loop, and `finish` truncates the table to
      `table_rows` anyway.
    - `last_before`'s `block > 0` → `true` and `>` → `>=` were missed,
      argued equivalents: the guard runs only for `index ≥ 1`, which
      needs `block ≥ 1`.
    - The 1 timeout is `audited`'s `<` → `<=` (below).
    - **Re-tested with the full suite** on `349a8a34`, with the one
      full-disk catch (`frames` → `Ok(())`): 6 tested, 3 caught (both
      `Head::check` mutants and `frames`), 3 missed (exactly the three
      equivalents).
  - **The disk allowance filled at about 22:11.** Tests that fail under a
    mutant leave their scratch directories behind: 11 GB of them in run
    B's TMPDIR and 4.8 GB in run C's. Run C stopped on its own log write.
    - Freed by removing scratch left for over 15 minutes, never a
      cargo-mutants copy. A sweeper now does this every 5 minutes while a
      run lasts.
    - Every outcome recorded in that window was checked. Only the logs
      that show the host's `os error 28` are suspect: 6 of run B's
      (`verdict.rs` 3081:5, 3084:21 ×2, 3084:32, 3108:21 ×2: 3 caught, 3
      unviable) and 1 of run C's (`sealed.rs` 2156:9 caught).
    - These 7 are re-tested with the full suite. No other log of the
      window holds the error, so the others failed for reasons of their
      own. The two unviables of 22:12 (`verdict.rs` 3094:5) are a real
      compile error: `File: Default`.
- **Coverage**: a final run on the final tree.
- **Equivalent mutants** still argued, each to be confirmed by the runs:
  - `verdict` `Code::worse` `<` → `<=`.
  - `verdict` `Series::feed_byte` → `Ok(0)` and `Ok(1)`.
  - `sealed` `Fold::push` `<` → `<=`. The table-length guard never stops
    the loop: after `entries` pushes, `next_start` is at or past `until`,
    which is past every stamp.
  - `sealed` `last_before` guard.
  - `file` `MAX_ROW_LEN` constants.
  - `file` `O_SYNC.is_none()` and `writer_flags`, which are other targets'
    arms.
  - `file` `flush_all` `<` → `<=`.
  - `sink` `mark_coverage` `>` → `>=`.
- **Gates**: the workspace test run, the CI gate 20 count, and the final
  report.

## Gates (so far)

| gate | result |
|---|---|
| `cargo fmt --check` | clean at every commit |
| `cargo clippy -p store -p telemetry --all-targets -- -D warnings` | clean at every commit since `2e6cf25c`. Commit `a84f53b5` carried one clippy error, fixed in the next commit |
| `cargo test -p store -p telemetry --lib` | pass at every commit since `0b04e863` |
| `cargo deny check` | `advisories FAILED` on RUSTSEC-2026-0344 (`ruzstd`) and RUSTSEC-2026-0346 to 0349 (`rustls`), and nothing else (bans, licenses and sources ok). This is the base's set. `Cargo.toml`, `Cargo.lock` and `deny.toml` are identical to `feat/db-fix` |
| `cargo test --workspace --locked` | On `dff7da8e`, as root with the capabilities dropped, in 85 min: 161 of 162 targets green. The one red target is `store --test cited_commits` (`every_commit_the_store_records_cite_is_on_main_or_says_why_not`), which is also red on the base and is BASE-CITED's to fix. To be re-run on the final tree |
| CI job `language-purity`, every `run:` step but 1e and 14, run locally through `.github/checks_all.rs`'s plan | 24 green. 8 red: gates 0, 1d, 10, 11, 12, 19, 23 and 30. **All eight are red identically on the base**, run the same way in a worktree of `origin/feat/db-fix`. Their violation lines, sorted and stripped of line numbers, are the same set, so this branch adds none; only the file and function counts differ. The causes are in `api`, `pull`, `web/` tests, `config/store-steps.toml` (R3-18 merged), and the base's `sealed.rs`, `sink.rs` and `settings.rs` lines |

## NEEDS THE MAC

- **CI gate 20** (`ci.yml`, the exact count of uncovered lines per telemetry
  file in a workspace profile): `sink.rs`'s count moves with this branch.
  - The re-applied 16 Oct tests and this pass's tests cover lines that it
    declared, and they add test-module lines.
  - The declared `sink.rs 20` must be re-measured with
    `cargo llvm-cov --workspace` and `report --text`, as CI does with
    cargo-llvm-cov 0.8.4.
  - The prose in `docs/06-limits.md` §54 must be updated to match.
  - The citations in `docs/06-limits.md` are BASE-CITED's to edit, so this
    branch does not touch the file.
- `store tests/cited_commits.rs every_commit_the_store_records_cite_is_on_main_or_says_why_not`
  fails here as it fails on the base. It is BASE-CITED's to fix, and this
  branch does not edit it.

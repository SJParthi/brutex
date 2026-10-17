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
- `docs/04-invariants.md` CMS-01 to CMS-08.

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

## Left

- **Mutation run B** (`file.rs` + `verdict.rs`) is running. Still to run:
  `sealed.rs`, `sealed_codec.rs`, `sink.rs`, and a final re-test of the
  survivors on the final tree.
- **Coverage**: a final per-binary union run. The production lines that
  remain uncovered on this tree, each with its reason:
  - **`file.rs` defensive re-borrows**, after the caller checked the same
    value: 3345, 3359, 3463, 3542, 3553, 3570, 3573 (each a
    `let Some(tix) = self.tix.as_ref() else`).
  - **`file.rs` 3370-3380 and 3488-3502**: `time_index::extend` refusing
    `OffGrid` or `Outside`. Admission has already refused those bars on
    the same grid (`Geometry::new` uses `Admission`'s `OPEN_ANCHOR_MICROS`
    and the same month bounds). `Err(other)` there is unreachable, because
    the stamps iterator never errs.
  - **`file.rs` 3483-3486**: `SharedSlot` at the threshold. An intraday
    slot holds one bar, and a daily month never reaches 1,024 bars.
  - **`file.rs` 2582, 2588, 3791, 3801-3803**: geometry refusals of a block
    range computed from the same layout, and a buffer that a `const`
    assertion sizes.
  - **`file.rs` 3937, 3989, 4419-4422**: the overlap walk past
    `held_through`, an empty month at `suffix_that_follows` (admission
    refuses first), and `index_refused`'s non-`Unreadable` arm, which only
    the arms above reach.
  - **`safe_open.rs` 107**: a file type Linux does not have.
  - **`safe_open.rs` 300-306**: an arm for targets without verified flags.
  - **`verdict.rs` 717**: a vendor table that disagrees with its
    declaration.
  - **`verdict.rs` 3924, 4076, 4137**: `handle` answering `None` right
    after `prefix` cached it.
  - **`sink.rs` 3408**: a posting path with no parent.
  - **`emits.rs` 788-792, 801**: test-only assertion messages inside a
    `#[test]`.
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
| `cargo test --workspace --locked` | running |
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

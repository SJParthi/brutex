STATUS: IN PROGRESS

# Cloud mutation and coverage pass: store (2026-10-17)

Branch `cloud/mut-store-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584`. Continues `cloud/mut-store-20261016` (its report:
`reports/cloud-mut-store-20261016.md` on that branch).

## Done so far

- **Re-applied the 16 Oct work** onto the newest `feat/db-fix`:
  `git diff c9e3472e5 origin/cloud/mut-store-20261016 -- . ':(exclude)reports' | git apply -3`.
  All eight files applied cleanly. No conflict, and no `feat/db-fix` change
  dropped.
- **Production changes**, each small and in `crates/store`:
  - `staging::create`: a look that catches a file mid-unlink (`nlink` 0) is
    a free name, not `staging_linked` "one of 0 names of a file". This was
    the 16 Oct finding.
  - `file::seal_committed`: a refused write of the checksum sidecar now
    names the sidecar. It named the bar file, which the host had not
    refused. Found by this pass.
  - `sort_index_writer::write_whole`: its `n_valid == 0` guard was
    unreachable (its only route returns first for an empty month), so it is
    removed, and the function returns `()`.
- **Test seams, all `#[cfg(test)]`**, in the codebase's existing shape
  (`file::sync_hooked`, `flock::unlock`, verdict's crash seam). Each one adds
  a branch under test, so the line that ships is the line every test runs.
  - `file::fault` and `impl Positional for File`: one per-thread host
    refusal, armed for one action on one file. The file is matched by path,
    or by device and inode for an open handle.
  - `staging`: a stand-in for the look, and an open refusal.
  - `sort_index_writer`: its index looks and reads go through the file seam.
  - `verdict`: an interference seam, which runs another actor's step before
    a host call.
  - `flock::Flock::try_lock`: a one-shot lock refusal.
  - `safe_open::name_the_refusal`: a look counter.
- **New tests.**
  - `file_fault_tests.rs` (14).
  - `staging_tests.rs` (+4; the leftover test is fixed for ext4).
  - `verdict_tests.rs` (+6) and `verdict::cloud_mutant_tests` (+1).
  - `safe_open_tests.rs` (+1).
- **Equivalents from the 16 Oct pass that the new tests kill**, by direct
  test (pending a confirming `cargo mutants` run):
  - `staging.rs` 254 and 263, now driven through the look and open seams;
  - `safe_open.rs` 310, through the look counter;
  - `sort_index_writer.rs` 651 and 661, through the look seam;
  - `verdict.rs` 3016, 3080, 3102 and 3277, through the interference seam;
  - `file.rs` `refuse_link` and `Gathering::drop`, called directly against
    their contracts.
- **Findings fixed.**
  - The `staging_tests.rs` and `safe_open_tests.rs` scratch directories are
    now cleared first, so they are hermetic.
  - The staging leftover test no longer fails on ext4's inode reuse.

## Left

- Run `cargo mutants` over every touched file, and re-test the survivors on
  the final tree. Run A is in progress. Its first pass used a snapshot in
  which a new test could disturb the `emits` test in parallel (fixed in
  `572fe3ee`), so its catches are checked log by log.
- `sink.rs` coverage, the branch gaps, the remaining equivalents, the
  `fold_extremes` citation, the decisions entry, and the gates.

## Gates

- `cargo fmt --check` and `cargo clippy -p store --all-targets -D warnings`
  are clean at each push.
- `cargo test -p store --lib` passes (305 tests, three runs in a row).
- The full gates have not been run yet.

## NEEDS THE MAC

- Nothing yet.

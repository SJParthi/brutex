# Store survivors from the cases CI never tested (run 1283)

Run on the coordinator's machine at 969493e1 with CI's mutation flags, `--in-place` in a
world-readable worktree so the uid 65534 re-exec tests run. Ledger:
/tmp/claude-0/mut-untested/store.done. At 07:46 UTC, 47 of 52 were done: 40 caught,
4 MISSED, 3 unviable. The other 5 are re-running in the resumable queue.

MISSED (owner: Gate 18 rest fixer):

- crates/store/src/file.rs:3752:22: replace match guard gone.kind() == ErrorKind::NotFound with true in remove_index
- crates/store/src/file.rs:3752:22: replace match guard gone.kind() == ErrorKind::NotFound with false in remove_index
- crates/store/src/file.rs:3994:21: replace match guard why.kind() == ErrorKind::NotFound with true in refuse_if_sealed
- crates/store/src/file.rs:4013:63: replace && with || in missing_below

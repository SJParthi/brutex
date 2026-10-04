# Rust and O(1) sweep (workstream 5.4) — status 2026-10-04

Thread: "Rust and O(1) sweep". Decision range D-2300..D-2399, invariant prefix AFG-.
Board: https://claude.ai/artifact/TjTxZFcK2XF7q8hFbm6Ryn ; Fix Board status file
/mnt/project-files/fix-board/status/sweep.tsv.

## Landed on final/all-fixes (PR #74), pushed at 8c16ba3
- w9: gaps-5, gaps-11, gaps-10 (D-1576..D-1578).
- OS-1..3 pool-oos streaming (D-2300), OS-4 union opens ledger once (D-2301),
  swept_rung count (D-2302), OS-5 selection-v6 paging (D-2303).
- OE-5 O(1) depth (D-2304), OE-2 one bootstrap buffer (D-2305), OS-8 (D-2306),
  OE-3 per-day window tables (D-2307), OE-4 sized heaps (D-2308),
  OS-6 detail slot lock (D-2309), OS-7 one read per row (D-2310; still O(runs):
  runs.bin v3 has no index, a new format version would be needed),
  OE-1 exact prefix sums over resample runs, receipts bit-identical (D-2316).
- OD-1..7 data side (D-2370..D-2376).
- RO-1..RO-10, rustonly2-5, forbid(unsafe) on tools (D-2340..D-2350).
- ci.yml awk/sed/jq -> Rust: .github/gates_tree.rs (D-2311), gates_runtime.rs
  (D-2312), gates_ledger.rs (D-2313, Gate 11 allowlists now live here),
  gates_bounds.rs (D-2314), gates_jobs.rs (D-2315, Gate 8 self-test D-2317).
  AWK_IN_CI ratchet = 0. ci.yml 472,900 -> ~277 KB.

## Owner-blocked (named, not guessed)
gaps-1, gaps-3 (D-1568, D-1544), gaps-6, gaps-7, gaps-8, hunt-costs-5,
hunt-runner-5 (D-1549), hunt-ci-1 (branch protection; code half on head), testgaps-7.

## Known local-environment test failures (not code)
/dev/null in the cloud box is a 48-byte root-owned regular file, so tests that
open /dev/null, fifos or spawn with Stdio::null fail as uid 65534.

## Open follow-ups
- OS-7: `cli top` O(1) via a sealed best-row sidecar is in progress on branch sweep/os7-index (D-2319); `cli results` stays O(runs) (arbitrary filters).
- Stale doc quotes of removed shell: docs/04 X-05 (gate 7), docs/06 ~1859 (gate 1d).
- Gate 22 next-line include hole: closed (D-2318).

## Update 2026-10-04 13:45Z (paused for usage)

- final/all-fixes is at bb6b3b4 (8c16ba3 plus the CI thread's D-1464 batch).
- OS-7 is closed as a documented decision (D-2319, AFG-19): an O(1) one-row index for `cli top` would lose its refusal of a damaged row it does not name, and would still leave the frontier and receipt passes.
- The OS-7 merge onto bb6b3b4 is b97b2ca, saved on claude/project-thread-v8j0jv and NOT yet on final/all-fixes.
  - All static gates and fmt pass on it.
  - Workspace clippy and the cli lib tests were still running when paused.
- Next step: on b97b2ca, run `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test -p cli --lib`; the one expected failure is the /dev/null one. Then merge the current origin/final/all-fixes and push to final/all-fixes. Set OS-7 to doc at b97b2ca in fix-board/status/sweep.tsv and republish the board artifact.

- 13:58Z: b97b2ca pushed to final/all-fixes after workspace clippy clean and cli lib 1736/1737 (only the /dev/null environmental failure). Remaining: republish the board artifact.

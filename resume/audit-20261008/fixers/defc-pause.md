# defc pause note (worktree /home/claude/wt-dc, branch audit/def-c, base a7a27dc3)

| item | state | decision | commit |
|---|---|---|---|
| 1 AC-whp-o1-1 (resume restores whole history) | DONE | D-4520 | b642cd7d |
| 6 W3-runner2-3 (OOS witness re-hashes slices) | WIP (compiles, untested, undocumented) | D-4521 (reserved, not written) | aa55d682 (WIP) |
| 2 cli-14 / W2-cli12-3 / W2-cli12-4 | NOT-STARTED | (D-4522 planned) | — |
| 3 W2-cli6-0 | NOT-STARTED | (D-4523 planned) | — |
| 4 W2-cli2-5 | NOT-STARTED | (D-4524 planned) | — |
| 5 W2-cli14-1/2/3 | NOT-STARTED | (D-4525 planned) | — |

Item 1 checks run: engine tests 19 ok (+1 ignored measurement, run: 5,173,280 vs 14,681,072 level bytes); cli `and_checkpoint` 24 ok; vocab stale_claims ok; clippy engine+vocab clean; fmt clean; gates-doc 10/10b/27/27b OK. NOT run: cli clippy, full cli test suite as uid 65534, Gates 11/12.

Resumer, item 6 next command:
`CARGO_TARGET_DIR=/home/claude/t-dc CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 cargo test -p cli --lib --locked -- strict_v6_one_oos_fold_serves_every_witness global_replay_oos_source_identity`
then write D-4521 (hoisted `run_digests`; per-mint `require_integrity` dropped on D-4468's argument; `require_integrity` now recomputes `ExecutionDigestsV1` and compares both), invariant DEFC-03, and amend docs/06-limits.md D-1143 "Sealing" bullet. Items 2–5 plan: no O(1) fix found yet; intended route is `#[ignore]`d latency tests (api::latency pattern, D-4434) recorded in docs/06-limits.md. No files half-edited; no cargo process of mine running.

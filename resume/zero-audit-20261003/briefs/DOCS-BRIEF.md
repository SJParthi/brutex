# Documents that state what the code no longer does: fix brief

You are fixing confirmed documentation findings in the brutex repo. CLAUDE.md is the law: §3 rule 1 (no invention: every claim traceable; if unsure write UNVERIFIED), §3 rule 6 (never claim a measurement you did not take). Rust only for code.

## Where to work
- Your own git worktree (isolation). Local branch `zero/doc-truth` from the commit you start on. Do NOT push, do NOT open a PR, do NOT create git tags. The main session merges.
- `CARGO_BUILD_JOBS=2`. /mnt/project-files is shared: read only. Install nothing.

## Findings (evidence in `/mnt/project-files/zero-rounds/tests-docs-security.md`, `### P1-...` sections)
- P1-16-04: add byte layouts to `docs/02-store-format.md` (new sections, or a linked sub-document) for `results/runs.bin` (`BRUTEXRS`, crates/cli/src/results.rs, versions 2 and 3), population rows (`BRUTEXPP`, population.rs), live (`BRUTEXLV`, live.rs), Pre-Admission Data V2 (pre_admission_data.rs), Execution V3 and V4 (execution_v3.rs, execution_v4.rs), Global Replay V3 (global_replay_v3.rs), Population Statistics V3 (population_statistics_v3.rs). Derive every offset from the encoder code, not from comments. For each new section add a test beside the code (like `cli::frontier::tests::the_store_format_doc_states_the_frontier_this_build_writes`) that binds the section's magic, version and stride to the constants.
- P1-18-01: docs/09-verify.md procedure must pass at HEAD (allow `crates/cli/build.rs` by name; point §4/§5 at pages that exist; assignment-only regex; correct counts). Run every command it lists and paste nothing you did not run.
- P1-18-02: docs/10-shared-core.md §3 counts (twelve sources, 328 positions), or cite the test instead of a literal.
- P1-18-03: the `tag = "vocab-v0.1.0"` snippet: do not create the tag. Mark the snippet as not yet possible (no tag exists) and say what a consumer can do today.
- P1-18-04: docs/07-plan.md hashes that resolve nowhere: replace each with the decision that carried the change where you can find it by grep in docs/05-decisions.md, otherwise say plainly the commit is not in this repository's history. Then add docs/07-plan.md to `crates/store/tests/cited_commits.rs`'s reading list so it stays true.
- P1-18-05, P1-18-06: docs/07-plan.md §5, §6, §7.1, R-9 against the current code and docs/04 rows.
- P1-18-07: docs/07-o1-architecture.md layer 4: state both the hit and miss bounds from `core::universe`'s tests.

## Ledger
- Decisions D-1940..D-1959 only, appended at the end of `docs/05-decisions.md` as `### D-NNNN — title — 2026-10-03`. Invariant rows (only for new tests) prefix `ZD-` in docs/04-invariants.md with full test module paths.
- Before finishing: `cargo fmt --check`, clippy `-D warnings` on touched crates, and the touched tests (including `cargo test -p store --test cited_commits`) pass.

## Return
Finding ID, fixed or not, commit, test name. Under 50 lines.

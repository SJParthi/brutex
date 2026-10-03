# Ledger torn-tail and rollback cluster: fix brief

You are fixing one cluster of confirmed audit findings in the brutex Rust workspace. CLAUDE.md in the repo is the law: read it first. Rust only.

## Where to work
- Work in your own git worktree (you were started with isolation). Base: branch `final/all-fixes-zero`. Commit on a local branch named `zero/ledger-tails`. Do NOT push. Do NOT open a PR. The main session merges your branch.
- Build and test with `CARGO_BUILD_JOBS=2`. The box has 4 CPUs and another build may be running.
- The container runs as root. Permission tests need non-root: `setpriv --reuid=65534 --regid=65534 --clear-groups cargo test ...` (use a target dir that user can write, or chmod), or skip only those tests and say so.

## Findings to fix (full evidence in these files; read the named sections)
- `/mnt/project-files/zero-rounds/concurrency.md`: cli2-1 (line ~1297), cli3-3 (~1503), pop1-1 (~1557), pop1-2 (~1606; REFUTED for V5 at ~2979, still check V6 and Observation), pop1-4 (~1685), pop2-1 (~1753), pop2-4 (~1806), pop2-5 (~1816), sel-1 (~1870), search-2 (~1944), sweep-2 (~2486, plus the kernel note at ~2440), store1-2 (~808), resources-1 (~4532). Verification verdicts are at ~2558, ~2727-2990, ~3127, ~4688.
- Slice reports in `/mnt/project-files/zero-rounds/r1-slices/`: slice23.md F1 (Admission V3 mid-block orphan), slice24.md F1, F2, F3 (Finalization V3/V4, Statistics V2, Observation V1/V2: no rollback, no directory fsync, duplicate-identity orphan).

## The rule to implement, consistently
1. Every append to a fixed-stride ledger rolls back on any write or sync error: `set_len` back to the length before the write, then sync. If the rollback itself fails, the error names both failures.
2. A ragged tail past the last whole record (kill or power loss mid-write) is provably uncommitted when readers only expose records under a later receipt or ledger row. Where that holds, the next WRITER, under the existing lock, truncates the torn bytes and says so (a telemetry event or a returned note, never silent). Read-only opens keep refusing or report the tail; they never truncate. Where the bytes could be committed history, keep refusing and name it.
3. A failed `sync_all` is never "confirmed" by a second `sync_all` (resources-1, store1-2, pop1-4). Roll back and fail instead.
4. A whole-record orphan without a receipt (Admission V3 pop2-1, pop2-4) is scratch: a rerun under the lock may complete it if its bytes equal the prepared ones, and otherwise truncates it and rewrites. It never wedges the rung.
5. Files and directories created on the first append get a directory fsync (pop2-5, slice24-F2).
6. A trailing orphan whose identity duplicates a completed authority is refused or discarded, not accepted (slice24-F3).
Prefer one shared helper over many copies, if the crate graph allows (CLAUDE.md §5).

## Proof and ledger
- Every fix gets a test that fails before and passes after. Assert real outcomes (CLAUDE.md §4: no test that asserts nothing). Kill paths can be simulated by writing a partial record directly.
- Decision entries: use D-1900..D-1919 only. Format `### D-NNNN — title — 2026-10-03` appended at the end of `docs/05-decisions.md`. Invariant rows: prefix `ZL-` in `docs/04-invariants.md` as `| ZL-01 | must hold | full::module::path::test_name | ✓ |`, with the full module path of each test.
- Gate rules that bite: no `.iter().find(` or `.position(` (gate 11); no new local `#[allow(` on production lines; a `#[path]` test file must start with `#![cfg(test)]`; no new lowercase string literal in `crates/pull`.
- Before you finish: `cargo fmt --check`, `cargo clippy -p <crate> --all-targets -- -D warnings`, and the tests of every touched module pass. If cargo-mutants 26.2.0 is installed (`cargo mutants --version`), run it on changed lines of the touched files (`--in-diff`); otherwise say it was not run.

## What to return
A short list: each finding ID, fixed or not, the commit, the test name, and anything you could not fix and why. Keep it under 60 lines.

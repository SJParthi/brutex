STATUS: IN PROGRESS

# Cloud mutation and coverage pass — `crates/pull`, 2026-10-17

Branch `cloud/mut-pull-20261017`, cut from `origin/feat/db-fix` at `f9cdb584`.
Continues `cloud/mut-pull-20261016` (report
`reports/cloud-mut-pull-20261016.md` on that branch), whose test work was
re-applied with `git diff c9e3472e5 origin/cloud/mut-pull-20261016 -- .
':(exclude)reports' | git apply -3`: every hunk applied cleanly, no conflict,
no `feat/db-fix` line touched.

**Scope:** the same ten files — `cash_session_cache.rs`, `fold.rs`,
`gdfl_archive.rs`, `gdfl_cm.rs`, `gdfl_cm_auction.rs`, `gdfl_import_scope.rs`,
`gdfl_nfo.rs`, `gdfl_tickstore.rs`, `request_minutes.rs`, `scrub.rs`. Of
these, `feat/db-fix` changed the production code of two since `c9e3472`:
`request_minutes.rs` (R3-18: `tally`, `stored_line`, `ledger_key`,
`append_named_picking`, the `stored_short` reason) and `gdfl_archive.rs`
(`dos_time`, `raw_name`). The other eight are byte-identical in production,
and no test was removed between the two bases (every `#[test]` name of
`c9e3472` is still in `f9cdb584`; 52 were added), so the old pass's kills in
those eight files still stand.

**No production line changed on this branch.** `git diff origin/feat/db-fix`
touches only test code under `crates/pull`, one test-order override in
`.config/nextest.toml`, one gate-1d declaration in `.github/gates_tree.rs`,
and this report.

## Method

| | |
|---|---|
| Tools | cargo-mutants **26.2.0** and cargo-nextest **0.9.145** (both CI's pins), cargo-deny **0.19.0** (CI's pin), cargo-llvm-cov 0.9.1, rustc 1.97.1 (pinned) and nightly 1.101.0 (2026-10-16) for branch coverage only |
| Mutation flags | exactly Gate 18's: `--test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle --cap-lints true --cargo-arg=--locked --minimum-test-timeout 900 --timeout-multiplier 2`, `--jobs 2`; the dev profile overridden to opt-level 0, incremental, no debuginfo, for rebuild time (outcomes do not depend on it; timings do). The old pass ran `cargo test`; under nextest each test is its own process and the run stops at the first failure, which is what decides a hang (see Timeouts) |
| User | every mutation, coverage and workspace-test run is as an **unprivileged user** (`mut`, uid 30033) in a mirror of the tree, so permission-bit tests run their own path as on CI and the operator's machine; the container itself runs as root |
| Coverage | `cargo +nightly llvm-cov -p pull --branch`, production lines and branches only (each file cut at its first `#[cfg(test)]` module) |

## Done

- Re-applied the 2026-10-16 tests onto the newest `feat/db-fix`.

## Left

## Gates

## NEEDS THE MAC

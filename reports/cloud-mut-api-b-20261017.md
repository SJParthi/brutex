STATUS: IN PROGRESS

# Cloud mutation and coverage pass B, continued — `crates/api`, 2026-10-17

Branch `cloud/mut-api-b-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584` (2026-10-17 12:16 IST). It carries the 16 Oct pass
(`origin/cloud/mut-api-b-20261016`, cut from `cloud/base-20261016` =
`c9e3472e5`, the snapshot of `feat/db-fix` at `4756cce0`) re-applied with
`git apply -3`, then finishes what that pass left open.

**This report holds no authority** (`CLAUDE.md` §10): it is evidence of what
was measured on this date, on this machine.

## Scope

The 16 Oct pass's seven files in `crates/api/src`: `audit.rs`,
`autopilot.rs`, `emitted.rs`, `import_journal.rs`, `pullrun.rs`,
`scratch.rs`, `server.rs`, plus the test files it added
(`server_cloud_mutant_tests_{a,b,c}.rs`, two tests in
`census_request_tests.rs`).

`emitted.rs` and `scratch.rs` are `#[cfg(test)]` modules (`lib.rs`), so
cargo-mutants lists **no** mutant in them. Mutants listed on the new base
(`cargo mutants --list -p api -f <file>`):

| File | Mutants |
|---|---:|
| `audit.rs` | 371 |
| `autopilot.rs` | 371 |
| `import_journal.rs` | 127 |
| `pullrun.rs` | 466 |
| `server.rs` | 1,447 |
| **total** | **2,782** |

`crates/api/src/bars_route.rs`, `facets.rs` and `livejson.rs` are not
touched (the Mac's CLOUD-ATTACKO1 merge-back owns them).

## Re-apply

`git diff c9e3472e5 origin/cloud/mut-api-b-20261016 -- . ':(exclude)reports' | git apply -3`
applied to all eleven files with **no textual conflict**. `feat/db-fix` had
changed four of them since the snapshot (`emitted.rs`, `import_journal.rs`,
`pullrun.rs`, `server.rs`). Three semantic conflicts surfaced when the crate
was built and run, and each is resolved keeping both sides' intent:

1. **`pullrun::Interrupted` gained three fields** (AG-01: `open_count`,
   `also_open`, `runs_unread`). The re-applied test
   `an_fno_walk_with_no_ask_or_window_is_written_with_nulls_and_its_doubts`
   now builds the one-run-open case (`1`, empty, `0`), whose sentence AG-01
   keeps byte for byte.
2. **`run_in_over` gained a fifth parameter** (R3-14: the `Held` that keeps
   the store and the port for the caller). The four re-applied calls pass
   `&mut Held::default()`, as the existing serve tests do.
3. **R3-14 repair 5 races the start-up load against the stop**, polling the
   stop first. Five re-applied serve tests used a stop that had already
   fired, so they now ended before the load and failed. They stop after the
   load instead: `server::tests::after_the_load` waits for the `listening`
   event, which is written after the load and before `serve`; the child
   whose log cannot open has no sink, so it serves a real port and stops
   once the port answers one request (`once_answering`).

## Production and test defects found

### A `feat/db-fix` test that fails on a host with a free store (fixed)

`server::tests::run_serves_until_the_signal_and_exits_zero` serves with a
stop that has already fired. Since R3-14 repair 5 that stop wins the race
with the load, so on a host whose store root exists and no server holds,
the serve exits `0` before reading the universe and the `listening` event
the test reads is never written. **Measured** here with `BRUTEX_STORE`
naming an empty directory: `left: 0, right: 3`. CI has no store and returns
before that branch; the operator's Mac passes it only while another server
holds the store. It now stops after the load and passes both with and
without a store (commit `19398209`).

### Carried from the 16 Oct pass

- **Fixed there, kept here:** the spot run's vendor-down breaker could
  never trip (`with_retry` and `fetch_chunks` dropped the `VENDOR_DOWN`
  marker), and a lost line continuation in `price_group`'s refusal.
- **Found there, NOT fixed, needs the operator's call:** a backend failure
  the vendor names (`Disposition::RetryBounded`) under a status outside
  500–599 gets a zero-second ladder and a refusal saying the vendor failed
  "0 time(s)". Whether Dhan or Groww send those names under a non-5xx
  status is **UNVERIFIED**. (Details below when this report is final.)

## Method

- cargo-mutants 27.1.0, cargo-nextest 0.9.145 (the CI pin), cargo-llvm-cov
  0.8.4 (the CI pin), cargo-deny 0.19.0 (the CI pin), toolchain 1.97.1.
- Every test runs as the non-root user `mut`: root ignores mode bits, and
  several refusals are mode-driven.
- Worker copies of the tree, without `.git` (so `cli`'s build script stamps
  a constant and only `api` rebuilds per mutant), each bind-mounted at the
  same path `/work` in its own mount namespace so one target directory
  serves all of them.
- Mutation builds only: `api` at `opt-level = 0`, incremental, and
  debuginfo off workspace-wide, from an untracked `.cargo/config.toml` in the
  worker copies. Gates and coverage use the checked-in profile.
- Baseline on the re-applied tree, before the serve fixes: the whole `api`
  lib suite under nextest in **77 s**, 2,397 passed, 5 failed (the five
  serve tests above), 8 skipped (`#[ignore]`).

## Done

- Re-apply, the three semantic conflicts, the latent `feat/db-fix` test
  failure.

## Left

- Mutation run over the 2,782 mutants; survivors killed or explained.
- Line coverage of the five mutable files; branch coverage if measurable.
- Gates.

## Gates

- (in progress)

## NEEDS THE MAC

- (in progress)

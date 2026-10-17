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
`census_request_tests.rs`) and the one this pass adds
(`server_cloud_mutant_tests_d.rs`).

`emitted.rs` and `scratch.rs` are `#[cfg(test)]` modules (`lib.rs`), so
cargo-mutants lists **no** mutant in them, and their every line is covered.
Mutants listed on the new base (`cargo mutants --list -p api -f <file>`):

| File | Mutants |
|---|---:|
| `audit.rs` | 371 |
| `autopilot.rs` | 371 |
| `import_journal.rs` | 127 |
| `pullrun.rs` | 466 |
| `server.rs` | 1,447 |
| **total** | **2,782** |

`crates/api/src/bars_route.rs`, `facets.rs` and `livejson.rs` are not
touched (the Mac's CLOUD-ATTACKO1 merge-back owns them). `feat/db-fix`
changed other `api` files since the snapshot (`feeds_route.rs`, `speed.rs`,
`settings_route.rs`, `sweeprun.rs` and more); they are outside half B and
not measured here.

## Re-apply

`git diff c9e3472e5 origin/cloud/mut-api-b-20261016 -- . ':(exclude)reports' | git apply -3`
applied to all eleven files with **no textual conflict**. Three semantic
conflicts surfaced when the crate was built and run; each is resolved keeping
both sides' intent:

1. **`pullrun::Interrupted` gained three fields** (AG-01: `open_count`,
   `also_open`, `runs_unread`). The re-applied test now builds the
   one-run-open case, whose sentence AG-01 keeps byte for byte.
2. **`run_in_over` gained a fifth parameter** (R3-14: the `Held` that keeps
   the store and the port for the caller). The four re-applied calls pass
   `&mut Held::default()`, as the existing serve tests do.
3. **R3-14 repair 5 races the start-up load against the stop**, polling the
   stop first, so five re-applied serve tests with an already-fired stop
   ended before the load. They now stop after it
   (`server::tests::after_the_load`, which waits for the `listening` event;
   the child whose log cannot open serves a real port and stops once it
   answers, `once_answering`).

## Defects found

### Fixed, each with a test measured failing before the fix

1. **A `feat/db-fix` test failed on any host with a free store**
   (`19398209`). `server::tests::run_serves_until_the_signal_and_exits_zero`
   served with an already-fired stop; since R3-14 repair 5 that stop wins the
   race with the load, so the serve exited `0` before reading the universe.
   Measured with `BRUTEX_STORE` naming an empty directory: `left: 0, right:
   3`. CI has no store and returns before that branch; the operator's Mac
   passes only while another server holds the store.
2. **A released serve lock kept its key in a poisoned root set**
   (`0bc97f04`). `take_serve_lock` inserts through a poisoned mutex;
   `ServeLock::drop` and `release_root` skipped the removal on poison. Once
   a panic had poisoned the set, a served root stayed in it after its file
   lock was released, so the next serve of that root in this process was
   handed the in-process pass with **no file lock held**, and a second
   process could take the store beside it. Both now remove through the
   poison. Test: `server::cloud_mutant_tests_d::a_released_lock_gives_its_key_back_through_a_poisoned_set`
   (a child process); against the code before the fix its child's second
   take answered `held: None`.
3. **A doc comment called a reachable arm unreachable.** `Journal::page`'s
   doc said its `seek` refusal could not fire. `lseek` takes a signed
   offset, so a start past `i64::MAX` bytes is `EINVAL`; the doc now says
   so and `a_page_past_every_seekable_offset_is_refused_by_name` drives it.

### Carried from the 16 Oct pass

- Fixed there, kept here: the spot run's vendor-down breaker could never
  trip; a lost line continuation in `price_group`'s refusal.
- **Found there, NOT fixed, needs the operator's call:** a backend failure
  the vendor names (`Disposition::RetryBounded`) under a status outside
  500–599 is counted by neither `with_retry` nor `laddered` (both count
  `server_errors` only for `500..=599`), so its ladder waits `1000·0²` = 0
  ms five times and the refusal says the vendor failed "0 time(s)". Still so
  on this base (server.rs `laddered` and `with_retry`). Whether Dhan or Groww
  send those names under a non-5xx status is **UNVERIFIED**. The proposed
  patch (count a named `RetryBounded` with the 5xx) changes how fast this
  repository re-asks a live vendor, so it is the operator's.

## Method

- cargo-mutants 27.1.0, cargo-nextest 0.9.145 (the CI pin), cargo-llvm-cov
  0.8.4 (the CI pin), cargo-deny 0.19.0 (the CI pin), toolchain 1.97.1.
- Every test runs as the non-root user `mut`: root ignores mode bits.
- Worker copies of the tree without `.git` (so `cli`'s build script stamps a
  constant and only `api` rebuilds per mutant), each bind-mounted at `/work`
  in its own mount namespace so one target directory serves all of them.
- Mutation builds only: `api` at `opt-level = 0`, incremental, debuginfo off
  workspace-wide, from an untracked `.cargo/config.toml` in the worker
  copies. Gates use the checked-in profile.
- **Phase 1 runs the WHOLE `api` lib suite per mutant** (nextest,
  `--max-fail=1:immediate`), with a per-file nextest tool config that only
  ORDERS the file's own tests first (no test is skipped). Under three
  concurrent copies the unmutated suite passed 2,402 of 2,402 in ~85 s, so
  no load-flaky test was found to skip. Per mutant: ~20 s build, 0–4 s to
  the first failure.
- **A killed build is not an unviable mutant.** One build was OOM-killed
  (four concurrent `api` builds); its mutant (`audit.rs:529:14`) was logged
  unviable and is re-run by name. Dev builds now wait for a window with at
  most one mutation build running.
- Coverage: `cargo llvm-cov nextest -p api --lib --branch` with
  `RUSTC_BOOTSTRAP=1` on the pinned toolchain (branch coverage is a nightly
  flag; this is measurement only, nothing is committed that needs it).

## Results so far

| File | Mutants | Caught | Missed | Timeout | Unviable |
|---|---:|---:|---:|---:|---:|
| `import_journal.rs` | 127 | 116 | **0** | 1 | 10 |
| `audit.rs` | 371 | (running) | 6 so far, all equivalent | | |
| `autopilot.rs`, `pullrun.rs`, `server.rs` | | queued | | | |

Coverage, first run on the re-applied base (before this pass's tests):
uncovered production lines / lines with an unhit branch side — `audit` 5 / 3,
`import_journal` 1 / 1, `autopilot` 29 / 14, `pullrun` 34 / 32, `server`
232 / 49. Each is being driven or explained; see below when final.

## Done

- Re-apply, its three conflicts, the latent `feat/db-fix` test failure.
- Two production fixes above, each with its proving test.
- New tests (17 Oct): `audit` 4, `autopilot` 4, `pullrun` 9,
  `server_cloud_mutant_tests_c` 3, `server_cloud_mutant_tests_d` 17.

## Left

- Finish phase 1 (`audit`, `autopilot`, `pullrun`, `server`), then phase 2:
  every survivor and every mutant in a line this pass changed, re-run on the
  newest source against the whole suite.
- The remaining coverage gaps, driven or explained.
- Gates, final.

## Gates

- `cargo deny check`: **equal to the base**. Both fail `advisories` only, on
  RUSTSEC-2026-0344 (`ruzstd`) and RUSTSEC-2026-0346, -0347, -0348, -0349
  (`rustls`); `bans`, `licenses`, `sources` ok. No manifest or lockfile is
  touched (DENY-1 owns them).
- `cargo fmt --check` clean on every commit.
- clippy and the workspace tests: final run pending.

## NEEDS THE MAC

- Nothing yet in `bars_route.rs`, `facets.rs` or `livejson.rs`.

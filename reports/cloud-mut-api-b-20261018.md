STATUS: IN PROGRESS

# Cloud mutation and coverage pass B, finished — `crates/api`, 2026-10-18

Branch `cloud/mut-api-b-20261018`, cut from `origin/cloud/mut-api-b-20261017`
at `6a957d3e` (the 17 Oct pass's last push), with `origin/feat/db-fix` at
`67c4e58d` merged in at `19e2c2f9`.

**This report holds no authority** (`CLAUDE.md` §10): it is evidence of what
was measured on this date, on this machine. Every number below was measured
in this session unless it says it was carried.

## Carried

The 17 Oct report (`reports/cloud-mut-api-b-20261017.md`) is kept as it was.
Its scope, its five fixed defects, its carried 16 Oct findings and its
phase-1 results for `import_journal.rs` and `audit.rs` are carried, not
re-measured, except where phase 2 below re-runs them. Its 214 of 371
`autopilot.rs` results are **not** carried: no record of which 214 survived
the session, so every `autopilot.rs` mutant is run again here on the merged
tree.

## Merge

`origin/feat/db-fix` moved 16 commits since the 17 Oct pass merged
`dd7868a5` (GF-1, BASE-CITED, AH-02, R3-09 repair 1, R4-06 repair 1b, AG-04,
DENY-1, BASE-GATE10, CLOUD-ATTACKO1, R3-14). It merged with **no textual
conflict**. Of the scoped files it changed only `server.rs` (+460 / −77, the
AH-02 head deadline and the R3-09 census changes) and `emitted.rs`. After the
merge the branch differs from `feat/db-fix` only in the 17 Oct pass's
fourteen files, and `Cargo.toml`, `Cargo.lock` and `deny.toml` are
byte-identical to it.

**One semantic conflict**, resolved keeping both sides' intent (`00b32d1d`).
AH-02 (D-4975) moved the re-arm of the next head's clock from a response's
first written byte to the flush that finds every request finished, and only
after a write that moved bytes. Two 17 Oct tests
(`server::cloud_mutant_tests_a::a_direct_write_through_the_head_deadline_rearms_only_when_bytes_moved`
and its vectored twin in `_d`) asserted the old re-arm at the write and failed
on the merged tree. They now assert what both sides mean: the empty write
moves and counts nothing and its flush leaves the clock off; the real write
reports, counts and delivers every byte, the clock stays off while the
response is in flight, and its flush re-arms it a whole timeout ahead.

## Red on the base, not this pass's

**Five AH-02 tests fail on `feat/db-fix` `67c4e58d` on this Linux host**, at
the checked-in profile and at the mutation profile alike, the same way each
time:

- `server::head_deadline_body_tests::a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole`
  (`cut after 5374637 of 16777216 bytes at 1.32s`)
- `server::head_deadline_body_tests::a_chunked_body_reaches_a_slow_reader_whole`
  (`cut after 5389321 framed bytes`)
- `server::head_deadline_body_tests::a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole`
  (`cut after 5374638 of 8388608`)
- `server::head_deadline_body_tests::slow_readers_at_the_cap_are_served_whole_and_the_next_waits`
  (`cut after 3904835 bytes`)
- `server::head_deadline_body_tests::pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is`
  (`cut after 3904834 of 16777216`)

Measured on a copy of the pure base tree (every file under `crates/` equal to
`67c4e58d`) and on the merged tree, where `HeadDeadline` and
`head_deadline_body_tests.rs` are byte-identical to the base. Neither is in
this pass's scope to edit, and neither is edited.

**What decides it is the kernel's send buffer.** This host's
`net.ipv4.tcp_wmem` is `4096 16384 4194304`, the stock Linux maximum of
4 MiB. With that maximum lowered to 256 KiB (`4096 16384 262144`), all twelve
`head_deadline_body_tests` pass on the same build; restored to 4 MiB, the same
five fail again. A paced reader (64 KiB a read, 10 ms apart) keeps taking
bytes, yet the server's write sees no progress for the tests' 300 ms stall
deadline, and the cut lands after 3.9 to 5.4 MB: about one autotuned send
buffer plus what the reader took meanwhile. That fits Linux reporting a TCP
socket writable only once a large share of its send queue has drained,
**UNVERIFIED** against the kernel source. Production's stall deadline is
`HEAD_READ_TIMEOUT` = 10 s. By the same arithmetic, *an extrapolation, not a
measurement*, a Linux server could cut a steady reader slower than roughly
130–140 KB/s. The operator's server runs on macOS. CI runs `ubuntu-24.04`,
whose stock maximum is the same 4 MiB, so this gate is likely red there too.
**UNVERIFIED**: no CI run was read.

## Method

- cargo-mutants 27.1.0 (the 17 Oct pass's version; CI's Gate 18 pins
  26.2.0), cargo-nextest 0.9.145, cargo-llvm-cov 0.8.4, cargo-deny 0.19.0,
  toolchain 1.97.1. All four tools are the release binaries of those
  versions.
- Host: 4 cores, 15 GB of memory, Linux 6.18. Every test runs as the non-root
  user `mut`, because root ignores mode bits.
- Worker copies are the tracked tree (`git archive`, no `.git`, so `cli`'s
  build script stamps a constant), each bind-mounted at the same path in its
  own mount namespace so all of them reuse one prebuilt target. Three workers,
  each mutating in place, `--shard k/3 --sharding round-robin` per file.
- Mutation builds only: `api` at `opt-level = 0`, incremental, debuginfo off
  workspace-wide, from an untracked `.cargo/config.toml` in the worker copies.
  Gates use the checked-in profile. An incremental `api` rebuild takes about
  32 s and peaks at 2.8 GB.
- **Phase 1 runs the WHOLE `api` lib suite per mutant** (nextest
  `--max-fail=1:immediate --no-tests=fail`, `--cap-lints true`), with a
  per-file nextest tool config that only ORDERS the mutated file's own tests
  first.
- **The mutation runs use `tcp_wmem` max 256 KiB**, set on this container
  for their duration, so that the five tests above run green and kill
  mutants as they do on the Mac, instead of failing every mutant. The final
  gates run at the stock 4 MiB as well.
- **One test is excluded from the mutation runs, and only from them:**
  `head_deadline_body_tests::a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole`.
  With `api` at opt-level 0 it fails on the unmutated tree, every time, even
  run alone and with the small send buffer. Its second request, sent after the
  16 MiB body is checked byte by byte, gets nothing back. The likely cause is
  that the check of 16 Mi bytes, unoptimised, outlasts the test's 300 ms head
  deadline. It passes at the checked-in opt-level 3. A test red on the
  unmutated tree cannot tell a mutant from the original, so it is left out of
  those runs. Every `HeadDeadline` survivor is re-run against the whole suite
  on the checked-in profile in phase 2.
- `--timeout 600 --build-timeout 1200`. A disk watchdog logs free space every
  minute and removes `/tmp/brutex-<pid>-*` fixtures whose process is gone.
- **A flaky test killed mutants it cannot see, so every run retries.** The
  first `autopilot.rs` run had no retries. Three mutants the 17 Oct pass had
  argued equivalent came back "caught", and the log of each named its killer:
  `server::head_deadline_body_tests::slow_readers_at_the_cap_are_served_whole_and_the_next_waits`,
  an AH-02 timing test that serves a router with no autopilot in it. Under
  three workers' load it failed on seven `autopilot.rs` mutants, and one
  `census_cache` timing test failed on one more. A real kill fails on every
  attempt, and a flake does not. So the workers were stopped (their sources
  checked byte-identical to the tree they launched from) and restarted with
  nextest `--retries=2`. Every `autopilot.rs` mutant whose killer was a test
  outside `autopilot`'s own modules (20 of them) was run again that way
  (`p1b`). cargo-mutants 27.1.0 does not apply `-F` to "delete field"
  mutants, so nine of those were run again too; all nine were caught.

## Left (in progress)

- Phase 1: `autopilot.rs` (371), `pullrun.rs` (466), `server.rs` (1,509),
  every listed mutant on the merged tree. `autopilot.rs` is done (below);
  `pullrun.rs` and `server.rs` are running with retries.
- New tests so far: `server_cloud_mutant_tests_e.rs` (5), for AH-02's
  `HeadDeadline` and `exchange_watch` arms that a coverage run of the merged
  tree left unexecuted. `cargo clippy --workspace --all-targets -- -D
  warnings` is clean at `f9a7c020`.
- Phase 2: every survivor and every mutant in code changed since phase 1's
  base (`audit.rs`'s `Journal::page` doc and `Journal::identity`, changed
  since `bdcfa9bd`).
- The final coverage run; gates.

## NEEDS THE MAC

- Confirm the five `head_deadline_body_tests` above pass on macOS at
  `67c4e58d`, and decide whether a stock Linux send buffer is a case the
  server must meet (CI is Linux).

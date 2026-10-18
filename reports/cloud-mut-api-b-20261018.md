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
- **Retries were not enough for one test, so it is excluded from the
  mutation runs that started after 11:30 UTC.** In the retried re-check,
  `slow_readers_at_the_cap_are_served_whole_and_the_next_waits` again
  "caught" `autopilot.rs:2785:38`, and in phase 2 `audit.rs:535:16` (a loop
  inside a `const` block that no running test can reach). It failed all three
  attempts each time: twice no response head within its 10 s, then a cut.
  Under three workers' load it is red, not flaky. From then on it is left out
  of the mutation runs the same way as the opt-level-0 one (every
  `server.rs` run and the phase-2 re-checks). Both mutants it "caught" were
  re-run without it and survived. Every kill it made in the runs before is
  re-checked, and phase 2 re-runs `HeadDeadline`'s survivors against the
  whole suite, both tests included, at the checked-in profile on an idle
  host.

## Phase 1 results

| File | Mutants | Caught | Missed | Timeout | Unviable |
|---|---:|---:|---:|---:|---:|
| `autopilot.rs` | 371 | 305 | 7 (+1, see below) | 0 | 58 |
| `pullrun.rs` | 466 | 432 (+2, see below) | 2 | 1 | 29 |
| `server.rs` | 1,509 | queued | | | |

`autopilot.rs` counts its first run with the 20 out-of-module kills replaced
by their retried re-check (`p1b`). Of those 20, 13 were caught again by a
test that calls the mutated code (the emit-site test, `hand_stop_tests`,
`fno_boundary_tests`, `recovery_control`, and `pullrun`'s callers of
`credential_fault_in_page`), and 6 now survive. One more,
`autopilot.rs:2785:38`, was "caught" again only by
`slow_readers_at_the_cap_are_served_whole_and_the_next_waits`, failing all
three attempts under load. It is counted apart and re-run in phase 2 without
that test. Every unviable mutant's log names a compile error: 55 `E0277`
(the return type has no `Default`), one `E0080` (a constant shift that
overflows) and two `||` inside a `let` chain.

**Every `autopilot.rs` survivor is equivalent.** Each is a strict comparison
turned non-strict where both arms give the same value on the tie:

- `671:13` (`probe_secs`) and `1527:17` (`FeedState::backoff_secs`),
  `>` → `>=`: when the shifted wait equals the ceiling, both arms return the
  ceiling.
- `863:25` and `864:22` (`month_span`), `941:32` (`next_window`'s
  `resume < from`), `<` → `<=`: a max or a min of two `Day`s, which are the
  same `Day` on a tie.
- `2785:38` and `2802:27` (`frontier`), `>` → `>=` over `ordinal(..)`:
  `ordinal` is `year·12 + month − 1`, injective, so a tie is the same month.
- `3029:19` (`fly`), `waited > 0` → `>=`: with `waited == 0` the mutant calls
  `nap(&site, 0)`, whose loop over `(1..=0)` is empty. It returns at its first
  poll, sleeping and publishing nothing.

The 17 Oct pass had found four of these (671:13, 863:25, 864:22, 1527:17) in
its 214. Three of those four came back "caught" here before the retries,
each by the flaky test.

**`pullrun.rs`** ran with retries. Every kill but two came from `pullrun`'s
own test modules. The two others came from the load-red
`slow_readers_at_the_cap_are_served_whole_and_the_next_waits` and are
counted apart:

- `2371:58`, `*at > other.ordinal` → `>=` in `decorate` (the
  `CLOSE_UNDECODED` doubt). Re-run on the newest source without the timing
  tests, it is **caught** by this pass's
  `pullrun::cloud_tests_1018::damage_casts_doubt_only_on_a_run_opened_before_it`.
- `2449:45`, `*at < oldest` → `<=` in `guessed_answer`'s count filter.
  Re-run, it **survives**, and it is **equivalent in every state the code
  writes**. The tie is a record that would not decode, at the very position
  of the opening record of a counted pairing's oldest run, which the walk
  decoded. Through `restart_scan` only an index whose `runs` and `faults`
  files both name one position reaches it, and the builder files each record
  once. It is not pinned by a test. At that tie the code drops the count
  (claims less), while its comment says only damage AFTER the oldest opening
  record drops it, so a test would pin a side the comment does not state.
  Said here for the operator; nothing is changed.

The two survivors are real gaps, each closed by a new test that fails on the
hand-applied mutant and passes on the original:

- `996:24`, `unnamed > 0` → `>=` in `Interrupted::others_words`: the refusal
  would say "0 more run(s) read have no closing record and are counted, not
  named". The 17 Oct test held the same guard in `logged` only.
- `2360:49`, `weak > *oldest` → `>=` in `decorate`'s `newer_may_be_open`.
  Its tie is reachable only as `2449:45`'s is, but here code and comment
  agree ("a closing record after that one"), so the contract is pinned by
  value.

The timeout is a detection: `2718:22`, the match guard `waited < waits` →
`true` in `older_runs_built`, waits for ever while another writer holds the
lock, and
`pullrun::tests::an_absent_index_is_built_by_the_scan_and_not_known_is_said_when_it_cannot_be`
hangs to the timeout instead of passing. Unviable: 22 `E0277`, five `E0080`
(the constants at lines 70–84) and two `||` inside a `let` chain.

## Left (in progress)

- Phase 1: `pullrun.rs` (466) and `server.rs` (1,509), running with retries
  (16:33 UTC: `pullrun.rs` 466 of 466 tested, `server.rs` 791 of 1,509).
  `pullrun.rs` survivors so far: `996:24` and `2360:49`, each closed by a new
  test (below).
- New tests so far, each measured failing on its hand-applied mutant and
  passing on the original where it targets one:
  `server_cloud_mutant_tests_e.rs` (5), for AH-02's `HeadDeadline` and
  `exchange_watch` arms that a coverage run of the merged tree left
  unexecuted; `pullrun::cloud_tests_1018` (3): `996:24` (`unnamed > 0` → `>=`
  in `Interrupted::others_words`, which would write "0 more run(s) …
  counted, not named"), `2360:49` (`weak > *oldest` → `>=` in `decorate`'s
  `newer_may_be_open`) and its sibling `2371:58` (`*at > other.ordinal` →
  `>=`, the `CLOSE_UNDECODED` doubt). Both `decorate` ties need an index
  whose `runs` and `faults` files name one position, which this code never
  writes. `decorate`'s own contract ("a record AFTER") is pinned by value
  instead of argued from the index's consistency.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean at
  `f9a7c020`.
- Phase 2 (the newest source, the whole suite less the one opt-level-0
  exclusion, retries): `audit.rs`'s five mutants in code changed since
  `bdcfa9bd` (`Journal::identity`, `Journal::read_index`): **5 of 5 caught.**
  The 17 Oct survivors of `audit.rs` (8) survive again and stay equivalent
  (the `const` loops at 527 and 535, `Drops::peak`'s ties), and
  `import_journal.rs:529:11` times out again (detected). Every phase-1
  survivor: after phase 1.
- The final coverage run; gates.

## NEEDS THE MAC

- Confirm the five `head_deadline_body_tests` above pass on macOS at
  `67c4e58d`, and decide whether a stock Linux send buffer is a case the
  server must meet (CI is Linux).

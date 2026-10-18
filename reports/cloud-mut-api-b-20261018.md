STATUS: DONE

# Cloud mutation and coverage pass B, finished — `crates/api`, 2026-10-18

Branch `cloud/mut-api-b-20261018`, cut from `origin/cloud/mut-api-b-20261017`
at `6a957d3e` (the 17 Oct pass's last push). `origin/feat/db-fix` was merged
in twice: `67c4e58d` at the start (`19e2c2f9`) and `7aa28d65` at the end
(`fbdff6f7`). The last code commit is `cdc81a6b`.

**This report holds no authority** (`CLAUDE.md` §10): it is evidence of what
was measured on this date, on this machine. Every number below was measured
in this session unless it says it was carried.

**In short.** Phase 1 is finished for `autopilot.rs`, `pullrun.rs` and
`server.rs`: 2,346 mutants, each against the whole `api` lib suite on the
merged tree. Phase 2 re-ran every survivor on the newest source, and every
mutant in code changed since phase 1's base. Every survivor is closed by one
of the 19 new tests, argued equivalent in writing, or clock-bound and
measured caught when the clock allows. No production source was changed.
The final coverage run lists and reasons every remaining gap. fmt, clippy
and deny are clean. `cargo test -p api` fails only on six `feat/db-fix`
tests that are red on the base on this 4-core Linux host. Four 17 Oct tests
were adapted to the two merges.

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

**A second merge, at the end** (`fbdff6f7`). `origin/feat/db-fix` moved 46
more commits during the pass, to `7aa28d65` (R3-07, R3-08 and their
repairs). It merged with **no textual conflict**. Of the scoped files it
changed `server.rs` (a `count_route` layer in `admitted_gated`, the
`/pools.json` route, the `pool_table_tests` module) and `emitted.rs` (one
emit-site row). Again every `feat/db-fix` change is kept, and the manifests
are byte-identical to it. After it the branch differs from `feat/db-fix` in
sixteen files: the 17 Oct pass's fourteen, `server_cloud_mutant_tests_e.rs`
and this report.

**One more semantic conflict**, resolved keeping both sides' intent
(`cdc81a6b`). R3-08 sizes every pool by the core count under its `MAX_*`
ceiling, and gives the vendor scrub its own pool of `MAX_VERIFY_CONCURRENT`
instead of a store-read slot. Two 17 Oct tests
(`server::cloud_mutant_tests_a::a_scrub_that_cannot_be_admitted_answers_the_admission_refusal`
and `the_index_map_reads_the_catalogue_from_the_masters_folder_on_each_request`)
started `MAX_STORE_READ_CONCURRENT` (8) holders and waited for every one. On
this 4-core host the holders past the pool's size never started, so both
tests **hung**, past 540 s, on the merged tree. On a host with eight or more
cores they would have passed, and then the scrub would no longer have been
refused by store-read holders. Now the index-map test holds every store-read
slot this machine has (`Named::StoreRead.pool().max()`), and the scrub test
holds every slot of the scrub's own pool (`run_verify`) and is refused by it,
as before.

Phase 2 covers upstream's changes too: the five mutants in the functions the
second merge changed. `admitted_gated → Default::default()` is caught, and
`route_table`'s four replacements are unviable (`E0433`, an unqualified
`Router`).

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

**One more, from the second merge: a `feat/db-fix` test that needs eight
cores.** `detail::tests::a_refused_admission_names_what_its_bound_and_why`
fails on `7aa28d65`'s `detail.rs` (byte-identical on this branch) on this
4-core host. It asserts the refusal names "at most 8". Since R3-08,
`admission_refused` names the slots the pool has on this machine
(`slots_under(8)` = min(cores, 8) = 4 here), and its own doc comment says
so. It is red on any host with fewer than eight cores, and is not edited
here.

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
| `server.rs` | 1,509 | 1,179 | 34 | 5 | 291 |
| **total** | **2,346** | **1,916** (+3) | **43** | **6** | **378** |

The three counted apart were "caught" only by the load-red AH-02 timing test.
Each was re-run without it: `autopilot.rs:2785:38` and `pullrun.rs:2449:45`
missed (both equivalent), and `pullrun.rs:2371:58` was caught by a test this
pass adds.

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

**`server.rs`** ran with retries and with both load-red AH-02 tests left out
(every server run started after the second exclusion). Every kill came from a
test that drives the mutated code. Of its 34 survivors, **20 are real gaps
or clock-bound cases, all caught in phase 2**, and **14 are equivalent**.

Real gaps, each closed by a new test that fails on the hand-applied mutant
and passes on the original (all in `server_cloud_mutant_tests_e.rs`):

- `15421:20`, the `!` of `(!faults.is_empty()).then(..)` in
  `held_bars_page`: a damaged month's `/bars` page would hide its fault
  sentences, and a clean one would carry an empty trouble block.
  `a_damaged_month_names_its_unreadable_records_and_a_clean_one_names_none`.
- `17528:17`, `17532:17`, `17533:17`, the three line-end arms of
  `HeadDeadline::observe` (AH-02). A head ending in `\r\n\r\n` was still
  delivered a poll later by the service's own count, so no served test saw
  the scan stop working. `a_head_ends_at_its_blank_line_in_either_spelling`
  holds the scan byte by byte.
- `17647:22`, `n > 0` → `>=` in `HeadDeadline::wrote`: an empty write would
  restart a stopped reader's stall clock.
  `a_write_that_moves_no_byte_does_not_end_a_stall`.
- `17943:5` (×2), `open_requests` → `0` / `1`: the "stop started" line's
  count. Its one serve test stops with exactly one request open and never
  read the field.
  `the_stop_counts_every_request_open_across_the_speed_table` (a child
  process: the speed table is per process).
- `18002:29`, `&&` → `||` in `Held::holds`: every `Held` a test built kept
  both or neither. `a_serve_holds_only_with_both_its_lock_and_its_port`.
- `18089:5` (×2), `end_served` → `0` / `1`: the binary's own end, driven by
  no test because its cancel is process-wide.
  `ending_a_served_process_with_no_engine_work_abandons_none` (a child
  process). `→ 0` is caught by its last-word assertion: the mutant replaces
  the whole body, so the last word is never said.
- `18983:8`, the `!` of `if !clean` in `announce_universe`: the banner would
  print a clean read's notes and the degraded exit.
  `a_degraded_universe_says_why_on_the_banner_and_a_clean_one_says_no_more`
  (a child process: the banner is stdout).
- `19521:18` (×3), `abandoned > 0` → `==` / `>=` / `<` in
  `wait_then_end_counting`: the abandon line, said on a quiet stop or never.
  `a_stop_names_the_engine_work_it_abandons_and_only_then`.
- `19601:5` (×2), `run_held` → `0` / `1`: `main`'s entry, with `main` as its
  only caller.
  `the_entry_answers_a_command_it_cannot_parse_as_misused_and_keeps_nothing`.
- `35956:17`, `&&` → `||` in `indexmap_reading`'s filter: shares and indexes
  the feed does not hold would be handed to the join.
  `the_index_map_lists_a_feeds_own_indexes_and_nothing_else_it_holds` (a
  child process: the catalogue is read from the environment's masters
  directory).

**Clock-bound, and measured both ways.** `6723:25` (the match guard
`asked.window.to() == today` → `true` / `false`) and `6723:43` (`==` → `!=`)
in `finished_day_only`, which reads `SystemTime::now()` itself and has no
clock seam. The mutants differ from the original only while today's NSE
session is unsettled (an open day, before its close plus five minutes).
Phase 1 reached them on Sunday 18 Oct, a closed day, and all three survived.
No test held the " Today: …" reason for a window ending today, so this pass
adds `a_window_ending_today_is_held_back_exactly_while_today_is_unsettled`. It
asks the guard's own parts (the calendar of record, the venue's dated close,
the minute of the day) before and after the call, and holds the answer to
one of them. Phase 2 ran from 02:30 to 03:35 IST on Monday 19 Oct, an open
day before its session. There the new test **caught** `→ false`, and the 17
Oct pass's `a_window_past_today_is_refused_before_the_socket` caught `→ true`
and `!=`. Any run on a settled day misses all three again. That is a property
of the clock, not of the tests.

Equivalent, each argued from the code (every one survived phase 2 too):

- `4470:33` `>` → `>=` and `4476:30` `<` → `<=` in `failure_window`: a max
  and a min of two `Day`s, the same `Day` on a tie.
- `5726:59` `-` → `+` and `5726:93` `+` → `*` in
  `observed_cash_source_days`: the expression only sizes
  `days.try_reserve(count)`, a capacity hint. `to + from + 1` (at most about
  5.9 million) reserves more and `to − from` one less (0 for a one-day
  window). Both succeed, and the set's contents are unchanged.
- `5743:36` `||` → `&&` in the same function: the mutant never skips an
  out-of-window row, but `Window::verdict` converts the same epoch the same
  way and answers `Ok(Some(BeforeWindow | AfterWindow))` for it, so the day is
  not kept. The skip is a fast path.
- `7198:13` `wait > 0` → `>=` in `await_budget`: with `wait == 0` the mutant
  awaits a zero-length `tokio::time::sleep`, whose deadline has passed. It
  completes at once (at most one cooperative yield).
- `7984:19` (×2), `7985:15`, `7985:27`: the loop and sum of a
  `const _: () = { .. }` compile-time assertion (the 429 ladder against
  `MAX_CHUNK_BACKOFF_MS`). The mutant only skips or shrinks the check, which
  still holds, and nothing at run time differs.
- `17365:9`, `WatchedBody::is_end_stream` → `false`: hyper 1.11
  (`proto/h1/dispatch.rs`, `poll_write`) reads it before the head (`true`
  writes no body, `false` a body of `size_hint`'s exact length) and after
  each data frame (to end with it). Every empty body this server answers
  with also says `size_hint` exact 0, so the head carries `Content-Length: 0`
  either way. The mutant adds one poll that answers `None`, and the
  `InFlight` drop still comes before the flush that settles the head clock.
  Re-run in phase 2 against the **whole** suite (both exclusions included)
  at the checked-in opt-level 3 on an idle host, where the unmutated suite
  passed 2,546 of 2,546: **missed**.
- `18042:9` `Held::drop` → `()` and `18035:9` `Held::release_in_order` →
  `()`: `Held`'s fields drop in declaration order after `Drop::drop`. `log`
  is a `&'static Sink`, so dropping it does nothing; then `lock`, then
  `port`. That is the order `release_in_order` gives, and at the same point
  (the end of `release`, or of the drop).
- `19517:54` `<` → `<=` in `wait_then_end_counting`'s wait loop: the two
  differ only when the clock reads exactly the deadline (nanosecond
  equality), where the mutant polls once more, 20 ms later.

Timeouts, each a detection: `17197:20` (×2) and `17194:9` (`Slots::try_take`
can never take a slot) and `17232:15` (the acceptor waits while a slot is
free). Every served test hangs past the timeout. `7986:17` (`attempt *= 1`)
timed out at **build**, because the compile-time `const` loop never ends; no
binary exists to test. Unviable, each read from its log: 137 `E0433` (a
mutant naming `Html::..` unqualified where the function names
`axum::response::Html`), 118 `E0277`, 17 `||` inside a `let` chain, 6
`E0015`, 5 `E0080`, 4 `E0369`, 3 `E0599`, 1 `E0505`.

## Coverage

`cargo llvm-cov nextest -p api --lib --branch` (`RUSTC_BOOTSTRAP=1` for
`--branch` only; measurement, nothing committed needs it), the whole `api`
lib suite on the dev copy at the merged tree (`cdc81a6b`), `api`
at opt-level 0, 23:18–23:21 UTC (04:48–04:51 IST Monday, an open NSE day
before its session). 2,578 of 2,580 tests passed. The two failures are the
opt-level-0 artifact named under Method and the eight-core `detail.rs` test
red on the base, whose lines executed all the same.
A branch is counted untaken only if no instantiation of it took that side;
the text report lists one record per instantiation, and they are summed.
Production lines only: the inline `#[cfg(test)]` modules are left out.

| File | Lines never run | Lines with a side untaken | 17 Oct run 4 | This session's first run (08:43 UTC) |
|---|---:|---:|---|---|
| `audit.rs` | 1 | 0 | 1 / 0 | 1 / 0 |
| `import_journal.rs` | 1 | 1 | 1 / 1 | 1 / 1 |
| `autopilot.rs` | 29 | 7 | 29 / 8 | 29 / 7 |
| `pullrun.rs` | 3 | 6 | 3 / 6 | 3 / 6 |
| `server.rs` | 192 | 19 | 206 / 22 | 222 / 23 |
| `emitted.rs` | 0 | 38 (test-only) | 0 / 0 | 0 / 37 |
| `scratch.rs` | 0 | 0 | 0 / 0 | 0 / 0 |

`server.rs` gained 16 unexecuted lines in the merge (AH-02's
`HeadDeadline` and `exchange_watch` arms, at 17387–17791). This session's
tests drive all of them, and also `end_served` (17711–17719 on 17 Oct),
`run_held`, `poll_read`'s fill check and `finished_day_only`'s refusal of
today while unsettled (6722–6723).

### Lines not covered, and why

Line numbers are the newest tree's. "L" is a line never executed, "B" a line
with a branch side never taken. Every item is **unreachable** in this build,
with the reason, or **not driven**, with the rule or the missing seam. Every
`audit.rs`, `import_journal.rs`, `autopilot.rs` and `pullrun.rs` item is the
17 Oct report's, line for line (none of those files' production code changed
since), and its reasons are carried there:

- `audit.rs` L 634; `import_journal.rs` L 361, B 467.
- `autopilot.rs` L 319, B 928 / L 929–930, B 2814 / L 2815, B 2907 /
  L 2909–2932, B 2963 / L 2964–2968, B 3018 / L 3019, B 3553 / L 3554,
  B 3639 / L 3640.
- `pullrun.rs` B 863 / L 864, B 1713 / L 1714, B 2080 / L 2081, B 3814,
  3854, 3859.

**`server.rs`, unreachable in this build** (17 Oct reasons, new numbers):
B 1903 / L 1904–1908 (`VendorCensusNow::census()` is always `Some`,
D-3327); B 1971 / L 1972 (`report_from` always ends with a newline); B 2739
(`slot` enumerates `SpotTarget::ALL`); B 4462 (a one-day `Window` of a valid
`Day` always builds); L 5756–5757 (`SessionLengthUnmeasured` never reaches
`observed_cash_source_days`); B 6001 (the first guard already returned for
every other vendor); L 6477 (`ledger.source()` is `None` only for a run that
never opened); B 9279 / L 9280–9285 and L 9693, 9697, 9699
(`Feed::store_vendor()` is `Some` for every feed); B 9769 / L 9770–9776 (both
archive feeds declare an FNO layout); B 11213 / L 11214–11215, B 11681 /
L 11682, L 12561 (a `Day` and a `YearMonth` span 1970..=9999, and a window
has a day); B 12631 / L 12632–12637 (`window.verdict` converts the same stamp
first); L 15299 (`contract_param` re-reads a query already accepted); B 36061
/ L 36062, B 36068 / L 36069 (the vocabulary is dense and far below
`u16::MAX`).

**`server.rs`, not driven, by rule:** L 8828–8969 (`read_credential`,
`credentialed_source`: AWS Parameter Store, `CLAUDE.md` §8); B 35261 /
L 35278–35364 (the `/universe` crawl of NSE's index pages from
`INDEX_HOST`; no test reaches a non-loopback host, so it needs a
crawl-source seam); L 11605 (`fno_report` after a chain discovery: discovery
URLs come from the shipped descriptor, not the wire's base URL, so it needs a
discovery-URL seam); B 19868 / L 19870, 19874–19889 and L 20411
(`open_unless_suppressed` past its suppression, and a browser that opened:
none in a test); B 20341 / L 20342–20345 (the autopilot's start banner needs
a writable serve with `BRUTEX_AUTOPILOT=run` over the real credential
source).

**`server.rs`, not driven: host resources, a thread, the wall clock, a
race:** L 20170–20174 (`local_addr()` failing after a bind) and
L 20215–20220 (`keep_port` with no descriptor left), which need descriptor
exhaustion that would starve every other test; L 20294 (`Loading::Failed`,
the start-up load's thread panicked); B 20425 / L 20426 and B 20431 /
L 20432 (`warm_at_start`, `census_cache::start`: a thread that cannot be
spawned); L 10339–10342 and L 14272–14275 (`ist_day(now)` failing: a host
clock outside 1970..=9999); B 6495 / L 6496 (the 17 Oct report's race: a
member-failure append refused while the closing append right after it
lands). **B 6713**, the `!closed` side of `finished_day_only`'s guard: in this
run (an unsettled hour) no window ending today found today settled. In
phase 1's Sunday hours the opposite side was the one untaken, and
`a_window_ending_today_is_held_back_exactly_while_today_is_unsettled` takes
whichever the hour allows.

**`emitted.rs`** (38 lines with a side untaken; 37 in this session's first
run; the 17 Oct report counted none for it): every one is the false side of
an operand of the `&&` chain in one of the emit-site table's `mine:`
predicates (checked line by line: 38 lines in 27 predicates). The module is
test-only (`#[cfg(test)]` in `lib.rs`, so cargo-mutants lists no mutant in
it). A false side would be the predicate meeting a record that matches its
earlier operands and not that one, and each case's own record is the one it
meets.

**Test files** (not production): `server_cloud_mutant_tests_a` 7 / 10,
`_b` 33 / 13, `_c` 16 / 10, `_d` 3 / 1, `_e` 3 / 3. These are failure paths
(a `panic!` or an `else` arm an assertion never takes), arms one copy of the
environment never takes (a build with or without repository metadata), and
in `_e` the day-rollover and settled-day arms of `ending_on`, which a run
before the open does not take.

## Phase 2

Every phase-1 survivor and every mutant in code changed since phase 1's base,
on the newest source. Retries 2, the whole `api` lib suite except the two
mutation-run exclusions (Method), and a whole-suite opt-level-3 run for the
`HeadDeadline` survivor. Phase 2 ran on the newest tree of its time
(`0ab9fb01`, every test this pass adds). The second merge changed no scoped
production code except `admitted_gated` and `route_table`, whose mutants were
run after it.

| Set | Mutants | Caught | Missed | Timeout | Unviable |
|---|---:|---:|---:|---:|---:|
| `audit.rs`, code changed since `bdcfa9bd` (`Journal::identity`, `Journal::read_index`) | 5 | 5 | 0 | 0 | 0 |
| `audit.rs`, the 17 Oct survivors | 8 | 0 | 8, equivalent | 0 | 0 |
| `import_journal.rs:529:11`, the 17 Oct timeout | 1 | 0 | 0 | 1, detected | 0 |
| `autopilot.rs` survivors | 8 | 0 | 8, equivalent | 0 | 0 |
| `pullrun.rs` survivors and its two flaky-killed | 4 | 3 | 1, equivalent | 0 | 0 |
| `server.rs` survivors | 34 | 20 | 14, equivalent | 0 | 0 |
| `server.rs`, the second merge's changes | 5 | 1 | 0 | 0 | 4 |

The cargo-mutants 27.1.0 quirk (neither `-F` nor `-E` filters "delete field"
mutants) re-ran those of each file too. Every one was caught or unviable, as
in phase 1. `audit.rs:535:16` `==` was "caught" by the load-red test and was
re-run without it: missed, equivalent. The carried `audit.rs` arguments hold:
the four are the loops of `const _: () = { .. }` compile-time assertions at
527 and 535, and the other four are `Drops::peak`'s running-maximum ties at
652, 655, 658 and 661. `import_journal.rs:529:11` hangs as the 17 Oct report
says.

**Every survivor of this pass is therefore one of:** equivalent, argued above
in writing (8 in `autopilot.rs`, 1 in `pullrun.rs`, 14 in `server.rs`, 8
carried in `audit.rs`); caught by a test added in this pass (20 mutants: 3 in
`pullrun.rs`, 17 in `server.rs`); or clock-bound and caught when the clock
allows (3, measured both ways).

## New tests

19 tests, every one in a scoped file, and no production source changed.
Each was run green on the original. Each that targets a mutant was measured
failing on that mutant, applied by hand to the dev copy, before it was
committed. The clock-bound one is the exception: on Sunday it could only be
shown green, and phase 2 later measured its kill.

`pullrun::cloud_tests_1018`:
`a_refusal_whose_every_other_open_run_is_named_counts_none_unnamed`,
`damage_casts_doubt_only_on_a_run_opened_before_it`,
`a_newer_run_may_be_open_only_after_a_close_past_the_damage`.

`server_cloud_mutant_tests_e.rs` (module `server::cloud_mutant_tests_e`):
`a_client_that_ends_its_stream_mid_response_is_reported_once_as_its_close`,
`a_read_that_fails_mid_response_is_reported_as_the_clients_close`,
`a_read_that_fails_with_nothing_in_flight_reports_nothing`,
`a_direct_write_while_the_head_clock_runs_starts_a_response`,
`a_router_served_without_the_limited_listener_passes_through_the_watch`
(these five for coverage of AH-02's arms),
`a_window_ending_today_is_held_back_exactly_while_today_is_unsettled`,
`a_damaged_month_names_its_unreadable_records_and_a_clean_one_names_none`,
`a_head_ends_at_its_blank_line_in_either_spelling`,
`a_write_that_moves_no_byte_does_not_end_a_stall`,
`the_stop_counts_every_request_open_across_the_speed_table`,
`a_degraded_universe_says_why_on_the_banner_and_a_clean_one_says_no_more`,
`ending_a_served_process_with_no_engine_work_abandons_none`,
`a_stop_names_the_engine_work_it_abandons_and_only_then`,
`the_entry_answers_a_command_it_cannot_parse_as_misused_and_keeps_nothing`,
`a_serve_holds_only_with_both_its_lock_and_its_port`,
`the_index_map_lists_a_feeds_own_indexes_and_nothing_else_it_holds`.

Four 17 Oct tests were adapted to the two `feat/db-fix` merges (Merge,
above), each keeping both sides' intent.
`server::cloud_mutant_tests_a::a_direct_write_through_the_head_deadline_rearms_only_when_bytes_moved`
and its `_d` twin follow AH-02. `a_scrub_that_cannot_be_admitted_answers_the_admission_refusal`
and `the_index_map_reads_the_catalogue_from_the_masters_folder_on_each_request`
follow R3-08.

## Gates

Final, on a real git checkout of `77aaefaa` (its crates are `cdc81a6b`'s, the
last code commit), at the checked-in profile, as the non-root user:

- `cargo fmt --all --check`: **clean** (also on every commit of this pass;
  the one commit pushed before its format was fixed, `5d616ec3`, was amended
  to `00b32d1d` on this branch within a minute, before anything else).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **clean**,
  exit 0, no warning.
- `cargo deny check`: **green** (`advisories ok, bans ok, licenses ok,
  sources ok`). No manifest, lockfile or `deny.toml` is touched; all three
  are byte-identical to `feat/db-fix` `7aa28d65`.
- `cargo test -p api --locked`: **red, and only on the base's own tests.** At
  this host's stock `tcp_wmem` (4 MiB max), the lib target ran 2,574 passed,
  6 failed, 8 ignored. The six are the five AH-02 `head_deadline_body_tests`
  and `detail::tests::a_refused_admission_names_what_its_bound_and_why`, all
  red on `feat/db-fix` itself (Red on the base). With `tcp_wmem` max 256 KiB
  and `--no-fail-fast`, every target ran: lib 2,579 passed, 1 failed (the
  eight-core `detail.rs` test), 8 ignored; `src/main.rs` 4 / 4;
  `tests/binary.rs` 4 / 4; `tests/pool_load.rs` 5 / 5;
  `tests/store_step_forms.rs` 3 / 3; doc-tests 1 / 1. No test this pass adds
  or adapts fails at either setting.
- The 19 new tests under `strace -f -e trace=connect`, with no proxy in the
  environment: 7 `connect()` calls, every one to `127.0.0.1`. No test reads a
  credential or reaches a vendor.

## Left

Nothing in this pass's scope. What remains needs the Mac or the operator
(below).

## NEEDS THE MAC

- **The five AH-02 `head_deadline_body_tests`** (Red on the base): confirm
  they pass on macOS at the base, and decide whether a stock Linux send
  buffer (`tcp_wmem` max 4 MiB) is a case the server must meet. CI is
  `ubuntu-24.04` and likely red on them (UNVERIFIED: no CI run was read).
- **`detail::tests::a_refused_admission_names_what_its_bound_and_why`** (Red on
  the base): it passes only with eight or more cores. GitHub's standard
  Linux runners have fewer (UNVERIFIED for this repository's runners).
- **The two R3-08 adaptations** were measured only here, at 4 cores. Re-run
  them on the Mac, where the pools have their full eight slots.
- **The new socket tests** (half-close, `SO_LINGER(0)` reset, the
  `HeadDeadline` unit tests) were measured on Linux only. The 17 Oct pass's
  reset test uses the same method.
- **`a_window_ending_today_is_held_back_exactly_while_today_is_unsettled`**
  asserts a different side by the hour, by design. It passed here in both
  (Sunday, settled; Monday 02:30–04:51 IST, unsettled).
- **For the operator's call, unchanged:** the carried 16 Oct finding still
  holds on the merged tree. A vendor-named `Disposition::RetryBounded` under a
  status outside 500–599 is laddered on `server_errors`, which is counted only
  for 500–599 (server.rs:7021, 8207, 8359), so it waits 0 ms five times and
  says the vendor failed "0 time(s)". Whether Dhan or Groww send those names
  under a non-5xx status is **UNVERIFIED**. Also for information,
  `pullrun.rs:2449:45`: at a tie no consistent index can produce, the count
  filter drops the count while its comment says only damage after the oldest
  opening record does (Phase 1 results). Nothing changed.

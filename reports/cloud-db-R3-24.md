STATUS: IN PROGRESS

# R3-24 (cloud): logs and audit — audit lines never roll away, more search keys, every state change audited

Branch `cloud/db-R3-24-20261017`, cut from `origin/feat/db-fix` at `f9cdb584`
(Merge R3-14). Prerequisites, confirmed merged into `origin/feat/db-fix` with
`git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-14|R3-16|WP-62):"`:
R3-14, R3-16 and WP-62, all three present.

Every file changed is one R3-24 owns (`git diff --stat origin/feat/db-fix -- crates`):

| file | what changed |
|---|---|
| `crates/telemetry/src/state_events.rs` (new) | the state-change table: 21 rows, 5 classes, each bound to its emit site |
| `crates/telemetry/src/state_events_tests.rs` (new) | `every_state_change_event_is_audited` and four more |
| `crates/telemetry/src/sink.rs` | the never-rolling state stream, per-file time spans, reach, the picture counters, the live copy, `reason`/`kind` index dimensions |
| `crates/telemetry/src/run_dirs.rs` | hosts `state_events` (`#[path]`), `STATE_STREAM_DIR`, and re-exports R3-24's new types (see "Shared lines" below) |
| `crates/api/src/logs.rs` | the new keys, the time-span search, target alone through the index, "the first 2,000 of N", reach and its alert, the contradicted-line mark, the picture page, the live tail from memory, the refusal follower, both streams' health |
| `crates/api/src/audit_json.rs` | `/audit.json` names the store's log reach and its newest state changes |
| `crates/api/benches/ratio/logs_dirs.rs` | bench row C-LG-03 |

## Done

Each goal item of the work package, what now does it, and the test that proves it.

1. **The who-did-what lines go to their own stream that never rolls** (DB-G88).
   Every state change (the table in item 4) and every lifecycle line is
   appended to `<log dir>/audit.state/`, laid out exactly as the audit stream
   is (same files, same index, same pointers, same readers) but never rolled
   and never reaped. A directory an earlier build wrote is seeded once at open
   from its audit stream and its ring, merged by `seq`, a line both hold copied
   once (`a_new_state_stream_is_seeded_from_the_audit_stream_and_the_ring`,
   `a_ring_line_sharing_a_seq_with_another_is_seeded_too`). Its health is on
   every `/logs.json` answer; it is loud past 256 MiB (`STATE_LOUD_BYTES`),
   which is a size to look at, not a bound — the stream never drops a line.
   Named test: `an_audit_line_survives_2_gib_of_refusals` — the refusals roll
   the audit stream past its whole retention twice, the rolling stream loses
   the first state change, and the state stream still finds every one by key,
   in order, once. Run here scaled by 1,024 on both sides (a 64 KiB x 16
   stream, 2 MiB of refusals: the same ratio); the same proof at the real
   bounds is `..._at_full_size`, `#[ignore]`d because it writes 2 GiB (NEEDS
   THE MAC, below).
2. **Every log answer says how far back it reaches, with one alert under 30
   days.** Each audit generation keeps a 16-byte `span` file (first and last
   event time; the open file is open-ended); `reach_of` answers the ring's,
   the audit stream's and the state stream's oldest line per directory without
   reading a log line. `/logs.json`, every keyed answer, the `/logs` page and
   `/audit.json` (`log_reach`) carry it; one alert when the audited history is
   younger than 30 days, and "no directory read keeps an audit history" when
   there is none (`every_log_answer_says_how_far_back_it_reaches`,
   `a_generation_keeps_its_first_and_last_event_time`).
3. **More keys.** `month=`, `reason=`, `kind=` (each its own index dimension;
   `reason` and `kind` are new dimensions, recorded per generation in its
   `dims` file so an older generation is named as not covering them),
   `from=`/`to=` (epoch ms, an IST day/minute/second, or the same in UTC with
   `Z`), and **`target=` alone through the index** (DB-023: it used to filter
   the newest 4 MiB of the ring and stop). A span alone is read from the files
   whose span meets it: per generation one span probe, one binary search over
   byte offsets, and backward reads of at most 1 MiB (`SPAN_SLICE_BYTES`) in
   all; beside a key it prunes every generation whose span misses
   (`the_log_is_asked_by_month_reason_kind_and_span`,
   `a_target_alone_reads_every_audited_line_not_the_newest_4_mib`,
   `a_span_is_read_across_generations_and_stops_where_it_must`,
   `a_key_beside_a_span_passes_a_generation_unread`). Garbage in any of them
   is named in `ignored`, never read as "no filter".
   **A capped answer says "the first 2,000 of N"** and hands back a cursor
   that continues it (`a_capped_answer_can_be_continued`); a span alone that
   stopped at its slice says "at least N", never a total.
4. **One table names every state change** —
   `crates/telemetry/src/state_events.rs`, 21 rows in 5 classes (operator,
   server, maintenance, lag, run), each row naming its emit site;
   `is_audited` takes every row, so a row is audited by construction. The ten
   the spec names (autopilot paused and resumed, stop requested, masters
   refreshed, census migrated, compacted and recounted, server started and
   stopped, census lag) are checked by name in
   `every_state_change_event_is_audited`; `every_row_names_its_emit_site`
   reads each named source file and fails when the row's target or message is
   not in it, so a renamed message cannot leave the table describing nothing.
5. **The importer's refusal lines are followed on a background thread.** A
   page request reads at most 256 KiB (`REQUEST_SLICE_BYTES`) of a ring; one
   thread reads 1 MiB slices of each followed directory every 100 ms; a
   directory still behind is named "N byte(s) of its ring are not read yet",
   never answered as "none" (`a_request_reads_at_most_its_slice`,
   `the_follower_reads_what_a_request_left`,
   `a_region_with_no_line_in_it_never_stalls_the_ring`). A follower that cannot
   start is said in the same words, not by a new emit site
   (`a_follower_that_cannot_start_is_said`). Both streams' waiting-delete
   counters are served (`both_streams_and_their_waiting_deletes_are_served`).
6. **The `/logs` picture is read from counters, not files.** The sink counts
   every line it writes at write time: 60 one-minute buckets by level, a
   16-code space-saving table of top codes (a replacement counted and said),
   the ring's oldest line moved on at each roll; the live tail is the sink's
   in-memory copy of its newest 512 lines (`the_logs_picture_is_read_from_counters_not_from_files`
   — the ring's files deleted under the sink change nothing on the page —
   `the_live_tail_is_a_copy_from_memory`, and the seven `picture_tests` in
   `sink.rs`). The page is drawn by `logs.rs`.
7. **A contradicted "day imported" line is marked** (DB-A48). Each "day
   imported" line on `/logs.json` and `/logs` whose day the import journal
   closed other than `done`, or holds no line for, carries
   `conflict: {with: "import journal", journal_outcome, journal_line}` and the
   page a CONFLICT mark (`a_contradicted_day_imported_line_is_marked`). The
   journal is read as one snapshot (a tail of what was appended since the
   last read) per answer, and only when the page holds a "day imported"
   line; each such line is one range probe of its day.
8. **Bench row C-LG-03** (`crates/api/benches/ratio/logs_dirs.rs`): each of
   nine keyed searches cold (the first call on its store) and warm (n=200),
   then with 60 clients at once (4 calls each) and n=200 while an import
   writes, p50/p99/max each, the 5 ms p99 target printed beside it; and the
   live tail from memory. Gated on the 1x/10x ratio as C-LG-01 is; the
   5 ms target is printed, not gated, because a sandbox figure is not the
   Mac's.

### Fails before, passes after

The named tests call what R3-24 adds (the state stream, the new keys, the
reach), so on the base they do not compile. To show each finding fails on the
base for the reason stated, three probes in the base's own API were run on a
scratch worktree of `origin/feat/db-fix` at `f9cdb584` (kept outside the
repository, never committed):

| finding | probe on base | base result |
|---|---|---|
| DB-G88 | a state change, then 2 MiB of refusals through a 16 x 64 KiB audit stream; look for it by key | FAILED: "the first state change is gone" |
| DB-023 | one refusal, 4.5 MiB of other lines; `target=pull.spot` | FAILED: the line is not found, `hit_scan_cap: true` |
| DB-A48 | journal closes 2020-01-02 `incomplete`; the log says "day imported" | FAILED: the line carries no mark |

On the branch the same properties are the named tests above, all green.

## Shared lines (outside the files R3-24 owns)

- **`crates/telemetry/src/lib.rs` is not owned.** `state_events` is hosted in
  `run_dirs.rs` with `#[path]`, and R3-24's new public names are re-exported
  through `telemetry::run_dirs::` instead of the crate root. When `lib.rs`'s
  owner next touches it, `pub mod state_events;` and the re-exports can move
  to the root; nothing else changes.
- **`/imports.json` marking of contradicted lines** (DB-A48's fix hint names
  both `/logs.json` and `/imports.json`): `/logs.json` and `/logs` mark them;
  `/imports.json` is `crates/api/src/imports.rs`, not owned. The mark is one
  call — the `contradicted` function in `logs.rs` — for that file's owner.
- **DB-G88's fix hint also names pull-side aggregation** (one refusal event
  per instrument, month and reason) and a `store.tix` rate limit. Those emit
  sites are in `crates/pull` and `crates/store`, not owned; R3-24's goal
  chose the never-rolling stream, which keeps every state change whatever the
  refusal volume. The volume itself is unchanged.
- **CI gate 21's declaration** (`.github/workflows/ci.yml`, not owned) counts
  the logger's file calls. `sink.rs` now has `File::open` 3 (was 1: the span
  read of a generation's first line, and the state stream's seeding reader),
  `create_dir_all` 7 (was 5: the span file's generation, the state stream),
  `metadata` 3 (was 2: a generation's last line), `read` 2 (was 1: the span
  file), `write` 2 (was 1: the span file). Each opens only the logger's own
  files. The declaration needs those five numbers.
- **CI gate 20's declaration** (same file) counts uncovered lines per
  telemetry file from a workspace coverage profile; see "Gates" for what was
  measured here and what was not.
- **`docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`** are
  not owned. Proposed entries, for whoever lands them, with their numbers
  assigned then:
  - invariant: "a state change is never rolled away" —
    `an_audit_line_survives_2_gib_of_refusals`; "every state change is
    audited" — `every_state_change_event_is_audited`; "a capped answer can be
    continued" — `a_capped_answer_can_be_continued`; "the picture is never
    read from files" — `the_logs_picture_is_read_from_counters_not_from_files`.
  - decision: a never-rolling state stream laid out as an audit stream,
    seeded once; `target=` alone is an index key; a span alone is read by
    binary search of the files its time meets.
  - limits: the state stream grows with state changes and has no bound (loud
    past 256 MiB); a span alone reads at most 1 MiB per request; a request
    reads at most 256 KiB of a ring and the follower 1 MiB per directory per
    100 ms; the picture's code table is 16 codes, approximate past that
    (space-saving), and says when it replaced one.

## Left

- **The `api` mutation run is still going** (see "Gates"): the result is
  recorded here when it ends.
- **Every measured target that needs the Mac** — the 5 ms p99 of C-LG-03,
  the audit roll on WD_BLACK at n=40, the 2 GiB proof at full size, and every
  check against the served store — is under NEEDS THE MAC. No figure for them
  is claimed here: this sandbox was running the mutation job on all four of
  its cores, and a latency taken beside it would be neither the Mac's nor
  this machine's.
- The shared lines above, for their owners.

## Gates

Run on the final tree of this branch; the base is `origin/feat/db-fix` at
`f9cdb584`, run in a scratch worktree.

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked --no-fail-fast` | see below: green, once the permission tests run as an ordinary user |
| web tests for touched files | none apply: no file under `web/` is touched |
| `cargo deny check` | red, exactly as the base: `advisories FAILED, bans ok, licenses ok, sources ok`, the five advisories RUSTSEC-2026-0344 (ruzstd 0.8.2) and RUSTSEC-2026-0346, -0347, -0348, -0349 (rustls 0.23.45), the same five on the base; `Cargo.toml` and `Cargo.lock` untouched (`git diff --quiet origin/feat/db-fix -- Cargo.toml Cargo.lock`) |
| line coverage of the added lines | see below |
| branch coverage | NOT MEASURED: `cargo llvm-cov --branch` needs a nightly toolchain and only `1.97.1` stable is installed here |
| mutants | `telemetry`: none survives; `api`: running (below) |

**Tests.** The container runs as root, and root ignores the mode bits that the
permission tests close a file or directory with, so those tests cannot fail
the way they assert. As root, `cargo test --workspace --locked --no-fail-fast
-- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store` (the
shared context's named 95-minute test, which R3-24 does not touch) failed 27
tests in 6 binaries: `api` 12, `cli` 3, `pull` 3, `store` 3, `telemetry` 5,
`store/tests/cited_commits.rs` 1. The five library test binaries, re-run as
uid 65534 (`setpriv --reuid=65534 --regid=65534 --clear-groups`), all pass:

| binary | as uid 65534 |
|---|---|
| `api` lib | 2145 passed, 0 failed, 8 ignored |
| `cli` lib | 2122 passed, 0 failed, 4 ignored, 1 filtered (the 95-minute test) |
| `pull` lib | 1255 passed, 0 failed, 19 ignored |
| `store` lib | 259 passed, 0 failed |
| `telemetry` lib | 187 passed, 0 failed, 1 ignored (the 2 GiB full-size proof) |

- One of the five `telemetry` root failures was not a permission test:
  `a_run_is_read_from_every_form_a_line_carries_it_in` (a base test) found
  its directory's lock held when it reopened a sink it had just dropped. It
  passed 5 of 5 alone and in a full re-run of the library as root, and as
  uid 65534. Its cause is UNVERIFIED; one candidate is a child process another
  test spawns, which holds an inherited copy of the lock between `fork` and
  `exec`.
- `cited_commits` fails on the base the same way: `docs/06-limits.md`, not
  touched here, cites commits that are not ancestors of `origin/main`.
- Doc-tests: all 13 groups pass.

**Coverage**, measured with `cargo llvm-cov` (stable, line coverage only),
each crate's library tests run as uid 65534:

- `telemetry`: every added line outside a test module is covered.
  `run_dirs.rs`, `state_events.rs` and `state_events_tests.rs` have no
  uncovered line at all. `sink.rs` has 61 uncovered lines in the file against
  the base's 63 in the same measurement; no base line lost its coverage
  (compared by content), and the only uncovered added lines are 13 in its
  test modules: the `#[ignore]`d full-size proof's body, two arms of a test
  helper that a passing test never takes, and a test double's `sync` and
  `reopen`.
- `api`: the run before the last two commits left 13 added lines uncovered in `logs.rs`, none in `audit_json.rs`; the four on production paths are now gone (two tested, one rewritten without an arm that could not run, one tested by the slice test). The re-measure of the final tree is running and its result is recorded here when it ends.
- Gate 20's declaration counts uncovered lines per `telemetry` file from a
  WORKSPACE profile, which was not built here (it compiles every crate
  instrumented, and this sandbox has neither the disk nor the time beside the
  mutation job). The numbers above are crate-scoped; gate 20's are NOT
  MEASURED and need the CI run.

**Mutants** (`cargo mutants --in-diff` against the branch's own diff):

- `telemetry`: 149 mutants; first run 103 caught, 32 unviable, 8 timeouts,
  6 missed. The six missed and the eight timeouts were re-run after
  four new tests (`a_state_line_is_read_without_its_newline`,
  `a_ring_line_sharing_a_seq_with_another_is_seeded_too`, the open span of a
  rolled generation, the code field found wherever it stands, and the state
  stream's loud size pinned) and an assertion that makes the 2 MiB flood fail
  instead of spinning when its audit stream takes no line: 21 mutants, 20
  caught, 1 timeout. The timeout is `delete !` in `StateLines::next`, which
  turns the end of a file into an endless loop: the suite hangs and does not
  pass, so it is not a survivor.
- `api`: 390 mutants in the diff, running (`-j 2`, the library tests whose names hold `logs` or `audit`, the three root-only permission tests skipped, built at `opt-level 0` through `CARGO_PROFILE_DEV_*` environment variables so a build takes minutes, not ten). Baseline green. The result is recorded here when it ends.

**CI gates** (`.github/checks_all`'s plan, every job a container can run,
base and branch): every exit status is the base's except **gate 21**, red on
the branch only, whose declaration is in `ci.yml` (see "Shared lines" for the
five numbers it needs). Gates 0, 1d, 10, 12, 19, 23 and 30 are red on both
and their output differs only in counts (two more files, more lines; gate 12
finds two more cost claims, both written `UNVERIFIED`). **Gate 11** is red on
both, and the branch adds twelve occurrences in files R3-24 owns; each is
bounded, and its allowlist is `.github/gates_ledger.rs`, not owned:

| rule | where | bounded by |
|---|---|---|
| 4 `.sort` | `logs.rs` `state_changes_json` | the lines of at most 64 KiB (`STATE_TAIL_BYTES`) of each of the fixed log directories |
| 4 `.sort` | `sink.rs` `Picture` codes | the 16-entry code table (`PICTURE_CODES`) |
| 6 `.position` | `logs.rs` `line_after` (two) | one read of at most two lines' ceiling (`MAX_LINE_BYTES`) |
| 6 `.position` | `logs.rs` `span_backwards` | one slice of at most `SPAN_SLICE_BYTES` (1 MiB) |
| 6 `.rposition` | `logs.rs` `read_queued` | one slice of at most `FOLLOW_SLICE_BYTES` (1 MiB) |
| 6 `.find` | `logs.rs` `state_changes_json` halves | the fixed set of log directories |
| 6 `.find` | `logs.rs` `contradicted` | the journal rows of one day and kind (the `*` row sorts first in the journal's key order, so the search stops at the first row of the kind whenever it is there) |
| 6 `.find` | `sink.rs` `count_written` | one event's fields |
| 6 `.find` | `state_events.rs` `row_of` | the 21-row `TABLE` |
| 7 `.contains(&` | `logs.rs` refusal follower | one slice of at most `NO_LINE_BYTES` and the request or follow slice |

## NEEDS THE MAC

Every command below runs on the operator's Mac, in the checkout, at this
branch's head. Each is a read or a scratch write: none writes the store, and
none POSTs.

1. **The 2 GiB proof at the real bounds** (writes 2 GiB through a 16 x 64 MiB
   audit stream; peak on disk about 1.1 GiB: the 1 GiB the stream keeps, the
   64 MiB ring, and the generations waiting for the reaper). With `TMPDIR`
   set to a scratch directory on WD_BLACK:

   ```
   TMPDIR=<scratch directory on WD_BLACK> cargo test -p telemetry --release --locked --lib -- --ignored an_audit_line_survives_2_gib_of_refusals_at_full_size
   ```

2. **The audit roll on WD_BLACK, n=40** (C-AU-01, and C-AU-02 printed; as
   the bench is written, 40 rolls at 10 and at 1,000 posting files and 3 at
   100,000):

   ```
   TMPDIR=<scratch directory on WD_BLACK> cargo bench -p telemetry --bench ratio --locked
   ```

3. **Every keyed search, cold and warm, 1 and 60 clients, with an import
   writing** (C-LG-03; the target is p99 under 5 ms, printed beside each
   row; the row also prints the live tail from memory):

   ```
   cargo bench -p api --bench ratio --locked 2>&1 | grep -E "C-LG-0[13]"
   ```

4. **The served store**, against a running server (port as configured; GET
   only). First start after this branch lands, then:

   ```
   curl -s 'http://127.0.0.1:8080/logs.json?limit=5' | jq '{reach, picture, seeded: .state_stream.seeded, state_loud: .state_stream.loud, audit_waiting: .audit_stream.gone_waiting}'
   curl -s 'http://127.0.0.1:8080/logs.json?month=2026-10&limit=20' | jq '.keyed.index.said, (.records | length)'
   curl -s 'http://127.0.0.1:8080/logs.json?reason=census_full&limit=20' | jq '.keyed.index.said'
   curl -s 'http://127.0.0.1:8080/logs.json?kind=options&month=2026-10&limit=20' | jq '.keyed.index.said'
   curl -s 'http://127.0.0.1:8080/logs.json?target=pull.spot&limit=20' | jq '.keyed.index.said, .hit_scan_cap'
   curl -s 'http://127.0.0.1:8080/logs.json?from=2026-10-16&to=2026-10-16&limit=20' | jq '.keyed.span, .keyed.index.said'
   curl -s 'http://127.0.0.1:8080/logs.json?target=gdfl.import&limit=200' | jq '[.records[] | select(.conflict)] | length'
   curl -s 'http://127.0.0.1:8080/audit.json' | jq '.log_reach, .state_changes.rows[:5]'
   curl -s 'http://127.0.0.1:8080/imports.json' | jq '.'
   curl -s -o /dev/null -w '%{http_code}\n' 'http://127.0.0.1:8080/logs'
   ```

   What each should show: `reach` with an `audited_from` and, while the
   state stream is younger than 30 days, its one alert; `state_stream.seeded`
   at the first start (how many lines the seed copied, and how long the first
   start took beside it — NOT MEASURED here); each key's answer saying how
   many lines it read of how many; `target=pull.spot` through the index, not
   the ring; the "day imported" lines DB-A48 counted (214 non-done days: 212
   the journal closed `incomplete`, and 2022-03-07 with no journal line)
   each marked with a conflict (the count above is one page's, so ask by
   `day=` for a day of each sort to see its mark); `/audit.json`
   naming the reach and the newest state changes; `/imports.json` never
   answering "none" for a directory the follower has not read yet (it says
   how many bytes are behind); the `/logs` page drawing the picture (events
   per minute by level, top codes, oldest line) and the reach line.

5. **The `api` mutants, if this sandbox's run did not finish** (see
   "Gates"):

   ```
   git diff origin/feat/db-fix -- crates/api/src > /tmp/r324-api.diff
   cargo mutants --in-diff /tmp/r324-api.diff -p api -j 4 -- --lib
   ```

6. **The CI gates that need an edit outside R3-24's files**: gate 21's five
   numbers and gate 20's per-file counts in `.github/workflows/ci.yml`, and
   gate 11's twelve rows in `.github/gates_ledger.rs`, for their owners; then
   the CI run, or locally the loop `checks_all script` prints (build it with
   `rustc --edition=2024 .github/checks_all.rs -o <scratch>/checks_all`), to
   show them green.

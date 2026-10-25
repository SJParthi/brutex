STATUS: BLOCKED

# R3-12: /db reads its lists from the server: no whole-list download, no typed lists, rows kept as columns

Branch `cloud/db-R3-12-20261024`, cut from `origin/feat/db-fix` at `e851da1c` (Merge DATA-2).
Spec: `reports/db-specs-20261024/R3-12.md` on `origin/cloud/db-base-20261024`.

## Prerequisites

All six are merged on `origin/feat/db-fix` (`git log origin/feat/db-fix --first-parent --merges --format=%s | grep "^Merge <name>:"`):
R3-01, R3-02, R3-03, R3-04, R3-05, R3-09 (one merge each). R3-12 itself: not merged.

## Done so far (commit 1655f44b)

- FM-621 (provisional id), the 12 Oct acceptance case and the whole-census fallback, in
  `web/src/lib/store-census.js`, `store.svelte.js`, `routes/db/DbCensusBanner.svelte`,
  `lib/db/selection.svelte.js`, `lib/db/selection-model.js`, `routes/db/DbControls.svelte`:
  - `ScopeWanted` gains `segments` (the page's segment keys); `viewSegments` resolves them against
    the root's own list through `selectionSegments` (notes-labels.js, the one place the rule lives).
  - A spot view reads INDEX/CASH rows only and never asks FNO anything. A contract view reads the
    underlying's facets answer (`/census/facets.json?feed&segment&underlying`, no rows) and offers its
    expiries from it; its rows are read only at a chosen expiry. A segment the view does not read is
    never fetched and never reported unread; the root's counts stay on its chip (`scope.listed`).
  - Every selection change the held scope does not cover is read as its own scope (`scopeWanted`,
    `scopeCovers`, `scopeNeed`); `planRead` scopes every caller that names an instrument; the
    whole-census fallback and `wholeStore` are removed from /db (a failed scope is the page's error,
    with its reason, `scopeFailure`). The instrument picker lists the server's underlyings
    (`scopeUnderlyings`, facets answers moved forward by deltas).
  - Unknown view keys (`seg=index`) are refused before any request (`unknownViews`).
- Tests (web/tests/census-scope.test.js, 19 tests, all green): the three named FM-621 tests
  `a_spot_view_never_requests_fno_facet_rows`, `an_options_view_with_no_expiry_offers_expiries_from_the_facets_answer`,
  `a_segment_switch_reads_the_new_segments_scope`, plus `viewSegments` (unplaced server segment, feed
  with no spot segment, `seg=index`). Changed, not deleted, and said in each test: the empty-instrument
  case (now the store's own answer), the chain refused at the expiry on screen (now named unread, no
  whole read), the planRead whole-asked case, the banner's whole-read line.
- `node --test` of census-scope, census-first-load, census-degraded, store-census, db-selection,
  db-seam, db-address: 120 pass, 0 fail.
- svelte-check: 0 errors in the files R3-12 touched; 37 errors in other files at the time of the run
  (read-pools, ingest, gap-verdict, database-pages and settings-ceilings tests): base reds to be
  confirmed on the unmodified base before the final gates.

## Done after the first hold (commits f98a30a9, 785ec646, 5a93cc1e)

- No /db view reads the whole census: `no_db_view_downloads_the_whole_census`
  (web/tests/census-first-load.test.js) compiles `store.svelte.js` and drives `syncStore` over a fake
  server for every view, a refresh, a feed switch, a failing scope and `seg=index`: no `/store.json`;
  a failed scope is the page's error with its reason.
- Coverage (NEW-07, RA-029): the server's roll-up (`/census/rollup.json`, `readRollups`) for the
  instrument and for its segment, drawn under the Coverage summary, refusals named. The re-basable
  check reads the row's own `corporate_action_unverified` verdict (an options view reads FNO only, so
  the old INDEX lookup in the rows read would have called every index option a share's).
- Lists and limits (RA-258, RA-259): ceilings from `/settings.json` (`adoptCeilings`), plans refused by
  them, "x of 1,024 series · y of 240 months" shown before a request (`ceilingText`); the import source
  read from the feed's archive `source`; `derived_rungs` from `/feeds.json` with no typed fallback;
  the imports journal asked for a `folder` feed with no fixed probe day; the dead `FLOOR_YEAR` /
  `spanFloor` removed; `seg=index` refused by name. Build check
  `no_typed_feed_kind_or_date_in_page_logic` (web/tests/no-typed-lists.test.js) over R3-12's page
  files, feed names read from the descriptor table, segment words from `Segment::as_str`; one named
  exception, `CLOSE_CHANGE_DAY` (sourced exchange rule; no route serves the session schedule yet).
- `a_silent_worker_is_dropped_and_the_page_says_so` (census-worker.test.js): dropped at 10 s, rows
  decoded on the page and published, the banner's worker line and the refresh log say why (RA-043).
- `db_loads_at_ten_times_the_list_without_a_long_task` (web/tests/db-ten-times.test.js).
- `yieldTask` refs its MessagePort while a yield waits: census-worker tests 9-10 pass in Node 22.22.

## Done in the third round (commits 34b28d51, 7548d5fd, b422255d)

- Merged the newest `origin/feat/db-fix` (7b397bbe: R4-27, IA-4, CLOUD-MUTSTORE, CLOUD-MUTCLI; no
  file under `web/` changed upstream) as `34b28d51`.
- Extreme cases (census-scope.test.js, census-first-load.test.js): one record; the 4,096 cap and one
  past it; today / archive end / one day past it / a year turning; an out-of-order listing; torn
  root, rows and listing bodies (now named "its body is not JSON", never thrown); garbage views;
  out of the box, a hyphenated and an ampersand symbol (`BAJAJ-AUTO` is not BAJAJ's row: readScope
  now reads its own rows by `parseKey`, and a chain's rows carry the expiry asked); a superseded read
  never published; combinations: saturated + moved generation, ten times the expiries + a chain past
  the cap, a segment switch in flight + a failing read.
- Fails before, passes after (base = e851da1c sources exported to the scratchpad, the new test files
  copied in): census-scope, no-typed-lists and db-ten-times fail at import at base (the functions they
  test do not exist there); census-first-load 7 (`no_db_view_downloads_the_whole_census`), 8 and 9
  fail on behaviour; census-worker 9-10 are cancelled at base by the `yieldTask` defect.
  Behaviour of the three FM-621 cases on the base `readScope`/`planRead`, by script: the spot view
  asks `segment=FNO&underlying=ALPHA&rows=census-tuples-v1`; the options view with no expiry offers
  no expiries (`scope.expiries` undefined, FNO named unread); a segment switch is the same request key
  (`example#0#scope:ALPHA:`), so nothing is re-read. `a_silent_worker_is_dropped_and_the_page_says_so`
  PASSES at base when run alone: the 10 s ceiling and the page decode already existed (WP-56b); the
  test is R3-12's proof of it, not a fix.
- Web gates in this container (Node 22.22.0; CI runs Node 24):
  - W1 (`web/build` matches a fresh build): RED AT BASE: a fresh build of the unmodified base changes
    55 files under `web/build` (worktree of e851da1c in the scratchpad). R3-12 does not commit
    `web/build` (the spec's rule); the Mac rebuilds it at merge-back.
  - W3 (svelte-check, ceiling 0): base 32 errors in 9 files, this branch 32 in 9 (the same set; none
    in a file R3-12 touched). Red at base, not raised by R3-12.
  - W4 (unused CSS selectors, ceiling 0): 0 on this branch, 0 at base.
  - `npm run build`: exit 0.

## Done in the fourth round (commits 1d5d9677, 1b4986b3, c42e9730, 01889efe)

- D-7111 (the one decision R3-12 locks: a /db view reads only what it shows), invariants
  R312-01..11, limits L-R312-01..07.
- DB-B47, the page half: the window route's half is fixed at the head (`life_of`, `served_end` in
  `crates/api/src/bars_series.rs`: an `INDEX` is owed to the feed's served end; a `CASH` share and a
  contract keep their held life). The Coverage grid drew an index's months after its newest file as
  nothing; now `owedTo` (selection-model.js) moves its last owed month to the wider of the feed's
  `archive.to` and `window.to` (the route's widest-of rule; `archive.to` on `/feeds.json` is itself
  the widest of the descriptor, the import journal and the census), and `absentMonthsOf`
  (DbCoverageGrid.svelte) draws them as absent rows. Test `an_index_owes_its_months_to_the_feeds_span_end`
  (census-scope.test.js, 30 pass).
- L-R312-07, said plainly: a scope's rows are row objects (`decodeCensus`, at most 4,096 a segment
  read), not typed columns; the typed columns are WP-56b's whole-census path, which /db no longer
  takes. Making the scope columnar changes `StoreRow` for every reader of `store.rows`; not done.
- Rust gates on this branch (R3-12 changes no file under `crates/`: `git diff 7b397bbe HEAD --
  crates Cargo.toml Cargo.lock` is empty):
  - `cargo fmt --all --check`: exit 0. `cargo clippy --workspace --all-targets --locked -- -D
    warnings`: exit 0.
  - `cargo test --workspace --locked` stopped at its first failing suite, `api --lib`: 2,461 pass,
    10 fail, 13 ignored. Nine reproduce alone and are red at the base inputs too (the tree's `web/`
    and `docs/` checked out at 7b397bbe, then restored): `detail::tests::a_refused_admission_names_what_its_bound_and_why`,
    `pullrun::restart_tests::the_journals_own_extremes`, `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`
    ("this host reads a 000 directory (root?)": the container runs as root), five
    `server::head_deadline_body_tests` (slow-reader timing), and
    `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
    (its child process never finishes: killed after 25 min on the branch; at the base inputs it did
    not finish inside 240 s either). The tenth, `logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code`,
    passed when run again alone.
  - Every other test target, one at a time with its own timeout (a hang cannot hide the rest;
    `cargo test --locked -p <crate> --no-fail-fast`, the api targets by `--test`), on this
    4-core container running as root with 4.5 to 9 GB of disk free:

    | target | pass | fail | ignored | why it fails here |
    |---|---|---|---|---|
    | core, costs, engine, greeks, indicators, lake, runner, vocab | 255, 374, 131, 59, 504, 130, 797, 171 | 0 | 0, 2, 0, 0, 0, 6, 0, 0 | green |
    | api lib (with the hang skipped) | 2,462 | 8 | 13 | the 8 named above, red at the base inputs |
    | api `cloud_mutants_bars` | 58 | 0 (+1 hang) | 0 | `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held` never ends (killed at 900 s); the other 58 pass with it skipped |
    | api `logs_r3_24`, `pool_load`, `state_stream_emits`, `store_step_forms`, `binary`, bin, doc | 6, 5, 3, 3, 4, 4, 1 | 0 | 0 | green |
    | cli | 2,428 | 30 | 12 | 26 GDFL-import and tick-second-check tests: the import refuses `disk_full` below `pull::gdfl_import::DEFAULT_FREE_FLOOR` (10 GiB) and the volume has 4.7 GB free (`BRUTEX_GDFL_FREE_FLOOR=0` frees the two integration tests, 2,430 pass / 28 fail: the unit tests pass their own environment); `scrub_blocks` unwritable ledger and two `watchdog_command` read-only tests: root writes anyway |
    | pull | 2,507 | 12 | 49 | the same disk floor (`ia16`, `gdfl_run`), read-only and refused-open tests root passes, and `gdfl_cm::tests::fixtures_are_built_not_pasted` (a check over `crates/cli/src/tick_second_check_tests.rs`, unchanged from the base) |
    | store | 905 | 3 | 8 | three `settings` tests that need an unreadable or unwritable directory: root reads and writes it |
    | telemetry | 237 | 4 | 1 | four `sink` tests that need a refused rename, delete or look-up: root is not refused |

    THE TWO HANGS HAVE ONE CAUSE, and it is the machine: `api::detail`'s `calendar` and
    `store_read` pools are sized `cores().clamp(1, 8)`, 4 slots here, and both tests park
    `MAX_*_CONCURRENT` (8) holders on a `Barrier` of 8 + 1 parties: four holders never get a slot,
    so the barrier never fills. A host with 8 or more cores passes them. None of these failures is
    in a file R3-12 changed or reads a file R3-12 changed; the Mac's run is in NEEDS THE MAC.
  - Gate 27 (`gates-ledger invariant-ids`): OK, 3,210 rows unique. Gate 27b: red at base only
    (D-6247 heads two entries; not R3-12's). Gate 10: red at base only (R427-11, HA-02, AG05-13); with
    those three rows taken out of a scratch copy, `invariant-paths` checks 3,024 references with exit
    0 and `invariant-tests` names only HA-02 and AG05-13: every R312 row resolves.

## Final round, 25 Oct (commits 5ce8d562, 76f1c6d3 and this report)

STATUS is BLOCKED, not DONE, for base and environment items only; R3-12's code is complete. Each
red gate below is named with the proof that it is red without R3-12 (the newest `origin/feat/db-fix`,
`666ddf7f`, checked out as a worktree in the scratchpad and run the same way).

- Merged the newest `origin/feat/db-fix` (`666ddf7f`: R3-13, DATA-1w) as `5ce8d562`. Conflicts only
  in the three append-only ledgers (`docs/04-invariants.md`, `05-decisions.md`, `06-limits.md`):
  both sides kept whole, the base's entries first. `git diff origin/feat/db-fix HEAD -- crates
  Cargo.toml Cargo.lock deny.toml` is empty: R3-12 changes no Rust file and not the lock.
- `76f1c6d3`: THE TEN-TIMES GATE'S LIST-RULES PHASE MEASURED NOTHING. `longestTask` returned before
  its first beat whenever the work held no await, so the list rules (2,000 underlyings, 4,040
  expiries, seven segment keys) read 0.00 ms in all 20 runs. The last stretch is now timed; new test
  `the_timer_times_a_stretch_with_no_await` fails before (a 100 ms busy stretch read 0 ms: `not ok 2`)
  and passes after. Invariant R312-09 names it. Each run writes one TAP diagnostic line.

### Gates, with their commands

| gate | command | result | red at base? |
|---|---|---|---|
| fmt | `cargo fmt --all --check` | exit 0 | - |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | - |
| deny | `cargo deny check` (cargo-deny 0.20.2, installed here) | RED: advisories FAILED on RUSTSEC-2026-0386 (tokio 1.53.1, `AsyncFd::new` unsound) only; bans, licenses, sources ok | BASE ITEM: the lock is the base's byte for byte; BASE-DENY-1024 bumps tokio to 1.53.3 on the Mac. Not bumped here. |
| test | `cargo test --workspace --locked --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held` | RED, environment: 11,160 pass, 28 fail, 94 ignored, 252 targets; 5 targets red (`api`, `cli`, `pull`, `store`, `telemetry` lib) | every red is the container's (table below) |
| front-end tests | `npm test` in `web/` (Node 22.22.0) | 1,614 tests, 1,611 pass, 0 fail, 3 cancelled: `tests/ask.test.js` 2-4, "Promise resolution is still pending but the event loop has already resolved" | not R3-12's file; cancelled the same way since the first baseline (Node 22 here, CI runs Node 24) |
| W3 svelte-check | `npm run check` | RED AT BASE: 44 errors, 2 warnings, 11 files; base: 44 errors, 2 warnings, 11 files, the same per-file counts (read-pools.js 8, ReadPools.svelte 7, feed-checks.test.js 12, read-pools.test.js 8, ingest +page 2, database-pages.test 2, settings-ceilings.test 2, gap-verdict 1, ingest-months 1, ingest-months.test 1) | yes; none in a file R3-12 touched |
| W4 unused CSS | `npm run build`, count of "unused css selector" | 0 | - |
| build | `npm run build` | exit 0; `web/build` restored (`git checkout -- web/build && git clean -fd web/build`), never committed | W1 red at base (last round: 55 files) |
| gate 27 | `rustc --edition=2024 -D warnings .github/gates_ledger.rs -o gl && ./gl invariant-ids` | OK, 3,276 rows unique | - |
| gate 27b | `./gl decision-numbers` | RED AT BASE: D-6247 heads two entries (lines 81344, 81588); the base gives the same | yes |
| gate 10 | ci.yml's gate 10 steps run by hand (source_scan, invariant_paths, `gl invariant-tests`) | RED AT BASE: the same three rows, R427-11, HA-02, AG05-13; the output equals the base's but for the row count (4,866 here, 4,853 base); no R312 row named | yes |
| Rust coverage, mutants | `cargo llvm-cov --branch`, `cargo mutants --in-diff` | NOT APPLICABLE: the diff touches no `.rs` file, so there is no touched Rust module to cover or mutate | - |
| node coverage (information) | `node --test --experimental-test-coverage` over the 21 census, store, feed, selection, bars-plan, seam, address, preparation, ten-times and no-typed-lists test files | 194 pass, 0 fail. Line / branch: store-census.js 99.65 / 91.24, store-census-wire.js 99.39 / 95.35, census-worker.js 98.27 / 84.62, selection-model.js 100 / 93.99, bars-plan.js 100 / 98.08, feed-row.js 100 / 85.45, feed-summary.js 100 / 93.55; `.svelte.js` and `.svelte` files are compiled at test time and not counted by node. Not 100% branch; the law's 100% gate is per crate and R3-12 touches none. Said plainly, not claimed green. | - |

The 28 cargo reds, by cause (this container: 4 cores, runs as root, 30 GB free this time, so last
round's GDFL `disk_full` floor reds are gone):

| target | fails | cause |
|---|---|---|
| api lib | 9 | the 8 named in round four (root reads a 000 directory, five slow-reader `head_deadline_body_tests`, `the_journals_own_extremes`, `a_refused_admission_names_what_its_bound_and_why`) and `logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code`, which passes alone (`cargo test --locked -p api --lib -- logs::r3_24_tests::a_two_week_old`: 1 pass) |
| cli lib | 3 | `scrub_blocks` unwritable ledger, two `watchdog_command` read-only tests: root writes anyway |
| pull lib | 9 | read-only and refused-open tests root passes (`alerts`, `gdfl_dropbox` x2, `gdfl_import::ia16`, `ingest::census_store` x2, `zerodha_verify` x2) and `gdfl_cm::tests::fixtures_are_built_not_pasted` (a check over an unchanged cli file, red at the base inputs last round) |
| store lib | 3 | `settings` tests needing an unreadable or unwritable directory |
| telemetry lib | 4 | `sink` tests needing a refused rename, delete or look-up |
| skipped | 2 | the two 4-core `Barrier` hangs explained in round four (8 holders, 4 slots) |

No red is in a file R3-12 changed or reads a file R3-12 changed.

### Measurements: p50 / p99 / max

| what | n, method | p50 | p99 | max |
|---|---|---|---|---|
| ten-times gate, spot view open, longest task | n = 20 runs, one `node --test tests/db-ten-times.test.js` process each, this container | 3.68 ms | 6.53 ms | 6.53 ms |
| ten-times, options view with no expiry (4,040 expiries offered) | same | 8.35 ms | 13.98 ms | 13.98 ms |
| ten-times, a chosen expiry's chain | same | 8.46 ms | 12.89 ms | 12.89 ms |
| ten-times, the page's list rules (2,000 underlyings a segment, 4,040 expiries, 7 keys) | same | 14.59 ms | 23.78 ms | 23.78 ms |
| ten-times, spot open on the wire | same | 40,101 B | 40,101 B | 40,101 B |
| contract listing read at 404 / 4,040 expiries | n = 200, node, fake server (round four; `docs/06-limits.md` L-R312-01) | 0.79 / 4.13 ms | 2.60 / 8.68 ms | - |
| `scopeNeed`, `scopeWanted`, `planRead` | n = 2,000 each, node (L-R312-03) | - | 2.1 / 3.1 / 4.0 µs | 0.35 / 0.11 / 1.53 ms (a collection pause) |

With n = 20, p99 is the maximum. Every number is node on invented data; the browser targets (first
content, memory, wire, picker click) are M16 and M17 below, unmeasured here.

### Extreme cases (tests in web/tests; before and after as rounds three and four record: every one fails at base but the silent-worker test, which passes at base and is R3-12's proof of WP-56b's ceiling)

1. Empty: a feed with no spot segment; the empty instrument (the store's own answer).
2. One record: a scope of one row.
3. Maximum: the server's 4,096-row cap and one past it; a chain past the cap; ten times the list (`db-ten-times`).
4. Dates: today, the archive's end, one day past it, a year turning; an index's months after its newest file owed to the feed's span end (`an_index_owes_its_months_to_the_feeds_span_end`).
5. Out of order: an out-of-order listing.
6. Torn: torn root, rows and listing bodies ("its body is not JSON", never thrown).
7. Renamed or odd symbols: `BAJAJ-AUTO` against BAJAJ, an ampersand symbol.
8. Crash or stall: a silent worker dropped at 10 s, rows still shown (`a_silent_worker_is_dropped_and_the_page_says_so`); a superseded read never published.
9. Garbage input: unknown view keys (`seg=index`) refused before any request.
10. COMBINATION: a saturated scope with a moved generation.
11. COMBINATION: ten times the expiries and a chain past the cap.
12. COMBINATION: a segment switch in flight with a failing read.
13. OUT OF THE BOX: the gate's own timer could not see a stretch with no await (`the_timer_times_a_stretch_with_no_await`).

### Files outside ownership (each minimal)

- `web/src/routes/db/DbSearch.svelte`: the one permitted line, the Find placeholder prefixed `e.g. `.
- `web/tests/database-preparation.test.js`: a source-shape pin updated to R3-12's shape.
- `web/tests/db-bars-plan.test.js`: `spanFloor` / `FLOOR_YEAR` now asserted absent (RA-258).
- `web/vite.config.js`: the dev proxy lists `/census/rollup.json` and `/settings.json`.
- `web/tests/store-module.js`: a new shared test helper (new test files are allowed).
- `docs/04-invariants.md`, `05-decisions.md`, `06-limits.md`: R3-12's own rows only (R312-01..11, D-7111, L-R312-01..07).
- Named, NOT edited (typed lists outside R3-12's files, L-R312-06): `DbCalendar.svelte`'s feed list (WP-54, R3-02), `notes-coverage.js`'s dates (R3-04), the store segment words in `notes-labels.js` and `dayflags.js` (R3-13), `completeness.js` (R3-04).


## Left (NEXT)

- Nothing left for the cloud. At merge-back: the NEEDS THE MAC list below, a fresh `web/build`, and
  the base items (BASE-DENY-1024's tokio bump; gates 10, 27b and svelte-check red at base).

## Log

### 2026-10-24 (IST) start

- GO present, HOLD absent, spec present. Branch created.
- Baseline `npm test` in `web/` (Node 22.22.0, this container): 1568 tests, 1563 pass, 0 fail,
  5 cancelled: `tests/ask.test.js` 2-4 (not R3-12's file) and `tests/census-worker.test.js` 9-10
  (R3-12's): "Promise resolution is still pending but the event loop has already resolved".

### 2026-10-24 09:31 IST save point, HOLD found

- HOLD present on `origin/cloud/db-base-20261024`. Finished the step in progress (FM-621 wiring and
  its svelte-check), committed `1655f44b`, pushed, waiting for GO without HOLD.

### 2026-10-24 12:38 IST resumed (worker restarted; HOLD lifted at 55dfb044)

- `f98a30a9`: no-whole-census test, Coverage roll-up, typed words out of DbCoverageGrid.
  `npm test` before it: 1576 tests, 1567 pass, 4 fail (source-shape pins of other owners'
  tests and proxy.test.js's missing `/census/rollup.json`), 5 cancelled (ask.test.js 2-4,
  census-worker 9-10). The four were fixed: two shapes kept in R3-12's code, two pins updated
  minimally (database-preparation.test.js, the proxy line in vite.config.js); then
  `node --test` of those files plus census-scope and census-first-load: 78 pass, 0 fail.
- `785ec646`: ceilings from `/settings.json`, the build check, `seg=index` sentence, DbSearch
  placeholder. `node --test tests/no-typed-lists.test.js`: 3 pass.
- `5a93cc1e`: silent worker, ten-times gate (5 runs green), `yieldTask` ref fix.
  `node --test tests/census-worker.test.js`: 11 pass, 0 cancelled.

### 2026-10-24 12:56 IST save point, HOLD found again

- HOLD present again. Step finished (task 4), committed `5a93cc1e`, pushed; waiting for GO.

### 2026-10-24 18:05 IST resumed (worker restarted; HOLD lifted)

- Merged `origin/feat/db-fix` (`34b28d51`); extreme cases and the parseKey / torn-body hardening
  (`7548d5fd`): `node --test tests/census-scope.test.js` 29 pass, `tests/census-first-load.test.js`
  9 pass. svelte-check 33 then 32 after typing the fake server's answers (`b422255d`).

### 2026-10-24 18:25 IST save point, HOLD found a third time

- HOLD present (18:16 IST note: expected lift at the latest after 22:30 IST). Step finished,
  `b422255d` pushed, report HELD; the cargo workspace build started before the hold runs on
  locally. Waiting for GO.

### 2026-10-24 19:47 IST resumed (GO-R3-12 released the part)

- Cargo gates, DB-B47's page half, D-7111 and the ledger rows (round four above).

### 2026-10-24 20:30 IST save point, HOLD applies again

- Every GO-<NAME> removed at 20:20 IST (`604b8f20`), HOLD present. Step finished, `01889efe`
  pushed, report HELD. The `--no-fail-fast` cargo run continues locally. Waiting for HOLD to go.

### 2026-10-25 IST session resumed, still HELD

- New cloud session on the existing branch at `cebde51d`. `origin/cloud/db-base-20261024` at `40f1cdb2`:
  HOLD present, per-part GO only for R4-09b, IA-10a and IA-17b; no GO-R3-12. No step started; waiting.

### 2026-10-25 13:12 IST resumed (HOLD lifted at `1dcc33e9`: all nine cloud parts build)

- Merged `origin/feat/db-fix` `666ddf7f` (`5ce8d562`); the ten-times timer fix (`76f1c6d3`); every
  gate run (Final round above). STATUS BLOCKED on base and environment items only.

## Decision numbers

D-7111 (the block D-7110..D-7119's second id, R3-12's): /db reads only what its view shows. No
other number used.

## NEEDS THE MAC

None of this can run in the cloud (no store, no shadow server, no :8080, no browser on real data).
Read-only, never on :8080. `<W>` = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`, the
worktree `wt/P-R3-12`, port 8423, `B=http://127.0.0.1:8423`.

Setup (hold the pool-5 lock and touch the tree first, OPS-09/OPS-10):

````text
W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925
git -C $W/wt/DBFIX worktree add $W/wt/P-R3-12 -b fix/db-R3-12 feat/db-fix
git -C $W/wt/P-R3-12 merge --no-ff origin/cloud/db-R3-12-20261024
ln -s $W/wt/APP/web/node_modules $W/wt/P-R3-12/web/node_modules
export CARGO_TARGET_DIR=$W/tgt/pool-5          # mkdir $W/tgt/pool-5.lock ... rmdir after the last cargo command
(cd $W/wt/P-R3-12 && git ls-files -z | xargs -0 touch)
(cd $W/wt/P-R3-12 && cargo build --release -p api -p cli)
mkdir -p $W/tgt/bin-R3-12/release && cp $CARGO_TARGET_DIR/release/api $CARGO_TARGET_DIR/release/cli $W/tgt/bin-R3-12/release/
(cd $W/wt/P-R3-12/web && npm run build && npm test)
$W/state/fix/stop.sh R3-12; $W/state/fix/shadow.sh R3-12 && $W/state/fix/serve.sh R3-12 8423 $W/tgt/bin-R3-12 $W/wt/P-R3-12
lsof -nP -iTCP:8423 -sTCP:LISTEN -t     # must print the pid in $W/shadow/R3-12/test-server.pid
````

| # | what | command | expected answer |
|---|---|---|---|
| M1 | the facets root lists the feed's segments | `curl -s "$B/census/facets.json?feed=gdfl" \| jq '[.segments[].segment]'` | the segments in the server's order (INDEX, CASH, FNO as held); the page's chips and counts come from this answer only |
| M2 | NIFTY FNO rows with no expiry are still refused | `curl -s -o /dev/null -w '%{http_code}\n' "$B/census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1"` | 400 `rows_too_many` (cap 4,096): the page must not ask it |
| M3 | the options view's expiries come from the facets answer | `curl -s "$B/census/facets.json?feed=gdfl&segment=FNO&underlying=NIFTY" \| jq '.expiries \| length'`; size and time with `curl -s -o /dev/null -w '%{size_download} %{time_total}\n'` | a count (404 on 12 Oct) equal to what M6 offers; 12 Oct: 77,947 B in 7 ms |
| M4 | Coverage numbers come from the roll-up | `curl -s "$B/census/rollup.json?feed=gdfl&segment=FNO&underlying=NIFTY" \| jq '.months \| length'` (and `segment=INDEX`, and `feed=zerodha`) | month rows with their rungs; `rollup_too_many` only past 4,096 cells; the /db Coverage tab shows the same |
| M5 | NIFTY gdfl 1s spot view | open `$B/db?feed=gdfl&seg=spot&sym=NIFTY&tf=1s` in a fresh profile, read the network log | no `segment=FNO` request; no 400; no `/store.json?...encoding=census-tuples-v1`; NIFTY spot rows; the banner names a scoped read |
| M6 | options view with NO expiry (12 Oct acceptance case) | open `$B/db?feed=gdfl&seg=options&sym=NIFTY&tf=1s` | expiries offered at once (count = M3); no FNO `rows=` request without an expiry; no `/store.json`; JS heap under 100 MB (12 Oct: 53,131,576 B, 8-18 s, 465 MB) |
| M7 | the same view with an expiry | open `$B/db?feed=gdfl&seg=options&sym=NIFTY&tf=1s&exp=2026-09-22` | a handful of list requests of a few ms (12 Oct: 5 of 1-2 ms, 16 MB); rows drawn |
| M8 | an unknown segment word | open `$B/db?feed=gdfl&seg=index&sym=NIFTY` | refused by name (the word and the known list); no whole-list read |
| M9 | a segment switch | on M5 pick Options, then Spot | one scope request for the new segment each time; never a segment drawn from a scope that did not read it |
| M10 | a feed switch | on M5 switch to Zerodha and back | no whole census; record time to first row |
| M11 | the Coverage tab | on M5, a BANKNIFTY and a RELIANCE view | no whole census; no task over 50 ms; counts equal M4 |
| M12 | lists and limits from the server | `curl -s "$B/settings.json" \| jq '.ceilings'`; `curl -s "$B/feeds.json" \| jq -c '[.. \| objects \| .wire? // empty]'` | the page's 'x of 1,024 series', 'x of 240 months', feed list, window and opening instrument equal these; only Zerodha and GDFL offered |
| M13 | the window label | open `$B/db?feed=zerodha&seg=spot&sym=NIFTY&tf=1day` | 2015-2017 months hidden by default, shown under 'outside your window' |
| M14 | DB-B47 page side | open `$B/db?feed=zerodha&seg=spot&sym=NIFTY&tf=1min` over 2026-09..2026-12, and `curl -s "$B/bars/window.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&from=2026-09&to=2026-12&sort=ts&dir=desc&limit=2" \| jq '{months_missing, missing_months, months_outside_life}'` | absent months of an index are named holes, not 'outside life'; the Coverage grid draws them as absent rows to the wider of `archive.to` and `window.to` |
| M15 | NIFTY FIRST repeats | M5, M6, M9 on a BANKNIFTY selection, RELIANCE (CASH) and one illiquid strike | the same answers through the one code path |
| M16 | the scope read's cost, before and after | n = 20 page loads of M5 on the feat/db-fix build and on this build; n >= 200 sequential `curl` of M1, M3, M4 and the scoped rows request | p50, p99, max written down with n and method |
| M17 | the four page-side targets | browser runs, n = 200 cold and warm, of /db first load and a feed switch; a picker click to its list at today's list (by hand: R3-15's harness is not merged) | first content p99 under 300 ms; no pause over 50 ms; memory under 100 MB; under 400 KB on the wire (before: feed switch p50 815 / p99 972 ms; first row p50 1,112 / p99 1,285 ms; memory p50 279 MB) |
| M18 | the silent worker | pause the census worker in devtools on an option view | dropped at 10 s; rows still shown; the page says the worker was dropped |

Then `$W/state/fix/stop.sh R3-12`. Store writes: none.

Gates the container could not make green (run on the Mac after the merge):

| # | command | expected answer |
|---|---|---|
| R1 | `cargo test --workspace --locked` (8+ cores, not root) | green, including the two `Barrier` tests skipped here and the 28 root, timing and fixture reds listed above; any red also red on `feat/db-fix` is a base item |
| R2 | `cargo deny check` after BASE-DENY-1024 (tokio 1.53.3) | advisories ok; R3-12 does not touch the lock |
| R3 | `cd web && npm run check` | the base's count (44 errors in 11 files on 25 Oct); none in an R3-12 file |
| R4 | `cd web && npm test` under Node 24 | 0 fail, 0 cancelled (`ask.test.js` 2-4 are cancelled under Node 22 here) |
| R5 | `cd web && npm run build`, then commit `web/build` at merge-back | W1 green (the cloud never commits `web/build`) |
| R6 | gates 10 and 27b | the base's reds only (R427-11, HA-02, AG05-13; D-6247) |

STATUS: HELD

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

## Left (NEXT)

- NEXT: when HOLD is gone (or a GO-R3-12 returns): the per-crate cargo results are in (above);
  `cargo deny` (not installed here: `cargo install cargo-deny --locked`, else NEEDS THE MAC), one full
  `npm test`, svelte-check and `npm run build` (then `git checkout -- web/build`), the ten-times
  numbers at n = 20, then the final report: STATUS, every gate with its command, p50/p99/max, the
  extreme-case list, files outside ownership, D-7111, and NEEDS THE MAC M1-M18.

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

## Decision numbers

D-7111 (the block D-7110..D-7119's second id, R3-12's): /db reads only what its view shows. No
other number used.

## NEEDS THE MAC

To be completed with the final report (spec section 6, M1-M18).

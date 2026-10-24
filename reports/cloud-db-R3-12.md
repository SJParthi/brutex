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

## Left (NEXT)

- NEXT: on GO: extreme-case tests (one-record listing, today/archive-end/year boundaries, out-of-order
  listing, mixed garbage views, a superseded read never published, two combinations), fails-before
  proof of every new named test against the base, merge the newest `origin/feat/db-fix`, the gates
  (cargo fmt/clippy/test/deny, web test/build, svelte-check, CI shell gates), final report.

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

## Decision numbers

None used yet. D-7111 is reserved for R3-12 only if it locks a choice.

## NEEDS THE MAC

To be completed with the final report (spec section 6, M1-M18).

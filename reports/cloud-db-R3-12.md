STATUS: IN PROGRESS

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

## Left (NEXT)

- NEXT: on GO, task 2 rest: `no_db_view_downloads_the_whole_census` test; Coverage grid store-wide
  figures from `/census/rollup.json` (NEW-07) and its typed words; then task 3 (ceilings, opening
  instrument and floor year from the server, `no_typed_feed_kind_or_date_in_page_logic`), task 4
  (silent worker, ten-times gate, `yieldTask` ref fix), task 5 (gates, extreme cases, report).

## Log

### 2026-10-24 (IST) start

- GO present, HOLD absent, spec present. Branch created.
- Baseline `npm test` in `web/` (Node 22.22.0, this container): 1568 tests, 1563 pass, 0 fail,
  5 cancelled: `tests/ask.test.js` 2-4 (not R3-12's file) and `tests/census-worker.test.js` 9-10
  (R3-12's): "Promise resolution is still pending but the event loop has already resolved".

### 2026-10-24 09:31 IST save point, HOLD found

- HOLD present on `origin/cloud/db-base-20261024`. Finished the step in progress (FM-621 wiring and
  its svelte-check), committed `1655f44b`, pushed, waiting for GO without HOLD.

## Decision numbers

None used yet. D-7111 is reserved for R3-12 only if it locks a choice.

## NEEDS THE MAC

To be completed with the final report (spec section 6, M1-M18).

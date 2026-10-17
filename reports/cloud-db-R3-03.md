STATUS: IN PROGRESS

# R3-03 /db bar grid: pager, Go to, sort offers, requests per change, and the words in an empty or refused grid

Branch `cloud/db-R3-03-20261017`, cut from `origin/feat/db-fix` at `50093fa3`
(Merge R3-35). Prerequisite R3-01 is merged (`00c07ee0 Merge R3-01: ...`).

## Done

- `web/src/lib/db/pager.js` (new): the server's depths mirrored
  (`MAX_SORT_DEPTH`, `MAX_READ_HELD`, checked against the Rust source by a
  test); `servedPages` / `isServed` / `lastServed` / `clampServed` /
  `stepServed` / `pageItems` — the pager offers only pages the server serves
  (a time-of-day order stops at row 100,000; a value order reads its own
  `sort_index` report; both ends when nothing was indexed); `parsePageBox`
  refuses 0, -5, 2.5, 1e308, empty, letters, 6,000 digits, past-the-end and
  dead pages in words; `pageToRead` + `askOnce` — one request per change;
  `generationOf` / `joinPage` / `joinSlices` — two pages (and a whole read's
  slices) are checked against the total and the store state, repeats dropped
  and the move named.
- `web/src/lib/db/goto.js` (new): `parseGoto` (moved from `bars-plan.js`)
  takes a time alone, `H:MM`, seconds, and refuses empty, nonsense, 6,000
  characters (never echoed), impossible days and times, and days outside the
  held span or the date window (1969, 2099) in words; `gotoAsk` asks the
  instant ASCENDING whatever the direction; `landing` lands on the minute's
  first bar in both directions (newest first used to mark the bar before).
- `web/src/routes/db/DbPager.svelte` (new): the pager, moved out of the grid
  (the grid's `pager` snippet now renders it, so the shell is untouched).
  Readout is the page the rows on screen belong to, with "opening page N…"
  while one is read; page box is text and refuses in words; buttons, Next,
  Last and the box never reach a page the server refuses and the stretch
  says why; "Fit · measuring" instead of "Fit · 50" before the box is measured.
- `web/src/routes/db/DbBarsGrid.svelte`:
  - one request per change: the read effect asks page one itself on a change
    of order (no old-offset request first), dedupes by question, waits for the
    selection to settle (`db.settled`) and for a Go to's landing, is not
    cancelled by a re-run that asks nothing new, and stops a read for a row set
    that has moved on;
  - newest close taken from the extremes read or page one newest first; asked
    alone only when a bound narrows or neither has it, once per selection;
  - sort offers: Volume on an index and open interest on an index or a share
    are disabled with the reason; a sort carried onto such a selection reads
    time order and says so; a column the browser cannot afford is refused
    BEFORE the sort moves (keeps the sort that was on) with advice that works
    for the rung ("a day alone is not enough" on 1s);
  - a Change % measured across the overnight gap carries an "overnight" mark
    with the bar it was measured against (`acrossDay`, only where known);
  - failure words: a refusal the page made is not "the server's own" nor "a
    failed read"; Try again only where it can help, and it re-asks the page
    (not the census); the HTTP status is recorded per read;
  - empty states: no answer yet draws loading rows (no "census and files
    disagree" flash at open); the disagreement is said only when the census has
    not moved; an empty time-of-day window is not blamed on the import;
  - Fit: the heading's two rows are measured (`fitHead`), slack back to 8 px,
    and the notes strip is always one line in the box (the rest open over the
    grid on a press), so no notice cuts a row;
  - Go to scrolls to and marks the landed row, again on a second Go to to the
    same minute, only on the rows of the page it names.
- `web/src/routes/db/DbExtremes.svelte`: the fold waits on `db.barsReady`,
  asks a set once, is not cancelled by a no-op re-run, and carries the newest
  bar for the grid.
- `web/src/lib/db/bars-plan.js`: `acrossDay`; `parseGoto`, `minuteOf`,
  `pageList` moved/removed; dead `pooled`/`IN_FLIGHT` removed; `FIT_SLACK` 8.
- `web/src/lib/db/notes-bars.js`: `segmentOfKey`, `sortRefusal`,
  `localSortAdvice`, `readFailure`; dead `READ_BUDGETS`/`budgetLabel` removed.
- Tests: `web/tests/db-pager.test.js`, `web/tests/db-goto.test.js` (new);
  `db-bars-plan.test.js` updated. The six named tests exist:
  `a_sort_change_at_a_deep_page_sends_one_request`,
  `the_pager_offers_no_page_past_the_served_depth`,
  `go_to_a_minute_lands_on_its_first_bar_in_both_directions`,
  `no_sort_is_offered_on_a_column_without_values`,
  `a_page_made_refusal_is_not_called_the_servers`,
  `the_store_growing_between_two_pages_repeats_no_row`.

### ONE EDIT OUTSIDE THE OWNED LIST — please check at merge-back

`web/tests/database-page-fixture.js`: three lines added to `DB_PAGE_FILES`
(`DbPager.svelte`, `pager.js`, `goto.js`). `web/tests/db-seam.test.js` fails
for any file under `routes/db/` or `lib/db/` that is not listed, and this part
is told to create exactly these three. Every earlier part that added a file
there did the same in its own change (R3-01 `1f9b9b4d`, WP-57 `cf3cf4ed`,
WP-81 `83b29639`). If the Mac prefers to own that line itself, drop the hunk
and add the same three paths at merge.

## Left

- Mutation run (Stryker 10.0.0, scratch install, sandbox copy) on `pager.js`
  and `goto.js` is running; `bars-plan.js` and `notes-bars.js` next.
- `cargo clippy`, `cargo test --workspace --locked`, `cargo deny check` on the
  branch and on the base.
- Final full web suite, report STATUS.

## Gate results (so far)

- `cargo fmt --check`: clean.
- Web tests (`node --test tests/*.test.js`, Node 22.22.0): branch 1,407 pass,
  0 fail, 5 cancelled; the 5 are timing tests in `ask.test.js` (3) and
  `census-worker.test.js` (2), cancelled identically on the base (1,393 pass,
  0 fail, 5 cancelled) — files this part does not own.
- `svelte-check` (Gate W3): 6 errors, 0 warnings — the same 6 as the base,
  all in files this part does not own (`ingest-months.js` `openEnd` ×4 sites,
  `settings-ceilings.test.js` implicit any ×2). None in a touched file.
- Gate W5 (`node web/ci/css-comments.mjs` on the three touched components):
  clean.
- `vite build` in a scratch copy: succeeds (only the existing chunk-size
  warning).
- Coverage (`node --test --experimental-test-coverage`, whole web suite):
  `pager.js`, `goto.js`, `bars-plan.js`, `notes-bars.js` 100% lines and 100%
  branches. Svelte components are not measured by any tool in this setup.

## NEEDS THE MAC

To be completed with exact commands in the final report.

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
    selection's reconcile to finish (`db.reconciling`, not `db.settled`, which never settles for census rows with no first or last stamp) and for a Go to's landing, is not
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
- `web/src/routes/db/DbExtremes.svelte`: the fold waits on `db.barsReady` (Fit applied, no reconcile),
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

Nothing here was measured in the cloud: there is no store, no `:8080` and no
real data. Every number the goal asks for is the Mac's to take.

Set-up, the way every package is checked on the Mac: merge this branch into a
scratch copy of `feat/db-fix`, then

```
npm --prefix web ci && npm --prefix web run build   # web/build is tracked (Gate W1); this part does not commit it
cargo build --release -p api
```

then start that build on a FRESH shadow store at a port of its own, `$P`, with
the Mac's `state/fix/shadow.sh` and `state/fix/serve.sh` exactly as they start
every package's check (never `:8080`, never the real store). Below,
`H=http://127.0.0.1:$P`.

1. **Gate W1** — after the merge, `npm --prefix web run build` and commit
   `web/build` (Gate W1: `git diff --exit-code -- web/build`). This part does
   not own `web/build`, so it is not committed here.

2. **The server rules the page mirrors** (exact curl, one-second option
   contract of the 11 Oct re-check, 153,107 rows):
   ```
   K=NSE-FNO-NIFTY-2019-03-28-1160000-CE
   curl -s "$H/bars/window.json?feed=gdfl&timeframe=1s&from=2019-03&to=2019-03&key=$K&sort=tod&dir=desc&offset=99992&limit=8&extremes=0" | head -c 300   # 200, a page
   curl -s "$H/bars/window.json?feed=gdfl&timeframe=1s&from=2019-03&to=2019-03&key=$K&sort=tod&dir=desc&offset=100000&limit=8&extremes=0" | head -c 300  # sort_too_deep
   curl -s "$H/bars/window.json?feed=gdfl&timeframe=1s&from=2019-03&to=2019-03&key=$K&sort=c&dir=desc&offset=0&limit=8&extremes=0" | grep -o '"sort_index":{[^}]*}'  # what the pager reads to offer pages
   # Go to 2019-03-15 10:02 (1552624320): ascending, the minute's FIRST bar
   curl -s "$H/bars/window.json?feed=gdfl&timeframe=1s&from=2019-03&to=2019-03&from_day=2019-03-15&to_day=2019-03-15&key=$K&sort=ts&dir=asc&at=1552624320&limit=1&extremes=0" | grep -o '"at_index":[0-9]*\|"t":[0-9]*' | head -2
   ```
   Expect the last line's `t` ≥ 1552624320 and < 1552624380 if the contract
   traded in 10:02; the grid's newest-first landing is `total − 1 − at_index`.
   (Correct the key if the census names the 11,600 CE differently.)

3. **Requests per change, in the browser** (Chromium, DevTools Network
   filtered to `window.json`, cache disabled; or the Mac's Playwright
   harness). Open each and count `/bars/window.json` requests after the page
   settles; target **1 page read + 1 extremes read** per change, and **no
   request for an intermediate selection**:
   - `$H/db` (default NIFTY spot, finest rung): 2 on open (page, extremes);
     the Rows per page summary reads `Fit · measuring` then `Fit · N`, never
     `Fit · 50`; exactly one page read (no limit=4, 3, 8).
   - `$H/db?feed=gdfl&sym=NIFTY&seg=spot&tf=1s&page=900`: click the Close
     header — exactly 1 request, `offset=0`; no request at
     `offset=(900−1)·N`.
   - NIFTY 26 Mar 2020 12,000 CE → PE (Side), and BANKNIFTY options → Spot
     (Segment): each change 1 page read + 1 extremes read; no request names a
     key or month the final selection does not hold.
   - Next page twice, Prev once: 1 request each, the readout's "page N"
     always matches the rows on screen ("opening page N…" while one loads).

4. **Pager depth** — the 153,107-row contract, Time sort (`&sort=tod`),
   Rows per page 8: page buttons stop at 12,500 with a ⋯ that names
   `sort_too_deep`; Last lands on 12,500; Next is disabled there; typing
   12501, 19139, 0, -5, 2.5, 1e308, empty and 6,000 digits in "Go to page"
   each shows a sentence under the box and moves nothing. Repeat under Close:
   if the server's `sort_index` says every row came from an index (or ≤
   1,000,000 were read), every page is offered.

5. **Go to** — the 11,600 CE on 15 Mar 2019, 10:00–10:04: pick 10:02 in
   the minute picker, newest first and oldest first: the marked row is the
   minute's first bar (not 10:01:5x) and is on screen. First page, then 10:02
   again: marked again. Type `10:02`, `10:02:07`, `1969-12-31 23:59:59`,
   `2099-01-01 10:00:00`, `2019-02-30 10:00:00`, `25:61:61`, `abc`, empty, and
   6,000 characters: a time lands; every other one is refused in words and
   moves nothing.

6. **Sort offers and words** — NIFTY spot · Zerodha 1min (01–30 Sep 2026):
   Volume and Open interest headers are disabled with the n/a reason; Change %
   sort shows `overnight` on each row measured across the gap. NIFTY spot ·
   GDFL 1s whole series: Change % is refused with the "a day alone is not
   enough" advice and the grid keeps its sort and page. `&sort=dte&dir=desc`
   on the whole contract: refused in words, back to time order. A too-many-
   series selection (or a window outside the held months): "This selection is
   not read as one page", no Try again. `&tfrom=03:00&tto=04:00` on 15 Mar
   2019: the time window is named, no import blamed. Page open on every
   selection above: no "census and the files disagree" flash.

7. **The four page-side targets** (the goal's measured target), in the
   Mac's browser harness, n = 200 cold and n = 200 warm, on NIFTY spot GDFL 1s
   and on the 153,107-row contract: first content p50/p99/max (< 300 ms at
   p99), longest main-thread pause (< 50 ms), JS heap (< 100 MB), bytes on
   the wire per page (< 400 KB). Write p50, p99, max and n for each. "Shown
   on: the Speed page" — no Speed page exists in this tree; its owner shows
   these numbers.

STATUS: DONE

Every gate this part can move is green: fmt, clippy, the web suite (0 failures), Gate W5, and 100% line, branch and mutation coverage on the four touched JS modules. Four gates are red ONLY with the base's own failures, each shown identical on `origin/feat/db-fix` `50093fa3` below: `cargo deny check` (the five advisories DENY-1 owns), `cargo test --workspace` (two `store` tests: `cited_commits`, and an inode-reuse test this ext4 disk fails), `svelte-check` (the base's 6 errors, none in a touched file) and the web suite's 5 cancelled timing tests. This part changes no Rust. The task's DONE rule names only deny as an allowed base failure; the other three are reported here, not hidden — the Mac decides.

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

Nothing in the owned files. What only the Mac can do is listed under NEEDS
THE MAC: the real-data, `:8080`-style browser and timing checks, the
`web/build` regeneration, and numbering the decisions and invariants below.

## Gate results

Taken on the final commit of this branch. This part changes no Rust:
`git diff origin/feat/db-fix -- Cargo.toml Cargo.lock deny.toml crates` is
empty, so every cargo gate is the base's own result, run on this branch.

- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean
  (exit 0).
- `cargo deny check`, with the CI pin 0.19.0 and with 0.20.2, on this branch
  AND on the base (`origin/feat/db-fix` `50093fa3`, a scratch worktree): both
  `advisories FAILED, bans ok, licenses ok, sources ok`, the same five and only
  those — RUSTSEC-2026-0344 (ruzstd 0.8.2) and RUSTSEC-2026-0346, -0347,
  -0348, -0349 (rustls 0.23.45). DENY-1 owns them; Cargo.toml and Cargo.lock
  are untouched here.
- `cargo test --workspace --locked --no-fail-fast` (skipping only `cli`'s
  known slow `the_pool_verb_prints_its_whole_page_on_a_generated_store`, as
  CONTEXT allows), run with root's file-access override dropped
  (`setpriv --inh-caps=-dac_override,-dac_read_search,-fowner
  --bounding-set=-dac_override,-dac_read_search,-fowner cargo test …`):
  220 suites, **9,034 passed, 2 failed**, 47 ignored. Both failures are in
  `store` and both fail identically on the base:
  - `store::staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`
    — asserts the recreated file has a NEW inode; this container's disk is
    ext4, which hands back the inode just freed (failed 3 of 3 runs here, and
    on the base). APFS on the Mac does not reuse it at once.
  - `store --test cited_commits` — `docs/06-limits.md` on `feat/db-fix`
    cites nine commits (023d6ab8, 079ad4af, 4756cce0, 521066e6, 72c91a46,
    91ef0094, 9cb9de51, cde73992, ff636f58) that are not on `origin/main`;
    a property of the base branch's docs, not of this part.
  Run as plain root first, `api`'s lib suite also failed 12 permission tests
  (unreadable directory, unlinkable name, unwritable store): uid 0 bypasses
  permission bits; with the override dropped all 12 pass (162/162 in those
  groups), and the whole-workspace figure above is with it dropped.
- Web tests (`node --test tests/*.test.js`, Node 22.22.0): **1,415 pass, 0
  fail, 5 cancelled**; the 5 are timing tests in `ask.test.js` (3) and
  `census-worker.test.js` (2), cancelled identically on the base (1,393 pass,
  0 fail, 5 cancelled) — files this part does not own.
- Fail before, pass after: with this branch's `pager.js`, `goto.js`,
  `bars-plan.js` and `notes-bars.js` copied into a base worktree and the
  base's own grid, pager and extremes left in place, `db-pager.test.js` +
  `db-goto.test.js` give **12 of 18 failing — all six named tests among
  them**; on this branch 18 of 18 pass. On the plain base both files fail at
  import (no `pager.js`, no `goto.js`).
- `svelte-check` (Gate W3): 6 errors, 0 warnings — the same 6 as the base, all
  in files this part does not own (`ingest-months.js` / `ingest/+page.svelte`
  / `ingest-months.test.js` `openEnd` ×4, `settings-ceilings.test.js` implicit
  any ×2). None in a touched file. (Gate W3's ceiling is 0, so W3 is red on the
  base already.)
- Gate W5 (`node web/ci/css-comments.mjs` on `DbBarsGrid.svelte`,
  `DbPager.svelte`, `DbExtremes.svelte`): clean.
- Gate W1: not run here — `web/build` is tracked and not owned; see NEEDS THE
  MAC 1. `vite build` of the final sources in a scratch copy succeeds (only the
  existing chunk-size warning).
- Coverage (`node --test --experimental-test-coverage`, whole web suite):
  `pager.js` 303/303 lines, 115/115 branches; `goto.js` 144/144, 65/65;
  `bars-plan.js` 583/583, 139/139; `notes-bars.js` 460/460, 46/46 —
  **100% lines and branches on every touched JS module**. No tool here
  measures coverage of a `.svelte` file; the three touched components are
  held by source-shape tests in `db-pager.test.js`, `db-goto.test.js` and
  `db-bars-plan.test.js`, the way `db-address.test.js` holds R3-01's.
- Mutation (Stryker 10.0.0, installed in the scratchpad, not in
  `package.json`; command runner over the 11 test files that drive these
  modules; run on a sandbox copy): **1,245 mutants over the four touched
  modules, 1,243 killed, 2 timeouts (both a loop increment turned into a
  decrement, i.e. an endless loop), 0 survived — 100%**. The first runs left
  83 and 264 survivors; each was killed by an exact test or the code was
  rewritten where both sides of a mutation gave the same answer (ties in
  min/max, `?? ''` defaults, boundary-redundant conditions). One real bug was
  found that way and fixed: `landing` read an `at_index` of `null` as row 0.
  `greeks.test.js` and `moneyness.test.js` read `notes-bars.js` as TEXT and
  cannot run against Stryker's instrumented copy, so they were left out of the
  mutation command (they pass on the branch). Svelte components are not
  mutation-tested by any tool here.

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

8. **cargo test on the Mac** (not root, APFS): `cargo test --workspace
   --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`
   — expect only `store --test cited_commits` red, as on the base; the ext4
   inode test and the 12 root-only permission tests should pass there.

9. **Ledger entries at landing** (`docs/` is not owned here; numbers are
   taken from the ledger's head at landing, per amendment A6). Proposed text:
   - *Decision — one request per change.* The bar read asks page one itself
     when the order or row set changes, asks a question once (`askOnce`), waits
     for the selection's reconcile (`db.reconciling`) and a Go to's landing, and
     stops a read for a row set that has moved on; the newest close comes from
     the extremes read or page one newest first, and is asked alone only when a
     bound narrows or neither carries it. Target per change: 1 page read + 1
     extremes read.
   - *Decision — the pager offers only served pages.* `pager.js` mirrors
     `bars::MAX_SORT_DEPTH` (100,000) and `bars_sort::MAX_READ_HELD`
     (1,000,000) and the answer's `sort_index`; pages past them are not drawn,
     stepped to or accepted, and the stretch is named.
   - *Decision — Go to asks ascending.* `at=` is sent with `dir=asc` whatever
     the grid's direction; the landing is `total − 1 − at_index` newest first;
     a minute lands on its first bar.
   - *Decision — Fit measures the heading; the notes strip is one fixed line.*
     `FIT_SLACK` 24 → 8; the strip is always in the box; more notes open over
     the grid on a press.
   - *Decision — failure words by who refused* (`readFailure`): page / server
     (4xx) / failed; Try again only for failed, and it re-asks the page.
   - *Invariants* (`docs/04-invariants.md`, each beside its test in
     `web/tests/db-pager.test.js` or `db-goto.test.js`): one request per change
     (`a_sort_change_at_a_deep_page_sends_one_request`); no page past the served
     depth is offered (`the_pager_offers_no_page_past_the_served_depth`); Go to a
     minute lands on its first bar in both directions
     (`go_to_a_minute_lands_on_its_first_bar_in_both_directions`); no sort on a
     column without values (`no_sort_is_offered_on_a_column_without_values`); a
     page-made refusal is not called the server's
     (`a_page_made_refusal_is_not_called_the_servers`); a store that grows
     between two pages repeats no row
     (`the_store_growing_between_two_pages_repeats_no_row`).
   - `docs/06-limits.md`: nothing new is unbounded. `joinSlices` is O(rows) once
     per local-sort read (≤ 20,000, the existing budget); `acrossDay` is O(rows
     log rows) once per read; `pageItems` draws at most 16 items at any page
     count.

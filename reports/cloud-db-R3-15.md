STATUS: IN PROGRESS

# R3-15 (cloud): Speed page, a browser harness for every page, a speed run at every deploy, page budgets

Branch `cloud/db-R3-15-20261017`, from `origin/feat/db-fix` at `f9cdb584` (Merge R3-14). No Rust
file is touched; every change is under `web/` plus this report.

## Prerequisites

Both merged into `origin/feat/db-fix` (checked with
`git log origin/feat/db-fix --merges --format=%s | grep -E "^Merge (R3-14|WP-58):"`):

- `Merge R3-14: Measured everywhere: p50 and p99 per route kept by the app, a limits row and a growth bench per route, one command for every check`
- `Merge WP-58: All sources page`

## What was built

| Goal item | What changed | Tests (each fails on the base: the module or rule is absent) |
|---|---|---|
| (1) Speed page: p50, p99, max per route for the running build, from `/speed.json`, with the build stamp | New `web/src/routes/speed/+page.svelte`: three windows (last hour, last minute, since the start), routes and stages, four orders, the cold first request after the start, the server's resolution sentence, the build from the answer's own `x-brutex-build` header, over-cap counts. A window with no request prints a dash, never `0 µs`; a p99 past the 300 ms first-content target is marked. It asks `/speed.json` once and on Read again only: no poll. The rules are in new `web/src/lib/speed.js` (`readSpeedReport` refuses a report missing any field by name). A Speed tab in the nav. | `a speed report is read field by field…`, `a time is printed in its unit…`, `the table keeps the server's order…`, `the Speed page reads /speed.json once…` |
| (1) The "speed" tile of the health page | `speedTile(report)` in `$lib/speed.js`: routes, requests, failures and the slowest p99 over the last hour (since the start before a minute is frozen), stages left out. R3-33 (which depends on R3-15) draws it; the Speed page shows the same line. | `the speed tile names the slowest route and every failure…` |
| (2) A tracked speed run of every route, re-run after every deploy | New `web/ci/speed-sheet.mjs`: `cold` (n requests, a new connection each), `warm` at each of `--clients` (default 1, 10, 60), `warm+writer` while `--writer`'s command runs (an import or a pull writing), `first after restart` over `--restarts` (default 20) runs of `--restart`'s command; n defaults to 200. A section whose command was not given is printed NOT MEASURED. | `every phase of the sheet is run or said NOT MEASURED…`, `the sheet's own edges…`, `the sheet runs over the real network…` |
| (2) Each request taken from the page's own query builder | The catalogue calls the pages' own builders (`windowQuery`, `dayflagsUrl`, `calendarUrl`, `provenanceUrl`, `scrubUrl`, `facetsUrl`, `rungsUrl`, `dayUrl`, `barsUrl`), imported from the pages' libraries; the test checks each is the library's own export and that the page file calls it, and that no query string is typed in the catalogue. `/gaps`, `/audit` and `/markets` build their URLs inline in their `+page.svelte` (not R3-15 files), so their requests are captured from the page by the harness (`--capture`) and replayed verbatim (`--from-pages`); a capture holding a write is refused. | **`the_speed_sheet_uses_the_pages_own_requests`** |
| (2) Each row stamped with the commit | A row's commit is the `x-brutex-build` of the answers it timed; a row answered by two builds, or by a build other than the run's, fails the run; an unstamped server fails it unless `--allow-unstamped`. | `the sheet fails a server that is down, unstamped, mixed, refusing or failing…` |
| (2) The run fails when a request is answered with a field ignored | Every request with a query is asked once more per field with that field garbled; `ignoredFields` fails the run on the server's own evidence: an `ignored` list in the answer (`/logs.json`'s shape), a 400 saying the route does not read the field (`/imports.json`'s shape), or a garbled field answered 2xx with the same bytes. | **`a_request_with_an_ignored_field_fails_the_run`**, `a garbled field is the field only…` |
| (3) A browser harness for every page against the four targets | New `web/ci/browser-harness.mjs`, no npm dependency (the DevTools protocol over Node's own WebSocket): for `MEASURED_PAGES` (`/db`, `/ingest`, `/gaps`, `/audit`, `/markets`, `/sources`, `/speed`), n cold loads (a fresh browser context each) and n warm, first content (`first-contentful-paint`), the longest pause (`longtask`), heap (`Runtime.getHeapUsage`) and wire bytes, p50/p99/max with n, against first content p99 < 300 ms, no pause > 50 ms, heap < 100 MB, < 400 KB on the wire. A load the server did not answer with the page (a 503, a 404) fails rather than being timed. | `a page is judged against the four targets…`, `the harness in a real browser…` (real Chromium against a stand-in server) |
| (3) Times a picker click, a feed switch, a page turn of option cells and the 20,000-row sort edge | Four timed interactions (click to settled), each naming its selector; a selector the page lacks is NOT MEASURED, said, never a pass. | `the harness in a real browser…` |
| (3) Hook for R4-04's hover coverage | `--hook <module.mjs>`: the module's default export is called with the harness and returns rows and failures. | `the harness in a real browser…` |
| (4) A size budget per page that fails the build | `vite.config.js` `pageBudget()` reads the client build's own output after it is written and fails the build on any page past `BUNDLE_BUDGETS` (`$lib/speed.js`): the 400 KB wire target by default; `/backtest`, `/db` and `/ingest` already ship more and are held just above today's size, OVER THE TARGET and said so on every line. The same rule (`builtPages`) runs over the committed `web/build` in the test. | **`the_bundle_is_inside_its_budget`**, `each page has a budget…`, `the budget refuses a build it cannot read…` |
| (4) An idle tab asks only for a header-only "server up" check | `upCheck` in `web/src/lib/api-status.js`: `HEAD /inspection.json` (a constant answer from the commit stamp alone) in place of the header button's `GET /feeds.json` every 8 s, which read the census, a settings stat and a held-span walk per probe per tab. The harness measures idle tabs (1, 10, 20) and fails on any other request. | `the up check asks for headers only…`, `the button's title names what was asked…`, `an idle tab may ask only for the header-only up check…` |
| (4) One shared reporter sends a failed request, an uncaught error and a fall-back to the report route | New `web/src/lib/page-report.js`, installed on every page by one import line in `+layout.svelte`: wraps `window.fetch` (so `ask` and every other call site report with no change), listens for `error`, `unhandledrejection` and `brutex:fallback` (`reportFallback(what)`). The server's own outcome classes decide (a 5xx that is not busy, or no answer, is a failure; a refusal, a busy and the page's own cancel are not). One report per distinct failure per minute and ten a minute a tab; the count held back rides on the next; the report route never reports itself; a report the server did not keep is one console line a minute. | **`a_failed_fetch_is_found_in_the_logs`**, `a report body fits…`, `an outcome is the server's word…`, `one report per distinct failure…`, `installed, fetch reports a failure…` |

### Files

Owned by R3-15: `web/vite.config.js`, `web/src/lib/api-status.js`, `web/src/routes/speed/+page.svelte`
(new), `web/src/lib/speed.js` (new), `web/src/lib/page-report.js` (new), `web/ci/browser-harness.mjs`
(new), `web/ci/speed-sheet.mjs` (new), `web/tests/speed.test.js` (new).

**Outside `files_owned`, and why:** `web/src/routes/+layout.svelte`. IA-12b's spec records that
"R3-01, R3-15, R3-31 and R3-33 each add one line to `+layout.svelte`". R3-15's touch is more than
one line, because the goal's idle-tab rule needs the header's probe itself changed and the probe
lives in the layout:

1. the `$lib/api-status.js` import gains `upCheck, UP_CHECKING` (one changed line);
2. `import '$lib/page-report.js';` (one added line: the shared reporter for every page);
3. `{ href: '/speed', label: 'Speed', why: … }` in `NAV` (one added line);
4. the probe's state starts as `{ ...UP_CHECKING }` (two changed lines) and its body is one call
   to `upCheck` (the 30-line `GET /feeds.json` body removed; the abort, ticket and content-type
   rules move unchanged into `upCheck`);
5. the status-strip comment says `/inspection.json`, headers only, where it said `/feeds.json`.

Every line the existing layout tests pin is unchanged (`watchVisible(probe, PROBE_MS`,
`firstCheck(probe, options)`, `signal: ticket.signal`, `refreshProbe = watching.refresh`,
`title={apiTitle(api, stampLabel)}`), and `web/tests/api-status.test.js` passes unedited: a state
that names no route keeps the old wording, so its pinned titles still hold. R3-31 and R3-33 come
after R3-15, so they rebase on this.

### Defects found in this part's own code while building it (each fixed and tested)

1. `measure` destroyed a client's keep-alive agent as soon as that client finished, hanging up on a
   request another client still had on it; the sheet counted its own hang-up as the server's "no
   answer" (reproduced: 6 requests over 2 clients, 1 "socket hang up"). Agents are now released
   after every client is done.
2. The harness timed a `503 front end not built` page as a fast page (found against the real
   binary started with a wrong web directory). A load whose document is not 2xx now fails.
3. `settle` waited for requests whose end the page session never sees — the reporter's keepalive
   POST and the census worker's script — so every `/db` load hit the 30 s cap. It now waits only on
   requests still owed an answer, and a worker that started counts as its script answered.
4. The reporter check looked only at finished requests, so the real server's keepalive report
   would have read "sent no report".
5. The uninstall put back a bound copy of `fetch`, not the page's own function.

## Measured

To be filled in from the runs in progress (bundle per page, the browser harness at n = 200 against
the real `api` binary over an empty synthetic store, the speed sheet against it).

## Gates

To be filled in.

## NEEDS THE MAC

To be filled in.

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

Every command runs from the merged worktree's root. `<shadow>` is a fresh shadow server made with
the Mac's own `state/fix/shadow.sh` / `serve.sh` (never one another agent left); `<state>` is the
Mac's state directory. Nothing below writes the real store.

1. **Rebuild and commit the bundle (gate W1)**, from the merged tree:

   ```
   npm --prefix web ci
   npm --prefix web run build
   git add web/build && git commit -m "R3-15: web build from the merged tree"
   ```

   The build now prints one `page budget` line per page and FAILS if a page is past its budget.
   Expected from this branch's build: `/speed` 237.2 KB; every page about 4.2 KB larger than on the
   base (the reporter and the up check live in the shared shell); `/db` 731.1 KB against its
   750 KB budget.

2. **Web tests on the Mac** (`npm --prefix web test`). Two of the new tests drive a real browser:
   they find Chrome at `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome` (or
   `BRUTEX_CHROME`) and fail by name if there is none. CI gate W2 runs on GitHub's ubuntu-24.04
   image, which ships Google Chrome at `/usr/bin/google-chrome` (one of `CHROME_PATHS`): NOT
   VERIFIED from the cloud (no CI run here).

3. **The browser harness over the real store** (n = 200 cold and warm, all seven pages, the four
   interactions, the reporter end to end, and the idle tabs for an hour each), against a shadow:

   ```
   node web/ci/browser-harness.mjs --base http://127.0.0.1:<shadow port> --n 200 \
     --idle-tabs 1,10,20 --idle-ms 3600000 \
     --capture <state>/r315/capture.json --out <state>/r315/harness.md
   ```

   About four hours with the three one-hour idle windows (`--no-idle`, then a second run with
   `--pages /db --no-interactions --no-report-check`, splits it). The reporter check sends ONE
   `POST /page-report` (one audit line in the shadow's log), so it is a shadow check; against
   `:8080` itself use `--no-report-check`, and it then only reads. If a selector of the four
   interactions has changed on the real `/db`, its row says NOT MEASURED and names it.

4. **The speed sheet, after every deploy** — the deploy step's last command, against `:8080`,
   reads only (GET and HEAD):

   ```
   node web/ci/speed-sheet.mjs --base http://127.0.0.1:8080 --n 200 --clients 1,10,60 \
     --from-pages <state>/r315/capture.json \
     --out <state>/r315/speed-sheet-$(git rev-parse --short HEAD).md
   ```

   and the writer and restart sections against a shadow only (never restart `:8080`):

   ```
   node web/ci/speed-sheet.mjs --base http://127.0.0.1:<shadow port> --n 200 --clients 1,10,60 \
     --from-pages <state>/r315/capture.json \
     --writer '<an import or a pull into the shadow store>' \
     --restart '<stop.sh then serve.sh for that shadow>' --restarts 20 \
     --out <state>/r315/speed-sheet-shadow.md
   ```

   Exit 1 lists every failure: a field the server ignored, a refused or failed request, a row
   answered by two builds. This is where the real routes' ignored fields are found: on this
   machine's empty store most routes answer 404/400 before they read a field, so the garbled-field
   check had nothing to compare (see Measured).

5. **`:8080` by eye (main session)**: `/speed` lists the routes with the deployed commit in its
   build line; the header's API button title reads "Header-only round trip (HEAD) to
   /inspection.json"; the Speed tab is in the nav.

6. **Docs** (not R3-15 files; text proposed below): `docs/04-invariants.md`,
   `docs/05-decisions.md` (number from the ledger head at landing), `docs/06-limits.md`.

### Proposed `docs/04-invariants.md` section

```
## The Speed page, the speed sheet, the browser harness and the page budget (R3-15; decision owed at landing)

| Id | Invariant | Proof | |
|---|---|---|---|
| R315-01 | **Every request the speed sheet times is the page's own.** Each catalogue line calls the builder the page's library exports and the page file calls; no query string is typed in the catalogue; a page that builds its URL inline is replayed verbatim from the harness's capture, and a capture holding a write is refused | `web/tests/speed.test.js` `the_speed_sheet_uses_the_pages_own_requests` | ✓ |
| R315-02 | **A field the server ignores fails the sheet's run**, on the server's own evidence: an `ignored` list, a 400 saying it does not read the field, or the field garbled and answered 2xx with the same bytes | `a_request_with_an_ignored_field_fails_the_run` | ✓ |
| R315-03 | **Every sheet row is stamped with the one build that answered it**; two builds, another build than the run's, or an unstamped server (without `--allow-unstamped`) fail the run | `the sheet fails a server that is down, unstamped, mixed, refusing or failing — each by name` | ✓ |
| R315-04 | **A page past its budget fails `vite build`**, read from the built files by the same rule the test applies to the committed `web/build`; a build the rule cannot read is an error naming what is missing | `the_bundle_is_inside_its_budget` · `the budget refuses a build it cannot read, naming what was missing` | ✓ |
| R315-05 | **An idle tab asks only `HEAD /inspection.json`**: the header's probe is header-only; the harness fails any other request an idle tab makes | `the up check asks for headers only, from a constant route, and an idle tab asks for nothing else` · `an idle tab may ask only for the header-only up check; anything else is listed` | ✓ (one hour at 1, 10, 20 tabs: the Mac's run) |
| R315-06 | **A failed request, an uncaught error and a fallback on any page reach `POST /page-report` and the server's log**; a refusal, a busy and the page's own cancel do not; the report route never reports itself; one report per distinct failure a minute and ten a minute a tab | `a_failed_fetch_is_found_in_the_logs` · `installed, fetch reports a failure and no ordinary answer, never itself, and uninstalls cleanly` · `one report per distinct failure a minute, at most ten a minute, and what was held back is carried` | ✓ |
| R315-07 | **The harness never times a load the server did not answer with the page** (a 503, a 404 fails the page) | `a page is judged against the four targets, each miss by name and figure` · `the harness in a real browser…` | ✓ |
| R315-08 | **The Speed page refuses a report missing any field by name and draws an empty window as a dash, never 0 µs** | `a speed report is read field by field and a missing figure is refused by name` · `a time is printed in its unit, and a window that holds no request is a dash, never 0 µs` | ✓ |
```

### Proposed `docs/05-decisions.md` entry

```
### D-<next> — R3-15: the page targets, the page budgets, the header-only up check, the shared reporter's ration and the speed sheet's ignored-field rule — 2026-10-17

1. The four page-side targets are first content p99 < 300 ms, no pause > 50 ms, heap < 100 MB and
   < 400 KB on the wire, bytes decimal (the stricter reading), for /db, /ingest, /gaps, /audit,
   /markets, /sources and /speed (`web/src/lib/speed.js` PAGE_TARGETS, MEASURED_PAGES).
2. Each page's first-load script and style is budgeted at build time: 400 KB by default; /backtest
   1,100,000 B, /db 750,000 B and /ingest 500,000 B, held just above their 16 Oct sizes because
   the server sends them uncompressed. They are over the target and say so; raising a budget is a
   decision with its reason.
3. The header's probe is `HEAD /inspection.json`, a constant answer, in place of `GET /feeds.json`.
4. One shared reporter on every page, rationed per tab to one report per distinct failure a minute
   and ten a minute; the server's own outcome classes decide what is a failure.
5. The speed sheet fails a run on a field the server ignored, by three signals of the server's own
   (an `ignored` list, a 400 that does not read it, a garbled value answered 2xx with the same
   bytes); signal 3 is a lower bound for answers that carry their own clock.
6. The browser harness has no npm dependency: it drives Chrome over the DevTools protocol on Node's
   own WebSocket; `--no-sandbox` only when the operator asks.
```

### Proposed `docs/06-limits.md` rows

```
| R3-15 page budget | Counts the first load's script and style only; a lazily imported chunk and the HTML shell are not counted (the harness's wire figure counts everything a load fetched). |
| R3-15 over-target pages | /backtest 1,096,161 B, /db 731,132 B, /ingest 486,772 B raw on their first load; gzip-6 309,389 / 238,652 / 166,862 B; brotli 252,071 / 206,299 / 146,423 B. The server compresses nothing (`crates/api/src/assets.rs`), so the wire carries the raw bytes; meeting 400 KB needs compression there or splitting those pages. |
| R3-15 ignored-field check | Signal 3 (same bytes for a garbled value) cannot see a field ignored by a route whose answer carries its own clock; signals 1 and 2 still can. On an empty store a route refuses before reading fields, so the check needs real data. |
| R3-15 harness bytes | A worker's own script and a keepalive report finish outside the page's session: their bytes count only as far as the page session sees them (`dataReceived`); the census worker script is 5,098 B. |
| R3-15 harness pauses | Chrome reports a long task only past 50 ms, so "no pause over 50 ms" is exact and a shorter pause reads 0. |
```

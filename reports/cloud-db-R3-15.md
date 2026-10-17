STATUS: DONE

Every gate R3-15 can move is green on the branch. Three are red here exactly as on the base and
are not this part's, said plainly: `cargo deny` (the base's five advisories, DENY-1's);
`cargo test --workspace` (the same 27 failures on base and branch, all of this container's: 25
pass once root's permission override is dropped, the other 2 are an inode-reuse and a
commits-on-main check); `svelte-check` (the base's 7 errors, all in other parts' files). The web
suite is fully green under Node 24, the version CI uses. The page targets are NOT all met: see
"Met and not met" below; the measured figures are written down either way. `web/build` is not
committed here (NEEDS THE MAC 1).

# R3-15 (cloud): Speed page, a browser harness for every page, a speed run at every deploy, page budgets

Branch `cloud/db-R3-15-20261017`, from `origin/feat/db-fix` at `f9cdb584` (Merge R3-14). No Rust
file is touched; every change is under `web/` plus this report.

## Met and not met (the work package's measured target)

| Target | Result (this container, empty store, n = 200 cold and warm) |
|---|---|
| Speed page from `/speed.json` with the build stamp | MET (drawn from the real server; see Measured) |
| Speed run of every route after each deploy, the page's own requests, stamped, failing on an ignored field | MET as code and tests; run here against the real binary with 20 real restarts; `warm+writer` and the real-data run are NEEDS THE MAC 4 (no import or pull can run here) |
| Browser harness, four targets, n = 200 cold and warm, p50/p99/max written | MET as a tool; figures below |
| First content p99 < 300 ms | HOLDS on all seven pages, cold and warm (worst `/markets` cold 228 ms) |
| Heap < 100 MB | HOLDS on all seven pages (worst 56 MB) |
| < 400 KB on the wire | NOT MET cold on `/db` (815 KB), `/markets` (577 KB), `/ingest` (560 KB); holds on the other four and on every warm load. Needs compression in `crates/api/src/assets.rs` (not an R3-15 file) or splitting those pages |
| No pause > 50 ms | NOT MET on any cold load (57–235 ms max: the shared shell's script evaluated in one task); holds on every warm load but one 51 ms pause on `/db`. The Speed page: cold p99 62 ms, max 66 ms; warm 0 |
| Interactions timed (picker, feed switch, option-cell page turn, 20,000-row sort) | NOT MEASURED here: on an empty store `/db` draws none of the four controls; each row names its selector; NEEDS THE MAC 3 |
| Size budget per page that fails the build | MET (`vite.config.js`; every page inside its budget; three pages over the 400 KB target and said so) |
| Idle tab asks only the header-only up check, 1/10/20 tabs for an hour | The header's own probe: MET (`HEAD /inspection.json`, no body). The rule over whole pages: NOT MET — `/db`, `/ingest` and `/audit` keep their own polls on an idle tab (named below; not R3-15 files). Measured 60 s here; the hour is NEEDS THE MAC 3 |
| One shared reporter (failed request, uncaught error, fallback) | MET, proved end to end against the real server |

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
6. Idle tabs opened in one window were background tabs, so hidden, so silent: "10 idle tabs, 7
   requests a minute" measured one tab. Each idle tab now has a browser context (a window) of its
   own, and the table says how many were visible.
7. The sheet's "first after restart" rows printed "0 answered" for twenty answers (their classes
   were not counted).
8. A capture the sheet could not read threw out of the run unnamed; it is now a failed run that
   says so, and nothing is timed from it.
9. Empty option values (`--pages ''`, `--idle-tabs ''`, `--clients ''`) are refused, never quietly
   replaced by the defaults (found by the mutation pass).

## Measured

All on this cloud container (4 cores, a VM much slower than the operator's Mac), with the real
`api` binary (`target/debug/api`, the workspace's `dev` profile, which is optimised, stamped
`30283a95…`) serving this branch's own `vite build` over an EMPTY synthetic store (no market data
exists here). A page over an empty store draws its empty state, so these are not the operator's
figures: the bytes are exact for this build, the times are this machine's. The real-data run is
NEEDS THE MAC 3.

### What each page ships on its first load (the build's own budget lines)

| page | script + style, raw | budget | gzip-6 | brotli |
|---|---:|---:|---:|---:|
| `/` | 221,299 B | 400 KB | 80,537 | 72,223 |
| `/audit` | 314,735 B | 400 KB | 110,304 | 98,437 |
| `/autopilot` | 288,874 B | 400 KB | 99,998 | 89,194 |
| `/backtest` | 1,096,161 B | 1,100 KB, OVER TARGET | 309,389 | 252,071 |
| `/db` | 731,132 B | 750 KB, OVER TARGET | 238,652 | 206,299 |
| `/gaps` | 257,051 B | 400 KB | 93,366 | 83,485 |
| `/ingest` | 486,772 B | 500 KB, OVER TARGET | 166,862 | 146,423 |
| `/mapping` | 252,383 B | 400 KB | 89,756 | 80,179 |
| `/markets` | 325,334 B | 400 KB | 109,219 | 97,304 |
| `/selection` | 231,921 B | 400 KB | 84,682 | 75,777 |
| `/sources` | 309,783 B | 400 KB | 109,592 | 97,879 |
| `/speed` | 237,246 B | 400 KB | 86,526 | 77,495 |
| `/terminal` | 325,644 B | 400 KB | 115,933 | 103,578 |

Every page is about 4.2 KB larger than on the base: the shared reporter and the up check are in
the shared shell. The server sends every byte raw (`crates/api/src/assets.rs` compresses nothing),
so the raw column is what crosses the wire.

### The browser harness, n = 200 cold and 200 warm, quiet machine (load ≈ 2)

`node web/ci/browser-harness.mjs --base http://127.0.0.1:18317 --n 200 --idle-tabs 1,10,20 --idle-ms 60000 --no-sandbox`
(Chromium 1194, headless). p50 / p99 / max:

| page | phase | first content ms | longest pause ms | heap MB | wire KB | targets |
|---|---|---|---|---|---|---|
| /db | cold | 156 / 204 / 268 | 52 / 80 / 235 | 3.1 / 3.1 / 3.1 | 815.2 / 815.2 / 815.2 | FAILS: pause, wire |
| /db | warm | 64 / 84 / 84 | 0 / 0 / 51 | 22.1 / 44.8 / 49.3 | 70.5 / 70.5 / 70.5 | FAILS: one pause of 51 ms |
| /ingest | cold | 156 / 192 / 212 | 53 / 74 / 114 | 2.8 / 2.9 / 2.9 | 560.0 / 560.0 / 560.0 | FAILS: pause, wire |
| /ingest | warm | 76 / 100 / 100 | 0 / 0 / 0 | 24.7 / 52.7 / 54.5 | 54.6 | holds |
| /gaps | cold | 140 / 172 / 180 | 0 / 59 / 65 | 2.4 | 314.2 | FAILS: pause |
| /gaps | warm | 56 / 80 / 84 | 0 / 0 / 0 | 20.2 / 54.8 / 56.0 | 45.6 | holds |
| /audit | cold | 140 / 176 / 180 | 0 / 57 / 60 | 2.7 / 2.8 / 2.8 | 372.5 | FAILS: pause |
| /audit | warm | 56 / 80 / 84 | 0 / 0 / 0 | 19.8 / 35.3 / 35.3 | 46.4 | holds |
| /markets | cold | 168 / 228 / 296 | 63 / 90 / 125 | 3.1 | 576.6 | FAILS: pause, wire |
| /markets | warm | 68 / 100 / 116 | 0 / 0 / 0 | 20.2 / 51.4 / 53.9 | 45.6 | holds |
| /sources | cold | 148 / 188 / 252 | 0 / 62 / 84 | 2.2 | 368.8 | FAILS: pause |
| /sources | warm | 56 / 72 / 76 | 0 / 0 / 0 | 21.2 / 52.4 / 53.6 | 47.3 | holds |
| /speed | cold | 136 / 172 / 184 | 0 / 62 / 66 | 2.3 | 300.9 | FAILS: pause |
| /speed | warm | 56 / 72 / 84 | 0 / 0 / 0 | 15.5 / 39.9 / 40.3 | 52.9 | holds |

What this says, plainly:

* **First content p99 under 300 ms holds on every page**, cold and warm (worst: `/markets` cold
  228 ms). **Heap under 100 MB holds on every page** (worst 56 MB, warm, after 200 reloads in one
  tab).
* **Wire under 400 KB fails on three pages cold**: `/db` 815 KB, `/markets` 577 KB, `/ingest`
  560 KB. `/markets` ships 325 KB of code, so about 250 KB of its cold load is data it reads.
  Compression in `assets.rs` would bring `/db`'s code to 239 KB (gzip); that is not an R3-15 file.
* **No pause over 50 ms fails on every cold load**, by 7 to 185 ms at the max: the first load
  evaluates the shared shell's script (about 190 KB on every page) in one task. The Speed page's
  own code is small; its 62 ms p99 is the shell's. Warm loads hold on every page except one 51 ms
  pause on `/db`. On the Mac's faster cores these figures will be lower; whether they pass is the
  Mac's run.
* **The four interactions are NOT MEASURED here**: on an empty store `/db` draws none of the four
  controls (the search, the feed picker and the grid all wait for data). The selectors are named
  in each row, and the Mac's real-data run measures them.
* **Idle tabs** (60 s here; an hour is the Mac's run): every idle tab in its own window, all
  visible. The header now asks only `HEAD /inspection.json`, but other pages' own polls also run
  on an idle tab and the harness fails on them, by name: `HEAD /store.json?feed=zerodha` (`/db`'s
  census generation watch), `GET /ingest/status.json` and `GET /pull/run.json` (`/ingest`),
  `GET /audit.json?feed=zerodha` (`/audit`). 1 tab: 19 requests a minute; 10 tabs: 119; 20 tabs:
  238. None of those polls is an R3-15 file; each owner's page has to move to the header-only
  check, or the operator has to allow it, before goal 4's idle rule holds.
* **The shared reporter end to end, against the real server**: a fetch failed at the network on a
  page was posted to `/page-report`, answered `202`, and found in `/logs.json?target=api.page.access`.
  It also reported, unprompted, the real empty-store fault `/db` hit on every load:
  `{"target":"api.page.access","msg":"page reported a failure","fields":{"page":"/db","what":"request: GET /instruments.json?feed=zerodha answered HTTP 503"}}`.

An earlier run of the same command on a loaded machine (load ≈ 10, two other jobs) gave first
content p99 up to 496 ms and pauses up to 316 ms; it is not the measurement and is not used.

### The speed sheet against the same server, with 20 real restarts

`node web/ci/speed-sheet.mjs --base http://127.0.0.1:18317 --n 200 --clients 1,10,60 --feed zerodha --timeframe 1min --month 2026-09 --day 2026-09-30 --from-pages <the harness's capture> --restart <a scratch script that stops and starts this session's own api> --restarts 20`.
Every row is stamped `12c02093d79e` (the binary the branch's test run rebuilt). 24 requests: the
14 of the catalogue and 10 captured from the pages by the harness. ms from sent to last byte,
p50 / p99 / max, n = 200 (n = 20 after restart):

| request | cold (new connection) | warm, 1 client | warm, 10 | warm, 60 | first after restart | body bytes |
|---|---|---|---|---|---|---:|
| `HEAD /inspection.json` (the header's up check now) | 0.94 / 3.12 / 3.41 | 0.25 / 0.92 / 1.41 | 1.40 / 4.53 / 4.74 | 7.60 / 28.3 / 28.4 | 1.09 / 1.88 / 1.88 | 0 |
| `GET /feeds.json` (what it asked before) | 1.58 / 3.18 / 3.94 | 1.13 / 2.56 / 12.7 | 3.84 / 9.74 / 10.4 | 15.7 / 50.5 / 53.5 | 1.88 / 4.34 / 4.34 | 34,668 |
| `GET /speed.json` (the Speed page's read) | 0.92 / 1.81 / 4.01 | 0.31 / 1.18 / 1.67 | 1.71 / 4.92 / 5.28 | 12.1 / 36.0 / 36.9 | 0.97 / 1.92 / 1.92 | 11,416–11,450 |

So the header's probe went from 34,668 bytes and a census read per probe per tab to no body at
all, and is about 2× to 4.5× faster at p50 on this machine (on the real store `/feeds.json`'s
body and its census walk are larger).

The run FAILED, as it must on an empty store: `/bars/window.json` (both reads) refused `400`,
`/provenance.json` `404`, `/instruments.json` `503` (the universe is degraded with no masters),
each named per phase and per restart. No `--writer` was given (no import or pull can run here),
so `warm+writer` is NOT MEASURED. **No field was ignored**: eleven query routes answered 2xx
(`/census/facets.json`, `/dayflags.json`, `/calendar.json` twice, `/scrub.json`, `/sources.json`,
`/sources/day.json`, `/sources/bars.json`, `/store.json` three ways, `/audit.json`), and each
garbled field of each was refused or answered differently. The routes that refused could not be
judged; the real-data run is NEEDS THE MAC 4.

A defect this run found in the sheet itself, fixed and tested: the restart rows printed "0
answered" for twenty answers, because their classes were not counted.

## Gates

Run in this cloud container (4 cores, runs as root, rustc 1.97.1; node 22.22.0 is installed, and the official Node 24.22.0 binary was fetched to run the web gates as CI does). Base = `f9cdb584`.

| Gate | Base | Branch |
|---|---|---|
| `cargo fmt --all --check` | EXIT 0 | EXIT 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | EXIT 0 | EXIT 0 |
| `cargo test --workspace --locked --no-fail-fast -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store` (the known slow cli test skipped, per CONTEXT) | 9,066 passed, 27 failed | 9,066 passed, the **same** 27 failed |
| `cargo deny check` | EXIT 1: RUSTSEC-2026-0344, 0346, 0347, 0348, 0349 | EXIT 1: exactly the same five (no `Cargo.toml`, `Cargo.lock` or crate file changed) |
| `node --test web/tests/*.test.js` under **Node 24.22.0** (gate W2's version; from the repo root, as CI runs it) | 1,405 pass, 0 fail, 0 cancelled | **1,445 pass, 0 fail, 0 cancelled** (the 40 new included) |
| `npm --prefix web test` under Node 22.22.0 (this container's) | 1,400 pass, 0 fail, 5 cancelled (`ask.test.js` ×3, `census-worker.test.js` ×2) | run while load was ≈ 7: 1,435 pass, 1 fail, 9 cancelled — the base's 5, plus `db-search.test.js:505` (a 16 ms p99 test read 22 ms) and `store-watch.test.js` ×4; both files pass alone (33/33), and all of it passes under Node 24 above |
| `svelte-check` (gate W3, ceiling 0) | 7 errors | the same 7 errors, none in an R3-15 file |
| `vite build` (gates W1, W4) | builds | builds, every page inside its budget, 0 unused CSS selectors (`gates_jobs w4`: ceiling 0 held); `web/build` NOT committed here (NEEDS THE MAC 1) |
| gate W5 (`web/ci/css-comments.mjs` on the new page and the layout), W6 | pass | pass |
| gate 1 and 1b (`.github/source_scan.rs`, `.github/gates_tree.rs`), `.github/gates_routes.rs` | pass | pass |

The 27 Rust failures are the same on base and branch, in crates R3-15 does not touch, and they
are this container's, not the code's. **Proved, not inferred:** the same test binaries run with
root's two permission-override capabilities dropped
(`setpriv --bounding-set -dac_override,-dac_read_search --inh-caps -dac_override,-dac_read_search <binary> --exact <the 27 names>`,
from each crate's directory) pass **25 of the 27** (api 12, cli 3, pull 3, store 3, telemetry 4).
The two left: `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`
asserts a new inode number and this container's filesystem handed the deleted file's inode
straight back (`left: 2220892, right: 2220892`); `every_commit_the_store_records_cite_is_on_main_or_says_why_not`
finds commits cited in `docs/06-limits.md` that are on `feat/db-fix` and not yet ancestors of
`origin/main` in this clone. Neither reads an R3-15 file. The full list:
`every_commit_the_store_records_cite_is_on_main_or_says_why_not`,
`gdfl_import::tests::a_day_ledger_that_will_not_take_a_line_leaves_the_day_open`,
`gdfl_import::tests::a_journal_that_cannot_be_written_fails_the_day_by_name`,
`gdfl_tickstore::tests::a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses`,
`logs::tests::a_directory_that_turns_unreadable_is_never_answered_as_read`,
`logs::tests::a_ring_file_that_stays_unreadable_is_named_once`,
`logs::tests::logs_names_a_directory_it_may_not_search`,
`pullrun::restart_tests::the_journals_own_extremes`,
`pullrun::tests::an_absent_index_is_built_by_the_scan_and_not_known_is_said_when_it_cannot_be`,
`pullrun::tests::an_index_that_cannot_be_read_never_leaves_the_answer_clean`,
`scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`,
`server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`,
`server::tests::a_failure_of_the_store_keeps_the_leg_owed_and_is_never_called_standing`,
`server::tests::a_probe_name_that_cannot_be_unlinked_is_a_fault_by_name`,
`server::tests::a_run_cut_short_by_a_restart_is_logged_when_it_is_found`,
`server::tests::a_store_that_cannot_take_a_write_is_found_by_asking_it`,
`server::tests::an_expired_fno_walk_cut_short_is_seen_at_the_next_start`,
`settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
`settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
`settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`,
`sink::tests::a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`,
`sink::tests::a_failed_delete_is_counted_named_and_left_for_the_next_start`,
`sink::tests::a_rename_aside_that_cannot_happen_fails_the_roll_by_name`,
`sink::tests::an_audit_stream_that_cannot_be_looked_for_is_an_error`,
`staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`,
`watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many`,
`watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`.

Coverage (Node 24.22.0, `node --test --experimental-test-coverage` over `tests/speed.test.js` and
`tests/api-status.test.js`, the browser-driving and command tests included, child processes
merged; 45 pass): 100% lines, 100% branches and 100% functions on all six
touched JavaScript modules — `src/lib/speed.js`, `src/lib/page-report.js`,
`src/lib/api-status.js`, `ci/speed-sheet.mjs`, `ci/browser-harness.mjs`, `vite.config.js`.
`+page.svelte` files are not measured by node's coverage (no tool here instruments a Svelte
component under `node --test`); the Speed page's logic is in `$lib/speed.js`, which is.

Mutation: no JavaScript mutation tool is in `web/package.json` and adding one is a dependency
change outside this part, so a small mutation runner (scratch, not tracked) applied 17 operators
(`===`/`!==`, `<`/`<=`/`>`/`>=`, `&&`/`||`, `true`/`false`, `±1`, `some`/`every`, `!` removal,
`??`→`||`, statement deletion) line by line and ran the node tests (the four browser and command
tests left out, for time). First pass, before the tests it led to:

| Module | Mutants | Killed | Survived | Of the survivors |
|---|---:|---:|---:|---|
| `src/lib/speed.js` | 122 | 94 | 28 | 11 equivalent (6 inside JSDoc casts, 5 `??`→`||` on values never falsy); 17 real: tests added (exact 1 ms / 10 ms / 1 s edges, n = 0 never slow, tie order, `some`/`every`, NaN sizes and heaps, status 299 / 300, exactly-at-budget), and one redundant assignment removed |
| `src/lib/page-report.js` | 106 | 80 | 26 | 9 equivalent (`??`→`||` on functions and never-empty strings, space→space); 17 real: tests added (DEL and no-break space, a newline in a page, exactly 256 bytes, exactly 1,024 bytes, the longest prefix that fits, HTTP 400, an entry exactly a minute old, a ration of 0, empty error messages, line 0, an empty fallback detail, a window without `dispatchEvent`) |
| `src/lib/api-status.js` | 14 | 9 | 5 | 4 equivalent; 1 real: test added (an empty error message) |
| `ci/speed-sheet.mjs` | 180 | 136 | 44 | about 30 read at first as equivalent (`??`→`||`), four of which were not and have tests now (an empty page name in a capture, empty `--clients` and `--instrument`, an unreadable capture); the rest real: tests added (the extremes URL exactly, a duplicated capture entry, 512 and `1,0` clients, Retry-After on a 400, the largest answer's bytes, no sentence on a clean row, an offered feed chosen, a month or a day discovered alone, exactly 10 ms, the blank line), and the warm agent made one named factory so a test can count its connections |
| `ci/browser-harness.mjs` (its rules: `parseArgs`, `chromeOf`, `exceptionText`, `judge`, `idleVerdict`, `captured`, `renderHarness`) | 79 | 66 | 13 | 4 read at first as equivalent, two of which were not and have tests now (empty `--pages` and `--idle-tabs`); 9 real: tests added (a mixed page list, 64 / `1,0` / 1.5 idle tabs, `--no-sandbox` read, HTTP 300, a pause of exactly 50 ms) |

The browser-driving functions of the harness (`launch`, `openTab`, `loadOnce`, the report check,
`runHarness`) are covered 100% by the real-browser tests but were not mutated (each run needs
Chromium, about 40 s).

Second pass, on the code as committed (tests in-process, `--experimental-test-isolation=none`,
so a mutant that loops forever is killed by the timeout with its whole run), plus the six lines
re-run after the last tests:

| Module | Mutants | Killed | Survived, every one equivalent |
|---|---:|---:|---|
| `src/lib/speed.js` | 120 | 109 | 11: six inside JSDoc type casts (comments); five `??`→`||` where the left side is never a falsy value that is not null (an object, a function, a budget, a script's text) |
| `src/lib/page-report.js` | 106 | 97 | 9: `<= 0x20` (a space becomes a space); seven `??`→`||` on functions and strings never empty; the prune `>=`→`>` at exactly one minute (pruning then only frees memory: the next check allows the entry either way) |
| `src/lib/api-status.js` | 14 | 10 | 4: `??`→`||` on two clocks, a null header and a route never empty |
| `ci/speed-sheet.mjs` | 186 | 157 | 29: 28 `??`→`||` where the left side is a function, an object, a count, a non-empty string or `undefined`; one wall-clock deadline `>`→`>=` |
| `ci/browser-harness.mjs` (its rules) | 79 | 76 | 3: `--base ''` (refused either way); `fcp.n >= 0` (an empty window's p99 is 0, never ≥ 300); `??`→`||` on a count |

No mutant that changes behaviour survives.

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

7. **Findings for other owners** (measured here; none is an R3-15 file, so nothing was changed):
   - Idle-tab polls that the harness fails on, by page: `/db` `HEAD /store.json?feed=<wire>`
     (the census generation watch, `$lib/store.svelte.js` / `store-watch`); `/ingest`
     `GET /ingest/status.json` and `GET /pull/run.json`; `/audit` `GET /audit.json?feed=<wire>`.
     Goal 4's idle rule holds for the whole app only when each moves to the header-only check or
     the operator allows it by name.
   - Compression: the server sends the bundle raw. gzip-6 would take `/db`'s first-load code from
     731,132 B to 238,652 B, `/ingest`'s from 486,772 B to 166,862 B, `/backtest`'s from
     1,096,161 B to 309,389 B — inside 400 KB. `crates/api/src/assets.rs`.
   - `/markets` reads about 250 KB of data on a cold load over an EMPTY store (577 KB on the wire
     against 325 KB of code).
   - `/db` on an empty store reports `GET /instruments.json?feed=zerodha answered HTTP 503` on
     every load (the new reporter put it in the log unprompted).

8. **Ordering**: R3-31 and R3-33 depend on R3-15 and add their own `+layout.svelte` lines; R3-33
   draws `speedTile()` from `$lib/speed.js` as the health page's "speed" tile; R4-04's hover
   coverage plugs into the harness with `--hook web/ci/hover-coverage.mjs`.

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

## Left on this machine (nothing of it is tracked)

Everything below is in this session's scratchpad, `/tmp/claude-0/-home-user-brutex/f7d1ed70-61fe-5585-8a99-7d847d66b4f9/scratchpad/`:
the base worktree `base-wt` (registered with `git worktree list`, detached at `f9cdb584`), the
web copies `webcopy1`–`webcopy3` and `mut1`–`mut3`, the three empty synthetic stores and their
logs under `live`, `live2`, `live3` (the harness sheets, the speed sheet and the capture are in
`live3`), the mutation runner `mutate.mjs` and its results `mut-*.json`, `mut2-*.json`, and a
Node 24.22.0 binary under `node24`. The scratch `api` servers and every browser this part started
are stopped. Nothing was deleted except one marker file of this session's own mutation runner in
the scratchpad. No real store, no `:8080` and no credential was touched; no file holds a
credential, a token, a parameter path or a link.

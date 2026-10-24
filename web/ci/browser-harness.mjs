#!/usr/bin/env node
/**
 * THE BROWSER HARNESS — every page measured in a real browser against the
 * four page-side targets, the timed interactions, the idle tabs, and the
 * page's own failure report found in the server's log (R3-15; NEW-01, NEW-08).
 *
 * # What it measures
 *
 * For each page of `$lib/speed.js` `MEASURED_PAGES` (or `--pages`), `--n`
 * cold loads (a fresh browser context each: no cache, no cookies) and `--n`
 * warm loads (the same context, its cache kept), each with:
 *
 * * first content — the `first-contentful-paint` entry, ms from navigation;
 * * the longest pause — the longest task the page's `longtask` observer saw
 *   from navigation to settled (a task over 50 ms is a pause the reader feels);
 * * memory — the page's script heap once settled (`Runtime.getHeapUsage`);
 * * on the wire — every response's encoded bytes for the load
 *   (`Network.loadingFinished`).
 *
 * p50, p99 and the max with n for each, against `PAGE_TARGETS`: first
 * content p99 under 300 ms, no pause over 50 ms, heap under 100 MB, under
 * 400 KB on the wire. A page that misses one FAILS, by name and figure.
 *
 * A PAGE IS THE APP'S, OR IT IS NOT TIMED (verify r4, 23 Oct): the server
 * answers every path with the app's shell and 200, so the first load of
 * each page asks the build that answered for its own routes
 * ({@link READ_ROUTES}); a page none of them routes, or a document that is
 * not the app's, fails by name and is not loaded again. EVERY WAIT ON THE
 * BROWSER IS BOUNDED: a DevTools command by {@link CDP_WAIT_MS}, a load by
 * `--load-wait-ms`, two frames by {@link FRAME_WAIT_MS}; a load that does
 * not end within its bound fails its page by name, and a browser that stops
 * answering ends the run with the sheet still written.
 *
 * Then four timed interactions: a picker click, a feed switch, a page turn
 * of option cells and a sort at the 20,000-row browser sort edge
 * (`LOCAL_SORT_BUDGET`), each in a browser context of its own. Each is
 * timed from the click to the last network event it caused, or to two
 * frames drawn after it when that is later; the page must then stay quiet
 * for the settle window, which is NOT counted (found on the Mac, 20 Oct: the
 * first figures each carried the whole 500 ms quiet window). Each names the
 * selector it needs; a selector the page does not have within
 * `--step-wait-ms` (30 s) of the step before it is NOT MEASURED, said, never
 * a pass. The page is settled before every click, so a click's time is its
 * own and never the tail of the step before it; a click whose page has not
 * settled 30 s later fails the run by name rather than being timed.
 *
 * The sort edge is FOUND, never typed ({@link sortEdge}): the opening
 * instrument's rungs from `/sources.json` (each with the `/db` address the
 * server builds for it), and for the first one on which one held day fits
 * the budget, the longest range of days ending at its newest day whose bars
 * `/bars/window.json` counts at or under `LOCAL_SORT_BUDGET` — asked with
 * the page's own `windowQuery`, by halving. Its column is the Change %
 * header, which the browser orders itself.
 *
 * Then idle tabs (`--idle-tabs 1,10,20`, each for `--idle-ms`, an hour by
 * default, at most `MAX_WAIT_MS`, the longest wait a Node timer holds):
 * every request an idle tab makes is listed; any but the header-only up
 * check (`HEAD /inspection.json`) fails the run. The row prints the time the
 * clock read, and a wait that ended early is NOT MEASURED and fails the run.
 *
 * Then the shared reporter end to end: one request of a page is made to fail
 * at the network (`Fetch.failRequest`), and the report must be found in the
 * server's own log (`/logs.json?target=api.page.access`).
 *
 * `--read-only` (for a live server, `:8080` included): no write leaves the
 * browser. The shared reporter's `POST /page-report`, which a page sends on
 * its own whenever one of its requests fails, is blocked in the browser and
 * printed as a note with what it would have said; any other write that left
 * the browser fails the run by name; the reporter check above is not run (it
 * needs its one POST). `--no-report-check` alone does NOT make a run read
 * only: a page that meets a failure still reports it.
 *
 * `--hook <module.mjs>` adds a check of its own (R4-04's hover coverage): its
 * default export is called with {@link Harness} and returns
 * `{rows?: string[], failures?: string[]}`.
 *
 * `--capture <file>` writes every same-origin read each page made on its
 * first load, for `speed-sheet.mjs --from-pages`: the page's own requests,
 * verbatim, whoever built them.
 *
 * # Usage
 *
 *   node web/ci/browser-harness.mjs --base http://127.0.0.1:8080 \
 *     [--chrome <path to Chrome or Chromium>] [--n 200] [--pages /db,/ingest] \
 *     [--idle-tabs 1,10,20] [--idle-ms 3600000] [--no-interactions] [--no-idle] [--no-sandbox] \
 *     [--no-report-check] [--read-only] [--hook <module.mjs>]... [--capture <file>] [--out <sheet.md>] \
 *     [--settle-ms 500] [--step-wait-ms 30000] [--load-wait-ms 30000] [--instrument <key>]
 *
 * No npm dependency: it drives the browser over the DevTools protocol on
 * Node's own WebSocket. Exit 0 when every target holds; 1 with every reason
 * otherwise (the sheet is written, and names the build that answered); 2
 * for a misused command, an `--out` or `--capture` that cannot be written,
 * or no browser.
 */
import { spawn } from 'node:child_process';
import { accessSync, constants as fsConstants, existsSync, mkdtempSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve as resolvePath } from 'node:path';
import { pathToFileURL } from 'node:url';

import { PAGE_TARGETS, MEASURED_PAGES, summary, routePath } from '../src/lib/speed.js';
import { UP_ROUTE, UP_METHOD } from '../src/lib/api-status.js';
import { REPORT_ROUTE } from '../src/lib/page-report.js';
import { BUILD_HEADER } from '../src/lib/runtime-inspection.js';
import { LOCAL_SORT_BUDGET, windowQuery } from '../src/lib/db/bars-plan.js';
import { rungsUrl, rungSeconds, SOURCES_DEFAULTS } from '../src/lib/sources.js';
import { linkOfKey } from '../src/lib/db/selection-model.js';
import { writeWhole, MAX_WAIT_MS, DECIMAL, GROUP_POLL_MS, groupRunning, signalGroup } from './speed-sheet.mjs';

// The longest wait a Node timer holds, and the sheet's own reading of a whole
// number: one rule for both commands (`speed-sheet.mjs` says why). The
// process-group stop is one rule too: the browser here, a command there.
export { MAX_WAIT_MS, GROUP_POLL_MS, groupRunning, signalGroup };

/** Where a browser is looked for when `--chrome` and `BRUTEX_CHROME` say nothing. */
export const CHROME_PATHS = Object.freeze([
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
  '/opt/pw-browsers/chromium-1194/chrome-linux/chrome',
  '/usr/bin/chromium',
  '/usr/bin/google-chrome'
]);

/**
 * The controls the timed interactions press, as `/db` draws them (checked
 * against the real page on the Mac, 20 Oct: the feed picker is the shared
 * `Picker` whose title states the page's whole scope, and an option grid has
 * pages only once an expiry is chosen).
 */
/** /db's feed picker: the `Picker` button whose title says the feed is the page's whole scope. */
export const FEED_PICKER = 'button.pbtn[title*="WHOLE SCOPE"]';
/** /db's expiry picker: the `Picker` in the field captioned "Which contract". */
export const EXPIRY_PICKER = '.field:has(> span.lab[title^="Which contract"]) button.pbtn:not([disabled])';
/** An open picker's first choice that is neither the one held nor refused. */
export const FREE_CHOICE = '.pmenu input[type="radio"]:not(:checked):not(:disabled)';
/** The bar grid's next page, offered. */
export const NEXT_PAGE = 'button[aria-label="Next page"]:not([disabled])';
/** A column the browser orders itself (Change %), offered. */
export const BROWSER_SORT = 'th[aria-sort] button.sortbtn[title*="the browser orders this column itself"]:not([disabled])';

/**
 * The timed interactions for an instrument: the picker click types its
 * underlying (`linkOfKey(key).sym`) into `/db`'s instrument search.
 * `open: null` with `edge` is opened at the
 * address {@link sortEdge} finds. Each step is `{click}`, `{type: [selector, text]}`
 * or `{waitFor}`; the time is from the LAST step's click to the last network event it caused, or two frames after it if later.
 * `open` is the address the page is opened at first.
 *
 * `last` clicks the last match rather than the first: the expiry picker
 * lists "None chosen" first, so its last free choice is the newest expiry
 * the page offers, whose option cells run to many pages.
 *
 * @typedef {{click: string, last?: boolean} | {type: [string, string]} | {waitFor: string}} Step
 * @typedef {{name: string, open: string | null, edge?: boolean, steps: Step[]}} Interaction
 * @param {string} instrument an instrument key, `NSE-INDEX-NIFTY`
 * @returns {readonly Readonly<Interaction>[]}
 */
export function interactionsFor(instrument) {
  const sym = linkOfKey(instrument)?.sym;
  if (!sym) throw new Error(`--instrument ${JSON.stringify(instrument)} is not an instrument key (NSE-INDEX-NIFTY, NSE-EQ-RELIANCE, ...)`);
  // /db's own opening is its bare address; another instrument is named by
  // `sym=`, the field /db reads for the underlying.
  const own = sym === linkOfKey(SOURCES_DEFAULTS.key)?.sym;
  const db = (/** @type {string} */ q) => (own ? `/db${q === '' ? '' : `?${q}`}` : `/db?${q === '' ? '' : `${q}&`}sym=${encodeURIComponent(sym)}`);
  /** @type {Interaction[]} */
  const timed = [
    { name: 'picker click', open: db(''), steps: [{ type: ['[aria-controls="db-search-list"]', sym] }, { waitFor: '#db-search-list [role="option"]' }, { click: '#db-search-list [role="option"]' }] },
    { name: 'feed switch', open: db(''), steps: [{ click: FEED_PICKER }, { waitFor: FREE_CHOICE }, { click: FREE_CHOICE }] },
    { name: 'page turn of option cells', open: db('seg=options'), steps: [{ click: EXPIRY_PICKER }, { waitFor: FREE_CHOICE }, { click: FREE_CHOICE, last: true }, { waitFor: NEXT_PAGE }, { click: NEXT_PAGE }] },
    { name: `sort at the ${LOCAL_SORT_BUDGET.toLocaleString('en-IN')}-row browser sort edge`, open: null, edge: true, steps: [{ waitFor: BROWSER_SORT }, { click: BROWSER_SORT }] }
  ];
  return Object.freeze(timed.map((t) => Object.freeze(t)));
}

/** How long a step waits for its selector, ms: real data can take seconds to draw a grid. */
export const STEP_WAIT_MS = 30_000;

/**
 * The instrument `/db` opens on, whose rungs the sort edge is found on and
 * whose underlying the picker click types: READ from the app's own default
 * (`$lib/sources.js` `SOURCES_DEFAULTS.key`, "the instrument /db opens on",
 * whose underlying is `$lib/db/selection-model.js` `DEFAULT_INSTRUMENT`; a
 * test holds the two together), never typed here. `--instrument` names
 * another.
 */
export const EDGE_INSTRUMENT = SOURCES_DEFAULTS.key;

/** The timed interactions on {@link EDGE_INSTRUMENT}. */
export const INTERACTIONS = interactionsFor(EDGE_INSTRUMENT);

/** `day` less `n` days, both `YYYY-MM-DD`. @param {string} day @param {number} n */
const daysBefore = (day, n) => new Date(Date.parse(`${day}T00:00:00Z`) - n * 86_400_000).toISOString().slice(0, 10);

/**
 * THE SORT EDGE, FOUND FROM THE DATA: the `/db` address whose grid holds the
 * most bars the browser still orders itself (at most `budget`), on the
 * opening instrument.
 *
 * The instrument's rungs come from `/sources.json` (the page's own
 * `rungsUrl`), each stored rung with the `/db` address the server builds
 * for it; feeds in the server's order, then finest first. A rung on which its
 * newest held day alone is past the budget (NIFTY at one second: 22,500 bars
 * a day) is passed over, said in `passed`. On the first that remains, the
 * days back from its newest day are halved until the longest range at or
 * under the budget is found, each count asked with the page's own
 * `windowQuery` (`limit=1`, so one row a question): at most about twelve
 * GETs over eight years of days.
 *
 * REFUSES rather than guesses: an answer without a whole `total`, a route
 * that answers anything but 200, or no rung on which one day fits.
 *
 * @param {(path: string) => Promise<{status: number, body: string}>} get a GET on the server
 * @param {string} [instrument]
 * @param {number} [budget]
 * @returns {Promise<{feed: string, rung: string, from: string, to: string, rows: number, asked: number,
 *   passed: string[], address: string}>}
 */
export async function sortEdge(get, instrument = EDGE_INSTRUMENT, budget = LOCAL_SORT_BUDGET) {
  let asked = 0;
  /** @param {string} path */
  const json = async (path) => {
    asked += 1;
    const a = await get(path);
    if (a.status !== 200) throw new Error(`finding the sort edge: ${path} answered HTTP ${a.status}`);
    return JSON.parse(a.body);
  };
  const url = rungsUrl(instrument);
  if (url === null) throw new Error(`finding the sort edge: ${instrument} is not an instrument key rungsUrl can ask about`);
  const body = await json(url);
  const feeds = (Array.isArray(body?.feeds) ? body.feeds : []).map((/** @type {any} */ f) => String(f?.feed ?? ''));
  const rank = (/** @type {string} */ feed) => (feeds.indexOf(feed) + feeds.length + 1) % (feeds.length + 1);
  const rungs = (Array.isArray(body?.rungs) ? body.rungs : [])
    .filter((/** @type {any} */ r) => r?.stored === true && typeof r?.db === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(String(r?.first_day)) && /^\d{4}-\d{2}-\d{2}$/.test(String(r?.last_day)))
    .sort((/** @type {any} */ a, /** @type {any} */ b) => rank(a.feed) - rank(b.feed) || rungSeconds(a.rung) - rungSeconds(b.rung));
  /** @param {any} r @param {string} from @param {string} to */
  const total = async (r, from, to) => {
    const q = windowQuery(String(r.feed), { keys: [instrument], timeframe: String(r.rung), from: from.slice(0, 7), to: to.slice(0, 7) }, { sort: 'ts', desc: true, offset: 0, limit: 1, fromDay: from, toDay: to });
    const b = await json(`/bars/window.json?${q}`);
    if (!Number.isSafeInteger(b?.total) || b.total < 0) throw new Error(`finding the sort edge: /bars/window.json?${q} answered no whole total`);
    return /** @type {number} */ (b.total);
  };
  /** @type {string[]} */
  const passed = [];
  for (const r of rungs) {
    const to = String(r.last_day);
    const one = await total(r, to, to);
    if (one > budget) {
      passed.push(`${r.feed} ${r.rung}: ${one} bars on ${to} alone`);
      continue;
    }
    // Days back from `to`: `lo` always fits, `hi` is the most there are.
    let lo = 0;
    let hi = Math.round((Date.parse(`${to}T00:00:00Z`) - Date.parse(`${r.first_day}T00:00:00Z`)) / 86_400_000);
    if ((await total(r, String(r.first_day), to)) <= budget) lo = hi;
    while (lo < hi) {
      const mid = Math.ceil((lo + hi) / 2);
      if ((await total(r, daysBefore(to, mid), to)) <= budget) lo = mid;
      else hi = mid - 1;
    }
    const from = daysBefore(to, lo);
    const rows = await total(r, from, to);
    return { feed: String(r.feed), rung: String(r.rung), from, to, rows, asked, passed, address: `${r.db}&from=${from}&to=${to}` };
  }
  throw new Error(`finding the sort edge: no stored rung of ${instrument} holds a day at or under ${budget} bars${passed.length > 0 ? ` (${passed.join('; ')})` : ''}`);
}

/**
 * The observer every page gets before its own scripts run.
 *
 * READ THROUGH `flush()`, NEVER FROM THE FIELDS ALONE (found on the Mac, 18
 * Oct): a `PerformanceObserver` delivers its entries in a later, low-priority
 * task, so a read taken right after the page settled could find the paint
 * already drawn and its entry not yet delivered. Under load (the Mac at a
 * load average of 62) a `/speed` cold load that had painted read "painted no
 * first content". `flush()` takes every entry still queued (`takeRecords()`)
 * and folds it in before it answers, so the figures are the browser's whole
 * record at the moment of the read.
 *
 * AND A LOAD WAITS FOR ITS PAINT, BOUNDED (found on the Mac, 20 Oct, the
 * whole web suite running beside other agents' builds): `flush()` alone
 * still read "painted no first content" for a page that paints, because
 * Chrome queues the first-contentful-paint entry only when the frame's
 * presentation is reported, which under load came after the two frames
 * `settle` waits for. `painted(capMs)` flushes every 25 ms until a first
 * content is known or `capMs` has passed ({@link AWAIT_PAINT}). The figure
 * is the browser's own timestamp, so the wait changes no measurement; a page
 * that never paints is still "not painted", after at most the cap.
 */
export const OBSERVER = `(() => {
  const h = (window.__harness = { fcp: null, longest: 0, tasks: 0 });
  const paint = (entries) => { for (const e of entries) if (e.name === 'first-contentful-paint') h.fcp = e.startTime; };
  const long = (entries) => { for (const e of entries) { h.tasks += 1; if (e.duration > h.longest) h.longest = e.duration; } };
  const watching = [];
  for (const [type, fold] of [['paint', paint], ['longtask', long]]) {
    try {
      const o = new PerformanceObserver((list) => fold(list.getEntries()));
      o.observe({ type, buffered: true });
      watching.push([o, fold]);
    } catch {}
  }
  h.flush = () => {
    for (const [o, fold] of watching) fold(o.takeRecords());
    return { fcp: h.fcp, longest: h.longest, tasks: h.tasks };
  };
  h.painted = async (capMs) => {
    const end = performance.now() + capMs;
    while (h.flush().fcp === null && performance.now() < end) await new Promise((r) => setTimeout(r, 25));
    return h.flush();
  };
})();`;

/** How long a load waits for a paint the browser has not delivered yet, ms. */
export const PAINT_WAIT_MS = 2_000;

/** Waits, at most {@link PAINT_WAIT_MS}, for the first content to be known; answers the figures. */
export const AWAIT_PAINT = `window.__harness.painted(${PAINT_WAIT_MS})`;

/** The page's figures, every queued entry folded in first. */
export const READ_FIGURES = 'window.__harness.flush()';

/** A fresh pause count from here on: what was queued before is folded, then forgotten. */
export const RESET_PAUSE = '(() => { window.__harness.flush(); window.__harness.longest = 0; })()';

/**
 * An uncaught exception as the page's console would name it: the thrown
 * error's description (`Error: boom` and its stack), else the protocol's
 * text (`Uncaught` for a thrown string).
 *
 * @param {{exception?: {description?: string}, text?: string}} d `Runtime.exceptionThrown`'s details
 */
export function exceptionText(d) {
  return String(d.exception?.description ?? d.text ?? 'an exception the browser did not describe');
}

/**
 * The browser to drive: `--chrome`, else `BRUTEX_CHROME`, else the first of
 * {@link CHROME_PATHS} that exists, else empty (and `launch` refuses it by name).
 *
 * @param {string | undefined} given
 * @param {string | undefined} env
 * @param {(path: string) => boolean} exists
 */
export function chromeOf(given, env, exists) {
  return given ?? env ?? CHROME_PATHS.find((p) => exists(p)) ?? '';
}

/**
 * A page as one page, however it is spelled — as the browser reads the
 * address the run opens (`<base><page>`: dot segments resolved, a fragment
 * dropped, an empty query dropped) and then as the app's router reads its
 * path (`$lib/speed.js` `routePath`: escapes decoded, case kept, one
 * trailing slash the same page); the query is kept as the browser wrote it
 * (a value such as an instrument key is case sensitive). `null` for a path
 * the router cannot read. Linear in the length, with no pattern that can
 * backtrack.
 *
 * NOT FOLDED TO LOWER CASE (verify r4, 23 Oct): the router matches a route
 * case-sensitively, so `/SPEED` draws the app's not-found page, not the
 * Speed page; this key folded it into `/speed` and the run then measured the
 * not-found page under the Speed page's name. And `/speed/../speed`,
 * `/%73peed`, `/speed#x`, `/speed?` and `/speed/.` each opened the Speed
 * page under a key of its own, so one page could be measured twice.
 *
 * @param {string} page a path, starting `/`
 * @returns {string | null}
 */
export function pageKey(page) {
  const u = new URL(`http://h${page}`);
  const path = routePath(u.pathname);
  return path === null ? null : path + u.search;
}

/**
 * The route the app's router draws for `page`, from `routes`, the route ids
 * of the build that answered (its route dictionary, {@link READ_ROUTES}):
 * matched as the router matches a route with no parameter — the path read
 * by `routePath`, equal to the id less its group segments (`(name)`), case
 * kept. `{route: null, unmatched}` when none matches; `unmatched` lists the
 * ids with a parameter (`[id]`), which this harness does not match, so a
 * page that could be one of them is said, never passed as no page or as a
 * page.
 *
 * @param {string} page a path, starting `/`
 * @param {readonly string[]} routes
 * @returns {{route: string | null, unmatched: string[]}}
 */
export function routeOf(page, routes) {
  const path = routePath(new URL(`http://h${page}`).pathname);
  /** @type {string[]} */
  const unmatched = [];
  for (const id of routes) {
    const segments = id.split('/').filter((s) => s !== '' && !/^\(.+\)$/.test(s));
    if (segments.some((s) => s.includes('['))) unmatched.push(id);
    else if (path !== null && `/${segments.join('/')}` === path) return { route: id, unmatched: [] };
  }
  return { route: null, unmatched };
}

/**
 * How long the route read waits for the app entry's module, ms, in the page
 * ({@link READ_ROUTES}) and, with {@link FRAME_SLACK_MS}, here: the module
 * is one the page already holds, read in under 4 ms on the shadow (n = 200),
 * so one that has not loaded by then is said, never waited on.
 */
export const ROUTES_WAIT_MS = 5_000;

/**
 * The route ids of the build that drew the page in a tab: the app entry the
 * document names (`/_app/immutable/entry/app.<hash>.js`, the module
 * SvelteKit's start script ran) imported again in the page — the module the
 * page already holds, so nothing is fetched (checked on the Mac, 23 Oct: 0
 * new resource entries, under 1 ms) — and the ids of its route
 * `dictionary`. A document that names no app entry, or an entry with no
 * dictionary, answers `{routes: null, why}`: that document is not the app's.
 *
 * WHY THE BUILD'S OWN LIST (verify r4, 23 Oct): the server answers every
 * path it has no file for with the app's shell and 200, so the status says
 * nothing, and the shell then draws the app's not-found page; `--pages
 * /nope,/SPEED` wrote a PASS sheet for two pages the app does not have. The
 * build that answered is the one that knows its routes; the harness's own
 * source tree may be another build's.
 */
export const READ_ROUTES = `(async () => {
  const entry = /\\/_app\\/immutable\\/entry\\/app\\.[^/]+\\.js$/;
  const named = [...document.querySelectorAll('link[href]')].map((l) => l.href)
    .concat([...document.scripts].flatMap((s) => [...s.textContent.matchAll(/import\\(\\s*["']([^"']+)["']\\s*\\)/g)].map((m) => new URL(m[1], document.baseURI).href)));
  const at = named.find((u) => { try { return entry.test(new URL(u).pathname); } catch { return false; } });
  if (at === undefined) return { routes: null, why: 'the document names no app entry (/_app/immutable/entry/app.*.js)' };
  const where = new URL(at).pathname;
  try {
    const imported = await Promise.race([import(at).then((m) => ({ m })), new Promise((r) => setTimeout(() => r(null), ${ROUTES_WAIT_MS}))]);
    if (imported === null) return { routes: null, why: 'its app entry ' + where + ' did not load within ${ROUTES_WAIT_MS} ms' };
    const d = imported.m.dictionary;
    if (d === null || typeof d !== 'object' || Array.isArray(d)) return { routes: null, why: 'its app entry ' + where + ' holds no route dictionary' };
    return { routes: Object.keys(d), entry: where };
  } catch (e) {
    return { routes: null, why: 'its app entry ' + where + ' could not be imported: ' + String((e && e.message) || e) };
  }
})()`;

/**
 * Why a page is not the app's, or `''` when it is: the route list read in
 * the tab ({@link READ_ROUTES}) and the page's path ({@link routeOf}).
 *
 * @param {string} page
 * @param {{routes?: unknown, why?: unknown} | undefined} read what `READ_ROUTES` answered in the tab
 */
export function notTheApps(page, read) {
  if (read === undefined || read === null || typeof read !== 'object') return 'the harness could not read the app\'s routes in the page';
  if (!Array.isArray(read.routes)) return `the document is not the app's: ${String(read.why ?? 'it named no route list')}`;
  const routes = read.routes.map(String);
  const r = routeOf(page, routes);
  if (r.route !== null) return '';
  const listed = routes.filter((id) => !r.unmatched.includes(id)).join(', ');
  if (r.unmatched.length > 0) return `the harness cannot tell whether the app has this page: no plain route of the build that answered is ${page}, and it does not match the routes with a parameter (${r.unmatched.join(', ')})`;
  return `the app has no such page: no route of the build that answered matches ${page} (its routes: ${listed}), so it drew its not-found page`;
}

/** @param {string[]} argv */
export function parseArgs(argv) {
  /** @type {Record<string, string | boolean | string[]>} */
  const o = { hook: [] };
  const flags = new Set(['no-interactions', 'no-idle', 'no-report-check', 'no-sandbox', 'read-only']);
  const valued = new Set(['base', 'chrome', 'n', 'pages', 'idle-tabs', 'idle-ms', 'capture', 'out', 'settle-ms', 'step-wait-ms', 'load-wait-ms', 'instrument']);
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (!a.startsWith('--')) throw new Error(`unexpected argument ${JSON.stringify(a)}`);
    const name = a.slice(2);
    const v = argv[i + 1];
    if (flags.has(name)) o[name] = true;
    else if (name === 'hook' || valued.has(name)) {
      if (v === undefined || v.startsWith('--')) throw new Error(`--${name} needs a value`);
      if (name === 'hook') /** @type {string[]} */ (o.hook).push(v);
      else o[name] = v;
      i += 1;
    } else throw new Error(`unknown option --${name}`);
  }
  const base = String(o.base ?? '');
  if (!/^https?:\/\/[^/]+$/.test(base)) throw new Error('--base must be an origin such as http://127.0.0.1:8080 (no path)');
  const whole = (/** @type {string} */ name, /** @type {number} */ dflt, /** @type {number} */ min, max = Number.MAX_SAFE_INTEGER) => {
    const v = o[name] === undefined ? dflt : DECIMAL.test(String(o[name])) ? Number(o[name]) : Number.NaN;
    if (!Number.isSafeInteger(v) || v < min) throw new Error(`--${name} must be a whole number of at least ${min}, in decimal digits`);
    if (v > max) throw new Error(`--${name} must be at most ${max}`);
    return v;
  };
  const pages = String(o.pages ?? MEASURED_PAGES.join(',')).split(',');
  if (pages.some((p) => !p.startsWith('/'))) throw new Error('--pages is a comma list of paths, each starting /');
  // ONE PAGE, ONE SPELLING, AS THE ROUTER READS IT (pageKey): /speed,
  // /speed/, /%73peed and /speed/../speed are one page (verify r3 and r4);
  // /SPEED is another address, which the run then finds is no page of the
  // app. One pass, one map lookup a page.
  /** @type {Map<string, string>} each page's key, to the spelling that named it first */
  const named = new Map();
  for (const p of pages) {
    const key = pageKey(p);
    if (key === null) throw new Error(`--pages names ${p}, whose path no router can read: a percent escape that is not UTF-8`);
    const first = named.get(key);
    if (first !== undefined) throw new Error(`--pages names ${first} twice${first === p ? '' : ` (as ${first} and ${p})`}: each page is measured once a run`);
    named.set(key, p);
  }
  const idleTabs = String(o['idle-tabs'] ?? '1,10,20').split(',').map((t) => (DECIMAL.test(t) ? Number(t) : Number.NaN));
  if (idleTabs.some((t) => !Number.isSafeInteger(t) || t < 1 || t > 64)) throw new Error('--idle-tabs is a comma list of whole numbers from 1 to 64');
  const chrome = chromeOf(o.chrome === undefined ? undefined : String(o.chrome), process.env.BRUTEX_CHROME, existsSync);
  const instrument = String(o.instrument ?? EDGE_INSTRUMENT);
  if (rungsUrl(instrument) === null || linkOfKey(instrument) === null) throw new Error(`--instrument ${JSON.stringify(instrument)} is not an instrument key (NSE-INDEX-NIFTY, NSE-EQ-RELIANCE, ...)`);
  // ONE FILE IS NOT BOTH (verify r5 follow-up, 23 Oct: --out and --capture
  // naming one path left the sheet where the capture was, and exit 0).
  if (o.out !== undefined && o.capture !== undefined && resolvePath(String(o.out)) === resolvePath(String(o.capture))) {
    throw new Error(`--out and --capture name one file (${String(o.out)}): the sheet would be written over the capture`);
  }
  return {
    base,
    chrome,
    instrument,
    n: whole('n', 200, 1, MAX_N),
    pages,
    idleTabs,
    idleMs: whole('idle-ms', 3_600_000, 1_000, MAX_WAIT_MS),
    // The settle and step waits are clock loops, not one timer, but no wait
    // option takes a value no timer could hold.
    settleMs: whole('settle-ms', 500, 50, MAX_WAIT_MS),
    stepWaitMs: whole('step-wait-ms', STEP_WAIT_MS, 100, MAX_WAIT_MS),
    // A load's bound: from the navigation to its load event (verify r4).
    loadWaitMs: whole('load-wait-ms', LOAD_WAIT_MS, 100, MAX_WAIT_MS),
    interactions: o['no-interactions'] !== true,
    idle: o['no-idle'] !== true,
    // --read-only sends no write, so it never runs the reporter check (one POST).
    reportCheck: o['no-report-check'] !== true && o['read-only'] !== true,
    readOnly: o['read-only'] === true,
    noSandbox: o['no-sandbox'] === true,
    hooks: /** @type {string[]} */ (o.hook),
    capture: o.capture === undefined ? null : String(o.capture),
    out: o.out === undefined ? null : String(o.out)
  };
}

/**
 * How long any DevTools command waits for the browser's answer, ms, unless
 * its caller gives a bound of its own (a load, a step's selector wait, the
 * two frames). A browser that does not answer within it is refused by name
 * ({@link CdpTimeout}), never waited on for ever (verify r4, 23 Oct: a
 * document that sent its headers and part of its body held a run for 9 min
 * 49 s with nothing printed, because no wait on the browser had a bound).
 */
export const CDP_WAIT_MS = 30_000;

/** How long one load waits for its load event, ms (`--load-wait-ms`). */
export const LOAD_WAIT_MS = 30_000;

/**
 * How long a tab is given to draw two frames, ms. A hidden tab draws none
 * (verify r4, 23 Oct: 3 of 3 hidden tabs drew no frame in 10 s), so the
 * wait ends after this and says the frames were not drawn.
 */
export const FRAME_WAIT_MS = 2_000;

/**
 * How much longer than a page-side wait (the two frames, the paint) this
 * process waits for the page's answer to it, ms: a hidden page's timers lag
 * (measured 23 Oct: a hidden headless tab answered a 1.5 s page-side wait
 * after 6.7 s), so the page's own bound alone does not bound the wait.
 */
export const FRAME_SLACK_MS = 1_000;

/** A DevTools command the browser did not answer within its bound. */
export class CdpTimeout extends Error {}

/**
 * A wait on the browser for `expression` (a page-side wait bounded by
 * `pageMs`), given {@link CDP_WAIT_MS} more for the answer to travel, and
 * never more than a timer holds.
 *
 * @param {number} pageMs
 */
export const answerWithin = (pageMs) => Math.min(MAX_WAIT_MS, pageMs + CDP_WAIT_MS);

/**
 * One DevTools connection: commands answered by id, events by name and
 * session. EVERY COMMAND IS BOUNDED: an answer that has not come within its
 * wait rejects with {@link CdpTimeout} naming the command and the bound, and
 * an answer that comes after that is dropped (the command was already
 * refused by name). A connection that closes refuses every command still
 * waiting, and every later one, by name.
 */
export class Cdp {
  /** @param {WebSocket} ws @param {number} [waitMs] the default bound of a command */
  constructor(ws, waitMs = CDP_WAIT_MS) {
    this.ws = ws;
    this.next = 1;
    this.waitMs = waitMs;
    /** Why the connection can carry no command, once it cannot. @type {string | null} */
    this.closed = null;
    /** @type {Map<number, {resolve: (v: any) => void, reject: (e: Error) => void, timer: ReturnType<typeof setTimeout>, method: string}>} */
    this.waiting = new Map();
    /** @type {Set<(m: any) => void>} */
    this.listeners = new Set();
    ws.addEventListener('message', (ev) => {
      const m = JSON.parse(String(ev.data));
      if (m.id !== undefined) {
        const w = this.waiting.get(m.id);
        // Answered after its wait ended: that command was already refused.
        if (w === undefined) return;
        this.waiting.delete(m.id);
        clearTimeout(w.timer);
        if (m.error) w.reject(new Error(`${m.error.message} (${m.error.code})`));
        else w.resolve(m.result);
      } else for (const l of this.listeners) l(m);
    });
    ws.addEventListener('close', () => {
      this.closed = 'the browser\'s DevTools connection closed';
      for (const w of this.waiting.values()) {
        clearTimeout(w.timer);
        w.reject(new Error(`${w.method}: ${this.closed}`));
      }
      this.waiting.clear();
    });
  }

  /**
   * @param {string} method @param {object} [params] @param {string} [sessionId]
   * @param {number} [waitMs] how long the answer may take, ms (the connection's default)
   * @returns {Promise<any>}
   */
  send(method, params = {}, sessionId, waitMs = this.waitMs) {
    if (this.closed !== null) return Promise.reject(new Error(`${method}: ${this.closed}`));
    const id = this.next++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.waiting.delete(id);
        reject(new CdpTimeout(`the browser did not answer ${method} within ${waitMs} ms`));
      }, waitMs);
      this.waiting.set(id, { resolve, reject, timer, method });
      this.ws.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
    });
  }

  /** @param {(m: any) => void} f @returns {() => void} */
  on(f) {
    this.listeners.add(f);
    return () => this.listeners.delete(f);
  }
}

/** The name every browser profile this harness makes starts with, in the temporary directory. */
export const PROFILE_PREFIX = 'brutex-harness-';

/**
 * The most loads of a page one run takes, cold and warm each (`--n`): fifty
 * times the n = 200 the targets are measured at. A run past it would take
 * days, and is refused by name rather than started (the R3-15 re-check,
 * 21 Oct: `--n` had no ceiling).
 */
export const MAX_N = 10_000;

/**
 * How early a Node timer can end, by the clock: it counts whole milliseconds
 * of a loop clock read before the wait, so up to 1 ms (measured 23 Oct on
 * this Mac, Node 24.1: 0.633 ms early at the most over 800 timers). An idle
 * wait that ended earlier than this before the time asked was not a wait of
 * that time, and {@link idleRow} says so.
 */
export const IDLE_EARLY_MS = 1;

/**
 * Waits `ms` and returns how long it waited, by the clock — the idle row
 * prints this, never the time asked. A wait no timer can hold (past
 * {@link MAX_WAIT_MS}, below zero, or not a whole number) is refused by name.
 *
 * @param {number} ms
 * @param {{wait?: (ms: number) => Promise<unknown>, now?: () => number}} [io] a test's timer and clock
 * @returns {Promise<number>} the milliseconds waited
 */
export async function idleFor(ms, io = {}) {
  if (!Number.isSafeInteger(ms) || ms < 0 || ms > MAX_WAIT_MS) throw new Error(`an idle wait of ${ms} ms is not one a timer holds (a whole number from 0 to ${MAX_WAIT_MS})`);
  const wait = io.wait ?? ((/** @type {number} */ d) => new Promise((resolve) => setTimeout(resolve, d)));
  const now = io.now ?? (() => performance.now());
  const t0 = now();
  await wait(ms);
  return now() - t0;
}

/**
 * The sheet's row for one idle check: the time the tabs were measured idle
 * as the clock read it, never the time asked. A wait that ended more than
 * {@link IDLE_EARLY_MS} early is NOT MEASURED and fails the run by name: the
 * tabs were not idle for the time the row would otherwise claim.
 *
 * A TAB THE BROWSER HID IS SAID: a hidden page may poll less than a visible
 * one, so an idle check over hidden tabs claims less than it seems to (the
 * idle hour of 23 Oct: 2 of 20 headless windows read `visible`). It is a
 * note, not a failure: what the hidden tabs did ask is still listed.
 *
 * @param {{count: number, visible: number, askedMs: number, idledMs: number, requests: number, other: string[]}} o
 * @returns {{row: string, failures: string[], notes: string[]}}
 */
export function idleRow(o) {
  const secs = (/** @type {number} */ v) => `${(v / 1_000).toFixed(0)} s`;
  const said = o.other.length === 0 ? 'none' : o.other.join('; ');
  /** @type {string[]} */
  const failures = [];
  const hidden = o.count - o.visible;
  const notes = hidden > 0 ? [`${o.count} idle tabs: ${hidden} of them were hidden, and a hidden page may poll less than a visible one`] : [];
  if (o.idledMs + IDLE_EARLY_MS < o.askedMs) {
    failures.push(`${o.count} idle tabs: NOT MEASURED — the wait ended after ${o.idledMs.toFixed(1)} ms of the ${o.askedMs} ms asked`);
    return { row: `| ${o.count} | ${o.visible} | NOT MEASURED: ${o.idledMs.toFixed(1)} ms of ${o.askedMs} ms asked | ${o.requests} | ${said} |`, failures, notes };
  }
  if (o.other.length > 0) failures.push(`${o.count} idle tabs asked for more than the header-only up check: ${said}`);
  return { row: `| ${o.count} | ${o.visible} | ${secs(o.idledMs)} | ${o.requests} | ${said} |`, failures, notes };
}

/**
 * Every profile directory this process made and has not removed yet.
 * {@link removeProfile} removes only these: a path that merely LOOKS like
 * one (the prefix and six characters) is refused (the R3-15 re-check,
 * 21 Oct: the name alone was checked, so R315-13 claimed more than the code
 * did).
 *
 * @type {Set<string>}
 */
const MADE = new Set();

/** How long a browser has to leave after SIGTERM before it is killed, ms. */
export const STOP_WAIT_MS = 5_000;

/**
 * Removes a profile directory this harness made, and nothing else: the path
 * must be one {@link launch} made in this process (kept in {@link MADE}) and
 * named as `mkdtempSync` names one for {@link PROFILE_PREFIX} (the prefix and
 * its six random characters, nothing past them), so any other path is
 * refused by name, never removed.
 *
 * @param {string} profile
 */
export function removeProfile(profile) {
  if (!MADE.has(profile) || !new RegExp(`^${PROFILE_PREFIX}[A-Za-z0-9]{6}$`).test(basename(profile))) {
    throw new Error(`refusing to remove ${JSON.stringify(profile)}: not a profile this harness made in this process (${PROFILE_PREFIX}XXXXXX)`);
  }
  rmSync(profile, { recursive: true, force: true, maxRetries: 3 });
  MADE.delete(profile);
}

/**
 * Every browser this process started and has not stopped yet. If the
 * process ends first — it exits, throws, or is sent SIGINT, SIGTERM or
 * SIGHUP — each is killed and its profile removed (found by the R3-15
 * re-check, 20 Oct: 127 profiles were left in the temporary directory, and a
 * harness that died left headless browsers running). A SIGKILL of the
 * harness itself runs nothing, so a browser it started can outlive it then
 * (docs/06-limits.md).
 *
 * @type {Set<{killNow: () => void}>}
 */
const LIVE = new Set();
let guarded = false;

/** Installs the exit and signal guards, once a process. */
function guardProcess() {
  if (guarded) return;
  guarded = true;
  process.on('exit', () => {
    for (const b of LIVE) b.killNow();
  });
  for (const signal of /** @type {const} */ (['SIGINT', 'SIGTERM', 'SIGHUP'])) {
    const on = () => {
      for (const b of LIVE) b.killNow();
      process.removeListener(signal, on);
      // The signal keeps its meaning: with no other listener, the process
      // ends as it would have without this guard.
      if (process.listenerCount(signal) === 0) process.kill(process.pid, signal);
    };
    process.on(signal, on);
  }
}

// The browser's process group is asked after and signalled by the sheet's
// `groupRunning` and `signalGroup` (imported above), whose comments carry the
// 22 Oct measurement that made the stop a group's: the browser is started
// detached, so it leads a group of its own, and every helper it starts is in
// that group.

/**
 * A started browser's stop: SIGTERM to its process group, then SIGKILL to
 * the group after {@link STOP_WAIT_MS}; then the same wait for any helper
 * that outlived the browser, which is killed past it; its profile removed
 * only once nothing of the browser is left to write into it, and said by
 * name if it was written again all the same. Registered in {@link LIVE}
 * until it has stopped.
 *
 * @param {import('node:child_process').ChildProcess} child
 * @param {string} profile
 */
function tracked(child, profile) {
  let gone = false;
  const exited = new Promise((resolve) => {
    const done = () => {
      gone = true;
      resolve(undefined);
    };
    child.once('exit', done);
    child.once('error', done);
  });
  const alive = () => !gone && child.exitCode === null && child.signalCode === null && child.pid !== undefined;
  /** Waits, at most `ms`, for every process of the group to leave; whether they did. @param {number} ms */
  const groupLeft = async (ms) => {
    const until = Date.now() + ms;
    while (groupRunning(child.pid)) {
      if (Date.now() >= until) return false;
      await new Promise((resolve) => setTimeout(resolve, GROUP_POLL_MS));
    }
    return true;
  };
  const remove = () => {
    removeProfile(profile);
    if (existsSync(profile)) throw new Error(`${profile} was written again after it was removed`);
  };
  const entry = {
    // Runs inside an exit or signal handler, so it never throws and never
    // waits: what it could not do is said on stderr, by name. A process sent
    // SIGKILL runs nothing more, so the group is killed before the profile
    // goes.
    killNow() {
      LIVE.delete(entry);
      try {
        if (alive() || groupRunning(child.pid)) signalGroup(child, 'SIGKILL');
        remove();
      } catch (why) {
        process.stderr.write(`browser-harness: could not stop the browser ${child.pid} or remove ${profile}: ${/** @type {Error} */ (why).message}\n`);
      }
    }
  };
  LIVE.add(entry);
  guardProcess();
  return {
    alive,
    async stop() {
      if (alive()) {
        signalGroup(child, 'SIGTERM');
        /** @type {ReturnType<typeof setTimeout> | undefined} */
        let timer;
        await Promise.race([exited, new Promise((resolve) => (timer = setTimeout(resolve, STOP_WAIT_MS)))]);
        clearTimeout(timer);
        if (alive()) {
          signalGroup(child, 'SIGKILL');
          // Bounded too: a process the kernel cannot end yet is waited on no longer than the rest.
          await Promise.race([exited, new Promise((resolve) => (timer = setTimeout(resolve, STOP_WAIT_MS)))]);
          clearTimeout(timer);
        }
      }
      if (!(await groupLeft(STOP_WAIT_MS))) {
        signalGroup(child, 'SIGKILL');
        await groupLeft(STOP_WAIT_MS);
      }
      // Out of LIVE before the removal, not after: a removal that fails is
      // this stop's error, never a second one from the exit guard.
      LIVE.delete(entry);
      remove();
    }
  };
}

/**
 * Starts the browser headless on a debugging port it picks, in a new
 * profile directory under `root`, and connects. `close()` stops the browser
 * and waits for it to leave, then removes the profile; a launch that fails
 * stops what it started first. Either way nothing it made is left.
 *
 * @param {string} chrome
 * @param {boolean} [noSandbox]
 * @param {number} [waitMs] how long the browser has to print its DevTools address
 * @param {string} [root] where the profile is made (the temporary directory)
 */
export async function launch(chrome, noSandbox = false, waitMs = 30_000, root = tmpdir()) {
  if (!chrome || !existsSync(chrome)) throw new Error(`no browser at ${JSON.stringify(chrome)}: pass --chrome or set BRUTEX_CHROME`);
  const profile = mkdtempSync(join(root, PROFILE_PREFIX));
  MADE.add(profile);
  // `--no-sandbox` only when the operator asks (a browser run as root, in a
  // container, refuses to start without it): never chosen here on its own.
  const flags = ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-background-timer-throttling', '--disable-renderer-backgrounding', ...(noSandbox ? ['--no-sandbox'] : []), 'about:blank'];
  // DETACHED, so the browser leads a process group of its own and a stop
  // reaches every helper it starts (signalGroup); nothing else about the
  // browser changes, and the exit and signal guards still end it with this
  // process.
  const child = spawn(chrome, flags, { stdio: ['ignore', 'ignore', 'pipe'], detached: true });
  const browser = tracked(child, profile);
  /** @type {WebSocket} */
  let ws;
  try {
    const url = await new Promise((resolve, reject) => {
      let text = '';
      const timer = setTimeout(() => reject(new Error(`the browser printed no DevTools address in ${waitMs} ms: ${text.slice(-400)}`)), waitMs);
      child.stderr?.on('data', (c) => {
        text += c;
        const m = /DevTools listening on (ws:\/\/\S+)/.exec(text);
        if (m) {
          clearTimeout(timer);
          resolve(m[1]);
        }
      });
      child.on('error', (why) => {
        clearTimeout(timer);
        reject(new Error(`the browser could not be started: ${why.message}`));
      });
      child.on('exit', (code) => {
        clearTimeout(timer);
        reject(new Error(`the browser exited ${code} before it listened: ${text.slice(-400)}`));
      });
    });
    const socket = new WebSocket(String(url));
    ws = socket;
    await new Promise((resolve, reject) => {
      // BOUNDED by the same wait as the address (verify r5 follow-up, 23 Oct):
      // a browser that never finished the handshake held the run for Node's
      // own 300 s.
      const timer = setTimeout(() => {
        reject(new Error(`could not connect to ${url}: no handshake within ${waitMs} ms`));
        socket.close();
      }, waitMs);
      socket.addEventListener('open', () => {
        clearTimeout(timer);
        resolve(undefined);
      }, { once: true });
      socket.addEventListener('error', () => {
        clearTimeout(timer);
        reject(new Error(`could not connect to ${url}`));
      }, { once: true });
    });
  } catch (why) {
    await browser.stop();
    throw why;
  }
  return {
    cdp: new Cdp(ws),
    close: async () => {
      ws.close();
      await browser.stop();
    },
    alive: browser.alive,
    profile,
    pid: child.pid
  };
}

/**
 * The Fetch pattern a read-only tab pauses: any address that mentions the
 * report route. Wide on purpose — Chrome's pattern rules decide only which
 * requests pause; {@link isReportWrite} decides which of them is the report.
 */
export const reportPausePattern = () => `*${REPORT_ROUTE}*`;

/**
 * Whether a paused request is the shared reporter's write: not a GET or a
 * HEAD, to the report route exactly, on the run's own origin. A script whose
 * name contains the route, a GET of it, or another origin's route is not.
 *
 * @param {{method: string, url: string}} request
 * @param {string} origin the run's origin
 */
export function isReportWrite(request, origin) {
  if (request.method === 'GET' || request.method === 'HEAD') return false;
  let u;
  try {
    u = new URL(request.url);
  } catch {
    return false;
  }
  return u.origin === origin && u.pathname === REPORT_ROUTE;
}

/**
 * A write a tab sent or tried to send: any request that is not a GET or a
 * HEAD, with its body as the browser reported it, and whether the browser
 * blocked it before it left (`--read-only`).
 *
 * @typedef {{method: string, url: string, body: string, blocked: boolean}} Write
 */

/**
 * The page-side wait for two frames, bounded: `true` once two frames are
 * drawn, `false` when `ms` passed first (a hidden tab draws no frame).
 *
 * @param {number} ms
 */
export const twoFrames = (ms) =>
  `new Promise((r) => { const t = setTimeout(() => r(false), ${ms}); requestAnimationFrame(() => requestAnimationFrame(() => { clearTimeout(t); r(true); })); })`;

/**
 * A response header by name, whatever case the browser reports it in, or
 * `null`.
 *
 * @param {Record<string, unknown> | undefined} headers
 * @param {string} name in lower case
 */
export function headerOf(headers, name) {
  for (const [k, v] of Object.entries(headers ?? {})) if (k.toLowerCase() === name) return String(v);
  return null;
}

/**
 * A tab: a target attached flat, with its network watched.
 *
 * `readOnly` (`--read-only`, with the run's `base`): the one write a page
 * makes on its own — the shared reporter's `POST /page-report`, sent whenever
 * a request failed — is BLOCKED IN THE BROWSER, so it never reaches the
 * server; every write a tab sent or tried is pushed to `writes`, and the
 * caller fails the run on one that was not blocked. Without it, a run
 * against a live server whose page met a failure would have written a log
 * line there, which "reads only" must not.
 *
 * EXACTLY THAT ADDRESS, NEVER ONE THAT MERELY CONTAINS IT (found on the Mac,
 * 19 Oct, Chrome 155): the first block was `Network.setBlockedURLs` with
 * `*\/page-report*`, and Chrome's blocked-URL patterns match anywhere in the
 * address, so it also blocked a script named `/lib/page-report.js` — the
 * reporter itself — and the page ran with no reporter, and the run read "no
 * write" for a page that had nothing installed to write. Now the Fetch
 * domain pauses the few requests whose address mentions the route
 * ({@link reportPausePattern}) and {@link isReportWrite} decides each one by
 * its parsed origin, path and method: the report is failed
 * (`BlockedByClient`) and every other paused request goes on unchanged.
 * Only the report route's look-alikes pause, so no other request's timing
 * moves. A hook that enables the Fetch domain on such a tab replaces this
 * pattern; the harness's own report check never runs read only.
 *
 * @param {Cdp} cdp
 * @param {string | undefined} context a browser context id, or the default
 * @param {{readOnly?: boolean, base?: string, writes?: Write[]}} [options]
 */
export async function openTab(cdp, context, options = {}) {
  const writes = options.writes ?? [];
  if (options.readOnly && !options.base) throw new Error('openTab: a read-only tab needs the run\'s base, to know which report address to block');
  const reportOrigin = options.readOnly ? new URL(String(options.base)).origin : null;
  /** @type {Map<string, Write>} */
  const writing = new Map();
  /** Network ids of writes this tab failed before they left (`--read-only`). */
  const blocked = new Set();
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', ...(context ? { browserContextId: context } : {}) });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  /** @type {Map<string, {method: string, url: string, status: number, answered: boolean, bytes: number}>} */
  const inflight = new Map();
  /** @type {{method: string, url: string, bytes: number, status: number, failed: string | null}[]} */
  const done = [];
  /** @type {string[]} */
  const errors = [];
  /**
   * The status the page's own document was answered with, per load, and the
   * build that answered it (its `x-brutex-build` header, or `null`).
   * @type {{status: number, build: string | null}}
   */
  const doc = { status: 0, build: null };
  /** @type {{drew: boolean | null}} */
  const frames = { drew: null };
  let lastActivity = Date.now();
  const off = cdp.on((m) => {
    if (m.sessionId !== sessionId) return;
    const p = m.params;
    if (m.method === 'Network.requestWillBeSent') {
      inflight.set(p.requestId, { method: p.request.method, url: p.request.url, status: 0, answered: false, bytes: 0 });
      lastActivity = Date.now();
      if (p.request.method !== 'GET' && p.request.method !== 'HEAD') {
        const w = { method: p.request.method, url: p.request.url, body: String(p.request.postData ?? ''), blocked: blocked.has(p.requestId) };
        writing.set(p.requestId, w);
        writes.push(w);
      }
    } else if (m.method === 'Network.responseReceived' || m.method === 'Network.responseReceivedExtraInfo') {
      // ANSWERED: its headers are here. A request whose body then ends
      // elsewhere (a worker's own script, a keepalive report the browser
      // finishes for the page) never reports its end in this session, and
      // is not one the page waits on: it no longer holds `settle` open.
      const r = inflight.get(p.requestId);
      lastActivity = Date.now();
      if (r) {
        r.answered = true;
        r.status = m.method === 'Network.responseReceived' ? p.response.status : r.status || Number(p.statusCode);
      }
      if (p.type === 'Document') {
        doc.status = p.response.status;
        doc.build = headerOf(p.response.headers, BUILD_HEADER);
      }
    } else if (m.method === 'Network.dataReceived') {
      const r = inflight.get(p.requestId);
      lastActivity = Date.now();
      if (r) r.bytes += Number(p.encodedDataLength);
    } else if (m.method === 'Network.loadingFinished' || m.method === 'Network.loadingFailed') {
      const r = inflight.get(p.requestId);
      inflight.delete(p.requestId);
      lastActivity = Date.now();
      const w = writing.get(p.requestId);
      if (w && m.method === 'Network.loadingFailed' && p.blockedReason) w.blocked = true;
      if (r) done.push({ method: r.method, url: r.url, bytes: Number(p.encodedDataLength ?? 0), status: r.status, failed: m.method === 'Network.loadingFailed' ? String(p.errorText) : null });
    } else if (m.method === 'Fetch.requestPaused' && reportOrigin !== null) {
      // READ ONLY. A write is marked blocked only once the browser said it
      // failed it; if that call is refused the write stays unblocked and the
      // verdict fails the run naming it. A request let through whose
      // continue is refused never ends, so its load does not settle and says
      // so: neither case is hidden.
      if (isReportWrite(p.request, reportOrigin)) {
        cdp.send('Fetch.failRequest', { requestId: p.requestId, errorReason: 'BlockedByClient' }, sessionId).then(() => {
          blocked.add(p.networkId);
          const w = writing.get(p.networkId);
          if (w) w.blocked = true;
        }, () => undefined);
      } else cdp.send('Fetch.continueRequest', { requestId: p.requestId }, sessionId).catch(() => undefined);
    } else if (m.method === 'Runtime.exceptionThrown') {
      errors.push(exceptionText(p.exceptionDetails));
    } else if (m.method === 'Target.attachedToTarget') {
      // A WORKER THAT STARTED HAS ITS SCRIPT: the page session sees the
      // request and never its end, which the worker's own target reports.
      lastActivity = Date.now();
      for (const r of inflight.values()) if (r.url === p.targetInfo.url) r.answered = true;
    }
  });
  /**
   * Whether two frames were drawn within `ms`, by the page's own clock and
   * by this one: a hidden page's timers and answers can lag (measured 23 Oct:
   * a hidden headless tab answered a 1.5 s page-side wait after 6.7 s), so
   * the wait also ends {@link FRAME_SLACK_MS} past `ms` here, and an answer
   * after that is dropped.
   *
   * @param {number} ms
   * @returns {Promise<boolean>}
   */
  const drawn = async (ms) => (await within(twoFrames(ms), ms)).value === true;
  /**
   * A page-side wait (`expression`, bounded in the page by `ms`), waited on
   * here at most {@link FRAME_SLACK_MS} past `ms` as well: `{late: true}`
   * when the answer had not come by then, and that answer is dropped. The
   * page's own clock is not enough: a hidden page's timers lag.
   *
   * @param {string} expression
   * @param {number} ms
   * @returns {Promise<{value: unknown, late: boolean}>}
   */
  const within = async (expression, ms) => {
    const asked = cdp.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId, answerWithin(ms));
    /** @type {ReturnType<typeof setTimeout> | undefined} */
    let t;
    const LATE = Symbol('late');
    const late = new Promise((resolve) => (t = setTimeout(() => resolve(LATE), ms + FRAME_SLACK_MS)));
    try {
      const got = await Promise.race([asked.then((a) => a?.result?.value), late]);
      return got === LATE ? { value: undefined, late: true } : { value: got, late: false };
    } finally {
      clearTimeout(t);
      asked.catch(() => undefined);
    }
  };
  for (const domain of ['Page', 'Network', 'Runtime']) await cdp.send(`${domain}.enable`, {}, sessionId);
  if (reportOrigin !== null) await cdp.send('Fetch.enable', { patterns: [{ urlPattern: reportPausePattern(), requestStage: 'Request' }] }, sessionId);
  await cdp.send('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true }, sessionId);
  await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: OBSERVER }, sessionId);
  return {
    targetId,
    sessionId,
    doc,
    done,
    errors,
    /** Every write this tab sent or tried (see `Write`). */
    writes,
    /**
     * @param {string} expression
     * @param {number} [waitMs] how long the answer may take (the connection's default)
     */
    evaluate: async (expression, waitMs) => (await cdp.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId, waitMs)).result?.value,
    /** When this tab last saw a network event, ms since the epoch. */
    lastActive: () => lastActivity,
    /** Every request this tab made since its last `reset`, ended or not, with its bytes. */
    seen: () => [...done, ...[...inflight.values()].map((r) => ({ method: r.method, url: r.url, bytes: r.bytes, status: r.status, failed: null }))],
    /** A clean slate for the next load: a navigation ends the last page's requests. */
    reset: () => {
      inflight.clear();
      done.length = 0;
      doc.status = 0;
      doc.build = null;
      frames.drew = null;
    },
    /** Whether the last settle saw two frames drawn (`null` before any). */
    frames,
    /**
     * Settled: no network event for `quietMs` and no request still waiting
     * for its answer, at most `capMs`; then two frames drawn, at most
     * `frameMs` — a hidden tab draws none, and that is said in `frames.drew`
     * rather than waited on (verify r4, 23 Oct: the two-frame wait had no
     * bound). Whether no request was still waiting.
     */
    settle: async (quietMs = 500, capMs = 30_000, frameMs = FRAME_WAIT_MS) => {
      const end = Date.now() + capMs;
      const waiting = () => [...inflight.values()].some((r) => !r.answered);
      while (Date.now() < end && (waiting() || Date.now() - lastActivity < quietMs)) await new Promise((r) => setTimeout(r, 25));
      frames.drew = await drawn(frameMs);
      return !waiting();
    },
    drawn,
    within,
    close: async () => {
      off();
      await cdp.send('Target.closeTarget', { targetId });
    }
  };
}

/**
 * One load of `path`: navigate, wait for the load event, settle, read the
 * four figures.
 *
 * EVERY WAIT ON THE BROWSER IS BOUNDED (verify r4, 23 Oct): the navigation
 * and its load event together by `loadWaitMs`; the settle by its cap and its
 * two frames by {@link FRAME_WAIT_MS}; each command after by its own bound.
 * A load that did not end within its bound — a document whose body never
 * ends, a server that never answers, or a page that stopped answering the
 * browser — answers `ended: false` with `why`, its loading stopped, and its
 * figures are not figures of the page. Before, `/stall` (headers and part of
 * a body, never the end) held a run for 9 min 49 s with nothing printed.
 *
 * @param {Awaited<ReturnType<typeof openTab>>} tab
 * @param {Cdp} cdp
 * @param {string} url
 * @param {number} settleMs
 * @param {number} [loadWaitMs]
 */
export async function loadOnce(tab, cdp, url, settleMs, loadWaitMs = LOAD_WAIT_MS) {
  tab.reset();
  /** @type {() => void} */
  let off = () => undefined;
  const loaded = new Promise((resolve) => {
    off = cdp.on((m) => {
      if (m.sessionId === tab.sessionId && m.method === 'Page.loadEventFired') resolve(true);
    });
  });
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let timer;
  const bound = new Promise((resolve) => (timer = setTimeout(() => resolve(false), loadWaitMs)));
  const t0 = Date.now();
  /** @param {string} why */
  const unended = async (why) => {
    // What the tab is still loading is stopped; the caller closes the tab.
    await cdp.send('Page.stopLoading', {}, tab.sessionId).catch(() => undefined);
    const seen = tab.seen();
    return { ended: false, why, fcp: null, longest: 0, heap: 0, wire: seen.reduce((s, r) => s + r.bytes, 0), settled: false, drew: false, status: tab.doc.status, build: tab.doc.build, requests: seen.map((r) => ({ method: r.method, url: r.url, status: r.status, failed: r.failed })) };
  };
  let ended;
  try {
    ended = await Promise.race([cdp.send('Page.navigate', { url }, tab.sessionId, loadWaitMs).then(() => loaded), bound]);
  } catch (why) {
    if (!(why instanceof CdpTimeout)) throw why;
    ended = false;
  } finally {
    clearTimeout(timer);
    off();
  }
  if (!ended) return unended(`the load did not end within ${loadWaitMs} ms${tab.doc.status === 0 ? ' (no answer to its document)' : ` (its document was answered HTTP ${tab.doc.status} and did not finish)`}`);
  let settled;
  let h;
  let heap;
  try {
    settled = await tab.settle(settleMs);
    // At most PAINT_WAIT_MS by the page's clock, and FRAME_SLACK_MS more by this one (D-5915 item 9).
    await tab.within(AWAIT_PAINT, PAINT_WAIT_MS);
    h = await tab.evaluate(READ_FIGURES);
    heap = await cdp.send('Runtime.getHeapUsage', {}, tab.sessionId);
  } catch (why) {
    // The page loaded and then stopped answering: its renderer is busy or gone.
    if (!(why instanceof CdpTimeout)) throw why;
    return unended(`the page stopped answering ${Date.now() - t0} ms after its load began: ${why.message}`);
  }
  const seen = tab.seen();
  return {
    ended: true,
    why: '',
    fcp: typeof h.fcp === 'number' ? h.fcp : null,
    longest: Number(h.longest),
    heap: Number(heap.usedSize),
    wire: seen.reduce((s, r) => s + r.bytes, 0),
    settled,
    drew: tab.frames.drew === true,
    status: tab.doc.status,
    build: tab.doc.build,
    requests: seen.map((r) => ({ method: r.method, url: r.url, status: r.status, failed: r.failed }))
  };
}

/**
 * A page's figures against the targets: the lines to print and the
 * failures, each naming the figure.
 *
 * A load whose tab drew no frame within {@link FRAME_WAIT_MS} once settled
 * (`drew: false`; a hidden tab draws none) is not a settled load, said by
 * count (verify r4, 23 Oct).
 *
 * @param {string} page
 * @param {string} phase
 * @param {{fcp: number | null, longest: number, heap: number, wire: number, settled: boolean, status: number, drew?: boolean}[]} loads
 */
export function judge(page, phase, loads) {
  const fcp = summary(loads.map((l) => l.fcp).filter((v) => v !== null).map(Number));
  const longest = summary(loads.map((l) => l.longest));
  const heap = summary(loads.map((l) => l.heap));
  const wire = summary(loads.map((l) => l.wire));
  /** @type {string[]} */
  const failures = [];
  // A LOAD THAT WAS NOT THE PAGE IS NOT TIMED AS ONE: a 503 "front end not
  // built" answers fast and paints, and was read as a fast page.
  const refused = loads.filter((l) => l.status < 200 || l.status >= 300);
  if (refused.length > 0) failures.push(`${page} ${phase}: ${refused.length} of ${loads.length} loads were not the page (HTTP ${[...new Set(refused.map((l) => l.status))].join(', ')})`);
  const unpainted = loads.filter((l) => l.fcp === null).length;
  if (unpainted > 0) failures.push(`${page} ${phase}: ${unpainted} of ${loads.length} loads painted no first content`);
  if (fcp.n > 0 && fcp.p99 >= PAGE_TARGETS.firstContentP99Ms) failures.push(`${page} ${phase}: first content p99 ${fcp.p99.toFixed(1)} ms, not under ${PAGE_TARGETS.firstContentP99Ms} ms`);
  if (longest.max > PAGE_TARGETS.longestPauseMs) failures.push(`${page} ${phase}: a pause of ${longest.max.toFixed(1)} ms, over ${PAGE_TARGETS.longestPauseMs} ms`);
  if (heap.max >= PAGE_TARGETS.heapBytes) failures.push(`${page} ${phase}: script heap ${(heap.max / 1e6).toFixed(1)} MB, not under ${PAGE_TARGETS.heapBytes / 1e6} MB`);
  if (wire.max >= PAGE_TARGETS.wireBytes) failures.push(`${page} ${phase}: ${(wire.max / 1e3).toFixed(1)} KB on the wire, not under ${PAGE_TARGETS.wireBytes / 1e3} KB`);
  const unsettled = loads.filter((l) => !l.settled).length;
  if (unsettled > 0) failures.push(`${page} ${phase}: ${unsettled} loads still had a request in flight after 30 s`);
  const undrawn = loads.filter((l) => l.drew === false).length;
  if (undrawn > 0) failures.push(`${page} ${phase}: ${undrawn} of ${loads.length} loads drew no frame within ${FRAME_WAIT_MS} ms of settling (a hidden tab draws none), so they are not settled loads`);
  const f = (/** @type {{n: number, p50: number, p99: number, max: number}} */ s, /** @type {(v: number) => string} */ t) => `${t(s.p50)} / ${t(s.p99)} / ${t(s.max)}`;
  const ms = (/** @type {number} */ v) => v.toFixed(1);
  const row = `| ${page} | ${phase} | ${loads.length} | ${fcp.n === 0 ? 'not painted' : f(fcp, ms)} | ${f(longest, ms)} | ${f(heap, (v) => (v / 1e6).toFixed(1))} | ${f(wire, (v) => (v / 1e3).toFixed(1))} | ${failures.length === 0 ? 'holds' : 'FAILS'} |`;
  return { row, failures };
}

/**
 * Whether every request an idle tab made is the header-only up check.
 *
 * @param {{method: string, url: string}[]} requests
 * @param {string} base
 */
export function idleVerdict(requests, base) {
  const origin = new URL(base).origin;
  /** @type {Map<string, number>} */
  const counts = new Map();
  /** @type {string[]} */
  const other = [];
  for (const r of requests) {
    const u = new URL(r.url, origin);
    const key = `${r.method} ${u.origin === origin ? u.pathname + u.search : u.href}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
    if (!(r.method === UP_METHOD && u.origin === origin && u.pathname === UP_ROUTE)) other.push(key);
  }
  return { counts, other: [...new Set(other)] };
}

/**
 * The page's own reads on its first load, for the sheet: same origin, GET
 * and HEAD, path and query only.
 *
 * @param {string} page
 * @param {{method: string, url: string}[]} requests
 * @param {string} base
 */
export function captured(page, requests, base) {
  const origin = new URL(base).origin;
  return requests
    .filter((r) => (r.method === 'GET' || r.method === 'HEAD') && new URL(r.url, origin).origin === origin)
    .map((r) => {
      const u = new URL(r.url, origin);
      return { page, method: r.method, url: u.pathname + u.search };
    })
    .filter((r) => !r.url.startsWith('/_app/') && !/\.(html|css|js|svg|ico|png|woff2?)$/.test(new URL(r.url, origin).pathname) && r.url !== page);
}

/**
 * The shared reporter, end to end: the page's next read of `/feeds.json` is
 * failed at the network, and the report is looked for in the server's log.
 *
 * @param {Cdp} cdp
 * @param {string} base
 * @param {number} settleMs
 * @param {number} [loadWaitMs]
 */
async function reportCheck(cdp, base, settleMs, loadWaitMs = LOAD_WAIT_MS) {
  const tab = await openTab(cdp, undefined);
  try {
    const opened = await loadOnce(tab, cdp, `${base}/speed`, settleMs, loadWaitMs);
    if (!opened.ended) return { row: 'not checked: /speed did not load', failures: [`the shared reporter was not checked: /speed: ${opened.why}`] };
    const marker = `/feeds.json?harness=${Date.now()}`;
    await cdp.send('Fetch.enable', { patterns: [{ urlPattern: `*${marker}*` }] }, tab.sessionId);
    const off = cdp.on((m) => {
      if (m.sessionId === tab.sessionId && m.method === 'Fetch.requestPaused') void cdp.send('Fetch.failRequest', { requestId: m.params.requestId, errorReason: 'Failed' }, tab.sessionId);
    });
    await tab.evaluate(`fetch(${JSON.stringify(marker)}).catch(() => null)`);
    await tab.settle(settleMs);
    off();
    await cdp.send('Fetch.disable', {}, tab.sessionId);
    const sent = tab.seen().find((r) => r.method === 'POST' && new URL(r.url).pathname === REPORT_ROUTE);
    if (!sent) return { row: 'the failed request sent no report', failures: [`the shared reporter sent no ${REPORT_ROUTE} for a failed ${marker}`] };
    if (sent.status !== 202) return { row: `${REPORT_ROUTE} answered ${sent.status}`, failures: [`${REPORT_ROUTE} answered ${sent.status} for the report of ${marker}, not 202 written`] };
    // Bounded in the page and in the answer: a log that does not answer is said, never waited on.
    const logs = await tab.evaluate(`fetch('/logs.json?target=api.page.access&limit=50', { cache: 'no-store', signal: AbortSignal.timeout(${CDP_WAIT_MS}) }).then(async (r) => ({ text: await r.text() }), (e) => ({ why: String((e && e.message) || e) }))`, answerWithin(CDP_WAIT_MS));
    if (typeof logs?.why === 'string') return { row: 'the log could not be read', failures: [`the report of the failed ${marker} was answered 202 and /logs.json?target=api.page.access could not be read: ${logs.why}`] };
    const found = String(logs?.text).includes(marker);
    return found
      ? { row: `a failed ${marker} was reported (202) and found in /logs.json`, failures: [] }
      : { row: 'the report was written and not found in the log', failures: [`the report of the failed ${marker} was answered 202 and is not in /logs.json?target=api.page.access`] };
  } finally {
    await tab.close();
  }
}

/**
 * @typedef {{cdp: Cdp, base: string, settleMs: number, openTab: typeof openTab, loadOnce: typeof loadOnce}} Harness
 */

/**
 * The page the browser draws once before anything is timed: text, so the
 * paint path runs, and a `data:` address, so the server is asked nothing
 * and no cache of the site is made.
 */
export const WARM_UP_PAGE = 'data:text/html,<!doctype html><title>warm-up</title><p>warm-up</p>';

/**
 * THE BROWSER IS STARTED BEFORE ANY PAGE IS TIMED (repair round 1, 20 Oct).
 * The first page a new browser draws pays for the browser's own start — its
 * paint and network processes come up then — and that cost landed on the
 * first cold sample of the first page: measured on a five-line page, cold
 * first content 1,108 ms at n = 1, and 20 ms p50 against a 2,272 ms max at
 * n = 5. It is the browser starting, not the page loading (a reader's
 * browser is already running), and it was the 2,332 ms max in the 20 Oct
 * `/db` cold row. So one load of {@link WARM_UP_PAGE} runs first, in a
 * browser context of its own that is then thrown away: every timed cold
 * load is still a fresh context with no cache and no cookies.
 *
 * @param {Cdp} cdp
 * @param {(context: string | undefined) => ReturnType<typeof openTab>} open
 * @param {number} settleMs
 */
async function warmUp(cdp, open, settleMs) {
  const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
  const tab = await open(browserContextId);
  const warmed = await loadOnce(tab, cdp, WARM_UP_PAGE, settleMs);
  await tab.close();
  await cdp.send('Target.disposeBrowserContext', { browserContextId });
  // A browser that cannot draw a page that asks nothing of the server can time no page.
  if (!warmed.ended) throw new Error(`the browser did not draw its warm-up page: ${warmed.why}`);
}

/** @param {ReturnType<typeof parseArgs>} args */
export async function runHarness(args) {
  const browser = await launch(args.chrome, args.noSandbox);
  const { cdp } = browser;
  /** @type {string[]} */
  const lines = ['| page | phase | n | first content p50 / p99 / max ms | longest pause p50 / p99 / max ms | heap p50 / p99 / max MB | wire p50 / p99 / max KB | targets |', '|---|---|---:|---|---|---|---|---|'];
  /** @type {string[]} */
  const failures = [];
  /** @type {string[]} */
  const notes = [];
  /** @type {{page: string, method: string, url: string}[]} */
  const capture = [];
  /** @type {Write[]} every write any tab of this run sent or tried */
  const writes = [];
  /** @param {string | undefined} context */
  const open = (context) => openTab(cdp, context, { readOnly: args.readOnly, base: args.base, writes });
  /** Every build that answered a page's document this run (`x-brutex-build`). @type {Set<string>} */
  const builds = new Set();
  let unstamped = 0;
  /** A document the server answered names its build, or is counted unstamped. @param {{build: string | null, status: number}} l */
  const stamped = (l) => {
    if (l.status === 0) return;
    if (l.build === null) unstamped += 1;
    else builds.add(l.build);
  };
  /**
   * EVERY TAB'S UNCAUGHT ERRORS, by where and text, counted (verify r5
   * follow-up, 23 Oct: only the warm tab's were read, so an error a cold load,
   * an interaction or an idle tab threw was never reported). Failed by name
   * once the run has ended.
   *
   * @type {Map<string, Map<string, number>>}
   */
  const thrown = new Map();
  /** @param {string} where @param {{errors: string[]}} tab */
  const heard = (where, tab) => {
    if (tab.errors.length === 0) return;
    const by = thrown.get(where) ?? new Map();
    for (const e of tab.errors) by.set(e, (by.get(e) ?? 0) + 1);
    thrown.set(where, by);
  };
  /** One load of `page`, bounded. @param {Awaited<ReturnType<typeof openTab>>} tab @param {string} page */
  const load = async (tab, page) => {
    const l = await loadOnce(tab, cdp, `${args.base}${page}`, args.settleMs, args.loadWaitMs);
    stamped(l);
    return l;
  };
  try {
    await warmUp(cdp, open, args.settleMs);
    for (const page of args.pages) {
      // A PAGE THAT IS NOT THE APP'S, OR WHOSE LOAD DOES NOT END, IS NOT
      // TIMED AGAIN: its row says why, it fails by name, and the run goes on
      // to the next page (verify r4, 23 Oct).
      let stop = '';
      let stoppedIn = 'cold';
      /** @type {Awaited<ReturnType<typeof loadOnce>>[]} */
      const cold = [];
      for (let i = 0; i < args.n && stop === ''; i += 1) {
        const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
        const tab = await open(browserContextId);
        try {
          const l = await load(tab, page);
          if (!l.ended) stop = l.why;
          else {
            // The first load that the server answered with a page is asked
            // which page the app drew: the build's own routes (READ_ROUTES).
            if (i === 0 && l.status >= 200 && l.status < 300) {
              // Bounded in the page and here: a page that does not answer is this page's failure, by name.
              const read = await tab.within(READ_ROUTES, ROUTES_WAIT_MS);
              stop = read.late ? `the page did not answer within ${ROUTES_WAIT_MS + FRAME_SLACK_MS} ms when asked for the app's routes` : notTheApps(page, /** @type {any} */ (read.value));
            }
            cold.push(l);
          }
        } finally {
          heard(`${page} cold`, tab);
          await tab.close();
          await cdp.send('Target.disposeBrowserContext', { browserContextId });
        }
      }
      /** @type {Awaited<ReturnType<typeof loadOnce>>[]} */
      const warm = [];
      if (stop === '') {
        stoppedIn = 'warm';
        const tab = await open(undefined);
        try {
          const first = await load(tab, page);
          if (!first.ended) stop = first.why;
          for (let i = 0; i < args.n && stop === ''; i += 1) {
            const l = await load(tab, page);
            if (l.ended) warm.push(l);
            else stop = l.why;
          }
        } finally {
          heard(`${page} warm`, tab);
          await tab.close();
        }
      }
      /** The row of a phase that could not be measured, and its failure by name. @param {string} phase @param {number} n */
      const unmeasured = (phase, n) => {
        lines.push(`| ${page} | ${phase} | ${n} | NOT MEASURED: ${stop} | — | — | — | FAILS |`);
        failures.push(`${page} ${phase}: ${stop}`);
      };
      // Not measured cold, so not captured either: the sheet replays only a measured page's own requests.
      if (stop !== '' && stoppedIn === 'cold') {
        unmeasured('cold', cold.length);
        continue;
      }
      capture.push(...captured(page, cold[0].requests, args.base));
      const coldRow = judge(page, 'cold', cold);
      lines.push(coldRow.row);
      failures.push(...coldRow.failures);
      // Its cold loads stand when a warm load is the one that did not end.
      if (stop !== '') {
        unmeasured('warm', warm.length);
        continue;
      }
      const warmRow = judge(page, 'warm', warm);
      lines.push(warmRow.row);
      failures.push(...warmRow.failures);
    }
    if (args.interactions) {
      lines.push('', '| interaction | n | click to its last answer or frame p50 / p99 / max ms | longest pause max ms |', '|---|---:|---|---:|');
      /** @type {Awaited<ReturnType<typeof sortEdge>> | null} */
      let edge = null;
      let edgeWhy = '';
      const interactions = interactionsFor(args.instrument);
      if (interactions.some((it) => it.edge)) {
        try {
          edge = await sortEdge(async (path) => {
            // Bounded like a load: a server that does not answer is said, never waited on.
            try {
              const r = await fetch(`${args.base}${path}`, { cache: 'no-store', signal: AbortSignal.timeout(args.loadWaitMs) });
              return { status: r.status, body: await r.text() };
            } catch (why) {
              throw new Error(`finding the sort edge: ${path} had no whole answer within ${args.loadWaitMs} ms: ${/** @type {Error} */ (why).message}`);
            }
          }, args.instrument);
          notes.push(`sort edge: ${edge.address} holds ${edge.rows} bars of ${edge.feed} ${edge.rung}, the longest range of days ending ${edge.to} at or under ${LOCAL_SORT_BUDGET} (${edge.asked} GETs${edge.passed.length > 0 ? `; passed over ${edge.passed.join('; ')}` : ''})`);
        } catch (why) {
          edgeWhy = String(/** @type {{message?: unknown}} */ (why)?.message ?? why);
        }
      }
      for (const it of interactions) {
        const at = it.edge ? edge?.address ?? null : it.open;
        if (at === null) {
          lines.push(`| ${it.name} | 0 | NOT MEASURED: ${edgeWhy} | — |`);
          notes.push(`${it.name}: NOT MEASURED — ${edgeWhy}`);
          continue;
        }
        /** @type {number[]} */
        const times = [];
        let pause = 0;
        let missing = '';
        /** Why a sample could not be taken at all: its page did not load, or stopped answering. */
        let broke = '';
        let unsettled = 0;
        let undrawn = 0;
        for (let i = 0; i < Math.min(args.n, 50) && missing === '' && broke === ''; i += 1) {
          // A BROWSER CONTEXT OF ITS OWN FOR EVERY SAMPLE (found on the Mac,
          // 20 Oct): the feed switch persists the feed it chose, so in one
          // shared context the next sample of the next interaction opened
          // /db on the other feed, where NIFTY holds no option, and its
          // expiry picker was refused. Each sample starts from what a reader
          // opening the address starts from.
          const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
          const tab = await open(browserContextId);
          try {
            const opened = await load(tab, at);
            if (!opened.ended) {
              broke = `${at}: ${opened.why}`;
              continue;
            }
            let t0 = 0;
            let drawnAt = 0;
            let drew = true;
            for (const step of it.steps) {
              const sel = 'click' in step ? step.click : 'type' in step ? step.type[0] : step.waitFor;
              const found = await tab.evaluate(`(async () => { const end = Date.now() + ${args.stepWaitMs}; while (Date.now() < end) { if (document.querySelector(${JSON.stringify(sel)})) return true; await new Promise((r) => setTimeout(r, 50)); } return Boolean(document.querySelector(${JSON.stringify(sel)})); })()`, answerWithin(args.stepWaitMs));
              if (!found) {
                missing = sel;
                break;
              }
              if ('type' in step) await tab.evaluate(`(() => { const el = document.querySelector(${JSON.stringify(step.type[0])}); el.focus(); el.value = ${JSON.stringify(step.type[1])}; el.dispatchEvent(new Event('input', { bubbles: true })); })()`);
              if ('click' in step) {
                // SETTLED BEFORE EVERY CLICK: the click is timed alone, never
                // with the tail of the step before it still loading.
                await tab.settle(args.settleMs);
                await tab.evaluate(RESET_PAUSE);
                t0 = Date.now();
                await tab.evaluate(`[...document.querySelectorAll(${JSON.stringify(step.click)})].at(${step.last ? -1 : 0}).click()`);
                // Two frames after the click, bounded: a tab that draws none is not timed.
                drew = await tab.drawn(FRAME_WAIT_MS);
                drawnAt = Date.now();
              }
            }
            if (missing === '') {
              if (!drew) undrawn += 1;
              else if (await tab.settle(args.settleMs)) times.push(Math.max(tab.lastActive(), drawnAt) - t0);
              else unsettled += 1;
              pause = Math.max(pause, Number((await tab.evaluate(READ_FIGURES)).longest));
            }
          } catch (why) {
            // A page that stopped answering ends this interaction, by name; the browser itself is asked again below.
            if (!(why instanceof CdpTimeout)) throw why;
            broke = `${at}: the page stopped answering: ${why.message}`;
          } finally {
            heard(it.name, tab);
            await tab.close();
            await cdp.send('Target.disposeBrowserContext', { browserContextId });
          }
        }
        if (broke !== '') {
          lines.push(`| ${it.name} | 0 | NOT MEASURED: ${broke} | — |`);
          failures.push(`${it.name}: NOT MEASURED — ${broke}`);
          continue;
        }
        if (missing !== '') {
          lines.push(`| ${it.name} | 0 | NOT MEASURED: ${missing} is not on ${at} | — |`);
          notes.push(`${it.name}: NOT MEASURED — the selector ${missing} was not found on ${at}`);
          continue;
        }
        if (unsettled > 0) failures.push(`${it.name}: ${unsettled} click(s) still had a request in flight 30 s later, not timed`);
        if (undrawn > 0) failures.push(`${it.name}: ${undrawn} click(s) drew no frame within ${FRAME_WAIT_MS} ms, not timed`);
        const s = summary(times);
        lines.push(s.n === 0 ? `| ${it.name} | 0 | NOT TIMED: no click settled within 30 s | ${pause.toFixed(1)} |` : `| ${it.name} | ${s.n} | ${s.p50.toFixed(1)} / ${s.p99.toFixed(1)} / ${s.max.toFixed(1)} | ${pause.toFixed(1)} |`);
        if (pause > PAGE_TARGETS.longestPauseMs) failures.push(`${it.name}: a pause of ${pause.toFixed(1)} ms, over ${PAGE_TARGETS.longestPauseMs} ms`);
      }
    }
    if (args.idle) {
      lines.push('', '| idle tabs | visible | for | requests | other than the up check |', '|---:|---:|---:|---:|---|');
      for (const count of args.idleTabs) {
        const tabs = [];
        /** @type {string[]} */
        const contexts = [];
        for (let i = 0; i < count; i += 1) {
          // EACH IN A BROWSER CONTEXT, SO A WINDOW, OF ITS OWN: a background
          // tab is hidden and a hidden page polls nothing, so N tabs in one
          // window measured one.
          const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
          contexts.push(browserContextId);
          const tab = await open(browserContextId);
          const idlePage = args.pages[i % args.pages.length];
          const opened = await load(tab, idlePage);
          // Still idled and listed: what a tab whose load did not end asks is still asked.
          if (!opened.ended) failures.push(`${count} idle tabs: ${idlePage}: ${opened.why}`);
          tab.reset();
          tabs.push(tab);
        }
        /** @type {unknown[]} */
        const states = [];
        for (const t of tabs) states.push(await t.evaluate('document.visibilityState'));
        const visible = states.filter((s) => s === 'visible').length;
        // MEASURED, NOT ASKED: the row prints the time the clock read (verify r3).
        const idledMs = await idleFor(args.idleMs);
        const requests = tabs.flatMap((t) => t.seen());
        const v = idleVerdict(requests, args.base);
        const idle = idleRow({ count, visible, askedMs: args.idleMs, idledMs, requests: requests.length, other: v.other });
        lines.push(idle.row);
        failures.push(...idle.failures);
        notes.push(...idle.notes);
        for (const t of tabs) {
          heard(`${count} idle tabs`, t);
          await t.close();
        }
        for (const c of contexts) await cdp.send('Target.disposeBrowserContext', { browserContextId: c });
      }
    }
    if (args.reportCheck) {
      const r = await reportCheck(cdp, args.base, args.settleMs, args.loadWaitMs);
      lines.push('', `Shared reporter: ${r.row}.`);
      failures.push(...r.failures);
    }
    for (const hook of args.hooks) {
      const mod = await import(pathToFileURL(hook).href);
      // A hook's tabs are the run's tabs: `--read-only` holds for them too.
      /** @type {typeof openTab} */
      const hookTab = (c, context, options = {}) => openTab(c, context, { readOnly: args.readOnly, base: args.base, writes, ...options });
      /** @type {{rows?: string[], failures?: string[]}} */
      const out = (await mod.default({ cdp, base: args.base, settleMs: args.settleMs, openTab: hookTab, loadOnce })) ?? {};
      lines.push('', `Hook ${hook}:`, ...(out.rows ?? []));
      failures.push(...(out.failures ?? []));
    }
  } catch (why) {
    // A RUN THAT COULD NOT FINISH IS SAID, AND ITS SHEET IS STILL WRITTEN: the
    // browser stopped answering (every command is bounded), or a hook threw.
    // What was measured before it stands; the run fails by name.
    const said = String(/** @type {{message?: unknown}} */ (why)?.message ?? why);
    lines.push('', `The run stopped here: ${said}.`);
    failures.push(`the run stopped before it finished: ${said}`);
  } finally {
    await browser.close();
  }
  for (const [where, by] of thrown) {
    for (const [e, count] of by) failures.push(`${where}: an uncaught error: ${e}${count > 1 ? ` (${count} times)` : ''}`);
  }
  if (args.readOnly) {
    const v = readOnlyVerdict(writes, args.base);
    notes.push(...v.notes);
    failures.push(...v.failures);
  }
  // THE BUILD THAT ANSWERED, named (verify r4 follow-up): the speed sheet
  // stamps every row; the harness names the build of every document it
  // timed, and two builds in one run fail it (a deploy mid-run).
  if (builds.size > 1) failures.push(`the pages were answered by ${builds.size} builds (${[...builds].join(', ')}): a deploy during the run, so its figures are not one build's`);
  if (unstamped > 0) notes.push(`page loads answered with no ${BUILD_HEADER} header, so no build is named for them: ${unstamped}`);
  if (args.capture) {
    try {
      writeWhole(args.capture, `${JSON.stringify({ base: args.base, requests: capture }, null, 1)}\n`);
    } catch (why) {
      // The sheet is still written: a capture that cannot be is this run's failure, never its end.
      failures.push(`the capture could not be written: ${/** @type {Error} */ (why).message}`);
    }
  }
  return { ok: failures.length === 0, lines, failures, notes, builds: [...builds] };
}

/**
 * What a `--read-only` run says about the writes its pages sent or tried: a
 * write the browser blocked is a note (it never reached the server, and its
 * body is what the page wanted to say: a reporter's body names the failure
 * the page met), and a write that left the browser FAILS the run by name.
 *
 * @param {Write[]} writes
 * @param {string} base
 * @returns {{notes: string[], failures: string[]}}
 */
export function readOnlyVerdict(writes, base) {
  const origin = new URL(base).origin;
  /** @type {string[]} */
  const notes = [];
  /** @type {string[]} */
  const failures = [];
  for (const w of writes) {
    const u = new URL(w.url, origin);
    const where = `${w.method} ${u.origin === origin ? u.pathname + u.search : u.href}`;
    // A reporter's form body is printed as what it says; any other body as sent.
    const form = new URLSearchParams(w.body);
    const said = form.has('what') ? `${form.get('page') ?? '?'}: ${form.get('what')}` : w.body;
    if (w.blocked) notes.push(`read-only: ${where} was blocked in the browser and never sent${said === '' ? '' : ` (${said})`}`);
    else failures.push(`read-only: ${where} left the browser${said === '' ? '' : ` (${said})`}`);
  }
  return { notes, failures };
}

/**
 * The harness's sheet as Markdown: its tables, then every note and every
 * failure, under a line naming the base and the build that answered.
 *
 * @param {{ok: boolean, lines: string[], notes: string[], failures: string[], builds?: string[]}} result
 * @param {{n: number, base: string}} args
 */
export function renderHarness(result, args) {
  const builds = result.builds ?? [];
  const by = builds.length === 0 ? 'answered by no named build' : builds.length === 1 ? `answered by build ${builds[0]}` : `answered by ${builds.length} builds: ${builds.join(', ')}`;
  return [`# Browser harness — ${result.ok ? 'PASS' : 'FAIL'}`, '', `n = ${args.n} cold and ${args.n} warm loads a page against ${args.base}, ${by}.`, '', ...result.lines, '', ...result.notes.map((n) => `- ${n}`), ...result.failures.map((f) => `- FAILED: ${f}`), ''].join('\n');
}

/**
 * Why a file the run will write at its end cannot be written, or `''`: its
 * directory must exist, be a directory and take a new file, and the path
 * must not be a directory itself. Asked before the browser starts, so a run
 * is never thrown away at its end for a path it could have refused at once
 * (verify r4 follow-up: a `--capture` that could not be written ended the
 * run with exit 2 and no sheet, after the whole run).
 *
 * @param {string} option `--out` or `--capture`
 * @param {string} path
 */
export function unwritable(option, path) {
  const dir = dirname(path);
  try {
    if (!statSync(dir).isDirectory()) return `${option} ${path}: ${dir} is not a directory`;
    accessSync(dir, fsConstants.W_OK);
  } catch (why) {
    return `${option} ${path}: ${dir} cannot take a new file: ${/** @type {Error} */ (why).message}`;
  }
  try {
    if (statSync(path).isDirectory()) return `${option} ${path} is a directory`;
  } catch {
    // Not there yet: the file is made whole at the end (writeWhole).
  }
  return '';
}

async function main() {
  let args;
  try {
    args = parseArgs(process.argv.slice(2));
    for (const [option, path] of /** @type {const} */ ([['--out', args.out], ['--capture', args.capture]])) {
      const why = path === null ? '' : unwritable(option, path);
      if (why !== '') throw new Error(why);
    }
  } catch (why) {
    console.error(`browser-harness: ${/** @type {Error} */ (why).message}`);
    process.exit(2);
  }
  let result;
  try {
    result = await runHarness(args);
  } catch (why) {
    console.error(`browser-harness: ${/** @type {Error} */ (why).message}`);
    process.exit(2);
  }
  const text = renderHarness(result, args);
  process.stdout.write(text);
  if (args.out) {
    try {
      writeWhole(args.out, text);
    } catch (why) {
      // The sheet is on stdout already; the file that could not be written is named.
      console.error(`browser-harness: ${/** @type {Error} */ (why).message}`);
      process.exit(2);
    }
  }
  process.exit(result.ok ? 0 : 1);
}

if (import.meta.url === pathToFileURL(String(process.argv[1])).href) await main();

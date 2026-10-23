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
 *     [--settle-ms 500] [--step-wait-ms 30000] [--instrument <key>]
 *
 * No npm dependency: it drives the browser over the DevTools protocol on
 * Node's own WebSocket. Exit 0 when every target holds; 1 with every reason
 * otherwise; 2 for a misused command or no browser.
 */
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { PAGE_TARGETS, MEASURED_PAGES, summary } from '../src/lib/speed.js';
import { UP_ROUTE, UP_METHOD } from '../src/lib/api-status.js';
import { REPORT_ROUTE } from '../src/lib/page-report.js';
import { LOCAL_SORT_BUDGET, windowQuery } from '../src/lib/db/bars-plan.js';
import { rungsUrl, rungSeconds, SOURCES_DEFAULTS } from '../src/lib/sources.js';
import { linkOfKey } from '../src/lib/db/selection-model.js';
import { writeWhole, MAX_WAIT_MS, DECIMAL } from './speed-sheet.mjs';

// The longest wait a Node timer holds, and the sheet's own reading of a whole
// number: one rule for both commands (`speed-sheet.mjs` says why).
export { MAX_WAIT_MS };

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
 * A page as one page, however it is spelled: its path in lower case (the
 * app's routes are), its trailing slashes dropped (the root `/` kept), and a
 * query kept exactly as given (a value such as an instrument key is case
 * sensitive). Linear in the length, with no pattern that can backtrack.
 *
 * @param {string} page
 */
export function pageKey(page) {
  const q = page.indexOf('?');
  const path = q === -1 ? page : page.slice(0, q);
  let end = path.length;
  while (end > 1 && path[end - 1] === '/') end -= 1;
  return path.slice(0, end).toLowerCase() + (q === -1 ? '' : page.slice(q));
}

/** @param {string[]} argv */
export function parseArgs(argv) {
  /** @type {Record<string, string | boolean | string[]>} */
  const o = { hook: [] };
  const flags = new Set(['no-interactions', 'no-idle', 'no-report-check', 'no-sandbox', 'read-only']);
  const valued = new Set(['base', 'chrome', 'n', 'pages', 'idle-tabs', 'idle-ms', 'capture', 'out', 'settle-ms', 'step-wait-ms', 'instrument']);
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
  // ONE PAGE, ONE SPELLING: the app's routes are lower case and a trailing
  // slash names the same page, so /speed, /speed/ and /SPEED are one page
  // (verify r3, 22 Oct: they were measured as three).
  // One pass, one map lookup a page.
  /** @type {Map<string, string>} each page's key, to the spelling that named it first */
  const named = new Map();
  for (const p of pages) {
    const key = pageKey(p);
    const first = named.get(key);
    if (first !== undefined) throw new Error(`--pages names ${first} twice${first === p ? '' : ` (as ${first} and ${p})`}: each page is measured once a run`);
    named.set(key, p);
  }
  const idleTabs = String(o['idle-tabs'] ?? '1,10,20').split(',').map((t) => (DECIMAL.test(t) ? Number(t) : Number.NaN));
  if (idleTabs.some((t) => !Number.isSafeInteger(t) || t < 1 || t > 64)) throw new Error('--idle-tabs is a comma list of whole numbers from 1 to 64');
  const chrome = chromeOf(o.chrome === undefined ? undefined : String(o.chrome), process.env.BRUTEX_CHROME, existsSync);
  const instrument = String(o.instrument ?? EDGE_INSTRUMENT);
  if (rungsUrl(instrument) === null || linkOfKey(instrument) === null) throw new Error(`--instrument ${JSON.stringify(instrument)} is not an instrument key (NSE-INDEX-NIFTY, NSE-EQ-RELIANCE, ...)`);
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
 * One DevTools connection: commands answered by id, events by name and
 * session.
 */
export class Cdp {
  /** @param {WebSocket} ws */
  constructor(ws) {
    this.ws = ws;
    this.next = 1;
    /** @type {Map<number, {resolve: (v: any) => void, reject: (e: Error) => void}>} */
    this.waiting = new Map();
    /** @type {Set<(m: any) => void>} */
    this.listeners = new Set();
    ws.addEventListener('message', (ev) => {
      const m = JSON.parse(String(ev.data));
      if (m.id !== undefined) {
        const w = /** @type {{resolve: (v: any) => void, reject: (e: Error) => void}} */ (this.waiting.get(m.id));
        this.waiting.delete(m.id);
        if (m.error) w.reject(new Error(`${m.error.message} (${m.error.code})`));
        else w.resolve(m.result);
      } else for (const l of this.listeners) l(m);
    });
  }

  /** @param {string} method @param {object} [params] @param {string} [sessionId] @returns {Promise<any>} */
  send(method, params = {}, sessionId) {
    const id = this.next++;
    this.ws.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
    return new Promise((resolve, reject) => this.waiting.set(id, { resolve, reject }));
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

/** How often a stop looks again for a helper of the browser still running, ms. */
export const GROUP_POLL_MS = 25;

/**
 * Whether any process of the browser's process group is still running, asked
 * with signal 0, which signals nothing. The browser is started detached, so
 * it leads a process group of its own, and every helper it starts is in that
 * group: all ten processes of a headless Chrome on this Mac on 22 Oct (each
 * renderer, the GPU and network services, the rest), none in another group.
 * A group whose id is now someone else's cannot be signalled (EPERM) and is
 * not ours: it reads as gone.
 *
 * @param {number | undefined} pid the browser's pid, which is its group's id
 */
export function groupRunning(pid) {
  if (pid === undefined) return false;
  try {
    process.kill(-pid, 0);
    return true;
  } catch {
    return false;
  }
}

/**
 * Sends `signal` to the browser and every helper in its process group. Sent
 * only while the group is known to be the browser's — the browser not yet
 * reaped (its pid, so its group's id, cannot have passed to another
 * process), or {@link groupRunning} having just found the group's processes
 * still there (an id in use by a group is given to no new one). Never
 * throws: where the group cannot be signalled the browser alone is, through
 * its own process handle, which reaches nothing once the browser has been
 * reaped — so a stop never waits on a browser no signal reached.
 *
 * WHY THE GROUP, AND NOT THE BROWSER ALONE (the R3-15 repair 2 run, 22 Oct,
 * at a load average near 140): a harness sent SIGTERM killed the browser,
 * removed its profile at once, and Chrome's network service, still running,
 * wrote `Default/Reporting and NEL` into the profile again, so the profile
 * was left. Probed the same day: after a SIGTERM to the browser alone, the
 * browser left in 35 ms with a renderer still running; to its group, every
 * process was gone within 300 ms.
 *
 * @param {{pid?: number, kill: (signal: NodeJS.Signals | 0) => boolean}} child the browser's process
 * @param {NodeJS.Signals | 0} signal
 */
export function signalGroup(child, signal) {
  try {
    process.kill(-(/** @type {number} */ (child.pid)), signal);
  } catch {
    child.kill(signal);
  }
}

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
          await exited;
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
    ws = new WebSocket(String(url));
    await new Promise((resolve, reject) => {
      ws.addEventListener('open', resolve, { once: true });
      ws.addEventListener('error', () => reject(new Error(`could not connect to ${url}`)), { once: true });
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
  /** The status the page's own document was answered with, per load. */
  const doc = { status: 0 };
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
      if (p.type === 'Document') doc.status = p.response.status;
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
    /** @param {string} expression */
    evaluate: async (expression) => (await cdp.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId)).result?.value,
    /** When this tab last saw a network event, ms since the epoch. */
    lastActive: () => lastActivity,
    /** Every request this tab made since its last `reset`, ended or not, with its bytes. */
    seen: () => [...done, ...[...inflight.values()].map((r) => ({ method: r.method, url: r.url, bytes: r.bytes, status: r.status, failed: null }))],
    /** A clean slate for the next load: a navigation ends the last page's requests. */
    reset: () => {
      inflight.clear();
      done.length = 0;
      doc.status = 0;
    },
    /** Settled: no network event for `quietMs` and no request still waiting for its answer, then two frames drawn; at most `capMs`. */
    settle: async (quietMs = 500, capMs = 30_000) => {
      const end = Date.now() + capMs;
      const waiting = () => [...inflight.values()].some((r) => !r.answered);
      while (Date.now() < end && (waiting() || Date.now() - lastActivity < quietMs)) await new Promise((r) => setTimeout(r, 25));
      await cdp.send('Runtime.evaluate', { expression: 'new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))', awaitPromise: true }, sessionId);
      return !waiting();
    },
    close: async () => {
      off();
      await cdp.send('Target.closeTarget', { targetId });
    }
  };
}

/**
 * One load of `path`: navigate, settle, read the four figures.
 *
 * @param {Awaited<ReturnType<typeof openTab>>} tab
 * @param {Cdp} cdp
 * @param {string} url
 * @param {number} settleMs
 */
export async function loadOnce(tab, cdp, url, settleMs) {
  tab.reset();
  const loaded = new Promise((resolve) => {
    const off = cdp.on((m) => {
      if (m.sessionId === tab.sessionId && m.method === 'Page.loadEventFired') {
        off();
        resolve(undefined);
      }
    });
  });
  await cdp.send('Page.navigate', { url }, tab.sessionId);
  await loaded;
  const settled = await tab.settle(settleMs);
  await tab.evaluate(AWAIT_PAINT);
  const h = await tab.evaluate(READ_FIGURES);
  const heap = await cdp.send('Runtime.getHeapUsage', {}, tab.sessionId);
  const seen = tab.seen();
  return {
    fcp: typeof h.fcp === 'number' ? h.fcp : null,
    longest: Number(h.longest),
    heap: Number(heap.usedSize),
    wire: seen.reduce((s, r) => s + r.bytes, 0),
    settled,
    status: tab.doc.status,
    requests: seen.map((r) => ({ method: r.method, url: r.url, status: r.status, failed: r.failed }))
  };
}

/**
 * A page's figures against the targets: the lines to print and the
 * failures, each naming the figure.
 *
 * @param {string} page
 * @param {string} phase
 * @param {{fcp: number | null, longest: number, heap: number, wire: number, settled: boolean, status: number}[]} loads
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
 */
async function reportCheck(cdp, base, settleMs) {
  const tab = await openTab(cdp, undefined);
  try {
    await loadOnce(tab, cdp, `${base}/speed`, settleMs);
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
    const logs = await tab.evaluate(`fetch('/logs.json?target=api.page.access&limit=50', { cache: 'no-store' }).then((r) => r.text())`);
    const found = String(logs).includes(marker);
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
  await loadOnce(tab, cdp, WARM_UP_PAGE, settleMs);
  await tab.close();
  await cdp.send('Target.disposeBrowserContext', { browserContextId });
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
  try {
    await warmUp(cdp, open, args.settleMs);
    for (const page of args.pages) {
      /** @type {Awaited<ReturnType<typeof loadOnce>>[]} */
      const cold = [];
      for (let i = 0; i < args.n; i += 1) {
        const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
        const tab = await open(browserContextId);
        cold.push(await loadOnce(tab, cdp, `${args.base}${page}`, args.settleMs));
        await tab.close();
        await cdp.send('Target.disposeBrowserContext', { browserContextId });
      }
      capture.push(...captured(page, cold[0].requests, args.base));
      const tab = await open(undefined);
      await loadOnce(tab, cdp, `${args.base}${page}`, args.settleMs);
      /** @type {Awaited<ReturnType<typeof loadOnce>>[]} */
      const warm = [];
      for (let i = 0; i < args.n; i += 1) warm.push(await loadOnce(tab, cdp, `${args.base}${page}`, args.settleMs));
      for (const e of tab.errors) failures.push(`${page}: an uncaught error: ${e}`);
      await tab.close();
      for (const [phase, loads] of /** @type {const} */ ([['cold', cold], ['warm', warm]])) {
        const j = judge(page, phase, loads);
        lines.push(j.row);
        failures.push(...j.failures);
      }
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
            const r = await fetch(`${args.base}${path}`, { cache: 'no-store' });
            return { status: r.status, body: await r.text() };
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
        let unsettled = 0;
        for (let i = 0; i < Math.min(args.n, 50) && missing === ''; i += 1) {
          // A BROWSER CONTEXT OF ITS OWN FOR EVERY SAMPLE (found on the Mac,
          // 20 Oct): the feed switch persists the feed it chose, so in one
          // shared context the next sample of the next interaction opened
          // /db on the other feed, where NIFTY holds no option, and its
          // expiry picker was refused. Each sample starts from what a reader
          // opening the address starts from.
          const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
          const tab = await open(browserContextId);
          await loadOnce(tab, cdp, `${args.base}${at}`, args.settleMs);
          let t0 = 0;
          let drawnAt = 0;
          for (const step of it.steps) {
            const sel = 'click' in step ? step.click : 'type' in step ? step.type[0] : step.waitFor;
            const found = await tab.evaluate(`(async () => { const end = Date.now() + ${args.stepWaitMs}; while (Date.now() < end) { if (document.querySelector(${JSON.stringify(sel)})) return true; await new Promise((r) => setTimeout(r, 50)); } return Boolean(document.querySelector(${JSON.stringify(sel)})); })()`);
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
              await tab.evaluate('new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(true))))');
              drawnAt = Date.now();
            }
          }
          if (missing === '') {
            if (await tab.settle(args.settleMs)) times.push(Math.max(tab.lastActive(), drawnAt) - t0);
            else unsettled += 1;
            pause = Math.max(pause, Number((await tab.evaluate(READ_FIGURES)).longest));
          }
          await tab.close();
          await cdp.send('Target.disposeBrowserContext', { browserContextId });
        }
        if (missing !== '') {
          lines.push(`| ${it.name} | 0 | NOT MEASURED: ${missing} is not on ${at} | — |`);
          notes.push(`${it.name}: NOT MEASURED — the selector ${missing} was not found on ${at}`);
          continue;
        }
        if (unsettled > 0) failures.push(`${it.name}: ${unsettled} click(s) still had a request in flight 30 s later, not timed`);
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
          await loadOnce(tab, cdp, `${args.base}${args.pages[i % args.pages.length]}`, args.settleMs);
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
        for (const t of tabs) await t.close();
        for (const c of contexts) await cdp.send('Target.disposeBrowserContext', { browserContextId: c });
      }
    }
    if (args.reportCheck) {
      const r = await reportCheck(cdp, args.base, args.settleMs);
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
  } finally {
    await browser.close();
  }
  if (args.readOnly) {
    const v = readOnlyVerdict(writes, args.base);
    notes.push(...v.notes);
    failures.push(...v.failures);
  }
  if (args.capture) writeWhole(args.capture, `${JSON.stringify({ base: args.base, requests: capture }, null, 1)}\n`);
  return { ok: failures.length === 0, lines, failures, notes };
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
 * The harness's sheet as Markdown: its tables, then every note and every failure.
 *
 * @param {{ok: boolean, lines: string[], notes: string[], failures: string[]}} result
 * @param {{n: number, base: string}} args
 */
export function renderHarness(result, args) {
  return [`# Browser harness — ${result.ok ? 'PASS' : 'FAIL'}`, '', `n = ${args.n} cold and ${args.n} warm loads a page against ${args.base}.`, '', ...result.lines, '', ...result.notes.map((n) => `- ${n}`), ...result.failures.map((f) => `- FAILED: ${f}`), ''].join('\n');
}

async function main() {
  let args;
  try {
    args = parseArgs(process.argv.slice(2));
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
  if (args.out) writeWhole(args.out, text);
  process.exit(result.ok ? 0 : 1);
}

if (import.meta.url === pathToFileURL(String(process.argv[1])).href) await main();

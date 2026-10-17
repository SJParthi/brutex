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
 * Then, timed from the click to the page settled (no request in flight and
 * two frames drawn): a picker click, a feed switch, a page turn of option
 * cells and a sort at the 20,000-row browser sort edge (`LOCAL_SORT_BUDGET`).
 * Each names the selector it needs; a selector the page does not have is
 * NOT MEASURED, said, never a pass.
 *
 * Then idle tabs (`--idle-tabs 1,10,20`, each for `--idle-ms`, an hour by
 * default): every request an idle tab makes is listed; any but the
 * header-only up check (`HEAD /inspection.json`) fails the run.
 *
 * Then the shared reporter end to end: one request of a page is made to fail
 * at the network (`Fetch.failRequest`), and the report must be found in the
 * server's own log (`/logs.json?target=api.page.access`).
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
 *     [--no-report-check] [--hook <module.mjs>]... [--capture <file>] [--out <sheet.md>]
 *
 * No npm dependency: it drives the browser over the DevTools protocol on
 * Node's own WebSocket. Exit 0 when every target holds; 1 with every reason
 * otherwise; 2 for a misused command or no browser.
 */
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { PAGE_TARGETS, MEASURED_PAGES, summary } from '../src/lib/speed.js';
import { UP_ROUTE, UP_METHOD } from '../src/lib/api-status.js';
import { REPORT_ROUTE } from '../src/lib/page-report.js';
import { LOCAL_SORT_BUDGET } from '../src/lib/db/bars-plan.js';

/** Where a browser is looked for when `--chrome` and `BRUTEX_CHROME` say nothing. */
export const CHROME_PATHS = Object.freeze([
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
  '/opt/pw-browsers/chromium-1194/chrome-linux/chrome',
  '/usr/bin/chromium',
  '/usr/bin/google-chrome'
]);

/**
 * The timed interactions. Each step is `{click}`, `{type: [selector, text]}`
 * or `{waitFor}`; the time is from the LAST step's click to settled.
 * `open` is the address the page is opened at first.
 *
 * @typedef {{click: string} | {type: [string, string]} | {waitFor: string}} Step
 * @type {{name: string, open: string, steps: Step[]}[]}
 */
const TIMED = [
  { name: 'picker click', open: '/db', steps: [{ type: ['[aria-controls="db-search-list"]', 'NIFTY'] }, { waitFor: '#db-search-list [role="option"]' }, { click: '#db-search-list [role="option"]' }] },
  { name: 'feed switch', open: '/db', steps: [{ click: '[aria-controls="feed-listbox"]' }, { waitFor: '#feed-listbox [role="option"][aria-selected="false"]:not([aria-disabled="true"])' }, { click: '#feed-listbox [role="option"][aria-selected="false"]:not([aria-disabled="true"])' }] },
  { name: 'page turn of option cells', open: '/db?seg=options', steps: [{ waitFor: 'button[aria-label="Next page"]:not([disabled])' }, { click: 'button[aria-label="Next page"]:not([disabled])' }] },
  { name: `sort at the ${LOCAL_SORT_BUDGET.toLocaleString('en-IN')}-row browser sort edge`, open: '/db', steps: [{ waitFor: 'th[aria-sort] button:not([disabled])' }, { click: 'th[aria-sort] button:not([disabled])' }] }
];

/** {@link TIMED}, frozen. */
export const INTERACTIONS = Object.freeze(TIMED.map((t) => Object.freeze(t)));

/** The observer every page gets before its own scripts run. */
const OBSERVER = `(() => {
  const h = (window.__harness = { fcp: null, longest: 0, tasks: 0 });
  try {
    new PerformanceObserver((list) => {
      for (const e of list.getEntries()) if (e.name === 'first-contentful-paint') h.fcp = e.startTime;
    }).observe({ type: 'paint', buffered: true });
  } catch {}
  try {
    new PerformanceObserver((list) => {
      for (const e of list.getEntries()) { h.tasks += 1; if (e.duration > h.longest) h.longest = e.duration; }
    }).observe({ type: 'longtask', buffered: true });
  } catch {}
})();`;

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

/** @param {string[]} argv */
export function parseArgs(argv) {
  /** @type {Record<string, string | boolean | string[]>} */
  const o = { hook: [] };
  const flags = new Set(['no-interactions', 'no-idle', 'no-report-check', 'no-sandbox']);
  const valued = new Set(['base', 'chrome', 'n', 'pages', 'idle-tabs', 'idle-ms', 'capture', 'out', 'settle-ms']);
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
  const whole = (/** @type {string} */ name, /** @type {number} */ dflt, /** @type {number} */ min) => {
    const v = o[name] === undefined ? dflt : Number(o[name]);
    if (!Number.isSafeInteger(v) || v < min) throw new Error(`--${name} must be a whole number of at least ${min}`);
    return v;
  };
  const pages = String(o.pages ?? MEASURED_PAGES.join(',')).split(',');
  if (pages.some((p) => !p.startsWith('/'))) throw new Error('--pages is a comma list of paths, each starting /');
  const idleTabs = String(o['idle-tabs'] ?? '1,10,20').split(',').map(Number);
  if (idleTabs.some((t) => !Number.isSafeInteger(t) || t < 1 || t > 64)) throw new Error('--idle-tabs is a comma list of whole numbers from 1 to 64');
  const chrome = chromeOf(o.chrome === undefined ? undefined : String(o.chrome), process.env.BRUTEX_CHROME, existsSync);
  return {
    base,
    chrome,
    n: whole('n', 200, 1),
    pages,
    idleTabs,
    idleMs: whole('idle-ms', 3_600_000, 1_000),
    settleMs: whole('settle-ms', 500, 50),
    interactions: o['no-interactions'] !== true,
    idle: o['no-idle'] !== true,
    reportCheck: o['no-report-check'] !== true,
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

/**
 * Starts the browser headless on a debugging port it picks, in a new
 * profile directory, and connects.
 *
 * @param {string} chrome
 * @param {boolean} [noSandbox]
 * @param {number} [waitMs] how long the browser has to print its DevTools address
 */
export async function launch(chrome, noSandbox = false, waitMs = 30_000) {
  if (!chrome || !existsSync(chrome)) throw new Error(`no browser at ${JSON.stringify(chrome)}: pass --chrome or set BRUTEX_CHROME`);
  const profile = mkdtempSync(join(tmpdir(), 'brutex-harness-'));
  // `--no-sandbox` only when the operator asks (a browser run as root, in a
  // container, refuses to start without it): never chosen here on its own.
  const flags = ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-background-timer-throttling', '--disable-renderer-backgrounding', ...(noSandbox ? ['--no-sandbox'] : []), 'about:blank'];
  const child = spawn(chrome, flags, { stdio: ['ignore', 'ignore', 'pipe'] });
  const url = await new Promise((resolve, reject) => {
    let text = '';
    const timer = setTimeout(() => {
      child.kill('SIGKILL');
      reject(new Error(`the browser printed no DevTools address in ${waitMs} ms: ${text.slice(-400)}`));
    }, waitMs);
    child.stderr.on('data', (c) => {
      text += c;
      const m = /DevTools listening on (ws:\/\/\S+)/.exec(text);
      if (m) {
        clearTimeout(timer);
        resolve(m[1]);
      }
    });
    child.on('exit', (code) => {
      clearTimeout(timer);
      reject(new Error(`the browser exited ${code} before it listened: ${text.slice(-400)}`));
    });
  });
  const ws = new WebSocket(String(url));
  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, { once: true });
    ws.addEventListener('error', () => {
      child.kill('SIGKILL');
      reject(new Error(`could not connect to ${url}`));
    }, { once: true });
  });
  return { cdp: new Cdp(ws), close: () => { ws.close(); child.kill('SIGTERM'); }, profile };
}

/**
 * A tab: a target attached flat, with its network watched.
 *
 * @param {Cdp} cdp
 * @param {string | undefined} context a browser context id, or the default
 */
export async function openTab(cdp, context) {
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
      if (r) done.push({ method: r.method, url: r.url, bytes: Number(p.encodedDataLength ?? 0), status: r.status, failed: m.method === 'Network.loadingFailed' ? String(p.errorText) : null });
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
  await cdp.send('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true }, sessionId);
  await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: OBSERVER }, sessionId);
  return {
    targetId,
    sessionId,
    doc,
    done,
    errors,
    /** @param {string} expression */
    evaluate: async (expression) => (await cdp.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId)).result?.value,
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
  const h = await tab.evaluate('window.__harness');
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
  try {
    for (const page of args.pages) {
      /** @type {Awaited<ReturnType<typeof loadOnce>>[]} */
      const cold = [];
      for (let i = 0; i < args.n; i += 1) {
        const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
        const tab = await openTab(cdp, browserContextId);
        cold.push(await loadOnce(tab, cdp, `${args.base}${page}`, args.settleMs));
        await tab.close();
        await cdp.send('Target.disposeBrowserContext', { browserContextId });
      }
      capture.push(...captured(page, cold[0].requests, args.base));
      const tab = await openTab(cdp, undefined);
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
      lines.push('', '| interaction | n | click to settled p50 / p99 / max ms | longest pause max ms |', '|---|---:|---|---:|');
      for (const it of INTERACTIONS) {
        /** @type {number[]} */
        const times = [];
        let pause = 0;
        let missing = '';
        for (let i = 0; i < Math.min(args.n, 50) && missing === ''; i += 1) {
          const tab = await openTab(cdp, undefined);
          await loadOnce(tab, cdp, `${args.base}${it.open}`, args.settleMs);
          let t0 = 0;
          for (const step of it.steps) {
            const sel = 'click' in step ? step.click : 'type' in step ? step.type[0] : step.waitFor;
            const found = await tab.evaluate(`(async () => { for (let i = 0; i < 100; i += 1) { if (document.querySelector(${JSON.stringify(sel)})) return true; await new Promise((r) => setTimeout(r, 50)); } return false; })()`);
            if (!found) {
              missing = sel;
              break;
            }
            if ('type' in step) await tab.evaluate(`(() => { const el = document.querySelector(${JSON.stringify(step.type[0])}); el.focus(); el.value = ${JSON.stringify(step.type[1])}; el.dispatchEvent(new Event('input', { bubbles: true })); })()`);
            if ('click' in step) {
              await tab.evaluate('window.__harness.longest = 0');
              t0 = performance.now();
              await tab.evaluate(`document.querySelector(${JSON.stringify(step.click)}).click()`);
            }
          }
          if (missing === '') {
            await tab.settle(args.settleMs);
            times.push(performance.now() - t0);
            pause = Math.max(pause, Number(await tab.evaluate('window.__harness.longest')));
          }
          await tab.close();
        }
        if (missing !== '') {
          lines.push(`| ${it.name} | 0 | NOT MEASURED: ${missing} is not on ${it.open} | — |`);
          notes.push(`${it.name}: NOT MEASURED — the selector ${missing} was not found on ${it.open}`);
          continue;
        }
        const s = summary(times);
        lines.push(`| ${it.name} | ${s.n} | ${s.p50.toFixed(1)} / ${s.p99.toFixed(1)} / ${s.max.toFixed(1)} | ${pause.toFixed(1)} |`);
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
          const tab = await openTab(cdp, browserContextId);
          await loadOnce(tab, cdp, `${args.base}${args.pages[i % args.pages.length]}`, args.settleMs);
          tab.reset();
          tabs.push(tab);
        }
        /** @type {unknown[]} */
        const states = [];
        for (const t of tabs) states.push(await t.evaluate('document.visibilityState'));
        const visible = states.filter((s) => s === 'visible').length;
        await new Promise((r) => setTimeout(r, args.idleMs));
        const requests = tabs.flatMap((t) => t.seen());
        const v = idleVerdict(requests, args.base);
        lines.push(`| ${count} | ${visible} | ${(args.idleMs / 1_000).toFixed(0)} s | ${requests.length} | ${v.other.length === 0 ? 'none' : v.other.join('; ')} |`);
        if (v.other.length > 0) failures.push(`${count} idle tabs asked for more than the header-only up check: ${v.other.join('; ')}`);
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
      /** @type {{rows?: string[], failures?: string[]}} */
      const out = (await mod.default({ cdp, base: args.base, settleMs: args.settleMs, openTab, loadOnce })) ?? {};
      lines.push('', `Hook ${hook}:`, ...(out.rows ?? []));
      failures.push(...(out.failures ?? []));
    }
  } finally {
    browser.close();
  }
  if (args.capture) writeFileSync(args.capture, `${JSON.stringify({ base: args.base, requests: capture }, null, 1)}\n`);
  return { ok: failures.length === 0, lines, failures, notes };
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
  if (args.out) writeFileSync(args.out, text);
  process.exit(result.ok ? 0 : 1);
}

if (import.meta.url === pathToFileURL(String(process.argv[1])).href) await main();

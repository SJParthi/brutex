#!/usr/bin/env node
/**
 * THE SPEED SHEET — every route the pages ask, timed against a running
 * server, after every deploy, each row stamped with the build that answered
 * (R3-15; NEW-01, NEW-08).
 *
 * # What it replaces
 *
 * The live speed sheet was a hand-run script outside the app, one build
 * behind, that typed its own URLs. A URL typed by hand drifts from the page:
 * it times a request no page sends, and a field the server ignores times the
 * wrong answer (a `/bars.json?limit=2` that is answered with the whole month
 * is a 71 MB answer timed as two rows). So:
 *
 * * EVERY REQUEST IS THE PAGE'S OWN. Each line of {@link CATALOGUE} calls the
 *   builder the page itself calls (`windowQuery`, `dayflagsUrl`, ...), imported
 *   from the page's own library; a page that builds its URL inline in its
 *   `+page.svelte` (`/gaps`, `/audit`, `/markets`) is not copied by hand —
 *   its requests are captured from the page by the browser harness
 *   (`browser-harness.mjs --capture <file>`) and replayed here verbatim
 *   (`--from-pages <file>`). `web/tests/speed.test.js`
 *   `the_speed_sheet_uses_the_pages_own_requests` holds all of it.
 * * A FIELD IGNORED FAILS THE RUN. Every request with a query is asked once
 *   more per field with that field garbled; the server's own evidence
 *   (`$lib/speed.js` `ignoredFields`) decides.
 * * EVERY ROW IS STAMPED. A row's commit is the `x-brutex-build` header of
 *   the answers it timed; a row answered by two builds (a deploy mid-run) is
 *   refused, and an unstamped server fails the run unless `--allow-unstamped`.
 *
 * # What is timed
 *
 * For each request: `cold` (n requests, one client, a new connection each),
 * `warm` (n requests over one kept-alive connection a client, at each client
 * count of `--clients`), `warm+writer` (the same while `--writer`'s command
 * runs: an import or a pull writing), and `first after restart` (the first
 * request after each of `--restarts` runs of `--restart`'s command). Each is
 * p50, p99 and the max, nearest rank, with n, and every answer's class
 * (answered, refused, busy, failed, no answer). A section whose command was
 * not given is printed NOT MEASURED, never left out.
 *
 * # Usage (the deploy step runs this after every deploy)
 *
 *   node web/ci/speed-sheet.mjs --base http://127.0.0.1:8080 \
 *     [--n 200] [--clients 1,10,60] [--writer '<command>'] \
 *     [--restart '<command>' --restarts 20] [--from-pages <capture.json>] \
 *     [--feed <wire>] [--instrument NSE-INDEX-NIFTY] [--timeframe <rung>] \
 *     [--month YYYY-MM] [--day YYYY-MM-DD] [--out <sheet.md>] [--allow-unstamped] \
 *     [--timeout-ms 60000] [--up-wait-ms 120000]
 *
 * Exit 0 when every row was answered, stamped by one build and no field was
 * ignored; 1 with every reason printed otherwise; 2 for a misused command.
 * It only reads: GET and HEAD, never a write, whatever a capture holds.
 */
import http from 'node:http';
import https from 'node:https';
import { spawn } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

import { windowQuery } from '../src/lib/db/bars-plan.js';
import { dayflagsUrl } from '../src/lib/db/dayflags.js';
import { provenanceUrl } from '../src/lib/db/notes-labels.js';
import { scrubUrl } from '../src/lib/db/scrub.js';
import { calendarUrl } from '../src/lib/calendar-owed.js';
import { facetsUrl } from '../src/lib/store-census.js';
import { rungsUrl, dayUrl, barsUrl, rungSeconds } from '../src/lib/sources.js';
import { UP_ROUTE, UP_METHOD } from '../src/lib/api-status.js';
import { SPEED_ROUTE, summary, ignoredFields, garbledUrl } from '../src/lib/speed.js';
import { BUILD_HEADER } from '../src/lib/runtime-inspection.js';

/**
 * @typedef {{feed: string, instrument: string, timeframe: string, month: string, day: string,
 *   from: string, to: string}} SheetContext the selection every request is built for: the
 *   one /db opens on (NIFTY · Spot · the feed's finest timeframe · newest first)
 * @typedef {{page: string, route: string, label?: string, method?: string, builder: Function | null,
 *   from: string | null, usedBy: string, make: (c: SheetContext) => string | null}} CatalogueEntry
 */

/** One month's first and last IST day. @param {string} month */
export function monthDays(month) {
  const [y, m] = month.split('-').map(Number);
  const last = new Date(Date.UTC(y, m, 0)).getUTCDate();
  return { from: `${month}-01`, to: `${month}-${String(last).padStart(2, '0')}` };
}

/** The `/db` series plan for the context: one series, one rung, one month. @param {SheetContext} c */
const plan = (c) => ({ keys: [c.instrument], timeframe: c.timeframe, from: c.month, to: c.month });

/**
 * Every request the sheet times that a page's own library builds, with the
 * builder, the file it comes from and the page file that calls it. A route
 * with no query (`/feeds.json`) has no builder and is named bare.
 *
 * @type {CatalogueEntry[]}
 */
const ENTRIES = [
  { page: '(every page)', route: UP_ROUTE, label: 'header-only up check', method: UP_METHOD, builder: null, from: null, usedBy: 'src/lib/api-status.js', make: () => UP_ROUTE },
  { page: '(every page)', route: '/feeds.json', builder: null, from: null, usedBy: 'src/lib/feed-startup.js', make: () => '/feeds.json' },
  { page: '/db', route: '/census/facets.json', builder: facetsUrl, from: 'src/lib/store-census.js', usedBy: 'src/lib/store-census.js', make: (c) => facetsUrl({ feed: c.feed }) },
  { page: '/db', route: '/bars/window.json', label: 'the grid, newest first', builder: windowQuery, from: 'src/lib/db/bars-plan.js', usedBy: 'src/routes/db/DbBarsGrid.svelte', make: (c) => `/bars/window.json?${windowQuery(c.feed, plan(c), { sort: 'ts', desc: true, offset: 0 })}` },
  { page: '/db', route: '/bars/window.json', label: 'extremes', builder: windowQuery, from: 'src/lib/db/bars-plan.js', usedBy: 'src/routes/db/DbExtremes.svelte', make: (c) => `/bars/window.json?${windowQuery(c.feed, plan(c), { sort: 'ts', desc: true, offset: 0, limit: 1, extremes: true })}` },
  { page: '/db', route: '/dayflags.json', builder: dayflagsUrl, from: 'src/lib/db/dayflags.js', usedBy: 'src/routes/db/DbDayFlags.svelte', make: (c) => dayflagsUrl(c.feed, c.instrument, c.from, c.to) },
  { page: '/db', route: '/calendar.json', builder: calendarUrl, from: 'src/lib/calendar-owed.js', usedBy: 'src/routes/db/DbCalendar.svelte', make: (c) => calendarUrl(c.feed, c.timeframe, null) },
  { page: '/db', route: '/provenance.json', builder: provenanceUrl, from: 'src/lib/db/notes-labels.js', usedBy: 'src/routes/db/DbProvenance.svelte', make: (c) => provenanceUrl({ feed: c.feed, instrument: c.instrument, timeframe: c.timeframe, month: c.month }) },
  { page: '/db', route: '/scrub.json', builder: scrubUrl, from: 'src/lib/db/scrub.js', usedBy: 'src/routes/db/DbScrubNote.svelte', make: (c) => scrubUrl(c.feed, plan(c)) },
  { page: '/db', route: '/aliases.json', builder: null, from: null, usedBy: 'src/routes/db/DbSearch.svelte', make: () => '/aliases.json' },
  { page: '/sources', route: '/sources.json', builder: rungsUrl, from: 'src/lib/sources.js', usedBy: 'src/routes/sources/+page.svelte', make: (c) => rungsUrl(c.instrument) },
  { page: '/sources', route: '/sources/day.json', builder: dayUrl, from: 'src/lib/sources.js', usedBy: 'src/routes/sources/+page.svelte', make: (c) => dayUrl(c.instrument, c.month) },
  { page: '/sources', route: '/sources/bars.json', builder: barsUrl, from: 'src/lib/sources.js', usedBy: 'src/routes/sources/+page.svelte', make: (c) => barsUrl({ key: c.instrument, rung: c.timeframe, day: c.day }) },
  { page: '/speed', route: SPEED_ROUTE, builder: null, from: null, usedBy: 'src/routes/speed/+page.svelte', make: () => SPEED_ROUTE }
];

/** {@link ENTRIES}, frozen: the catalogue the run and the test read. */
export const CATALOGUE = Object.freeze(ENTRIES.map((e) => Object.freeze(e)));

/**
 * @typedef {{page: string, route: string, label: string, method: string, url: string,
 *   builder: string}} SheetRequest
 */

/**
 * The requests for a context: every catalogue line, built by its own
 * builder, then every request a capture holds. A builder that answers `null`
 * for this context (nothing to ask) is said in `skipped`, never dropped in
 * silence.
 *
 * @param {SheetContext} ctx
 * @param {{page: string, method: string, url: string}[]} [captured]
 * @returns {{requests: SheetRequest[], skipped: string[]}}
 */
export function requestsFor(ctx, captured = []) {
  /** @type {SheetRequest[]} */
  const requests = [];
  /** @type {string[]} */
  const skipped = [];
  for (const e of CATALOGUE) {
    const url = e.make(ctx);
    const label = e.label ?? '';
    if (url === null) {
      // Only a builder answers null: a bare route is always asked.
      skipped.push(`${e.page} ${e.route}: ${/** @type {Function} */ (e.builder).name} built no request for this selection`);
      continue;
    }
    requests.push({ page: e.page, route: e.route, label, method: e.method ?? 'GET', url, builder: e.builder?.name ?? '(no query)' });
  }
  const seen = new Set(requests.map((r) => `${r.method} ${r.url}`));
  for (const c of captured) {
    const key = `${c.method} ${c.url}`;
    if (seen.has(key)) continue;
    seen.add(key);
    requests.push({ page: c.page, route: new URL(c.url, 'http://speed.sheet').pathname, label: 'captured from the page', method: c.method, url: c.url, builder: '(the page, captured)' });
  }
  return { requests, skipped };
}

/**
 * A capture the browser harness wrote, read and checked: same-origin paths,
 * GET and HEAD only. A write in a capture is refused by name — the sheet
 * never replays one.
 *
 * @param {string} text
 * @returns {{page: string, method: string, url: string}[]}
 */
export function readCapture(text) {
  const body = JSON.parse(text);
  if (!body || !Array.isArray(body.requests)) throw new Error('the capture holds no requests list');
  return body.requests.map((/** @type {any} */ r, /** @type {number} */ i) => {
    const method = String(r?.method ?? '').toUpperCase();
    const url = String(r?.url ?? '');
    if (method !== 'GET' && method !== 'HEAD') throw new Error(`capture request ${i} is a ${method || 'method-less'} ${url}: the sheet replays reads only`);
    if (!url.startsWith('/') || url.startsWith('//')) throw new Error(`capture request ${i} (${url}) is not a path of this server`);
    return { page: String(r?.page ?? '?'), method, url };
  });
}

/**
 * The command line, read: every value checked, a misuse named.
 *
 * @param {string[]} argv
 */
export function parseArgs(argv) {
  /** @type {Record<string, string | boolean>} */
  const o = {};
  const flags = new Set(['allow-unstamped', 'help']);
  const valued = new Set(['base', 'n', 'clients', 'writer', 'restart', 'restarts', 'from-pages', 'feed', 'instrument', 'timeframe', 'month', 'day', 'out', 'timeout-ms', 'up-wait-ms']);
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (!a.startsWith('--')) throw new Error(`unexpected argument ${JSON.stringify(a)}`);
    const name = a.slice(2);
    if (flags.has(name)) o[name] = true;
    else if (valued.has(name)) {
      const v = argv[i + 1];
      if (v === undefined || v.startsWith('--')) throw new Error(`--${name} needs a value`);
      o[name] = v;
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
  const clients = String(o.clients ?? '1,10,60').split(',').map(Number);
  if (clients.some((c) => !Number.isSafeInteger(c) || c < 1 || c > 512)) throw new Error('--clients is a comma list of whole numbers from 1 to 512');
  const day = o.day === undefined ? null : String(o.day);
  if (day !== null && !/^\d{4}-\d{2}-\d{2}$/.test(day)) throw new Error('--day is YYYY-MM-DD');
  const month = o.month === undefined ? null : String(o.month);
  if (month !== null && !/^\d{4}-\d{2}$/.test(month)) throw new Error('--month is YYYY-MM');
  return {
    base,
    n: whole('n', 200, 1),
    clients,
    writer: o.writer === undefined ? null : String(o.writer),
    restart: o.restart === undefined ? null : String(o.restart),
    restarts: whole('restarts', 20, 1),
    fromPages: o['from-pages'] === undefined ? null : String(o['from-pages']),
    feed: o.feed === undefined ? null : String(o.feed),
    instrument: String(o.instrument ?? 'NSE-INDEX-NIFTY'),
    timeframe: o.timeframe === undefined ? null : String(o.timeframe),
    month,
    day,
    out: o.out === undefined ? null : String(o.out),
    allowUnstamped: o['allow-unstamped'] === true,
    timeoutMs: whole('timeout-ms', 60_000, 1),
    upWaitMs: whole('up-wait-ms', 120_000, 1)
  };
}

/**
 * @typedef {{status: number, headers: Record<string, string | string[] | undefined>, body: string,
 *   ms: number, bytes: number, build: string | null}} Answer
 * @typedef {{status: 0, error: string, ms: number, build: null, body: '', bytes: 0, headers: {}}} NoAnswer
 */

/**
 * One request, timed from the moment it is sent to the last byte of its
 * answer. Never throws: a request that got no answer is `status: 0` with the
 * reason.
 *
 * @param {string} base
 * @param {string} path
 * @param {{method?: string, agent?: http.Agent | false, timeoutMs?: number}} [o]
 * @returns {Promise<Answer | NoAnswer>}
 */
export function timed(base, path, o = {}) {
  const url = new URL(path, base);
  const lib = url.protocol === 'https:' ? https : http;
  const t0 = process.hrtime.bigint();
  const ms = () => Number(process.hrtime.bigint() - t0) / 1e6;
  const limit = o.timeoutMs ?? 60_000;
  return new Promise((resolve) => {
    const req = lib.request(url, { method: o.method ?? 'GET', agent: o.agent ?? false, headers: { accept: 'application/json' } }, (res) => {
      /** @type {Buffer[]} */
      const parts = [];
      res.on('data', (c) => parts.push(c));
      res.on('end', () => {
        const body = Buffer.concat(parts);
        const raw = res.headers[BUILD_HEADER];
        resolve({ status: Number(res.statusCode), headers: res.headers, body: body.toString('utf8'), ms: ms(), bytes: body.length, build: typeof raw === 'string' ? raw : null });
      });
      // A body cut off mid-answer is no answer: the headers alone are not one.
      res.on('error', (e) => resolve({ status: 0, error: `the answer was cut off: ${e.message}`, ms: ms(), build: null, body: '', bytes: 0, headers: {} }));
    });
    req.setTimeout(limit, () => req.destroy(new Error(`no answer within ${limit} ms`)));
    req.on('error', (e) => resolve({ status: 0, error: e.message, ms: ms(), build: null, body: '', bytes: 0, headers: {} }));
    req.end();
  });
}

/** A warm client's connection: one, kept alive across its requests. */
export const warmAgent = () => new http.Agent({ keepAlive: true, maxSockets: 1 });

/** An answer's class, the server's five words less `cancelled` (the sheet never leaves early). @param {Answer | NoAnswer} a */
export function classOf(a) {
  if (a.status === 0) return 'no answer';
  if (a.status < 400) return 'answered';
  if (a.status === 429 || (a.status === 503 && a.headers['retry-after'] !== undefined)) return 'busy';
  if (a.status < 500) return 'refused';
  return 'failed';
}

/**
 * @typedef {{page: string, route: string, label: string, url: string, builder: string, phase: string,
 *   clients: number, n: number, p50: number, p99: number, max: number,
 *   classes: Record<string, number>, builds: string[], bytes: number, said: string | null}} SheetRow
 */

/**
 * `n` requests over `clients` concurrent clients, each sample one request;
 * `agentOf` gives each client its connection policy.
 *
 * @param {(path: string, o: object) => Promise<Answer | NoAnswer>} send
 * @param {SheetRequest} request
 * @param {{n: number, clients: number, phase: string, agentOf: () => http.Agent | false, timeoutMs: number}} o
 * @returns {Promise<SheetRow>}
 */
export async function measure(send, request, o) {
  /** @type {number[]} */
  const times = [];
  /** @type {Record<string, number>} */
  const classes = {};
  const builds = new Set();
  let bytes = 0;
  /** @type {string | null} */
  let said = null;
  let left = o.n;
  /** @type {Set<http.Agent>} */
  const agents = new Set();
  const client = async () => {
    const agent = o.agentOf();
    if (agent) agents.add(agent);
    while (left > 0) {
      left -= 1;
      const a = await send(request.url, { method: request.method, agent, timeoutMs: o.timeoutMs });
      times.push(a.ms);
      const k = classOf(a);
      classes[k] = (classes[k] ?? 0) + 1;
      if (a.build !== null) builds.add(a.build);
      bytes = Math.max(bytes, a.bytes);
      if (said === null && k !== 'answered') said = 'error' in a ? a.error : `HTTP ${a.status}: ${a.body.slice(0, 300)}`;
    }
  };
  await Promise.all(Array.from({ length: Math.min(o.clients, o.n) }, client));
  // RELEASED ONLY ONCE EVERY CLIENT IS DONE: an agent destroyed when its
  // first client finished hung up on a request another client still had on
  // it, and the sheet counted its own hang-up as the server's "no answer".
  for (const agent of agents) agent.destroy();
  const s = summary(times);
  return { page: request.page, route: request.route, label: request.label, url: request.url, builder: request.builder, phase: o.phase, clients: o.clients, n: s.n, p50: s.p50, p99: s.p99, max: s.max, classes, builds: [...builds], bytes, said };
}

/**
 * The context, from the server, the way /db opens: the feed `/feeds.json`
 * offers first (or `--feed`), the instrument (default NIFTY spot), and the
 * newest month and day of the feed's finest stored rung from `/sources.json`
 * — asked with the All sources page's own `rungsUrl`. Any of them can be
 * given instead. Refuses, naming what is missing, rather than guessing.
 *
 * @param {(path: string) => Promise<Answer | NoAnswer>} get
 * @param {ReturnType<typeof parseArgs>} args
 * @returns {Promise<SheetContext>}
 */
export async function discover(get, args) {
  /** @param {string} path */
  const json = async (path) => {
    const a = await get(path);
    if (a.status !== 200) throw new Error(`discovering the selection: ${path} answered ${'error' in a ? a.error : `HTTP ${a.status}`}`);
    return JSON.parse(a.body);
  };
  let feed = args.feed;
  if (feed === null) {
    const rows = await json('/feeds.json');
    const row = Array.isArray(rows) ? (rows.find((r) => r?.ready === true && r?.offered !== false) ?? rows[0]) : null;
    if (!row?.wire) throw new Error('discovering the selection: /feeds.json named no feed; pass --feed');
    feed = String(row.wire);
  }
  let { timeframe, month, day } = args;
  if (timeframe === null || month === null || day === null) {
    const url = rungsUrl(args.instrument);
    if (url === null) throw new Error(`discovering the selection: ${args.instrument} is not an instrument key rungsUrl can ask about`);
    const body = await json(url);
    const held = (Array.isArray(body?.rungs) ? body.rungs : [])
      .filter((/** @type {any} */ r) => r?.feed === feed && r?.stored === true && r?.last_month && (timeframe === null || r.rung === timeframe))
      .sort((/** @type {any} */ a, /** @type {any} */ b) => rungSeconds(a.rung) - rungSeconds(b.rung));
    const finest = held[0];
    if (!finest) throw new Error(`discovering the selection: ${url} holds no stored rung of ${feed} for ${args.instrument}; pass --timeframe, --month and --day`);
    timeframe = timeframe ?? String(finest.rung);
    month = month ?? String(finest.last_month);
    day = day ?? String(finest.last_day ?? `${month}-01`);
  }
  return { feed, instrument: args.instrument, timeframe, month, day, ...monthDays(month) };
}

/**
 * Every field of every request with a query, asked once garbled; the
 * fields the server's own answers show were ignored.
 *
 * @param {(path: string, o?: object) => Promise<Answer | NoAnswer>} send
 * @param {SheetRequest[]} requests
 */
export async function fieldCheck(send, requests) {
  /** @type {{request: SheetRequest, field: string, how: string, said: string}[]} */
  const out = [];
  for (const r of requests) {
    if (!r.url.includes('?')) continue;
    const first = await send(r.url, { method: 'GET' });
    if (first.status === 0) continue; // timed below, and failed there by name
    /** @type {Record<string, {status: number, body: string}>} */
    const garbled = {};
    for (const field of new Set(new URL(r.url, 'http://speed.sheet').searchParams.keys())) {
      const probe = await send(garbledUrl(r.url, field), { method: 'GET' });
      garbled[field] = { status: probe.status, body: probe.body };
    }
    for (const f of ignoredFields({ url: r.url, status: first.status, body: first.body, garbled })) {
      out.push({ request: r, ...f });
    }
  }
  return out;
}

/**
 * Runs `command` through the shell. `wait` resolves with its exit code.
 *
 * @param {string} command
 */
function run(command) {
  const child = spawn(command, { shell: true, stdio: ['ignore', 'inherit', 'inherit'] });
  const wait = new Promise((resolve) => child.on('exit', (code) => resolve(code ?? 1)));
  return { child, wait };
}

/**
 * The whole run: discover, check every field, time every phase, and say
 * every failure. `send` is the network, injected so a test drives the run
 * against a stand-in server.
 *
 * @typedef {{ok: boolean, rows: SheetRow[], ignored: {request: SheetRequest, field: string, how: string, said: string}[],
 *   failures: string[], notMeasured: string[], skipped: string[], ctx: SheetContext | null,
 *   commit: string | null}} SheetResult
 *
 * @param {ReturnType<typeof parseArgs>} args
 * @param {{send?: typeof timed, log?: (line: string) => void, capture?: string | null,
 *   restart?: (command: string) => Promise<unknown>}} [deps] `restart` runs one restart and
 *   resolves with its exit code (the shell command, by default)
 * @returns {Promise<SheetResult>}
 */
export async function runSheet(args, deps = {}) {
  const sendTo = deps.send ?? timed;
  const restartOnce = deps.restart ?? ((/** @type {string} */ command) => run(command).wait);
  const log = deps.log ?? ((line) => console.error(line));
  /** @param {string} path @param {object} [o] */
  const send = (path, o = {}) => sendTo(args.base, path, { timeoutMs: args.timeoutMs, ...o });
  /** @type {string[]} */
  const failures = [];
  /** @type {string[]} */
  const notMeasured = [];

  const up = await send(UP_ROUTE, { method: UP_METHOD });
  if (up.status !== 200) {
    return { ok: false, rows: [], ignored: [], failures: [`the server at ${args.base} is not up: ${UP_METHOD} ${UP_ROUTE} answered ${'error' in up ? up.error : `HTTP ${up.status}`}`], notMeasured, skipped: [], ctx: null, commit: null };
  }
  const commit = up.build;
  if (commit === null || commit === 'unstamped') {
    const why = `the server names ${commit === null ? 'no build' : 'an unstamped build'} (${BUILD_HEADER}); a sheet that cannot name its commit measures no build`;
    if (args.allowUnstamped) log(`WARNING: ${why} — allowed by --allow-unstamped`);
    else failures.push(why);
  }
  /** @type {SheetContext} */
  let ctx;
  try {
    ctx = await discover((p) => send(p), args);
  } catch (why) {
    return { ok: false, rows: [], ignored: [], failures: [/** @type {Error} */ (why).message], notMeasured, skipped: [], ctx: null, commit };
  }
  const captureText = deps.capture ?? (args.fromPages ? readFileSync(args.fromPages, 'utf8') : null);
  /** @type {{page: string, method: string, url: string}[]} */
  let captured = [];
  try {
    captured = captureText === null ? [] : readCapture(captureText);
  } catch (why) {
    // A CAPTURE THAT CANNOT BE READ IS A FAILED RUN, NAMED: replaying part
    // of it, or none of it in silence, would time fewer pages than asked.
    return { ok: false, rows: [], ignored: [], failures: [`the capture of the pages' own requests could not be read: ${/** @type {Error} */ (why).message}`], notMeasured, skipped: [], ctx, commit };
  }
  const { requests, skipped } = requestsFor(ctx, captured);
  if (captureText === null) notMeasured.push('the requests /gaps, /audit, /markets and the other pages build inline: no --from-pages capture was given');
  log(`speed sheet: ${requests.length} requests for ${ctx.feed} ${ctx.instrument} ${ctx.timeframe} ${ctx.month}, build ${commit ?? 'not named'}`);

  const ignored = await fieldCheck(send, requests);
  for (const i of ignored) failures.push(`${i.request.page} ${i.request.url}: field ${i.field} was ignored (${i.how}: ${i.said})`);

  /** @type {SheetRow[]} */
  const rows = [];
  for (const r of requests) rows.push(await measure(send, r, { n: args.n, clients: 1, phase: 'cold', agentOf: () => false, timeoutMs: args.timeoutMs }));
  for (const clients of args.clients) {
    for (const r of requests) rows.push(await measure(send, r, { n: args.n, clients, phase: 'warm', agentOf: warmAgent, timeoutMs: args.timeoutMs }));
  }
  if (args.writer === null) notMeasured.push('warm+writer: no --writer command was given (an import or a pull writing)');
  else {
    const writer = run(args.writer);
    let exited = false;
    void writer.wait.then(() => (exited = true));
    for (const clients of [Math.min(...args.clients), Math.max(...args.clients)]) {
      for (const r of requests) rows.push(await measure(send, r, { n: args.n, clients, phase: 'warm+writer', agentOf: warmAgent, timeoutMs: args.timeoutMs }));
    }
    if (exited) failures.push('the --writer command ended before the warm+writer pass did: part of that pass ran with nothing writing');
    writer.child.kill('SIGTERM');
  }
  if (args.restart === null) notMeasured.push(`first after restart: no --restart command was given (${args.restarts} restarts asked)`);
  else {
    /** @type {Map<string, number[]>} */
    const firsts = new Map();
    /** @type {Map<string, Record<string, number>>} each request's answers after the restarts, by class */
    const firstClasses = new Map();
    for (let i = 0; i < args.restarts; i += 1) {
      const code = await restartOnce(args.restart);
      if (code !== 0) {
        failures.push(`restart ${i + 1}: the --restart command exited ${code}`);
        break;
      }
      const deadline = Date.now() + args.upWaitMs;
      while ((await send(UP_ROUTE, { method: UP_METHOD })).status !== 200) {
        if (Date.now() > deadline) {
          failures.push(`restart ${i + 1}: the server was not up ${args.upWaitMs} ms after the restart`);
          break;
        }
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      for (const r of requests) {
        const a = await send(r.url, { method: r.method });
        const key = `${r.method} ${r.url}`;
        firsts.set(key, [...(firsts.get(key) ?? []), a.ms]);
        const counted = firstClasses.get(key) ?? {};
        counted[classOf(a)] = (counted[classOf(a)] ?? 0) + 1;
        firstClasses.set(key, counted);
        if (classOf(a) !== 'answered') failures.push(`restart ${i + 1}: the first ${r.url} answered ${'error' in a ? a.error : `HTTP ${a.status}`}`);
      }
    }
    for (const r of requests) {
      const key = `${r.method} ${r.url}`;
      const s = summary(firsts.get(key) ?? []);
      // The classes are counted here as in every other row, so a restart row
      // never prints "0 answered" for twenty answers.
      rows.push({ page: r.page, route: r.route, label: r.label, url: r.url, builder: r.builder, phase: 'first after restart', clients: 1, n: s.n, p50: s.p50, p99: s.p99, max: s.max, classes: firstClasses.get(key) ?? {}, builds: commit ? [commit] : [], bytes: 0, said: null });
    }
  }
  for (const row of rows) {
    if (row.builds.length > 1) failures.push(`${row.url} (${row.phase}, ${row.clients} clients) was answered by ${row.builds.length} builds: ${row.builds.join(', ')}`);
    else if (row.phase !== 'first after restart' && commit !== null && row.builds[0] !== commit) failures.push(`${row.url} (${row.phase}) was answered by ${row.builds[0] ?? 'no named build'}, not ${commit}`);
    // A restart row's answers were each failed by name, restart by restart, above.
    if (row.phase === 'first after restart') continue;
    for (const k of ['failed', 'no answer', 'refused']) {
      if ((row.classes[k] ?? 0) > 0) failures.push(`${row.page} ${row.url} (${row.phase}, ${row.clients} clients): ${row.classes[k]} ${k} — ${row.said}`);
    }
  }
  return { ok: failures.length === 0, rows, ignored, failures, notMeasured, skipped, ctx, commit };
}

/** @param {number} ms */
const msText = (ms) => (ms < 10 ? ms.toFixed(2) : ms.toFixed(1));

/**
 * The sheet as Markdown: every row stamped with its commit, then every
 * section not measured and every failure.
 *
 * @param {SheetResult} result
 * @param {{n: number, clients: number[]}} args
 */
export function renderSheet(result, args) {
  const lines = [
    `# Speed sheet — ${result.ok ? 'PASS' : 'FAIL'}`,
    '',
    `Build ${result.commit ?? 'not named'}; selection ${result.ctx ? `${result.ctx.feed} ${result.ctx.instrument} ${result.ctx.timeframe} ${result.ctx.month} (${result.ctx.day})` : 'not discovered'}; n = ${args.n} a row, clients ${args.clients.join(', ')}. Times are milliseconds from the request sent to the last byte, nearest rank.`,
    '',
    '| commit | page | route | request | phase | clients | n | p50 ms | p99 ms | max ms | answered | refused | busy | failed | no answer | bytes |',
    '|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|'
  ];
  for (const r of result.rows) {
    const c = (/** @type {string} */ k) => String(r.classes[k] ?? 0);
    const commit = r.builds.length === 1 ? r.builds[0].slice(0, 12) : r.builds.length === 0 ? 'not named' : 'MIXED';
    lines.push(`| ${commit} | ${r.page} | \`${r.route}\` | ${r.label || r.builder} | ${r.phase} | ${r.clients} | ${r.n} | ${msText(r.p50)} | ${msText(r.p99)} | ${msText(r.max)} | ${c('answered')} | ${c('refused')} | ${c('busy')} | ${c('failed')} | ${c('no answer')} | ${r.bytes} |`);
  }
  lines.push('');
  for (const s of result.skipped) lines.push(`- SKIPPED: ${s}`);
  for (const s of result.notMeasured) lines.push(`- NOT MEASURED: ${s}`);
  for (const f of result.failures) lines.push(`- FAILED: ${f}`);
  return `${lines.join('\n')}\n`;
}

async function main() {
  let args;
  try {
    args = parseArgs(process.argv.slice(2));
  } catch (why) {
    console.error(`speed-sheet: ${/** @type {Error} */ (why).message}`);
    process.exit(2);
  }
  const result = await runSheet(args);
  const text = renderSheet(result, args);
  process.stdout.write(text);
  if (args.out) writeFileSync(args.out, text);
  process.exit(result.ok ? 0 : 1);
}

if (import.meta.url === pathToFileURL(String(process.argv[1])).href) await main();

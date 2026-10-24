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
 *     [--timeout-ms 60000] [--up-wait-ms 120000] [--restart-wait-ms <the up wait>] \
 *     [--max-answer-bytes 16000000]
 *
 * # Every wait is bounded, and each bound is said (verify r5, 23 Oct)
 *
 * The bound on a request was the socket's idle time, so an answer that sent
 * one byte every 500 ms was never ended, and the `--restart` command was
 * waited on until it exited: either way the deploy step's last command hung
 * with nothing printed. Now a request waits at most `--timeout-ms` from its
 * send to the last byte of its answer — the WHOLE answer — and is then ended
 * and counted `no answer`, its reason saying how much had come. A request
 * that ran that whole bound is asked no more this run: the rest of its row,
 * and each later pass of it, is a NOT MEASURED line by name, because each ask
 * would wait as long and a warm row's kept connection ended with it (the
 * harness's rule for a load that does not end). The `--restart` command may
 * run `--restart-wait-ms` (by default `--up-wait-ms`: one budget for the
 * server to come back); past it its process group is sent SIGTERM, and
 * SIGKILL {@link COMMAND_STOP_MS} later, and the run fails by name. The up
 * check after a restart waits `--up-wait-ms` in all, each check within it at
 * most the time left; a server not up ends the restart pass. A command the
 * sheet started is ended with the sheet if the sheet ends first. So a run
 * ends, and docs/06-limits.md (L-R315-24) states the sum it ends within.
 *
 * # Two bounds, each said where it applies (found on the Mac, 20 Oct)
 *
 * `/gaps`, `/ingest` and `/markets` each ask a 184 MB census on their first
 * load, and the page capture replays it. Sixty clients at once would hold
 * 60 × 184 MB in this process, so a request whose cold answer is past
 * `--max-answer-bytes` is timed warm at ONE client only; every client count
 * it skips is a NOT MEASURED line naming the bytes. And an answer past
 * {@link MAX_TEXT_BYTES} is never made a string (a 591 MB `/store.json` is past
 * what one string may hold): it is kept as its length and its sha256, so the
 * ignored-field check still compares it exactly.
 *
 * Exit 0 when every row was answered, stamped by one build and no field was
 * ignored; 1 with every reason printed otherwise; 2 for a misused command.
 * It only reads: GET and HEAD, never a write, whatever a capture holds.
 */
import http from 'node:http';
import https from 'node:https';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { closeSync, constants, fstatSync, fsyncSync, openSync, readFileSync, renameSync, unlinkSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, resolve as resolvePath } from 'node:path';
import { pathToFileURL } from 'node:url';

import { windowQuery } from '../src/lib/db/bars-plan.js';
import { dayflagsUrl } from '../src/lib/db/dayflags.js';
import { provenanceUrl } from '../src/lib/db/notes-labels.js';
import { scrubUrl } from '../src/lib/db/scrub.js';
import { calendarUrl } from '../src/lib/calendar-owed.js';
import { facetsUrl } from '../src/lib/store-census.js';
import { rungsUrl, dayUrl, barsUrl, rungSeconds } from '../src/lib/sources.js';
import { UP_ROUTE, UP_METHOD } from '../src/lib/api-status.js';
import { SPEED_ROUTE, MEASURED_PAGES, summary, ignoredFields, garbledUrl, routePath } from '../src/lib/speed.js';
import { BUILD_HEADER } from '../src/lib/runtime-inspection.js';

/**
 * A page's path as the app's router reads it (`routePath`), so `/db` and a
 * capture's `/db/` or `/db?seg=options` are one page; anything that is not a
 * path is kept as written.
 *
 * @param {string} page
 */
export function pagePathOf(page) {
  if (!page.startsWith('/')) return page;
  const u = new URL(`http://h${page}`);
  return routePath(u.pathname) ?? u.pathname;
}

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
 * Whether `path`, asked of `base`, stays on `base`'s own origin. A URL
 * reader treats a backslash as a slash and drops a tab or a line break, so
 * `/\evil.invalid/x.json` is `//evil.invalid/x.json`, another host (verify r4
 * follow-up, 23 Oct): only the resolved origin can say.
 *
 * @param {string} path
 * @param {string} base an origin
 */
export function onOrigin(path, base) {
  try {
    return new URL(path, base).origin === new URL(base).origin;
  } catch {
    return false;
  }
}

/**
 * A capture the browser harness wrote, read and checked: same-origin paths,
 * GET and HEAD only. A write in a capture is refused by name — the sheet
 * never replays one — and so is a path that a URL reader would take to
 * another host ({@link onOrigin}).
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
    if (!url.startsWith('/') || url.startsWith('//') || url.includes('\\') || !onOrigin(url, 'http://speed.sheet')) throw new Error(`capture request ${i} (${url}) is not a path of this server`);
    return { page: String(r?.page ?? '?'), method, url };
  });
}

/**
 * The longest wait a Node timer holds: 2,147,483,647 ms (2^31 - 1, about
 * 24.8 days). Past it Node fires a `setTimeout` after 1 ms and cuts a
 * socket's timeout to it, each with only a `TimeoutOverflowWarning` on
 * stderr (measured 23 Oct, Node 24.1: a request timed with a `--timeout-ms`
 * of 2,147,483,648 ran under 2,147,483,647). Either way the wait run is not
 * the one asked, so a wait option past it is refused by name: the harness's
 * `--idle-ms` past it printed an idle never measured (verify r3, 22 Oct).
 * The harness reads its ceiling from here.
 */
export const MAX_WAIT_MS = 2_147_483_647;

/**
 * The most timed requests of a row (`--n`) or restarts (`--restarts`) one
 * run takes: fifty times the n = 200 the targets are measured at, the same
 * ceiling as the harness's `MAX_N`. It bounds an argument and chooses no
 * sample size (verify r3, 22 Oct: `--restarts` had no ceiling).
 */
export const MAX_N = 10_000;

/**
 * A whole number as the command line may write one: decimal digits, an
 * exponent allowed (`1e3`). `Number` alone also reads `0x10` as 16, `0b11`
 * as 3 and `' 12 '` as 12 (verify r3, 22 Oct), so those are refused.
 */
export const DECIMAL = /^[0-9]+(?:[eE][0-9]+)?$/;

/**
 * The command line, read: every value checked, a misuse named.
 *
 * @param {string[]} argv
 */
export function parseArgs(argv) {
  /** @type {Record<string, string | boolean>} */
  const o = {};
  const flags = new Set(['allow-unstamped', 'help']);
  const valued = new Set(['base', 'n', 'clients', 'writer', 'restart', 'restarts', 'from-pages', 'feed', 'instrument', 'timeframe', 'month', 'day', 'out', 'timeout-ms', 'up-wait-ms', 'restart-wait-ms', 'max-answer-bytes']);
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
  const whole = (/** @type {string} */ name, /** @type {number} */ dflt, /** @type {number} */ min, max = Number.MAX_SAFE_INTEGER) => {
    const v = o[name] === undefined ? dflt : DECIMAL.test(String(o[name])) ? Number(o[name]) : Number.NaN;
    if (!Number.isSafeInteger(v) || v < min) throw new Error(`--${name} must be a whole number of at least ${min}, in decimal digits`);
    if (v > max) throw new Error(`--${name} must be at most ${max}`);
    return v;
  };
  const clients = String(o.clients ?? '1,10,60').split(',').map((c) => (DECIMAL.test(c) ? Number(c) : Number.NaN));
  if (clients.some((c) => !Number.isSafeInteger(c) || c < 1 || c > 512)) throw new Error('--clients is a comma list of whole numbers from 1 to 512');
  // One count, one pass (verify r4 follow-up: `10,10` timed every request twice at 10).
  const twice = clients.find((c, i) => clients.indexOf(c) !== i);
  if (twice !== undefined) throw new Error(`--clients names ${twice} twice: each client count is measured once a run`);
  const day = o.day === undefined ? null : String(o.day);
  if (day !== null && !/^\d{4}-\d{2}-\d{2}$/.test(day)) throw new Error('--day is YYYY-MM-DD');
  const month = o.month === undefined ? null : String(o.month);
  if (month !== null && !/^\d{4}-\d{2}$/.test(month)) throw new Error('--month is YYYY-MM');
  // ONE FILE IS NOT BOTH (verify r5 follow-up, 23 Oct: the harness wrote its
  // sheet over its own capture and exited 0): the sheet read here would be
  // replaced by the one written at the end.
  if (o.out !== undefined && o['from-pages'] !== undefined && resolvePath(String(o.out)) === resolvePath(String(o['from-pages']))) {
    throw new Error(`--out and --from-pages name one file (${String(o.out)}): the sheet would be written over the capture it reads`);
  }
  return {
    base,
    n: whole('n', 200, 1, MAX_N),
    clients,
    writer: o.writer === undefined ? null : String(o.writer),
    restart: o.restart === undefined ? null : String(o.restart),
    restarts: whole('restarts', 20, 1, MAX_N),
    fromPages: o['from-pages'] === undefined ? null : String(o['from-pages']),
    feed: o.feed === undefined ? null : String(o.feed),
    instrument: String(o.instrument ?? 'NSE-INDEX-NIFTY'),
    timeframe: o.timeframe === undefined ? null : String(o.timeframe),
    month,
    day,
    out: o.out === undefined ? null : String(o.out),
    allowUnstamped: o['allow-unstamped'] === true,
    timeoutMs: whole('timeout-ms', 60_000, 1, MAX_WAIT_MS),
    upWaitMs: whole('up-wait-ms', 120_000, 1, MAX_WAIT_MS),
    // How long one run of the --restart command may take before its process
    // group is ended (verify r5, 23 Oct: it was waited on with no bound). Not
    // given, it is the up wait's: one budget for the server to come back.
    restartWaitMs: whole('restart-wait-ms', whole('up-wait-ms', 120_000, 1, MAX_WAIT_MS), 1, MAX_WAIT_MS),
    // At most the largest answer kept as text: past it an answer is a digest,
    // and no answer that large is timed by many clients at once (verify r4
    // follow-up: 9,007,199,254,740,991 was taken).
    maxAnswerBytes: whole('max-answer-bytes', MAX_ANSWER_BYTES, 1, MAX_TEXT_BYTES)
  };
}

/**
 * @typedef {{status: number, headers: Record<string, string | string[] | undefined>, body: string,
 *   ms: number, bytes: number, build: string | null}} Answer
 * @typedef {{status: 0, error: string, ms: number, build: string | null, body: '', bytes: 0, headers: {}, timedOut?: true}} NoAnswer
 *   `timedOut` when its whole answer did not come within its bound ({@link timed}); `build` the
 *   build its headers named, when they came before it was cut
 */

/**
 * The largest answer kept as text, bytes. A longer one is kept as
 * `(an answer of N bytes, sha256 H)`: exact for comparing two answers, and
 * never a string longer than one may be.
 */
export const MAX_TEXT_BYTES = 64_000_000;

/** The largest answer timed by more than one client at once, bytes (`--max-answer-bytes`). */
export const MAX_ANSWER_BYTES = 16_000_000;

/**
 * An answer's bytes as the sheet keeps them: the text, or past `limit`
 * its length and digest.
 *
 * @param {Buffer} body
 * @param {number} [limit]
 */
export function answerText(body, limit = MAX_TEXT_BYTES) {
  return body.length <= limit ? body.toString('utf8') : `(an answer of ${body.length} bytes, sha256 ${createHash('sha256').update(body).digest('hex')})`;
}

/**
 * One request, timed from the moment it is sent to the last byte of its
 * answer. Never throws: a request that got no answer is `status: 0` with the
 * reason.
 *
 * THE WHOLE ANSWER IS BOUNDED, from the send to its last byte (verify r5,
 * 23 Oct): the only bound was the socket's idle time (`req.setTimeout`), so
 * an answer that sent one byte every 500 ms was never ended, and a sheet run
 * with `--timeout-ms 2000` was still waiting 90 s later, having printed
 * nothing. A request whose whole answer has not come within `timeoutMs` is
 * ended here and answered `timedOut`, saying how much of it had come.
 *
 * @param {string} base
 * @param {string} path
 * @param {{method?: string, agent?: http.Agent | false, timeoutMs?: number}} [o]
 * @returns {Promise<Answer | NoAnswer>}
 */
export function timed(base, path, o = {}) {
  const t0 = process.hrtime.bigint();
  const ms = () => Number(process.hrtime.bigint() - t0) / 1e6;
  // ONLY THE SERVER UNDER TEST IS ASKED: a path that resolves to another
  // origin is never sent (verify r4 follow-up, 23 Oct).
  if (!onOrigin(path, base)) return Promise.resolve({ status: 0, error: `${path} is not a path of ${base}: never sent`, ms: ms(), build: null, body: '', bytes: 0, headers: {} });
  const url = new URL(path, base);
  const lib = url.protocol === 'https:' ? https : http;
  const limit = o.timeoutMs ?? 60_000;
  return new Promise((resolve) => {
    /** @type {ReturnType<typeof setTimeout> | undefined} */
    let timer;
    let settled = false;
    /** @type {string | null | undefined} the build the headers named, once they came */
    let headedBy;
    let got = 0;
    // The first ending wins: an error the ending itself causes is dropped.
    /** @param {Answer | NoAnswer} a */
    const settle = (a) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(a);
    };
    // A cut answer whose headers came keeps the build they named: the server
    // that answered them is known, though its answer is not one.
    /** @param {string} error @param {boolean} [late] */
    const none = (error, late = false) => settle({ status: 0, error, ms: ms(), build: headedBy ?? null, body: '', bytes: 0, headers: {}, ...(late ? { timedOut: /** @type {const} */ (true) } : {}) });
    const req = lib.request(url, { method: o.method ?? 'GET', agent: o.agent ?? false, headers: { accept: 'application/json' } }, (res) => {
      const raw = res.headers[BUILD_HEADER];
      headedBy = typeof raw === 'string' ? raw : null;
      /** @type {Buffer[]} */
      const parts = [];
      res.on('data', (c) => {
        parts.push(c);
        got += c.length;
      });
      res.on('end', () => {
        const body = Buffer.concat(parts);
        settle({ status: Number(res.statusCode), headers: res.headers, body: answerText(body), ms: ms(), bytes: body.length, build: headedBy ?? null });
      });
      // A body cut off mid-answer is no answer: the headers alone are not one.
      res.on('error', (e) => none(`the answer was cut off: ${e.message}`));
    });
    timer = setTimeout(() => {
      none(`no whole answer within ${limit} ms (${headedBy !== undefined ? `its headers came, then ${got} bytes of its body` : 'not even its headers came'})`, true);
      req.destroy();
    }, limit);
    req.on('error', (e) => none(e.message));
    req.end();
  });
}

/** A warm client's connection: one, kept alive across its requests. */
export const warmAgent = () => new http.Agent({ keepAlive: true, maxSockets: 1 });

/**
 * An answer's class, the server's five words less `cancelled`: the sheet
 * leaves a request only at its whole bound ({@link timed}), and that is `no
 * answer`.
 *
 * @param {Answer | NoAnswer} a
 */
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
 *   classes: Record<string, number>, builds: string[], bytes: number, said: string | null,
 *   cut: string | null, notAsked: number}} SheetRow `cut` is the reason of the answer that ran its
 *   whole bound and ended the row; `notAsked` the requests of the row never sent after it
 */

/**
 * `n` requests over `clients` concurrent clients, each sample one request;
 * `agentOf` gives each client its connection policy.
 *
 * A REQUEST THAT RAN ITS WHOLE BOUND ENDS ITS ROW (verify r5, 23 Oct): every
 * later sample would wait as long — `n` of them — and a warm client's kept
 * connection ended with it, so what follows would not be warm. The clients
 * take no further request; the row says why (`cut`) and how many were never
 * sent (`notAsked`).
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
  /** @type {string | null} */
  let cut = null;
  let left = o.n;
  /** @type {Set<http.Agent>} */
  const agents = new Set();
  const client = async () => {
    const agent = o.agentOf();
    if (agent) agents.add(agent);
    while (left > 0 && cut === null) {
      left -= 1;
      const a = await send(request.url, { method: request.method, agent, timeoutMs: o.timeoutMs });
      times.push(a.ms);
      const k = classOf(a);
      classes[k] = (classes[k] ?? 0) + 1;
      if (a.build !== null) builds.add(a.build);
      bytes = Math.max(bytes, a.bytes);
      if (said === null && k !== 'answered') said = 'error' in a ? a.error : `HTTP ${a.status}: ${a.body.slice(0, 300)}`;
      if ('timedOut' in a && a.timedOut && cut === null) cut = a.error;
    }
  };
  await Promise.all(Array.from({ length: Math.min(o.clients, o.n) }, client));
  // RELEASED ONLY ONCE EVERY CLIENT IS DONE: an agent destroyed when its
  // first client finished hung up on a request another client still had on
  // it, and the sheet counted its own hang-up as the server's "no answer".
  for (const agent of agents) agent.destroy();
  const s = summary(times);
  return { page: request.page, route: request.route, label: request.label, url: request.url, builder: request.builder, phase: o.phase, clients: o.clients, n: s.n, p50: s.p50, p99: s.p99, max: s.max, classes, builds: [...builds], bytes, said, cut, notAsked: left };
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
 * EACH REQUEST IS ASKED AS THE PAGE ASKS IT (verify r4 follow-up, 23 Oct): a
 * HEAD request was checked with a whole GET, so the `/db` census watch's
 * `HEAD /store.json?feed=gdfl` streamed 591,200,898 B per ask into the
 * sheet's own memory (1.28 GB at its peak) against the served app. A HEAD
 * answer has no body, so of the three signals only the status can show a
 * field read: a garbled field that is refused (4xx) was read; one answered
 * 2xx like the original cannot be told from an ignored one, and is said in
 * `unchecked` (a NOT MEASURED line), never passed and never failed.
 *
 * A GARBLED ASK THE SERVER FAILED ON, OR GAVE NO ANSWER TO, tells neither a
 * read nor an ignore, and is a failure by name in `failed` (verify r5
 * follow-up, 23 Oct: a 5xx was said nowhere, neither ignored nor failed); one
 * answered busy is `unchecked`. One that ran its whole bound ends the check
 * of its request: its other fields are `unchecked`, not asked. A request
 * whose own first ask ran its whole bound is in `stalled`, asked no more this
 * run; one answered other than 2xx has no garbled asks at all, since only a
 * 2xx original can be compared (its own body is still read for a field it
 * names).
 *
 * @param {(path: string, o?: object) => Promise<Answer | NoAnswer>} send
 * @param {SheetRequest[]} requests
 * @returns {Promise<FieldCheck>}
 */
export async function fieldCheck(send, requests) {
  /** @type {FieldCheck} */
  const out = Object.assign([], { unchecked: /** @type {string[]} */ ([]), failed: /** @type {string[]} */ ([]), stalled: /** @type {{request: SheetRequest, said: string}[]} */ ([]) });
  const ok = (/** @type {number} */ s) => s >= 200 && s < 300;
  for (const r of requests) {
    if (!r.url.includes('?')) continue;
    const method = r.method === 'HEAD' ? 'HEAD' : 'GET';
    const first = await send(r.url, { method });
    if (first.status === 0) {
      // Timed below, and failed there by name; one that ran its whole bound is asked no more.
      if ('timedOut' in first && first.timedOut) out.stalled.push({ request: r, said: first.error });
      continue;
    }
    /** @type {Record<string, {status: number, body: string}>} */
    const garbled = {};
    /** @type {string[]} */
    const unasked = [];
    /** @type {string | null} */
    let cut = null;
    // Only a 2xx original can be compared with a garbled ask, so only its fields are asked.
    const fields = ok(first.status) ? new Set(new URL(r.url, 'http://speed.sheet').searchParams.keys()) : new Set();
    for (const field of fields) {
      if (cut !== null) {
        unasked.push(field);
        continue;
      }
      const probe = await send(garbledUrl(r.url, field), { method });
      const k = classOf(probe);
      if (k === 'failed' || k === 'no answer') {
        out.failed.push(`${r.page} ${method} ${garbledUrl(r.url, field)}: the field ${field}, garbled, was answered ${'error' in probe ? probe.error : `HTTP ${probe.status}`}, so whether the server reads it is not known`);
        if ('timedOut' in probe && probe.timedOut) cut = probe.error;
        continue;
      }
      if (k === 'busy') {
        out.unchecked.push(`the field ${field} of ${method} ${r.url} (${r.page}): its garbled value was answered busy (HTTP ${probe.status}), which tells neither a read nor an ignore`);
        continue;
      }
      garbled[field] = { status: probe.status, body: probe.body };
    }
    if (unasked.length > 0) out.unchecked.push(`the fields ${unasked.join(', ')} of ${method} ${r.url} (${r.page}): not asked garbled, after an earlier garbled ask of it had ${cut}`);
    if (method === 'HEAD') {
      const untold = Object.entries(garbled).filter(([, p]) => ok(first.status) && ok(p.status)).map(([field]) => field);
      if (untold.length > 0) out.unchecked.push(`the fields ${untold.join(', ')} of HEAD ${r.url} (${r.page}): each garbled value was answered ${first.status} like the original, and a HEAD answer has no body to tell a field read from one ignored`);
      continue;
    }
    for (const f of ignoredFields({ url: r.url, status: first.status, body: first.body, garbled })) {
      out.push({ request: r, ...f });
    }
  }
  return out;
}

/**
 * @typedef {{request: SheetRequest, field: string, how: string, said: string}[] & {unchecked: string[],
 *   failed: string[], stalled: {request: SheetRequest, said: string}[]}} FieldCheck
 */

/** How often a stop looks again for a process of a group still running, ms. */
export const GROUP_POLL_MS = 25;

/**
 * Whether any process of a process group is still running, asked with
 * signal 0, which signals nothing. A process started detached leads a group
 * of its own, and every process it starts is in that group unless it leaves
 * it: all ten processes of a headless Chrome on this Mac on 22 Oct (each
 * renderer, the GPU and network services, the rest), none in another group.
 * A group whose id is now someone else's cannot be signalled (EPERM) and is
 * not ours: it reads as gone. The browser harness stops its browser by this
 * rule, and the sheet its commands.
 *
 * @param {number | undefined} pid the leader's pid, which is its group's id
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
 * Sends `signal` to a detached process and every process in its group. Sent
 * only while the group is known to be that process's — the process not yet
 * reaped (its pid, so its group's id, cannot have passed to another
 * process), or {@link groupRunning} having just found the group's processes
 * still there (an id in use by a group is given to no new one). Never
 * throws: where the group cannot be signalled the process alone is, through
 * its own handle, which reaches nothing once it has been reaped — so a stop
 * never waits on a process no signal reached.
 *
 * WHY THE GROUP, AND NOT THE PROCESS ALONE (the R3-15 repair 2 run, 22 Oct,
 * at a load average near 140): a harness sent SIGTERM killed the browser,
 * removed its profile at once, and Chrome's network service, still running,
 * wrote `Default/Reporting and NEL` into the profile again, so the profile
 * was left. Probed the same day: after a SIGTERM to the browser alone, the
 * browser left in 35 ms with a renderer still running; to its group, every
 * process was gone within 300 ms. A command run through the shell is the
 * same: a SIGTERM to the shell alone leaves what it started running.
 *
 * @param {{pid?: number, kill: (signal: NodeJS.Signals | 0) => boolean}} child the process's handle
 * @param {NodeJS.Signals | 0} signal
 */
export function signalGroup(child, signal) {
  try {
    process.kill(-(/** @type {number} */ (child.pid)), signal);
  } catch {
    child.kill(signal);
  }
}

/** How long a command the sheet ends has to leave after SIGTERM before its group is sent SIGKILL, ms: the harness's wait for a browser. */
export const COMMAND_STOP_MS = 5_000;

/**
 * Every command this process started that has not exited yet. If the sheet
 * ends first — it exits, throws, or is sent SIGINT, SIGTERM or SIGHUP — each
 * one's process group is sent that signal (SIGTERM on an exit), as a command
 * in the sheet's own group would have been by the terminal: a command is
 * started in a group of its own ({@link run}), so the terminal's signal no
 * longer reaches it. A command that has exited is not signalled, so a server
 * a `--restart` command started in the background is left running. A SIGKILL
 * of the sheet runs nothing, so a command can outlive it then
 * (docs/06-limits.md).
 *
 * @type {Set<import('node:child_process').ChildProcess>}
 */
const COMMANDS = new Set();
let guarded = false;

/** Installs the exit and signal guards for {@link COMMANDS}, once a process. */
function guardCommands() {
  if (guarded) return;
  guarded = true;
  process.on('exit', () => {
    for (const child of COMMANDS) signalGroup(child, 'SIGTERM');
  });
  for (const signal of /** @type {const} */ (['SIGINT', 'SIGTERM', 'SIGHUP'])) {
    const on = () => {
      for (const child of COMMANDS) signalGroup(child, signal);
      process.removeListener(signal, on);
      // The signal keeps its meaning: with no other listener, the process
      // ends as it would have without this guard.
      if (process.listenerCount(signal) === 0) process.kill(process.pid, signal);
    };
    process.on(signal, on);
  }
}

/** @param {number} ms */
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Runs `command` through the shell, DETACHED, so it leads a process group of
 * its own and a stop reaches every process it starts (verify r5, 23 Oct: a
 * `--restart` command was waited on with no bound, and the `--writer` was
 * stopped by a SIGTERM to its shell alone). `wait` resolves with its exit
 * code, or with what ended it instead, said after "the command": `was ended
 * by SIGTERM`, `could not be started: ...`; what it prints goes to the
 * sheet's stderr. `stop()` ends its group — SIGTERM,
 * then SIGKILL past {@link COMMAND_STOP_MS}, each wait bounded — and says how,
 * after "its process group".
 *
 * @param {string} command
 * @returns {{child: import('node:child_process').ChildProcess, wait: Promise<number | string>, stop: () => Promise<string>}}
 */
export function run(command) {
  // Its output goes to the sheet's stderr, so the sheet's stdout is the sheet alone.
  const child = spawn(command, { shell: true, stdio: ['ignore', 2, 2], detached: true });
  COMMANDS.add(child);
  guardCommands();
  let exited = false;
  /** @type {Promise<number | string>} */
  const wait = new Promise((resolve) => {
    /** @param {number | string} how */
    const done = (how) => {
      exited = true;
      COMMANDS.delete(child);
      resolve(how);
    };
    child.once('exit', (code, signal) => done(code ?? `was ended by ${signal}`));
    child.once('error', (why) => done(`could not be started: ${why.message}`));
  });
  /** Whether the command exited within `ms`. @param {number} ms */
  const exitsWithin = async (ms) => {
    /** @type {ReturnType<typeof setTimeout> | undefined} */
    let t;
    const ended = await Promise.race([wait.then(() => true), new Promise((resolve) => (t = setTimeout(() => resolve(false), ms)))]);
    clearTimeout(t);
    return ended;
  };
  /** Whether every process of its group left within `ms`. @param {number} ms */
  const groupLeft = async (ms) => {
    const until = performance.now() + ms;
    while (groupRunning(child.pid)) {
      if (performance.now() >= until) return false;
      await sleep(GROUP_POLL_MS);
    }
    return true;
  };
  return {
    child,
    wait,
    async stop() {
      const running = !exited && child.pid !== undefined;
      if (!running && !groupRunning(child.pid)) return 'had already left';
      signalGroup(child, 'SIGTERM');
      if ((!running || (await exitsWithin(COMMAND_STOP_MS))) && (await groupLeft(COMMAND_STOP_MS))) return 'was sent SIGTERM and left';
      // Bounded too: a process the kernel cannot end yet is waited on no longer than the rest.
      signalGroup(child, 'SIGKILL');
      await exitsWithin(COMMAND_STOP_MS);
      await groupLeft(COMMAND_STOP_MS);
      return `was sent SIGTERM and, still running ${COMMAND_STOP_MS} ms later, SIGKILL`;
    }
  };
}

/**
 * One run of the `--restart` command, BOUNDED (verify r5, 23 Oct: `--restart
 * 'sleep 1000'` held the run 60 s with nothing printed): its exit code, or
 * what happened instead, said after "the --restart command". Past `ms` its
 * process group is ended ({@link run}'s stop), and the run fails by name.
 *
 * @param {string} command
 * @param {number} ms `--restart-wait-ms`
 * @returns {Promise<number | string>}
 */
export async function restartWithin(command, ms) {
  const cmd = run(command);
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let t;
  const late = Symbol('late');
  const got = await Promise.race([cmd.wait, new Promise((resolve) => (t = setTimeout(() => resolve(late), ms)))]);
  clearTimeout(t);
  if (got !== late) return /** @type {number | string} */ (got);
  return `did not exit within ${ms} ms (--restart-wait-ms), so its process group ${await cmd.stop()}`;
}

/**
 * The text of a capture file, read only if it is a regular file a capture
 * could be (verify r5's class, 24 Oct: reading a named pipe that nobody
 * writes blocks the whole process for good, a wait with no bound and nothing
 * said). Opened without blocking and checked through the open file, so the
 * file read is the file checked; one past {@link MAX_TEXT_BYTES} is refused
 * too (a capture the harness writes is a few kilobytes). Throws by name.
 *
 * @param {string} path
 */
export function readCapturedFile(path) {
  const fd = openSync(path, constants.O_RDONLY | constants.O_NONBLOCK);
  try {
    const st = fstatSync(fd);
    if (!st.isFile()) throw new Error('not a regular file, and only a regular file is read');
    if (st.size > MAX_TEXT_BYTES) throw new Error(`${st.size} bytes, past the ${MAX_TEXT_BYTES} a capture may be`);
    return readFileSync(fd, 'utf8');
  } finally {
    closeSync(fd);
  }
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
 *   restart?: (command: string, waitMs: number) => Promise<number | string>}} [deps] `restart` runs one
 *   restart within `waitMs` and resolves with its exit code, or with what happened instead
 *   ({@link restartWithin}, the shell command, by default)
 * @returns {Promise<SheetResult>}
 */
export async function runSheet(args, deps = {}) {
  const sendTo = deps.send ?? timed;
  const restartOnce = deps.restart ?? restartWithin;
  const log = deps.log ?? ((line) => console.error(line));
  /** @param {string} path @param {object} [o] */
  const send = (path, o = {}) => sendTo(args.base, path, { timeoutMs: args.timeoutMs, ...o });
  /** @type {string[]} */
  const failures = [];
  /** @type {string[]} */
  const notMeasured = [];
  // THE CAPTURE IS READ BEFORE THE SERVER IS ASKED ANYTHING (verify r4
  // follow-up, 23 Oct): a missing `--from-pages` file was an uncaught ENOENT
  // with a stack, after the server had been asked. It is refused by name.
  /** @type {string | null} */
  let captureText = deps.capture ?? null;
  if (captureText === null && args.fromPages !== null) {
    try {
      captureText = readCapturedFile(args.fromPages);
    } catch (why) {
      return { ok: false, rows: [], ignored: [], failures: [`the capture of the pages' own requests could not be read: --from-pages ${args.fromPages}: ${/** @type {Error} */ (why).message}`], notMeasured, skipped: [], ctx: null, commit: null };
    }
  }

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
  else {
    // A CAPTURE THAT HOLDS NO REQUEST OF A MEASURED PAGE times none of what
    // that page builds inline, and says so page by page (verify r4
    // follow-up, 23 Oct: an empty capture gave a PASS sheet with no word).
    const held = new Set(captured.map((c) => pagePathOf(c.page)));
    for (const page of MEASURED_PAGES) {
      if (!held.has(pagePathOf(page))) notMeasured.push(`${page}: the capture holds no request of this page, so what it builds inline is not timed`);
    }
  }
  log(`speed sheet: ${requests.length} requests for ${ctx.feed} ${ctx.instrument} ${ctx.timeframe} ${ctx.month}, build ${commit ?? 'not named'}`);

  /**
   * A REQUEST THAT RAN ITS WHOLE `--timeout-ms` IS ASKED NO MORE THIS RUN
   * (verify r5, 23 Oct; the harness's rule for a load that does not end):
   * each later ask would wait as long, n of them a row, so a route that
   * never ends its answer held the deploy step for hours. By method and URL,
   * where it ran out; each pass that skips it says so by name.
   *
   * @type {Map<string, string>}
   */
  const stalled = new Map();
  const keyOf = (/** @type {{method: string, url: string}} */ r) => `${r.method} ${r.url}`;
  /** @param {SheetRequest} r @param {string} where @param {string} said */
  const stall = (r, where, said) => {
    if (stalled.has(keyOf(r))) return;
    stalled.set(keyOf(r), where);
    log(`speed sheet: ${r.method} ${r.url} had ${said} in ${where}; it is asked no more this run`);
  };
  /** The line a pass prints for a request it does not ask. @param {string} pass @param {SheetRequest} r */
  const skipLine = (pass, r) => `${pass}: ${r.method} ${r.url} was not asked: it had no whole answer within --timeout-ms ${args.timeoutMs} in ${stalled.get(keyOf(r))}, and each ask would wait as long`;

  log(`speed sheet: the field check of ${requests.filter((r) => r.url.includes('?')).length} requests with a query`);
  const ignored = await fieldCheck(send, requests);
  for (const i of ignored) failures.push(`${i.request.page} ${i.request.url}: field ${i.field} was ignored (${i.how}: ${i.said})`);
  for (const u of ignored.unchecked) notMeasured.push(u);
  failures.push(...ignored.failed);
  for (const s of ignored.stalled) {
    failures.push(`${s.request.page} ${s.request.method} ${s.request.url} (the field check): ${s.said}`);
    stall(s.request, 'the field check', s.said);
  }

  /** @type {SheetRow[]} */
  const rows = [];
  // A LARGE ANSWER IS TIMED BY ONE CLIENT AT A TIME, and every client count
  // it skips is said: its cold row's largest answer decides.
  /** @type {Map<SheetRequest, number>} */
  const large = new Map();
  /** @param {number} clients @param {string} phase */
  const asked = (clients, phase) => requests.filter((r) => {
    if (stalled.has(keyOf(r))) {
      notMeasured.push(skipLine(`${phase} at ${clients} clients`, r));
      return false;
    }
    const bytes = large.get(r);
    if (clients === 1 || bytes === undefined) return true;
    notMeasured.push(`${phase} at ${clients} clients: ${r.method} ${r.url} answered ${bytes} bytes, past --max-answer-bytes ${args.maxAnswerBytes}; ${clients} at once would hold ${clients * bytes} bytes in this process, so it is timed at one client only`);
    return false;
  });
  /** One row, timed; a row its request ended is said, and the request asked no more. @param {SheetRequest} r @param {{n: number, clients: number, phase: string, agentOf: () => http.Agent | false, timeoutMs: number}} o */
  const timedRow = async (r, o) => {
    const row = await measure(send, r, o);
    rows.push(row);
    if (row.cut !== null) {
      if (row.notAsked > 0) notMeasured.push(`${o.phase} at ${o.clients} clients: ${r.method} ${r.url}: ${row.notAsked} of its ${o.n} requests were not sent, after one had ${row.cut}`);
      stall(r, `the ${o.phase} pass at ${o.clients} clients`, row.cut);
    }
    return row;
  };
  log(`speed sheet: cold, n = ${args.n} a request`);
  for (const r of asked(1, 'cold')) {
    const row = await timedRow(r, { n: args.n, clients: 1, phase: 'cold', agentOf: () => false, timeoutMs: args.timeoutMs });
    if (row.bytes > args.maxAnswerBytes) large.set(r, row.bytes);
  }
  for (const clients of args.clients) {
    log(`speed sheet: warm at ${clients} clients`);
    for (const r of asked(clients, 'warm')) await timedRow(r, { n: args.n, clients, phase: 'warm', agentOf: warmAgent, timeoutMs: args.timeoutMs });
  }
  if (args.writer === null) notMeasured.push('warm+writer: no --writer command was given (an import or a pull writing)');
  else {
    const writer = run(args.writer);
    let exited = false;
    void writer.wait.then(() => (exited = true));
    // One pass a client count: a single --clients count is not timed twice.
    for (const clients of new Set([Math.min(...args.clients), Math.max(...args.clients)])) {
      log(`speed sheet: warm+writer at ${clients} clients`);
      for (const r of asked(clients, 'warm+writer')) await timedRow(r, { n: args.n, clients, phase: 'warm+writer', agentOf: warmAgent, timeoutMs: args.timeoutMs });
    }
    if (exited) failures.push('the --writer command ended before the warm+writer pass did: part of that pass ran with nothing writing');
    // ITS WHOLE GROUP, BOUNDED: a SIGTERM to its shell alone left what the
    // shell started still writing after the sheet ended (verify r5 follow-up).
    log(`speed sheet: the --writer command's process group ${await writer.stop()}`);
  }
  if (args.restart === null) notMeasured.push(`first after restart: no --restart command was given (${args.restarts} restarts asked)`);
  else {
    /** @type {Map<string, number[]>} */
    const firsts = new Map();
    /** @type {Map<string, Record<string, number>>} each request's answers after the restarts, by class */
    const firstClasses = new Map();
    let ran = 0;
    /** @type {string | null} why the pass ended before its last restart */
    let ended = null;
    for (let i = 0; i < args.restarts; i += 1) {
      log(`speed sheet: restart ${i + 1} of ${args.restarts}`);
      const code = await restartOnce(args.restart, args.restartWaitMs);
      if (code !== 0) {
        failures.push(`restart ${i + 1}: the --restart command ${typeof code === 'number' ? `exited ${code}` : code}`);
        ended = `the --restart command failed at restart ${i + 1}`;
        break;
      }
      // THE UP WAIT IS --up-wait-ms IN ALL, on a clock that never steps
      // back: each check within it waits at most the time left.
      const t0 = performance.now();
      let up = false;
      for (let left = args.upWaitMs; left > 0 && !up; left = args.upWaitMs - (performance.now() - t0)) {
        up = (await send(UP_ROUTE, { method: UP_METHOD, timeoutMs: Math.max(1, Math.min(args.timeoutMs, Math.ceil(left))) })).status === 200;
        if (!up) await sleep(Math.max(0, Math.min(250, args.upWaitMs - (performance.now() - t0))));
      }
      if (!up) {
        failures.push(`restart ${i + 1}: the server was not up ${args.upWaitMs} ms after the restart`);
        ended = `the server was not up after restart ${i + 1}`;
        break;
      }
      ran += 1;
      for (const r of requests) {
        if (stalled.has(keyOf(r))) continue;
        const a = await send(r.url, { method: r.method });
        const key = keyOf(r);
        firsts.set(key, [...(firsts.get(key) ?? []), a.ms]);
        const counted = firstClasses.get(key) ?? {};
        counted[classOf(a)] = (counted[classOf(a)] ?? 0) + 1;
        firstClasses.set(key, counted);
        if (classOf(a) !== 'answered') failures.push(`restart ${i + 1}: the first ${r.url} answered ${'error' in a ? a.error : `HTTP ${a.status}`}`);
        if ('timedOut' in a && a.timedOut) stall(r, `the first ask after restart ${i + 1}`, a.error);
      }
    }
    if (ended !== null) notMeasured.push(`first after restart: timed after ${ran} of the ${args.restarts} restarts asked: ${ended}`);
    for (const r of requests) {
      const key = keyOf(r);
      const times = firsts.get(key);
      if (times === undefined) {
        if (ran > 0) notMeasured.push(skipLine('first after restart', r));
        continue;
      }
      const s = summary(times);
      if (s.n < ran) notMeasured.push(`first after restart: ${r.method} ${r.url} was asked after ${s.n} of the ${ran} restarts run: it had no whole answer within --timeout-ms ${args.timeoutMs} in ${stalled.get(key)}, and is asked no more`);
      // The classes are counted here as in every other row, so a restart row
      // never prints "0 answered" for twenty answers.
      rows.push({ page: r.page, route: r.route, label: r.label, url: r.url, builder: r.builder, phase: 'first after restart', clients: 1, n: s.n, p50: s.p50, p99: s.p99, max: s.max, classes: firstClasses.get(key) ?? {}, builds: commit ? [commit] : [], bytes: 0, said: null, cut: null, notAsked: 0 });
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

/**
 * Writes `text` to `path` WHOLE: into a new file beside it (opened `wx`, so
 * never one that exists), flushed to the disk, then renamed over `path`, so
 * a reader of `path` finds the old file or the whole new one, never half a
 * sheet (the R3-15 re-check, 20 Oct: `--out` and the harness's `--capture`
 * were written in place). The directory is flushed after the rename where
 * the platform allows it. A write that fails takes back the file it made,
 * that one file only, and throws by name; `path` is untouched.
 *
 * @param {string} path
 * @param {string} text
 */
export function writeWhole(path, text) {
  // A writer killed by SIGKILL between this open and the rename runs no
  // handler: it leaves this file, named by its pid and time, beside the
  // target, and the target as it was — never half written (docs/06-limits.md).
  const tmp = join(dirname(path), `.${basename(path)}.${process.pid}.${Date.now()}.tmp`);
  let fd;
  try {
    fd = openSync(tmp, 'wx', 0o644);
  } catch (why) {
    throw new Error(`could not write ${path} whole: ${/** @type {Error} */ (why).message}`);
  }
  try {
    writeFileSync(fd, text);
    fsyncSync(fd);
    closeSync(fd);
    fd = -1;
    renameSync(tmp, path);
  } catch (why) {
    if (fd !== -1) closeSync(fd);
    unlinkSync(tmp);
    throw new Error(`could not write ${path} whole: ${/** @type {Error} */ (why).message}`);
  }
  try {
    const dir = openSync(dirname(path), 'r');
    try {
      fsyncSync(dir);
    } finally {
      closeSync(dir);
    }
  } catch {
    // A directory cannot be flushed on every platform: the rename above is
    // what makes the file whole, and it has happened.
  }
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
  if (args.out) {
    try {
      writeWhole(args.out, text);
    } catch (why) {
      // The sheet is on stdout already; the file that could not be written is named, never a stack.
      console.error(`speed-sheet: ${/** @type {Error} */ (why).message}`);
      process.exit(2);
    }
  }
  process.exit(result.ok ? 0 : 1);
}

if (import.meta.url === pathToFileURL(String(process.argv[1])).href) await main();

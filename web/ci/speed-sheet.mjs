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
 * # A restart is proved before anything is timed (verify r9, 26 Oct)
 *
 * The pass checked only the command's exit code, that the up check answered
 * and the build it named, so a `--restart` command that exited 0 and
 * restarted nothing ('true', or a stop that failed before a start that found
 * the old server still listening) published warm answers as `first after
 * restart` rows under a PASS. Now the server's own start (`started_ms` of its
 * `/speed.json`) is read before each run of the command and again once the
 * server is up after it; a start that did not change fails that restart by
 * name and ends the pass, as another build does, and a start that cannot be
 * read is said and ends it too.
 *
 * # A first is its route's first, and none waits for the start (verify r10)
 *
 * The up check that found the server up was the up route's sample, so a
 * `--restart` command that returned before the server answered timed the
 * server's start as `HEAD /inspection.json`'s first (4.5 to 6.6 s against
 * the server's own 42 µs); and a route the command asked itself (serve.sh's
 * readiness `GET /feeds.json`) was published as a first with a warm time
 * (1.3 ms against the server's own 5.3 s). Now the up wait asks
 * {@link UP_WAIT_ROUTE}, which no request times, until the server answers
 * it, so the wait for the start is that route's; the read that proves the
 * restart is `/speed.json`'s first; and an ask after a restart is timed as a
 * first only when the server's own count proves it ({@link answeredOf}:
 * `since_start.n` of its `/speed.json`, read with the proof and again after
 * the pass): nobody asked its route before this sheet did, and nobody but
 * this sheet during the pass. Every other ask is a NOT MEASURED line naming
 * its restarts, the count and why.
 *
 * # Only a route's first ask in a pass is its first (verify r11, 27 Oct)
 *
 * The count proof took this sheet's own earlier asks as its own, so the
 * second and later asks of a route in one pass passed it: the
 * `/bars/window.json` extremes request, asked right after the grid, read
 * 4.06 / 8.34 ms as a first against its own first's 100 to 212 ms, and 13 of
 * the 30 requests of the deploy step's `/db` capture were published the same
 * way. Now an ask is timed as a first only when it was this sheet's first
 * ask of its route in the pass ({@link firstAfterRestart}); a later one is a
 * NOT MEASURED line naming the ask that went first. And a route's requests
 * take turns going first, one restart after another ({@link passOrder}), so
 * each still gets its own firsts.
 *
 * # Usage (the deploy step runs this after every deploy)
 *
 *   node web/ci/speed-sheet.mjs --base http://127.0.0.1:8080 \
 *     [--n 200] [--clients 1,10,60] [--writer '<command>'] \
 *     [--restart '<command>' --restarts 20] [--from-pages <capture.json>] \
 *     [--feed <wire>] [--instrument NSE-INDEX-NIFTY] [--timeframe <rung>] \
 *     [--month YYYY-MM] [--day YYYY-MM-DD] [--out <sheet.md>] [--allow-unstamped] \
 *     [--timeout-ms 60000] [--up-wait-ms 120000] [--restart-wait-ms <the up wait>] \
 *     [--max-answer-bytes 16000000] [--command-log <file>]
 *
 * # What a command prints goes to a file of its own (verify r9, 26 Oct)
 *
 * The `--writer` and `--restart` commands were given the sheet's own stderr.
 * A reader of it that had gone killed them with SIGPIPE (a writer that
 * printed failed its pass, a restart that printed failed by name), and the
 * child's start cleared O_NONBLOCK on the stream they shared, so a reader
 * alive and not reading froze the whole sheet inside a write, past every
 * timer. Now what they print is appended to `--command-log` (by default
 * `<--out>.commands.log`, or a file in the system's temporary directory
 * when there is no `--out`), each run of a command under a line that names
 * it, and the sheet names the file. A file never blocks and never breaks: a
 * server a `--restart` command starts in the background keeps printing into
 * it after the sheet has ended, where a pipe would end it at its next line.
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
 * load, and the page capture replays it. Sixty clients at once would ask
 * 60 × 184 MB of the server under test at once, so a request whose cold
 * answer is past `--max-answer-bytes` is timed warm at ONE client only;
 * every client count it skips is a NOT MEASURED line naming the bytes. And
 * an answer past {@link MAX_TEXT_BYTES} is never made a string (a 591 MB
 * `/store.json` is past what one string may hold): it is kept as its length
 * and its sha256, so the ignored-field check still compares it exactly.
 *
 * # Every answer's memory is bounded too (verify r6, 24 Oct)
 *
 * Every chunk of an answer was kept until the answer ended, so an answer that
 * streamed fast and never ended held 4.78 GB of this process within a 5 s
 * `--timeout-ms`. Now {@link timed} counts the bytes as they come and keeps
 * at most a bound of them: a timed row its first {@link HEAD_BYTES} (it reads
 * only the start of a refusal), discovery and the field check at most
 * {@link MAX_TEXT_BYTES}, past which what was kept is hashed once and dropped
 * and every later byte is hashed as it comes. So no answer is ever held
 * whole, and docs/06-limits.md (L-R315-26) states the run's bound.
 *
 * # The build that answered is every row's own (verify r6, 24 Oct)
 *
 * The `first after restart` rows were stamped with the run's commit and 0
 * bytes, never with the answers they timed, so a restart that brought the
 * server back as another build passed. Each restart row is now built from
 * its own answers; the up check after each restart names the build it
 * found, and another build than the run's ends the restart pass by name.
 *
 * Exit 0 when every row was answered, stamped by one build and no field was
 * ignored; 1 with every reason printed otherwise; 2 for a misused command.
 * A closed or stalled stdout or stderr changes none of these and ends no run
 * (verify r7, r8 and r9): what a reader that has gone did not take is
 * counted, the sheet names the first of the run's lines stderr did not take,
 * a reader that does not read costs the run at most {@link STDERR_SETTLE_MS}
 * at its end, the commands never share the stream, and `--out` is still
 * written whole.
 * It only reads: GET and HEAD, never a write, whatever a capture holds.
 */
import http from 'node:http';
import https from 'node:https';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { accessSync, closeSync, constants, fstatSync, fsyncSync, openSync, readFileSync, renameSync, statSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
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
 *   from: string, to: string, dayFrom?: 'given' | 'newest' | 'first'}} SheetContext the selection every request is built for: the
 *   one /db opens on (NIFTY · Spot · the feed's finest timeframe · newest first); `dayFrom`
 *   says how the day was chosen (given, the rung's newest, or the month's first, taken)
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
    // A CAPTURED REQUEST NAMES ITSELF (repair 11, 27 Oct): two of one route
    // read the same in the sheet's table, and each now has its own firsts
    // after a restart ({@link passOrder}).
    requests.push({ page: c.page, route: new URL(c.url, 'http://speed.sheet').pathname, label: `captured from the page: ${key}`, method: c.method, url: c.url, builder: '(the page, captured)' });
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
 * The first year the store holds: `store::path::YearMonth::new` refuses a
 * year outside 1970 to 9999 (a bar's time is microseconds since the Unix
 * epoch, so none is before 1970, and a four-digit year ends at 9999), so the
 * server refuses every request for an earlier month by name. `--month` and
 * `--day` are held to it (verify r10 follow-up, 26 Oct).
 */
export const FIRST_STORE_YEAR = 1970;

/**
 * Whether `YYYY-MM-DD` is a day the calendar has: its year 0001 or later
 * (the common era has no year 0; verify r9 follow-up, 26 Oct: `--month
 * 0000-01` was taken, and the server then refused every request by name),
 * its month 01 to 12 and its day in that month (2024-02-29 is, 2024-02-30
 * and 2024-13-01 are not). `setUTCFullYear` takes the year as written, where
 * `Date.UTC` would read 20 as 1920.
 *
 * @param {string} day digits in the YYYY-MM-DD shape
 */
export function calendarHas(day) {
  const [y, m, d] = day.split('-').map(Number);
  const t = new Date(0);
  t.setUTCFullYear(y, m - 1, d);
  return y >= 1 && t.getUTCFullYear() === y && t.getUTCMonth() === m - 1 && t.getUTCDate() === d;
}

/**
 * Whether two paths name one file: one device and one inode, so two
 * spellings of a file are one — a link to it, `/tmp` beside `/private/tmp`,
 * another case on a volume that folds case (verify r6 follow-up, 24 Oct: the
 * paths were compared as strings, resolved, and each of those got through).
 * Where either cannot be read the resolved spellings decide, as before: a
 * path that names nothing yet cannot be the file the other names.
 *
 * @param {string} a
 * @param {string} b
 */
export function sameFile(a, b) {
  try {
    const x = statSync(a);
    const y = statSync(b);
    return x.dev === y.dev && x.ino === y.ino;
  } catch {
    return resolvePath(a) === resolvePath(b);
  }
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
  const valued = new Set(['base', 'n', 'clients', 'writer', 'restart', 'restarts', 'from-pages', 'feed', 'instrument', 'timeframe', 'month', 'day', 'out', 'timeout-ms', 'up-wait-ms', 'restart-wait-ms', 'max-answer-bytes', 'command-log']);
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
  // A FILE IS NAMED, NEVER EMPTY (verify r10 follow-up, 26 Oct: `--out ''`
  // wrote no file and said nothing, and `--command-log ''` was refused only
  // at the first command, after the cold and warm passes, as ENOENT for '').
  for (const name of ['out', 'command-log', 'from-pages']) {
    if (o[name] === '') throw new Error(`--${name} names no file: an empty name is refused`);
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
  // A DAY AND A MONTH THE CALENDAR HAS (verify r6 follow-up, 24 Oct: the
  // format alone took 2024-13 and 2024-02-30, and the run then asked for them).
  const day = o.day === undefined ? null : String(o.day);
  if (day !== null && !(/^\d{4}-\d{2}-\d{2}$/.test(day) && calendarHas(day))) throw new Error('--day is YYYY-MM-DD, a day the calendar has');
  const month = o.month === undefined ? null : String(o.month);
  if (month !== null && !(/^\d{4}-\d{2}$/.test(month) && calendarHas(`${month}-01`))) throw new Error('--month is YYYY-MM, a month the calendar has');
  // A YEAR THE STORE HOLDS, as the server refuses any other (verify r10
  // follow-up, 26 Oct: 0001-01 to 1969-12 were taken, and the server then
  // refused every request by name). Four digits already bound the top.
  for (const [flag, v] of /** @type {const} */ ([['--day', day], ['--month', month]])) {
    if (v !== null && Number(v.slice(0, 4)) < FIRST_STORE_YEAR) throw new Error(`${flag} ${v} is before ${FIRST_STORE_YEAR}: the store holds no month before it (store::path::YearMonth holds ${FIRST_STORE_YEAR} to 9999), so the server would refuse every request`);
  }
  // A DAY OF ITS MONTH (verify r7 follow-up, 25 Oct: `--day 2024-02-29
  // --month 2024-12` was taken, and the run asked the month's pages about a
  // day the month does not hold).
  if (day !== null && month !== null && day.slice(0, 7) !== month) throw new Error(`--day ${day} is not a day of --month ${month}`);
  // ONE FILE IS NOT BOTH (verify r5 follow-up, 23 Oct: the harness wrote its
  // sheet over its own capture and exited 0): the sheet read here would be
  // replaced by the one written at the end. By the file, not its spelling.
  if (o.out !== undefined && o['from-pages'] !== undefined && sameFile(String(o.out), String(o['from-pages']))) {
    throw new Error(`--out and --from-pages name one file (${String(o.out)}): the sheet would be written over the capture it reads`);
  }
  // WHERE THE COMMANDS PRINT (verify r9, 26 Oct): a file of their own, never
  // the sheet's stderr ({@link openCommandLog}); beside the sheet unless named.
  const commandLog = o['command-log'] !== undefined ? String(o['command-log']) : o.out !== undefined ? `${String(o.out)}.commands.log` : null;
  for (const other of /** @type {const} */ (['out', 'from-pages'])) {
    if (commandLog !== null && o[other] !== undefined && sameFile(commandLog, String(o[other]))) {
      throw new Error(`--command-log and --${other} name one file (${commandLog}): what the commands print would be written into the ${other === 'out' ? 'sheet' : 'capture'}`);
    }
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
    maxAnswerBytes: whole('max-answer-bytes', MAX_ANSWER_BYTES, 1, MAX_TEXT_BYTES),
    // `null`: a file in the system's temporary directory, made at the first command.
    commandLog
  };
}

/**
 * @typedef {{status: number, headers: Record<string, string | string[] | undefined>, body: string,
 *   ms: number, bytes: number, build: string | null, kept?: 'text' | 'digest' | 'head'}} Answer
 *   `bytes` every byte of its body, counted as it came; `kept` what `body` holds of them
 *   ({@link timed}): the text, the length and sha256, or the first {@link HEAD_BYTES} as text
 * @typedef {{status: 0, error: string, ms: number, build: string | null, body: '', bytes: 0, headers: {}, timedOut?: true}} NoAnswer
 *   `timedOut` when its whole answer did not come within its bound ({@link timed}); `build` the
 *   build its headers named, when they came before it was cut
 */

/**
 * The largest answer kept as text, bytes. A longer one is kept as
 * `(an answer of N bytes, sha256 H)`: exact for comparing two answers, and
 * never a string longer than one may be. It is also the most of an answer
 * {@link timed} ever holds (verify r6, 24 Oct).
 */
export const MAX_TEXT_BYTES = 64_000_000;

/** The largest answer timed by more than one client at once, bytes (`--max-answer-bytes`). */
export const MAX_ANSWER_BYTES = 16_000_000;

/**
 * How much of an answer a timed row keeps, bytes: its start, as text, for
 * the reason line of an answer that is not one (`measure` reads 300
 * characters of it, at most 1,200 bytes of UTF-8). A timed row compares no
 * body, so every later byte is counted and dropped (verify r6, 24 Oct).
 */
export const HEAD_BYTES = 4_096;

/** An answer kept as its length and digest. @param {number} bytes @param {string} hex */
const digestText = (bytes, hex) => `(an answer of ${bytes} bytes, sha256 ${hex})`;

/**
 * An answer's bytes as the sheet keeps them: the text, or past `limit`
 * its length and digest — the form {@link timed} keeps, for bytes in hand.
 *
 * @param {Buffer} body
 * @param {number} [limit]
 */
export function answerText(body, limit = MAX_TEXT_BYTES) {
  return body.length <= limit ? body.toString('utf8') : digestText(body.length, createHash('sha256').update(body).digest('hex'));
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
 * AND SO IS WHAT IT HOLDS (verify r6, 24 Oct): every chunk was kept until
 * the answer ended, so an answer that streamed fast and never ended grew the
 * sheet to 4.78 GB within a 5 s bound, and Buffer.concat then made a second
 * copy. Now every byte is counted as it comes and copied into ONE buffer
 * that never grows past its bound: `keep: 'whole'` (the default; discovery
 * and the field check, which read or compare bodies) keeps up to
 * {@link MAX_TEXT_BYTES} as text, and past it hashes what it kept once,
 * drops it and hashes every later chunk as it comes, so the answer is its
 * length and sha256 without ever being held whole; `keep: 'head'` (a timed
 * row) keeps its first {@link HEAD_BYTES} and counts the rest. The time is
 * read at the last byte, before the text is decoded.
 *
 * @param {string} base
 * @param {string} path
 * @param {{method?: string, agent?: http.Agent | false, timeoutMs?: number, keep?: 'whole' | 'head'}} [o]
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
  const head = o.keep === 'head';
  const textLimit = head ? HEAD_BYTES : MAX_TEXT_BYTES;
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
      // The bytes kept: one buffer, grown by doubling, never past textLimit.
      let kept = Buffer.alloc(0);
      let used = 0;
      /** @type {import('node:crypto').Hash | null} the whole answer's digest, once it is past MAX_TEXT_BYTES */
      let hash = null;
      res.on('data', (/** @type {Buffer} */ c) => {
        got += c.length;
        if (hash !== null) return void hash.update(c);
        const room = textLimit - used;
        if (c.length > room && !head) {
          // Past the text bound: from here on the answer is its length and digest.
          hash = createHash('sha256').update(kept.subarray(0, used)).update(c);
          kept = Buffer.alloc(0);
          used = 0;
          return;
        }
        const take = Math.min(c.length, room);
        if (take === 0) return;
        if (used + take > kept.length) {
          const grown = Buffer.allocUnsafe(Math.min(textLimit, Math.max(used + take, 2 * kept.length, 1_024)));
          kept.copy(grown, 0, 0, used);
          kept = grown;
        }
        c.copy(kept, used, 0, take);
        used += take;
      });
      res.on('end', () => {
        const took = ms();
        const body = hash !== null ? digestText(got, hash.digest('hex')) : kept.toString('utf8', 0, used);
        settle({ status: Number(res.statusCode), headers: res.headers, body, ms: took, bytes: got, build: headedBy ?? null, kept: hash !== null ? 'digest' : used < got ? 'head' : 'text' });
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
 *   classes: Record<string, number>, builds: string[], unnamed: number, bytes: number, said: string | null,
 *   cut: string | null, notAsked: number}} SheetRow `builds` the builds its answers named, `unnamed`
 *   how many of its answers came with no build named (one build more, when others named one); `bytes`
 *   its largest answer's; `cut` is the reason of the answer that ran its whole bound and ended the row;
 *   `notAsked` the requests of the row never sent after it
 */

/**
 * How an answer's build differs from the run's, said for a failure line, or
 * `null` when it is the run's: the build, or "no named build", then the
 * run's commit, or what the run's own up check named when it named none.
 *
 * @param {string | null} build
 * @param {string | null} commit the run's
 */
export function otherBuild(build, commit) {
  if (build === commit) return null;
  return `${build ?? 'no named build'}, not ${commit ?? "the run's, whose server named no build"}`;
}

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
  let unnamed = 0;
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
      // A TIMED ROW KEEPS ONLY THE START OF EACH ANSWER (verify r6): it reads no body but a refusal's first words.
      const a = await send(request.url, { method: request.method, agent, timeoutMs: o.timeoutMs, keep: 'head' });
      times.push(a.ms);
      const k = classOf(a);
      classes[k] = (classes[k] ?? 0) + 1;
      if (a.build !== null) builds.add(a.build);
      else if (a.status !== 0) unnamed += 1;
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
  return { page: request.page, route: request.route, label: request.label, url: request.url, builder: request.builder, phase: o.phase, clients: o.clients, n: s.n, p50: s.p50, p99: s.p99, max: s.max, classes, builds: [...builds], unnamed, bytes, said, cut, notAsked: left };
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
    // AN ANSWER NOT KEPT AS TEXT, OR NOT JSON, IS REFUSED BY NAME (verify r6): it was a bare SyntaxError.
    if ('kept' in a && a.kept !== undefined && a.kept !== 'text') throw new Error(`discovering the selection: ${path} answered ${a.bytes} bytes, past the ${MAX_TEXT_BYTES} the sheet keeps as text`);
    try {
      return JSON.parse(a.body);
    } catch (why) {
      throw new Error(`discovering the selection: ${path} answered HTTP 200 with a body that is not JSON (${/** @type {Error} */ (why).message})`);
    }
  };
  let feed = args.feed;
  if (feed === null) {
    const rows = await json('/feeds.json');
    const row = Array.isArray(rows) ? (rows.find((r) => r?.ready === true && r?.offered !== false) ?? rows[0]) : null;
    if (!row?.wire) throw new Error('discovering the selection: /feeds.json named no feed; pass --feed');
    feed = String(row.wire);
  }
  let { timeframe, month, day } = args;
  /** @type {SheetContext['dayFrom']} */
  let dayFrom = 'given';
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
    // THE DAY AND THE MONTH ARE ONE (verify r7 follow-up): a day given alone
    // is read in its own month, never in the rung's newest; a month given
    // alone takes the rung's newest day only when that day is in it, and
    // otherwise starts at its first day, as a rung that names no day does.
    month = month ?? (day !== null ? day.slice(0, 7) : String(finest.last_month));
    const newest = finest.last_day ? String(finest.last_day) : null;
    if (day === null) {
      // A DAY TAKEN, NOT FOUND, IS SAID (verify r8 follow-up, 26 Oct: --month
      // 2025-03 alone took 2025-03-01, a Saturday, and timed its empty answer
      // as the day's, saying nothing): `dayFrom` says how the day was chosen.
      dayFrom = newest !== null && newest.slice(0, 7) === month ? 'newest' : 'first';
      day = dayFrom === 'newest' ? String(newest) : `${month}-01`;
    }
  }
  return { feed, instrument: args.instrument, timeframe, month, day, dayFrom, ...monthDays(month) };
}

/**
 * What the sheet says of a day it took rather than found: `null` for a day
 * given or the rung's newest, a line for a month's first day, which nothing
 * here knows to be a session.
 *
 * @param {SheetContext} ctx
 * @returns {string | null}
 */
export function dayTakenNote(ctx) {
  if (ctx.dayFrom !== 'first') return null;
  return `the day ${ctx.day} was taken, not found: it is the first of ${ctx.month}, and the rung's newest day is not in that month, so whether it holds a session is not known here and the day's rows may time a day with no bars; pass --day to name a session day`;
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
      // KEPT ONLY IF IT COULD SHOW AN IGNORE (verify r6, 24 Oct): a garbled
      // answer says a field was ignored only when it is 2xx with the very
      // bytes of the original, so it is kept as the original's own text, and
      // any other is dropped here. The check holds the original and one
      // answer at a time, never one answer a field.
      if (ok(probe.status) && probe.body === first.body) garbled[field] = { status: probe.status, body: first.body };
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

/** What {@link run}'s stop says of a group that SIGTERM did not end: it was killed. */
export const KILLED = `was sent SIGTERM and, still running ${COMMAND_STOP_MS} ms later, SIGKILL`;

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
 * The file the commands the sheet runs print to, opened to append, by its
 * descriptor (verify r9, 26 Oct). Opened without blocking and checked
 * through the open file: only a regular file is taken, because a named pipe
 * or a terminal here would be the shared stream this file replaces — a
 * reader that goes kills the command, one that does not read stalls it — and
 * a named pipe nobody reads would block the open itself. Throws by name.
 *
 * @param {string} path
 * @returns {number}
 */
export function openCommandLog(path) {
  const fd = openSync(path, constants.O_WRONLY | constants.O_APPEND | constants.O_CREAT | constants.O_NONBLOCK, 0o644);
  if (fstatSync(fd).isFile()) return fd;
  closeSync(fd);
  throw new Error('not a regular file, and what the commands print is kept only in one');
}

/**
 * Appends one line to the commands' log: the line that names a command
 * before it runs. Never throws: the reason a line could not be written is
 * returned, for the caller to say (`null` once it is written).
 *
 * @param {number} fd {@link openCommandLog}'s
 * @param {string} line
 * @returns {string | null}
 */
export function heading(fd, line) {
  try {
    writeFileSync(fd, `${line}\n`);
    return null;
  } catch (why) {
    return /** @type {Error} */ (why).message;
  }
}

/**
 * Runs `command` through the shell, DETACHED, so it leads a process group of
 * its own and a stop reaches every process it starts (verify r5, 23 Oct: a
 * `--restart` command was waited on with no bound, and the `--writer` was
 * stopped by a SIGTERM to its shell alone). `wait` resolves with its exit
 * code, or with what ended it instead, said after "the command": `was ended
 * by SIGTERM`, `could not be started: ...`. What it prints goes to `log`
 * ({@link openCommandLog}), never to the sheet's own stdout or stderr.
 * `stop()` ends its group — SIGTERM,
 * then SIGKILL past {@link COMMAND_STOP_MS}, each wait bounded — and says how,
 * after "its process group".
 *
 * NEVER THE SHEET'S OWN STDERR (verify r9, 26 Oct): it was given the sheet's
 * fd 2. With the reader of that stream gone, the command died of SIGPIPE at
 * its first line; and a child's start clears O_NONBLOCK on its stdio, which
 * an inherited stream shares with the sheet, so from then on a reader alive
 * and not reading froze the sheet inside a write that no timer could end
 * (probed 26 Oct: a 300 KB write after such a spawn returned at 5,976 ms,
 * when the reader left; after a spawn given a file, 12 ms).
 *
 * @param {string} command
 * @param {number} log the commands' log, for its stdout and its stderr
 * @returns {{child: import('node:child_process').ChildProcess, wait: Promise<number | string>, stop: () => Promise<string>}}
 */
export function run(command, log) {
  const child = spawn(command, { shell: true, stdio: ['ignore', log, log], detached: true });
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
      return KILLED;
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
 * @param {number} log the commands' log ({@link run})
 * @returns {Promise<number | string>}
 */
export async function restartWithin(command, ms, log) {
  const cmd = run(command, log);
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let t;
  const late = Symbol('late');
  const got = await Promise.race([cmd.wait, new Promise((resolve) => (t = setTimeout(() => resolve(late), ms)))]);
  clearTimeout(t);
  if (got !== late) return /** @type {number | string} */ (got);
  return `did not exit within ${ms} ms (--restart-wait-ms), so its process group ${await cmd.stop()}`;
}

/**
 * THE ROUTE THE UP WAIT ASKS AFTER A RESTART, WHICH NO REQUEST TIMES (verify
 * r10, 26 Oct). Whatever is asked first after a `--restart` command that
 * returned before the server answers waits in the server's listen queue for
 * its start (the api says `listening`, then starts for about 5 s), so that
 * ask's time is the start's and not its route's: the up check that found the
 * server up was the up route's sample, and `HEAD /inspection.json`'s first
 * after restart read 4,515 ms against the server's own 42 µs. `/health` is
 * the server's word on itself (`api::server::health`, a read of what is
 * already in memory); no page asks it (`+layout.svelte` says why), so no
 * timed row is its; any answer it gives, a 503 too, says the server answers;
 * and every answer names its build. A request of a capture on this route is
 * never timed as a first: its first ask after a restart is this wait's.
 */
export const UP_WAIT_ROUTE = '/health';

/** The up wait's method: headers only, as the pages' own up check asks. */
export const UP_WAIT_METHOD = 'HEAD';

/**
 * An answer of `GET /speed.json` read as JSON with its `started_ms`, or why
 * it cannot be: the one parse {@link startedOf} and {@link answeredOf} share.
 *
 * @param {Answer | NoAnswer} a
 * @returns {{started: number, body: {rows?: unknown}} | {why: string}}
 */
function speedOf(a) {
  if (a.status !== 200) return { why: `GET ${SPEED_ROUTE} answered ${'error' in a ? a.error : `HTTP ${a.status}`}` };
  if ('kept' in a && a.kept !== undefined && a.kept !== 'text') return { why: `GET ${SPEED_ROUTE} answered ${a.bytes} bytes, past the ${MAX_TEXT_BYTES} the sheet keeps as text` };
  /** @type {unknown} */
  let body;
  try {
    body = JSON.parse(a.body);
  } catch (why) {
    return { why: `GET ${SPEED_ROUTE} answered HTTP 200 with a body that is not JSON (${/** @type {Error} */ (why).message})` };
  }
  const started = /** @type {{started_ms?: unknown} | null} */ (body)?.started_ms;
  if (typeof started !== 'number' || !Number.isSafeInteger(started) || started < 0) return { why: `GET ${SPEED_ROUTE} answered without a whole started_ms` };
  return { started, body: /** @type {{rows?: unknown}} */ (body) };
}

/**
 * The server's own start, from an answer of `GET /speed.json`: its
 * `started_ms`, the time the server's process began counting, which a new
 * process gives anew and the same process never changes (`api::speed`). Or
 * why it cannot be read, for the caller to say. A restart is proved by this
 * stamp changing across the `--restart` command (verify r9, 26 Oct: nothing
 * proved one, so a command that restarted nothing published warm answers as
 * `first after restart` rows under a PASS).
 *
 * @param {Answer | NoAnswer} a
 * @returns {{started: number} | {why: string}}
 */
export function startedOf(a) {
  const read = speedOf(a);
  return 'why' in read ? read : { started: read.started };
}

/**
 * What the server answered since its own start, route by route, with that
 * start: each row's `since_start.n` of an answer of `GET /speed.json` —
 * every ask the server answered, refused, failed or saw cancelled since its
 * process began counting, each counted when it ended (so not the ask that
 * reads it), under the route the server matched: one row for a path's `HEAD`
 * and `GET`, its query no part of it (`api::speed`). Or why it cannot be read.
 * An ask after a restart is timed as a first only when these counts prove it
 * its route's first (verify r10, 26 Oct: a route the `--restart` command had
 * asked itself was published as a first with a warm time). Linear in the
 * rows, which the server holds to `ROUTE_CAP` (256).
 *
 * @param {Answer | NoAnswer} a
 * @returns {{started: number, counts: Map<string, number>} | {why: string}}
 */
export function answeredOf(a) {
  const read = speedOf(a);
  if ('why' in read) return read;
  const rows = read.body.rows;
  if (!Array.isArray(rows)) return { why: `GET ${SPEED_ROUTE} answered without a rows list` };
  /** @type {Map<string, number>} */
  const counts = new Map();
  for (const [i, row] of rows.entries()) {
    const route = row?.route;
    const n = row?.since_start?.n;
    if (typeof route !== 'string' || typeof n !== 'number' || !Number.isSafeInteger(n) || n < 0) return { why: `GET ${SPEED_ROUTE} answered a row (its ${i}) without a route and a whole since_start.n` };
    if (counts.has(route)) return { why: `GET ${SPEED_ROUTE} answered the route ${route} in two rows` };
    counts.set(route, n);
  }
  return { started: read.started, counts };
}

/**
 * Restart numbers as runs: `[1, 2, 3, 5]` is `1-3, 5`, so a line that names
 * the restarts of a request stays short at any `--restarts`.
 *
 * @param {number[]} at ascending
 */
export function restartsText(at) {
  /** @type {string[]} */
  const runs = [];
  for (let i = 0; i < at.length; i += 1) {
    let j = i;
    while (j + 1 < at.length && at[j + 1] === at[j] + 1) j += 1;
    runs.push(j === i ? String(at[i]) : `${at[i]}-${at[j]}`);
    i = j;
  }
  return `restart${at.length === 1 ? '' : 's'} ${runs.join(', ')}`;
}

/**
 * A count over some restarts: one number, or the least and the most.
 *
 * @param {number[]} seen
 */
const timesText = (seen) => {
  const lo = Math.min(...seen);
  const hi = Math.max(...seen);
  return lo === hi ? `${lo} time${lo === 1 ? '' : 's'}${seen.length > 1 ? ' each' : ''}` : `${lo} to ${hi} times`;
};

/**
 * Names in a sentence: `a`, `a and b`, `a, b and c`.
 *
 * @param {string[]} names
 */
const andText = (names) => (names.length < 2 ? names.join('') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);

/**
 * The asks of this sheet that went first on a request's route in the passes
 * where it went later (`by`, one a restart in `at`, never null for an
 * `earlier` line), each named once, with its restarts when more than one
 * did: the route's requests take turns ({@link passOrder}). The read that
 * proved a restart is named as that.
 *
 * @param {{at: number[], by: (SheetRequest | null)[]}} l
 */
const earlierText = (l) => {
  /** @type {Map<string, {name: string, at: number[]}>} */
  const by = new Map();
  for (const [i, e] of l.by.entries()) {
    const name = `${/** @type {SheetRequest} */ (e).method} ${/** @type {SheetRequest} */ (e).url}`;
    const b = by.get(name) ?? { name, at: [] };
    by.set(name, b);
    b.at.push(l.at[i]);
  }
  return andText([...by.values()].map((b) => `${b.name}${b.name === `GET ${SPEED_ROUTE}` ? ' (the read that proved the restart)' : ''}${by.size > 1 ? ` (${restartsText(b.at)})` : ''}`));
};

/**
 * WHY AN ASK AFTER A RESTART IS NOT TIMED AS A FIRST, said after "is not
 * timed as a first after restart i (k of the n timed): " (verify r10, 26 Oct),
 * by the kind {@link firstAfterRestart} found and what the server's count
 * said at each of those restarts (`seen`); `earlier` names this sheet's ask
 * that went first on the route at each (`by`, verify r11, 27 Oct).
 *
 * @type {Record<string, (l: {r: SheetRequest, seen: number[], at: number[], by: (SheetRequest | null)[]}) => string>}
 */
export const NOT_FIRST = Object.freeze({
  'up wait': () => `its route is the one the up wait asks (${UP_WAIT_METHOD} ${UP_WAIT_ROUTE}) after each restart before anything is timed, so its first ask after the restart is that wait's`,
  'no answer': () => 'it had no answer (its FAILED line says why), so there is no first to time',
  'old server': () => `the up wait was answered before this server's own start by this machine's clock (its ${SPEED_ROUTE} started_ms is later than that answer): either the wait reached the server the command stopped, and this read, the new server's first, may have waited for its start, or the server's clock is ahead of this machine's; the two cannot be told apart`,
  earlier: (l) => `its route ${l.r.route} was asked first in ${l.at.length === 1 ? 'that pass' : 'each of those passes'} by this sheet's ${earlierText(l)}, so this ask was not its route's first since the server's start, and would time a route that ask had warmed`,
  before: (l) => `the server had already answered its route ${l.r.route} since its start when this sheet asked it (${timesText(l.seen)}: the --restart command's asks or another client's), so the first ask after the restart was not this sheet's`,
  during: (l) => `the server answered its route ${l.r.route} more often than this sheet asked it during the pass (by ${timesText(l.seen)}): another client asked it then, so this sheet's ask is not proved the first`,
  left: (l) => `the server counted its route ${l.r.route} more often than this sheet's answered asks of it (by ${timesText(l.seen)}), and this sheet left an ask of it with no answer, which the server may or may not have counted, so whether another client asked it first cannot be told`,
  unnamed: (l) => `the server's ${SPEED_ROUTE} has no row ${l.r.route} after this sheet's answered ask of it (the server counts that path under another route), so its count cannot prove the ask its route's first`,
  fewer: (l) => `the server counted its route ${l.r.route} ${timesText(l.seen)} by the end of the pass, fewer than this sheet's answered asks of it, so that row is not this route's count`
});

/**
 * Whether one ask after a restart is its route's first since the server's
 * start, by the server's own counts (verify r10, 26 Oct), or the kind of
 * {@link NOT_FIRST} it is instead and the count that says so. `before` is
 * read with the proof of the restart, before anything is timed, and `after`
 * once the pass has asked everything; `mine` is this sheet's asks of the
 * route in that pass: those answered, which the server counted before it
 * answered, and those left with no answer, which it may not have counted;
 * `earlier` is this sheet's ask that went first on the route in that pass
 * when it was not this one (the read that proved the restart, for
 * `/speed.json`). Proved when this ask was this sheet's first of the route
 * in the pass (verify r11, 27 Oct: a later one, the `/bars/window.json`
 * extremes after the grid, timed an answer the first had woken, 4.06 ms
 * against its own first's 100 to 212 ms), nobody had asked the route before
 * the pass and the server counted it exactly as often as this sheet's
 * answered asks; the route of the up wait is that wait's, and an answer that
 * names a start after the up wait's answer means the wait reached the old
 * server (the read that proved the restart, the new server's first, may then
 * hold its start).
 *
 * @param {SheetRequest} r
 * @param {Answer | NoAnswer} a this sheet's ask of `r` after the restart
 * @param {{before: Map<string, number>, after: Map<string, number>, mine: {answered: number, left: number},
 *   earlier: SheetRequest | null, proofRead: boolean, startedAfterUp: boolean}} o `proofRead` when `a` is
 *   the read that proved the restart; `startedAfterUp` when that read named a start later than the up
 *   wait's answer
 * @returns {{kind: keyof typeof NOT_FIRST, seen: number} | null}
 */
export function firstAfterRestart(r, a, o) {
  if (r.route === UP_WAIT_ROUTE) return { kind: 'up wait', seen: 0 };
  if (a.status === 0) return { kind: 'no answer', seen: 0 };
  if (o.proofRead && o.startedAfterUp) return { kind: 'old server', seen: 0 };
  if (o.earlier !== null) return { kind: 'earlier', seen: 0 };
  const before = o.before.get(r.route) ?? 0;
  if (before > 0) return { kind: 'before', seen: before };
  const n = o.after.get(r.route);
  if (n === undefined) return { kind: 'unnamed', seen: 0 };
  if (n < o.mine.answered) return { kind: 'fewer', seen: n };
  if (n > o.mine.answered) return { kind: o.mine.left > 0 ? 'left' : 'during', seen: n - o.mine.answered };
  return null;
}

/**
 * THE ORDER A RESTART'S PASS ASKS ITS REQUESTS IN: a route's requests take
 * turns going first (verify r11, 27 Oct). Only the first ask of a route after
 * a restart is that route's first since the server's start, so a request
 * asked after another of its route in one pass (the `/bars/window.json`
 * extremes after the grid, a capture's other queries of one path) is never
 * timed as a first ({@link firstAfterRestart}); in a fixed order it would
 * never be timed at all. So at restart `at` a route's ((at - 1) mod g)-th of
 * its g requests is asked first and the others after it in turn, each
 * request of the route in a place one of them holds in `requests`: the routes
 * asked before a route's first ask are the same at every restart, and over g
 * restarts each request goes first once. `/speed.json` turns as any route
 * does, though its first after a restart is always the read that proved
 * it, made before the pass, which the pass takes as that route's first ask.
 * Linear in the requests, once a restart.
 *
 * @param {SheetRequest[]} requests
 * @param {number} at the restart, from 1
 * @returns {SheetRequest[]}
 */
export function passOrder(requests, at) {
  /** @type {Map<string, number[]>} each route's places in `requests` */
  const places = new Map();
  for (const [i, r] of requests.entries()) {
    const p = places.get(r.route);
    if (p === undefined) places.set(r.route, [i]);
    else p.push(i);
  }
  /** @type {SheetRequest[]} every place is written once */
  const order = new Array(requests.length);
  for (const p of places.values()) {
    const turn = (at - 1) % p.length;
    for (const [j, place] of p.entries()) order[place] = requests[p[(j + turn) % p.length]];
  }
  return order;
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
 *   commit: string | null, notes?: string[], commandLog?: string}} SheetResult `commandLog` the file the run's
 *   commands printed to, when it kept one
 *
 * @param {ReturnType<typeof parseArgs>} args
 * @param {{send?: typeof timed, log?: (line: string) => void, capture?: string | null,
 *   restart?: (command: string, waitMs: number, i: number) => Promise<number | string>,
 *   openLog?: (path: string) => number}} [deps] `restart` runs restart `i` (from 1) within `waitMs` and
 *   resolves with its exit code, or with what happened instead ({@link restartWithin}, the shell command
 *   printing to the commands' log, by default); `openLog` opens that log ({@link openCommandLog} by default)
 * @returns {Promise<SheetResult & {notes: string[]}>} every return carries its notes (the day taken, R315-38)
 */
export async function runSheet(args, deps = {}) {
  const sendTo = deps.send ?? timed;
  const openLog = deps.openLog ?? openCommandLog;
  // A closed stderr is counted, never thrown (verify r8: the console's line threw it a tick later, mid-run).
  const log = deps.log ?? saidOn(process.stderr).say;
  /** @param {string} path @param {object} [o] */
  const send = (path, o = {}) => sendTo(args.base, path, { timeoutMs: args.timeoutMs, ...o });
  /**
   * WHERE THE COMMANDS PRINT (verify r9, 26 Oct): one file, opened at the
   * first command the run starts and appended to, never the sheet's stderr
   * ({@link run}). Each run of a command is headed there by a line naming it.
   *
   * @type {{fd: number, path: string, names: Set<string>} | {why: string} | null}
   */
  let kept = null;
  /**
   * The log's descriptor, headed for one run of a command, or why that
   * command is not run: one whose output could not be kept is refused, never
   * run with its output lost.
   *
   * @param {'--writer' | '--restart'} name @param {string} title what the heading line says of this run
   * @returns {{fd: number} | {why: string}}
   */
  const loggedFor = (name, title) => {
    if (kept === null) {
      const path = args.commandLog ?? join(tmpdir(), `speed-sheet-${process.pid}-${Date.now()}.commands.log`);
      try {
        kept = { fd: openLog(path), path, names: new Set() };
        log(`speed sheet: what the commands print goes to ${path}`);
      } catch (why) {
        kept = { why: `what it prints could not be kept in ${path}: ${/** @type {Error} */ (why).message}` };
      }
    }
    if ('why' in kept) return kept;
    // THE HEADING NAMES THIS SHEET (verify r10 follow-up, 26 Oct): sheets
    // sharing one --command-log interleaved with no way to tell their runs apart.
    const failed = heading(kept.fd, `=== ${new Date().toISOString()} speed sheet ${process.pid}: ${title}`);
    if (failed !== null) return { why: `what it prints could not be kept in ${kept.path}: ${failed}` };
    kept.names.add(name);
    return kept;
  };
  const restartOnce = deps.restart ?? (async (/** @type {string} */ command, /** @type {number} */ waitMs, /** @type {number} */ i) => {
    const into = loggedFor('--restart', `restart ${i} of ${args.restarts}, the --restart command: ${command}`);
    return 'why' in into ? `was not run: ${into.why}` : restartWithin(command, waitMs, into.fd);
  });
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
      return { ok: false, rows: [], ignored: [], failures: [`the capture of the pages' own requests could not be read: --from-pages ${args.fromPages}: ${/** @type {Error} */ (why).message}`], notMeasured, skipped: [], ctx: null, commit: null, notes: [] };
    }
  }

  const up = await send(UP_ROUTE, { method: UP_METHOD });
  if (up.status !== 200) {
    return { ok: false, rows: [], ignored: [], failures: [`the server at ${args.base} is not up: ${UP_METHOD} ${UP_ROUTE} answered ${'error' in up ? up.error : `HTTP ${up.status}`}`], notMeasured, skipped: [], ctx: null, commit: null, notes: [] };
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
    return { ok: false, rows: [], ignored: [], failures: [/** @type {Error} */ (why).message], notMeasured, skipped: [], ctx: null, commit, notes: [] };
  }
  const taken = dayTakenNote(ctx);
  /** @type {string[]} */
  const notes = taken === null ? [] : [taken];
  /** @type {{page: string, method: string, url: string}[]} */
  let captured = [];
  try {
    captured = captureText === null ? [] : readCapture(captureText);
  } catch (why) {
    // A CAPTURE THAT CANNOT BE READ IS A FAILED RUN, NAMED: replaying part
    // of it, or none of it in silence, would time fewer pages than asked.
    return { ok: false, rows: [], ignored: [], failures: [`the capture of the pages' own requests could not be read: ${/** @type {Error} */ (why).message}`], notMeasured, skipped: [], ctx, commit, notes };
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
  // it skips is said: its cold row's largest answer decides. A timed row no
  // longer holds its answers (verify r6), so what many at once would cost is
  // the server's and the wire's: the bytes asked of the server at once.
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
    notMeasured.push(`${phase} at ${clients} clients: ${r.method} ${r.url} answered ${bytes} bytes, past --max-answer-bytes ${args.maxAnswerBytes}; ${clients} at once would ask ${clients * bytes} bytes of the server at once, so it is timed at one client only`);
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
  const into = args.writer === null ? null : loggedFor('--writer', `the --writer command: ${args.writer}`);
  if (into === null) notMeasured.push('warm+writer: no --writer command was given (an import or a pull writing)');
  else if ('why' in into) {
    failures.push(`the --writer command was not run: ${into.why}`);
    notMeasured.push('warm+writer: the --writer command was not run (its FAILED line says why)');
  } else {
    const writer = run(/** @type {string} */ (args.writer), into.fd);
    /** @type {number | string | null} how it ended, once it has */
    let gone = null;
    void writer.wait.then((how) => (gone = how));
    // One pass a client count: a single --clients count is not timed twice.
    for (const clients of new Set([Math.min(...args.clients), Math.max(...args.clients)])) {
      log(`speed sheet: warm+writer at ${clients} clients`);
      for (const r of asked(clients, 'warm+writer')) await timedRow(r, { n: args.n, clients, phase: 'warm+writer', agentOf: warmAgent, timeoutMs: args.timeoutMs });
    }
    // HOW IT ENDED IS NAMED (verify r9, 26 Oct: a writer killed by SIGPIPE read as any early end).
    if (gone !== null) failures.push(`the --writer command ended before the warm+writer pass did: it ${typeof gone === 'number' ? `exited ${gone}` : gone}, so part of that pass ran with nothing writing`);
    // ITS WHOLE GROUP, BOUNDED: a SIGTERM to its shell alone left what the
    // shell started still writing after the sheet ended (verify r5 follow-up).
    const how = await writer.stop();
    log(`speed sheet: the --writer command's process group ${how}`);
    // A WRITER KILLED IS SAID IN THE SHEET (verify r6 follow-up, 24 Oct): it was said on stderr only.
    if (how === KILLED) failures.push(`the --writer command's process group ${how}: what it was writing was ended by force, not asked to stop`);
    else notes.push(`the --writer command's process group ${how} at the end of the warm+writer pass`);
  }
  if (args.restart === null) notMeasured.push(`first after restart: no --restart command was given (${args.restarts} restarts asked)`);
  else {
    /**
     * Each request after the restarts: how many it was asked after, and its
     * asks proved firsts, kept as a timed row keeps its answers: times,
     * classes, the builds they named, the bytes.
     *
     * @type {Map<string, {asked: number, times: number[], classes: Record<string, number>, builds: Set<string>, unnamed: number, bytes: number, said: string | null}>}
     */
    const firsts = new Map();
    /**
     * The asks after a restart not timed as firsts, by request and kind
     * ({@link NOT_FIRST}): the restarts, what the server's count said at
     * each, and this sheet's ask that went first on the route there, if one
     * did. Said once a request and kind, after the pass.
     *
     * @type {Map<string, {r: SheetRequest, kind: keyof typeof NOT_FIRST, at: number[], seen: number[], by: (SheetRequest | null)[]}>}
     */
    const lost = new Map();
    let ran = 0;
    /** @type {string | null} why the pass ended before its last restart */
    let ended = null;
    const speedKey = `GET ${SPEED_ROUTE}`;
    /** @type {string[]} each restart timed, as the server's start before and after its command */
    const proved = [];
    /** @type {number[]} each restart timed, how long after its command the server answered the up wait, ms */
    const waited = [];
    /**
     * The server's `/speed.json` now, read by `of`, with the answer it was
     * read from, or why it cannot be read; one asked no more this run is not
     * asked again.
     *
     * @template {object} T
     * @param {(a: Answer | NoAnswer) => T | {why: string}} of
     * @returns {Promise<(T & {answer: Answer | NoAnswer}) | {why: string}>}
     */
    const speedNow = async (of) => {
      if (stalled.has(speedKey)) return { why: `GET ${SPEED_ROUTE} is asked no more this run: it had no whole answer within --timeout-ms ${args.timeoutMs} in ${stalled.get(speedKey)}` };
      const answer = await send(SPEED_ROUTE);
      const read = of(answer);
      return 'why' in read ? read : { ...read, answer };
    };
    for (let i = 0; i < args.restarts; i += 1) {
      const at = i + 1;
      log(`speed sheet: restart ${at} of ${args.restarts}`);
      // THE SERVER'S OWN START, READ BEFORE THE COMMAND (verify r9, 26 Oct):
      // the restart it runs is proved below by this stamp changing.
      const was = await speedNow(startedOf);
      if ('why' in was) {
        failures.push(`restart ${at}: the server's start could not be read before the --restart command, so a restart could not be told from none: ${was.why}`);
        ended = `the server's start could not be read at restart ${at}`;
        break;
      }
      const code = await restartOnce(args.restart, args.restartWaitMs, at);
      if (code !== 0) {
        failures.push(`restart ${at}: the --restart command ${typeof code === 'number' ? `exited ${code}` : code}`);
        ended = `the --restart command failed at restart ${at}`;
        break;
      }
      // THE UP WAIT ASKS A ROUTE NO REQUEST TIMES (verify r10, 26 Oct;
      // UP_WAIT_ROUTE): what is asked first after a command that returned
      // before the server answers waits for the server's start, so the up
      // check that found the server up timed that start as the up route's
      // first. Any answer says the server answers. --up-wait-ms in all, on a
      // clock that never steps back: each check within it waits at most the
      // time left.
      const t0 = performance.now();
      /** @type {Answer | null} the up wait's answer */
      let up = null;
      /** what the last check met when none was answered */
      let met = '';
      for (let left = args.upWaitMs; left > 0 && up === null; left = args.upWaitMs - (performance.now() - t0)) {
        const a = await send(UP_WAIT_ROUTE, { method: UP_WAIT_METHOD, timeoutMs: Math.max(1, Math.min(args.timeoutMs, Math.ceil(left))) });
        if (a.status !== 0) up = /** @type {Answer} */ (a);
        else {
          met = /** @type {NoAnswer} */ (a).error;
          await sleep(Math.max(0, Math.min(250, args.upWaitMs - (performance.now() - t0))));
        }
      }
      if (up === null) {
        failures.push(`restart ${at}: the server was not up ${args.upWaitMs} ms after the restart: ${UP_WAIT_METHOD} ${UP_WAIT_ROUTE} had no answer (${met})`);
        ended = `the server was not up after restart ${at}`;
        break;
      }
      // When the up wait was answered, on the clock the server stamps its start with.
      const upAt = Date.now();
      const upMs = Math.round(performance.now() - t0);
      // THE BUILD THAT CAME BACK IS NAMED (verify r6, 24 Oct): the up check
      // read only the status, so a restart that brought the server back as
      // another build passed, every row stamped with the run's commit. A
      // server back as another build ends the pass: what it would time is not
      // the run's build.
      log(`speed sheet: restart ${at}: up ${upMs} ms after the command (${UP_WAIT_METHOD} ${UP_WAIT_ROUTE} answered HTTP ${up.status}), answered by ${up.build ?? 'no named build'}`);
      const back = otherBuild(up.build, commit);
      if (back !== null) {
        failures.push(`restart ${at}: the server came back as ${back} (its ${UP_WAIT_METHOD} ${UP_WAIT_ROUTE} said so)`);
        ended = `the server came back as another build than the run's at restart ${at}`;
        break;
      }
      // THE RESTART IS PROVED BEFORE ANYTHING IS TIMED (verify r9, 26 Oct):
      // the command's exit code and an up check prove nothing, since a
      // command that restarted nothing ('true', or a stop refused before a
      // start that found the old server listening) exits 0 and the old
      // server answers. Its start that did not change ends the pass, as
      // another build does: what it would time is a warm server.
      const is = await speedNow(startedOf);
      if ('why' in is) {
        failures.push(`restart ${at}: the server's start could not be read after the --restart command, so a restart could not be told from none: ${is.why}`);
        ended = `the server's start could not be read at restart ${at}`;
        break;
      }
      if (is.started === was.started) {
        failures.push(`restart ${at}: the server at ${args.base} did not restart: its ${SPEED_ROUTE} said started_ms ${was.started} before the --restart command and after it`);
        ended = `the server did not restart at restart ${at}`;
        break;
      }
      log(`speed sheet: restart ${at}: the server restarted: started_ms ${was.started} before the command, ${is.started} after it`);
      // THE SAME ANSWER SAYS WHAT THE SERVER HAD ANSWERED SINCE THAT START
      // (verify r10, 26 Oct), which every ask of the pass is proved against:
      // counts that cannot be read time nothing after this restart.
      const before = answeredOf(is.answer);
      if ('why' in before) {
        failures.push(`restart ${at}: what the server answered since its start could not be read with the proof of the restart, so no ask after it could be proved its route's first: ${before.why}`);
        ended = `the server's count could not be read at restart ${at}`;
        break;
      }
      // THE PASS: each request asked once, `/speed.json` by the read that
      // proved the restart (asked again, its row would time a second ask as a
      // first), a route's requests in turn ({@link passOrder}). Its failures
      // are said as they come; whether each ask is timed as a first is
      // decided once the server's count after the pass is read.
      /** @type {Map<string, {answered: number, left: number}>} this sheet's asks of each route in this pass */
      const own = new Map();
      /** @type {Map<string, SheetRequest>} this sheet's first ask of each route after this restart: `/speed.json`'s is the read that proved it */
      const firstOf = new Map(requests.filter((r) => keyOf(r) === speedKey).map((r) => [r.route, r]));
      /** @type {{r: SheetRequest, a: Answer | NoAnswer, earlier: SheetRequest | null}[]} */
      const pass = [];
      let switched = false;
      for (const r of passOrder(requests, at)) {
        if (stalled.has(keyOf(r))) continue;
        // ONLY A ROUTE'S FIRST ASK IN THE PASS CAN BE ITS FIRST (verify r11,
        // 27 Oct): a later one would time what the first had warmed.
        const went = firstOf.get(r.route);
        if (went === undefined) firstOf.set(r.route, r);
        const a = keyOf(r) === speedKey ? is.answer : await send(r.url, { method: r.method, keep: 'head' });
        const mine = own.get(r.route) ?? { answered: 0, left: 0 };
        own.set(r.route, mine);
        if (a.status === 0) mine.left += 1;
        else mine.answered += 1;
        pass.push({ r, a, earlier: went === undefined || went === r ? null : went });
        if (classOf(a) !== 'answered') failures.push(`restart ${at}: the first ${r.url} answered ${'error' in a ? a.error : `HTTP ${a.status}`}`);
        // EACH ANSWER NAMES ITS BUILD: one whose headers came from another
        // build than the run's fails by name, the restart with it.
        const by = a.status === 0 && a.build === null ? null : otherBuild(a.build, commit);
        if (by !== null) {
          failures.push(`restart ${at}: ${r.method} ${r.url} was answered by ${by}`);
          switched = true;
        }
        if ('timedOut' in a && a.timedOut) stall(r, `the first ask after restart ${at}`, a.error);
      }
      // THE SERVER'S OWN COUNT, AGAIN AFTER THE PASS (verify r10, 26 Oct): an
      // ask of the route by another client during the pass is in it. One
      // that cannot be read times nothing of this restart, and a start that
      // changed during the pass makes its answers two servers'.
      const after = await speedNow(answeredOf);
      if ('why' in after) {
        failures.push(`restart ${at}: what the server answered since its start could not be read after the pass, so no ask of it could be proved its route's first: ${after.why}`);
        ended = `the server's count could not be read after the pass of restart ${at}`;
        break;
      }
      if (after.started !== is.started) {
        failures.push(`restart ${at}: the server started again during the pass: its ${SPEED_ROUTE} said started_ms ${is.started} before the pass and ${after.started} after it, so its answers are not one start's`);
        ended = `the server started again during the pass of restart ${at}`;
        break;
      }
      ran += 1;
      proved.push(`${was.started} to ${is.started}`);
      waited.push(upMs);
      for (const { r, a, earlier } of pass) {
        const key = keyOf(r);
        const f = firsts.get(key) ?? { asked: 0, times: [], classes: {}, builds: new Set(), unnamed: 0, bytes: 0, said: null };
        firsts.set(key, f);
        f.asked += 1;
        const not = firstAfterRestart(r, a, { before: before.counts, after: after.counts, mine: own.get(r.route) ?? { answered: 0, left: 0 }, earlier, proofRead: key === speedKey, startedAfterUp: is.started > upAt });
        if (not !== null) {
          const l = lost.get(`${key}\n${not.kind}`) ?? { r, kind: not.kind, at: [], seen: [], by: [] };
          lost.set(`${key}\n${not.kind}`, l);
          l.at.push(at);
          l.seen.push(not.seen);
          l.by.push(earlier);
          continue;
        }
        f.times.push(a.ms);
        const k = classOf(a);
        f.classes[k] = (f.classes[k] ?? 0) + 1;
        if (a.build !== null) f.builds.add(a.build);
        else f.unnamed += 1;
        f.bytes = Math.max(f.bytes, a.bytes);
        if (k !== 'answered') f.said ??= 'error' in a ? a.error : `HTTP ${a.status}: ${a.body.slice(0, 300)}`;
      }
      if (switched) {
        ended = `an answer after restart ${at} came from another build than the run's`;
        break;
      }
    }
    if (ended !== null) notMeasured.push(`first after restart: timed after ${ran} of the ${args.restarts} restarts asked: ${ended}`);
    // EACH RESTART TIMED IS SAID WITH ITS PROOF, in the sheet (verify r9: only
    // stderr said anything of it), and how its firsts were taken (verify r10).
    if (proved.length > 0) {
      notes.push(`first after restart: ${proved.length} restart${proved.length === 1 ? '' : 's'} timed, each proved by the server's own start (${SPEED_ROUTE} started_ms) changing across the --restart command: ${proved.join(', ')}`);
      const lo = Math.min(...waited);
      const hi = Math.max(...waited);
      notes.push(`first after restart: after each --restart command the up wait asked ${UP_WAIT_METHOD} ${UP_WAIT_ROUTE}, which no request times, until the server answered it (${lo === hi ? lo : `${lo} to ${hi}`} ms after the command), so no first waits for the server's start; and an ask is timed as its route's first only when it is this sheet's first ask of that route in the pass (a route's requests take turns going first, one restart after another) and the server's own count (${SPEED_ROUTE} since_start.n), read with the proof and again after the pass, holds no ask of that route but this sheet's`);
    }
    for (const r of requests) {
      const key = keyOf(r);
      const f = firsts.get(key);
      if (f === undefined) {
        if (ran > 0) notMeasured.push(skipLine('first after restart', r));
        continue;
      }
      if (f.asked < ran) notMeasured.push(`first after restart: ${r.method} ${r.url} was asked after ${f.asked} of the ${ran} restarts run: it had no whole answer within --timeout-ms ${args.timeoutMs} in ${stalled.get(key)}, and is asked no more`);
      // An ask not timed as a first is said by its own line, below.
      if (f.times.length === 0) continue;
      const s = summary(f.times);
      // EACH RESTART ROW IS ITS OWN ANSWERS' (verify r6, 24 Oct): it was
      // stamped with the run's commit and 0 bytes whatever answered it. Its
      // classes, builds and bytes are counted as every other row's are.
      rows.push({ page: r.page, route: r.route, label: r.label, url: r.url, builder: r.builder, phase: 'first after restart', clients: 1, n: s.n, p50: s.p50, p99: s.p99, max: s.max, classes: f.classes, builds: [...f.builds], unnamed: f.unnamed, bytes: f.bytes, said: f.said, cut: null, notAsked: 0 });
    }
    // EVERY ASK NOT TIMED AS A FIRST IS SAID, with its restarts, the server's count and why (verify r10).
    for (const l of lost.values()) notMeasured.push(`first after restart: ${l.r.method} ${l.r.url} is not timed as a first after ${restartsText(l.at)} (${l.at.length} of the ${ran} timed): ${NOT_FIRST[l.kind](l)}`);
  }
  for (const row of rows) {
    // AN ANSWER THAT NAMED NO BUILD IS ONE BUILD MORE beside those that named
    // one (verify r6): a server swapped for one that stamps nothing passed.
    const named = row.unnamed > 0 && row.builds.length > 0 ? [...row.builds, `no named build (${row.unnamed} of its answers)`] : row.builds;
    if (named.length > 1) failures.push(`${row.url} (${row.phase}, ${row.clients} clients) was answered by ${named.length} builds: ${named.join(', ')}`);
    else if (row.phase !== 'first after restart') {
      const by = otherBuild(row.builds[0] ?? null, commit);
      if (by !== null) failures.push(`${row.url} (${row.phase}) was answered by ${by}`);
    }
    // A restart row's answers were each failed by name, restart by restart, above.
    if (row.phase === 'first after restart') continue;
    for (const k of ['failed', 'no answer', 'refused']) {
      if ((row.classes[k] ?? 0) > 0) failures.push(`${row.page} ${row.url} (${row.phase}, ${row.clients} clients): ${row.classes[k]} ${k} — ${row.said}`);
    }
  }
  // THE COMMANDS' LOG IS NAMED IN THE SHEET, and this run's hold on it let go:
  // a process a command left running keeps its own.
  if (kept !== null && 'fd' in kept) {
    closeSync(kept.fd);
    if (kept.names.size > 0) notes.push(`what the ${[...kept.names].join(' and ')} command${kept.names.size > 1 ? 's' : ''} printed is kept in ${kept.path}, each run under a line that names it`);
  }
  return { ok: failures.length === 0, rows, ignored, failures, notMeasured, skipped, ctx, commit, notes, ...(kept !== null && 'fd' in kept ? { commandLog: kept.path } : {}) };
}

/** @param {number} ms */
const msText = (ms) => (ms < 10 ? ms.toFixed(2) : ms.toFixed(1));

/**
 * The sheet as Markdown: every row stamped with its commit, then every
 * section not measured and every failure. A note (what the run's own stderr
 * did not take, the proof of each restart, where the commands printed) comes
 * first, and never changes the verdict.
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
  // A cell from a capture may hold a '|' (a path or a query keeps it as
  // written), which would end the cell there: it is written escaped.
  const cell = (/** @type {string} */ s) => s.replaceAll('|', '\\|');
  for (const r of result.rows) {
    const c = (/** @type {string} */ k) => String(r.classes[k] ?? 0);
    // Answers that named no build beside some that named one are a second build (verify r6).
    const named = r.builds.length + (r.unnamed > 0 && r.builds.length > 0 ? 1 : 0);
    const commit = named === 1 ? r.builds[0].slice(0, 12) : named === 0 ? 'not named' : 'MIXED';
    lines.push(`| ${commit} | ${cell(r.page)} | \`${cell(r.route)}\` | ${cell(r.label || r.builder)} | ${r.phase} | ${r.clients} | ${r.n} | ${msText(r.p50)} | ${msText(r.p99)} | ${msText(r.max)} | ${c('answered')} | ${c('refused')} | ${c('busy')} | ${c('failed')} | ${c('no answer')} | ${r.bytes} |`);
  }
  lines.push('');
  for (const n of result.notes ?? []) lines.push(`- ${n}`);
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

/**
 * Why a file the run will write at its end cannot be written, or `''`: its
 * directory must exist, be a directory and take a new file, and the path
 * must not be a directory itself. Asked before the run starts, so a run is
 * never thrown away at its end for a path it could have refused at once
 * (verify r4 follow-up: a `--capture` that could not be written ended the
 * harness's run with exit 2 and no sheet, after the whole run; verify r6
 * follow-up: the sheet's `--out` was looked at only at the end). The
 * harness reads it from here.
 *
 * @param {string} option `--out` or `--capture`
 * @param {string} path
 */
export function unwritable(option, path) {
  const dir = dirname(path);
  try {
    if (!statSync(dir).isDirectory()) return `${option} ${path}: ${dir} is not a directory`;
    accessSync(dir, constants.W_OK);
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

/**
 * A command's own lines on a stream (stderr), and what became of them.
 *
 * @typedef {{say: (line: string) => void, closed: () => ({why: string, step: string | null, lost: number} | null),
 *   pending: () => number, settled: (ms: number) => Promise<boolean>}} Said
 */

/** Each stream {@link saidOn} says lines on, once a stream. @type {WeakMap<object, Said>} */
const SAYS = new WeakMap();

/**
 * How long the end of a run waits for stderr to take the run's own lines
 * before its sheet is written, ms: a line a closed stderr did not take is
 * known only once its write has called back, and the sheet says it. A
 * reader alive and not reading costs the run this wait and no more, because
 * the stream stays non-blocking: no command the sheet starts is given it
 * ({@link run}; verify r9, 26 Oct: one that was froze the sheet in a write).
 */
export const STDERR_SETTLE_MS = 1_000;

/**
 * A command's lines on `stream` (its stderr), one line a call, through ONE
 * persistent `error` listener a stream, so a reader that has gone is
 * COUNTED, never thrown (verify r8, 26 Oct: `process.stderr` had no `error`
 * listener in either command, and Node's console keeps none there, so the
 * EPIPE of a line written after the reader had gone was thrown as an
 * uncaught exception, a tick after its write: a sheet whose stderr reader had gone
 * (`2>&1 | head -1`) died mid-run with its `--out` never written and
 * nothing said, and either command's end turned its exit into 1 whatever
 * its run's verdict). Once a write has failed, no later line is written:
 * each is counted. `closed()` names the failure, the first line not taken
 * (`step`, null while none was lost) and how many were not; `settled(ms)`
 * resolves once every line handed has called back, or `false` at the bound.
 * The same object for the same stream, however often it is asked, so every
 * line of the process is counted in one place.
 *
 * @param {NodeJS.WritableStream} stream
 * @returns {Said}
 */
export function saidOn(stream) {
  const known = SAYS.get(stream);
  if (known !== undefined) return known;
  /** @type {string | null} */
  let why = null;
  /** @type {string | null} */
  let step = null;
  let lost = 0;
  let pending = 0;
  /** @type {Promise<void> | null} resolved once no line is in flight; a new one for each wait after that */
  let drained = null;
  /** @type {() => void} */
  let release = () => {};
  /** @param {NodeJS.ErrnoException} e */
  const failed = (e) => {
    why ??= String(e.code ?? e.message);
  };
  /** @param {string} line */
  const lose = (line) => {
    lost += 1;
    step ??= line;
  };
  stream.on?.('error', failed);
  /** @type {Said} */
  const said = {
    say(line) {
      if (why !== null) return void lose(line);
      pending += 1;
      stream.write(`${line}\n`, (/** @type {NodeJS.ErrnoException | null | undefined} */ e) => {
        pending -= 1;
        if (e) {
          failed(e);
          lose(line);
        }
        if (pending > 0) return;
        drained = null;
        release();
      });
    },
    closed: () => (why === null ? null : { why, step, lost }),
    pending: () => pending,
    settled(ms) {
      if (pending === 0) return Promise.resolve(true);
      drained ??= new Promise((resolve) => {
        release = resolve;
      });
      const now = drained;
      return new Promise((resolve) => {
        const t = setTimeout(() => resolve(false), ms);
        void now.then(() => {
          clearTimeout(t);
          resolve(true);
        });
      });
    }
  };
  SAYS.set(stream, said);
  return said;
}

/**
 * What a run's own stderr did not take, for its sheet, once the run's lines
 * have settled (at most `ms`): `null` when stderr took every one.
 *
 * @param {Said} said
 * @param {number} [ms]
 * @returns {Promise<string | null>}
 */
export async function stderrNote(said, ms = STDERR_SETTLE_MS) {
  const took = await said.settled(ms);
  const gone = said.closed();
  if (gone !== null && gone.step === null) return `stderr was closed (${gone.why}) during the run; none of this run's own lines on it was lost`;
  if (gone !== null) return `stderr was closed (${gone.why}) from the line "${gone.step}": ${gone.lost} of this run's own lines on it were not taken`;
  return took ? null : `stderr had not taken ${said.pending()} of this run's own lines ${ms} ms after the run`;
}

/** Each stream `written` listens to, and the sheets still waiting on it. @type {WeakMap<object, Set<(why: NodeJS.ErrnoException) => void>>} */
const HEARD = new WeakMap();

/** How many sheets handed to `stream` are still waiting on it (none once each has ended). @param {object} stream */
export const sheetsWaitingOn = (stream) => HEARD.get(stream)?.size ?? 0;

/** How long a command's end waits for stdout to take its whole sheet, ms. */
export const STDOUT_WAIT_MS = 60_000;

/**
 * Writes `text` to `stream` and resolves once the stream has taken it, so
 * the process may then exit (verify r6 follow-up, 24 Oct: `process.exit`
 * right after `process.stdout.write` cut a piped sheet past 64 KB on macOS,
 * 73,728 of 101,986 B, and the FAILED lines, printed last, were the ones
 * lost). Bounded: a reader that has not taken it within `ms` is said on
 * stderr, by name, and the promise resolves `false`.
 *
 * A READER THAT CLOSED THE PIPE IS SAID, NOT THROWN (verify r7 follow-up,
 * 25 Oct): the stream's `error` (EPIPE when the reader is gone, as
 * `| head -1` leaves it) had no listener, so it was thrown as uncaught and
 * a run that passed exited 1 with a stack. It is heard here, said on stderr
 * by name, and the promise resolves `false`; the caller's exit code stays
 * the run's own, and an `--out` file is still written whole. A stream's
 * failure reaches its write's callback first and its `error` event after,
 * so the listener stays for the rest of the process: ONE listener a stream
 * (`HEARD`), however many sheets it is handed, each failure said once.
 *
 * EVERY SHEET STILL WAITING HEARS ITS STREAM'S FAILURE (verify r8, 26 Oct:
 * the listener told only the newest sheet, and no test could tell, so a
 * sheet whose write was still in flight when its stream failed, behind an
 * earlier one, waited out its whole bound and then said the wrong reason,
 * "did not take ... within N ms"). Each sheet waits in its stream's set
 * from its write until it ends, each is said once, by its own size, and
 * none stays behind its end ({@link sheetsWaitingOn}). Its line goes out
 * through {@link saidOn}, so a closed stderr is counted, never thrown.
 *
 * @param {NodeJS.WritableStream} stream
 * @param {string} text
 * @param {string} name the command, for the line on stderr
 * @param {number} [ms]
 * @returns {Promise<boolean>}
 */
export function written(stream, text, name, ms = STDOUT_WAIT_MS) {
  return new Promise((resolve) => {
    let sheets = HEARD.get(stream);
    if (sheets === undefined) {
      /** @type {Set<(why: NodeJS.ErrnoException) => void>} */
      const all = new Set();
      sheets = all;
      HEARD.set(stream, all);
      // Each sheet leaves the set as it ends, and a Set's walk takes the deletion of the one it is on.
      stream.on?.('error', (/** @type {NodeJS.ErrnoException} */ why) => {
        for (const lost of all) lost(why);
      });
    }
    const waiting = sheets;
    let over = false;
    /** @type {ReturnType<typeof setTimeout> | undefined} */
    let t;
    /** @param {boolean} took @param {string} [said] */
    const end = (took, said) => {
      if (over) return;
      over = true;
      clearTimeout(t);
      waiting.delete(lost);
      if (said !== undefined) saidOn(process.stderr).say(said);
      resolve(took);
    };
    /** @param {NodeJS.ErrnoException} why */
    const lost = (why) => end(false, `${name}: stdout was closed before it took the whole sheet (${Buffer.byteLength(text)} B): ${why.code ?? why.message}; what it did not take is lost`);
    waiting.add(lost);
    t = setTimeout(() => end(false, `${name}: stdout did not take the whole sheet (${Buffer.byteLength(text)} B) within ${ms} ms; what it did not take is lost`), ms);
    stream.write(text, (/** @type {NodeJS.ErrnoException | null | undefined} */ why) => (why ? lost(why) : end(true)));
  });
}

async function main() {
  // FIRST: every line of the run goes to stderr through one listener, so a reader that has gone is counted, never thrown.
  const said = saidOn(process.stderr);
  let args;
  try {
    args = parseArgs(process.argv.slice(2));
    // CHECKED BEFORE THE RUN, as the harness checks its outputs (verify r6 follow-up).
    const why = args.out === null ? '' : unwritable('--out', args.out);
    if (why !== '') throw new Error(why);
    // And the file the commands print to, when a command is given (verify r9).
    const logWhy = args.commandLog === null || (args.writer === null && args.restart === null) ? '' : unwritable('--command-log', args.commandLog);
    if (logWhy !== '') throw new Error(logWhy);
  } catch (why) {
    said.say(`speed-sheet: ${/** @type {Error} */ (why).message}`);
    process.exit(2);
  }
  const result = await runSheet(args, { log: said.say });
  // The sheet says what its own stderr did not take, from which line (verify r8).
  const note = await stderrNote(said);
  const text = renderSheet(note === null ? result : { ...result, notes: [...result.notes, note] }, args);
  const flushed = written(process.stdout, text, 'speed-sheet');
  let code = result.ok ? 0 : 1;
  if (args.out) {
    try {
      // NEVER OVER THE COMMANDS' LOG (verify r10 follow-up, 26 Oct): --out and
      // --command-log naming one file that did not exist yet, by two spellings
      // (a case on a volume that folds it, /tmp and /private/tmp), passed the
      // check before the run, and the sheet's rename then replaced the log
      // the sheet names. Now that the log exists, the files are compared.
      if (result.commandLog !== undefined && sameFile(args.out, result.commandLog)) throw new Error(`--out ${args.out} is the commands' log ${result.commandLog}: the sheet is not written over what the commands printed`);
      writeWhole(args.out, text);
    } catch (why) {
      // The sheet is on stdout already; the file that could not be written is named, never a stack.
      said.say(`speed-sheet: ${/** @type {Error} */ (why).message}`);
      code = 2;
    }
  }
  await flushed;
  process.exit(code);
}

if (import.meta.url === pathToFileURL(String(process.argv[1])).href) await main();

/**
 * HOW FAST THE APP IS, SAID BY THE APP — the pure half of the Speed page, the
 * speed sheet, the browser harness and the bundle budget (R3-15; NEW-01,
 * NEW-08). No runes and no imports, so `web/tests/speed.test.js` drives every
 * rule here under `node --test`, `vite.config.js` reads the budget from it at
 * build time and the two `web/ci/` scripts share it with the page.
 *
 * # What was missing
 *
 * R3-14 made the server keep p50, p99 and the max per route (`/speed.json`,
 * `crates/api/src/speed.rs`) and nothing drew it: the live speed sheet was a
 * hand-run script outside the app, one build behind, that named its own
 * URLs; no browser ever measured a page; and nothing stopped a page's code
 * from growing.
 *
 * # What is here
 *
 * * {@link readSpeedReport}: `/speed.json` read and checked field by field. A
 *   report missing a figure is refused by name, never drawn as zeros.
 * * {@link speedRows}, {@link sortRows}, {@link microsText}: the table the
 *   Speed page draws, one window at a time.
 * * {@link speedTile}: the health page's "speed" tile, from the same report.
 * * {@link summary}: p50, p99 and the max of client-side samples, nearest rank
 *   (the sheet's and the harness's own measurements).
 * * {@link ignoredFields}: whether an answer shows that a field the request
 *   sent was ignored — the speed sheet fails its run on any.
 * * {@link builtPages}, {@link budgetVerdict}: what each page ships on its
 *   first load, read from the built files, against {@link BUNDLE_BUDGETS}.
 */

/**
 * The four page-side targets, as the operator set them for every page: first
 * content p99 under 300 ms, no pause over 50 ms, memory under 100 MB and under
 * 400 KB on the wire.
 *
 * Bytes are DECIMAL (1 KB = 1,000 B, 1 MB = 1,000,000 B): the stricter of the
 * two readings, so a page that passes passes under either.
 */
export const PAGE_TARGETS = Object.freeze({
  firstContentP99Ms: 300,
  longestPauseMs: 50,
  heapBytes: 100_000_000,
  wireBytes: 400_000
});

/**
 * The pages the browser harness measures against {@link PAGE_TARGETS}: the
 * five the work package names, then the pages added since — WP-58's All
 * sources page and this Speed page, which is held to the same targets.
 */
export const MEASURED_PAGES = Object.freeze(['/db', '/ingest', '/gaps', '/audit', '/markets', '/sources', '/speed']);

/** The route the Speed page draws (R3-14, `crates/api/src/speed_route.rs`). */
export const SPEED_ROUTE = '/speed.json';

/** The three windows every row carries, in the order the page offers them. */
export const WINDOWS = Object.freeze([
  Object.freeze({ key: 'last_hour', label: 'Last hour', why: 'The last hour of whole minutes the server froze (fewer minutes in the first hour after a start).' }),
  Object.freeze({ key: 'last_minute', label: 'Last minute', why: 'The last whole minute the server froze.' }),
  Object.freeze({ key: 'since_start', label: 'Since the start', why: 'Every request since this server process started.' })
]);

/** The figures one window carries, every one a whole number. */
const FIGURES = Object.freeze(['n', 'answered', 'refused', 'busy', 'failed', 'cancelled', 'p50_micros', 'p99_micros', 'max_micros']);

/** @param {unknown} v @returns {v is number} */
const whole = (v) => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;

/**
 * @typedef {{n: number, answered: number, refused: number, busy: number, failed: number,
 *   cancelled: number, p50_micros: number, p99_micros: number, max_micros: number}} Figures
 * @typedef {{route: string, kind: 'route' | 'stage', since_start: Figures, last_minute: Figures,
 *   last_hour: Figures, last_hour_minutes: number, first_after_start_micros: number | null}} SpeedRow
 * @typedef {{started_ms: number, now_ms: number, rows: SpeedRow[], over_cap: number, resolution: string}} SpeedReport
 */

/**
 * `/speed.json`, checked: every field the page draws is present and whole, so
 * a missing or renamed figure is a sentence naming it, never a row of zeros
 * that reads like a route nobody asked.
 *
 * @param {unknown} body the parsed answer
 * @returns {SpeedReport}
 */
export function readSpeedReport(body) {
  const b = /** @type {Record<string, unknown> | null} */ (body);
  if (!b || typeof b !== 'object' || Array.isArray(b)) throw new Error(`${SPEED_ROUTE} answered something that is not a report object`);
  for (const field of ['started_ms', 'now_ms', 'over_cap']) {
    if (!whole(b[field])) throw new Error(`${SPEED_ROUTE} answered without a whole ${field}`);
  }
  if (typeof b.resolution !== 'string' || b.resolution === '') throw new Error(`${SPEED_ROUTE} answered without its resolution sentence`);
  if (!Array.isArray(b.rows)) throw new Error(`${SPEED_ROUTE} answered without a rows list`);
  const rows = b.rows.map((raw, i) => {
    const r = /** @type {Record<string, unknown> | null} */ (raw);
    if (!r || typeof r !== 'object' || typeof r.route !== 'string' || r.route === '') {
      throw new Error(`${SPEED_ROUTE} row ${i} has no route name`);
    }
    if (r.kind !== 'route' && r.kind !== 'stage') throw new Error(`${SPEED_ROUTE} row ${r.route} has kind ${JSON.stringify(r.kind)}, not route or stage`);
    for (const w of WINDOWS) {
      const f = /** @type {Record<string, unknown> | null} */ (r[w.key]);
      if (!f || typeof f !== 'object') throw new Error(`${SPEED_ROUTE} row ${r.route} has no ${w.key} window`);
      for (const name of FIGURES) {
        if (!whole(f[name])) throw new Error(`${SPEED_ROUTE} row ${r.route} ${w.key} has no whole ${name}`);
      }
    }
    if (!whole(r.last_hour_minutes)) throw new Error(`${SPEED_ROUTE} row ${r.route} has no whole last_hour_minutes`);
    if (r.first_after_start_micros !== null && !whole(r.first_after_start_micros)) {
      throw new Error(`${SPEED_ROUTE} row ${r.route} has a first_after_start_micros that is neither null nor whole`);
    }
    return /** @type {SpeedRow} */ (/** @type {unknown} */ (r));
  });
  return /** @type {SpeedReport} */ ({ ...b, rows });
}

/**
 * A time in microseconds as the page prints it: µs under a millisecond, ms
 * under a second, s above; an em dash for a window that holds no request,
 * because 0 µs would claim a measurement nobody took.
 *
 * @param {number} micros
 * @param {number} [n] the window's request count; 0 prints the dash
 */
export function microsText(micros, n = 1) {
  if (n === 0 || !Number.isFinite(micros)) return '—';
  if (micros < 1_000) return `${micros} µs`;
  if (micros < 1_000_000) return `${(micros / 1_000).toFixed(micros < 10_000 ? 2 : 1)} ms`;
  return `${(micros / 1_000_000).toFixed(2)} s`;
}

/**
 * @typedef {{route: string, kind: 'route' | 'stage', order: number, n: number, answered: number,
 *   refused: number, busy: number, failed: number, cancelled: number, p50: number, p99: number,
 *   max: number, cold: number | null, minutes: number, slow: boolean}} TableRow
 */

/**
 * The table for one window: one line per row of the report, in the order the
 * server first saw each route, `slow` where the window's p99 alone is past
 * the 300 ms first-content target — a page that waits on that route cannot
 * draw inside its target, whatever the page does.
 *
 * @param {SpeedReport} report
 * @param {string} window one of {@link WINDOWS}' keys
 * @param {'all' | 'route' | 'stage'} [kind]
 * @returns {TableRow[]}
 */
export function speedRows(report, window, kind = 'all') {
  if (!WINDOWS.some((w) => w.key === window)) throw new Error(`no speed window is named ${JSON.stringify(window)}`);
  const limit = PAGE_TARGETS.firstContentP99Ms * 1_000;
  /** @type {TableRow[]} */
  const out = [];
  report.rows.forEach((row, order) => {
    if (kind !== 'all' && row.kind !== kind) return;
    const f = /** @type {Figures} */ (row[/** @type {'since_start'} */ (window)]);
    out.push({
      route: row.route,
      kind: row.kind,
      order,
      n: f.n,
      answered: f.answered,
      refused: f.refused,
      busy: f.busy,
      failed: f.failed,
      cancelled: f.cancelled,
      p50: f.p50_micros,
      p99: f.p99_micros,
      max: f.max_micros,
      cold: row.first_after_start_micros,
      minutes: row.last_hour_minutes,
      slow: f.n > 0 && f.p99_micros > limit
    });
  });
  return out;
}

/** The orders the table offers. */
export const SORTS = Object.freeze([
  Object.freeze({ key: 'order', label: 'First asked' }),
  Object.freeze({ key: 'p99', label: 'Slowest p99' }),
  Object.freeze({ key: 'n', label: 'Most asked' }),
  Object.freeze({ key: 'route', label: 'Name' })
]);

/**
 * The rows in one of {@link SORTS}' orders. A copy; ties keep the order the
 * server first saw them in, so a re-sort never shuffles equal rows.
 *
 * @param {TableRow[]} rows
 * @param {string} key
 */
export function sortRows(rows, key) {
  /** @type {(a: TableRow, b: TableRow) => number} */
  const by =
    key === 'p99' ? (a, b) => b.p99 - a.p99 :
    key === 'n' ? (a, b) => b.n - a.n :
    key === 'route' ? (a, b) => (a.route < b.route ? -1 : a.route > b.route ? 1 : 0) :
    () => 0;
  return [...rows].sort((a, b) => by(a, b) || a.order - b.order);
}

/**
 * @typedef {{state: 'empty' | 'ok' | 'slow' | 'failing', window: string, routes: number,
 *   requests: number, failed: number, worst: {route: string, p99: number} | null, line: string}} SpeedTile
 */

/**
 * The health page's "speed" tile: the slowest route's p99 and every failed
 * request, over the last hour, or since the start while no minute has been
 * frozen yet. Stages are left out: an import stage's minutes are not a page's
 * wait.
 *
 * @param {SpeedReport} report
 * @returns {SpeedTile}
 */
export function speedTile(report) {
  const hour = speedRows(report, 'last_hour', 'route');
  const window = hour.some((r) => r.n > 0) ? 'last_hour' : 'since_start';
  const rows = window === 'last_hour' ? hour : speedRows(report, 'since_start', 'route');
  const asked = rows.filter((r) => r.n > 0);
  const requests = asked.reduce((sum, r) => sum + r.n, 0);
  const failed = asked.reduce((sum, r) => sum + r.failed, 0);
  const worstRow = sortRows(asked, 'p99')[0] ?? null;
  const worst = worstRow ? { route: worstRow.route, p99: worstRow.p99 } : null;
  const label = WINDOWS.find((w) => w.key === window)?.label.toLowerCase() ?? window;
  if (!worst) return { state: 'empty', window, routes: 0, requests: 0, failed: 0, worst: null, line: 'No route has been asked since this server started.' };
  const state = failed > 0 ? 'failing' : asked.some((r) => r.slow) ? 'slow' : 'ok';
  const line =
    `${asked.length} routes, ${requests} requests (${label}); slowest p99 ${worst.route} ${microsText(worst.p99)}` +
    (failed > 0 ? `; ${failed} failed` : '');
  return { state, window, routes: asked.length, requests, failed, worst, line };
}

/**
 * What a page can say about its own load from the browser's performance
 * timeline: when its first content was painted, the bytes its navigation and
 * every resource it fetched carried on the wire, and its script heap where
 * the browser reports one. `null` for a figure the browser did not give —
 * never a zero. The longest pause is not here: a page cannot observe its own
 * long tasks after the fact in every browser, so the harness measures it.
 *
 * @param {{getEntriesByType: (type: string) => any[], memory?: {usedJSHeapSize?: number}} | null | undefined} perf
 * @returns {{firstContentMs: number | null, wireBytes: number | null, resources: number, heapBytes: number | null}}
 */
export function pageSelf(perf) {
  if (!perf || typeof perf.getEntriesByType !== 'function') return { firstContentMs: null, wireBytes: null, resources: 0, heapBytes: null };
  const paint = perf.getEntriesByType('paint').find((e) => e?.name === 'first-contentful-paint');
  const loads = [...perf.getEntriesByType('navigation'), ...perf.getEntriesByType('resource')];
  const sizes = loads.map((e) => e?.transferSize).filter((s) => typeof s === 'number' && Number.isFinite(s));
  const heap = perf.memory?.usedJSHeapSize;
  return {
    firstContentMs: paint && Number.isFinite(paint.startTime) ? Math.round(paint.startTime) : null,
    wireBytes: sizes.length > 0 ? sizes.reduce((a, b) => a + b, 0) : null,
    resources: loads.length,
    heapBytes: typeof heap === 'number' && Number.isFinite(heap) ? heap : null
  };
}

/**
 * p50, p99 and the max of `samples`, by nearest rank: the `ceil(n × q)`th
 * smallest, so every figure is a time that was actually measured. All zero
 * with `n: 0` for no samples — the caller must say "not measured", never
 * print the zero.
 *
 * @param {number[]} samples
 * @returns {{n: number, p50: number, p99: number, max: number}}
 */
export function summary(samples) {
  const n = samples.length;
  if (n === 0) return { n: 0, p50: 0, p99: 0, max: 0 };
  const sorted = [...samples].sort((a, b) => a - b);
  /** @param {number} q */
  const rank = (q) => sorted[Math.max(1, Math.ceil(n * q)) - 1];
  return { n, p50: rank(0.5), p99: rank(0.99), max: sorted[n - 1] };
}

/**
 * The value the speed sheet sends in place of a field to see whether the
 * server reads it: no reader of any field of this API can accept it.
 */
export const GARBLED = '~speed-sheet-garbled~';

/**
 * Whether one answer shows that a field the request sent was IGNORED, by the
 * server's own evidence — the speed sheet fails its run on any (R3-15 goal 2).
 *
 * Three signals, any one enough:
 *
 * 1. `named`: the answer is JSON with a non-empty `ignored` list (the shape
 *    `/logs.json` answers with), each entry naming a field.
 * 2. `refused`: a 400 whose sentence says the route does not read a field the
 *    request sent (`/imports.json`'s "does not read"). The page's own builder
 *    sent a field the route does not take, which is the same defect.
 * 3. `garbled`: the same request with that one field set to {@link GARBLED}
 *    was answered 2xx with the same bytes as the original 2xx answer. A route
 *    that reads the field refuses a value it cannot read, or answers
 *    differently; the same bytes mean the field changed nothing.
 *
 * Signal 3 is a lower bound: an answer that carries its own clock (a
 * `now_ms`) differs between two asks whatever was read, so a field ignored
 * there is caught only by signals 1 and 2.
 *
 * @param {{url: string, status: number, body: string,
 *   garbled?: Record<string, {status: number, body: string}>}} answer
 * @returns {{field: string, how: 'named' | 'refused' | 'garbled', said: string}[]}
 */
export function ignoredFields(answer) {
  const sent = [...new URL(answer.url, 'http://speed.sheet').searchParams.keys()];
  /** @type {{field: string, how: 'named' | 'refused' | 'garbled', said: string}[]} */
  const out = [];
  /** @type {any} */
  let json = null;
  try {
    json = JSON.parse(answer.body);
  } catch {
    json = null;
  }
  if (json && typeof json === 'object' && Array.isArray(json.ignored)) {
    for (const entry of json.ignored) {
      const field = typeof entry === 'string' ? entry : String(entry?.param ?? entry?.field ?? '?');
      out.push({ field, how: 'named', said: JSON.stringify(entry) });
    }
  }
  if (answer.status === 400) {
    const said = String(json?.error ?? answer.body);
    if (/does not read|is not a parameter|unknown parameter/i.test(said)) {
      for (const field of new Set(sent)) {
        if (said.includes(field)) out.push({ field, how: 'refused', said });
      }
    }
  }
  const ok = answer.status >= 200 && answer.status < 300;
  for (const [field, probe] of Object.entries(answer.garbled ?? {})) {
    if (ok && probe.status >= 200 && probe.status < 300 && probe.body === answer.body) {
      out.push({ field, how: 'garbled', said: `${field}=${GARBLED} was answered ${probe.status} with the same ${probe.body.length} bytes` });
    }
  }
  return out;
}

/**
 * The same URL with `field` set to {@link GARBLED}: every occurrence of a
 * repeated field, so a route that reads only the first is still asked.
 *
 * @param {string} url
 * @param {string} field
 */
export function garbledUrl(url, field) {
  const u = new URL(url, 'http://speed.sheet');
  const count = u.searchParams.getAll(field).length;
  u.searchParams.delete(field);
  for (let i = 0; i < Math.max(1, count); i += 1) u.searchParams.append(field, GARBLED);
  return `${u.pathname}?${u.searchParams}`;
}

/**
 * Each page's budget for what it ships on its first load, bytes: the
 * JavaScript and CSS the browser fetches before the page can draw. A page
 * past its budget FAILS THE BUILD (`vite.config.js`, `pageBudget`).
 *
 * The default is the 400 KB wire target ({@link PAGE_TARGETS}). Three pages
 * already ship more than that, measured on the build of `f9cdb584` (16 Oct
 * 2026) — `/backtest` 1,091,954 B, `/db` 726,909 B, `/ingest` 482,536 B —
 * and the server sends them uncompressed (`crates/api/src/assets.rs`), so the
 * wire carries every byte. Their budgets are held just above today's size
 * (rounded up to 25 KB), so they can shrink and cannot grow unseen; each is
 * OVER THE TARGET and every report says so. Raising one is a decision with
 * its reason, never an edit to make a build pass.
 *
 * @type {Readonly<Record<string, number>>}
 */
export const BUNDLE_BUDGETS = Object.freeze({
  default: PAGE_TARGETS.wireBytes,
  '/backtest': 1_100_000,
  '/db': 750_000,
  '/ingest': 500_000
});

/** @param {string} route */
export const budgetOf = (route) => BUNDLE_BUDGETS[route] ?? BUNDLE_BUDGETS.default;

/**
 * @typedef {{size: number, text?: string}} BuiltFile one file of the build:
 *   its size, and its text for a script (the only files read for imports)
 * @typedef {{route: string, node: number, js: number, css: number, bytes: number,
 *   files: string[]}} PageWeight
 */

/** A relative or root-absolute reference, resolved against `from`'s directory. */
function resolve(/** @type {string} */ from, /** @type {string} */ ref) {
  if (ref.startsWith('/')) return ref.slice(1);
  const parts = from.split('/').slice(0, -1);
  for (const piece of ref.split('/')) {
    if (piece === '..') parts.pop();
    else if (piece !== '.') parts.push(piece);
  }
  return parts.join('/');
}

/** Every static import a built script names: `from"./x.js"` and `import"./x.js"`. */
const STATIC_IMPORT = /(?:\bfrom|\bimport)\s*["'](\.{1,2}\/[^"']+\.js)["']/g;

/**
 * What every page ships on its first load, read from the BUILT FILES — the
 * same rule for `vite build` (over its client output) and for the test (over
 * the committed `web/build`), so the two cannot disagree about a page.
 *
 * A page's first load is the client entry `start` and `app` scripts with
 * everything they import statically, the root layout's node and the page's
 * own node with every script and stylesheet Vite preloads for each (its
 * `__vite__mapDeps` list in `app`). A script the page imports only when the
 * reader asks for it (a lazily loaded chart) is not counted: it is not on
 * the first load.
 *
 * REFUSES rather than guesses: a build whose `app` entry, preload list or
 * route table it cannot find is an error naming what was missing, so a change
 * in the bundler's output fails the build loudly instead of reading as a page
 * that ships nothing.
 *
 * @param {Map<string, BuiltFile>} files every built file, by its path under
 *   the build root (`_app/immutable/entry/app.X.js`)
 * @returns {PageWeight[]} one per route, in the route table's order
 */
export function builtPages(files) {
  const entries = [...files.keys()].filter((p) => /^_app\/immutable\/entry\/(start|app)\.[^/]+\.js$/.test(p));
  const app = entries.find((p) => p.includes('/entry/app.'));
  if (!app) throw new Error('the page budget found no _app/immutable/entry/app.*.js in the build');
  const appText = files.get(app)?.text ?? '';
  const deps = /__vite__mapDeps=\(i,m=__vite__mapDeps,d=\(m\.f\|\|\(m\.f=(\[[^\]]*\])\)\)\)/.exec(appText);
  if (!deps) throw new Error(`the page budget found no __vite__mapDeps preload list in ${app}`);
  /** @type {string[]} */
  const list = JSON.parse(deps[1]).map((/** @type {string} */ p) => resolve(app, p));
  /** @type {Map<number, string[]>} */
  const nodes = new Map();
  for (const m of appText.matchAll(/import\([`"']\.\.\/nodes\/(\d+)\.[^`"']*\.js[`"']\),__vite__mapDeps\(\[([\d,]*)\]\)/g)) {
    nodes.set(Number(m[1]), m[2] === '' ? [] : m[2].split(',').map((i) => list[Number(i)]));
  }
  if (!nodes.has(0)) throw new Error(`the page budget found no root layout node (nodes/0) in ${app}`);
  /** @type {[string, number][]} */
  const routes = [...appText.matchAll(/"(\/[^"]*)":\[~?(\d+)/g)].map((m) => [m[1], Number(m[2])]);
  if (routes.length === 0) throw new Error(`the page budget found no route table in ${app}`);
  /** @param {string} path @param {Set<string>} seen */
  const walk = (path, seen) => {
    if (seen.has(path)) return;
    seen.add(path);
    for (const m of (files.get(path)?.text ?? '').matchAll(STATIC_IMPORT)) walk(resolve(path, m[1]), seen);
  };
  /** @type {Set<string>} */
  const base = new Set();
  for (const entry of entries) walk(entry, base);
  for (const p of nodes.get(0) ?? []) base.add(p);
  return routes.map(([route, node]) => {
    const own = nodes.get(node);
    if (!own) throw new Error(`the page budget found no preload list for ${route} (nodes/${node}) in ${app}`);
    const set = new Set(base);
    for (const p of own) set.add(p);
    let js = 0;
    let css = 0;
    for (const p of set) {
      const file = files.get(p);
      if (!file) throw new Error(`the page budget could not find ${p}, which ${route} loads`);
      if (p.endsWith('.css')) css += file.size;
      else js += file.size;
    }
    return { route, node, js, css, bytes: js + css, files: [...set].sort() };
  });
}

/**
 * Each page against its budget and against the wire target.
 *
 * @param {PageWeight[]} pages
 * @returns {{route: string, bytes: number, budget: number, over: boolean, overTarget: boolean, line: string}[]}
 */
export function budgetVerdict(pages) {
  return pages.map((p) => {
    const budget = budgetOf(p.route);
    const over = p.bytes > budget;
    const overTarget = p.bytes > PAGE_TARGETS.wireBytes;
    const kb = (/** @type {number} */ b) => `${(b / 1_000).toFixed(1)} KB`;
    const line =
      `${p.route}: ${kb(p.bytes)} on first load (${kb(p.js)} script, ${kb(p.css)} style), budget ${kb(budget)}` +
      (over ? ' — OVER ITS BUDGET' : '') +
      (overTarget ? ` — over the ${kb(PAGE_TARGETS.wireBytes)} wire target` : '');
    return { route: p.route, bytes: p.bytes, budget, over, overTarget, line };
  });
}

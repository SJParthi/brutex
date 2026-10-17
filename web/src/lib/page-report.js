/**
 * ONE SHARED REPORTER: A PAGE THAT FAILED SAYS SO TO THE SERVER (R3-15; the
 * client half of R3-14's `POST /page-report`, `crates/api/src/page_report_route.rs`).
 *
 * # What was missing
 *
 * A page that threw, a request that failed and a page that fell back to a
 * lesser answer were seen in the operator's browser and nowhere else. The
 * server saw a `200` for the data, the log said nothing, and "the page broke
 * at 14:02" had no record anyone could search. R3-14 gave the server the
 * route; nothing in `web/src` called it.
 *
 * # What is reported, and what is not
 *
 * Three kinds, and nothing else:
 *
 * * `request` — a request the server answered with a FAILURE (a 5xx that is
 *   not busy) or that never got an answer (the network, or `ask`'s ceiling).
 *   The classes are the server's own (`crate::speed::Outcome::of`): an
 *   answer below 400, a refusal (4xx), a busy (429, or 503 with
 *   `Retry-After`) and a request the page itself cancelled are NOT failures —
 *   each is an answer the page already shows, and reporting it would bury
 *   the failures under the ordinary.
 * * `error` — an uncaught error or an unhandled rejection.
 * * `fallback` — a page that degraded to a lesser answer says so through
 *   {@link reportFallback} (or by dispatching {@link FALLBACK_EVENT}).
 *
 * # Rationed here as well as there
 *
 * The server writes at most 30 reports a window for every page together. A
 * page in a loop would spend that on one failure, so this side sends one
 * report per distinct failure per minute and at most
 * {@link REPORTS_PER_MINUTE} a minute; what it holds back is counted and the
 * next report carries the count. The report route itself is never reported
 * (a failing report would report itself), and a report that cannot be sent
 * is written to the console, loudly, once per minute.
 *
 * # Installed once, for every page
 *
 * Importing this module in a browser installs it on `window` (one line in
 * `+layout.svelte`). It wraps `window.fetch`, which every request in this app
 * goes through (`ask` calls it), so no call site changes. An idle tab sends
 * nothing: a report goes only when something failed.
 */

/** The route the server keeps reports on. */
export const REPORT_ROUTE = '/page-report';

/** The server's whole-body ceiling (`page_report_route::MAX_REPORT_BYTES`). */
export const MAX_REPORT_BYTES = 1_024;

/** The server's ceiling on the page path (`page_report_route::MAX_PAGE_BYTES`). */
export const MAX_PAGE_BYTES = 256;

/** At most this many reports leave one tab a minute. */
export const REPORTS_PER_MINUTE = 10;

/** One minute, ms. */
export const MINUTE_MS = 60_000;

/** The event a page dispatches on `window` to report a fallback; `detail` is what it fell back from. */
export const FALLBACK_EVENT = 'brutex:fallback';

/** Marks a window this module is installed on, so a second import adds nothing. */
const INSTALLED = Symbol.for('brutex.page-report');

const encoder = new TextEncoder();

/** Bytes of `s` once form-encoded. @param {string} s */
const encodedBytes = (s) => encoder.encode(encodeURIComponent(s)).length;

/**
 * A control character, which the server refuses in `page` and allows in
 * `what` only as a newline or a tab, becomes a space.
 *
 * @param {string} s
 * @param {boolean} keepLines
 */
function clean(s, keepLines) {
  let out = '';
  for (const c of s) {
    const code = c.codePointAt(0) ?? 0;
    const control = code < 0x20 || (code >= 0x7f && code < 0xa0);
    out += control && !(keepLines && (c === '\n' || c === '\t')) ? ' ' : c;
  }
  return out;
}

/**
 * The form body the server reads, `page=<path>&what=<what failed>`, inside
 * {@link MAX_REPORT_BYTES}: a `what` too long is cut at a character and ends
 * with `…`, so a report is shortened, never refused for its length. `null`
 * for a `page` the server would refuse (empty, not a path, past
 * {@link MAX_PAGE_BYTES}) or an empty `what`, so a report that cannot be
 * written is never sent to be refused.
 *
 * @param {string} page the page's path, `/db`
 * @param {string} what what failed, in the page's words
 * @returns {string | null}
 */
export function reportBody(page, what) {
  const path = clean(String(page ?? ''), false);
  const said = clean(String(what ?? ''), true).trim();
  if (!path.startsWith('/') || encoder.encode(path).length > MAX_PAGE_BYTES || said === '') return null;
  const head = `page=${encodeURIComponent(path)}&what=`;
  const room = MAX_REPORT_BYTES - encoder.encode(head).length;
  if (encodedBytes(said) <= room) return head + encodeURIComponent(said);
  const chars = [...said];
  let lo = 0;
  let hi = chars.length;
  // The longest prefix that fits with its ellipsis: a binary search over at
  // most a few thousand characters, once per report.
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (encodedBytes(chars.slice(0, mid).join('') + '…') <= room) lo = mid;
    else hi = mid - 1;
  }
  return head + encodeURIComponent(chars.slice(0, lo).join('') + '…');
}

/**
 * How one request ended, in the server's five words
 * (`crate::speed::Outcome::of`): `answered` below 400, `busy` for a 429 or a
 * 503 with `Retry-After`, `refused` for another 4xx, `failed` for another
 * 5xx — and, for a request that threw, `cancelled` when the page itself
 * aborted it and `failed` for anything else (the network, `ask`'s ceiling).
 *
 * @param {{status?: number, retryAfter?: boolean, error?: unknown}} ended
 * @returns {'answered' | 'refused' | 'busy' | 'failed' | 'cancelled'}
 */
export function outcomeOf(ended) {
  if (ended.error !== undefined) {
    const name = /** @type {{name?: unknown}} */ (ended.error)?.name;
    return name === 'AbortError' ? 'cancelled' : 'failed';
  }
  const status = Number(ended.status);
  if (status < 400) return 'answered';
  if (status === 429 || (status === 503 && ended.retryAfter)) return 'busy';
  if (status < 500) return 'refused';
  return 'failed';
}

/**
 * The reporter, with its clock and its sender injected so a test drives it.
 *
 * @param {{send: (body: string) => Promise<unknown>, page: () => string, now?: () => number,
 *   perMinute?: number, warn?: (line: string) => void}} options
 */
export function createPageReporter(options) {
  const now = options.now ?? Date.now;
  const perMinute = options.perMinute ?? REPORTS_PER_MINUTE;
  const warn = options.warn ?? ((line) => console.error(line));
  /** @type {Map<string, number>} when each distinct report was last sent */
  const sent = new Map();
  let windowStart = -Infinity;
  let inWindow = 0;
  let heldBack = 0;
  let sendFailures = 0;
  let lastWarn = -Infinity;
  let total = 0;

  /**
   * One report. Returns whether it was sent.
   *
   * @param {'request' | 'error' | 'fallback'} kind
   * @param {string} what
   */
  function report(kind, what) {
    const at = now();
    const page = options.page();
    const key = `${page}\n${kind}\n${what}`;
    if (at - windowStart >= MINUTE_MS) {
      windowStart = at;
      inWindow = 0;
      for (const [k, when] of sent) if (at - when >= MINUTE_MS) sent.delete(k);
    }
    const last = sent.get(key);
    if ((last !== undefined && at - last < MINUTE_MS) || inWindow >= perMinute) {
      heldBack += 1;
      return false;
    }
    const carried = heldBack > 0 ? ` [${heldBack} more held back by this tab's ration before this one]` : '';
    const body = reportBody(page, `${kind}: ${what}${carried}`);
    if (body === null) return false;
    sent.set(key, at);
    inWindow += 1;
    heldBack = 0;
    total += 1;
    void Promise.resolve()
      .then(() => options.send(body))
      .catch((/** @type {unknown} */ why) => {
        sendFailures += 1;
        if (at - lastWarn >= MINUTE_MS) {
          lastWarn = at;
          const said = String(/** @type {{message?: unknown}} */ (why)?.message ?? why);
          warn(`${REPORT_ROUTE} could not take the report "${kind}: ${what}" (${said}); ${sendFailures} reports unsent since this tab opened`);
        }
      });
    return true;
  }

  return {
    report,
    /** What this tab has done, for a test or a page that shows it. */
    counts: () => ({ sent: total, heldBack, sendFailures })
  };
}

/**
 * The URL and method of a `fetch` call, whatever form its arguments took.
 *
 * @param {unknown} input
 * @param {{method?: string} | undefined} init
 */
function requestOf(input, init) {
  const req = /** @type {{url?: unknown, method?: unknown}} */ (input);
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : String(req?.url ?? input);
  const method = String(init?.method ?? req?.method ?? 'GET').toUpperCase();
  return { url, method };
}

/** Whether `url` is the report route, on this origin or written bare. @param {string} url @param {string} origin */
function isReportRoute(url, origin) {
  try {
    return new URL(url, origin).pathname === REPORT_ROUTE;
  } catch {
    return false;
  }
}

/**
 * Installs the reporter on `win`: `fetch` wrapped, and listeners for uncaught
 * errors, unhandled rejections and {@link FALLBACK_EVENT}. A second install
 * on the same window adds nothing and returns a no-op. Returns the uninstall.
 *
 * @param {any} win a `window`, or a test's stand-in with `fetch`,
 *   `location`, `addEventListener` and `removeEventListener`
 * @param {{now?: () => number, perMinute?: number, warn?: (line: string) => void}} [options]
 * @returns {() => void}
 */
export function installPageReport(win, options = {}) {
  if (!win || typeof win.fetch !== 'function' || win[INSTALLED]) return () => {};
  const original = win.fetch.bind(win);
  const origin = String(win.location?.origin ?? 'http://localhost');
  const reporter = createPageReporter({
    ...options,
    page: () => String(win.location?.pathname ?? '/'),
    send: async (body) => {
      const response = await original(REPORT_ROUTE, {
        method: 'POST',
        headers: { 'content-type': 'application/x-www-form-urlencoded' },
        body,
        keepalive: true
      });
      // 202 is written (or counted by the log's own ration); 429 is this
      // route's ration, counted there. Anything else did not keep the report.
      if (response.status !== 202 && response.status !== 429) throw new Error(`HTTP ${response.status}`);
    }
  });

  /** @param {unknown} input @param {any} [init] */
  async function reportingFetch(input, init) {
    const { url, method } = requestOf(input, init);
    const own = isReportRoute(url, origin);
    try {
      const response = await original(input, init);
      if (!own && outcomeOf({ status: response.status, retryAfter: response.headers?.has?.('retry-after') }) === 'failed') {
        reporter.report('request', `${method} ${url} answered HTTP ${response.status}`);
      }
      return response;
    } catch (error) {
      if (!own && outcomeOf({ error }) === 'failed') {
        const said = String(/** @type {{message?: unknown}} */ (error)?.message ?? error);
        reporter.report('request', `${method} ${url} got no answer: ${said}`);
      }
      throw error;
    }
  }

  /** @param {any} event */
  const onError = (event) => {
    const where = event?.filename ? ` at ${event.filename}:${event.lineno ?? '?'}:${event.colno ?? '?'}` : '';
    reporter.report('error', `${String(event?.message ?? event?.error ?? 'an uncaught error')}${where}`);
  };
  /** @param {any} event */
  const onRejection = (event) => {
    const reason = event?.reason;
    reporter.report('error', `unhandled rejection: ${String(reason?.message ?? reason)}`);
  };
  /** @param {any} event */
  const onFallback = (event) => {
    reporter.report('fallback', String(event?.detail ?? 'a fallback with no description'));
  };

  win.fetch = reportingFetch;
  win.addEventListener('error', onError);
  win.addEventListener('unhandledrejection', onRejection);
  win.addEventListener(FALLBACK_EVENT, onFallback);
  win[INSTALLED] = reporter;
  return () => {
    if (win.fetch === reportingFetch) win.fetch = original;
    win.removeEventListener('error', onError);
    win.removeEventListener('unhandledrejection', onRejection);
    win.removeEventListener(FALLBACK_EVENT, onFallback);
    delete win[INSTALLED];
  };
}

/**
 * A page says it fell back to a lesser answer: the report says what it fell
 * back from. Through {@link FALLBACK_EVENT}, so a page needs nothing but this
 * call and the reporter decides whether it is sent.
 *
 * @param {string} what
 * @param {any} [win]
 * @returns {boolean} whether a reporter was there to hear it
 */
export function reportFallback(what, win = typeof window === 'undefined' ? undefined : window) {
  if (!win || !win[INSTALLED] || typeof win.dispatchEvent !== 'function' || typeof CustomEvent !== 'function') return false;
  win.dispatchEvent(new CustomEvent(FALLBACK_EVENT, { detail: what }));
  return true;
}

// INSTALLED BY BEING IMPORTED, in a browser only: `+layout.svelte` imports
// this module once and every page under it reports. Under `node --test` there
// is no `window`, and the tests install it on a stand-in of their own.
if (typeof window !== 'undefined') installPageReport(window);

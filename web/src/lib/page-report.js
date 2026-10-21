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
 * # It never changes the page's own answer
 *
 * The reporter sits in the path of every request, so it must not be able to
 * break one (found by the R3-15 re-check, 20 Oct): a failure text holding a
 * lone UTF-16 surrogate (a string cut between the two halves of an emoji, or
 * a server message decoded badly) made `encodeURIComponent` throw `URIError`,
 * and the wrapped `fetch` REJECTED where the bare one had answered `500`. A
 * lone surrogate is now written as U+FFFD, the replacement character, and
 * every reporting step runs guarded: a step that throws is counted
 * (`counts().faults`) and said on the console at most once a minute, and the
 * page's request answers exactly as it would have without the reporter.
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

/** What a lone UTF-16 surrogate is written as: U+FFFD, the replacement character. */
export const REPLACEMENT = '�';

/**
 * A control character, which the server refuses in `page` and allows in
 * `what` only as a newline or a tab, becomes a space; a lone surrogate, which
 * no encoding can write (`encodeURIComponent` throws `URIError` on it),
 * becomes {@link REPLACEMENT}. Iterating a string by code point yields a
 * PAIRED surrogate as one code point past U+FFFF, so a code point that is
 * itself in U+D800..U+DFFF is exactly a lone half. One pass, the same pass
 * as before.
 *
 * @param {string} s
 * @param {boolean} keepLines
 */
function clean(s, keepLines) {
  let out = '';
  for (const c of s) {
    const code = /** @type {number} */ (c.codePointAt(0));
    const control = code < 0x20 || (code >= 0x7f && code < 0xa0);
    const lone = code >= 0xd800 && code <= 0xdfff;
    out += lone ? REPLACEMENT : control && !(keepLines && (c === '\n' || c === '\t')) ? ' ' : c;
  }
  return out;
}

/**
 * The first `n` code points of `s`, read lazily, and whether anything was
 * left out: a text of any length costs at most `n` steps, never its whole
 * length (the R3-15 re-check, 21 Oct: a 4,000,000-character failure text
 * took 250 ms to clean before it was cut). A string of at most `n` UTF-16
 * units is at most `n` code points and is returned as it is.
 *
 * @param {string} s
 * @param {number} n
 * @returns {{part: string, cut: boolean}}
 */
function head(s, n) {
  if (s.length <= n) return { part: s, cut: false };
  let part = '';
  let taken = 0;
  for (const c of s) {
    if (taken === n) return { part, cut: true };
    part += c;
    taken += 1;
  }
  return { part, cut: false };
}

/**
 * The reporter's clock: `performance.now()`, which only moves forward, where
 * there is one. A wall clock stepped back held every report back until it
 * caught up (the R3-15 re-check, 21 Oct); the ration also starts a new
 * window on a clock that went back, for a clock a caller injects.
 */
const monotonic =
  typeof performance !== 'undefined' && typeof performance.now === 'function' ? () => performance.now() : () => Date.now();

/** Whether `at` is inside the minute that began at `then`; never for a clock that went back. @param {number} at @param {number} then */
const withinMinute = (at, then) => at >= then && at - then < MINUTE_MS;

/** A thrown value's words, never throwing itself. @param {unknown} why */
function wordsOf(why) {
  try {
    return String(/** @type {{message?: unknown}} */ (why)?.message ?? why);
  } catch {
    return 'a value that cannot be written as text';
  }
}

/**
 * The form body the server reads, `page=<path>&what=<what failed>`, inside
 * {@link MAX_REPORT_BYTES}: a `what` too long is cut at a character and ends
 * with `…`, so a report is shortened, never refused for its length. `null`
 * for a `page` the server would refuse (empty, not a path, past
 * {@link MAX_PAGE_BYTES}) or an empty `what`, so a report that cannot be
 * written is never sent to be refused.
 *
 * O(1) in the length of either text: each is cut to one code point past
 * what could ever fit before it is cleaned ({@link head}), and a `what` that
 * was cut always ends with `…`, so a cut is never passed off as the whole.
 *
 * @param {string} page the page's path, `/db`
 * @param {string} what what failed, in the page's words
 * @returns {string | null}
 */
export function reportBody(page, what) {
  const path = clean(head(String(page ?? ''), MAX_PAGE_BYTES + 1).part, false);
  const { part, cut } = head(String(what ?? ''), MAX_REPORT_BYTES + 1);
  const said = clean(part, true).trim();
  if (!path.startsWith('/') || encoder.encode(path).length > MAX_PAGE_BYTES || (said === '' && !cut)) return null;
  const lead = `page=${encodeURIComponent(path)}&what=`;
  const room = MAX_REPORT_BYTES - encoder.encode(lead).length;
  if (!cut && encodedBytes(said) <= room) return lead + encodeURIComponent(said);
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
  return lead + encodeURIComponent(chars.slice(0, lo).join('') + '…');
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
  const now = options.now ?? monotonic;
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
  let faults = 0;
  let lastFault = -Infinity;

  /**
   * A reporting step that threw: counted, and said on the console at most
   * once a minute. Never thrown on, so the page's own request or listener is
   * never changed by its reporter.
   *
   * @param {unknown} why
   */
  function fault(why) {
    faults += 1;
    try {
      const at = now();
      if (withinMinute(at, lastFault)) return;
      lastFault = at;
      warn(`the page reporter could not write a report (${wordsOf(why)}); ${faults} reporting faults since this tab opened`);
    } catch {
      // The clock or the console threw as well: the count is what is kept.
    }
  }

  /**
   * One report. Returns whether it was sent; NEVER THROWS (a step that
   * throws is a {@link fault}, and the answer is `false`).
   *
   * @param {'request' | 'error' | 'fallback'} kind
   * @param {string} what
   */
  function report(kind, what) {
    try {
      return reportOnce(kind, what);
    } catch (why) {
      fault(why);
      return false;
    }
  }

  /**
   * Runs one reporting step, guarded as {@link report} is.
   *
   * @param {() => void} step
   */
  function quietly(step) {
    try {
      step();
    } catch (why) {
      fault(why);
    }
  }

  /**
   * {@link report}'s body, which may throw.
   *
   * @param {'request' | 'error' | 'fallback'} kind
   * @param {string} what
   */
  function reportOnce(kind, what) {
    const at = now();
    const page = options.page();
    // Cut before it is used as a key or a body: the cost of a report never
    // grows with the failure's text (what is past the cut never fits).
    const { part: said } = head(String(what), MAX_REPORT_BYTES + 1);
    const key = `${page}\n${kind}\n${said}`;
    if (!withinMinute(at, windowStart)) {
      windowStart = at;
      inWindow = 0;
      for (const [k, when] of sent) if (!withinMinute(at, when)) sent.delete(k);
    }
    const last = sent.get(key);
    if ((last !== undefined && withinMinute(at, last)) || inWindow >= perMinute) {
      heldBack += 1;
      return false;
    }
    const carried = heldBack > 0 ? ` [${heldBack} more held back by this tab's ration before this one]` : '';
    const body = reportBody(page, `${kind}: ${said}${carried}`);
    if (body === null) return false;
    sent.set(key, at);
    inWindow += 1;
    heldBack = 0;
    total += 1;
    void Promise.resolve()
      .then(() => options.send(body))
      .catch((/** @type {unknown} */ why) => {
        sendFailures += 1;
        if (!withinMinute(at, lastWarn)) {
          lastWarn = at;
          quietly(() => warn(`${REPORT_ROUTE} could not take the report "${kind}: ${said}" (${wordsOf(why)}); ${sendFailures} reports unsent since this tab opened`));
        }
      });
    return true;
  }

  return {
    report,
    quietly,
    /** What this tab has done, for a test or a page that shows it. */
    counts: () => ({ sent: total, heldBack, sendFailures, faults })
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
  // The page's own `fetch`, kept to be put back as it was; called bound to
  // its window, which a browser's `fetch` requires.
  const own = win.fetch;
  const original = own.bind(win);
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

  /**
   * The page's `fetch`, answered exactly as the bare one answers: the same
   * response, or the same error thrown. Reading the request and reporting
   * run guarded ({@link createPageReporter}'s `quietly`), so nothing the
   * reporter does can turn an answer into a rejection.
   *
   * @param {unknown} input @param {any} [init]
   */
  async function reportingFetch(input, init) {
    /** @type {Response} */
    let response;
    try {
      response = await original(input, init);
    } catch (error) {
      reporter.quietly(() => {
        const { url, method } = requestOf(input, init);
        if (!isReportRoute(url, origin) && outcomeOf({ error }) === 'failed') {
          reporter.report('request', `${method} ${url} got no answer: ${wordsOf(error)}`);
        }
      });
      throw error;
    }
    reporter.quietly(() => {
      const { url, method } = requestOf(input, init);
      if (!isReportRoute(url, origin) && outcomeOf({ status: response.status, retryAfter: response.headers?.has?.('retry-after') }) === 'failed') {
        reporter.report('request', `${method} ${url} answered HTTP ${response.status}`);
      }
    });
    return response;
  }

  // Each listener's words are read guarded too: a listener that threw would
  // raise an error event of its own.
  /** @param {any} event */
  const onError = (event) => reporter.quietly(() => {
    const where = event?.filename ? ` at ${event.filename}:${event.lineno ?? '?'}:${event.colno ?? '?'}` : '';
    reporter.report('error', `${String(event?.message ?? event?.error ?? 'an uncaught error')}${where}`);
  });
  /** @param {any} event */
  const onRejection = (event) => reporter.quietly(() => {
    reporter.report('error', `unhandled rejection: ${wordsOf(event?.reason)}`);
  });
  /** @param {any} event */
  const onFallback = (event) => reporter.quietly(() => {
    reporter.report('fallback', String(event?.detail ?? 'a fallback with no description'));
  });

  win.fetch = reportingFetch;
  win.addEventListener('error', onError);
  win.addEventListener('unhandledrejection', onRejection);
  win.addEventListener(FALLBACK_EVENT, onFallback);
  win[INSTALLED] = reporter;
  return () => {
    if (win.fetch === reportingFetch) win.fetch = own;
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

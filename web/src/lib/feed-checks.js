/**
 * THE DATA CHECKS ON /ingest (DATA-1w, D-5190..D-5194): the arithmetic-free
 * half of `routes/ingest/panels/FeedChecks.svelte`, with no runes in it, so
 * `web/tests/feed-checks.test.js` can drive it under `node --test`.
 *
 * Three read-only routes, each paged by `limit` and `offset` with NO default
 * (operator, 16 Oct 2026 17:15 IST, "nothing typed"):
 *
 * - `/feed-checks.json`   DATA-1's feed reconcile, per month, share and day;
 * - `/tick-checks.json`   DATA-2's tick-to-second check, per year, month,
 *                         share and day;
 * - `/calendar-days.json` CAL-1's excluded days, per year and reason.
 *
 * A list is read whole in two asks: one row first, to learn `total`, then
 * `limit=total`. Nothing here picks a page size, and a refusal (a month with
 * no check yet, a damaged file) is shown in the server's own words.
 */

/**
 * A route's address with its fields, each value encoded: a share key such
 * as `M&M` travels as `M%26M`, never as a second field.
 *
 * @param {string} route
 * @param {Record<string, string | number | null | undefined>} fields
 */
export function address(route, fields) {
  const parts = [];
  for (const [name, value] of Object.entries(fields)) {
    if (value === null || value === undefined || value === '') continue;
    parts.push(`${name}=${encodeURIComponent(String(value))}`);
  }
  return parts.length ? `${route}?${parts.join('&')}` : route;
}

/**
 * Reads one answer: `{ ok, status, body, error }`, the error in the server's
 * words when it refused.
 *
 * @param {(url: string) => Promise<Response>} ask
 * @param {string} url
 */
export async function read(ask, url) {
  const r = await ask(url);
  const text = await r.text();
  /** @type {any} */
  let body = null;
  try {
    body = JSON.parse(text);
  } catch {
    body = null;
  }
  if (!r.ok) {
    return { ok: false, status: r.status, body, error: `GET ${url} answered ${r.status}: ${body?.error ?? text.slice(0, 300)}` };
  }
  if (body === null) return { ok: false, status: r.status, body, error: `GET ${url} answered ${r.status} with no JSON` };
  return { ok: true, status: r.status, body, error: null };
}

/**
 * A whole list: one row first to learn its `total`, then every row at once.
 *
 * @param {(url: string) => Promise<Response>} ask
 * @param {string} route
 * @param {Record<string, string | number | null | undefined>} fields
 */
export async function readAll(ask, route, fields) {
  const first = await read(ask, address(route, { ...fields, limit: 1, offset: 0 }));
  if (!first.ok) return first;
  const total = Number(first.body?.page?.total);
  if (!Number.isInteger(total) || total < 1) {
    return { ok: false, status: first.status, body: first.body, error: `GET ${route} named no total` };
  }
  if (total === 1) return first;
  return read(ask, address(route, { ...fields, limit: total, offset: 0 }));
}

/**
 * A share, exactly: `n of d` and its percentage, or a dash when nothing was
 * compared. The fraction is the server's; only the percentage is computed.
 *
 * @param {number | null | undefined} n
 * @param {number | null | undefined} of
 */
export function share(n, of) {
  if (typeof n !== 'number' || typeof of !== 'number' || of <= 0) return '–';
  return `${((100 * n) / of).toFixed(2)} % (${n} of ${of})`;
}

/**
 * A `{ word: count }` object as one line, largest first.
 *
 * @param {Record<string, number> | null | undefined} counts
 */
export function words(counts) {
  if (!counts || typeof counts !== 'object') return '–';
  const entries = Object.entries(counts).filter(([, n]) => typeof n === 'number' && n > 0);
  if (entries.length === 0) return '–';
  entries.sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  return entries.map(([w, n]) => `${w} ${n}`).join(', ');
}

/**
 * The per-month comparison table: every month either check holds, newest
 * first, the feed reconcile's counts beside the tick check's.
 *
 * @param {any[]} feedMonths `months` of `/feed-checks.json`
 * @param {Record<string, any>} tickMonths `YYYY-MM` -> a month of `/tick-checks.json?year=`
 */
export function monthRows(feedMonths, tickMonths) {
  /** @type {Map<string, any>} */
  const rows = new Map();
  for (const m of feedMonths ?? []) {
    if (!m || typeof m.month !== 'string') continue;
    rows.set(m.month, { month: m.month, feed: m, tick: null });
  }
  for (const [month, t] of Object.entries(tickMonths ?? {})) {
    const row = rows.get(month) ?? { month, feed: null, tick: null };
    row.tick = t;
    rows.set(month, row);
  }
  return [...rows.values()]
    .sort((a, b) => b.month.localeCompare(a.month))
    .map((r) => {
      const c = r.feed?.counts ?? null;
      const t = r.tick?.counts ?? null;
      return {
        month: r.month,
        feedError: r.feed?.error ?? null,
        version: r.feed?.version ?? null,
        stocks: r.feed?.stocks ?? null,
        stockDays: c?.stock_days ?? null,
        byStatus: c?.by_status ?? null,
        adjustedStocks: r.feed?.adjusted_stocks ?? null,
        exDateDays: c?.ex_date_days ?? null,
        shortDays: c?.zerodha_minute_short_days ?? null,
        tickShareDays: t?.share_days ?? null,
        ruleT: t ? share(t.rule_t_exact?.share_days, t.rule_t_exact?.of) : '–',
        ruleAHigh: t ? share(t.minute_match?.a?.high_match, t.minute_match?.a?.compared) : '–'
      };
    });
}

/**
 * The tick check's months of one `/tick-checks.json?year=` answer, by month.
 *
 * @param {any} body
 */
export function tickMonthsOf(body) {
  /** @type {Record<string, any>} */
  const out = {};
  for (const m of body?.months ?? []) {
    if (m && typeof m.month === 'string') out[m.month] = m;
  }
  return out;
}

/**
 * The years a list of months spans, each once, newest first.
 *
 * @param {string[]} months
 */
export function yearsOf(months) {
  const years = new Set();
  for (const m of months) {
    if (/^\d{4}-\d{2}$/.test(m)) years.add(m.slice(0, 4));
  }
  return [...years].sort().reverse();
}

/**
 * One share's rows of a month from both checks, joined by share.
 *
 * @param {any[]} feedStocks `stocks` of `/feed-checks.json?month=`
 * @param {any[]} tickStocks `stocks` of `/tick-checks.json?month=`
 */
export function stockRows(feedStocks, tickStocks) {
  /** @type {Map<string, any>} */
  const rows = new Map();
  for (const s of feedStocks ?? []) {
    if (s && typeof s.symbol === 'string') rows.set(s.symbol, { symbol: s.symbol, feed: s, tick: null });
  }
  for (const s of tickStocks ?? []) {
    if (!s || typeof s.symbol !== 'string') continue;
    const row = rows.get(s.symbol) ?? { symbol: s.symbol, feed: null, tick: null };
    row.tick = s;
    rows.set(s.symbol, row);
  }
  return [...rows.values()].sort((a, b) => a.symbol.localeCompare(b.symbol));
}

/**
 * The excluded-days-per-year table of `/calendar-days.json`: one row per
 * year, every reason a column, a refused year with its words.
 *
 * @param {any[]} years
 */
export function calendarRows(years) {
  const reasons = new Set();
  for (const y of years ?? []) for (const r of Object.keys(y?.by_reason ?? {})) if (r !== 'Weekend') reasons.add(r);
  const columns = [...reasons].sort();
  const rows = (years ?? []).map((y) => ({
    year: y?.year,
    refused: y?.refused ?? null,
    tradable: y?.tradable ?? null,
    excluded: y?.excluded_sessions ?? null,
    weekends: y?.weekends ?? null,
    unexplained: y?.unexplained ?? null,
    holidays: y?.holidays?.record === 'circular' ? y.holidays.circular : (y?.holidays?.record ?? null),
    counts: columns.map((r) => y?.by_reason?.[r] ?? 0)
  }));
  return { columns, rows };
}

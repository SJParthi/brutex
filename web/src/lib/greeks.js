import { ask } from './ask.js';
import { parseKey } from './instrument.js';
import { ownMonth } from './moneyness.js';

/**
 * IMPLIED VOLATILITY AND GREEKS FOR ONE OPTION BAR — D-2830.
 *
 * Every figure here is the SERVER'S. `GET /greeks.json` solves each bar's
 * volatility in Rust (`pull::pricing::bar_greeks`, which is `greeks::bsm`)
 * from the bar's own stored close, the underlying's stored close at the same
 * instant, the time to the expiry day's session close (D-2831) and the RBI
 * 91-day T-bill rate as of the latest release before the bar's day (D-2810).
 * This module computes no model and converts no unit: it groups the page's
 * rows into requests, files the answers, and formats numbers for reading.
 * A second Black-Scholes in the browser would be a second authority for one
 * fact, which this repository refuses (the `/vocab.json` reasoning, CLAUDE.md
 * §5).
 *
 * A cell with no figure carries the server's reason word, and `GREEK_WHY`
 * says it as a sentence. Nothing is blank and nothing is guessed.
 *
 * THE MODEL'S LIMITS ARE CARRIED, NOT HIDDEN (D-3330, DB-038, DB-G64). A
 * priced cell keeps the server's flags — `below_band` (a tenor under the band
 * the greeks crate validates itself in) and `above_rate_tenor` (a contract
 * longer than the 91-day rate's own maturity) — and its tenor in years; a
 * premium at or below the zero-carry discounted intrinsic is refused as
 * `below_zero_carry_bound`, a limit of the model's inputs, apart from
 * `iv_unsolved`.
 */

/** At most this many instants per `/greeks.json` request (the server's cap). */
export const GREEK_CHUNK = 400;

/** Why a greek cell has no figure, one sentence per reason the page can meet. */
export const GREEK_WHY = {
  rate_unset:
    'no risk-free rate table is configured: the server’s BRUTEX_TBILL91_SERIES is not set, and no rate is guessed in its place.',
  rate_unreadable: 'the configured risk-free rate table would not read; the server named the fault.',
  rate_before_series:
    'the rate table has no T-bill result released before this bar’s day, so no rate applies and none earlier is invented.',
  rate_stale:
    'the latest T-bill result before this bar’s day is more than 14 days old, so an auction is missing from the table and no rate is carried over the gap.',
  tenor_refused:
    'no time to expiry could be measured for this bar — it is at or past the expiry day’s session close, or that day’s hours are not verified.',
  iv_unsolved: 'no volatility reproduces this bar’s close; the model’s own reason is below.',
  below_zero_carry_bound:
    'the premium is at or below the intrinsic value discounted at the rate with ZERO carry (no dividend or carry is sourced, D-2830), so no volatility reproduces it under this model’s inputs. A limit of the model’s inputs, not a verdict on the print (D-3330).',
  no_option_bar: 'the option has no stored bar stamped at this instant.',
  no_option_file: 'the option’s month file is not in this feed’s store.',
  option_unreadable: 'the option’s bar for this instant would not read; the server named the fault.',
  option_order_broken:
    'the option’s file is not strictly increasing around this instant, so its close cannot be stated without a scan.',
  no_spot_bar:
    'the underlying has no stored bar stamped at this bar’s own instant, so there is no spot to price against — no nearest or earlier close is used unless “last spot at or before t” is opted in.',
  no_spot_in_session:
    'the underlying has no stored bar at or before this instant in this bar’s IST session, so even the opted-in earlier spot does not exist; a later bar is never used (D-3328).',
  t_outside_month:
    'the instant was asked against a month it does not lie in, so the server refused the question rather than call the option’s file empty (D-3328). Each bar is asked in its own IST month; this is a defect in the asking, not a fact about the store.',
  t_off_grid:
    'the instant is not on this rung’s grid, so the spot bar before it may close after it; no earlier spot is taken for it (D-3328).',
  no_spot_file: 'the underlying’s month file at this rung is not in this feed’s store, so there is no spot to price against.',
  spot_unreadable: 'the underlying’s bar for this instant would not read; the server named the fault.',
  spot_order_broken:
    'the underlying’s file is not strictly increasing around this instant, so its close cannot be stated without a scan.',
  greeks_request_failed: 'the greeks request failed, so this bar’s greeks are unknown.',
  greeks_pending: 'this bar’s greeks have not been read yet.'
};

/**
 * What a priced cell is flagged for, one sentence each (D-3330). A flag does
 * not withhold the figure; it says the figure is outside what the model's
 * inputs were sourced or validated for.
 */
export const GREEK_FLAG = {
  below_band:
    'the time to expiry is below the band the greeks crate validates itself in: near expiry the solved volatility and theta run to extremes, so these figures are shown flagged, not as ordinary ones.',
  above_rate_tenor:
    'the contract runs longer than 91 days, the risk-free rate’s own maturity (RBI 91-day T-bill, D-2810): the rate is shorter than the tenor it discounts over, and carry is zero because none is sourced (D-2830).'
};

/**
 * The lookup key one bar row's greeks are filed under: its contract, rung,
 * OWN month and second.
 * @param {{instrument: string, tf: string, ts: number, month?: string}} row
 */
export const greekKey = (row) => `${row.instrument}|${row.tf}|${ownMonth(row)}|${Math.floor(row.ts / 1000)}`;

/**
 * @typedef {object} GreekCell
 * @property {number|null} iv     decimal, the server's
 * @property {number|null} delta
 * @property {number|null} gamma  per rupee of spot
 * @property {number|null} theta  rupees per calendar day
 * @property {number|null} vega   rupees per volatility point
 * @property {number|null} rho    rupees per rate point
 * @property {string|null} why    the server's reason word, or null when priced
 * @property {string|null} detail the server's own sentence, when it sent one
 * @property {Record<string, any>|null} row the server's row, for the title
 * @property {string[]} flags  `below_band`, `above_rate_tenor` where the server set them
 * @property {number|null} tenorY 365-day years to the settled expiry's close
 * @property {number|null} age  seconds from the spot used to this bar, 0 for exact
 */

/** @param {unknown} v */
const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : null);

/**
 * One server row as a cell. A row the server priced has every figure; any
 * other row has none and carries its reason.
 * @param {Record<string, any>} s
 * @param {string|null} faults
 * @returns {GreekCell}
 */
export function cellOf(s, faults) {
  const iv = num(s.iv);
  /** @type {string[]} */
  const flags = [];
  if (s.below_band === true) flags.push('below_band');
  if (s.above_rate_tenor === true) flags.push('above_rate_tenor');
  const said = { flags, tenorY: num(s.tenor_y), age: num(s.spot_age_s) };
  if (s.why === null && iv !== null) {
    return {
      iv,
      delta: num(s.delta),
      gamma: num(s.gamma),
      theta: num(s.theta),
      vega: num(s.vega),
      rho: num(s.rho),
      why: null,
      detail: null,
      row: s,
      ...said
    };
  }
  const detail = [s.detail, faults].filter((x) => typeof x === 'string' && x).join(' — ');
  return {
    iv: null,
    delta: null,
    gamma: null,
    theta: null,
    vega: null,
    rho: null,
    why: String(s.why ?? 'greeks_request_failed'),
    detail: detail || null,
    row: s,
    ...said
  };
}

/**
 * The cell for one bar row, `null` for a row that is not an option.
 * @param {{side: string|null}} row
 * @param {GreekCell|undefined} cell
 * @returns {GreekCell|null}
 */
export function greekCell(row, cell) {
  if (!row.side) return null;
  return (
    cell ?? {
      iv: null,
      delta: null,
      gamma: null,
      theta: null,
      vega: null,
      rho: null,
      why: 'greeks_pending',
      detail: null,
      row: null,
      flags: [],
      tenorY: null,
      age: null
    }
  );
}

/**
 * A figure for reading: `digits` places, or `—` for none. Presentation only;
 * the value is the server's.
 * @param {number|null} v @param {number} digits
 */
export const greekText = (v, digits) => (v === null ? '—' : v.toFixed(digits));

/** The implied volatility as a percentage, two places. @param {number|null} iv */
export const ivText = (iv) => (iv === null ? '—' : (iv * 100).toFixed(2));

/**
 * A tenor for reading, in 365-day years (D-3330, DB-G64): years to two places
 * from one day up, days under that, seconds under a minute. The server's
 * `tenor_s` stays the figure; this is how it is said.
 * @param {number|null} years @param {number|null} [seconds]
 */
export function tenorText(years, seconds = null) {
  if (years === null || !Number.isFinite(years)) return seconds === null ? '—' : `${seconds} s`;
  const days = years * 365;
  if (days >= 365) return `${years.toFixed(2)} y`;
  if (days >= 1) return `${years.toFixed(3)} y (${days.toFixed(1)} d)`;
  const s = seconds ?? Math.round(days * 86_400);
  return s < 60 ? `${s} s` : `${(s / 3600).toFixed(2)} h`;
}

/**
 * The sentence for a cell's reason, and the server's own sentence after it.
 * @param {GreekCell} cell
 */
export const greekWhyText = (cell) =>
  `${/** @type {Record<string, string>} */ (GREEK_WHY)[cell.why ?? ''] ?? `unknown (${cell.why})`}${
    cell.detail ? ` — ${cell.detail}` : ''
  }`;

/**
 * Reads the greeks for every option row given — one request per contract,
 * rung and OWN month, chunked at {@link GREEK_CHUNK} — and files each answer
 * under {@link greekKey}. A failed request files `greeks_request_failed` for
 * each of its instants and keeps the server's own sentence in `detail`.
 *
 * @param {string} feed
 * @param {{instrument: string, tf: string, ts: number, side: string|null, month?: string}[]} rows
 * @param {AbortSignal} signal
 * @param {typeof ask} [request]
 * @param {'exact'|'prev'} [asof] `prev` opts in to the last spot at or before t (D-3328)
 * @returns {Promise<{cells: Map<string, GreekCell>, rate: Record<string, any>|null}>}
 */
export async function readGreeks(feed, rows, signal, request = ask, asof = 'exact') {
  /** @type {Map<string, {instrument: string, tf: string, month: string, times: Set<number>}>} */
  const groups = new Map();
  for (const r of rows) {
    if (!r.side) continue;
    const month = ownMonth(r);
    const g = `${r.instrument}|${r.tf}|${month}`;
    let e = groups.get(g);
    if (!e) groups.set(g, (e = { instrument: r.instrument, tf: r.tf, month, times: new Set() }));
    e.times.add(Math.floor(r.ts / 1000));
  }
  /** @type {Map<string, GreekCell>} */
  const cells = new Map();
  /** @type {Record<string, any>|null} */
  let rate = null;
  for (const [g, e] of groups) {
    const p = parseKey(e.instrument);
    const times = [...e.times].sort((a, b) => a - b);
    for (let i = 0; i < times.length; i += GREEK_CHUNK) {
      signal.throwIfAborted();
      const chunk = times.slice(i, i + GREEK_CHUNK);
      const q = new URLSearchParams({
        feed,
        exchange: String(p.exchange),
        segment: String(p.segment),
        symbol: String(p.underlying),
        contract: `${p.expiry}-${p.strike}-${p.side}`,
        timeframe: e.tf,
        month: e.month,
        t: chunk.join(',')
      });
      if (asof === 'prev') q.set('asof', 'prev');
      try {
        const res = await request(`/greeks.json?${q}`, { signal });
        const body = await res.json();
        if (!Array.isArray(body?.rows) || body.rows.length !== chunk.length) {
          throw new Error(body?.error ?? `HTTP ${res.status}`);
        }
        if (body.rate && typeof body.rate === 'object') rate = body.rate;
        const faults = typeof body.faults === 'string' ? body.faults : null;
        for (const s of body.rows) cells.set(`${g}|${s.t}`, cellOf(s, faults));
      } catch (why) {
        signal.throwIfAborted();
        const detail = String(why instanceof Error ? why.message : why);
        for (const t of chunk) {
          cells.set(`${g}|${t}`, cellOf({ why: 'greeks_request_failed', detail }, null));
        }
      }
    }
  }
  return { cells, rate };
}

import { ask } from './ask.js';
import { parseKey } from './instrument.js';

/**
 * MONEYNESS, INTRINSIC AND EXTRINSIC FOR ONE OPTION BAR — D-2807.
 *
 * Spot is the UNDERLYING's stored close at the option bar's OWN instant, on
 * the same feed and rung, read through `GET /spot.json`. The server answers a
 * close only for a bar stamped at exactly that instant and otherwise names the
 * absence (`no_spot_bar`, `no_spot_file`, `spot_unreadable`,
 * `spot_order_broken`); this module never substitutes a nearest or an earlier
 * close, so a missing spot is a named cell and never a stale number.
 *
 * Rungs are counted along the strikes the STORE LISTS for that underlying and
 * expiry (the same `rungOf` convention `/db`'s ladder uses): ATM is the listed
 * strike nearest spot, the lower of two equidistant ones; ITM-k / OTM+k is k
 * listed strikes away. No strike step is typed anywhere — the only step used
 * is the smallest gap between listed strikes, and only to refuse naming a rung
 * when spot sits beyond the chain's ends.
 *
 * Money stays integer paisa throughout (`CLAUDE.md` §7).
 */

/** At most this many instants per `/spot.json` request (the server's cap). */
export const SPOT_CHUNK = 400;

/** Why a cell has no figure, one sentence per reason the page can meet. */
export const SPOT_WHY = {
  no_spot_bar:
    'the underlying has no stored bar stamped at this bar’s own instant on this feed and rung, so there is no spot to measure from — no nearest or earlier close is used in its place.',
  no_spot_file:
    'the underlying’s month file at this rung is not in this feed’s store, so there is no spot to measure from.',
  spot_unreadable:
    'the underlying’s bar for this instant would not read; the server named the fault.',
  spot_order_broken:
    'the underlying’s file is not strictly increasing around this instant, so neither its close nor its absence can be stated without a scan.',
  spot_request_failed: 'the spot request failed, so this bar’s spot is unknown.',
  spot_pending: 'the underlying’s spot for this bar has not been read yet.',
  one_strike_listed:
    'the store lists one strike for this underlying and expiry, and a rung is a count of strikes from the money — one strike is no ladder.',
  strike_not_listed: 'this strike is not among the strikes the store lists for this expiry.',
  spot_outside_chain:
    'spot sits beyond the listed strikes by more than half their smallest gap, so the nearest listed strike is not the money.'
};

/**
 * The chain key for one underlying and expiry.
 * @param {string|null} exchange @param {string|null} underlying @param {string|null} expiry
 */
export const chainKey = (exchange, underlying, expiry) => `${exchange}-${underlying}|${expiry}`;

/**
 * Every listed strike per underlying and expiry, ascending and distinct, from
 * the census rows (`instrument` keys). Non-option keys are skipped.
 * @param {{instrument: string}[]} rows
 * @returns {Map<string, number[]>}
 */
export function chainsFrom(rows) {
  /** @type {Map<string, Set<number>>} */
  const sets = new Map();
  for (const r of rows) {
    const p = parseKey(r.instrument);
    if (p.kind !== 'option' || p.strike === null) continue;
    const k = chainKey(p.exchange, p.underlying, p.expiry);
    let s = sets.get(k);
    if (!s) sets.set(k, (s = new Set()));
    s.add(p.strike);
  }
  /** @type {Map<string, number[]>} */
  const out = new Map();
  for (const [k, s] of sets) out.set(k, [...s].sort((a, b) => a - b));
  return out;
}

/**
 * The rung name for an offset of `k` listed strikes from the money.
 * @param {number} k @param {'CE'|'PE'} side
 */
export const rungOf = (k, side) =>
  k === 0 ? 'ATM' : ((side === 'CE') === (k < 0) ? 'ITM-' : 'OTM+') + Math.abs(k);

/**
 * The rung one option sits on, or the reason it has none.
 * @param {number[]} strikes ascending, distinct
 * @param {number} strike @param {'CE'|'PE'} side @param {number} spot paisa
 * @returns {{rung: string|null, why: string|null}}
 */
export function moneyness(strikes, strike, side, spot) {
  const at = strikes.indexOf(strike);
  if (at === -1) return { rung: null, why: 'strike_not_listed' };
  if (strikes.length < 2) return { rung: null, why: 'one_strike_listed' };
  let step = Infinity;
  for (let i = 1; i < strikes.length; i += 1) step = Math.min(step, strikes[i] - strikes[i - 1]);
  if (2 * (strikes[0] - spot) > step || 2 * (spot - strikes[strikes.length - 1]) > step) {
    return { rung: null, why: 'spot_outside_chain' };
  }
  let money = 0;
  for (let i = 1; i < strikes.length; i += 1) {
    if (Math.abs(strikes[i] - spot) < Math.abs(strikes[money] - spot)) money = i;
  }
  return { rung: rungOf(at - money, side), why: null };
}

/**
 * Intrinsic value in paisa: spot over strike for a call, strike over spot for
 * a put, never below zero.
 * @param {number} strike @param {'CE'|'PE'} side @param {number} spot
 */
export const intrinsic = (strike, side, spot) =>
  Math.max(0, side === 'CE' ? spot - strike : strike - spot);

/**
 * The three cells for one bar row.
 *
 * `extr` is the premium (the option bar's close) less intrinsic, and it is NOT
 * clipped: a negative figure is a stored close below intrinsic, which is a
 * fact about the bar, not an arithmetic error to hide.
 *
 * @param {{instrument: string, strike: number|null, side: 'CE'|'PE'|null, c: number}} row
 * @param {{close: number|null, why: string|null, detail?: string} | undefined} spot
 * @param {Map<string, number[]>} chains
 */
export function optionCells(row, spot, chains) {
  if (row.side === null || row.strike === null) return null;
  if (!spot) {
    return { mny: null, mnyWhy: 'spot_pending', intr: null, extr: null, why: 'spot_pending', spot: null, detail: null };
  }
  const detail = spot.detail ?? null;
  if (spot.close === null) {
    const why = spot.why ?? 'spot_request_failed';
    return { mny: null, mnyWhy: why, intr: null, extr: null, why, spot: null, detail };
  }
  const p = parseKey(row.instrument);
  const strikes = chains.get(chainKey(p.exchange, p.underlying, p.expiry)) ?? [];
  const m = moneyness(strikes, row.strike, row.side, spot.close);
  const intr = intrinsic(row.strike, row.side, spot.close);
  return { mny: m.rung, mnyWhy: m.why, intr, extr: row.c - intr, why: null, spot: spot.close, detail };
}

/**
 * The lookup key one bar row's spot is filed under.
 * @param {{instrument: string, tf: string, month: string, ts: number}} row
 */
export function spotKey(row) {
  const p = parseKey(row.instrument);
  return `${p.exchange}-${p.underlying}|${row.tf}|${row.month}|${Math.floor(row.ts / 1000)}`;
}

/**
 * Reads the spot for every option row given, one request per underlying,
 * rung and month (chunked at {@link SPOT_CHUNK}), and files each answer under
 * {@link spotKey}. A failed request files `spot_request_failed` for each of
 * its instants and keeps the server's own sentence in `detail`.
 *
 * @param {string} feed
 * @param {{instrument: string, tf: string, month: string, ts: number, side: string|null}[]} rows
 * @param {AbortSignal} signal
 * @param {typeof ask} [request]
 * @returns {Promise<Map<string, {close: number|null, why: string|null, detail?: string}>>}
 */
export async function readSpots(feed, rows, signal, request = ask) {
  /** @type {Map<string, {exchange: string, symbol: string, tf: string, month: string, times: Set<number>}>} */
  const groups = new Map();
  for (const r of rows) {
    if (!r.side) continue;
    const p = parseKey(r.instrument);
    const g = `${p.exchange}-${p.underlying}|${r.tf}|${r.month}`;
    let e = groups.get(g);
    if (!e) {
      e = { exchange: String(p.exchange), symbol: String(p.underlying), tf: r.tf, month: r.month, times: new Set() };
      groups.set(g, e);
    }
    e.times.add(Math.floor(r.ts / 1000));
  }
  /** @type {Map<string, {close: number|null, why: string|null, detail?: string}>} */
  const out = new Map();
  for (const [g, e] of groups) {
    const times = [...e.times].sort((a, b) => a - b);
    for (let i = 0; i < times.length; i += SPOT_CHUNK) {
      signal.throwIfAborted();
      const chunk = times.slice(i, i + SPOT_CHUNK);
      const q = new URLSearchParams({ feed, exchange: e.exchange, symbol: e.symbol,
        timeframe: e.tf, month: e.month, t: chunk.join(',') });
      try {
        const res = await request(`/spot.json?${q}`, { signal });
        const body = await res.json();
        if (!Array.isArray(body?.spots) || body.spots.length !== chunk.length) {
          throw new Error(body?.error ?? `HTTP ${res.status}`);
        }
        for (const s of body.spots) {
          out.set(`${g}|${s.t}`, {
            close: typeof s.close === 'number' ? s.close : null,
            why: typeof s.close === 'number' ? null : String(s.why),
            ...(body.faults ? { detail: String(body.faults) } : {})
          });
        }
      } catch (why) {
        signal.throwIfAborted();
        const detail = String(why instanceof Error ? why.message : why);
        for (const t of chunk) out.set(`${g}|${t}`, { close: null, why: 'spot_request_failed', detail });
      }
    }
  }
  return out;
}

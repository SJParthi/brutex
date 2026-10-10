import { ask } from './ask.js';
import { istMonth } from './dates.js';
import { parseKey } from './instrument.js';

/**
 * MONEYNESS, INTRINSIC AND EXTRINSIC FOR ONE OPTION BAR — D-2807, D-3328,
 * D-3329.
 *
 * EVERY BAR IS ASKED IN ITS OWN MONTH (D-3328 item 3, DB-009). The month a
 * spot is read from is the IST month of the bar's own stamp, worked out here
 * from the stamp and never taken from whatever month a row was filed under:
 * a page sorted by close over 2018-09..2019-03 holds bars of several months,
 * and asking all of them in the plan's newest month answered every 2018 bar
 * with a false absence. The server refuses such an instant by name
 * (`t_outside_month`), so a slip here is loud, not blank.
 *
 * SPOT is the UNDERLYING's stored close on the same feed and rung, through
 * `GET /spot.json`. By default only the bar stamped at exactly this instant
 * (D-2807); opted in (`asof=prev`, D-3328), the underlying's last bar at or
 * before it in the same IST session, NEVER a later one, and always with the
 * stamp it came from and its age. Every absence is named by the server
 * (`no_spot_bar`, `no_spot_in_session`, `no_spot_file`, ...) and drawn as its
 * reason, never as a nearest close.
 *
 * THE RUNG IS THE SERVER'S, AT THE BAR'S OWN INSTANT (D-3329; DB-008, DB-037,
 * DB-G04, DB-G62, DB-G73). `GET /chain.json` counts it over the strikes of
 * this underlying and expiry STORED BY THEN — no strike first stored after the
 * bar, no other expiry's strike, no other underlying's — and refuses by name
 * where a rung would be a guess (`ladder_too_sparse`, `census_behind`, ...).
 * This module computes no chain and counts no rung: it groups the page's rows
 * into requests and files the answers. Until D-3329 it folded one ladder per
 * expiry out of the whole census, which put a 2018 bar on a ladder of strikes
 * first stored in 2019 (OTM+5 for what was OTM+1 at the time).
 *
 * Money stays integer paisa throughout (`CLAUDE.md` §7).
 */

/** At most this many instants per `/spot.json` request (the server's cap). */
export const SPOT_CHUNK = 400;

/** At most this many instants per `/chain.json` request for ONE contract
 *  (`strike` and `side` named): the server's `MAX_CHAIN_TIMES`. */
export const RUNG_CHUNK = 400;

/** At most this many instants per `/chain.json` request listing EVERY strike:
 *  the server's `MAX_FULL_CHAINS`. */
export const LADDER_CHUNK = 8;

/** The two ways a spot is taken (D-3328). `exact` is the default. */
export const SPOT_ASOF = /** @type {const} */ (['exact', 'prev']);

/** Why a cell has no spot, one sentence per reason the page can meet. */
export const SPOT_WHY = {
  no_spot_bar:
    'the underlying has no stored bar stamped at this bar’s own instant on this feed and rung, so there is no spot to measure from — no nearest or earlier close is used unless “last spot at or before t” is opted in.',
  no_spot_in_session:
    'the underlying has no stored bar at or before this instant in this bar’s IST session, so even the opted-in earlier spot does not exist; a later bar is never used (D-3328).',
  no_spot_file:
    'the underlying’s month file at this rung is not in this feed’s store, so there is no spot to measure from.',
  spot_unreadable: 'the underlying’s bar for this instant would not read; the server named the fault.',
  spot_order_broken:
    'the underlying’s file is not strictly increasing around this instant, so neither its close nor its absence can be stated without a scan.',
  t_outside_month:
    'the instant was asked against a month it does not lie in, so the server refused the question rather than call the file empty (D-3328). Each bar is asked in its own IST month; this is a defect in the asking, not a fact about the store.',
  t_off_grid:
    'the instant is not on this rung’s grid, so the bar before it may close after it; no earlier spot is taken for it (D-3328).',
  spot_request_failed: 'the spot request failed, so this bar’s spot is unknown.',
  spot_pending: 'the underlying’s spot for this bar has not been read yet.'
};

/**
 * Why a cell has no rung, one sentence per reason `/chain.json` can give
 * (D-3329). A spot word (`no_spot_bar`, ...) is also a rung refusal — no
 * spot, no money to count from — and reads from {@link SPOT_WHY}.
 */
export const RUNG_WHY = {
  census_absent: 'this feed has no census to count stored strikes from, so no rung is counted.',
  census_state_unknown:
    'the import journal that says whether the census is behind would not read, so whether every strike stored by this instant is counted is unknown — no rung is named on a ladder that may be short.',
  census_degraded:
    'the census loaded only by stepping over damage: it is a recovered, earlier generation, so strikes a later one counted may be missing and a ladder over it could be truncated anywhere.',
  census_behind:
    'the census is behind its import from a day on or before this bar and this expiry: strikes first stored from then are not counted yet, so the ladder at this instant would be truncated (D-3327). The rung appears once the census catches up.',
  expiry_not_in_census: 'the census holds no option of this underlying and expiry, so there is no chain to count along.',
  no_strike_stored_by_then: 'no strike of this expiry has a bar stored at or before this instant, so there is no chain yet.',
  ladder_too_sparse:
    'the nearest strike stored by this instant is further from spot than the chain’s local step (or only one strike is stored), so the nearest strike is not the money and no rung is named.',
  strike_not_stored_by_then:
    'this strike has no bar stored at or before this instant, so it is not on the chain the rung is counted along.',
  rung_request_failed: 'the chain request failed, so this bar’s rung is unknown.',
  rung_pending: 'this bar’s rung has not been read yet.'
};

/**
 * The IST month (`YYYY-MM`) of a bar's OWN stamp — the month its spot,
 * greeks and rung are asked in (DB-009). `row.ts` is epoch milliseconds.
 * @param {{ts: number, month?: string}} row
 */
export const ownMonth = (row) => istMonth(row.ts) || String(row.month ?? '');

/**
 * The chain key for one underlying and expiry.
 * @param {string|null} exchange @param {string|null} underlying @param {string|null} expiry
 */
export const chainKey = (exchange, underlying, expiry) => `${exchange}-${underlying}|${expiry}`;

/**
 * The rung name for an offset of `k` stored strikes from the money. The
 * server names every rung itself; this is the same convention, kept for the
 * Moneyness control's list (`$lib/db/chain.js` reads the names back).
 * @param {number} k @param {'CE'|'PE'} side
 */
export const rungOf = (k, side) =>
  k === 0 ? 'ATM' : ((side === 'CE') === (k < 0) ? 'ITM-' : 'OTM+') + Math.abs(k);

/**
 * Intrinsic value in paisa: spot over strike for a call, strike over spot for
 * a put, never below zero.
 * @param {number} strike @param {'CE'|'PE'} side @param {number} spot
 */
export const intrinsic = (strike, side, spot) =>
  Math.max(0, side === 'CE' ? spot - strike : strike - spot);

/**
 * @typedef {object} SpotAnswer
 * @property {number|null} close  paisa, the underlying's close used
 * @property {string|null} why    the server's reason word, or null with a close
 * @property {number|null} spotT  unix seconds of the bar the close came from
 * @property {number|null} age    seconds from that bar to this one, 0 for an exact hit
 * @property {string} asof        `exact` or `prev`, the rule that answered
 * @property {string|null} segment the store segment the server read (`INDEX`, `CASH`)
 * @property {string|null} key    the store key the server read (`NIFTY`, `TMPV`)
 * @property {string} [detail]    the server's own sentence, when it sent one
 */

/**
 * @typedef {object} RungAnswer
 * @property {string|null} rung   `ATM`, `ITM-k` or `OTM+k`, or null when refused
 * @property {number|null} k      signed offset from ATM along the chain
 * @property {string|null} why    the refusal word, or null with a rung
 * @property {string|null} detail the server's own sentence
 * @property {number|null} step   `step_local`, paisa
 * @property {number|null} atm    the ATM strike, paisa
 * @property {number|null} n      strikes stored by then
 * @property {number|null} spot   the spot the rung was counted from, paisa
 */

/**
 * The cells for one bar row: moneyness from its rung at its own instant,
 * intrinsic and extrinsic from its spot at its own instant.
 *
 * `extr` is the premium (the option bar's close) less intrinsic, and it is NOT
 * clipped: a negative figure is a stored close below intrinsic, which is a
 * fact about the bar, not an arithmetic error to hide.
 *
 * @param {{instrument: string, strike: number|null, side: 'CE'|'PE'|null, c: number, tf: string, ts: number}} row
 * @param {SpotAnswer | undefined} spot
 * @param {Map<string, RungAnswer> | undefined} rungs every rung read, by {@link rungKey}
 */
export function optionCells(row, spot, rungs) {
  if (!row.side || row.strike === null || row.strike === undefined) return null;
  const r = rungs?.get(rungKey(row));
  const rung = r
    ? { mny: r.rung, mnyWhy: r.rung === null ? (r.why ?? 'rung_request_failed') : null, mnyDetail: r.detail ?? null,
        step: r.step ?? null, atm: r.atm ?? null, n: r.n ?? null }
    : { mny: null, mnyWhy: 'rung_pending', mnyDetail: null, step: null, atm: null, n: null };
  if (!spot) {
    return { ...rung, intr: null, extr: null, why: 'spot_pending', spot: null, detail: null, spotT: null, age: null, asof: null };
  }
  const detail = spot.detail ?? null;
  const at = { spotT: spot.spotT ?? null, age: spot.age ?? null, asof: spot.asof ?? 'exact' };
  if (spot.close === null) {
    const why = spot.why ?? 'spot_request_failed';
    return { ...rung, intr: null, extr: null, why, spot: null, detail, ...at };
  }
  const intr = intrinsic(row.strike, row.side, spot.close);
  return { ...rung, intr, extr: row.c - intr, why: null, spot: spot.close, detail, ...at };
}

/**
 * The lookup key one bar row's spot is filed under: its underlying, rung,
 * OWN month and second.
 * @param {{instrument: string, tf: string, ts: number, month?: string}} row
 */
export function spotKey(row) {
  const p = parseKey(row.instrument);
  return `${p.exchange}-${p.underlying}|${row.tf}|${ownMonth(row)}|${Math.floor(row.ts / 1000)}`;
}

/**
 * The lookup key one bar row's rung is filed under: its contract, rung and
 * second. A rung needs no month: the chain is per expiry.
 * @param {{instrument: string, tf: string, ts: number}} row
 */
export const rungKey = (row) => `${row.instrument}|${row.tf}|${Math.floor(row.ts / 1000)}`;

/**
 * The key one bar row's chain group is filed under: underlying, expiry and
 * rung — the key {@link readRungs} files each group's newest ladder under.
 * @param {{instrument: string, tf: string}} row
 */
export function ladderKey(row) {
  const p = parseKey(row.instrument);
  return `${chainKey(p.exchange, p.underlying, p.expiry)}|${row.tf}`;
}

/** @param {unknown} v */
const int = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : null);

/**
 * Reads the spot for every option row given, one request per underlying,
 * rung and OWN month (chunked at {@link SPOT_CHUNK}), and files each answer
 * under {@link spotKey}. A failed request files `spot_request_failed` for
 * each of its instants and keeps the server's own sentence in `detail`.
 *
 * @param {string} feed
 * @param {{instrument: string, tf: string, ts: number, side: string|null, month?: string}[]} rows
 * @param {AbortSignal} signal
 * @param {typeof ask} [request]
 * @param {'exact'|'prev'} [asof] `prev` opts in to the last spot at or before t (D-3328)
 * @returns {Promise<Map<string, SpotAnswer>>}
 */
export async function readSpots(feed, rows, signal, request = ask, asof = 'exact') {
  /** @type {Map<string, {exchange: string, symbol: string, tf: string, month: string, times: Set<number>}>} */
  const groups = new Map();
  for (const r of rows) {
    if (!r.side) continue;
    const p = parseKey(r.instrument);
    const month = ownMonth(r);
    const g = `${p.exchange}-${p.underlying}|${r.tf}|${month}`;
    let e = groups.get(g);
    if (!e) {
      e = { exchange: String(p.exchange), symbol: String(p.underlying), tf: r.tf, month, times: new Set() };
      groups.set(g, e);
    }
    e.times.add(Math.floor(r.ts / 1000));
  }
  /** @type {Map<string, SpotAnswer>} */
  const out = new Map();
  for (const [g, e] of groups) {
    const times = [...e.times].sort((a, b) => a - b);
    for (let i = 0; i < times.length; i += SPOT_CHUNK) {
      signal.throwIfAborted();
      const chunk = times.slice(i, i + SPOT_CHUNK);
      const q = new URLSearchParams({ feed, exchange: e.exchange, symbol: e.symbol,
        timeframe: e.tf, month: e.month, t: chunk.join(',') });
      if (asof === 'prev') q.set('asof', 'prev');
      try {
        const res = await request(`/spot.json?${q}`, { signal });
        const body = await res.json();
        if (!Array.isArray(body?.spots) || body.spots.length !== chunk.length) {
          throw new Error(body?.error ?? `HTTP ${res.status}`);
        }
        const said = {
          asof: typeof body.asof === 'string' ? body.asof : 'exact',
          segment: typeof body.segment === 'string' ? body.segment : null,
          key: typeof body.key === 'string' ? body.key : null
        };
        for (const s of body.spots) {
          const close = int(s.close);
          out.set(`${g}|${s.t}`, {
            close,
            why: close === null ? String(s.why ?? 'spot_request_failed') : null,
            spotT: int(s.spot_t),
            age: int(s.spot_age_s),
            ...said,
            ...(body.faults ? { detail: String(body.faults) } : {})
          });
        }
      } catch (why) {
        signal.throwIfAborted();
        const detail = String(why instanceof Error ? why.message : why);
        for (const t of chunk) {
          out.set(`${g}|${t}`, { close: null, why: 'spot_request_failed', spotT: null, age: null, asof, segment: null, key: null, detail });
        }
      }
    }
  }
  return out;
}

/**
 * @typedef {object} Ladder
 * @property {number} at          unix seconds the chain was counted at
 * @property {string|null} why    the refusal word at that instant, or null
 * @property {string|null} detail
 * @property {number|null} atm    paisa
 * @property {number|null} step   `step_local`, paisa
 * @property {number|null} n      strikes stored by then
 * @property {number|null} spot   paisa
 * @property {{strike: number, k: number|null, ce: string|null, pe: string|null}[]} strikes
 * @property {{state: string|null, behindFrom: string|null, behindDays: number|null}} census
 * @property {number|null} generation the census generation the chain was counted at
 */

/**
 * The chain at the newest instant of each (underlying, expiry, rung) group,
 * from a `/chain.json` answer's instant `c` and body `body`.
 * @param {any} c @param {any} body @returns {Ladder}
 */
function ladderOf(c, body) {
  return {
    at: Number(c.at),
    why: c.refusal ? String(c.refusal) : null,
    detail: typeof c.detail === 'string' ? c.detail : null,
    atm: int(c.atm),
    step: int(c.step_local),
    n: int(c.n_strikes),
    spot: int(c.spot),
    strikes: Array.isArray(c.strikes)
      ? c.strikes.map((/** @type {any} */ s) => ({
          strike: Number(s.strike),
          k: int(s.k),
          ce: typeof s.ce === 'string' ? s.ce : null,
          pe: typeof s.pe === 'string' ? s.pe : null
        }))
      : [],
    census: {
      state: typeof body?.census?.state === 'string' ? body.census.state : null,
      behindFrom: typeof body?.census?.behind_from === 'string' ? body.census.behind_from : null,
      behindDays: int(body?.census?.behind_days)
    },
    generation: int(body?.generation)
  };
}

/**
 * One instant of a `/chain.json` answer as the rung of one contract.
 * @param {any} c the instant @param {number} strike @param {'CE'|'PE'} side
 * @returns {RungAnswer}
 */
export function rungFrom(c, strike, side) {
  const common = { detail: typeof c.detail === 'string' ? c.detail : null, step: int(c.step_local),
    atm: int(c.atm), n: int(c.n_strikes), spot: int(c.spot) };
  if (c.refusal) return { rung: null, k: null, why: String(c.refusal), ...common };
  if (c.pick && typeof c.pick === 'object') {
    if (typeof c.pick.rung === 'string') return { rung: c.pick.rung, k: int(c.pick.k), why: null, ...common };
    return { rung: null, k: null, why: String(c.pick.why ?? 'rung_request_failed'), ...common };
  }
  const s = Array.isArray(c.strikes) ? c.strikes.find((/** @type {any} */ x) => Number(x.strike) === strike) : undefined;
  if (!s) return { rung: null, k: null, why: 'strike_not_stored_by_then', ...common };
  const named = side === 'CE' ? s.ce : s.pe;
  return typeof named === 'string'
    ? { rung: named, k: int(s.k), why: null, ...common }
    : { rung: null, k: null, why: 'rung_request_failed', ...common };
}

/**
 * Reads the rung of every option row given at its own instant, and the
 * chain at the newest instant of each (underlying, expiry, rung) group.
 *
 * Two forms of one server answer, chosen per group by request count: every
 * strike at up to {@link LADDER_CHUNK} instants per request (a page of many
 * contracts over a few seconds), or one contract at up to {@link RUNG_CHUNK}
 * instants per request (a page of one contract over many seconds). Both are
 * the same server function (`chain::ladder`), so the rung does not depend on
 * which was asked. When the per-contract form was used, one more request
 * reads the whole chain at the group's newest instant for the Moneyness list.
 *
 * @param {string} feed
 * @param {{instrument: string, tf: string, ts: number, side: string|null, strike: number|null}[]} rows
 * @param {AbortSignal} signal
 * @param {typeof ask} [request]
 * @param {'exact'|'prev'} [asof]
 * @returns {Promise<{rungs: Map<string, RungAnswer>, ladders: Map<string, Ladder | {at: number, why: string, detail: string}>}>}
 */
export async function readRungs(feed, rows, signal, request = ask, asof = 'exact') {
  /** @typedef {{strike: number, side: 'CE'|'PE'}} Leg */
  /** @type {Map<string, {exchange: string, underlying: string, expiry: string, tf: string, at: Map<number, Map<string, Leg>>, contracts: Map<string, Leg & {times: Set<number>}>}>} */
  const groups = new Map();
  for (const r of rows) {
    if (!r.side || r.strike === null || r.strike === undefined) continue;
    const p = parseKey(r.instrument);
    if (p.kind !== 'option' || !p.expiry || p.strike === null || !p.side) continue;
    const g = `${chainKey(p.exchange, p.underlying, p.expiry)}|${r.tf}`;
    let e = groups.get(g);
    if (!e) {
      e = { exchange: String(p.exchange), underlying: String(p.underlying), expiry: p.expiry, tf: r.tf,
        at: new Map(), contracts: new Map() };
      groups.set(g, e);
    }
    const t = Math.floor(r.ts / 1000);
    let here = e.at.get(t);
    if (!here) e.at.set(t, (here = new Map()));
    here.set(r.instrument, { strike: p.strike, side: p.side });
    let k = e.contracts.get(r.instrument);
    if (!k) e.contracts.set(r.instrument, (k = { strike: p.strike, side: p.side, times: new Set() }));
    k.times.add(t);
  }
  /** @type {Map<string, RungAnswer>} */
  const rungs = new Map();
  /** @type {Map<string, any>} */
  const ladders = new Map();
  /** @param {string} detail @returns {RungAnswer} */
  const failed = (detail) => ({ rung: null, k: null, why: 'rung_request_failed', detail, step: null, atm: null, n: null, spot: null });
  for (const [g, e] of groups) {
    const base = { feed, exchange: e.exchange, underlying: e.underlying, expiry: e.expiry, timeframe: e.tf };
    /** @param {Record<string, string>} extra */
    const chainOf = async (extra) => {
      const q = new URLSearchParams({ ...base, ...extra });
      if (asof === 'prev') q.set('asof', 'prev');
      const res = await request(`/chain.json?${q}`, { signal });
      const body = await res.json();
      if (!Array.isArray(body?.chains) || body.chains.length !== extra.at.split(',').length) {
        throw new Error(body?.error ?? `HTTP ${res.status}`);
      }
      return body;
    };
    const instants = [...e.at.keys()].sort((a, b) => a - b);
    const newest = instants[instants.length - 1];
    let perContract = 0;
    for (const k of e.contracts.values()) perContract += Math.ceil(k.times.size / RUNG_CHUNK);
    if (Math.ceil(instants.length / LADDER_CHUNK) <= perContract) {
      for (let i = 0; i < instants.length; i += LADDER_CHUNK) {
        signal.throwIfAborted();
        const chunk = instants.slice(i, i + LADDER_CHUNK);
        try {
          const body = await chainOf({ at: chunk.join(',') });
          for (const c of body.chains) {
            const t = Number(c.at);
            for (const [instrument, leg] of e.at.get(t) ?? []) {
              rungs.set(`${instrument}|${e.tf}|${t}`, rungFrom(c, leg.strike, leg.side));
            }
            if (t === newest) ladders.set(g, ladderOf(c, body));
          }
        } catch (why) {
          signal.throwIfAborted();
          const detail = String(why instanceof Error ? why.message : why);
          for (const t of chunk) {
            for (const instrument of (e.at.get(t) ?? new Map()).keys()) rungs.set(`${instrument}|${e.tf}|${t}`, failed(detail));
            if (t === newest) ladders.set(g, { at: t, why: 'rung_request_failed', detail });
          }
        }
      }
      continue;
    }
    for (const [instrument, k] of e.contracts) {
      const times = [...k.times].sort((a, b) => a - b);
      for (let i = 0; i < times.length; i += RUNG_CHUNK) {
        signal.throwIfAborted();
        const chunk = times.slice(i, i + RUNG_CHUNK);
        try {
          const body = await chainOf({ at: chunk.join(','), strike: String(k.strike), side: k.side });
          for (const c of body.chains) rungs.set(`${instrument}|${e.tf}|${Number(c.at)}`, rungFrom(c, k.strike, k.side));
        } catch (why) {
          signal.throwIfAborted();
          const detail = String(why instanceof Error ? why.message : why);
          for (const t of chunk) rungs.set(`${instrument}|${e.tf}|${t}`, failed(detail));
        }
      }
    }
    signal.throwIfAborted();
    try {
      const body = await chainOf({ at: String(newest) });
      ladders.set(g, ladderOf(body.chains[0], body));
    } catch (why) {
      signal.throwIfAborted();
      ladders.set(g, { at: newest, why: 'rung_request_failed', detail: String(why instanceof Error ? why.message : why) });
    }
  }
  return { rungs, ladders };
}

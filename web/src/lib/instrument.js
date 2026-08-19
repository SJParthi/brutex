/**
 * WHAT A STORE KEY IS MADE OF — the parser, with no runes in it, so
 * `web/tests/instrument.test.js` can drive it under `node --test`.
 *
 * # The failure this exists to remove
 *
 * `/db`'s Instrument control offered the store's raw KEYS. Measured against a
 * live store on 19 Aug 2026: 27 distinct keys, of which 26 were option
 * contracts on one expiry and one was the spot series — **one underlying**,
 * BANKNIFTY, offered twenty-seven ways.
 *
 * A key such as `NSE-FNO-BANKNIFTY-2026-07-28-4810000-PE` already fixes the
 * segment, the expiry, the strike and the side. Offering it as "the
 * instrument" and then offering a Segment control beside it asks one question
 * twice, and the two answers can disagree. On the reported screen they did:
 * instrument was a PE option, segment was Spot, and Timeframe answered "No rung
 * stored" — true of that intersection and false about the store, which held
 * 10,296 bars for the same selection and said so two lines below.
 *
 * # The grammar, read off the keys this repository writes
 *
 *     spot     EXCHANGE-SEGMENT-UNDERLYING
 *     future   EXCHANGE-SEGMENT-UNDERLYING-YYYY-MM-DD-FUT
 *     option   EXCHANGE-SEGMENT-UNDERLYING-YYYY-MM-DD-STRIKE-CE|PE
 *
 * THE TAIL IS STRIPPED BEFORE THE UNDERLYING IS TAKEN, and that order is the
 * whole of the parser. `parts.slice(2).join('-')` alone is right for spot and
 * wrong for every contract: on the key above it yields
 * `BANKNIFTY-2026-07-28-4810000-PE`, which is the bug wearing a different
 * spelling.
 *
 * THE UNDERLYING MAY CONTAIN HYPHENS. `NSE-CASH-BAJAJ-AUTO` is a real NSE
 * trading symbol and the reason the underlying is "everything after the second
 * separator" rather than "the third token".
 *
 * # Money
 *
 * The strike stays the PAISA INTEGER the key carries. `CLAUDE.md` §7 bans the
 * float for money, and a strike is money; rendering it as rupees is the
 * caller's job and is done with integer arithmetic there.
 */

/** `-YYYY-MM-DD-FUT` at the end of a key. */
export const FUTURE_TAIL = /-(\d{4}-\d{2}-\d{2})-FUT$/;
/** `-YYYY-MM-DD-<paisa>-CE|PE` at the end of a key. */
export const OPTION_TAIL = /-(\d{4}-\d{2}-\d{2})-(\d+)-(CE|PE)$/;

/**
 * @typedef {object} Parsed
 * @property {string} key        the key, verbatim.
 * @property {string|null} exchange
 * @property {string|null} segment   the key's own second token: INDEX, CASH, FNO.
 * @property {string|null} underlying the trading symbol, hyphens and all, or
 *           `null` when the key cannot be read.
 * @property {'spot'|'future'|'option'|null} kind
 * @property {string|null} expiry    raw ISO day, contracts only.
 * @property {number|null} strike    PAISA INTEGER, options only.
 * @property {'CE'|'PE'|null} side   options only.
 * @property {string|null} why       why it could not be read, or `null`.
 */

/**
 * One store key, decomposed.
 *
 * A KEY THAT CANNOT BE READ IS NAMED, NEVER COERCED. It comes back with
 * `underlying: null` and a `why`, so a caller can draw it refused rather than
 * silently dropping it or filing it under a symbol it never had. Dropping is
 * how a held instrument becomes invisible, and inventing is worse.
 *
 * @param {unknown} key
 * @returns {Parsed}
 */
export function parseKey(key) {
  const empty = {
    key: typeof key === 'string' ? key : '',
    exchange: null,
    segment: null,
    underlying: null,
    kind: null,
    expiry: null,
    strike: null,
    side: null,
    why: null
  };
  if (typeof key !== 'string' || key.trim() === '') {
    return { ...empty, why: 'not a string' };
  }
  const k = key.trim();

  // THE TAIL FIRST. Taking the underlying before stripping this is the bug
  // this module exists to remove.
  let expiry = null;
  let strike = null;
  /** @type {'CE'|'PE'|null} */
  let side = null;
  /** @type {'spot'|'future'|'option'} */
  let kind = 'spot';
  let head = k;

  const opt = k.match(OPTION_TAIL);
  const fut = opt ? null : k.match(FUTURE_TAIL);
  if (opt) {
    kind = 'option';
    expiry = opt[1];
    // A PAISA INTEGER, and `Number` only because the pattern already proved it
    // is `\d+`. Beyond 2^53 it would lose precision, which no listed strike
    // approaches; a strike that did would be a store problem worth seeing
    // rather than hiding, and `strikeExact` below is what a caller renders with.
    strike = Number(opt[2]);
    side = /** @type {'CE'|'PE'} */ (opt[3]);
    head = k.slice(0, k.length - opt[0].length);
  } else if (fut) {
    kind = 'future';
    expiry = fut[1];
    head = k.slice(0, k.length - fut[0].length);
  }

  // EVERYTHING AFTER THE SECOND SEPARATOR, not the third token: the underlying
  // may contain hyphens, and BAJAJ-AUTO is the case that proves it.
  const first = head.indexOf('-');
  const second = first === -1 ? -1 : head.indexOf('-', first + 1);
  if (first === -1 || second === -1 || second + 1 >= head.length) {
    return {
      ...empty,
      key: k,
      why: `expected EXCHANGE-SEGMENT-UNDERLYING and found ${JSON.stringify(head)}`
    };
  }

  return {
    key: k,
    exchange: head.slice(0, first),
    segment: head.slice(first + 1, second),
    underlying: head.slice(second + 1),
    kind,
    expiry,
    strike,
    side,
    why: null
  };
}

/**
 * The distinct underlyings a set of keys covers, sorted, with the keys that
 * carry each one.
 *
 * ONE PASS, and the result is a `Map` so a caller's lookup is one probe rather
 * than a scan of the key list per interaction. Unreadable keys are collected
 * under `unreadable` instead of being dropped — see `parseKey`.
 *
 * @param {Iterable<string>} keys
 * @returns {{byUnderlying: Map<string, Parsed[]>, unreadable: Parsed[]}}
 */
export function groupByUnderlying(keys) {
  /** @type {Map<string, Parsed[]>} */
  const byUnderlying = new Map();
  /** @type {Parsed[]} */
  const unreadable = [];
  for (const key of keys ?? []) {
    const p = parseKey(key);
    if (p.underlying === null) {
      unreadable.push(p);
      continue;
    }
    let bucket = byUnderlying.get(p.underlying);
    if (!bucket) byUnderlying.set(p.underlying, (bucket = []));
    bucket.push(p);
  }
  return { byUnderlying, unreadable };
}

/**
 * Which of the three segment rungs a parsed key belongs to.
 *
 * THE KEY'S OWN SEGMENT TOKEN DOES NOT ANSWER THIS ON ITS OWN. `FNO` covers
 * both futures and options, and the difference is in the tail — so the rung is
 * decided by `kind`, which the tail already established, and `INDEX`/`CASH`
 * fall to spot because a continuous series is what they are.
 *
 * @param {Parsed} p
 * @returns {'spot'|'futures'|'options'|null}
 */
export function rungOf(p) {
  if (!p || p.underlying === null) return null;
  if (p.kind === 'option') return 'options';
  if (p.kind === 'future') return 'futures';
  return 'spot';
}

/**
 * A paisa strike as rupees, EXACTLY — integer arithmetic only.
 *
 * `CLAUDE.md` §7: prices are paisa integers and never floats. The remainder is
 * taken with `%`, and the quotient is a multiple of 100 divided by 100, which
 * IEEE returns exactly inside 2^53. The paise are printed only when they are
 * non-zero, which is the ordinary case for a listed strike being zero.
 *
 * @param {number|null} paisa
 * @returns {string}
 */
export function strikeExact(paisa) {
  if (paisa === null || !Number.isInteger(paisa)) return '—';
  const neg = paisa < 0;
  const abs = Math.abs(paisa);
  const rupees = (abs - (abs % 100)) / 100;
  const paise = abs % 100;
  const body = paise === 0 ? String(rupees) : `${rupees}.${String(paise).padStart(2, '0')}`;
  return neg ? `-${body}` : body;
}

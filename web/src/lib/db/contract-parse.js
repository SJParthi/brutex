/**
 * /db — READING WHAT WAS TYPED INTO THE SEARCH BOX AS A CONTRACT (WP-51;
 * D-3332, D-3337; DB-G97).
 *
 * The box is asked about a contract in many spellings, and before this module
 * it understood only the store's own key (`NSE-FNO-NIFTY-2019-03-28-1150000-CE`)
 * and a prefix of the tail it displays. Measured over the 2,42,341-row census
 * on 6 Oct 2026: `NIFTY28MAR1911500CE`, `NIFTY 11500 CE`, `NIFTY 11,500 CE`,
 * `11500`, `2019-03-28`, `28 Mar 2019` and `28MAR19` each found 0 rows.
 *
 * This module turns the text into READINGS — `{name, expiry | month |
 * dayMonth, strike, side}` — and nothing else. It never decides that a reading
 * is right: `search.js` keeps a reading only where the census holds the
 * contract it names, so a spelling that reads two ways (the forms below
 * overlap) is settled by the store, never by a guess here. A reading the
 * store does not hold is simply not a hit.
 *
 * # The vendor forms, and where each one is written down
 *
 * - GDFL's DAY form, `UNDERLYING DD MON YY STRIKE CE|PE` (`NIFTY28MAR1911500CE`):
 *   D-2806 item 2 and the GDFL CSV identity row of `docs/05-decisions.md`
 *   (`NIFTY03JUL2522800CE.NFO`, verified from a sample).
 * - GDFL's MONTHLY form, `UNDERLYING YY MON STRIKE CE|PE`
 *   (`ACC18DEC1500CE`): D-2806 item 2. It names a month, not a day: the store
 *   files the contract under its settled expiry (D-2806 item 3), so the
 *   search reads it as that underlying's last stored expiry of the month —
 *   the expiry `expiryTags` (WP-50) tags `monthly`. Kite's monthly
 *   tradingsymbol has the same shape; that it does is UNVERIFIED (no Kite
 *   symbol format is recorded in `docs/00-charter.md`), and it is read here
 *   only because the GDFL form is.
 * - KITE'S WEEKLY form, `UNDERLYING YY M DD STRIKE CE|PE` with `M` one of
 *   `1`-`9`, `O`, `N`, `D` (`NIFTY1932811500CE`): **UNVERIFIED** — no source
 *   in `docs/00-charter.md` records it. It is read because the plan asks for
 *   "Kite style", every hit it yields says it was read in this form, and a
 *   reading the store does not hold is never shown, so an unverified shape
 *   can only ever find a contract that is really there.
 * - A future in either GDFL form ends `FUT` (`NIFTY28MAR19FUT`,
 *   `NIFTY19MARFUT`). The operator's GDFL data holds no futures (D-2806
 *   item 1), so these find nothing today and find a future the day one is
 *   stored.
 *
 * # Natural text
 *
 * A name, then any of an expiry, a strike and a side, in any order:
 * `NIFTY 11500 CE`, `NIFTY 11,500 CE` (a comma between digits is a digit
 * group, Indian or Western), `nifty 28 mar 2019 11500 call`, `2019-03-28`,
 * `28/03/2019` (day first: GDFL's own CSV writes dates `DD/MM/YYYY`, charter
 * GDFL CSV columns row), `28MAR19`, `28MAR2019`, `Mar 2019`, `2019-03`,
 * `11500`, `11500 CE`, `11500CE`.
 *
 * # Cost
 *
 * O(length of the text): one pass to tokenise, and for a ticker one regular
 * expression test per split point (at most `MAX_TICKER` of them). Nothing
 * here reads the census.
 *
 * A PLAIN MODULE with relative imports and no runes, so `node --test` drives
 * it (`web/tests/db-search.test.js`).
 */

/** The month words a vendor name and a date use, in calendar order. */
export const MONTHS = ['JAN', 'FEB', 'MAR', 'APR', 'MAY', 'JUN', 'JUL', 'AUG', 'SEP', 'OCT', 'NOV', 'DEC'];

/** Month word (three letters, or the whole English name) to 1..12. */
const MONTH_NUMBER = new Map([
  ...MONTHS.map((m, i) => /** @type {[string, number]} */ ([m, i + 1])),
  ['JANUARY', 1],
  ['FEBRUARY', 2],
  ['MARCH', 3],
  ['APRIL', 4],
  ['JUNE', 6],
  ['JULY', 7],
  ['AUGUST', 8],
  ['SEPT', 9],
  ['SEPTEMBER', 9],
  ['OCTOBER', 10],
  ['NOVEMBER', 11],
  ['DECEMBER', 12]
]);

/**
 * The longest compact text read as a ticker. GDFL's longest option name is
 * far shorter (`MAX_TICKER` in `crates/pull/src/gdfl_nfo.rs` bounds the
 * importer the same way); past this the text is a phrase, not a name, and is
 * not split, which keeps the split loop bounded.
 */
export const MAX_TICKER = 48;

/** A symbol as the store files it: `core::symbol::Symbol`'s alphabet, at most 24. */
const SYMBOL = /^[A-Z0-9&_-]{1,24}$/;

/** Kite's weekly month letter to 1..12 (UNVERIFIED form; see the header). */
const KITE_MONTH = new Map([
  ['1', 1],
  ['2', 2],
  ['3', 3],
  ['4', 4],
  ['5', 5],
  ['6', 6],
  ['7', 7],
  ['8', 8],
  ['9', 9],
  ['O', 10],
  ['N', 11],
  ['D', 12]
]);

/**
 * @typedef {object} Reading
 * @property {string} name the underlying as typed (upper case, spaces kept), or `''`
 * @property {string | null} expiry an ISO day the contract expires on
 * @property {string | null} month `YYYY-MM`: some expiry in that month
 * @property {boolean} monthly with `month`: the underlying's last stored expiry of that month (a vendor monthly form)
 * @property {string | null} dayMonth `MM-DD`: an expiry on that day of any year
 * @property {number | null} strike paisa
 * @property {'CE' | 'PE' | 'FUT' | null} side
 * @property {string} form which spelling this reading came from, for the hit's note
 */

/**
 * A rupee strike as written (`11500`, `172.5`, `0.05`) to paisa, exactly.
 *
 * Integer arithmetic only (`CLAUDE.md` §7): the whole rupees times 100 plus
 * the paise, never a float multiply. More than two decimals, a leading `.`,
 * or zero is not a strike and answers `null`.
 *
 * @param {string} text
 * @returns {number | null}
 */
export function strikePaisa(text) {
  const m = /^(\d{1,9})(?:\.(\d{1,2}))?$/.exec(text);
  if (m === null) return null;
  const paise = m[2] === undefined ? 0 : Number(m[2].padEnd(2, '0'));
  const p = Number(m[1]) * 100 + paise;
  return p > 0 ? p : null;
}

/**
 * A real calendar day as `YYYY-MM-DD`, or `null` (30 Feb, month 13, day 0).
 *
 * @param {number} y
 * @param {number} m
 * @param {number} d
 * @returns {string | null}
 */
export function isoDay(y, m, d) {
  if (!Number.isInteger(y) || !Number.isInteger(m) || !Number.isInteger(d)) return null;
  if (y < 1900 || y > 2199 || m < 1 || m > 12 || d < 1) return null;
  const last = new Date(Date.UTC(y, m, 0)).getUTCDate();
  if (d > last) return null;
  return `${String(y).padStart(4, '0')}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
}

/**
 * `YYYY-MM`, or `null`.
 *
 * @param {number} y
 * @param {number} m
 */
function isoMonth(y, m) {
  return Number.isInteger(y) && Number.isInteger(m) && y >= 1900 && y <= 2199 && m >= 1 && m <= 12
    ? `${y}-${String(m).padStart(2, '0')}`
    : null;
}

/**
 * A year as written: four digits as they are; two digits are this century's
 * (every vendor name in the archive is 2018 to 2026, D-2806's evidence).
 *
 * @param {string} text
 */
function yearOf(text) {
  if (/^\d{4}$/.test(text)) return Number(text);
  if (/^\d{2}$/.test(text)) return 2000 + Number(text);
  return NaN;
}

/** @returns {Reading} */
function blank(/** @type {string} */ form) {
  return { name: '', expiry: null, month: null, monthly: false, dayMonth: null, strike: null, side: null, form };
}

/**
 * Upper case, digit groups closed (`11,500` → `11500`, `1,15,000` →
 * `115000`), every other separator a space, runs of space collapsed.
 *
 * @param {unknown} text
 * @returns {string}
 */
export function normalise(text) {
  return String(text ?? '')
    .toUpperCase()
    .replace(/(\d),(?=\d)/g, '$1')
    .replace(/[₹,;()[\]{}"'`]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Every reading of ONE compact token as a vendor option or future name, in
 * the three forms the header names. Each split of the leading symbol is
 * tried, so a symbol that ends in digits (`NIFTYNXT50`) or holds a month's
 * letters (`MARICO`) is read wherever the rest of the name fits.
 *
 * @param {string} token upper case, no spaces
 * @returns {Reading[]}
 */
export function tickerReadings(token) {
  /** @type {Reading[]} */
  const out = [];
  if (token.length < 6 || token.length > MAX_TICKER || !/^[A-Z0-9&_.-]+$/.test(token)) return out;
  /** @type {'CE' | 'PE' | 'FUT' | null} */
  const side = token.endsWith('FUT') ? 'FUT' : token.endsWith('CE') ? 'CE' : token.endsWith('PE') ? 'PE' : null;
  if (side === null) return out;
  const body = token.slice(0, token.length - side.length);
  const mon = MONTHS.join('|');
  const strike = side === 'FUT' ? '()' : '(\\d+(?:\\.\\d{1,2})?)';
  const dayForm = new RegExp(`^(\\d{2})(${mon})(\\d{2})${strike}$`);
  const monthForm = new RegExp(`^(\\d{2})(${mon})${strike}$`);
  const kiteWeekly = side === 'FUT' ? null : new RegExp(`^(\\d{2})([1-9OND])(\\d{2})${strike}$`);
  for (let cut = 1; cut < body.length; cut += 1) {
    const sym = body.slice(0, cut);
    if (!SYMBOL.test(sym)) break;
    const rest = body.slice(cut);
    const k = (/** @type {string} */ s) => (side === 'FUT' ? null : strikePaisa(s));
    let m = dayForm.exec(rest);
    if (m) {
      const expiry = isoDay(2000 + Number(m[3]), /** @type {number} */ (MONTH_NUMBER.get(m[2])), Number(m[1]));
      const p = k(m[4]);
      if (expiry && (side === 'FUT' || p !== null)) out.push({ ...blank('GDFL day form'), name: sym, expiry, strike: p, side });
    }
    m = monthForm.exec(rest);
    if (m) {
      const month = isoMonth(2000 + Number(m[1]), /** @type {number} */ (MONTH_NUMBER.get(m[2])));
      const p = k(m[3]);
      if (month && (side === 'FUT' || p !== null)) {
        out.push({ ...blank('GDFL monthly form'), name: sym, month, monthly: true, strike: p, side });
      }
    }
    if (kiteWeekly) {
      m = kiteWeekly.exec(rest);
      if (m) {
        const expiry = isoDay(2000 + Number(m[1]), /** @type {number} */ (KITE_MONTH.get(m[2])), Number(m[3]));
        const p = k(m[4]);
        if (expiry && p !== null) {
          out.push({ ...blank("Kite's weekly form (UNVERIFIED)"), name: sym, expiry, strike: p, side });
        }
      }
    }
  }
  return out;
}

/** @param {string} t */
const monthOf = (t) => MONTH_NUMBER.get(t) ?? null;

/**
 * What one side word means, or `null`.
 *
 * @param {string} t
 * @returns {'CE' | 'PE' | 'FUT' | null}
 */
function sideOf(t) {
  if (t === 'CE' || t === 'CALL' || t === 'CALLS') return 'CE';
  if (t === 'PE' || t === 'PUT' || t === 'PUTS') return 'PE';
  if (t === 'FUT' || t === 'FUTURE' || t === 'FUTURES') return 'FUT';
  return null;
}

/**
 * The natural reading of the text: the words that are not a date, a strike or
 * a side are the name. `null` when the text holds two strikes, two dates or
 * two sides — a phrase that names two contracts names none, and the name
 * search still runs on the whole text.
 *
 * @param {string} text normalised
 * @returns {Reading | null}
 */
export function naturalReading(text) {
  if (text === '') return null;
  /** @type {string[]} */
  const tokens = [];
  for (const raw of text.split(' ')) {
    /* A STRIKE GLUED TO ITS SIDE (`11500CE`) IS TWO WORDS. A name glued to
       one (`NIFTY28MAR1911500CE`) is not split here: it is a ticker, and
       `tickerReadings` reads it whole. */
    const glued = /^(\d+(?:\.\d{1,2})?)(CE|PE)$/.exec(raw);
    if (glued) tokens.push(glued[1], glued[2]);
    else tokens.push(raw);
  }
  const r = blank('natural text');
  /** @type {string[]} */
  const name = [];
  let dates = 0;
  let strikes = 0;
  let sides = 0;
  for (let i = 0; i < tokens.length; i += 1) {
    const t = tokens[i];
    const next = tokens[i + 1] ?? '';
    const after = tokens[i + 2] ?? '';
    let m;
    /** @param {string | null} e */
    const setExpiry = (e) => {
      if (e === null) return false;
      r.expiry = e;
      dates += 1;
      return true;
    };
    /** @param {string | null} mo */
    const setMonth = (mo) => {
      if (mo === null) return false;
      r.month = mo;
      dates += 1;
      return true;
    };
    if ((m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(t)) && setExpiry(isoDay(+m[1], +m[2], +m[3]))) continue;
    if ((m = /^(\d{4})-(\d{2})$/.exec(t)) && setMonth(isoMonth(+m[1], +m[2]))) continue;
    if ((m = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})$/.exec(t)) && setExpiry(isoDay(+m[3], +m[2], +m[1]))) continue;
    if ((m = /^(\d{1,2})([A-Z]{3,9})(\d{2}|\d{4})$/.exec(t)) && monthOf(m[2]) !== null) {
      if (setExpiry(isoDay(yearOf(m[3]), /** @type {number} */ (monthOf(m[2])), +m[1]))) continue;
    }
    /* `19MAR` IS YEAR THEN MONTH — the vendor monthly form's date (D-2806) —
       and `MAR19` the same month written the other way round. */
    if ((m = /^(\d{2})([A-Z]{3,9})$/.exec(t)) && monthOf(m[2]) !== null) {
      if (setMonth(isoMonth(yearOf(m[1]), /** @type {number} */ (monthOf(m[2]))))) continue;
    }
    if ((m = /^([A-Z]{3,9})(\d{2}|\d{4})$/.exec(t)) && monthOf(m[1]) !== null) {
      if (setMonth(isoMonth(yearOf(m[2]), /** @type {number} */ (monthOf(m[1]))))) continue;
    }
    /* `28 MAR 2019`, `28 MAR 19`, `MAR 28 2019`, `MAR 2019`, `28 MAR`. */
    if (/^\d{1,2}$/.test(t) && monthOf(next) !== null) {
      if (/^(\d{4}|\d{2})$/.test(after) && setExpiry(isoDay(yearOf(after), /** @type {number} */ (monthOf(next)), +t))) {
        i += 2;
        continue;
      }
      if (!/^\d/.test(after)) {
        r.dayMonth = `${String(monthOf(next)).padStart(2, '0')}-${t.padStart(2, '0')}`;
        dates += 1;
        i += 1;
        continue;
      }
    }
    if (monthOf(t) !== null && /^\d{1,2}$/.test(next) && /^\d{4}$/.test(after)) {
      if (setExpiry(isoDay(+after, /** @type {number} */ (monthOf(t)), +next))) {
        i += 2;
        continue;
      }
    }
    if (monthOf(t) !== null && /^\d{4}$/.test(next) && setMonth(isoMonth(+next, /** @type {number} */ (monthOf(t))))) {
      i += 1;
      continue;
    }
    const s = sideOf(t);
    if (s !== null) {
      r.side = s;
      sides += 1;
      continue;
    }
    if (/^\d+(?:\.\d{1,2})?$/.test(t)) {
      const p = strikePaisa(t);
      if (p !== null) {
        r.strike = p;
        strikes += 1;
        continue;
      }
    }
    name.push(t);
  }
  if (dates > 1 || strikes > 1 || sides > 1) return null;
  r.name = name.join(' ');
  return r;
}

/**
 * Everything the text could mean as a contract: its natural reading, and
 * every vendor-form reading of the text with its spaces removed (so
 * `NIFTY 28MAR19 11500CE` is read as the ticker too).
 *
 * @param {unknown} text
 * @returns {{ text: string, compact: string, readings: Reading[] }}
 */
export function parseQuery(text) {
  const norm = normalise(text);
  const compact = norm.replace(/ /g, '');
  /** @type {Reading[]} */
  const readings = [];
  const natural = naturalReading(norm);
  if (natural !== null && (natural.expiry || natural.month || natural.dayMonth || natural.strike !== null || natural.side)) {
    readings.push(natural);
  }
  readings.push(...tickerReadings(compact));
  return { text: norm, compact, readings };
}

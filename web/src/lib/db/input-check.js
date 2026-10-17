/**
 * ONE CHECKER FOR EVERY VALUE /db READS — TYPED INTO A BOX OR CARRIED BY A
 * LINK (R3-01; DB-077, DB-G98).
 *
 * The address reader (`urlstate.js` `decodeDb`, then the link reader in
 * `selection.svelte.js`) and the boxes on the page asked the same questions
 * with different code: is this a calendar day, is this a time, is this a
 * whole number in range, is this text short enough. Two codes for one
 * question answer differently on the same input — a day in 1969, a day in
 * 2099, From after To, 6,000 characters — and the operator saw one sentence
 * when he typed it and another (or none) when a link carried it. Every
 * answer below is a SENTENCE, the same one wherever the value came from.
 *
 * Pure, no runes, no DOM: `web/tests/db-address.test.js` drives it under
 * `node --test`. Each check is O(1) in the value it is given except
 * `checkText`, which reads at most `MAX_TEXT + 1` characters of it before it
 * answers, so a 6,000-character value costs the same as a 129-character one.
 *
 * Nothing here knows a vendor, an exchange hour or an instrument: the bounds a
 * day may take are handed in by the caller (the held span the census states,
 * else the feed's era), and a check with no bound given applies none.
 */

/**
 * The longest instrument name or instrument key /db reads, typed or linked.
 *
 * A BOUND, NOT A VENDOR FACT. The longest key this store writes is an option
 * key such as `NSE-FNO-NIFTY-2019-03-28-1150000-CE` (35 characters); 128 is
 * room for any underlying name with its contract tail, and short enough that a
 * 6,000-character value is refused before anything compares it.
 */
export const MAX_TEXT = 128;

/** The largest page number the address reads (nine digits). */
export const MAX_PAGE = 999_999_999;

const DAY = /^(\d{4})-(\d{2})-(\d{2})$/;
const TIME = /^([01]\d|2[0-3]):([0-5]\d)(?::([0-5]\d))?$/;
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/**
 * An ISO day as the page prints one: `03 Sep 2018`. Pure arithmetic on the
 * string, so no time zone can move it a day.
 *
 * @param {string} iso
 */
export function dayText(iso) {
  const m = DAY.exec(String(iso ?? ''));
  if (!m) return String(iso ?? '');
  return `${m[3]} ${MONTHS[Number(m[2]) - 1] ?? m[2]} ${m[1]}`;
}

/**
 * At most 40 characters of a value, for quoting it inside a sentence: a
 * 6,000-character value is named by its length, never echoed whole.
 *
 * @param {unknown} raw
 */
export function quote(raw) {
  const s = String(raw ?? '');
  return s.length > 40 ? `${JSON.stringify(s.slice(0, 24))}… (${s.length.toLocaleString('en-IN')} characters)` : JSON.stringify(s);
}

/**
 * @typedef {{ok: true, value: string}} Good
 * @typedef {{ok: false, why: string}} Bad
 */

/**
 * Over-long text. Reads at most `max + 1` characters before answering.
 *
 * @param {unknown} raw
 * @param {{label?: string, max?: number}} [o]
 * @returns {Good | Bad}
 */
export function checkText(raw, o = {}) {
  const max = o.max ?? MAX_TEXT;
  const s = String(raw ?? '');
  if (s.length > max) {
    return {
      ok: false,
      why: `${o.label ?? 'this text'} is ${s.length.toLocaleString('en-IN')} characters long, and /db reads at most ${max}`
    };
  }
  return { ok: true, value: s.trim() };
}

/** @param {number} y @param {number} mo */
const daysIn = (y, mo) => [31, (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0 ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][mo - 1] ?? 0;

/**
 * WHAT A DAY BOX READS, AND WHAT A LINK READS — ONE PARSER. `01 Jul 2026`,
 * `1 jul 2026`, `01-Jul-2026`, `1/7/2026` and `2026-07-01` are one day; the
 * slash form is DAY FIRST, as the box's own label and placeholder are. These
 * are the rules `$lib/DayField.svelte` reads typed text with (its `parse` and
 * `valid`, years 1900 to 2999), written once here so a link's `from=` and a
 * typed From date are read alike; the box imports this when its owner (R3-02)
 * moves onto it.
 *
 * @param {unknown} text
 * @returns {string} ISO `YYYY-MM-DD`, or `''` when it is not a day
 */
export function parseDay(text) {
  const s = String(text ?? '').trim();
  if (s === '' || s.length > 32) return '';
  /** @param {number} y @param {number} mo @param {number} d */
  const valid = (y, mo, d) =>
    mo >= 1 && mo <= 12 && d >= 1 && d <= daysIn(y, mo) && y >= 1900 && y <= 2999
      ? `${y}-${String(mo).padStart(2, '0')}-${String(d).padStart(2, '0')}`
      : '';
  let m = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(s);
  if (m) return valid(+m[1], +m[2], +m[3]);
  m = /^(\d{1,2})[ \-/.]+([A-Za-z]{3,})[ \-/.,]+(\d{4})$/.exec(s);
  if (m) {
    const [, day, name, year] = m;
    const mo = MONTHS.findIndex((x) => x.toLowerCase() === name.slice(0, 3).toLowerCase()) + 1;
    return mo === 0 ? '' : valid(+year, mo, +day);
  }
  m = /^(\d{1,2})[/\-.](\d{1,2})[/\-.](\d{4})$/.exec(s);
  if (m) return valid(+m[3], +m[2], +m[1]);
  return '';
}

/**
 * The caller's reason for a bound, punctuated once and cut to its first
 * sentence — the tail a day box puts after "Outside … to …" (`DayField`
 * `reasonTail`, same rule).
 *
 * @param {unknown} reason
 */
export function reasonTail(reason) {
  const raw = String(reason ?? '').trim();
  if (raw === '') return '.';
  const first = raw.split(/(?<=\.)\s+/)[0] ?? raw;
  return ` — ${first.replace(/[.;,\s]+$/, '')}.`;
}

/**
 * The sentence for a day outside `[lo, hi]`, or `null` inside — the words the
 * day box says under itself (`Outside 03 Sep 2018 to 30 Sep 2026 — …`), so
 * a linked day and a typed day outside the same bounds read alike.
 *
 * @param {string} iso
 * @param {{lo?: string, hi?: string, reason?: string}} [o]
 * @returns {string | null}
 */
export function dayBoundWhy(iso, o = {}) {
  const lo = o.lo || '';
  const hi = o.hi || '';
  const tail = reasonTail(o.reason);
  if (lo && hi && (iso < lo || iso > hi)) return `Outside ${dayText(lo)} to ${dayText(hi)}${tail}`;
  if (lo && iso < lo) return `Earlier than ${dayText(lo)}${tail}`;
  if (hi && iso > hi) return `Later than ${dayText(hi)}${tail}`;
  return null;
}

/**
 * A calendar day, read as a day box reads one (`parseDay`), inside
 * `[lo, hi]` when they are given. `2019-02-31` is not a day; `1969-01-01` is
 * a day and is refused only by a bound, in the box's own words
 * (`dayBoundWhy`), after the field and the day are named.
 *
 * @param {unknown} raw
 * @param {{label?: string, lo?: string, hi?: string, reason?: string}} [o]
 * @returns {Good | Bad}
 */
export function checkDay(raw, o = {}) {
  const label = o.label ?? 'the day';
  const long = checkText(raw, { label, max: 32 });
  if (!long.ok) return long;
  const s = long.value;
  const iso = parseDay(s);
  if (iso === '') {
    return {
      ok: false,
      why: `${label} ${quote(s)} is not a date this field can read — it takes dd Mon yyyy or YYYY-MM-DD, like ${dayText(o.lo || o.hi || '2019-03-15')}`
    };
  }
  const bound = dayBoundWhy(iso, o);
  if (bound) return { ok: false, why: `${label} ${dayText(iso)}: ${bound}` };
  return { ok: true, value: iso };
}

/**
 * A From/To day window that runs backwards, or `null` when it does not.
 *
 * @param {string} from
 * @param {string} to
 * @returns {string | null}
 */
export function windowWhy(from, to) {
  if (from && to && from > to) {
    return `From ${dayText(from)} is after To ${dayText(to)}, so the window holds no day — swap them or move one`;
  }
  return null;
}

/**
 * A LINKED DAY WINDOW AGAINST THE BOUNDS ITS BOXES WOULD TAKE (R3-01). Each
 * side is checked as its box checks a typed day — From inside
 * `[lo, kept To || hi]`, To inside `[kept From || lo, hi]`, the min and max
 * `DbControls` hands the two boxes — and a side outside is REFUSED in the
 * box's own sentence (`checkDay`, `dayBoundWhy`) and left out (`''`), never
 * kept silently and never sent to a server that refuses a year before 1970.
 * A bound that is not known (`''`) applies none, so with no bound at all both
 * sides are kept and nothing is said.
 *
 * `opens` names what the box shows instead, for the sentence's tail.
 *
 * @param {{from?: string, to?: string}} want ISO days the address carried
 * @param {{lo?: string, hi?: string, reason?: string, opens?: {from: string, to: string}}} bounds
 * @returns {{from: string, to: string, notes: string[]}}
 */
export function vetDayWindow(want, bounds) {
  const lo = bounds.lo || '';
  const hi = bounds.hi || '';
  /** @param {string} d */
  const inside = (d) => (d && (!lo || d >= lo) && (!hi || d <= hi) ? d : '');
  const fromKept = inside(want.from ?? '');
  const toKept = inside(want.to ?? '');
  const out = { from: want.from ?? '', to: want.to ?? '', notes: /** @type {string[]} */ ([]) };
  for (const [k, label, edge, min, max] of /** @type {const} */ ([
    ['from', 'From date', 'Only bars on or after this day.', lo, toKept || hi],
    ['to', 'To date', 'Only bars on or before this day.', fromKept || lo, hi]
  ])) {
    const v = want[k] ?? '';
    if (!v) continue;
    const r = checkDay(v, { label: `${k} (${label})`, lo: min, hi: max, reason: `${bounds.reason ?? ''} ${edge}`.trim() });
    if (r.ok) continue;
    out[k] = '';
    const opens = bounds.opens?.[k] ?? '';
    /* No closing stop: the page joins the link's notes with '; ' and ends
       the list with one ('DbControls'). */
    out.notes.push(`${r.why} The ${label} opens on ${opens || (k === 'from' ? 'the first day held' : 'the last day held')} instead`);
  }
  return out;
}

/**
 * A time of day, `HH:MM` or `HH:MM:SS`, 00:00 to 23:59:59.
 *
 * @param {unknown} raw
 * @param {{label?: string}} [o]
 * @returns {Good | Bad}
 */
export function checkTime(raw, o = {}) {
  const label = o.label ?? 'the time';
  const long = checkText(raw, { label, max: 16 });
  if (!long.ok) return long;
  const s = long.value;
  if (!TIME.test(s)) return { ok: false, why: `${label} ${quote(s)} is not a time of day in HH:MM or HH:MM:SS form, 00:00 to 23:59:59` };
  return { ok: true, value: s };
}

/**
 * A From/To time window that runs backwards, or `null`. Compared as text,
 * which is exact for fixed-width `HH:MM[:SS]`; a minute bound is widened to
 * its own seconds so `10:04` vs `10:04:30` is not called backwards.
 *
 * @param {string} from
 * @param {string} to
 * @returns {string | null}
 */
export function timeWindowWhy(from, to) {
  if (!from || !to) return null;
  const f = from.length === 5 ? `${from}:00` : from;
  const t = to.length === 5 ? `${to}:59` : to;
  return f > t ? `From time ${from} is after To time ${to}, so no bar of a day falls between them` : null;
}

/**
 * A whole number in `[min, max]`. `abc`, `-1`, `0` (when `min` is 1), `1.5`
 * and nine-plus digits past `max` are refused, each in words.
 *
 * @param {unknown} raw
 * @param {{label?: string, min?: number, max?: number}} [o]
 * @returns {{ok: true, value: number} | Bad}
 */
export function checkWhole(raw, o = {}) {
  const label = o.label ?? 'the number';
  const min = o.min ?? 0;
  const max = o.max ?? Number.MAX_SAFE_INTEGER;
  const long = checkText(raw, { label, max: 32 });
  if (!long.ok) return long;
  const s = long.value;
  if (!/^\d+$/.test(s)) return { ok: false, why: `${label} ${quote(s)} is not a whole number from ${min.toLocaleString('en-IN')}` };
  const n = Number(s);
  if (!Number.isSafeInteger(n) || n < min || n > max) {
    return { ok: false, why: `${label} ${quote(s)} is outside ${min.toLocaleString('en-IN')} to ${max.toLocaleString('en-IN')}` };
  }
  return { ok: true, value: n };
}

/**
 * One of a closed set of words, with every accepted word named in the
 * refusal.
 *
 * @param {unknown} raw
 * @param {Iterable<string>} allowed
 * @param {{label?: string}} [o]
 * @returns {Good | Bad}
 */
export function checkWord(raw, allowed, o = {}) {
  const label = o.label ?? 'the value';
  const long = checkText(raw, { label, max: 64 });
  if (!long.ok) return long;
  const list = [...allowed];
  if (list.includes(long.value)) return { ok: true, value: long.value };
  return { ok: false, why: `${label} ${quote(long.value)} is not one this page knows (it reads ${list.join(', ')})` };
}

/**
 * @typedef {object} AddressVocabulary what this build of /db knows, from the
 *   modules that define it (never a second copy): universe keys from
 *   `notes-controls.js` `UNIVERSES`, rows per page from `bars-plan.js`
 *   `PAGE_SIZES`, bar sorts from `notes-bars.js` `BAR_COLS` and Coverage sorts
 *   from `notes-coverage.js` `COLS`.
 * @property {Iterable<string>} universes
 * @property {Iterable<number | string>} rows
 * @property {Iterable<string>} barSorts
 * @property {Iterable<string>} censusSorts
 */

/**
 * The address's WELL-FORMED values checked against what this page knows. A
 * value it does not know is removed from `sel` (so the page keeps its
 * default) and named in one sentence each — `sort=bogus` opens newest first
 * AND says so, never one without the other.
 *
 * @param {Record<string, any>} sel a `decodeDb` selection; changed in place
 * @param {AddressVocabulary} vocab
 * @returns {string[]}
 */
export function vetAddress(sel, vocab) {
  /** @type {string[]} */
  const notes = [];
  if (sel.universe !== undefined) {
    const r = checkWord(sel.universe, vocab.universes, { label: 'universe' });
    if (!r.ok) {
      notes.push(`${r.why}; the universe stays at Everything`);
      delete sel.universe;
    }
  }
  if (sel.rows !== undefined && sel.rows !== 'fit') {
    const r = checkWord(String(sel.rows), [...vocab.rows].map(String), { label: 'rows' });
    if (!r.ok) {
      notes.push(`${r.why}; rows per page stays at Fit`);
      delete sel.rows;
    }
  }
  if (sel.sort !== undefined) {
    const census = sel.view === 'census';
    const r = checkWord(sel.sort, census ? vocab.censusSorts : vocab.barSorts, {
      label: census ? 'sort (Coverage)' : 'sort (Bars)'
    });
    if (!r.ok) {
      notes.push(`${r.why}; the grid opens in its own order${census ? '' : ', newest first'}`);
      delete sel.sort;
    }
  }
  return notes;
}

/**
 * Whether a page number a link asked for exists, said in words when it does
 * not. `count` is the page count of the answer that came back for it.
 *
 * @param {number} asked
 * @param {number} count
 * @returns {string | null}
 */
export function pageWhy(asked, count) {
  if (!Number.isSafeInteger(asked) || !Number.isSafeInteger(count) || count < 1 || asked <= count) return null;
  return `page ${asked.toLocaleString('en-IN')} does not exist: this selection has ${count.toLocaleString('en-IN')} page(s) at this many rows per page, so the last one, page ${count.toLocaleString('en-IN')}, is shown`;
}

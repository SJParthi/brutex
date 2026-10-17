/**
 * /db — THE SELECTION MODEL, PURE (WP-50; D-3337, D-3338).
 *
 * Everything the selection decides that is not a rune: which segment a stored
 * row belongs to, the exact-instrument index, the held span of a selection,
 * each feed's era, the session grid a time picker lists, the weekly/monthly
 * and holiday-shift tags of an expiry, the exchange calendar's day codes, and
 * what a feed change, a segment change or an instrument change drops.
 *
 * A PLAIN MODULE, so `web/tests/db-selection.test.js` drives it under
 * `node --test` with no compiler. `selection.svelte.js` holds the runes and
 * calls these; nothing here reads a store, a request or a clock it was not
 * handed.
 *
 * Nothing here is NIFTY-specific. `DEFAULT_INSTRUMENT` is the operator's
 * stated opening choice (D-3337 item 2) and is used only to SEED an empty
 * rung; every rule below is the same for every underlying.
 */
import { istDay } from '../dates.js';
import { parseKey } from '../instrument.js';

/** The operator's opening instrument (D-3337 item 2, operator answer 11). */
export const DEFAULT_INSTRUMENT = 'NIFTY';
/** The operator's opening segment (D-3337 item 2). */
export const DEFAULT_SEGMENT = 'spot';

/* ======================================================================
   SEGMENTS — what a stored row IS, against today and the archive's end
   ----------------------------------------------------------------------
   D-3337 item 6: "Expired options" holds only contracts whose settled expiry
   is before today's IST day. A contract expiring today or later has not
   settled, and a contract that settled after the vendor archive's last day
   has a life the archive cut short (its final sessions are not held). Both
   are listed apart and say so — never filed as "already settled" (DB-G67).
   ====================================================================== */

/** The seven segment keys, in the order the picker lists them. */
export const SEGMENT_KEYS = [
  'spot',
  'futures',
  'options',
  'futures-cut',
  'options-cut',
  'futures-live',
  'options-live'
];

/**
 * Which segment a stored row belongs to.
 *
 * @param {{ expiry: string | null, instrument: string }} row a census row
 *        (`expiry` is the ISO day read off the name, `null` for spot)
 * @param {string} today today's IST day, `YYYY-MM-DD`
 * @param {string | null | undefined} archiveTo the feed's archive end
 *        (`/feeds.json` `archive.to`), or nothing for a feed with no archive
 * @returns {string} one of `SEGMENT_KEYS`
 */
export function segmentOf(row, today, archiveTo) {
  if (row.expiry === null || row.expiry === undefined) return 'spot';
  const base = /-(CE|PE)$/.test(row.instrument) ? 'options' : 'futures';
  /* ISO days compare as strings: fixed width, most significant first. */
  if (today && row.expiry >= today) return `${base}-live`;
  if (archiveTo && row.expiry > archiveTo) return `${base}-cut`;
  return base;
}

/**
 * The contract rungs a segment has: spot none; futures an expiry; options
 * expiry, strike, moneyness and side. The `-cut` and `-live` variants are the
 * same contracts and have the same rungs.
 *
 * @param {string} seg
 */
export function contractRungsOf(seg) {
  const base = String(seg ?? '').replace(/-(cut|live)$/, '');
  if (base === 'futures') return { expiry: true, strike: false, money: false, side: false };
  if (base === 'options') return { expiry: true, strike: true, money: true, side: true };
  return { expiry: false, strike: false, money: false, side: false };
}

/* ======================================================================
   THE EXACT INSTRUMENT (DB-001)
   ----------------------------------------------------------------------
   A picked instrument is ONE underlying, matched by equality. The old rung
   wrote a text box and filtered with `startsWith`, so NIFTY brought NIFTY100,
   NIFTYIT and NIFTYCPSE with it and LT brought LTF and LTM. The index below is
   one pass per census (or universe) change; a pick is then one Map probe.
   ====================================================================== */

/**
 * Fold census rows by underlying.
 *
 * `short` sums SPOT months only: a contract-month measured against the
 * fullest series of its month is not a shortfall (a thinly traded strike can
 * never reach a 1s index), and summing it into one figure printed
 * "4,84,12,39,862 short" for NIFTY.
 *
 * One Map probe and two counters per row: the per-underlying `segs` and
 * `keys` sets the old fold also built had no reader, and cost a Set insert
 * per row on a 2,46,147-row census.
 *
 * @param {Iterable<any>} rows decorated census rows
 * @returns {Map<string, {key: string, months: number, bars: number, short: number,
 *   spotMonths: number, contractMonths: number, rows: any[]}>}
 */
export function foldInstruments(rows) {
  const by = new Map();
  for (const it of rows) {
    let a = by.get(it.underlying);
    if (!a) {
      a = {
        key: it.underlying,
        months: 0,
        bars: 0,
        short: 0,
        spotMonths: 0,
        contractMonths: 0,
        rows: []
      };
      by.set(it.underlying, a);
    }
    a.months += 1;
    a.bars += it.rows;
    if (it.expiry === null || it.expiry === undefined) {
      a.spotMonths += 1;
      a.short += it.short ?? 0;
    } else {
      a.contractMonths += 1;
    }
    a.rows.push(it);
  }
  return by;
}

/**
 * The opening instrument: the operator's default when this feed holds it,
 * else the first offered name with rows behind it, else the first offered.
 * `null` when nothing is offered.
 *
 * @param {Array<{key: string, months: number}>} offered
 * @returns {string | null}
 */
export function defaultInstrument(offered) {
  if (!Array.isArray(offered) || offered.length === 0) return null;
  const pick =
    offered.find((r) => r.key === DEFAULT_INSTRUMENT && r.months > 0) ??
    offered.find((r) => r.months > 0) ??
    offered[0];
  return pick.key;
}

/**
 * The opening segment: Spot when the instrument holds it, else the first
 * segment it holds, else Spot. `rows` are the segment picker's rows, each with
 * `why === undefined` when the instrument holds rows under it.
 *
 * @param {Array<{key: string, why?: string}>} rows
 * @returns {string}
 */
export function defaultSegment(rows) {
  const held = rows.filter((s) => s.why === undefined);
  return (held.find((s) => s.key === DEFAULT_SEGMENT) ?? held[0] ?? rows[0])?.key ?? DEFAULT_SEGMENT;
}

/* ======================================================================
   DAYS — the held span, the feed's era, the exchange calendar
   ====================================================================== */

/**
 * The first and last IST day the given census rows hold, from their own
 * `first_ts`/`last_ts` (decorated to `firstAt`/`lastAt`, milliseconds). This
 * is the CENSUS's answer for the whole selection — never the rows on screen,
 * which once reported "24 Sep 2026 to 24 Sep 2026" for a series held from
 * 2018 (DB-045).
 *
 * One pass over the rows; two clock conversions.
 *
 * @param {Iterable<{firstAt: number | null, lastAt: number | null}>} rows
 * @returns {{lo: string, hi: string}} `''` on a side nothing measured
 */
export function heldSpan(rows) {
  let lo = Number.POSITIVE_INFINITY;
  let hi = Number.NEGATIVE_INFINITY;
  for (const r of rows) {
    if (typeof r.firstAt === 'number' && r.firstAt > 0 && r.firstAt < lo) lo = r.firstAt;
    if (typeof r.lastAt === 'number' && r.lastAt > 0 && r.lastAt > hi) hi = r.lastAt;
  }
  return {
    lo: Number.isFinite(lo) ? istDay(lo) : '',
    hi: Number.isFinite(hi) ? istDay(hi) : ''
  };
}

/**
 * The span a feed answers for, from its own `/feeds.json` descriptor
 * (D-3338): GDFL's measured `archive`, Zerodha's operator `window`. With
 * `oow` (show months outside the window) the window's ends stop bounding.
 *
 * @param {any} feed a `/feeds.json` row, or nothing
 * @param {boolean} oow
 * @returns {{from: string, to: string, kind: 'archive' | 'window' | 'refused' | 'none', label: string}}
 */
export function eraOf(feed, oow = false) {
  const archive = feed?.archive;
  if (archive && typeof archive.from === 'string' && typeof archive.to === 'string') {
    return { from: archive.from, to: archive.to, kind: 'archive', label: `${feed.display ?? feed.wire}'s archive` };
  }
  const window = feed?.window;
  if (window && typeof window.from === 'string' && typeof window.to === 'string') {
    return oow
      ? { from: '', to: '', kind: 'window', label: 'every held month, including those outside your window' }
      : { from: window.from, to: window.to, kind: 'window', label: 'your window' };
  }
  // A WINDOW THE SERVER REFUSED (`why`, no days: the settings could not be
  // read, or the window in force is inverted) is SAID, never read as a feed
  // with no era (R3-05 repair 3, the second agent's follow-up).
  if (window && typeof window.why === 'string') {
    return { from: '', to: '', kind: 'refused', label: `the server refused your window: ${window.why}` };
  }
  return { from: '', to: '', kind: 'none', label: '' };
}

/**
 * Whether a census month lies outside a feed's operator window (D-3338).
 *
 * @param {string} month `YYYY-MM`
 * @param {any} feed
 */
export function outsideWindow(month, feed) {
  const w = feed?.window;
  if (!w || typeof w.from !== 'string' || typeof w.to !== 'string') return false;
  return month < w.from.slice(0, 7) || month > w.to.slice(0, 7);
}

/** `YYYY-MM-DD` to days since 1970-01-01 (a calendar day, no instant). */
export function epochDayOf(/** @type {string} */ iso) {
  const y = Number(iso.slice(0, 4));
  const m = Number(iso.slice(5, 7));
  const d = Number(iso.slice(8, 10));
  return Math.round(Date.UTC(y, m - 1, d) / 86_400_000);
}

/** Days since 1970-01-01 to `YYYY-MM-DD`. */
export function isoOfEpoch(/** @type {number} */ day) {
  return new Date(day * 86_400_000).toISOString().slice(0, 10);
}

/**
 * The exchange calendar of record out of a `/calendar.json` answer (D-3336):
 * a dense day-code string from epoch day `from`. `null` when the answer does
 * not carry one (a withheld calendar names its reason in `reason`).
 *
 * @param {any} body
 * @returns {{from: number, days: string, fromDate: string, toDate: string} | null}
 */
export function calendarOf(body) {
  if (typeof body?.days !== 'string' || !Number.isSafeInteger(body?.from)) return null;
  return {
    from: body.from,
    days: body.days,
    fromDate: isoOfEpoch(body.from),
    toDate: isoOfEpoch(body.from + body.days.length - 1)
  };
}

/**
 * One day's code: `.` closed, `R`/`X` regular, `P` special, `U` unmeasured,
 * `H` suspected halt, `-` outside; `null` when the calendar does not reach.
 *
 * @param {{from: number, days: string} | null} cal
 * @param {string} iso
 * @returns {string | null}
 */
export function dayCode(cal, iso) {
  if (!cal || !/^\d{4}-\d{2}-\d{2}$/.test(iso)) return null;
  const at = epochDayOf(iso) - cal.from;
  if (at < 0 || at >= cal.days.length) return null;
  const code = cal.days[at];
  return code === '-' ? null : code;
}

/* ======================================================================
   THE SESSION GRID (DB-074)
   ----------------------------------------------------------------------
   The time pickers list the RUNG'S SESSION GRID, not the minutes of the rows
   on screen (a page of 1s bars at 15:29 offered only "15:29"). The regular
   session is 09:15 inclusive to 15:30 exclusive: `docs/00-charter.md` §3,
   `pull::session::SESSION_OPEN_MINUTE` (555) and `SESSION_CLOSE_MINUTE` (930).
   Minutes OUTSIDE it (a pre-open print, a closing-auction print) are listed
   only where the loaded rows actually hold one, and say so: no session hour
   is written here that the charter does not state.
   ====================================================================== */

export const SESSION_OPEN_MINUTE = 9 * 60 + 15;
export const SESSION_CLOSE_MINUTE = 15 * 60 + 30;

/** @param {number} m minutes since IST midnight */
export const hhmm = (m) => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;

/**
 * The minutes a bar of this rung can open on, regular session first, plus any
 * held minute outside it.
 *
 * @param {number | undefined} rungSecs the rung's length (`TF_SECS`)
 * @param {Iterable<string>} [held] `HH:MM` minutes the loaded rows hold
 * @returns {Array<{key: string, name: string, detail: string, why?: string}>}
 */
export function sessionGrid(rungSecs, held = []) {
  if (!Number.isFinite(rungSecs) || /** @type {number} */ (rungSecs) <= 0 || /** @type {number} */ (rungSecs) >= 86_400) return [];
  const step = Math.max(1, Math.round(Math.max(/** @type {number} */ (rungSecs), 60) / 60));
  /** @type {Map<string, {key: string, name: string, detail: string, why?: string}>} */
  const out = new Map();
  for (let m = SESSION_OPEN_MINUTE; m < SESSION_CLOSE_MINUTE; m += step) {
    const t = hhmm(m);
    out.set(t, { key: t, name: t, detail: 'regular session' });
  }
  for (const t of held) {
    if (!/^\d{2}:\d{2}$/.test(t) || out.has(t)) continue;
    const m = Number(t.slice(0, 2)) * 60 + Number(t.slice(3, 5));
    const before = m < SESSION_OPEN_MINUTE;
    const after = m >= SESSION_CLOSE_MINUTE;
    out.set(t, {
      key: t,
      name: t,
      detail: before ? 'before 09:15' : after ? 'after 15:30' : 'held',
      why: before
        ? 'Outside the regular session (charter §3: 09:15 to 15:30); listed because a loaded bar opens here, such as a pre-open print.'
        : after
          ? 'Outside the regular session (charter §3: 09:15 to 15:30); listed because a loaded bar opens here, such as a closing-auction print.'
          : 'Off this rung’s grid; listed because a loaded bar opens here.'
    });
  }
  return [...out.values()].sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));
}

/** The sixty seconds of a minute, for the 1s rung's seconds picker. */
export const SECONDS = Array.from({ length: 60 }, (_, s) => String(s).padStart(2, '0'));

/**
 * A time bound at the rung's resolution: `HH:MM:SS` on a sub-minute rung,
 * `HH:MM` otherwise. A FROM bound starts at second `00` and a TO bound ends at
 * second `59` unless a second is given, so "09:15 to 09:16" on 1s covers both
 * whole minutes. Any other input is `''` (no bound).
 *
 * @param {string} minute `HH:MM`, or `''`
 * @param {number | undefined} rungSecs
 * @param {'from' | 'to'} end
 * @param {string | null} [second] `SS`
 */
export function timeBound(minute, rungSecs, end, second = null) {
  if (!/^\d{2}:\d{2}$/.test(String(minute ?? ''))) return '';
  if (!(Number.isFinite(rungSecs) && /** @type {number} */ (rungSecs) < 60)) return minute;
  const s = second !== null && /^[0-5]\d$/.test(second) ? second : end === 'from' ? '00' : '59';
  return `${minute}:${s}`;
}

/**
 * A time bound re-expressed at another rung's resolution: seconds are kept on
 * a sub-minute rung and dropped (floored to the minute) on a minute rung, so a
 * bound carried across a feed or rung change never compares `HH:MM` labels
 * against `HH:MM:SS` text. Invalid text is `''`.
 *
 * @param {string} bound
 * @param {number | undefined} rungSecs
 * @param {'from' | 'to'} end
 */
export function rebound(bound, rungSecs, end) {
  const b = String(bound ?? '');
  if (!/^\d{2}:\d{2}(:\d{2})?$/.test(b)) return '';
  return timeBound(b.slice(0, 5), rungSecs, end, b.length === 8 ? b.slice(6, 8) : null);
}

/* ======================================================================
   EXPIRY TAGS (DB-076)
   ----------------------------------------------------------------------
   Every tag is MEASURED from the stored expiries and the exchange calendar,
   never from a written rule about which weekday an index expires on (those
   rules changed in 2019, 2023, 2024 and 2025 and none is sourced here):

   * weekday — calendar arithmetic on the ISO day;
   * monthly — the LAST stored expiry of its calendar month for this
     underlying; every earlier one in that month is weekly;
   * shift — the expiry's weekday differs from the weekday its neighbours
     (stored expiries of the same underlying within six weeks) mostly fall
     on, and the first such weekday after it is a day the calendar of record
     marks closed: "moved from Thu 21 Mar 2019, a closed day". Where no closed
     day explains the difference it is said as that, with no cause invented.
   ====================================================================== */

export const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

/** 0 = Sunday, by the calendar (Sakamoto), unmovable by the reader's zone. */
export function weekdayOf(/** @type {string} */ iso) {
  const t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
  let y = Number(iso.slice(0, 4));
  const m = Number(iso.slice(5, 7));
  const d = Number(iso.slice(8, 10));
  if (m < 3) y -= 1;
  return (y + Math.floor(y / 4) - Math.floor(y / 100) + Math.floor(y / 400) + t[m - 1] + d) % 7;
}

/**
 * @param {Iterable<string>} expiries ONE underlying's stored expiries (ISO)
 * @param {(iso: string) => string | null} code the calendar's day code
 * @returns {Map<string, {weekday: string, cadence: 'weekly' | 'monthly',
 *   shift: null | {from: string | null, usual: string, why: string}}>}
 */
export function expiryTags(expiries, code) {
  const list = [...new Set(expiries)].filter((e) => /^\d{4}-\d{2}-\d{2}$/.test(e)).sort();
  /** @type {Map<string, string>} last expiry per month */
  const lastOfMonth = new Map();
  for (const e of list) lastOfMonth.set(e.slice(0, 7), e);
  const days = list.map(epochDayOf);
  const out = new Map();
  for (let i = 0; i < list.length; i += 1) {
    const e = list[i];
    const wd = weekdayOf(e);
    /** @type {number[]} */
    const votes = [0, 0, 0, 0, 0, 0, 0];
    for (let j = 0; j < list.length; j += 1) {
      if (j === i || Math.abs(days[j] - days[i]) > 42) continue;
      votes[weekdayOf(list[j])] += 1;
    }
    const top = Math.max(...votes);
    const leaders = votes.flatMap((v, k) => (v === top && v > 0 ? [k] : []));
    let shift = null;
    if (leaders.length === 1 && leaders[0] !== wd) {
      const usual = leaders[0];
      const ahead = ((usual - wd + 7) % 7) || 7;
      const fromIso = isoOfEpoch(days[i] + ahead);
      const c = code(fromIso);
      shift =
        c === '.'
          ? {
              from: fromIso,
              usual: WEEKDAYS[usual],
              why: `moved from ${WEEKDAYS[usual]} ${fromIso}, a day the exchange calendar of record marks closed`
            }
          : {
              from: null,
              usual: WEEKDAYS[usual],
              why:
                c === null
                  ? `falls on a ${WEEKDAYS[wd]} while this underlying's nearby expiries fall on a ${WEEKDAYS[usual]}; the calendar of record does not reach ${fromIso}, so no cause is named`
                  : `falls on a ${WEEKDAYS[wd]} while this underlying's nearby expiries fall on a ${WEEKDAYS[usual]}; ${fromIso} was not a closed day, so this is not a holiday shift`
            };
    }
    out.set(e, {
      weekday: WEEKDAYS[wd],
      cadence: lastOfMonth.get(e.slice(0, 7)) === e ? 'monthly' : 'weekly',
      shift
    });
  }
  return out;
}

/* ======================================================================
   WHAT A CHANGE DROPS (DB-019, DB-020)
   ====================================================================== */

/**
 * The contract picks a selection keeps after a change of feed, instrument or
 * segment: each is kept only where the new census still offers it, and every
 * pick a segment hides is dropped. Pure: the caller assigns the result.
 *
 * @param {{expiry: string, strikes: Set<string>, side: string, money: Set<string>}} picks
 * @param {{rungs: {expiry: boolean, strike: boolean, money: boolean, side: boolean},
 *   expiries: Iterable<string>, strikes: Iterable<string>, sides: Iterable<string>}} offer
 */
export function keepOffered(picks, offer) {
  const exp = new Set(offer.expiries);
  const str = new Set(offer.strikes);
  const sid = new Set(offer.sides);
  const expiry = offer.rungs.expiry && picks.expiry && exp.has(picks.expiry) ? picks.expiry : '';
  const strikes = new Set(
    offer.rungs.strike && expiry ? [...picks.strikes].filter((k) => str.has(k)) : []
  );
  const side = offer.rungs.side && expiry && picks.side && sid.has(picks.side) ? picks.side : '';
  const money = new Set(offer.rungs.money && expiry ? picks.money : []);
  return { expiry, strikes, side, money };
}

/**
 * An instrument key from a link (`key=` in the URL, an audit row, a gap row)
 * read into the rungs it names. A spot key names an underlying and nothing
 * else; a contract names its expiry and, for an option, strike and side.
 *
 * @param {string} key e.g. `NSE-FNO-NIFTY-2019-03-28-1150000-CE`
 * @returns {null | {sym: string, contract: 'spot' | 'futures' | 'options',
 *   expiry: string, strike: string, side: string}}
 */
export function linkOfKey(key) {
  const p = parseKey(key);
  if (!p.underlying || p.why) return null;
  return {
    sym: p.underlying,
    contract: p.kind === 'option' ? 'options' : p.kind === 'future' ? 'futures' : 'spot',
    expiry: p.expiry ?? '',
    strike: p.strike === null ? '' : String(p.strike),
    side: p.side ?? ''
  };
}

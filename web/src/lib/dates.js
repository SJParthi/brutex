/**
 * DATE PRESENTATION — the one definition, and presentation is ALL it is.
 *
 * ────────────────────────────────────────────────────────────────────────────
 * THE RULE THAT MAKES THIS SAFE, and it is the only reason a display-only
 * module is allowed to exist beside the data:
 *
 *   The `YYYY-MM` / ISO form REMAINS THE KEY FORM everywhere it is a key.
 *
 * That means all of these keep the raw string, untouched, forever:
 *   · `{#each …}` key expressions
 *   · `Map` keys and `Set` membership
 *   · range comparisons (`m.month >= from && m.month <= to`)
 *   · sort comparators
 *   · `<input type="date">` values, which the DOM only accepts as `YYYY-MM-DD`
 *   · request parameters — `/bars.json?month=…`, `/store.json?month=…`
 *
 * Reformatting a key in place changes IDENTITY, not presentation. `Jan 2020`
 * is not a month the store has ever heard of, so the lookup misses, the fetch
 * comes back empty, and the page renders a plausible blank instead of an
 * error. That is precisely the fallback that hides a failure which §4 of
 * CLAUDE.md bans, and it is invisible in review because the screen looks
 * right.
 *
 * Sorting in particular must keep comparing epochs or ISO strings and NEVER
 * display text: `YYYY-MM` sorts chronologically as a plain string because the
 * fields are fixed-width and most-significant-first, while `Apr 2020` sorts
 * before `Jan 2019` because `A` < `J`. The display form is alphabetical, and
 * alphabetical is not an order any month has.
 *
 * So: call these AT THE RENDER SITE, on the way to the screen, and nowhere
 * else. Every function here is pure — same input, same output, no state
 * touched, nothing observed but its argument.
 * ────────────────────────────────────────────────────────────────────────────
 */

/**
 * Month names, fixed at three letters, deliberately NOT from `Intl`.
 *
 * `en-IN` and `en-GB` both render September as `Sept` — four letters where
 * every other month is three, which breaks alignment in a monospace column and
 * is not the form Dhan or TradingView use. `en-US` is uniformly three letters
 * but orders the day AFTER the month (`Sep 02`), and switching the page locale
 * wholesale to get it would lose Indian digit grouping (8,78,28,617) on every
 * number on every page — the grouping is the more valuable of the two.
 *
 * A literal table is the only option that gets abbreviation, order and
 * grouping all correct at once. It also cannot drift: CLDR revises locale data
 * between runtime versions, and a column that silently regains a fourth letter
 * on a browser update is a defect nobody would think to look for.
 *
 * @type {readonly string[]}
 */
export const MON = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec'
];

/**
 * The store's month key on the way to a human. `2020-01` -> `Jan 2020`.
 *
 * Returns the input UNCHANGED when it does not parse — empty, null, undefined,
 * a month number outside 1..12, anything. An unparseable month key is a store
 * problem, and it is worth seeing as it actually is rather than as
 * `Invalid Date` or `undefined 2020`, which name neither the value nor the
 * fault.
 *
 * @param {string | null | undefined} ym
 * @returns {string | null | undefined} the label, or `ym` verbatim
 */
export function monthLabel(ym) {
  if (typeof ym !== 'string') return ym;
  const [y, mo] = ym.split('-').map(Number);
  if (!Number.isFinite(y) || !Number.isFinite(mo) || mo < 1 || mo > 12) return ym;
  return `${MON[mo - 1]} ${y}`;
}

/*
 * THE ZONE IS NAMED, not inherited.
 *
 * NSE trades 09:15–15:30 IST. A stamp rendered in the host's zone is not wrong
 * about the instant, it is wrong about the QUESTION — an operator in any other
 * zone reading `03:45` for the open has to do arithmetic to know whether the
 * pull covered the session. Naming `Asia/Kolkata` is exact whatever the
 * machine is set to, costs no library and no network.
 *
 * `Intl` supplies only the numeric fields. The month name comes from the
 * literal table above, for the reason stated there. `hourCycle: 'h23'` is
 * explicit because `hour12: false` alone has historically yielded `24` for
 * midnight on some runtimes, and `24:00` is not a time this product renders.
 */
const IST_PARTS = new Intl.DateTimeFormat('en-GB', {
  timeZone: 'Asia/Kolkata',
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hourCycle: 'h23'
});

/**
 * An instant -> its IST calendar fields, or `null` if there is no instant.
 *
 * A number is epoch MILLISECONDS, matching `new Date(n)`. It is never guessed
 * from magnitude: this repository carries bar times in epoch SECONDS and
 * response times in milliseconds, and a formatter that sniffs which one it was
 * handed would silently render 1970 for half its callers. Seconds multiply by
 * 1000 at the call site, where the unit is known.
 *
 * @param {Date | number | string | null | undefined} d
 * @returns {{ y: string, mo: number, day: string, hh: string, mm: string } | null}
 */
function istFields(d) {
  const ms = d instanceof Date ? d.getTime() : typeof d === 'number' ? d : typeof d === 'string' ? Date.parse(d) : NaN;
  if (!Number.isFinite(ms)) return null;
  /** @type {Record<string, string>} */
  const p = {};
  for (const part of IST_PARTS.formatToParts(new Date(ms))) p[part.type] = part.value;
  const mo = Number(p.month);
  if (!Number.isFinite(mo) || mo < 1 || mo > 12) return null;
  return { y: p.year, mo, day: p.day, hh: p.hour, mm: p.minute };
}

/**
 * TODAY, IN IST, AS `yyyy-mm`. The month a calendar opens on when nothing else
 * has named one.
 *
 * # Why this is here and not in the component that needs it
 *
 * `$lib/DayField.svelte` needs a month to draw when its value and both its
 * bounds are empty. Computing it there would mean a second `Intl.DateTimeFormat`
 * configured with a second copy of the `Asia/Kolkata` / `h23` pair above, and
 * two spellings of "now in IST" are two answers the day one of them is edited.
 * The clock lives in this module because every other date in this product is
 * rendered through it.
 *
 * @param {Date | number | string | null | undefined} [d] Defaults to now.
 * @returns {string} `yyyy-mm`, or `''` when there is no instant to read.
 */
export function istMonth(d) {
  const f = istFields(d ?? Date.now());
  if (f === null) return '';
  return `${f.y}-${String(f.mo).padStart(2, '0')}`;
}

/**
 * A calendar day in IST. `02 Sep 2024`.
 *
 * Accepts a `Date`, epoch milliseconds, or anything `Date.parse` accepts —
 * including the `YYYY-MM-DD` the store and the date inputs speak.
 *
 * Renders an em dash when there is no instant to render, never `Invalid Date`.
 * The em dash is the honest empty; the REASON it is empty belongs beside it at
 * the call site, which is the only place that knows.
 *
 * @param {Date | number | string | null | undefined} d
 * @returns {string}
 */
export function dayLabel(d) {
  const f = istFields(d);
  if (f === null) return '—';
  return `${f.day} ${MON[f.mo - 1]} ${f.y}`;
}

/**
 * A calendar day and a wall-clock minute in IST. `06 Aug 2026, 15:29`.
 *
 * Minutes, not seconds: every stamp this product shows a human is a pull, a
 * last bar or a last answer, and none of them are judged at second resolution.
 * Where seconds matter the page holds the raw value and says so itself.
 *
 * Same inputs and same em dash as `dayLabel`.
 *
 * @param {Date | number | string | null | undefined} d
 * @returns {string}
 */
export function stampLabel(d) {
  const f = istFields(d);
  if (f === null) return '—';
  return `${f.day} ${MON[f.mo - 1]} ${f.y}, ${f.hh}:${f.mm}`;
}

/**
 * The wall-clock minute alone, in IST. `15:29`.
 *
 * The other half of `stampLabel`, for a table that gives the day and the minute
 * their own columns. A bar grid is read down a column — every row's minute under
 * every other row's minute — and a single `06 Aug 2026, 15:29` cell makes that
 * scan impossible: the minute sits at a different x on every row because the day
 * in front of it is a different width.
 *
 * Minutes, not seconds, for the reason `stampLabel` gives.
 *
 * Same inputs and same em dash as `dayLabel`.
 *
 * @param {Date | number | string | null | undefined} d
 * @returns {string}
 */
export function timeLabel(d) {
  const f = istFields(d);
  if (f === null) return '—';
  return `${f.hh}:${f.mm}`;
}

/**
 * /db — WHAT THE COVERAGE GRID SAYS ABOUT ITSELF
 *
 * The census grid's columns, the sentence behind every withheld % change, and
 * the sentence behind a row that is its own denominator.
 *
 * Every line below was MOVED out of `routes/db/+page.svelte` by WP-00 (the /db
 * page split). The only edits to a moved line are the ones the move needs: a
 * page-level name that now lives in another file is imported, or, for state,
 * read as `db.<name>`; a plain module exports what it declares; and a callback
 * over such a value marks its parameter `any`, the way the page already did
 * elsewhere. Nothing was fixed in the move: every defect the audit found is
 * still present byte for byte, and is fixed by the package that owns this file
 * next.
 *
 * Owner after WP-00: WP-54 (coverage, coverage calendar and day flags).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */
import { contractOf, strikeText } from './decoration.js';
import { dayLabel, monthLabel } from '../dates.js';
import { coverPct } from '../completeness.js';
import { fmt } from './notes-labels.js';

/* ---- THE PERCENTAGE COLUMNS ----------------------------------------
   `/store.json` sends INTEGER BASIS POINTS. 125 is +1.25%. It is never a
   float on the wire, because `CLAUDE.md` §7 bans the float for money and a
   ratio derived from money is the same arithmetic — see the handler's own
   comment, which promised this shape for a long time before D-0069 made it
   true. The decimal point is inserted HERE, for display, exactly as it is
   for prices.
   ==================================================================== */

/**
 * Basis points as a signed percentage with a fixed two decimals.
 *
 * THE DIVISION IS EXACT AND IT IS NOT `bps / 100`. The remainder is taken
 * with `%` — an integer operation — and the quotient is a multiple of 100
 * divided by 100, which IEEE returns exactly for every value inside 2^53.
 * A naive `(bps / 100).toFixed(2)` is a float rounding of a value that has an
 * exact decimal expansion, and it would be the one place on this page where a
 * price-derived number went through a float for no reason at all.
 *
 * TWO DECIMALS ALWAYS, AND THE SIGN IS PART OF THE NUMBER. Adaptive precision
 * moves the decimal point from row to row; a `+` that only appears on losses'
 * mirror image makes a column of gains read as a column of magnitudes. Zero
 * carries no sign because it has no direction — the cell is neutral-coloured
 * and says `0.00%`, which is a real answer and not an unknown.
 *
 * @param {number} bps INTEGER basis points off the wire; 125 is +1.25%
 */
// `bpsText` and `dirOf` live in `$lib/bps.js` beside `basisPoints`, because
// the three are one rule: what a ratio rounds to, how it reads, and what
// colour it earns. The `-0` note there explains why this formatter is safe
// only while that producer rounds away from zero.

/** up / down / flat / none — the direction green and red are reserved for.
    `none` is its own word rather than `flat`, because a cell nobody can
    compute and a month that did not move are different facts and must not
    share a colour. */
/** @param {number | null} bps */


/**
 * WHY a percentage is a dash. One sentence per server reason code.
 *
 * `CLAUDE.md` §4 — degrade loudly and NAME THE REASON. A single "n/a" over
 * five distinct facts is the fallback that hides a failure: "this equity can
 * never be shown until a threshold is sourced" and "re-ingest this month and
 * it will appear" are opposite instructions, and one dash cannot say both.
 *
 * An unrecognised code is reported as an unrecognised code, WITH the code.
 * Guessing a sentence for it would be this page inventing a fact about the
 * server, which is the exact failure the map exists to prevent.
 */
export const WHY = {
  corporate_action_unverified:
    'REFUSED, not missing. This instrument can split, bonus or consolidate — a share, or an option on a share — and no corporate-action threshold is sourced anywhere in this repository: docs/00-charter.md names no verified split-and-bonus feed. An unadjusted 1:5 split reads as an 80% overnight crash that is indistinguishable from a real move, so docs/05-decisions.md D-0018 refuses the window loudly rather than printing it. An index, and an option on an index, can never be re-based, and both show a number (D-3325, DB-G66). This cell stays a dash until an operator supplies a threshold with a source.',
  previous_month_not_adjacent:
    'The month held before this one is not the calendar month immediately before it — a whole month is missing between them — so a change across the gap would be a change over an unknown span. Refused, never computed across the hole (D-3325).',
  absent:
    'No file: this month is not stored, so it has no first or last close. The row is drawn so the hole is seen, not because a price was read.',
  not_recorded:
    'The census holds this month and no close was ever read for it — a manifest version-1 entry, or a version-2 one written before its bar file was priced (D-0067). Not zero, and not a price of zero: nobody has looked. Re-ingesting the month records both closes and this fills in.',
  no_earlier_month:
    'This feed holds no bars for the month immediately before this one, so there is no previous change to show. The first month an instrument holds is the ordinary case; a hole in the middle is the other one. It is unknown, never 0.00%.',
  base_not_positive:
    "The month's first close is zero paisa, so the ratio has no base and is undefined. Zero means zero here (CLAUDE.md §7), so this is a real stored price and not a missing one — but nothing can be divided by it.",
  overflow:
    'The move times 10,000 leaves a 64-bit integer, so the server refused it rather than wrapping it into a plausible small number.'
};

/**
 * THE PROBE IS WIDENED AT THE LOOKUP, NOT AT THE TABLE. `WHY` keeps the
 * five-key shape it is written in — that is the fact this page holds, and
 * narrowing it to an index signature would let a sixth key be added above
 * and read below with nothing noticing the sentence was never written. The
 * CODE, by contrast, comes off the wire and may be anything at all, which is
 * the entire reason the fallback line exists. So the widening happens
 * exactly where the unknown key is used, and `undefined` in the cast is what
 * makes the `??` a real branch rather than dead code. `barWhyText` does the
 * same for the bar grid's five reasons.
 *
 * `null` is the ordinary case here and not an error: `chg_why` is null on
 * every row that carries a NUMBER, and this sentence is only rendered where
 * the number is absent.
 *
 * @param {string | null} code
 */
export const whyText = (code) =>
  // AN OWN KEY ONLY: `WHY.constructor` is Object's, never a reason (R3-04).
  Object.hasOwn(WHY, String(code))
    ? /** @type {Record<string, string>} */ (WHY)[String(code)]
    : `The server named a reason this page does not have a sentence for: ${code ?? 'none given'}. That is a mismatch between this page and /store.json, not a fact about the data.`;

/* ======================================================================
   THE COVERAGE VERDICT'S WORDS (D-3336, WP-54)
   ----------------------------------------------------------------------
   Every coverage cell is now measured against the calendar of record
   (`$lib/completeness.js` `judgeRow`), and each note it attaches is one of
   the codes below. A code with no sentence here is shown as its code, never
   as a guessed sentence.
   ====================================================================== */

/** One sentence per note code `judgeRow` attaches. */
export const NOTE_TEXT = {
  outside_window:
    "Outside your window (D-3338): Zerodha is held for 2018-01-01 to 2026-09-30. This month was filed by an earlier run and is kept, never deleted; it is shown because the 'outside your window' toggle is on.",
  derived:
    "Derived: folded from this feed's 1min bars by pull::fold (D-3000), not pulled. A derived month is only as complete as the 1min month it was folded from.",
  derived_no_source: 'Derived, and the 1min month it folds from is not stored, so the fold cannot be checked against its source.',
  derived_behind:
    'Behind its source: the last bar here is earlier than the last 1min bar of the same month by more than one bucket, so the fold has not caught up (DB-013, DB-G12, DB-G35). The rung is re-folded as a new generation (WP-80 step 5).',
  derived_partial:
    'Partial: this derived month holds fewer sessions than the 1min month it is folded from. A bucket is withheld wherever a minute is missing in an index, or a session was not folded (D-3321).',
  series_start:
    "Measured from the series' first stored bar: the days of the month before it are not owed. Whether the vendor had nothing earlier or an import refused the front is in the day band (D-3315).",
  series_end:
    "Measured to the series' last stored bar: its last month is earlier than the last month the store reaches at this rung, so the series ended here (a delisting, a rename, or a pull that stopped). The days after it are not owed to it; the coverage calendar and the day band show them.",
  outside_span: "Part of this month lies outside the calendar of record's span for this feed, and those days are owed nothing because nobody can say what they were — not because they were closed.",
  special:
    'Holds a special session (a Muhurat, a disaster-recovery drill, a weekend budget session or the 2021-02-24 outage), owed by its own recorded windows (D-3319).',
  halt: 'Holds a suspected market-wide halt (2020-03-13, 2020-03-23; no NSE circular recorded, UNVERIFIED). Halted minutes are neither owed nor missing (D-3320, D-3336).',
  unknown_days:
    'Holds a session whose length is not recorded (or, for an index, an outage day with no common index computation, D-0420). Those days are owed nothing here and are named rather than given a guessed length.',
  cas: "From 2026-08-03 a closing-auction-eligible share trades continuously only until 15:15 (charter §9, D-3320): 360 one-minute bars a session, and Zerodha's landing ends at 15:14. Eligibility is per share, from the dated NSE master, and is not on the census wire, so this month is judged against the eligible count and the other count is named.",
  cas_unresolved:
    "Holds auction days, and this share's dated closing-auction flag is not on the census wire: a flag-0 share owes 375 one-minute bars a session and an eligible one 360, so a count between the two is neither full nor short, and a count under both is short by at least the eligible count's shortfall (SC-CLOSE-17).",
  extra: 'Holds bars beyond what the calendar owes: bars on days the calendar of record says were closed, or more than a session holds.',
  surplus_unplaced:
    'A surplus no month total can place: the bars beyond what is owed may stand on one day while another lacks as many, so the count proves no day whole. It is never called full or 100.00% on its count; only a per-day count can clear it (R3-04).',
  unheld_sessions:
    "Short by at least what the sessions it holds no bar on owe, counted day by day from the series' own held days: a surplus on another day never pays for them (R3-04, DB-014).",
  store_wide:
    "A store-wide hole: at least nine in ten series stored at this month and rung hold no more bars than this one (or even the fullest is short by as much), so the pull or import did not land those days for nearly anything. It is a hole, never 'full'. Nine in ten is chosen, not sourced.",
  store_wide_part:
    "Part of this shortfall is a hole every stored series at this month and rung shares (the fullest of them is short of the calendar too); the rest is this series' own.",
  absent_month:
    "Absent: the store holds no file for this month, between the series' first and last stored months, so every bar the calendar owes it is missing (months_missing). It is drawn so the hole is seen; the day band names each day's reason.",
  no_trade_month:
    'No trade this month: a trade-driven series files a month only when a trade printed in it, so a month between its first and last with no file is a month nothing traded — not a hole, and not owed anything.',
  trade_driven:
    "Trade-driven (D-3336 §2): a one-second share or an option holds a bar only for a second (or minute) that printed a trade, so there is no fixed count to owe. It is measured by the sessions its span covers and the days it holds, never against an index's every second."
};

/**
 * The sentence for one note code, or the code itself named as unknown.
 *
 * @param {string} code
 */
export const noteText = (code) =>
  Object.hasOwn(NOTE_TEXT, code)
    ? /** @type {Record<string, string>} */ (NOTE_TEXT)[code]
    : `A coverage note this page has no sentence for: ${code}.`;

/** The state words, as the grid prints them. */
export const STATE_WORD = {
  full: 'full',
  near: 'near',
  gap: 'short',
  absent: 'absent',
  trade: 'trade-driven',
  unmeasured: 'unmeasured',
  over: 'over'
};

/* ======================================================================
   THE COUNTS THE PAGE SHOWS, ONE RULE EACH (R3-04; DB-048, DB-014, DB-046)
   ----------------------------------------------------------------------
   The Bars badge, the Coverage badge, the pager, the Coverage grid and the
   header line beside the tabs each counted their own thing: 434 against
   443 for one selection, 4 against 7 for one tab read twice, a strike
   printed in paisa. Every rule below is a pure function the components
   call and `web/tests/db-counts.test.js` drives.
   ====================================================================== */

/**
 * THE FROM AND TO DAYS AS ONE ORDERED WINDOW, or `null` when they narrow
 * nothing. Coverage follows the same window the Bars grid does (DB-046):
 * the fields narrow only once they differ from the span they were seeded
 * with (`dayNarrowing`). A window typed backwards is put in order, so its
 * range is never printed To before From.
 *
 * @param {string} fromDay `YYYY-MM-DD` or `''`
 * @param {string} toDay `YYYY-MM-DD` or `''`
 * @param {boolean} narrowing
 * @returns {{ from: string, to: string } | null}
 */
export function coverWindow(fromDay, toDay, narrowing) {
  if (!narrowing) return null;
  let from = fromDay || '';
  let to = toDay || '';
  if (from === '' && to === '') return null;
  if (from !== '' && to !== '') [from, to] = [from, to].sort();
  return { from, to };
}

/**
 * Whether a `YYYY-MM` month holds a day of the window. The month is the
 * Coverage grid's row, so a month the window touches on one day is listed,
 * and judged whole (the note under the grid says so).
 *
 * @param {string} month
 * @param {{ from: string, to: string } | null} window
 */
export function monthInWindow(month, window) {
  if (window === null) return true;
  // Its first day is not after To, and its last is not before From: `-31`
  // bounds every day of a month as text, and an empty From bounds nothing.
  return (window.to === '' || `${month}-01` <= window.to) && `${month}-31` >= window.from;
}

/**
 * The window as words, in order: `01 Oct 2024 → 31 Oct 2024`, an open end
 * named as one.
 *
 * @param {{ from: string, to: string }} window
 */
export function windowText(window) {
  return `${window.from ? dayLabel(window.from) : 'the first day held'} → ${window.to ? dayLabel(window.to) : 'the last day held'}`;
}

/**
 * WHY THE COVERAGE GRID IS EMPTY WHEN THE WINDOW EMPTIED IT, said once: the
 * window in order and the days the selection does hold, in order. `''`
 * when the window is not what emptied it (nothing matched before it, or
 * something is left), and the grid's own narrowing sentence stands.
 *
 * @param {{ matched: number, listed: number, window: { from: string, to: string } | null, held: { lo: string, hi: string } | null }} o
 */
export function windowEmptyWhy(o) {
  if (o.window === null || o.matched === 0 || o.listed > 0) return '';
  const [lo, hi] = o.held ? [o.held.lo, o.held.hi].sort() : ['', ''];
  const held = lo && hi ? `; this selection holds ${dayLabel(lo)} → ${dayLabel(hi)}` : '';
  return `The From and To window, ${windowText(o.window)}, holds no month of the ${fmt(o.matched)} instrument-month(s) this selection matches${held}. Coverage follows the window, as the Bars grid does: widen the dates to see them.`;
}

/**
 * THE COUNTED LINE ABOVE THE GRID AND THE DRAWER'S FOUR FIGURES, from ONE
 * function over the rows each shows, so neither can disagree with the rows
 * under it (R3-04). Missing counts an absent month's owed bars, which its
 * row prints beside the word; held is owed less missing, so a per-day floor
 * lowers the coverage too; a row whose missing count is unknown (an
 * unplaced surplus, an unread auction flag) joins neither sum and is
 * counted on its own.
 *
 * @param {Array<{ rows: number, absent?: boolean, cover: import('../completeness.js').Cover }>} rows
 */
export function coverTotals(rows) {
  const t = {
    n: 0, stored: 0, absent: 0, noTrade: 0, full: 0, over: 0, unresolved: 0,
    unverified: 0, trade: 0, missing: 0, owed: 0, held: 0, bars: 0,
    sessions: 0, heldSessions: 0, heldExact: true, storeWide: 0, derivedLagging: 0,
    /** @type {number | null} */
    coverage: null
  };
  for (const r of rows ?? []) {
    const c = r.cover;
    t.n += 1;
    // An absent month's row holds 0 bars (`absentRow`), so every row adds its own.
    t.bars += r.rows;
    if (r.absent) {
      if (c.basis === 'trade') t.noTrade += 1;
      else t.absent += 1;
    } else t.stored += 1;
    if (c.basis === 'none') t.unverified += 1;
    else if (c.basis === 'trade') t.trade += 1;
    if (c.state === 'full') t.full += 1;
    if (c.state === 'over') t.over += 1;
    if (c.notes.includes('cas_unresolved') && c.missing === null) t.unresolved += 1;
    if (c.notes.includes('store_wide')) t.storeWide += 1;
    if (c.notes.includes('derived_behind') || c.notes.includes('derived_partial')) t.derivedLagging += 1;
    t.sessions += c.sessions ?? 0;
    t.heldSessions += c.heldSessions ?? 0;
    if (!c.heldExact) t.heldExact = false;
    if (c.basis === 'calendar' && c.owed !== null && c.missing !== null) {
      t.missing += c.missing;
      t.owed += c.owed;
      t.held += c.owed - c.missing;
    }
  }
  t.coverage = t.owed > 0 ? t.held / t.owed : null;
  return t;
}

/**
 * What one drawer row prints under Missing, the figure the drawer's total
 * sums: `absent −N` for a month with no file, `full`, `+N` for an unplaced
 * surplus, `−N` short, and `—` where nothing is owed or measured.
 *
 * @param {{ absent?: boolean, cover: import('../completeness.js').Cover }} m
 */
export function missingText(m) {
  const c = m.cover;
  if (m.absent && c.basis === 'trade') return 'no trade';
  if (c.basis !== 'calendar' || c.missing === null) return c.state === 'over' ? `+${fmt(c.extra)}` : '—';
  if (m.absent) return `absent −${fmt(c.missing)}`;
  if (c.missing === 0) return 'full';
  return `−${fmt(c.missing)}`;
}

/**
 * THE SET A BAR COUNT BELONGS TO: feed, rung, months, series and bounds —
 * never the page or the order, so a count read on one page stands for
 * every page and every order of the same query. Built from the same plan
 * and bounds the bar read sends.
 *
 * @param {string} feed
 * @param {{ timeframe?: string, from?: string, to?: string, keys?: string[] } | null | undefined} plan
 * @param {{ fromDay?: string, toDay?: string, fromTime?: string, toTime?: string } | null | undefined} bounds
 * @param {unknown} nonce the page's re-read stamp
 */
export function barsQueryKey(feed, plan, bounds, nonce) {
  // `join` writes an absent field (`undefined`, `null`) as nothing.
  return [
    feed, plan?.timeframe, plan?.from, plan?.to, (plan?.keys ?? []).join(','),
    bounds?.fromDay, bounds?.toDay, bounds?.fromTime, bounds?.toTime, nonce
  ].join('|');
}

/**
 * THE BARS TAB'S COUNT, one number for one selection and one window, on
 * either tab and by link (R3-04, DB-048). It costs no request: it reads the
 * window answer the Bars grid already fetched.
 *
 * - the server's total for this query, live on the Bars tab or remembered
 *   from it on Coverage (leaving Bars drops the rows, by design, and the
 *   badge fell back to the census: 443 against the pager's 434);
 * - else, with no day or time bound, the census's stored count of the
 *   matched months — the same set, which the census banner says when it
 *   lags the files;
 * - else nothing: the census cannot count a day or time window, and a
 *   number for a different set would be the disagreement this removes.
 *
 * @param {{ said: number | null, remembered: { key: string, total: number } | null, key: string, census: number, narrows: boolean }} o
 * @returns {{ n: number | null, from: 'server' | 'census' | 'none', title: string }}
 */
export function barsBadge(o) {
  const server = o.said ?? (o.remembered !== null && o.remembered.key === o.key ? o.remembered.total : null);
  if (server !== null) {
    return {
      n: server,
      from: 'server',
      title: `${fmt(server)} bar(s) this query keeps — the server's count over every matched month and series, after the date and time bounds; the same number the Bars pager counts, on either tab.`
    };
  }
  if (!o.narrows) {
    return {
      n: o.census,
      from: 'census',
      title: `${fmt(o.census)} bar(s) stored in the matched months, by the census; no date or time bound narrows them. The Bars grid's count is the server's, and the census banner names any lag between the two.`
    };
  }
  return {
    n: null,
    from: 'none',
    title: 'Not counted yet: a date or time bound narrows this query and only the server counts a window. Open Bars and its one read counts it; the number then stays here on either tab.'
  };
}

/**
 * THE HEADER LINE'S RANGE: the From and To window in view when it narrows,
 * else the matched months on disk — in order either way.
 *
 * @param {string[]} months the matched months, any order
 * @param {{ from: string, to: string } | null} window
 * @returns {{ from: string, to: string, word: string } | null}
 */
export function headerRange(months, window) {
  const sorted = [...months].sort();
  const first = sorted.length > 0 ? month(sorted[0]) : '—';
  const last = sorted.length > 0 ? month(sorted[sorted.length - 1]) : '—';
  if (window === null) return sorted.length > 0 ? { from: first, to: last, word: 'on disk' } : null;
  return { from: window.from ? dayLabel(window.from) : first, to: window.to ? dayLabel(window.to) : last, word: 'in view' };
}

/**
 * A month key as words. `monthLabel` hands a string back for any string
 * (verbatim when it does not parse), so the cast states what it returns.
 *
 * @param {string} ym
 */
function month(ym) {
  return /** @type {string} */ (monthLabel(ym));
}

/**
 * Whether the newest stored bar falls inside the window: `in`, or `after`
 * / `before` it. The close beside the range describes the window in view,
 * so one outside it is named as outside, never printed as the window's.
 *
 * @param {number} ms the bar's epoch milliseconds
 * @param {{ from: string, to: string } | null} window
 * @param {(ms: number) => string} isoDay the bar's IST day, `YYYY-MM-DD`
 * @returns {'in' | 'after' | 'before'}
 */
export function closeInWindow(ms, window, isoDay) {
  if (window === null) return 'in';
  const day = isoDay(ms);
  if (window.to !== '' && day > window.to) return 'after';
  // An empty From is before every day.
  return day < window.from ? 'before' : 'in';
}

/**
 * A contract's tail as the header prints it: `11,650 CE` (the strike in
 * rupees, never `1165000 CE` in paisa), `FUT` for a future, `''` for a key
 * that is no contract.
 *
 * @param {string} key a census instrument key
 */
export function contractTail(key) {
  const ct = contractOf(key);
  if (ct !== null) return `${strikeText(ct.strike)} ${ct.side}`;
  return /-\d{4}-\d{2}-\d{2}-FUT$/.test(key) ? 'FUT' : '';
}

/**
 * REFUSED FRONTS THE DAY BAND READ (D-3315), as one sentence: days before a
 * series' first `1day` bar that a pull refused before append, so the rung
 * holds no daily bar there. Coverage measures a series from its first bar,
 * so these days are never counted missing; the box beside the coverage
 * figure and the empty grid both print this sentence, so the two say the
 * same thing. `null` when the band read none. Cells are the band's own,
 * oldest first (`bandOf`).
 *
 * @param {Array<{ day: string, kinds?: string[] }> | null | undefined} cells
 * @param {string} instrument the band's instrument, `''` when it names none
 * @returns {{ n: number, from: string, to: string, text: string } | null}
 */
export function refusedFrontOf(cells, instrument) {
  const days = (cells ?? []).filter((c) => Array.isArray(c.kinds) && c.kinds.includes('front'));
  if (days.length === 0) return null;
  const from = days[0].day;
  const to = days[days.length - 1].day;
  return {
    n: days.length,
    from,
    to,
    text: `${fmt(days.length)} day(s) of a refused front, ${dayLabel(from)} → ${dayLabel(to)}${instrument ? ` (${instrument})` : ''}: the vendor served them and a pull refused them before append, so no daily bar is stored there (D-3315). Coverage measures from the first stored bar, so these days are named here and never counted as held.`
  };
}

/** A coverage ratio, never 100.00% for a month missing a bar (`coverPct`). */
export { coverPct };

/** What the page says instead of a verdict it cannot support. */
export const SOLE_WHY =
  'This is the only instrument-month stored at this rung for this month, so the denominator is this row itself. ' +
  'Being short of itself by zero is a tautology, not a measurement: nothing here says the month is complete, and nothing says it is not. ' +
  'Pull a second instrument for this month, or read the Days column beside it.';

/* ======================================================================
   THE TWO PERCENTAGE COLUMNS — WHAT THEY ARE, AND WHY MOST CELLS ARE A DASH
   ----------------------------------------------------------------------
   Every number in this panel is COUNTED FROM THE ROWS ON SCREEN'S OWN
   DATASET, never typed in. The previous version of this block asserted that
   `/store.json` carries no price, which was true when it was written and
   false the day D-0069 landed — a hardcoded claim about the server outliving
   the server. Counting means the panel cannot say "0 refused" over a table
   full of dashes, or the reverse.
   ====================================================================== */

/** Bytes per bar on `/bars.json`, MEASURED: 668,251 bytes for 8,250 bars.
    A constant because it is a property of the wire format, not of the store;
    everything multiplied by it below is counted from the live data instead
    of typed in, so this panel cannot drift into asserting a row count the
    page beside it contradicts. */
export const BYTES_PER_BAR = 81.0;

/**
 * Bytes, at the magnitude that reads as a quantity rather than as `0.0`.
 *
 * @param {number} n
 */
export function bytesText(n) {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)} kB`;
  return `${Math.round(n)} B`;
}

/* ======================================================================
   SORT
   ----------------------------------------------------------------------
   Codepoint comparison, not `localeCompare`: instruments are `[A-Z0-9-_&]`
   by construction, so collation adds nothing and would make the order a
   function of the operator's locale. Same inputs, same order, everywhere.
   ====================================================================== */
export const COLS = [
  { key: 'instrument', label: 'Instrument', num: false },
  { key: 'month', label: 'Month', num: false },
  { key: 'timeframe', label: 'TF', num: false },
  { key: 'rows', label: 'Bars', num: true },
  {
    key: 'days',
    label: 'Sessions',
    num: true,
    why: 'sessions held of the sessions the calendar of record owes in the month (counted from the series\' own days where its calendar was read, else bars over the average session)'
  },
  {
    key: 'short',
    label: 'Missing',
    num: true,
    why: 'bars the calendar of record owes at this rung and the store does not hold (D-3336); trade-driven series owe none'
  },
  {
    key: 'pct',
    label: 'Coverage',
    num: true,
    why: 'bars held over bars the calendar owes this month at this rung — never against the fullest stored series'
  },
  // THESE TWO SORT NOW. They used to carry `key: null` with the comment "a
  // control that reorders 20,516 identical blanks is a control that lies
  // about having data" — which was right while they were blank and is wrong
  // now that they hold basis points. The rule the old comment states is
  // unchanged: EVERY COLUMN THAT HOLDS DATA SORTS. These hold data.
  {
    key: 'chg',
    label: '% in month',
    num: true,
    why: "this month's OWN first close to its last close (D-0069) — it leaves out the move from the previous month's close to this month's first; the row drawer shows the change vs the previous month's close beside it"
  },
  {
    key: 'prevChg',
    label: 'Prev month %',
    num: true,
    why: "the previous calendar month's in-month % for the same instrument (its own first close to its own last)"
  }
];

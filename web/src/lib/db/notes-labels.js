/**
 * /db — RUNG LENGTHS, SESSION SIZE AND THE TWO FORMATTERS EVERY READER USES
 *
 * The bar-length ladder (`TF_SECS`), the session size the completeness column
 * divides by (`BARS_PER_SESSION`), the label under the Timeframe control
 * (`tfNote`), the number formatter (`fmt`) and the three-way comparator (`txt`).
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
 * Owner after WP-00: WP-55 (price-basis and close-semantics labels; it fixes the `BARS_PER_SESSION` comment).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */
import { exact, rupee } from '../money.js';
import { dayLabel, istDay, secondLabel } from '../dates.js';
import { parseKey } from '../instrument.js';

/**
 * 375 IS ONE SESSION OF OPENING-STAMPED ONE-MINUTE BARS, 09:15–15:29
 * inclusive: the 09:15–15:30 session of an index, and of every share and
 * derivative before 2026-08-03. Stated because a completeness figure with an
 * unstated denominator is a number nobody can check.
 *
 * FROM 2026-08-03 THERE ARE THREE SESSIONS, NOT ONE (DB-082; charter §7 and
 * §9; D-3320). This comment used to put every continuous close at 15:29 from
 * that day, which is none of them:
 *
 * - **an index**: unchanged, 09:15–15:30 — 375 minutes;
 * - **a closing-auction-eligible share** (NSE/CMTR/73845, dated per share by
 *   the `NSE_CM_security` file): continuous trading 09:15–15:15, then the
 *   closing auction to 15:30 — 360 minute bars, 09:15–15:14, which is what
 *   Zerodha's minutes hold by design (`fetch::cash_close_verdict`), and one
 *   auction print, which GDFL files apart (`.auc`) or, in a month imported
 *   before that rule, holds as an ordinary one-second bar; an ineligible
 *   share keeps 09:15–15:30;
 * - **derivatives**: 09:15–15:40 — 385 minutes, the last opening at 15:39.
 *
 * So 375 is not the denominator of a CAS-eligible share or of a derivative
 * after that day. What a series OWES on a given day is `/calendar.json`'s
 * answer (D-3336), not this constant; the constant stays because the
 * completeness column's reader (WP-54's) still divides by it, and the label
 * under the Timeframe control says so.
 */
export const BARS_PER_SESSION = 375;

/**
 * Bars one session holds **at the rung the row is stored at**.
 *
 * # The defect this removes
 *
 * `BARS_PER_SESSION` is 375 because that is how many ONE-MINUTE bars a
 * session holds. It was applied to every row regardless of rung, and the
 * server compounded it by stamping `"timeframe":"1m"` on every census row —
 * so a complete 1-day month, 21 bars for 21 trading days, was divided by 375
 * and reported as `0.06 sessions observed`, i.e. ~99.7% missing. The store
 * was right; both readers were wrong in the same direction.
 *
 * A DAILY session is ONE bar by definition. That is not an approximation:
 * the rung IS the session for `1day`.
 *
 * A rung this function has no number for returns `null`, and every caller
 * renders a dash rather than a figure — an unknown denominator produces no
 * percentage at all, never a plausible one. The same rule the % change
 * columns already follow.
 *
 * @param {string | undefined} tf a rung name as the census spells it
 * @returns {number | null} `null` means NO RECORDED SESSION SIZE, never zero
 */
export const barsPerSession = (tf) => {
  if (tf === '1min' || tf === '1m') return BARS_PER_SESSION;
  if (tf === '1day') return 1;
  return null;
};

/**
 * THE BAR-LENGTH LADDER — a rung's own length in SECONDS.
 *
 * # Why a number and not a list position
 *
 * `store::path::Timeframe` is `{ secs: u32, name: &'static str }`, and the
 * seconds are the field that MEANS something: `1min` is 60 and `1day` is
 * 86,400 because those are the numbers in `crates/store/src/path.rs`, not
 * because of where they sit in an array here. Every value below is copied
 * from a `Timeframe` constant in that file and nothing is interpolated
 * between them — a rung this table has no second count for is not given a
 * guessed one.
 *
 * `1m` IS AN ALIAS AND IS MARKED AS SUCH. It is what the server stamped on
 * every census row before it learned to write the real rung, and it may still
 * be in a cached response, so it gets the length this table already knows
 * rather than falling off the end of the ladder.
 *
 * `1hr` WAS A SECOND ALIAS AND IS GONE. This comment used to read: *"`1hr` is
 * `pull::vendor::Granularity`'s spelling of the rung the store files under
 * `60min`"* — a plain statement that one rung had two names, absorbed here by
 * hand because the two crates disagreed. D-0132 made them one word: the
 * store's, `60min`, which is the one that becomes a path. Nothing emits `1hr`
 * any more, and the server refuses it by name as an unknown rung, so keeping
 * a row for it would map a spelling nothing sends onto a length — a workaround
 * outliving the defect it worked around.
 *
 * # Why this exists at all
 *
 * Sorted as TEXT, the rungs read `15min`, `1day`, `1min`, `30min`, `3min`,
 * `5min`, `60min` — ordered by first character. That is the same defect as a
 * month column ordered by `Apr`, and it is why `$lib/dates.js` opens with the
 * rule it does. A bar-length axis has ONE order and it is the length.
 *
 * @type {ReadonlyMap<string, number>}
 */
export const TF_SECS = new Map([
  ['1s', 1],
  ['1min', 60],
  ['1m', 60],
  // 2min AND 10min WERE MISSING, AND THE STORE HAS BOTH.
  //
  // `store::path::Timeframe::KNOWN` lists SECOND_1, MINUTE_1, MINUTE_2,
  // MINUTE_3, MINUTE_5, MINUTE_10, MINUTE_15, MINUTE_30, MINUTE_60 and
  // DAY_1 — ten rungs — and this table carried seven of them. The two
  // absentees sort to `Infinity` and land at the END of the ladder, after
  // 1day, each labelled "length not recorded". Measured on the live page:
  // the rung menu read 1min, 3min, 5min, 15min, 30min, 60min, 1day, 10min,
  // 2min. Two of the nine rungs the store actually holds were out of order
  // and disclaiming a length the store publishes.
  //
  // The seconds are `Timeframe::MINUTE_2.secs` = 120 and
  // `MINUTE_10.secs` = 600, read from `crates/store/src/path.rs`, not
  // arithmetic done here.
  ['2min', 120],
  ['3min', 180],
  ['5min', 300],
  ['10min', 600],
  ['15min', 900],
  ['30min', 1800],
  ['60min', 3600],
  ['1day', 86400]
]);

/**
 * A rung's position on that ladder.
 *
 * A rung the table has no length for sorts LAST — `Infinity`, which is past
 * every real length — and is never folded into the middle, because a string
 * nobody measured has no measured relation to the rungs either side of it.
 * The control draws it, says `length not recorded`, and lets it be chosen:
 * the store holds those rows and hiding the only way to reach them would be
 * worse than admitting the ladder does not reach that far.
 *
 * @param {string} tf
 */
export const tfSecs = (tf) => TF_SECS.get(tf) ?? Number.POSITIVE_INFINITY;

/**
 * THREE-WAY, and it is not a formality.
 *
 * `a < b ? -1 : 1` answers "greater" for two EQUAL inputs, in both
 * directions at once, and a comparator that contradicts itself leaves the
 * order of those elements unspecified — the same store could render its
 * rungs in a different order on the next pass. Equal lengths fall through to
 * `txt`, which is itself three-way, so two spellings of one length are
 * ordered and stay ordered.
 *
 * The example this used to give was `1hr` beside `60min`, and D-0132 removed
 * it: they were one rung with two names and there is now one. The rule is
 * kept because `1m` beside `1min` is still a live pair, and because a
 * comparator that only happens to be consistent is not one.
 *
 * @param {string} a
 * @param {string} b
 */
export const tfCmp = (a, b) => {
  const sa = tfSecs(a);
  const sb = tfSecs(b);
  return sa < sb ? -1 : sa > sb ? 1 : txt(a, b);
};

/* `tfNote` — the third of these — is NOT here, and its absence is the rule
   the FORMATTING block below states: it calls `fmt`, so it is declared after
   `fmt` and not before it. The two above call nothing. */

/* ======================================================================
   FORMATTING
   ----------------------------------------------------------------------
   DECLARED HERE, ABOVE EVERY READER. These sat at the bottom of the script
   and the refusal notice — a `$derived` that formats live counts — closed
   over `fmt` from above it. It ran, because a derived is lazy and the const
   is initialised by the time anything reads it, but `svelte-check` called it
   what it is: a temporal-dead-zone reference that works by scheduling
   accident. One reordering of the evaluation and it throws.
   ====================================================================== */
/* PINNED TO en-IN, LIKE EVERY OTHER PAGE. An unqualified `Intl.NumberFormat()`
   inherits whatever locale the host happens to run, so the same store renders
   8,78,28,617 on one machine and 878,286,617 on another. That makes a
   screenshot unreproducible and a bug report unfalsifiable — and this one
   formatter feeds `fmt`, which is called 36 times across the headline
   counters, the month cards, the receipt prose and every table cell.
   `audit/+page.svelte:145` pins the same way. */
// AND IT NOW DASHES AN UNKNOWN. `nf.format(NaN)` is the string "NaN" and
// `format(Infinity)` is "∞" — a non-answer rendered as though it were one, at
// all 126 of this page's call sites, while `/audit` guarded the same values.
// `exact` does not round, because a record count is already whole and
// rounding one would hide a fractional value that should never have arrived.
export const fmt = exact;

/**
 * What one bar-length rung says about itself under the Timeframe control —
 * its length, and the session size the completeness column divides by.
 *
 * DECLARED HERE AND NOT BESIDE `TF_SECS`, because it calls `fmt` and `fmt`
 * is declared in the block above this line. That is the whole rule the
 * FORMATTING block states, applied rather than quoted.
 *
 * Never a figure this page does not hold: a rung with no recorded
 * bars-per-session says so rather than printing a denominator nobody chose —
 * the same rule `sessions` and the two percentage columns already follow.
 *
 * @param {string} tf
 */
export const tfNote = (tf) => {
  const per = barsPerSession(tf);
  const secs = TF_SECS.get(tf);
  const len = secs === undefined ? 'length not recorded' : `${fmt(secs)}s bars`;
  return per === null ? `${len} · no recorded session size` : `${len} · ${fmt(per)}/session`;
};

/**
 * Three-way codepoint comparison, and STRINGS ONLY.
 *
 * Every one of the fourteen call sites passes a string — a month key, an
 * instrument, a rung name, a reason code — and the block above says why:
 * these keys are `[A-Z0-9-_&]` by construction, so `<` on the codepoints is
 * the whole ordering and `localeCompare` would only make it a function of
 * the operator's locale. Declaring the parameters as strings is what keeps
 * that true: a number reaching here would compare numerically on one
 * argument and lexically on the other the moment one side arrived as text,
 * and the caller — `sorted`, at 20,516 rows — is the last place that would
 * show it.
 *
 * @param {string} a
 * @param {string} b
 */
export function txt(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

/* ======================================================================
   WHAT A PRICE IS, WHAT A CLOSE IS, AND WHERE A BAR CAME FROM (WP-55)
   ----------------------------------------------------------------------
   Findings DB-017, DB-033, DB-035, DB-078, DB-082, DB-G38 to DB-G43,
   DB-G56, DB-G58, DB-G75 and DB-A40. Decisions D-3320 (the closing auction),
   D-3323 (each feed's price and volume basis), D-3324 (official close
   against last print) and D-3334 (every month says where its bars came
   from).

   NOTHING BELOW IS A FACT OF ITS OWN. Every basis, close and auction word is
   read off `/feeds.json` (the descriptor in `pull::vendor`, which cites the
   charter row it rests on), and every source off `/provenance.json` (the
   store's `.prv` sidecar, or the journals it is rebuilt from) or
   `/sources/day.json`. A field the server did not send is SAID to be
   missing; it is never filled with a plausible default. The only literals
   are the measured counts D-3323 and D-3324 record, quoted with their
   decision so a reader can check them, and the end of the closing-auction
   session D-3320 sets (`CAS_READ_TO`), which bounds a read and decides
   nothing.

   EVERY FUNCTION IS PURE AND BOUNDED: a walk of `/feeds.json`'s basis rows
   (at most three segments times ten rungs), of one page of bars, or of one
   answer's records (a month holds at most a few dozen). None reads the
   store or the census, so nothing here grows with the history.
   ====================================================================== */

/** The three store segments a census key can name, in display order. */
const SEGMENT_ORDER = new Map([
  ['INDEX', 0],
  ['CASH', 1],
  ['FNO', 2]
]);

/**
 * A census key's store segment, its second token: `INDEX`, `CASH` or `FNO`.
 * `null` for anything else — never a guessed segment.
 *
 * @param {unknown} key `NSE-CASH-RELIANCE`, `NSE-FNO-NIFTY-2019-03-28-1150000-CE`
 * @returns {string | null}
 */
export function segmentOfKey(key) {
  if (typeof key !== 'string') return null;
  const a = key.indexOf('-');
  const b = a < 0 ? -1 : key.indexOf('-', a + 1);
  if (b < 0) return null;
  const seg = key.slice(a + 1, b);
  return SEGMENT_ORDER.has(seg) ? seg : null;
}

/**
 * The store segments a /db selection shows, from the selection model's
 * `kinds` (`[segment, count]` over the picked underlying's rows) narrowed by
 * its segment control (`spot`, `options`, `futures`, each maybe `-cut` or
 * `-live`; `''` for every one).
 *
 * At most three entries in, at most three out.
 *
 * @param {any} kinds `[[segment, count], …]`
 * @param {string} kind the segment control's value
 * @returns {string[]}
 */
export function selectionSegments(kinds, kind) {
  const want = String(kind ?? '').replace(/-(cut|live)$/, '');
  /** @type {string[]} */
  const out = [];
  for (const entry of Array.isArray(kinds) ? kinds : []) {
    const seg = Array.isArray(entry) ? entry[0] : null;
    if (typeof seg !== 'string' || !SEGMENT_ORDER.has(seg) || out.includes(seg)) continue;
    if (want === 'spot' && seg === 'FNO') continue;
    if ((want === 'options' || want === 'futures') && seg !== 'FNO') continue;
    out.push(seg);
  }
  return out.sort((a, b) => (SEGMENT_ORDER.get(a) ?? 9) - (SEGMENT_ORDER.get(b) ?? 9));
}

/**
 * A feed's name as `/feeds.json` gives it.
 *
 * @param {any} feed a `/feeds.json` row
 */
function feedName(feed) {
  return typeof feed?.display === 'string' && feed.display !== ''
    ? feed.display
    : typeof feed?.wire === 'string' && feed.wire !== ''
      ? feed.wire
      : 'this feed';
}

/**
 * A feed's name in the possessive: "Zerodha's", "Global Datafeeds'".
 *
 * @param {string} name
 */
export function owns(name) {
  return /s$/i.test(name) ? `${name}'` : `${name}'s`;
}

/**
 * Every basis row `/feeds.json` serves for one segment of one feed (D-3323
 * item 1): one per rung the feed holds there.
 *
 * @param {any} feed
 * @param {string | null} segment
 * @returns {any[]}
 */
export function basisRowsOf(feed, segment) {
  const rows = Array.isArray(feed?.basis) ? feed.basis : [];
  return rows.filter((/** @type {any} */ r) => r && r.segment === segment);
}

/**
 * The basis row for one feed, segment and rung — or, with no rung, the
 * segment's first row: the PRICE and VOLUME basis of a segment is one fact at
 * every rung (`pull::vendor::SegmentBasis` is per segment; the rungs repeat
 * it), while the CLOSE differs by rung and is read by `closeBadge` from every
 * row. `null` where the feed serves none.
 *
 * @param {any} feed
 * @param {string | null} segment
 * @param {string} [rung]
 */
export function basisRow(feed, segment, rung = '') {
  const rows = basisRowsOf(feed, segment);
  if (!rung) return rows[0] ?? null;
  return rows.find((r) => r.rung === rung) ?? null;
}

/** D-3323's measurement, as the badge's title states it. */
export const ADJUSTED_MEASURED =
  "Measured over the operator's store on 6–7 Oct 2026 (D-3323, charter §4z): 101 of 208 F&O shares read at a factor off GDFL's raw prints over at least one run of sessions, and the other 107 over none. Some runs are not proportional or drift day to day (ASHOKLEY reads GDFL ÷ 2 − ₹2.48; CANBK, BANKBARODA and BPCL drift), so a percentage move on them is distorted; some sit on a grid coarser than the exchange's tick (BAJFINANCE on 50 paisa), so a tick distance on them is coarser. Which corporate actions the vendor applied is UNVERIFIED.";

/** D-3323 item 3. */
export const NEVER_SIDE_BY_SIDE =
  "Never set beside GDFL's raw price as a disagreement: All sources shows the factor between the two (D-3323 item 3, D-3339).";

/** D-3323 items 1 and 4: a share's Zerodha volume is on its own basis. */
export const VOLUME_OWN =
  "On 45 of the 101 adjusted shares the volume's factor differs from the price's by more than 2%, so price × volume over an adjusted run is not a traded value, and nothing on this page derives turnover or VWAP from these rows (D-3323 items 1 and 4).";

/**
 * WHAT A PRICE IS (D-3323 item 2; DB-017, DB-G43, DB-G58, DB-G75).
 *
 * "back-adjusted by Zerodha as of the pull date <day>" or "raw, as traded",
 * per feed, segment and rung, off `/feeds.json`'s `basis`. A feed nobody
 * measured says UNVERIFIED in the descriptor's own words; a segment the
 * feed's basis does not list is "basis not served" — never a borrowed one.
 *
 * @param {any} feed the `/feeds.json` row
 * @param {string | null} segment `INDEX`, `CASH` or `FNO`
 * @param {string} [rung] `''` for every rung (the price basis is the same at each)
 * @param {string | null} [pullDay] the day the month was pulled, from `/provenance.json`
 * @returns {{ tone: string, short: string, text: string, title: string }}
 */
export function priceBadge(feed, segment, rung = '', pullDay = null) {
  const name = feedName(feed);
  if (!feed) {
    const text = 'no feed is selected, so no price basis applies';
    return { tone: 'unknown', short: 'no feed', text, title: text };
  }
  if (typeof feed.basis_unverified === 'string' && feed.basis_unverified !== '') {
    return {
      tone: 'unverified',
      short: 'basis UNVERIFIED',
      text: feed.basis_unverified,
      title: `${name}: ${feed.basis_unverified}`
    };
  }
  const row = basisRow(feed, segment, rung);
  if (!row) {
    const text = `/feeds.json names no price basis for ${segment ?? 'this segment'}${rung ? ` at ${rung}` : ''} on ${name}, so none is claimed`;
    return { tone: 'unknown', short: 'basis not served', text, title: text };
  }
  const source = typeof row.source === 'string' && row.source !== '' ? ` Source: ${row.source}` : '';
  if (row.price === 'vendor_adjusted') {
    const text = `back-adjusted by ${name} as of the pull date${pullDay ? ` ${pullDay}` : ''}`;
    const where = pullDay ? '' : " Each month's pull date is under Source, below the grid.";
    return {
      tone: 'adjusted',
      short: 'back-adjusted',
      text,
      title: `${text}.${where} ${ADJUSTED_MEASURED} ${NEVER_SIDE_BY_SIDE}${source}`
    };
  }
  if (row.price === 'raw') {
    return {
      tone: 'raw',
      short: 'raw, as traded',
      text: 'raw, as traded',
      title: `raw, as traded: ${owns(name)} prices are the exchange's prints, unadjusted for any corporate action, so a split or bonus reads as a step in price (D-0018).${source}`
    };
  }
  if (row.price === 'as_published') {
    const text = typeof row.price_label === 'string' ? row.price_label : 'as published, an index level';
    return { tone: 'level', short: 'index level', text, title: `${text}.${source}` };
  }
  const text = `/feeds.json names a price basis this page has no words for: ${JSON.stringify(row.price)}`;
  return { tone: 'unknown', short: 'basis unknown', text, title: text };
}

/**
 * WHAT A VOLUME IS, flagged apart from the price (DB-033, DB-G39, D-3323).
 * Only a volume on the vendor's own basis carries a badge of its own: a raw
 * volume is the traded quantity and an index's has `v_why` on every row
 * (D-3322). `null` otherwise.
 *
 * @param {any} feed
 * @param {string | null} segment
 * @param {string} [rung]
 * @returns {{ tone: string, short: string, text: string, title: string } | null}
 */
export function volumeBadge(feed, segment, rung = '') {
  const row = basisRow(feed, segment, rung);
  if (!row || row.volume !== 'vendor_own') return null;
  const text = typeof row.volume_label === 'string' ? row.volume_label : "the vendor's own basis";
  return { tone: 'adjusted', short: "volume: vendor's basis", text, title: `${text}. ${VOLUME_OWN}` };
}

/** D-3324's measurement and rule, as the close badge's title states them. */
export const OFFICIAL_WHY =
  "A day bar's close is the exchange's official close, not the last print, and the day bar is not a fold of the same feed's minutes: Zerodha NIFTY's day close differs from its last minute close on 2,081 of 2,126 common days of 2018-01..2026-09, median 4.69 bps (D-3324). How the exchange forms the official close is not sourced. All sources sets the two side by side with the difference counted, never as a disagreement (D-3339).";

/** D-3324: an intraday close. */
export const LAST_WHY =
  "An intraday bar's close is the last print inside the bar (D-3324); a GDFL one-second bar's is the last kept row of its second (D-2802).";

/**
 * WHAT A CLOSE IS (D-3324; DB-035, DB-G41).
 *
 * `official close` for a day bar the descriptor marks `official`, `last
 * print` for every intraday rung. With no rung chosen, a segment holding
 * both says both, rung by rung, rather than one of them. `null` where the
 * feed's basis is unverified (the price badge already says so) or names no
 * close for the segment.
 *
 * @param {any} feed
 * @param {string | null} segment
 * @param {string} [rung]
 * @returns {{ tone: string, short: string, text: string, title: string } | null}
 */
export function closeBadge(feed, segment, rung = '') {
  if (!feed || (typeof feed.basis_unverified === 'string' && feed.basis_unverified !== '')) return null;
  const rows = basisRowsOf(feed, segment).filter((r) => !rung || r.rung === rung);
  const official = rows.filter((r) => r.close === 'official');
  const last = rows.filter((r) => r.close === 'last_print');
  const source = typeof feed.close_source === 'string' && feed.close_source !== '' ? ` Source: ${feed.close_source}` : '';
  const notFold = official.some((r) => r.day_is_fold === false) ? ', not a fold of its minutes' : '';
  if (official.length > 0 && last.length > 0) {
    const days = official.map((r) => r.rung).join(', ');
    return {
      tone: 'mixed',
      short: `${days}: official close · intraday: last print`,
      text: `${days} close: the official close${notFold}; every intraday rung closes on its last print`,
      title: `${OFFICIAL_WHY} ${LAST_WHY}${source}`
    };
  }
  if (official.length > 0) {
    return {
      tone: 'official',
      short: 'official close',
      text: `${official[0].rung} close: the official close${notFold}, not the last print`,
      title: `${OFFICIAL_WHY}${source}`
    };
  }
  if (last.length > 0) {
    return {
      tone: 'last',
      short: 'close = last print',
      text: `${rung || 'every rung'}: the close is the last print inside the bar`,
      title: `${LAST_WHY}${source}`
    };
  }
  return null;
}

/**
 * THE CLOSING AUCTION, ONCE PER CASH SELECTION (D-3320; DB-G42, DB-082).
 *
 * From the feed's `closing_auction` note: when it starts, an eligible share's
 * continuous close, and what this feed's store holds for the auction
 * (Zerodha's minutes end at 15:14 by design; GDFL keeps the print apart, or
 * in a month imported before that rule holds it as an ordinary bar). `null`
 * for any other segment, a feed with no note, or a selection that ends
 * before the auction began.
 *
 * @param {any} feed
 * @param {string | null} segment
 * @param {string} [through] the selection's last day (`YYYY-MM-DD`) or, with
 *   no day bound, its last month (`YYYY-MM`); `''` when open
 * @returns {{ tone: string, short: string, text: string, title: string } | null}
 */
export function auctionNote(feed, segment, through = '') {
  const a = feed?.closing_auction;
  if (segment !== 'CASH' || !a || typeof a.from !== 'string') return null;
  // A day against the auction's first day, a month against its first month:
  // ISO text compares in time order at equal length.
  if (through && through < a.from.slice(0, through.length)) return null;
  const from = dayLabel(a.from);
  const text = `from ${from} a closing-auction-eligible share trades continuously until ${a.eligible_continuous_close}, then in the closing auction to 15:30; ${a.label}`;
  return {
    tone: 'cas',
    short: `closing auction from ${from}`,
    text,
    title: `${text}. Eligibility is per share and per day (charter §9). An index keeps 09:15–15:30 and derivatives run to 15:40 (charter §7). Source: ${a.source}`
  };
}

/**
 * WHERE AN OPTION'S SPOT COMES FROM (DB-G58; D-3323 item 3, D-2807).
 *
 * The option cells read the underlying through `/spot.json` with this page's
 * own feed, and the server answers on that feed only, so a GDFL strike is
 * never set against a Zerodha spot. `null` for a segment with no strike.
 *
 * @param {any} feed
 * @param {string | null} segment
 */
export function spotNote(feed, segment) {
  if (segment !== 'FNO' || !feed) return null;
  const name = feedName(feed);
  const text = `an option's spot, moneyness, intrinsic, extrinsic, IV and greeks are read from ${owns(name)} own underlying at the bar's instant and rung, never from another feed's: a GDFL strike is never priced against a Zerodha spot`;
  return {
    tone: 'spot',
    short: `spot: ${name} only`,
    text,
    title: `${text} (D-3323 item 3, D-2807; /spot.json answers on the feed it is asked for and no other).`
  };
}

/** The server's phase words (`crates/api/src/bars_change.rs`, D-3320). */
const PHASE_WORDS = new Map([
  ['closed', 'closed'],
  ['before_open', 'before the open'],
  ['pre_open', 'pre-open'],
  ['continuous', 'continuous trading'],
  ['halted', 'halted'],
  ['reopening_auction', 're-opening auction'],
  ['break', 'break'],
  ['cas', 'closing-auction session'],
  ['post_close', 'post-close'],
  ['unverified', 'phase UNVERIFIED']
]);

/**
 * A session phase as words. `undefined` stays `undefined` (the row does not
 * carry a phase), `null` stays `null` (the server sends none: a day rung, or
 * a segment with no venue), and a word this page has no entry for is printed
 * as the server spelled it rather than dropped.
 *
 * @param {unknown} phase
 * @returns {string | null | undefined}
 */
export function phaseWord(phase) {
  if (phase === undefined || phase === null) return phase;
  return PHASE_WORDS.get(String(phase)) ?? `phase ${String(phase)}`;
}

/**
 * IS THIS BAR THE CLOSING AUCTION'S PRINT (D-3320; DB-G42)?
 *
 * Decided per bar from three answers the server gives, never from a clock
 * literal: the bar's own phase (`/bars/window.json`'s `phase`, which knows
 * the share's eligibility that day), the day's closing-auction tail (that
 * day's bars of the session, newest first: its first bar in the `cas` phase
 * is the session's last print), and the day's auction record (`/sources/day.json`'s
 * `auction`: `kept_apart` with the prints when the month files them apart,
 * `no_sidecar` when it was imported before that rule).
 *
 * - not `cas` → `null`;
 * - `kept_apart` → an ordinary bar in the session, NOT the auction's print,
 *   which is filed apart and listed;
 * - `no_sidecar` and the session's last print → **closing auction print**, held
 *   as an ordinary one-second bar until the month is re-imported (D-3320
 *   item 4); an inference, charter §9, and the text says so;
 * - `no_sidecar` and not the last → an ordinary bar whose phase is `cas`;
 * - anything unread → the session, and that the rest was not read.
 *
 * @param {{ feed: any, segment: string | null, t: number, phase: unknown,
 *   tail?: any, dayFacts?: any }} on
 * @returns {{ tone: string, short: string, text: string } | null}
 */
export function auctionVerdict({ feed, segment, t, phase, tail = null, dayFacts = null }) {
  if (phase !== 'cas') return null;
  const base = { tone: 'cas', short: 'closing-auction session' };
  const a = feed?.closing_auction;
  if (segment !== 'CASH' || !a) {
    return { ...base, text: 'a bar in the closing-auction session (D-3320)' };
  }
  if (a.handling === 'minutes_end_before_auction') {
    return {
      ...base,
      text: `a bar in the closing-auction session, although ${owns(feedName(feed))} minute candles of an eligible share end at ${a.eligible_continuous_close} by design (D-3320)`
    };
  }
  // THE NEWEST BAR IN THE SESSION, not the newest bar read: the tail is
  // bounded to the session, and a bar of another phase in it (none is
  // expected) is passed over rather than taken for the auction's print.
  const bars = Array.isArray(tail?.bars) ? tail.bars : null;
  const last = bars ? (bars.find((/** @type {any} */ b) => b?.phase === 'cas') ?? null) : null;
  const isLast = last ? last.t === t : null;
  const auction = dayFacts?.auction ?? null;
  if (auction?.state === 'kept_apart') {
    const prints = Array.isArray(auction.prints) ? auction.prints : [];
    const list = prints.map((/** @type {any} */ p) => `${p.at} at ₹${rupee(p.price)} × ${exact(p.quantity)}`).join('; ');
    return {
      ...base,
      text: `an ordinary bar in the closing-auction session, not the auction's print: this month files the auction's print${prints.length === 1 ? '' : 's'} apart (${list || 'none on this day'}), never in a one-second bar (D-3320 item 3)`
    };
  }
  if (auction?.state === 'unreadable') {
    return { tone: 'warn', short: 'auction record unreadable', text: `the day's auction record would not read: ${auction.why}` };
  }
  // THE LAST PRINT IS CALLED THE AUCTION'S ONLY WHERE IT EQUALS THE OFFICIAL
  // CLOSE (SC-CLOSE-10). On 9 of 133 share-sessions the vendor dropped the
  // auction's print and the session's last print was a 15:15:0x trade, which
  // this label called the auction's; and the print's book is not on the bar,
  // so no zero-book claim is made (it was false on 40 of 116).
  if (auction?.state === 'no_sidecar' && isLast === true && last) {
    const cmp = dayFacts?.close?.official_vs_gdfl_last_print;
    const bps = cmp && typeof cmp.bps === 'number' ? cmp.bps : null;
    const at = secondLabel(t * 1000);
    if (bps !== 0) {
      return {
        ...base,
        text:
          bps === null
            ? `the closing-auction session's last print (${at}); the official close was not compared with it, so it is not called the auction's print (D-3320 item 3)`
            : `the closing-auction session's last print (${at}) differs from the official day close by ${bps} bps (/sources/day.json), so it is not called the auction's print: the vendor's file may hold no auction print at all, and which row, if any, is the auction's is NOT PROVED`
      };
    }
    const twins = bars ? bars.filter((/** @type {any} */ b) => b?.phase === 'cas' && b.t !== t && b.c === last.c).length : 0;
    return {
      tone: 'auction',
      short: 'closing auction print',
      text: `the closing-auction session's last print, and it equals the official day close (/sources/day.json: 0 bps). This month was imported before D-3320's rule and holds it as an ordinary one-second bar until it is re-imported (D-3320 item 4); it is not a continuous trade. Calling it the auction's is an inference (charter §9)${twins > 0 ? `; ${exact(twins)} other print(s) of the session carry the same price, and which of them is the auction's is NOT PROVED` : ''}.`
    };
  }
  if (auction?.state === 'no_sidecar' && isLast === false && last) {
    return {
      ...base,
      text: `a print in the closing-auction session that is not the session's last (that one is at ${secondLabel(last.t * 1000)}), so not the auction's own print: an ordinary bar whose phase is cas (D-3320 item 3)`
    };
  }
  if (bars && !last) {
    return {
      ...base,
      text: "a bar in the closing-auction session; the session's bars as read hold none in the cas phase, so which of them is the auction's own print is not decided here"
    };
  }
  return {
    ...base,
    text: "a bar in the closing-auction session; whether it is the auction's own print is decided by the day's auction record and last print, which were not read"
  };
}

/* ── where a bar came from: /provenance.json (D-3334) ─────────────────── */

/**
 * At most this many month files on one page have their source read. A page
 * of one series straddles one or two; a page of an option chain holds a file
 * per contract, and each read is a path probe and the month's journal rows
 * (a few milliseconds on the operator's store), so sixteen are read and the
 * rest are said not to be.
 */
export const PROV_FILES_MAX = 16;
/** At most this many answers are kept in memory, oldest dropped first. */
export const PROV_CACHE_MAX = 64;
/** The closing-auction tail read for one day: an eligible share holds a few bars there. */
export const CAS_TAIL_LIMIT = 50;
/**
 * At most this many CASH days on one page have their closing-auction session
 * read (two requests each); the rest are counted and said, never dropped.
 */
export const CAS_DAYS_MAX = 8;
/**
 * The last second the closing-auction session can hold: D-3320 item 2 sets
 * `cas` to `[15:15:00, 15:30:00)` for an eligible share. It BOUNDS THE READ
 * only — so a post-close row (stored or not) can never push the session's
 * bars out of the tail — and decides nothing: whether a bar is in the
 * session is the bar's own `phase`, as the server gives it.
 */
export const CAS_READ_TO = '15:29:59';

/**
 * `/provenance.json` for one month file, or with `t` for one bar of it.
 *
 * @param {{ feed: string, instrument: string, timeframe: string, month: string, t?: number | null }} q
 */
export function provenanceUrl({ feed, instrument, timeframe, month, t = null }) {
  const q = new URLSearchParams({ feed, instrument, timeframe, month });
  if (t !== null && t !== undefined) q.set('t', String(t));
  return `/provenance.json?${q}`;
}

/**
 * `/bars/window.json` for the one bar stamped `t` (or the first after it): a
 * seek, through the month's `.tix` where it has one. It carries the bar's
 * stored values and its session phase.
 *
 * @param {{ feed: string, instrument: string, timeframe: string, month: string, t: number }} q
 */
export function barAtUrl({ feed, instrument, timeframe, month, t }) {
  const q = new URLSearchParams({
    feed,
    key: instrument,
    timeframe,
    from: month,
    to: month,
    at: String(t),
    limit: '1',
    sort: 'ts',
    dir: 'asc'
  });
  return `/bars/window.json?${q}`;
}

/**
 * `/bars/window.json` for one day's bars from the eligible continuous close
 * (`/feeds.json` `closing_auction.eligible_continuous_close`) to the end of
 * the session D-3320 sets (`CAS_READ_TO`), newest first: its first bar in
 * the `cas` phase is the session's last print, and every bar carries its
 * phase. One day, at most `CAS_TAIL_LIMIT` bars.
 *
 * @param {{ feed: string, instrument: string, timeframe: string, month: string, day: string, from: string }} q
 */
export function casTailUrl({ feed, instrument, timeframe, month, day, from }) {
  const q = new URLSearchParams({
    feed,
    key: instrument,
    timeframe,
    from: month,
    to: month,
    from_day: day,
    to_day: day,
    tod_from: `${from}:00`,
    tod_to: CAS_READ_TO,
    limit: String(CAS_TAIL_LIMIT),
    sort: 'ts',
    dir: 'desc'
  });
  return `/bars/window.json?${q}`;
}

/**
 * Does this run of rows reach the closing auction (D-3320)? A CASH run below
 * the day rung, on or after the day `/feeds.json` says the auction began.
 *
 * @param {any} run one of `sourceRuns`
 * @param {any} feed
 */
export function reachesAuction(run, feed) {
  const a = feed?.closing_auction;
  if (!a || typeof a.from !== 'string' || !run || typeof run.day !== 'string' || run.day === '') return false;
  return run.day >= a.from && segmentOfKey(run.instrument) === 'CASH';
}

/**
 * THE CASH DAYS OF ONE PAGE WHOSE CLOSING-AUCTION SESSION IS READ.
 *
 * Only a feed that files the auction's print apart (`print_kept_apart`)
 * needs the session read to tell its print from the session's other bars;
 * Zerodha's minutes of an eligible share end at the continuous close by
 * design, and each row's own phase says all there is. At most `cap`
 * share-days are read (two requests each); the rest are COUNTED, so the line
 * that cannot be told says so rather than reading as an ordinary trade.
 *
 * O(runs on the page).
 *
 * @param {any[]} runs `sourceRuns` of one page
 * @param {any} feed
 * @param {number} [cap]
 * @returns {{ read: Set<string>, over: number }} keys `<instrument>|<day>`
 */
export function auctionDaysOnPage(runs, feed, cap = CAS_DAYS_MAX) {
  /** @type {Set<string>} */
  const read = new Set();
  /** @type {Set<string>} */
  const skipped = new Set();
  if (feed?.closing_auction?.handling !== 'print_kept_apart') return { read, over: 0 };
  for (const run of Array.isArray(runs) ? runs : []) {
    if (!reachesAuction(run, feed)) continue;
    const key = `${run.instrument}|${run.day}`;
    if (read.has(key) || skipped.has(key)) continue;
    if (read.size < cap) read.add(key);
    else skipped.add(key);
  }
  return { read, over: skipped.size };
}

/**
 * What a run line says when its month file is past the page's cap
 * (`PROV_FILES_MAX`): not read, and why — never "reading…" for ever.
 *
 * @param {number} files the month files on the page
 * @param {number} [cap]
 * @returns {{ state: string, short: string, text: string, pullDay: null }}
 */
export function sourceNotRead(files, cap = PROV_FILES_MAX) {
  return {
    state: 'capped',
    short: 'source not read',
    text: `this page holds ${exact(files)} month files and the source of at most ${exact(cap)} is read per page; Details reads this row's`,
    pullDay: null
  };
}

/**
 * `/sources/day.json` for one share's one day — its auction record and last
 * print — or `null` for a key that is not one spot series.
 *
 * @param {string} instrument
 * @param {string} day `YYYY-MM-DD`
 */
export function dayFactsUrl(instrument, day) {
  const p = parseKey(instrument);
  if (p.why !== null || p.kind !== 'spot' || !p.exchange || !p.segment || !p.underlying) return null;
  const q = new URLSearchParams({ exchange: p.exchange, segment: p.segment, symbol: p.underlying, from: day, to: day });
  return `/sources/day.json?${q}`;
}

/**
 * The month files one page of bars was read from, in page order, each with
 * how many of the page's rows it holds. At most `cap` are returned; the rest
 * are counted, never dropped silently.
 *
 * @param {any[]} rows one page of the grid's rows
 * @param {number} [cap]
 */
export function filesOnPage(rows, cap = PROV_FILES_MAX) {
  /** @type {Map<string, { key: string, instrument: string, tf: string, month: string, n: number }>} */
  const seen = new Map();
  for (const r of Array.isArray(rows) ? rows : []) {
    const key = `${r.instrument}|${r.tf}|${r.month}`;
    const f = seen.get(key);
    if (f) f.n += 1;
    else seen.set(key, { key, instrument: r.instrument, tf: r.tf, month: r.month, n: 1 });
  }
  const all = [...seen.values()];
  return { files: all.slice(0, cap), over: Math.max(0, all.length - cap) };
}

/**
 * THE SOURCE COLUMN, ONE LINE PER MONTH FILE AND DAY (DB-078, DB-A40).
 *
 * Every row of the page that shares a month file and, below the day rung, an
 * IST day is one run: everything one import day (GDFL) or one pull (Zerodha)
 * wrote is one line, so fifty seconds of one day read as one line and fifty
 * day bars of one month as another — and on a page of several series merged
 * by time (an option chain, CE and PE together) a contract's rows are one
 * run although the other contracts' rows sit between them. Every row of the
 * page is in exactly one run; its positions (0-based, ascending) are `rows`,
 * the first and last of them `from` and `to`. Runs are in the order the page
 * first shows them. O(rows on the page).
 *
 * @param {any[]} rows
 */
export function sourceRuns(rows) {
  /** @type {Map<string, any>} */
  const runs = new Map();
  const list = Array.isArray(rows) ? rows : [];
  for (let i = 0; i < list.length; i++) {
    const r = list[i];
    const file = `${r.instrument}|${r.tf}|${r.month}`;
    const day = r.tf === '1day' ? '' : typeof r.day === 'string' ? r.day : istDay(r.ts);
    const key = `${file}|${day}`;
    const cur = runs.get(key);
    if (cur) {
      cur.rows.push(i);
      cur.to = i;
      cur.last = r;
      cur.n += 1;
      continue;
    }
    runs.set(key, { key, file, instrument: r.instrument, tf: r.tf, month: r.month, day, rows: [i], from: i, to: i, first: r, last: r, n: 1 });
  }
  return [...runs.values()];
}

/**
 * Row positions (0-based) as the 1-based ranges a reader counts: `rows 1–3,
 * 7, 9–12`. At most `max` ranges are spelled; the rest are counted.
 *
 * @param {number[]} positions ascending
 * @param {number} [max]
 */
export function rowsLabel(positions, max = 6) {
  /** @type {string[]} */
  const parts = [];
  let i = 0;
  let spelled = 0;
  const list = Array.isArray(positions) ? positions : [];
  if (list.length === 0) return 'no row';
  while (i < list.length) {
    let j = i;
    while (j + 1 < list.length && list[j + 1] === list[j] + 1) j += 1;
    if (parts.length < max) parts.push(j > i ? `${exact(list[i] + 1)}–${exact(list[j] + 1)}` : exact(list[i] + 1));
    spelled += j - i + 1;
    i = j + 1;
    if (parts.length === max && i < list.length) {
      const rest = list.length - spelled;
      parts.push(`and ${exact(rest)} more`);
      break;
    }
  }
  const one = list.length === 1;
  return `${one ? 'row' : 'rows'} ${parts.join(', ')}`;
}

/**
 * RUNS THAT SAY THE SAME THING ARE ONE LINE. On an option chain every
 * contract's month file answers with the same import day, journal line and
 * archive; drawn one per contract, the lines would repeat that sentence once
 * per strike. `keyOf` names what a line says (its source, basis and close,
 * as the caller composed them); runs with equal keys are folded, in the order
 * the page first shows them, and the rows of a fold are the union of its
 * runs'. Nothing is dropped: each fold keeps every run it holds.
 *
 * @template R
 * @param {R[]} runs
 * @param {(run: R) => string} keyOf
 * @returns {{ key: string, runs: R[], rows: number[] }[]}
 */
export function foldRuns(runs, keyOf) {
  /** @type {Map<string, { key: string, runs: R[], rows: number[] }>} */
  const folds = new Map();
  for (const run of Array.isArray(runs) ? runs : []) {
    const key = keyOf(run);
    const f = folds.get(key);
    const rows = /** @type {any} */ (run).rows ?? [];
    if (f) {
      f.runs.push(run);
      f.rows.push(...rows);
    } else folds.set(key, { key, runs: [run], rows: [...rows] });
  }
  for (const f of folds.values()) f.rows.sort((a, b) => a - b);
  return [...folds.values()];
}

/**
 * A journal value written percent-encoded (`gdfl_import`'s run line, D-3317),
 * decoded for reading; the raw text when it will not decode.
 *
 * @param {unknown} s
 */
function unpercent(s) {
  const text = String(s ?? '');
  try {
    return decodeURIComponent(text);
  } catch {
    return text;
  }
}

/**
 * A GDFL run line's fields (D-3317) as one sentence: the source (zip or tick
 * store), the archive's path, size and digest, and the build commit.
 *
 * @param {any} run `{ id, line, fields }` or `{ id, error }`
 */
function runText(run) {
  if (!run) return null;
  if (run.error) return `run ${run.id}: ${run.error}`;
  const f = run.fields ?? {};
  const digest = f.archive_cd_blake3
    ? `, central directory BLAKE3 ${String(f.archive_cd_blake3).slice(0, 16)}…`
    : f.format_sha256
      ? `, format SHA-256 ${String(f.format_sha256).slice(0, 16)}…`
      : '';
  const bytes = Number(f.archive_bytes);
  return `run ${run.id} (journal line ${run.line}): source ${f.source ?? 'unrecorded'}, archive ${unpercent(f.archive ?? 'unrecorded')}${Number.isFinite(bytes) ? ` (${exact(bytes)} bytes${digest})` : ''}, build ${f.commit ? String(f.commit).slice(0, 12) : 'unrecorded'}`;
}

/**
 * Where this build's readers would find a day, when no run recorded where it
 * WAS read (`vendor_files`, the server's own words for it). Labelled as such.
 *
 * @param {any} vf
 */
function vendorFilesText(vf) {
  if (!vf) return null;
  const member = vf.stem ? `${vf.day_folder}/${vf.stem}.csv` : `the contract's file in ${vf.day_folder}`;
  const zip = vf.zip_day ? `day zip ${vf.zip_day}` : null;
  const store = vf.tickstore_day ? `tick-store day ${vf.tickstore_day}` : null;
  const where = [zip, store].filter(Boolean).join(' or ');
  return `${member}${where ? `, in ${where}` : ''}`;
}

/**
 * One sidecar record (D-3334) as one sentence.
 *
 * @param {any} r a `recorded` entry of `/provenance.json`
 */
export function recordText(r) {
  const kind = String(r.source_kind ?? 'source').replace(/_/g, ' ');
  const size = typeof r.source_bytes === 'number' && r.source_bytes > 0
    ? ` (${exact(r.source_bytes)} bytes${r.source_crc32 ? `, CRC-32 ${r.source_crc32}` : ''})`
    : '';
  const commit = r.commit ? String(r.commit).slice(0, 12) : 'not stamped';
  return `${kind} ${r.source_name ?? '—'}${r.source_name_truncated ? '…' : ''}${size} · day ${r.day} · ${r.journal?.source ?? 'journal'} record ${r.journal_ordinal} · landed ${r.landed_at} · build ${commit}`;
}

/**
 * The IST day of a stamp the server writes as `YYYY-MM-DD HH:MM:SS IST`, or
 * `null` for anything else.
 *
 * @param {unknown} stamp
 */
function stampDay(stamp) {
  return typeof stamp === 'string' && /^\d{4}-\d{2}-\d{2}\b/.test(stamp) ? stamp.slice(0, 10) : null;
}

/** At most this many candidate runs are named in a line; the rest are counted. */
const CANDIDATES_NAMED = 6;

/**
 * The pull run `/provenance.json` names as a pulled month's writer, or why it
 * names none (D-3334 item 3, `pull_reconstruction`), and THE PULL DATE the
 * price basis is "as of" (D-3323 item 1).
 *
 * The pull date is the day the month file was last written (`bars.modified`):
 * a named writer committed the file once, inside its run, so that day is the
 * day its bars were fetched. With no writer named, it is still decided when
 * every write fell on one day — no candidate run began before that day
 * (`candidates[].started`), and the file was not written after it — and only
 * then; otherwise the line names the span and claims no date.
 *
 * @param {any} rec the answer's `reconstruction`
 * @param {any} [bars] the answer's `bars`
 * @returns {{ text: string, pullDay: string | null }}
 */
function pullText(rec, bars = null) {
  const candidates = Array.isArray(rec.candidates) ? rec.candidates : [];
  const named = rec.attributed?.run;
  const lastWrite = stampDay(bars?.modified) ?? stampDay(rec.last_write?.modified);
  const c = named === undefined || named === null ? null : candidates.find((/** @type {any} */ x) => x.ordinal === named);
  if (c) {
    const failed = c.outcome === 'FAILED' && c.failures > 0 ? ` (the run ended FAILED on ${exact(c.failures)} other request(s); this month was written inside it)` : '';
    return {
      text: `pulled by run ${c.ordinal} (${rec.source ?? 'audit/pull.journal'} record ${c.ordinal}), started ${c.started}, ${c.outcome}${failed}, from ${c.source}${c.source_cut ? '…' : ''}${rec.attributed?.basis ? `; named because ${rec.attributed.basis}` : ''}`,
      pullDay: lastWrite ?? stampDay(c.started)
    };
  }
  const starts = candidates.map((/** @type {any} */ x) => stampDay(x.started)).filter((/** @type {string | null} */ d) => d !== null).sort();
  const earliest = starts.length === candidates.length && starts.length > 0 ? starts[0] : null;
  const oneDay = earliest !== null && lastWrite !== null && earliest === lastWrite ? lastWrite : null;
  const list = candidates
    .slice(0, CANDIDATES_NAMED)
    .map((/** @type {any} */ x) => `run ${x.ordinal} (${x.started}, ${x.outcome})`)
    .join('; ');
  const more = candidates.length > CANDIDATES_NAMED ? `; and ${exact(candidates.length - CANDIDATES_NAMED)} more` : '';
  const inside = Array.isArray(rec.last_write?.inside_runs) && rec.last_write.inside_runs.length > 0
    ? ` The file was last written ${rec.last_write.modified ?? ''} inside run ${rec.last_write.inside_runs.join(', ')}.`
    : '';
  const when = oneDay
    ? ` Every write fell on ${oneDay}: no candidate began before it and the file was last written on it, so that is the pull date.`
    : earliest !== null && lastWrite !== null
      ? ` The month was written between ${earliest} and ${lastWrite}, so no one pull date is claimed.`
      : '';
  return {
    text: `${exact(candidates.length)} pull run(s) could have written this month and none is named as its writer: ${rec.why ?? 'the journal does not say'}.${list ? ` Candidates: ${list}${more}.` : ''}${inside}${when}`,
    pullDay: oneDay
  };
}

/**
 * WHERE ONE DAY OF ONE MONTH CAME FROM, off one `/provenance.json` answer
 * (D-3334; DB-078, DB-G56, DB-A40).
 *
 * - a sidecar record of that day → `recorded`, with its source, size,
 *   CRC-32, journal record, landing time and build;
 * - else the GDFL import journal's line for the day → `journal`, with the
 *   run's archive when a run line names it (D-3317) and, when none does,
 *   that it was NOT recorded, beside where this build's readers would find
 *   the day;
 * - else a pulled month's run → `pull`, with its pull date;
 * - `day` `''` asks about the whole month.
 *
 * @param {any} answer the parsed body
 * @param {string} day `YYYY-MM-DD`, or `''`
 * @returns {{ state: string, short: string, text: string, pullDay: string | null }}
 */
export function sourceOfDay(answer, day) {
  if (!answer || typeof answer !== 'object') {
    return { state: 'unread', short: 'source not read', text: 'no answer from /provenance.json yet', pullDay: null };
  }
  if (typeof answer.error === 'string') {
    return { state: 'error', short: 'source refused', text: answer.error, pullDay: null };
  }
  if (answer.held === false) {
    return { state: 'absent', short: 'no month file', text: `the store holds no month file at ${answer.path ?? 'this path'}`, pullDay: null };
  }
  const label = typeof answer.label === 'string' ? answer.label : '';
  const recorded = Array.isArray(answer.recorded) ? answer.recorded : [];
  const mine = day ? recorded.filter((/** @type {any} */ r) => r.day === day) : recorded;
  if (mine.length > 0) {
    const more = mine.length > 1 ? ` (and ${exact(mine.length - 1)} more record(s) of ${day || 'this month'})` : '';
    return { state: 'recorded', short: 'recorded', text: `${recordText(mine[0])}${more}`, pullDay: mine[0].day ?? null };
  }
  const rec = answer.reconstruction;
  if (rec && Array.isArray(rec.days)) {
    const src = rec.source ?? 'imports/gdfl.journal';
    if (!day) {
      return {
        state: 'journal',
        short: label || 'reconstructed',
        text: `${exact(rec.journal_days ?? rec.days.length)} ${rec.kind ?? ''} import day(s) in ${src}, ${exact(rec.journal_days_done ?? 0)} done; ${rec.archive?.recorded ? 'archive recorded by its run' : `archive not recorded${rec.archive?.why ? `: ${rec.archive.why}` : ''}`}`,
        pullDay: null
      };
    }
    const row = rec.days.find((/** @type {any} */ d) => d.day === day);
    if (!row) {
      return {
        state: 'journal',
        short: 'no import line',
        text: `${src} names no ${rec.kind ?? ''} import of ${day}; ${exact(rec.bars_on_no_journal_day ?? 0)} bar(s) of this month sit on a day the journal does not name`,
        pullDay: null
      };
    }
    const run = row.run !== null && row.run !== undefined && Array.isArray(rec.archive?.runs)
      ? rec.archive.runs.find((/** @type {any} */ x) => x.id === row.run)
      : null;
    const archive = run
      ? runText(run)
      : `archive not recorded (its closing line names no run: written before D-3317); this build would read ${vendorFilesText(row.vendor_files) ?? 'the day'}`;
    const attempts = row.attempts > 1 ? `, ${exact(row.attempts)} attempts` : '';
    // THE BAR DEFINITION THAT BUILT THE DAY rides on its rows' label (R3-13
    // goal 3), where the deciding line names one.
    const definition = Number.isSafeInteger(row.definition) ? ` · bar definition ${row.definition}` : '';
    return {
      state: 'journal',
      short: label || 'reconstructed',
      text: `import day ${row.day} · ${src} line ${row.line} (${row.outcome}${attempts}) · ${archive}${definition}`,
      pullDay: null
    };
  }
  if (rec && Array.isArray(rec.candidates)) {
    const p = pullText(rec, answer.bars);
    return { state: 'pull', short: label || 'reconstructed', text: p.text, pullDay: p.pullDay };
  }
  if (rec && typeof rec.error === 'string') {
    return { state: 'error', short: label || 'reconstruction refused', text: rec.error, pullDay: null };
  }
  return { state: 'none', short: label || 'no source', text: label || '/provenance.json named no source for these bars', pullDay: null };
}

/**
 * EVERYTHING THE DRAWER SAYS ABOUT ONE BAR (DB-078, DB-A40), as label and
 * value pairs, each value the server's own: the bar as served, its place in
 * its month file, its price, volume and close basis, its session phase and
 * whether it is the closing auction's print, where it came from (record,
 * import day and archive, or pull run), the month file and its sidecar, and
 * the operator's window.
 *
 * @param {{ feed: any, row: any, answer: any, bar?: any, tail?: any, dayFacts?: any }} on
 * @returns {{ k: string, v: string, tone?: string }[]}
 */
export function drawerFacts({ feed, row, answer, bar = null, tail = null, dayFacts = null, band = null, basis = null }) {
  /** @type {{ k: string, v: string, tone?: string }[]} */
  const out = [];
  if (!row) return out;
  const segment = segmentOfKey(row.instrument);
  const t = Math.floor(row.ts / 1000);
  const day = typeof row.day === 'string' ? row.day : istDay(row.ts);
  // A DAY BAR IS STAMPED AT ITS DAY'S IST MIDNIGHT and has no clock of its
  // own: printing 00:00:00 would name an instant no trade happened at.
  const when = row.tf === '1day' ? `${dayLabel(row.ts)}, a day bar` : `${dayLabel(row.ts)} ${secondLabel(row.ts)} IST`;
  out.push({ k: 'Bar', v: `${row.instrument} · ${row.tf} · ${when} (t=${t}) · month file ${row.month}` });
  if (bar) {
    out.push({
      k: 'As served',
      v: `open ₹${rupee(bar.o)} · high ₹${rupee(bar.h)} · low ₹${rupee(bar.l)} · close ₹${rupee(bar.c)} · volume ${exact(bar.v)}${bar.v_why ? ` (${bar.v_why})` : ''}`
    });
  }
  const src = sourceOfDay(answer, row.tf === '1day' ? '' : day);
  const price = priceBadge(feed, segment, row.tf, src.pullDay);
  out.push({ k: 'Price basis', v: price.text, tone: price.tone });
  // THE RUN THIS ROW'S DAY BELONGS TO, where its prices are adjusted or
  // quantised (R3-13 goal 4): the fact, from the served factor.
  const run = Array.isArray(basis) ? basisRunLabel(basis, day) : null;
  if (run) out.push({ k: 'Adjusted run', v: run.text, tone: run.tone });
  // WHICH BAR DEFINITION BUILT THE DAY (R3-13 goal 3).
  const defined = definitionFact(answer, day, band);
  if (defined) out.push(defined);
  const vol = volumeBadge(feed, segment, row.tf);
  if (vol) out.push({ k: 'Volume basis', v: `${vol.text}. ${VOLUME_OWN}`, tone: vol.tone });
  const close = closeBadge(feed, segment, row.tf);
  if (close) out.push({ k: 'Close', v: close.text, tone: close.tone });
  const spot = spotNote(feed, segment);
  if (spot) out.push({ k: 'Spot', v: spot.text, tone: spot.tone });
  const phase = bar && 'phase' in bar ? bar.phase : row.phase;
  const word = phaseWord(phase);
  // A DAY BAR HAS NO PHASE BY CONSTRUCTION (the server sends `null` for a day
  // rung), so it is said without waiting for the bar to be read.
  if (row.tf === '1day') out.push({ k: 'Session phase', v: 'none: a day bar spans the session' });
  else if (word === undefined) out.push({ k: 'Session phase', v: 'not read: the bar itself was not read' });
  else if (word === null) out.push({ k: 'Session phase', v: 'none: this segment has no venue phases' });
  else out.push({ k: 'Session phase', v: word });
  const verdict = auctionVerdict({ feed, segment, t, phase, tail, dayFacts });
  if (verdict) out.push({ k: 'Closing auction', v: `${verdict.short}: ${verdict.text}`, tone: verdict.tone });
  if (answer && typeof answer === 'object' && answer.held !== false && typeof answer.error !== 'string') {
    if (answer.bar && typeof answer.bar.index === 'number') {
      out.push({ k: 'In its file', v: `bar ${exact(answer.bar.index)} of ${exact(answer.bars?.n_valid)} (0-based) in ${answer.bars?.path}` });
    }
    out.push({ k: 'Source', v: `${src.short}: ${src.text}`, tone: src.state });
    if (answer.bars) {
      out.push({
        k: 'Month file',
        v: `${answer.bars.path} · ${exact(answer.bars.n_valid)} bars · generation ${exact(answer.bars.generation)} (one per landing that appended) · last written ${answer.bars.modified ?? 'not said by the host'}`
      });
    }
    if (answer.sidecar) {
      out.push({ k: 'Sidecar', v: `${answer.sidecar.path}: ${answer.sidecar.state}${answer.sidecar.why ? ` — ${answer.sidecar.why}` : ''}${answer.sidecar.error ? ` — ${answer.sidecar.error}` : ''}` });
    }
    if (answer.window?.label) out.push({ k: 'Window', v: `${answer.window.label} (${answer.window.from} to ${answer.window.to})`, tone: 'warn' });
    for (const note of Array.isArray(answer.notes) ? answer.notes : []) out.push({ k: 'Note', v: String(note) });
  } else {
    out.push({ k: 'Source', v: `${src.short}: ${src.text}`, tone: src.state });
  }
  return out;
}

/* ── a Zerodha share's adjusted or quantised run (R3-13 goal 4) ────────── */

/**
 * `/sources/day.json` over one share's days, the route that serves each day's
 * measured factor against GDFL's raw prints and whether Zerodha's grid is
 * coarser than an exact adjustment leaves (`share_basis`, DB-G38, DB-G40).
 * `null` for anything but a spot share key.
 *
 * @param {string} instrument
 * @param {string} from `YYYY-MM-DD`
 * @param {string} to `YYYY-MM-DD`
 */
export function shareBasisUrl(instrument, from, to) {
  const p = parseKey(instrument);
  if (p.why !== null || p.kind !== 'spot' || p.segment !== 'CASH' || !p.exchange || !p.underlying || !from || !to) return null;
  const q = new URLSearchParams({ exchange: p.exchange, segment: p.segment, symbol: p.underlying, from, to });
  return `/sources/day.json?${q}`;
}

/**
 * THE RUNS OF DAYS WHOSE ZERODHA PRICES ARE ADJUSTED OR QUANTISED, from a
 * `/sources/day.json` answer (R3-13 goal 4; DB-G38, DB-G40): consecutive days
 * of the answer (oldest first) whose served `share_basis` says the basis
 * differs from GDFL's raw prints, or the grid is quantised. A day neither
 * feed measured breaks no run and joins none. Each run names its first and
 * last day, the factors served on it and the grids, and whether it touches
 * the answer's edge (it may go on past the days read). The fact only, from
 * the served factor: whether a percentage statistic on such a run is refused
 * is the operator's decision D-3323, not decided here.
 *
 * @param {any} body
 * @returns {{ from: string, to: string, days: number, adjusted: boolean, quantised: boolean, factors: number[], grid: { zerodha: number | null, gdfl: number | null }, open: { before: boolean, after: boolean } }[]}
 */
export function basisRunsOf(body) {
  const days = Array.isArray(body?.days) ? [...body.days] : [];
  days.sort((a, b) => (String(a?.day) < String(b?.day) ? -1 : 1));
  /** @type {ReturnType<typeof basisRunsOf>} */
  const out = [];
  /** @type {ReturnType<typeof basisRunsOf>[number] | null} */
  let cur = null;
  for (const [i, d] of days.entries()) {
    const b = d?.share_basis;
    if (!b || typeof b !== 'object') continue;
    const adjusted = b.basis_differs === true;
    const quantised = b.quantised === true;
    if (!adjusted && !quantised) {
      cur = null;
      continue;
    }
    if (!cur) {
      cur = { from: d.day, to: d.day, days: 0, adjusted: false, quantised: false, factors: [], grid: { zerodha: null, gdfl: null }, open: { before: i === 0, after: false } };
      out.push(cur);
    }
    cur.to = d.day;
    cur.days += 1;
    cur.adjusted ||= adjusted;
    cur.quantised ||= quantised;
    if (typeof b.factor === 'number' && !cur.factors.includes(b.factor)) cur.factors.push(b.factor);
    if (Number.isSafeInteger(b.grid_paisa?.zerodha_minutes)) cur.grid.zerodha = b.grid_paisa.zerodha_minutes;
    if (Number.isSafeInteger(b.grid_paisa?.gdfl_seconds)) cur.grid.gdfl = b.grid_paisa.gdfl_seconds;
    cur.open.after = i === days.length - 1;
  }
  return out;
}

/**
 * The label a row of `day` carries for the run that holds it, or `null`:
 * "adjusted run 2019-06-03 → 2019-06-28 ×4.0000", with the grid when it is
 * quantised, and the edge said where the run may go on past the days read.
 *
 * @param {ReturnType<typeof basisRunsOf>} runs
 * @param {string} day `YYYY-MM-DD`
 * @returns {{ tone: string, short: string, text: string } | null}
 */
export function basisRunLabel(runs, day) {
  const run = runs.find((r) => day && r.from <= day && day <= r.to);
  if (!run) return null;
  const what = run.adjusted && run.quantised ? 'adjusted and quantised' : run.adjusted ? 'adjusted' : 'quantised';
  const factors = run.factors.length === 0
    ? 'no factor served'
    : run.factors.length === 1
      ? `factor ×${run.factors[0].toFixed(4)}`
      : `factor ×${Math.min(...run.factors).toFixed(4)} to ×${Math.max(...run.factors).toFixed(4)} (it drifts)`;
  const grid = run.quantised && run.grid.zerodha !== null
    ? `; Zerodha's prices sit on a ${exact(run.grid.zerodha)}-paisa grid${run.grid.gdfl !== null ? ` against GDFL's raw ${exact(run.grid.gdfl)}-paisa grid` : ''}, coarser than an exact adjustment leaves`
    : '';
  const edge = run.open.before || run.open.after
    ? ` (the run may go on ${run.open.before && run.open.after ? 'both ways' : run.open.before ? 'before' : 'after'} the days read)`
    : '';
  return {
    tone: 'adjusted',
    short: `${what} run ${run.from} → ${run.to}`,
    text: `Zerodha's prices on this run are ${what} against GDFL's raw prints: ${run.from} to ${run.to}, ${exact(run.days)} day(s) measured, ${factors}${grid}${edge}. The fact as /sources/day.json serves it; whether a percentage statistic on it is refused is D-3323's to decide.`
  };
}

/**
 * THE BAR DEFINITION THAT BUILT A ROW'S DAY, for the drawer (R3-13 goal 3):
 * the import journal's deciding line for the day (`/provenance.json`'s
 * reconstruction), and whether this build builds the same bars, from the day
 * band's record of that day where it was read. `null` where no journal row
 * names a definition.
 *
 * @param {any} answer `/provenance.json`'s month answer
 * @param {string} day `YYYY-MM-DD`
 * @param {{ definition?: { value: number, current: boolean, thisBuild: number | null, means: string } | null } | null} [band] the day band's cell for the day
 * @returns {{ k: string, v: string, tone?: string } | null}
 */
export function definitionFact(answer, day, band = null) {
  const rows = answer?.reconstruction?.days;
  const row = Array.isArray(rows) ? rows.find((/** @type {any} */ d) => d?.day === day) : null;
  const value = Number.isSafeInteger(row?.definition) ? row.definition : (band?.definition?.value ?? null);
  if (value === null) return null;
  const where = Number.isSafeInteger(row?.definition) ? `the import journal's line ${row.line}${row.recounted ? ', closed by the census recount' : ''}` : 'the day band';
  const d = band?.definition && band.definition.value === value ? band.definition : null;
  if (!d) {
    return { k: 'Bar definition', v: `definition ${value}, from ${where}; whether this build builds the same bars is the day band's to say, and it was not read for this day` };
  }
  const means = d.means ? `: ${d.means}` : '';
  return d.current
    ? { k: 'Bar definition', v: `definition ${value}${means} (from ${where}); this build builds the same bars` }
    : {
        k: 'Bar definition',
        v: `definition ${value}${means} (from ${where}); this build builds definition ${d.thisBuild ?? 'another'}, so the day is labelled until it is restated`,
        tone: 'warn'
      };
}

/**
 * Keeps at most `max` entries, dropping the oldest: `Map` keeps insertion
 * order. Returns a NEW map, so a `$state.raw` holder sees the change.
 *
 * @template V
 * @param {Map<string, V>} map
 * @param {string} key
 * @param {V} value
 * @param {number} [max]
 * @returns {Map<string, V>}
 */
export function remember(map, key, value, max = PROV_CACHE_MAX) {
  const next = new Map(map);
  next.delete(key);
  next.set(key, value);
  while (next.size > max) {
    const oldest = next.keys().next().value;
    if (oldest === undefined) break;
    next.delete(oldest);
  }
  return next;
}

/**
 * Per row, the auction verdict a run's rows carry, from the row's own phase
 * when the grid kept it, else from the day's closing-auction tail (which
 * carries every phase from the eligible close). Only the rows with a verdict
 * are returned, in page order.
 *
 * @param {{ feed: any, rows: any[], tail?: any, dayFacts?: any }} on
 * @returns {{ index: number, row: any, verdict: { tone: string, short: string, text: string } }[]}
 */
export function auctionMarks({ feed, rows, tail = null, dayFacts = null }) {
  /** @type {Map<string, unknown>} */
  const phases = new Map();
  for (const b of Array.isArray(tail?.bars) ? tail.bars : []) phases.set(String(b.t), b.phase);
  /** @type {{ index: number, row: any, verdict: { tone: string, short: string, text: string } }[]} */
  const out = [];
  const list = Array.isArray(rows) ? rows : [];
  for (let i = 0; i < list.length; i++) {
    const row = list[i];
    const t = Math.floor(row.ts / 1000);
    const phase = row.phase !== undefined && row.phase !== null ? row.phase : phases.get(String(t));
    const verdict = auctionVerdict({ feed, segment: segmentOfKey(row.instrument), t, phase, tail, dayFacts });
    if (verdict) out.push({ index: i, row, verdict });
  }
  return out;
}

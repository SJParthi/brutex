/**
 * /db — THE DAY BAND: what `/dayflags.json` says about each day, folded for
 * the page (WP-54; D-3335 §3, D-3336, D-3319, D-3320, D-3317, D-3315).
 *
 * A day on /db was a row or a hole, and nothing said which kind of hole: a
 * weekend, a disaster-recovery drill the GDFL import refused, a day whose
 * index file the store refused to file, a suspected circuit-breaker halt, a
 * Zerodha minute gap on a day GDFL traded, a degraded vendor day, a Zerodha
 * daily row priced at zero (DB-016, DB-025, DB-026, DB-028, DB-029, DB-030,
 * DB-031, DB-034, DB-053, DB-G37, DB-G42, DB-G47, DB-G69, DB-G87, DB-G96).
 * `/dayflags.json` (WP-33b) answers one record per (feed, instrument, day)
 * with every source merged and a `says` list of short phrases; this module
 * turns a page of records into the band's cells and the empty grid's reason.
 *
 * NOTHING HERE DECIDES A FACT THE RECORD DOES NOT CARRY. A cell's kind is
 * read off the record's own fields in a fixed order (most severe first), its
 * words are the record's `says`, and a field the server left `null` (a
 * ledger not written yet) is "not measured", never "clean".
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: `node --test` imports it with no
 * compiler (`web/tests/db-dayflags.test.js`).
 */

import { lineageLines } from './lineage-labels.js';

/** The most days one `/dayflags.json` page holds (`dayflags::PAGE_DAYS`). */
export const DAYFLAGS_PAGE_DAYS = 92;

/**
 * The `/dayflags.json` URL for one feed, instrument and day span.
 *
 * @param {string} feed
 * @param {string} instrument a census key, contract included
 * @param {string} from `YYYY-MM-DD`
 * @param {string} to `YYYY-MM-DD`
 * @param {number} [page]
 */
export function dayflagsUrl(feed, instrument, from, to, page = 0) {
  const q = new URLSearchParams({ feed, instrument, from, to, order: 'asc' });
  if (page > 0) q.set('page', String(page));
  return `/dayflags.json?${q}`;
}

/**
 * The kinds a day cell can be, MOST SEVERE FIRST: the first that applies is
 * the cell's colour. `word` is what the band prints; `tone` is the CSS hue
 * family (`bad` a hole or a refusal, `warn` a vendor fault, `info` an
 * exchange event, `quiet` a closed day, `ok` an ordinary session).
 */
export const DAY_KINDS = [
  { kind: 'refused', word: 'refused at import', tone: 'bad' },
  { kind: 'incomplete', word: 'incomplete at import', tone: 'bad' },
  { kind: 'store_gap', word: 'no bar on a session', tone: 'bad' },
  { kind: 'front', word: 'refused front', tone: 'bad' },
  { kind: 'degraded', word: 'degraded vendor day', tone: 'warn' },
  // A MINUTE GAP IS A MINUTE MISSING: Zerodha's `gaps.missing`, the minutes
  // a session owed that the vendor did not serve (D-3321). The GDFL quality
  // ledger's seconds are not minutes, and two of its three counts are not
  // missing time at all, so each has its own kind and word (AH-03,
  // g-ui-dom-vs-json-7; DB-G96's "flag them" kept, its word amended — D-5834).
  { kind: 'gap', word: 'minute gap', tone: 'warn' },
  // `quality.indices.holes_5s`: in-session index holes of 5 s or more.
  { kind: 'hole', word: 'holes of 5 s or more', tone: 'warn' },
  // `quality.stocks.shared_holes`: holes the stocks file shares across most
  // traded shares (the server's phrase names the length and the share).
  { kind: 'shared_hole', word: 'hole shared by most shares', tone: 'warn' },
  // `quality.indices.dual_spread_s`: one second holding two values at least
  // 10 bps apart — no time is missing (DB-G96).
  { kind: 'dual', word: 'two values in one second', tone: 'warn' },
  { kind: 'junk', word: 'junk row', tone: 'warn' },
  { kind: 'incomplete_other', word: 'import incomplete for other files', tone: 'warn' },
  { kind: 'late', word: 'late rows deferred', tone: 'warn' },
  { kind: 'fills', word: 'mostly zero-volume minutes', tone: 'warn' },
  { kind: 'halt', word: 'suspected halt', tone: 'info' },
  { kind: 'special', word: 'special session', tone: 'info' },
  { kind: 'unmeasured', word: 'session of unmeasured length', tone: 'info' },
  { kind: 'cas', word: 'closing auction from 15:15', tone: 'info' },
  { kind: 'not_reached', word: 'not imported yet', tone: 'quiet' },
  { kind: 'no_trade', word: 'no trade (trade-driven)', tone: 'quiet' },
  // `/feeds.json` `archive` is the vendor's archive AS MEASURED (R3-05): its
  // last day is a dated lower bound, so a day past it is not imported, never
  // a vendor hole or "outside the vendor's archive" — the files may be
  // delivered and waiting (SC-CLOSE-2).
  { kind: 'outside_archive', word: 'not imported', tone: 'quiet' },
  // A day before a BINDING first day (`archive.from_binds`, which the server
  // serves `true`: no file before it exists) is outside the vendor's
  // archive, not "not imported": no import can ever reach it (R3-05 repair
  // 3, the second agent's follow-up; it was worded "not imported" too).
  { kind: 'before_archive', word: "before the vendor's archive", tone: 'quiet' },
  { kind: 'closed', word: 'closed', tone: 'quiet' },
  { kind: 'outside', word: "outside the calendar's span", tone: 'quiet' },
  { kind: 'session', word: 'session', tone: 'ok' }
];

/** @type {Map<string, { kind: string, word: string, tone: string }>} */
const KIND = new Map(DAY_KINDS.map((k) => [k.kind, k]));

/** The kinds that are facts about the day, not flags: an ordinary or closed
 *  day, a day outside a span, a trade-driven day with no trade. */
const QUIET = new Set(['session', 'closed', 'outside', 'outside_archive', 'before_archive', 'no_trade']);

/**
 * The exchange calendar's session codes (`pull::calendar`, D-3319): a
 * regular day, a regular day with a closing auction, a special session, a
 * suspected halt, a session of unmeasured length. `.` is closed and `-` is
 * outside the calendar's span; neither owes a bar.
 */
const SESSION_CODES = new Set(['R', 'X', 'P', 'H', 'U']);

/**
 * Is a calendar code a session? A Muhurat or other special session on a
 * holiday is `P`, so it is one. O(1).
 *
 * @param {unknown} code
 */
export function isSessionCode(code) {
  return typeof code === 'string' && SESSION_CODES.has(code);
}

/** The kinds that say time is missing on the day: Zerodha's missing minutes,
 *  the index holes of 5 s or more, and the holes shared by most shares. A
 *  second with two values is not one (D-5834). */
const HOLE_KINDS = new Set(['gap', 'hole', 'shared_hole']);

/**
 * Does a cell say time is missing on its day? The chart draws its hole
 * bands only on such a day (`chart.js` `holeBands`). O(kinds).
 *
 * @param {{ kinds: string[] }} cell
 */
export function isHoleDay(cell) {
  return cell.kinds.some((k) => HOLE_KINDS.has(k));
}

/**
 * HOW MANY OF THE BAND'S DAYS ARE SESSIONS, from the calendar code and never
 * from the lead kind (AH-03, g-ui-dom-vs-json-6): a closed day that also
 * read "not imported" was counted as a session (67 against the calendar's
 * 65 for GDFL NIFTY 2026-07..09). O(1) per day.
 *
 * @param {{ code: string }[]} cells
 */
export function sessionCount(cells) {
  let n = 0;
  for (const c of cells) if (isSessionCode(c.code)) n++;
  return n;
}

/**
 * The bars a held/peer object reports, or `null` when the feed holds no file
 * or the count could not be read.
 *
 * @param {any} held
 * @returns {number | null}
 */
export function barsOf(held) {
  return Number.isSafeInteger(held?.bars) ? held.bars : null;
}

/**
 * Every kind that applies to one record, most severe first.
 *
 * - `refused` / `incomplete`: the GDFL import journal's outcome (D-3317).
 * - `store_gap`: a day the calendar owes a session on and this feed holds a
 *   month file with no bar that day, and no import refusal explains it —
 *   GDFL 2022-03-07's indices before D-3317, Zerodha's Muhurat holes.
 * - `front`: a day before a series' first `1day` bar that a pull refused
 *   (D-3315).
 * - `degraded`, `gap`, `hole`, `shared_hole`, `dual`, `junk`, `late`: the
 *   vendor-fault fields (D-3335); `gap` is Zerodha's missing minutes only,
 *   and the GDFL quality ledger's three counts each have their own kind
 *   (AH-03, D-5834).
 * - `halt`, `special`, `unmeasured`, `cas`: the calendar's events (D-3319,
 *   D-3320). `cas` only for a share: an index has no auction phase.
 *
 * - `outside_archive`: a day before the vendor archive's first day or after
 *   its last (GDFL's runs 2018-09-03 to 2026-09-24, `/feeds.json` `archive`,
 *   D-3338): no file can exist, so it is neither a store gap nor "not
 *   imported yet" (DB-G67, DB-G68).
 * - `before_archive`: a day before the archive's first day when that day
 *   binds (`from_binds`, R3-05 repair 3): no file can exist, ever.
 * - Both only for a SESSION that holds no bar (AH-03, D-5833): a closed day
 *   is `closed` on either side of the span, and a day holding bars was
 *   imported.
 *
 * @param {any} rec one `/dayflags.json` day record
 *
 * @param {{ archive?: { from: string, to: string, from_binds?: boolean, to_binds?: boolean } | null }} [ctx] the feed's archive span, where it has one
 * @returns {string[]}
 */
export function kindsOf(rec, ctx = {}) {
  /** @type {string[]} */
  const out = [];
  const code = rec?.calendar?.code ?? '-';
  const session = isSessionCode(code);
  const outcome = rec?.import?.outcome ?? null;
  const day = String(rec?.day ?? '');
  const archive = ctx.archive ?? null;
  const held = barsOf(rec?.held);
  // BEFORE A FIRST DAY THAT BINDS no file can exist; anything else outside
  // the span (past a measured last day, which is a lower bound) is not
  // imported. An absent `from_binds` is a server that bound the first day.
  //
  // ONLY A SESSION CAN BE OUTSIDE THE ARCHIVE (AH-03, g-ui-dom-vs-json-6):
  // the span says where a vendor's FILES are, and a weekend or a holiday
  // owes none on either side of it, so a closed day stays 'closed' (which
  // the band did not count as a session) instead of leading with "not
  // imported" and being counted as one — 26-27 Sep 2026 on GDFL, and every
  // weekend of 2018-08 before the span. And a day that HOLDS bars was
  // imported, whatever the span's measured last day says (that day is a
  // lower bound, R3-05): GDFL's 25-30 Sep 2026 hold 22,500 bars each.
  const beyond =
    session && !(held !== null && held > 0) && archive !== null && day !== '' && (day < archive.from || day > archive.to);
  const before = beyond && day < /** @type {{ from: string }} */ (archive).from && archive?.from_binds !== false;
  if (outcome === 'refused') out.push('refused');
  // INCOMPLETE FOR ANOTHER FILE IS NOT THIS INSTRUMENT'S HOLE (DB-G69): a
  // day the import left incomplete over another contract's file still holds
  // this one's bars, and says so; it is a warning, not a refusal of this day.
  if (outcome === 'incomplete') {
    const refusals = rec?.import?.refusals;
    const others = Number(refusals?.this_count) === 0 && Number(refusals?.other_count) > 0;
    out.push(others ? 'incomplete_other' : 'incomplete');
  }
  // A TRADE-DRIVEN SERIES HOLDS A BAR ONLY WHERE A TRADE PRINTED (D-3336 §2):
  // an option, or a share at one second. A session it holds nothing on is a
  // day nothing traded, never a store gap; a Zerodha minute series, which
  // sends a candle for every minute of a session, is not one.
  const segment = segmentOf(rec?.instrument);
  const tradeDriven = segment === 'FNO' || (segment === 'CASH' && rec?.held?.rung === '1s');
  if (before) out.push('before_archive');
  else if (beyond) out.push('outside_archive');
  if (session && !beyond && held === 0 && outcome !== 'refused' && !(outcome === 'incomplete' && out.includes('incomplete'))) {
    out.push(tradeDriven ? 'no_trade' : 'store_gap');
  }
  if (rec?.refused_front === true) out.push('front');
  if (Array.isArray(rec?.degraded) && rec.degraded.length > 0) out.push('degraded');
  const gaps = rec?.gaps;
  if (gaps && Number(gaps.missing) > 0) out.push('gap');
  const q = rec?.quality;
  if (q && typeof q === 'object') {
    const idx = q.indices;
    const stk = q.stocks;
    // EACH QUALITY COUNT IS ITS OWN KIND (AH-03, g-ui-dom-vs-json-7): a hole
    // of seconds, a hole the shares share, and a second with two values are
    // three different facts, and none of them is a missing minute.
    if (idx && Number(idx.holes_5s) > 0) out.push('hole');
    if (stk && Number(stk.shared_holes) > 0) out.push('shared_hole');
    if (idx && Number(idx.dual_spread_s) > 0) out.push('dual');
  }
  if (Array.isArray(rec?.junk) && rec.junk.length > 0) out.push('junk');
  if (Number(rec?.import?.late) > 0) out.push('late');
  // HELD IS NOT TRADED (DB-B16): a share's day whose minute bars are mostly
  // zero-volume candles reads complete by count alone.
  const fills = Number.isSafeInteger(rec?.held?.zero_volume) ? rec.held.zero_volume : null;
  if (held !== null && held > 0 && fills !== null && fills * 2 >= held) out.push('fills');
  if (code === 'H' || rec?.events?.halt) out.push('halt');
  if (code === 'P') out.push('special');
  if (code === 'U') out.push('unmeasured');
  if (code === 'X' && segmentOf(rec?.instrument) === 'CASH') out.push('cas');
  if (session && !beyond && outcome === 'none' && rec?.import?.reached === false) out.push('not_reached');
  if (code === '.') out.push('closed');
  if (code === '-') out.push('outside');
  if (out.length === 0) out.push('session');
  // ONE KIND ONCE, in the fixed order of `DAY_KINDS`.
  const seen = new Set(out);
  return DAY_KINDS.map((k) => k.kind).filter((k) => seen.has(k));
}

/** @param {unknown} key */
function segmentOf(key) {
  const parts = typeof key === 'string' ? key.split('-') : [];
  return parts.length > 2 ? parts[1] : '';
}

/**
 * A record's own phrases, as the server wrote them, with ONE label added:
 * on an INDEX record the stocks file's closing-auction count is the whole
 * stocks file's, every share's, and an index has no auction (D-3320), so the
 * phrase the route builds from `quality.stocks.auction_prints` is prefixed
 * "stocks file, all shares:" (AH-03, g-ui-dom-vs-json-7; the route carries
 * the stocks line on an index record by design, D-3335 §3). The phrase is
 * matched exactly as the route composes it from that very count, so no other
 * phrase is touched. O(phrases).
 *
 * @param {any} rec
 * @returns {string[]}
 */
function saysOf(rec) {
  /** @type {string[]} */
  const says = Array.isArray(rec?.says) ? rec.says.map(String) : [];
  const n = rec?.quality?.stocks?.auction_prints;
  if (segmentOf(rec?.instrument) !== 'INDEX' || !Number.isSafeInteger(n) || n <= 0) return says;
  const phrase = `${n} closing-auction print(s), kept apart from trades`;
  return says.map((s) => (s === phrase ? `stocks file, all shares: ${s}` : s));
}

/**
 * One band cell per record: its day, its lead kind, every kind, the counts
 * of both feeds, and the record's own phrases.
 *
 * @param {any} rec
 */
export function cellOf(rec, ctx = {}) {
  const kinds = kindsOf(rec, ctx);
  const lead = KIND.get(kinds[0]) ?? { kind: kinds[0], word: kinds[0], tone: 'quiet' };
  return {
    day: String(rec?.day ?? ''),
    code: String(rec?.calendar?.code ?? '-'),
    kind: lead.kind,
    word: lead.word,
    tone: lead.tone,
    kinds,
    held: barsOf(rec?.held),
    heldWhy: typeof rec?.held?.why === 'string' ? rec.held.why : null,
    peer: barsOf(rec?.peer),
    peerFeed: typeof rec?.peer?.feed === 'string' ? rec.peer.feed : '',
    peerWhy: typeof rec?.peer?.why === 'string' ? rec.peer.why : null,
    // ZERO-VOLUME BARS, where the route counted them (a Zerodha share's
    // minutes, DB-B16); `null` is "not counted", never zero.
    fills: Number.isSafeInteger(rec?.held?.zero_volume) ? rec.held.zero_volume : null,
    // REFUSALS AN EARLIER RUN LOGGED (DB-B45): history, with the run that
    // logged them where the journal records it — never the current reason.
    superseded: Array.isArray(rec?.import?.refusals?.superseded) ? rec.import.refusals.superseded : [],
    says: saysOf(rec),
    flagged: kinds.some((k) => !QUIET.has(k))
  };
}

/**
 * Fold a `/dayflags.json` answer into cells, oldest first, and its notes.
 * Refuses a body with no `days` list: a malformed answer is named, never an
 * empty band a reader could take for "nothing was wrong".
 *
 * @param {any} body
 * @param {{ archive?: { from: string, to: string } | null }} [ctx] the feed's archive span (`/feeds.json` `archive`)
 */
export function bandOf(body, ctx = {}) {
  if (!Array.isArray(body?.days)) {
    throw new Error(`/dayflags.json answered without a days list${typeof body?.error === 'string' ? `: ${body.error}` : ''}`);
  }
  /** @type {ReturnType<typeof cellOf>[]} */
  const cells = body.days.map((/** @type {any} */ rec) => cellOf(rec, ctx));
  cells.sort((a, b) => (a.day < b.day ? -1 : a.day > b.day ? 1 : 0));
  return {
    cells,
    pageFrom: String(body.page_from ?? ''),
    pageTo: String(body.page_to ?? ''),
    page: Number(body.page ?? 0),
    pages: Number(body.pages ?? 1),
    notes: Array.isArray(body.notes) ? body.notes.map(String) : [],
    rungs: body.rungs ?? null,
    // WHERE A CONTRACT CONTINUES AFTER A BONUS OR SPLIT RENAMED IT (D-3331,
    // DB-G72): `null` for anything but a GDFL contract.
    lineage: lineageOf(body.lineage),
    // THE DERIVED RUNGS' WITHHELD BUCKETS, per month (D-3335 §2 `derive`
    // lines, DB-G36): `null` when the Zerodha ledger is not written yet.
    derive: deriveOf(body.derive)
  };
}

/**
 * The lineage section of an answer, as the band says it: one sentence per
 * record, or the reason no ledger could say (D-3331, DB-G72). `null` when
 * the answer carries none (an index, a share, Zerodha).
 *
 * EVERY RECORD KIND IS KEPT (AH-03, g-ui-dom-vs-json-2): this used to word
 * only `continues_as`, `renamed_from` and `stitched`, and dropped the
 * re-key's `superseded_by` and `supersedes` and the decoder's `retired` and
 * `stopped` in silence. The words are `lineage-labels.js`'s, the one place a
 * lineage record becomes a sentence; a kind it has none for is counted and
 * said. O(records) for one key.
 *
 * @param {any} l `body.lineage`
 * @returns {{ present: boolean, lines: string[], why: string | null, successor: string | null, retired: boolean, unknown: number, malformed: number } | null}
 */
export function lineageOf(l) {
  if (!l || typeof l !== 'object') return null;
  const got = lineageLines(l.records);
  return { present: l.present === true, ...got, why: typeof l.why === 'string' ? l.why : null };
}

/**
 * The Zerodha ledger's `derive` lines for the page's months, as the band says
 * them: one sentence per rung and month that withheld or cut a bucket
 * (DB-G36, D-3321). A month with nothing withheld is not listed. `null` when
 * the answer carries no ledger (not written yet, or a GDFL page).
 *
 * @param {any} lines `body.derive`
 * @returns {string[] | null}
 */
export function deriveOf(lines) {
  if (!Array.isArray(lines)) return null;
  return lines
    .filter((l) => Number(l?.withheld) > 0 || Number(l?.partial) > 0)
    .map(
      (l) =>
        `${l.rung} ${l.month}: ${l.buckets} bucket(s) folded over ${l.days} day(s), ${l.withheld} withheld${Number(l.partial) > 0 ? `, ${l.partial} partial` : ''} (a minute missing in the bucket, D-3321)`
    );
}

/**
 * How many cells of each kind lead, in the fixed order: the band's legend.
 *
 * @param {ReturnType<typeof cellOf>[]} cells
 */
export function tally(cells) {
  /** @type {Map<string, number>} */
  const by = new Map();
  for (const c of cells) by.set(c.kind, (by.get(c.kind) ?? 0) + 1);
  return DAY_KINDS.filter((k) => by.has(k.kind)).map((k) => ({ ...k, n: by.get(k.kind) ?? 0 }));
}

/**
 * WHY A DAY WINDOW HOLDS NO BAR (DB-016).
 *
 * The empty grid used to accuse the store — "a disagreement between the
 * manifest and the file" — for a day the import refused, a weekend, or a
 * holiday. This names each asked day's own reason from its record: `null`
 * when no record covers the window (the band has not answered), so the
 * caller says it is still reading rather than inventing a reason.
 *
 * @param {ReturnType<typeof cellOf>[]} cells the band's cells
 * @param {string} from `YYYY-MM-DD`, or `''` for the band's first day
 * @param {string} to `YYYY-MM-DD`, or `''` for its last
 * @param {(iso: string) => string} label renders a day for a reader
 * @param {number} [most] how many days are named before the rest are counted
 * @returns {string | null}
 */
export function emptyWindowWhy(cells, from, to, label, most = 4) {
  const asked = cells.filter((c) => (!from || c.day >= from) && (!to || c.day <= to));
  if (asked.length === 0) return null;
  // EVERY ASKED DAY THAT IS A SESSION IS NAMED FIRST: that is where a bar
  // was owed. A window of closed days is said to be closed. A session is
  // the CALENDAR'S word, never the lead kind's (AH-03, g-ui-dom-vs-json-6):
  // a weekend past the archive's measured end led with "not imported" and
  // was named as "No bar on any of 2 session day(s)".
  const sessions = asked.filter((c) => isSessionCode(c.code));
  // A TRADE-DRIVEN WINDOW WITH NO TRADE IN IT is said as that, never as a hole.
  if (sessions.length > 0 && sessions.every((c) => c.kind === 'no_trade')) {
    return sessions.length === 1
      ? `No bar on ${label(sessions[0].day)}: no trade printed that session — this series holds a bar only where a trade printed (D-3336).`
      : `No bar on any of ${sessions.length} session day(s): no trade printed in them — this series holds a bar only where a trade printed (D-3336).`;
  }
  if (sessions.length === 0) {
    const words = [...new Set(asked.map((c) => c.says[0] ?? c.word))];
    if (asked.length === 1) return `No bar on ${label(asked[0].day)}: ${words.join('; ')}.`;
    // EVERY ONE IS CLOSED only when the calendar says so of every one; a day
    // past the calendar's span (`-`) is not known to be closed.
    return asked.every((c) => c.code === '.')
      ? `No bar in ${asked.length} days: every one is closed (${words.join('; ')}).`
      : `No bar in ${asked.length} days: none is a session (${words.join('; ')}).`;
  }
  // A DAY THAT HOLDS BARS IS NOT WHY THE WINDOW IS EMPTY: its bars fall
  // outside the time-of-day window. Only the days holding none are named.
  const empty = sessions.filter((c) => !(c.held !== null && c.held > 0));
  if (empty.length === 0) {
    const bars = sessions.reduce((n, c) => n + (c.held ?? 0), 0);
    return `Every one of the ${sessions.length} session day(s) holds bars (${bars} in all); none falls inside the time-of-day window.`;
  }
  // A DAY OUTSIDE THE ARCHIVE IS SAID AS THAT: the server's own phrases for
  // it ("not reached by the import yet") describe an import, not a vendor
  // that never published the day.
  const named = empty
    .slice(0, most)
    .map((c) => `${label(c.day)}: ${c.kind === 'outside_archive' || c.kind === 'before_archive' || !c.says.length ? c.word : c.says.join('; ')}`);
  const rest = empty.length - named.length;
  const lead = empty.length === 1 ? 'No bar on' : `No bar on any of ${empty.length} session day(s) —`;
  return `${lead} ${named.join(' · ')}${rest > 0 ? ` · and ${rest} more, each in the day band above` : ''}.`;
}

/**
 * The day span a band should ask for: the window when it fits one page,
 * else the newest page of it. `null` when there is no window to ask about.
 *
 * @param {string} from `YYYY-MM-DD` or `''`
 * @param {string} to `YYYY-MM-DD` or `''`
 * @param {(iso: string, days: number) => string} addDays
 * @returns {{ from: string, to: string, clipped: boolean } | null}
 */
export function bandSpan(from, to, addDays) {
  if (!from || !to || from > to) return null;
  const earliest = addDays(to, -(DAYFLAGS_PAGE_DAYS - 1));
  return earliest > from ? { from: earliest, to, clipped: true } : { from, to, clipped: false };
}

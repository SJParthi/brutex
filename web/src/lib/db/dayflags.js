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

/** The most days one `/dayflags.json` page holds (`dayflags::PAGE_DAYS`). */
export const DAYFLAGS_PAGE_DAYS = 92;

/**
 * The `/dayflags.json` URL for one feed, instrument and day span.
 *
 * THE GRID'S RUNG RIDES ALONG (R3-13 goal 1): `timeframe` is the rung the
 * grid shows, so the band counts what the grid shows — a Zerodha `1day` grid
 * asks for day bars, and a session that holds minutes and no day bar is the
 * day bar missing. `expiry` narrows an option family (`NSE-FNO-NIFTY`) to one
 * expiry, so the days after it read expired and owe nothing (SC-CLOSE-12).
 * Either is left out when empty, and the server counts the feed's finest.
 *
 * @param {string} feed
 * @param {string} instrument a census key, contract included
 * @param {string} from `YYYY-MM-DD`
 * @param {string} to `YYYY-MM-DD`
 * @param {number} [page]
 * @param {{ timeframe?: string, expiry?: string }} [at]
 */
export function dayflagsUrl(feed, instrument, from, to, page = 0, at = {}) {
  const q = new URLSearchParams({ feed, instrument, from, to, order: 'asc' });
  if (page > 0) q.set('page', String(page));
  if (at.timeframe) q.set('timeframe', at.timeframe);
  if (at.expiry) q.set('expiry', at.expiry);
  return `/dayflags.json?${q}`;
}

/**
 * THE ONE INSTRUMENT A DAY FLAG IS ABOUT, from the rows the selection
 * matches at the grid's rung: the exact key when there is one series (a spot
 * series, or ONE CONTRACT — R3-13: the band asks about that contract, never
 * the whole option family), else the underlying's options as a kind
 * (`NSE-FNO-NIFTY`), whose calendar and import days still answer though no
 * one contract's bars can be counted. `''` when the selection spans
 * instruments. A row of another rung than the grid's is not one the grid
 * shows and does not count; `rung` `''` counts every row.
 *
 * @param {any[]} rows
 * @param {string} [rung] the grid's rung
 */
export function bandInstrument(rows, rung = '') {
  let one = '';
  let base = '';
  for (const r of rows) {
    if (r.absent || (rung && r.timeframe && r.timeframe !== rung)) continue;
    if (one === '') one = r.instrument;
    else if (one !== r.instrument) one = '\u0000';
    const parts = String(r.instrument).split('-');
    const kind = parts.length > 2 ? `${parts[0]}-${parts[1]}-${r.underlying ?? parts.slice(2).join('-')}` : '';
    if (base === '') base = kind;
    else if (base !== kind) base = '\u0000';
  }
  if (one !== '' && one !== '\u0000') return one;
  return base !== '' && base !== '\u0000' ? base : '';
}

/**
 * THE ONE EXPIRY AN OPTION FAMILY'S ROWS SHARE (SC-CLOSE-12), or `''`: a
 * selection of one expiry at any strike asks the family, and the days after
 * that expiry then read expired and owe nothing, never a hole. Sent only
 * where the band asks the family: a contract carries its expiry in its key.
 *
 * @param {any[]} rows
 * @param {string} instrument what `bandInstrument` answered
 * @param {string} [rung] the grid's rung
 */
export function bandExpiry(rows, instrument, rung = '') {
  const family = String(instrument).split('-')[1] === 'FNO' && !/-\d{4}-\d{2}-\d{2}-/.test(String(instrument));
  if (!family) return '';
  let expiry = '';
  for (const r of rows) {
    if (r.absent || (rung && r.timeframe && r.timeframe !== rung)) continue;
    const e = typeof r.expiry === 'string' ? r.expiry : '';
    if (e === '') return '';
    if (expiry === '') expiry = e;
    else if (expiry !== e) return '';
  }
  return expiry;
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
  { kind: 'gap', word: 'minute gap', tone: 'warn' },
  { kind: 'junk', word: 'junk row', tone: 'warn' },
  // AN INDEX THAT STOPPED CHANGING, OR PRINTED OUTSIDE ITS DAY'S RANGE
  // (DB-B20): a frozen dissemination is not a halt and not a vendor gap.
  { kind: 'frozen', word: 'index frozen in session', tone: 'warn' },
  { kind: 'outside_range', word: 'intraday value outside the day range', tone: 'warn' },
  // A CLOSED DAY THE OTHER FEED TRADED (DB-A15): evidence the calendar,
  // measured from Zerodha's own bars, cannot give by itself.
  { kind: 'peer_on_closed', word: 'other feed traded on a closed day', tone: 'warn' },
  // A DAY BUILT UNDER A BAR DEFINITION THIS BUILD WOULD NOT BUILD (R3-13
  // goal 3): labelled until it is restated.
  { kind: 'old_definition', word: 'built under an older bar definition', tone: 'warn' },
  { kind: 'incomplete_other', word: 'import incomplete for other files', tone: 'warn' },
  { kind: 'late', word: 'late rows deferred', tone: 'warn' },
  { kind: 'fills', word: 'mostly zero-volume minutes', tone: 'warn' },
  { kind: 'halt', word: 'suspected halt', tone: 'info' },
  { kind: 'special', word: 'special session', tone: 'info' },
  { kind: 'unmeasured', word: 'session of unmeasured length', tone: 'info' },
  { kind: 'cas', word: 'closing auction from 15:15', tone: 'info' },
  // A DAY AFTER A CONTRACT'S EXPIRY OWES NOTHING (SC-CLOSE-12): never a hole.
  { kind: 'expired', word: 'after expiry', tone: 'quiet' },
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

/** The word of a closing-auction day whose dated flag no master holds (SC-CLOSE-8). */
export const CAS_UNHELD = 'closing-auction window, flag not held';

/** The kinds that are facts about the day, not flags: an ordinary or closed
 *  day, a day outside a span, a trade-driven day with no trade. */
const QUIET = new Set(['session', 'closed', 'outside', 'outside_archive', 'before_archive', 'no_trade', 'expired']);

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
 * - `degraded`, `gap`, `junk`, `late`: the vendor-fault fields (D-3335).
 * - `halt`, `special`, `unmeasured`, `cas`: the calendar's events (D-3319,
 *   D-3320). `cas` only for a share: an index has no auction phase.
 *
 * - `outside_archive`: a day before the vendor archive's first day or after
 *   its last (GDFL's runs 2018-09-03 to 2026-09-24, `/feeds.json` `archive`,
 *   D-3338): no file can exist, so it is neither a store gap nor "not
 *   imported yet" (DB-G67, DB-G68).
 *
 * @param {any} rec one `/dayflags.json` day record
 * - `before_archive`: a day before the archive's first day when that day
 *   binds (`from_binds`, R3-05 repair 3): no file can exist, ever.
 *
 * @param {{ archive?: { from: string, to: string, from_binds?: boolean, to_binds?: boolean } | null }} [ctx] the feed's archive span, where it has one
 * @returns {string[]}
 */
export function kindsOf(rec, ctx = {}) {
  /** @type {string[]} */
  const out = [];
  const code = rec?.calendar?.code ?? '-';
  const session = code === 'R' || code === 'X' || code === 'P' || code === 'H' || code === 'U';
  const outcome = rec?.import?.outcome ?? null;
  const day = String(rec?.day ?? '');
  const archive = ctx.archive ?? null;
  // BEFORE A FIRST DAY THAT BINDS no file can exist; anything else outside
  // the span (past a measured last day, which is a lower bound) is not
  // imported. An absent `from_binds` is a server that bound the first day.
  const beyond = archive !== null && day !== '' && (day < archive.from || day > archive.to);
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
  const held = barsOf(rec?.held);
  // A TRADE-DRIVEN SERIES HOLDS A BAR ONLY WHERE A TRADE PRINTED (D-3336 §2):
  // an option, or a share at one second. A session it holds nothing on is a
  // day nothing traded, never a store gap; a Zerodha minute series, which
  // sends a candle for every minute of a session, is not one.
  const segment = segmentOf(rec?.instrument);
  // THE FINEST RUNG DECIDES IT, whatever rung the grid counts (R3-13): `held`
  // may be the grid's coarser rung, `finest` is the feed's own.
  const tradeDriven = segment === 'FNO' || (segment === 'CASH' && (rec?.finest?.rung ?? rec?.held?.rung) === '1s');
  if (before) out.push('before_archive');
  else if (beyond) out.push('outside_archive');
  // A DAY AFTER EXPIRY OWES NOTHING (SC-CLOSE-12): no store gap, no "no
  // trade", no "not imported yet" — `/gaps.json` calls it expired too.
  const expired = rec?.calendar?.expired === true || rec?.contract?.expired === true;
  if (expired) out.push('expired');
  if (session && !beyond && !expired && held === 0 && outcome !== 'refused' && !(outcome === 'incomplete' && out.includes('incomplete'))) {
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
    if ((idx && (Number(idx.holes_5s) > 0 || Number(idx.dual_spread_s) > 0)) || (stk && Number(stk.shared_holes) > 0)) {
      out.push('gap');
    }
  }
  if (Array.isArray(rec?.junk) && rec.junk.length > 0) out.push('junk');
  if (rec?.index?.frozen?.flagged === true) out.push('frozen');
  if (rec?.index?.outside_day_range?.flagged === true) out.push('outside_range');
  const peer = barsOf(rec?.peer);
  if (code === '.' && peer !== null && peer > 0) out.push('peer_on_closed');
  if (rec?.bar_definition?.current === false) out.push('old_definition');
  if (Number(rec?.import?.late) > 0) out.push('late');
  // HELD IS NOT TRADED (DB-B16): a share's day whose minute bars are mostly
  // zero-volume candles reads complete by count alone.
  // The minutes are the finest rung's: on a coarser grid they ride on `finest`.
  const minutes = minutesOf(rec);
  const minuteBars = barsOf(minutes);
  const fills = Number.isSafeInteger(minutes?.zero_volume) ? minutes.zero_volume : null;
  if (minuteBars !== null && minuteBars > 0 && fills !== null && fills * 2 >= minuteBars) out.push('fills');
  if (code === 'H' || rec?.events?.halt) out.push('halt');
  if (code === 'P') out.push('special');
  if (code === 'U') out.push('unmeasured');
  // THE CLOSING AUCTION ONLY ON A DAY THE SHARE'S DATED FLAG IS 1 (SC-CLOSE-9):
  // the calendar's `X` says the day has an auction for eligible shares, and
  // `calendar.cas.eligible` says whether this share was one that day. Flag 0
  // is an ordinary session. A flag no master holds keeps the kind — its
  // 15:15-15:29 window is the day record's `unverified` phase, which the chart
  // draws as such (`chart.js` `dayBands`) — and its WORD says the flag is not
  // held (`cellOf`), never that the day has an auction.
  if (code === 'X' && segmentOf(rec?.instrument) === 'CASH' && rec?.calendar?.cas?.eligible !== false) out.push('cas');
  if (session && !beyond && !expired && outcome === 'none' && rec?.import?.reached === false) out.push('not_reached');
  if (code === '.') out.push('closed');
  if (code === '-') out.push('outside');
  if (out.length === 0) out.push('session');
  // ONE KIND ONCE, in the fixed order of `DAY_KINDS`.
  const seen = new Set(out);
  return DAY_KINDS.map((k) => k.kind).filter((k) => seen.has(k));
}

/**
 * The count a record's zero-volume minutes are counted in: `finest` where the
 * grid's rung is another one (R3-13), else `held`.
 *
 * @param {any} rec
 */
function minutesOf(rec) {
  return rec?.finest && typeof rec.finest === 'object' ? rec.finest : rec?.held;
}

/** @param {unknown} key */
function segmentOf(key) {
  const parts = typeof key === 'string' ? key.split('-') : [];
  return parts.length > 2 ? parts[1] : '';
}

/**
 * One band cell per record: its day, its lead kind, every kind, the counts
 * of both feeds, and the record's own phrases.
 *
 * @param {any} rec
 */
export function cellOf(rec, ctx = {}) {
  const kinds = kindsOf(rec, ctx);
  // EVERY KIND `kindsOf` ANSWERS IS ONE OF `DAY_KINDS` (it filters by them, and
  // always answers one), so the lookup always finds it.
  const known = /** @type {(typeof DAY_KINDS)[number]} */ (KIND.get(kinds[0]));
  // A CLOSING-AUCTION DAY WHOSE FLAG NO MASTER HOLDS is said as that
  // (SC-CLOSE-8, SC-CLOSE-9): never "closing auction from 15:15".
  const lead = known.kind === 'cas' && rec?.calendar?.cas?.eligible !== true ? { ...known, word: CAS_UNHELD } : known;
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
    fills: Number.isSafeInteger(minutesOf(rec)?.zero_volume) ? minutesOf(rec).zero_volume : null,
    // REFUSALS AN EARLIER RUN LOGGED (DB-B45): history, with the run that
    // logged them where the journal records it — never the current reason.
    superseded: Array.isArray(rec?.import?.refusals?.superseded) ? rec.import.refusals.superseded : [],
    says: Array.isArray(rec?.says) ? rec.says.map(String) : [],
    // THE GRID'S RUNG AND THE FINEST BESIDE IT (R3-13 goal 1): `held` counts
    // what the grid shows; `finest` is the feed's finest rung where the two
    // differ, `null` otherwise.
    rung: typeof rec?.held?.rung === 'string' ? rec.held.rung : '',
    finest: barsOf(rec?.finest),
    finestRung: typeof rec?.finest?.rung === 'string' ? rec.finest.rung : '',
    // THE BAR DEFINITION THAT BUILT THE DAY (R3-13 goal 3), or `null`.
    definition: definitionOf(rec?.bar_definition),
    // THE CLOSING-AUCTION FLAG (SC-CLOSE-8): `true`, `false`, or `null` when
    // no master holds it or none applies.
    cas: typeof rec?.calendar?.cas?.eligible === 'boolean' ? rec.calendar.cas.eligible : null,
    auctionSidecar: typeof rec?.auction_sidecar === 'boolean' ? rec.auction_sidecar : null,
    // DAY-LEDGER FIELDS THIS PAGE HAS NO NAME FOR (R3-13 goal 5), drawn by
    // the names they were written under.
    unknown: unknownFieldsOf(rec?.quality),
    flagged: kinds.some((k) => !QUIET.has(k))
  };
}

/**
 * The bar definition of a day, as the band says it: `null` where the record
 * names none.
 *
 * @param {any} d `rec.bar_definition`
 * @returns {{ value: number, current: boolean, means: string, from: string, thisBuild: number | null } | null}
 */
export function definitionOf(d) {
  if (!d || typeof d !== 'object' || !Number.isSafeInteger(d.value)) return null;
  return {
    value: d.value,
    current: d.current === true,
    means: typeof d.means === 'string' ? d.means : '',
    from: typeof d.from === 'string' ? d.from : '',
    thisBuild: Number.isSafeInteger(d.this_build) ? d.this_build : null
  };
}

/**
 * Every day-ledger field the server serves and this page has no name for,
 * kind by kind: the ones a line lists under `unknown`, with their values.
 * A field a later writer adds (R3-26) appears here with no change to this
 * page.
 *
 * @param {any} quality `rec.quality`
 * @returns {{ kind: string, name: string, value: string }[]}
 */
export function unknownFieldsOf(quality) {
  /** @type {{ kind: string, name: string, value: string }[]} */
  const out = [];
  if (!quality || typeof quality !== 'object') return out;
  for (const kind of ['indices', 'stocks', 'options']) {
    const line = quality[kind];
    if (!line || typeof line !== 'object' || !Array.isArray(line.unknown)) continue;
    for (const name of line.unknown) {
      if (typeof name !== 'string' || !(name in line)) continue;
      const value = line[name];
      out.push({ kind, name, value: value === null ? 'null' : String(value) });
    }
  }
  return out;
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
 * @param {any} l `body.lineage`
 * @returns {{ present: boolean, lines: string[], why: string | null } | null}
 */
export function lineageOf(l) {
  if (!l || typeof l !== 'object') return null;
  const records = Array.isArray(l.records) ? l.records : [];
  /** @type {string[]} */
  const lines = [];
  for (const r of records) {
    if (r?.kind === 'continues_as') lines.push(`renamed at the ${r.ex} ex-date, continues as ${r.key} (factor ${r.factor})`);
    else if (r?.kind === 'renamed_from') lines.push(`renamed at the ${r.ex} ex-date from ${r.key} (factor ${r.factor})`);
    else if (r?.kind === 'stitched') {
      lines.push(`holds two exchange contracts: its name was reused at the ${r.ex} ex-date${r.after ? `, then holding the contract that was ${r.after}` : ''}`);
    }
  }
  return { present: l.present === true, lines, why: typeof l.why === 'string' ? l.why : null };
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
  // BY KIND AND THE CELL'S OWN WORD: a closing-auction day whose flag is not
  // held is counted apart from one whose flag is 1 (SC-CLOSE-9).
  /** @type {Map<string, { n: number, word: string }[]>} */
  const by = new Map();
  for (const c of cells) {
    const words = by.get(c.kind) ?? [];
    const seen = words.find((w) => w.word === c.word);
    if (seen) seen.n += 1;
    else words.push({ word: c.word, n: 1 });
    by.set(c.kind, words);
  }
  return DAY_KINDS.flatMap((k) => (by.get(k.kind) ?? []).map((w) => ({ ...k, word: w.word, n: w.n })));
}

/**
 * The band's closing-auction line, counted (SC-CLOSE-8, SC-CLOSE-9): of the
 * cells whose only kind is `cas`, the days the share's dated flag is 1
 * (`days`), the rest, whose flag no master holds (`unheld`), and of the
 * first only, the days whose month keeps the print apart in its sidecar
 * (`apart`). An unheld day is never counted apart: the line says "kept
 * apart" of a closing-auction day, and that day is not known to be one.
 *
 * @param {ReturnType<typeof cellOf>[]} cells
 */
export function casCounts(cells) {
  const lone = cells.filter((c) => c.kind === 'cas' && c.kinds.length === 1);
  const flagged = lone.filter((c) => c.cas === true);
  return {
    days: flagged.length,
    unheld: lone.length - flagged.length,
    apart: flagged.filter((c) => c.auctionSidecar === true).length
  };
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
  // was owed. A window of closed days is said to be closed.
  const sessions = asked.filter((c) => c.kind !== 'closed' && c.kind !== 'outside');
  // A TRADE-DRIVEN WINDOW WITH NO TRADE IN IT is said as that, never as a hole.
  if (sessions.length > 0 && sessions.every((c) => c.kind === 'no_trade')) {
    return sessions.length === 1
      ? `No bar on ${label(sessions[0].day)}: no trade printed that session — this series holds a bar only where a trade printed (D-3336).`
      : `No bar on any of ${sessions.length} session day(s): no trade printed in them — this series holds a bar only where a trade printed (D-3336).`;
  }
  if (sessions.length === 0) {
    const words = [...new Set(asked.map((c) => c.says[0] ?? c.word))];
    return asked.length === 1
      ? `No bar on ${label(asked[0].day)}: ${words.join('; ')}.`
      : `No bar in ${asked.length} days: every one is closed (${words.join('; ')}).`;
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

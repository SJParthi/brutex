// /db — PRICE BASIS, CLOSE SEMANTICS, THE CLOSING AUCTION AND THE SOURCE
// COLUMN (WP-55; D-3320, D-3323, D-3324, D-3334; DB-017, DB-033, DB-035,
// DB-078, DB-082, DB-G38 to DB-G43, DB-G56, DB-G58, DB-G75, DB-A40).
//
// Every finding this file pins was measured on the operator's store before
// the fix: no /db text said Zerodha's share prices are back-adjusted (HDFCBANK
// 2019-06-03 at a quarter of GDFL's open), a Zerodha day close was shown as if
// it were the last print, GDFL's closing-auction print was an ordinary trade
// row, the BARS_PER_SESSION comment named a 15:29 continuous close that no
// session has, and the provenance drawer rendered nothing at all.
//
// THE FIXTURES ARE INVENTED. The `/feeds.json` rows below have the server's
// SHAPE (`crates/api/src/feeds_route.rs`) with words of this file's own; the
// `/provenance.json` and `/sources/day.json` answers have the routes' shape
// with made-up journal lines, run ids and paths; every price is a round
// number no instrument traded at. Nothing below is market evidence.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  BARS_PER_SESSION,
  segmentOfKey,
  selectionSegments,
  basisRow,
  priceBadge,
  volumeBadge,
  closeBadge,
  auctionNote,
  spotNote,
  phaseWord,
  auctionVerdict,
  auctionMarks,
  provenanceUrl,
  barAtUrl,
  casTailUrl,
  dayFactsUrl,
  reachesAuction,
  auctionDaysOnPage,
  owns,
  sourceNotRead,
  CAS_DAYS_MAX,
  CAS_READ_TO,
  filesOnPage,
  sourceRuns,
  foldRuns,
  rowsLabel,
  sourceOfDay,
  recordText,
  drawerFacts,
  shareBasisUrl,
  basisRunsOf,
  basisRunLabel,
  definitionFact,
  remember,
  PROV_FILES_MAX,
  PROV_CACHE_MAX
} from '../src/lib/db/notes-labels.js';

/* ── invented /feeds.json rows, in the server's shape ─────────────────── */

/**
 * One basis row.
 * @param {string} segment
 * @param {string} rung
 * @param {'raw' | 'as_published' | 'vendor_adjusted'} price
 * @param {'raw' | 'none' | 'vendor_own'} volume
 * @param {string} close
 * @param {object} [extra]
 */
const row = (segment, rung, price, volume, close, extra = {}) => ({
  segment,
  rung,
  derived: false,
  price,
  price_label: { raw: 'raw, as traded', as_published: 'as published, an index level', vendor_adjusted: 'back-adjusted by the vendor as of the pull date' }[price],
  volume,
  volume_label: { raw: 'raw, the traded quantity', none: 'none, an index has no traded volume', vendor_own: "the vendor's own basis, not scaled with its price" }[volume],
  adjusted_as_of: price === 'vendor_adjusted' ? 'pull_date' : null,
  close,
  close_label: close === 'official' ? 'official close' : 'last print',
  day_is_fold: rung === '1day' ? false : null,
  oi: segment === 'FNO' ? 'carried' : 'not_applicable',
  source: `fixture source for ${segment} ${rung}`,
  ...extra
});

const ZERODHA = {
  wire: 'zerodha',
  display: 'Zerodha',
  basis: [
    row('INDEX', '1min', 'as_published', 'none', 'last_print'),
    row('CASH', '1min', 'vendor_adjusted', 'vendor_own', 'last_print'),
    row('INDEX', '1day', 'as_published', 'none', 'official'),
    row('CASH', '1day', 'vendor_adjusted', 'vendor_own', 'official'),
    row('INDEX', '5min', 'as_published', 'none', 'last_print', { derived: true })
  ],
  basis_unverified: null,
  close_source: 'fixture: the day close is official',
  closing_auction: {
    from: '2026-08-03',
    eligible_continuous_close: '15:15',
    handling: 'minutes_end_before_auction',
    label: 'minute bars of a closing-auction-eligible share end at 15:14 by design; the day close is the auction price',
    source: 'fixture auction source'
  },
  window: { from: '2018-01-01', to: '2026-09-30' }
};

const GDFL = {
  wire: 'gdfl',
  display: 'GDFL',
  basis: [
    row('INDEX', '1s', 'as_published', 'none', 'last_print'),
    row('CASH', '1s', 'raw', 'raw', 'last_print'),
    row('FNO', '1s', 'raw', 'raw', 'last_print')
  ],
  basis_unverified: null,
  close_source: 'fixture: a second closes on its last kept row',
  closing_auction: {
    from: '2026-08-03',
    eligible_continuous_close: '15:15',
    handling: 'print_kept_apart',
    label: 'the closing-auction print is kept apart from trades; months imported before that rule hold it as an ordinary one-second bar until re-imported',
    source: 'fixture auction source'
  },
  window: null
};

const UNMEASURED = { wire: 'other', display: 'Other', basis: [], basis_unverified: 'UNVERIFIED: fixture says nobody measured it', closing_auction: null };

/**
 * An IST instant as epoch milliseconds.
 * @param {string} day
 * @param {string} clock
 */
const ist = (day, clock) => Date.parse(`${day}T${clock}+05:30`);

/**
 * A grid row, as `DbBarsGrid` builds one.
 * @param {string} instrument
 * @param {string} tf
 * @param {string} day
 * @param {string} clock
 * @param {object} [extra]
 * @returns {any}
 */
const bar = (instrument, tf, day, clock, extra = {}) => {
  const ts = ist(day, clock);
  return { rk: `${instrument} ${ts / 1000}`, instrument, tf, month: day.slice(0, 7), day, ts, ...extra };
};

/* ── DB-082: the session comment ──────────────────────────────────────── */

test('DB-082: the session comment names three sessions, and no longer a 15:29 continuous close', () => {
  const src = readFileSync(new URL('../src/lib/db/notes-labels.js', import.meta.url), 'utf8');
  const at = src.indexOf('export const BARS_PER_SESSION');
  // The comment as prose: the ` * ` gutter and the line breaks folded away.
  const comment = src
    .slice(src.lastIndexOf('/**', at), at)
    .replace(/\n\s*\* ?/g, ' ')
    .replace(/\s+/g, ' ');
  assert.doesNotMatch(comment, /continuous session ends at 15:29/);
  // The three sessions of charter §7 and §9, each with its own close.
  assert.match(comment, /an index\*\*: unchanged, 09:15–15:30/);
  assert.match(comment, /continuous trading 09:15–15:15, then the closing auction to 15:30 — 360 minute bars, 09:15–15:14/);
  assert.match(comment, /derivatives\*\*: 09:15–15:40/);
  assert.equal(BARS_PER_SESSION, 375);
});

/* ── segments ─────────────────────────────────────────────────────────── */

test('a census key names its store segment, and nothing else is guessed', () => {
  assert.equal(segmentOfKey('NSE-INDEX-NIFTY'), 'INDEX');
  assert.equal(segmentOfKey('NSE-CASH-BAJAJ-AUTO'), 'CASH');
  assert.equal(segmentOfKey('NSE-FNO-NIFTY-2019-03-28-1150000-CE'), 'FNO');
  assert.equal(segmentOfKey('NSE-BOND-X'), null);
  assert.equal(segmentOfKey('NIFTY'), null);
  assert.equal(segmentOfKey(undefined), null);
});

test('the badge covers exactly the segments the selection shows', () => {
  const kinds = [['FNO', 9000], ['INDEX', 141]];
  assert.deepEqual(selectionSegments(kinds, ''), ['INDEX', 'FNO']);
  assert.deepEqual(selectionSegments(kinds, 'spot'), ['INDEX']);
  assert.deepEqual(selectionSegments(kinds, 'options-cut'), ['FNO']);
  assert.deepEqual(selectionSegments([['CASH', 3], ['BOND', 1]], ''), ['CASH']);
  assert.deepEqual(selectionSegments(null, ''), []);
});

/* ── D-3323: the price basis ──────────────────────────────────────────── */

test('DB-017/DB-G43/DB-G75: a Zerodha share is back-adjusted as of its pull date; GDFL is raw', () => {
  const z = priceBadge(ZERODHA, 'CASH', '1day', '2026-10-06');
  assert.equal(z.tone, 'adjusted');
  assert.equal(z.short, 'back-adjusted');
  assert.equal(z.text, 'back-adjusted by Zerodha as of the pull date 2026-10-06');
  // DB-G38/DB-G40: the non-proportional and quantised runs, and DB-G58: never side by side.
  assert.match(z.title, /not proportional/);
  assert.match(z.title, /coarser than the exchange's tick/);
  assert.match(z.title, /Never set beside GDFL's raw price as a disagreement/);
  assert.match(z.title, /fixture source for CASH 1day/);
  // Without a pull date it says where each month's is, never a guessed one.
  const open = priceBadge(ZERODHA, 'CASH', '');
  assert.equal(open.text, 'back-adjusted by Zerodha as of the pull date');
  assert.match(open.title, /Each month's pull date is under Source/);
  const g = priceBadge(GDFL, 'CASH', '1s');
  assert.deepEqual([g.tone, g.short, g.text], ['raw', 'raw, as traded', 'raw, as traded']);
  assert.equal(priceBadge(GDFL, 'FNO', '1s').short, 'raw, as traded');
  assert.equal(priceBadge(ZERODHA, 'INDEX', '1day').short, 'index level');
});

test('a feed nobody measured, or a rung a feed does not hold, claims no basis', () => {
  const u = priceBadge(UNMEASURED, 'CASH', '1min');
  assert.equal(u.tone, 'unverified');
  assert.match(u.text, /UNVERIFIED: fixture says nobody measured it/);
  const missing = priceBadge(GDFL, 'CASH', '1day');
  assert.equal(missing.short, 'basis not served');
  assert.match(missing.text, /names no price basis for CASH at 1day on GDFL/);
  assert.equal(priceBadge(null, 'CASH').short, 'no feed');
  assert.equal(closeBadge(UNMEASURED, 'CASH', '1min'), null);
});

test('the price basis of a segment is one fact at every rung; the close is per rung', () => {
  assert.equal(basisRow(ZERODHA, 'CASH', '').rung, '1min');
  assert.equal(basisRow(ZERODHA, 'CASH', '1day').close, 'official');
  assert.equal(basisRow(ZERODHA, 'FNO', ''), null);
});

test('DB-033/DB-G39: a Zerodha share volume is flagged on its own basis; GDFL volume is not', () => {
  const v = volumeBadge(ZERODHA, 'CASH', '1min');
  assert.equal(v?.short, "volume: vendor's basis");
  assert.match(v?.title ?? '', /nothing on this page derives turnover or VWAP/);
  assert.equal(volumeBadge(GDFL, 'CASH', '1s'), null);
  assert.equal(volumeBadge(ZERODHA, 'INDEX', '1min'), null);
});

/* ── D-3324: official close against last print ────────────────────────── */

test('DB-035/DB-G41: a day close is the official close, an intraday close the last print', () => {
  const day = closeBadge(ZERODHA, 'INDEX', '1day');
  assert.equal(day?.short, 'official close');
  assert.equal(day?.text, '1day close: the official close, not a fold of its minutes, not the last print');
  assert.match(day?.title ?? '', /2,081 of 2,126/);
  assert.equal(closeBadge(ZERODHA, 'INDEX', '1min')?.short, 'close = last print');
  assert.equal(closeBadge(GDFL, 'INDEX', '1s')?.short, 'close = last print');
  const every = closeBadge(ZERODHA, 'INDEX', '');
  assert.equal(every?.tone, 'mixed');
  assert.equal(every?.short, '1day: official close · intraday: last print');
});

/* ── D-3320: the closing auction ──────────────────────────────────────── */

test('DB-G42: a CASH selection that reaches 2026-08-03 says what each feed holds of the auction', () => {
  const z = auctionNote(ZERODHA, 'CASH', '2026-09');
  assert.equal(z?.short, 'closing auction from 03 Aug 2026');
  assert.match(z?.text ?? '', /until 15:15, then in the closing auction to 15:30; minute bars of a closing-auction-eligible share end at 15:14 by design/);
  assert.match(z?.title ?? '', /derivatives run to 15:40/);
  assert.match(auctionNote(GDFL, 'CASH', '')?.text ?? '', /kept apart from trades/);
  assert.equal(auctionNote(ZERODHA, 'CASH', '2019-05'), null);
  // A day bound is compared as a day: the auction's first day is in, the day before is not.
  assert.equal(auctionNote(ZERODHA, 'CASH', '2019-06-03'), null);
  assert.equal(auctionNote(ZERODHA, 'CASH', '2026-08-02'), null);
  assert.equal(auctionNote(ZERODHA, 'CASH', '2026-08-03')?.short, 'closing auction from 03 Aug 2026');
  assert.equal(auctionNote(ZERODHA, 'CASH', '2026-08')?.short, 'closing auction from 03 Aug 2026');
  assert.equal(auctionNote(ZERODHA, 'INDEX', '2026-09'), null);
});

test('DB-G58: an option is priced against its own feed only', () => {
  assert.match(spotNote(GDFL, 'FNO')?.text ?? '', /read from GDFL's own underlying .* never from another feed's: a GDFL strike is never priced against a Zerodha spot/);
  assert.equal(spotNote(GDFL, 'CASH'), null);
});

test('a feed name ending in s takes a bare apostrophe', () => {
  assert.equal(owns('Zerodha'), "Zerodha's");
  assert.equal(owns('Global Datafeeds'), "Global Datafeeds'");
  assert.match(spotNote({ ...GDFL, display: 'Global Datafeeds' }, 'FNO')?.text ?? '', /read from Global Datafeeds' own underlying/);
});

test('phases are the server words, and an unread phase stays unread', () => {
  assert.equal(phaseWord('cas'), 'closing-auction session');
  assert.equal(phaseWord('continuous'), 'continuous trading');
  assert.equal(phaseWord('new_phase'), 'phase new_phase');
  assert.equal(phaseWord(undefined), undefined);
  assert.equal(phaseWord(null), null);
});

/* The day's closing-auction tail (newest first) and auction record, invented. */
const CAS_DAY = '2026-09-24';
const lastT = ist(CAS_DAY, '15:28:07') / 1000;
const firstCasT = ist(CAS_DAY, '15:15:00') / 1000;
const tail = { bars: [{ t: lastT, c: 50000, phase: 'cas' }, { t: firstCasT, c: 50010, phase: 'cas' }] };
const noSidecar = {
  day: CAS_DAY,
  auction: { state: 'no_sidecar', says: 'fixture: no sidecar' },
  close: { official_vs_gdfl_last_print: { bps: 0, flag: false } },
  last_print: { gdfl_last_print_at: '15:28:07' }
};

test('DB-G42: the day\'s last print in the closing-auction session is the auction\'s, in a month imported before the rule', () => {
  const v = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail, dayFacts: noSidecar });
  assert.equal(v?.short, 'closing auction print');
  assert.equal(v?.tone, 'auction');
  assert.match(v?.text ?? '', /holds it as an ordinary one-second bar until it is re-imported \(D-3320 item 4\); it is not a continuous trade/);
  assert.match(v?.text ?? '', /an inference \(charter §9\)/);
  assert.match(v?.text ?? '', /equals the official day close \(\/sources\/day\.json: 0 bps\)/);
  const other = auctionVerdict({ feed: GDFL, segment: 'CASH', t: firstCasT, phase: 'cas', tail, dayFacts: noSidecar });
  assert.equal(other?.short, 'closing-auction session');
  assert.match(other?.text ?? '', /not the session's last \(that one is at 15:28:07\)/);
});

// SC-CLOSE-10: on 9 of 133 share-sessions the vendor dropped the auction's
// print, the session's last print was a 15:15:0x trade, and this label called
// it the auction's; its text claimed a zero bid and ask that 40 of 116 prints
// do not carry. The print is now named only where it equals the official
// close, and no book is claimed.
test('SC-CLOSE-10: a last print that does not equal the official close is never called the auction\'s', () => {
  const early = ist(CAS_DAY, '15:15:01') / 1000;
  const lone = { bars: [{ t: early, c: 50100, phase: 'cas' }] };
  const differs = { ...noSidecar, close: { official_vs_gdfl_last_print: { bps: -16.75, flag: true } } };
  const v = auctionVerdict({ feed: GDFL, segment: 'CASH', t: early, phase: 'cas', tail: lone, dayFacts: differs });
  assert.equal(v?.short, 'closing-auction session');
  assert.notEqual(v?.tone, 'auction');
  assert.match(v?.text ?? '', /\(15:15:01\) differs from the official day close by -16\.75 bps/);
  assert.match(v?.text ?? '', /NOT PROVED/);
  // Not compared: not called the print either.
  const unmeasured = { ...noSidecar, close: {} };
  const u = auctionVerdict({ feed: GDFL, segment: 'CASH', t: early, phase: 'cas', tail: lone, dayFacts: unmeasured });
  assert.equal(u?.short, 'closing-auction session');
  assert.match(u?.text ?? '', /the official close was not compared with it/);
  // No verdict ever claims the print's book.
  for (const d of [noSidecar, differs, unmeasured]) {
    for (const t of [lastT, firstCasT, early]) {
      const said = auctionVerdict({ feed: GDFL, segment: 'CASH', t, phase: 'cas', tail, dayFacts: d })?.text ?? '';
      assert.doesNotMatch(said, /zero bid and ask/);
    }
  }
  // Two prints of the session at the official close: which is the auction's
  // is said to be NOT PROVED.
  const twin = { bars: [{ t: lastT, c: 50000, phase: 'cas' }, { t: lastT - 2, c: 50000, phase: 'cas' }] };
  const w = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail: twin, dayFacts: noSidecar });
  assert.equal(w?.short, 'closing auction print');
  assert.match(w?.text ?? '', /1 other print\(s\) of the session carry the same price, and which of them is the auction's is NOT PROVED/);
});

test('a month that files the auction apart never calls a bar its print', () => {
  const apart = { day: CAS_DAY, auction: { state: 'kept_apart', prints: [{ at: '15:28:07', price: 50000, quantity: 1000 }] } };
  const v = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail, dayFacts: apart });
  assert.equal(v?.short, 'closing-auction session');
  assert.match(v?.text ?? '', /not the auction's print: this month files the auction's print apart \(15:28:07 at ₹500\.00 × 1,000\)/);
});

test('no auction verdict without the cas phase, and an unread record is said to be unread', () => {
  assert.equal(auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'continuous', tail, dayFacts: noSidecar }), null);
  assert.equal(auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: undefined }), null);
  const unread = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas' });
  assert.match(unread?.text ?? '', /which were not read/);
  const broken = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail, dayFacts: { auction: { state: 'unreadable', why: 'fixture fault' } } });
  assert.deepEqual([broken?.tone, broken?.short], ['warn', 'auction record unreadable']);
  assert.match(auctionVerdict({ feed: ZERODHA, segment: 'CASH', t: lastT, phase: 'cas' })?.text ?? '', /end at 15:15 by design/);
});

test('the auction print is the newest bar IN the session, never a later bar of another phase', () => {
  // An invented post-close bar newest in the tail is passed over.
  const late = { bars: [{ t: ist(CAS_DAY, '15:29:30') / 1000, c: 49990, phase: 'post_close' }, ...tail.bars] };
  const v = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail: late, dayFacts: noSidecar });
  assert.equal(v?.short, 'closing auction print');
  // A tail that was read and holds no cas bar says so, and never names a print.
  const none = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail: { bars: [] }, dayFacts: noSidecar });
  assert.equal(none?.short, 'closing-auction session');
  assert.match(none?.text ?? '', /hold none in the cas phase/);
});

test('only a CASH run below the day rung on or after the auction began reaches it', () => {
  const run = (/** @type {string} */ instrument, /** @type {string} */ day) => ({ instrument, day });
  assert.equal(reachesAuction(run('NSE-CASH-ACME', '2026-08-03'), GDFL), true);
  assert.equal(reachesAuction(run('NSE-CASH-ACME', '2026-07-31'), GDFL), false);
  assert.equal(reachesAuction(run('NSE-CASH-ACME', ''), GDFL), false);
  assert.equal(reachesAuction(run('NSE-INDEX-NIFTY', '2026-09-24'), GDFL), false);
  assert.equal(reachesAuction(run('NSE-FNO-ACME-2026-09-29-100000-CE', '2026-09-24'), GDFL), false);
  assert.equal(reachesAuction(run('NSE-CASH-ACME', '2026-09-24'), UNMEASURED), false);
});

test('a page reads the auction session of at most CAS_DAYS_MAX share-days, and counts the rest', () => {
  const days = Array.from({ length: CAS_DAYS_MAX + 3 }, (_, i) => `2026-09-${String(i + 1).padStart(2, '0')}`);
  // Two runs of one share-day (a page sorted by volume) are one read.
  const runs = [...days, days[0]].map((day) => ({ instrument: 'NSE-CASH-ACME', day }));
  const got = auctionDaysOnPage(runs, GDFL);
  assert.equal(got.read.size, CAS_DAYS_MAX);
  assert.equal(got.over, 3);
  assert.ok(got.read.has(`NSE-CASH-ACME|${days[0]}`));
  // Zerodha's minutes end before the auction by design: its rows' own phases are the whole answer.
  assert.deepEqual([auctionDaysOnPage(runs, ZERODHA).read.size, auctionDaysOnPage(runs, ZERODHA).over], [0, 0]);
});

test('a month file past the page cap says it was not read, never that it is being read', () => {
  const s = sourceNotRead(11, PROV_FILES_MAX);
  assert.equal(s.state, 'capped');
  assert.equal(s.text, `this page holds 11 month files and the source of at most ${PROV_FILES_MAX} is read per page; Details reads this row's`);
  assert.equal(s.pullDay, null);
});

test('a page whose rows carry no phase is marked from the day\'s tail, and only its auction rows', () => {
  const rows = [
    bar('NSE-CASH-ACME', '1s', CAS_DAY, '15:28:07'),
    bar('NSE-CASH-ACME', '1s', CAS_DAY, '15:15:00'),
    bar('NSE-CASH-ACME', '1s', CAS_DAY, '15:14:59')
  ];
  const marks = auctionMarks({ feed: GDFL, rows, tail, dayFacts: noSidecar });
  assert.deepEqual(marks.map((m) => [m.index, m.verdict.short]), [[0, 'closing auction print'], [1, 'closing-auction session']]);
  // A row that carries its own phase is believed over the tail.
  const own = auctionMarks({ feed: GDFL, rows: [bar('NSE-CASH-ACME', '1s', CAS_DAY, '15:28:07', { phase: 'continuous' })], tail, dayFacts: noSidecar });
  assert.equal(own.length, 0);
});

/* ── D-3334: where a bar came from ────────────────────────────────────── */

test('the URLs name exactly the file, the bar and the day asked about', () => {
  const at = { feed: 'gdfl', instrument: 'NSE-INDEX-NIFTY', timeframe: '1s', month: '2019-03' };
  assert.equal(provenanceUrl(at), '/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-03');
  assert.equal(provenanceUrl({ ...at, t: 1551757500 }), '/provenance.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month=2019-03&t=1551757500');
  const b = new URL(barAtUrl({ ...at, t: 1551757500 }), 'http://x').searchParams;
  assert.deepEqual([b.get('key'), b.get('at'), b.get('limit'), b.get('sort'), b.get('from'), b.get('to')], ['NSE-INDEX-NIFTY', '1551757500', '1', 'ts', '2019-03', '2019-03']);
  const c = new URL(casTailUrl({ ...at, instrument: 'NSE-CASH-ACME', month: '2026-09', day: CAS_DAY, from: '15:15' }), 'http://x').searchParams;
  assert.deepEqual([c.get('from_day'), c.get('to_day'), c.get('tod_from'), c.get('tod_to'), c.get('dir')], [CAS_DAY, CAS_DAY, '15:15:00', CAS_READ_TO, 'desc']);
  // The read stops where D-3320 item 2 ends the session; the phase decides.
  assert.equal(CAS_READ_TO, '15:29:59');
  assert.equal(dayFactsUrl('NSE-CASH-BAJAJ-AUTO', CAS_DAY), `/sources/day.json?exchange=NSE&segment=CASH&symbol=BAJAJ-AUTO&from=${CAS_DAY}&to=${CAS_DAY}`);
  assert.equal(dayFactsUrl('NSE-FNO-ACME-2026-09-29-100000-CE', CAS_DAY), null);
});

test('one line per month file and day, covering every row of the page once', () => {
  const rows = [
    bar('NSE-INDEX-NIFTY', '1s', '2019-03-05', '09:15:02'),
    bar('NSE-INDEX-NIFTY', '1s', '2019-03-05', '09:15:01'),
    bar('NSE-INDEX-NIFTY', '1s', '2019-03-01', '15:29:59'),
    bar('NSE-INDEX-NIFTY', '1s', '2019-02-28', '15:29:59')
  ];
  const runs = sourceRuns(rows);
  assert.deepEqual(runs.map((r) => [r.from, r.to, r.day, r.month]), [[0, 1, '2019-03-05', '2019-03'], [2, 2, '2019-03-01', '2019-03'], [3, 3, '2019-02-28', '2019-02']]);
  assert.equal(runs.reduce((a, r) => a + r.n, 0), rows.length);
  // A day rung is one line per month file: its source is the month's pull.
  const days = sourceRuns([bar('NSE-INDEX-NIFTY', '1day', '2019-05-31', '09:15:00'), bar('NSE-INDEX-NIFTY', '1day', '2019-05-30', '09:15:00')]);
  assert.deepEqual(days.map((r) => [r.from, r.to, r.day]), [[0, 1, '']]);
  const files = filesOnPage(rows);
  assert.deepEqual(files.files.map((f) => [f.month, f.n]), [['2019-03', 3], ['2019-02', 1]]);
  assert.equal(files.over, 0);
  const many = Array.from({ length: PROV_FILES_MAX + 2 }, (_, i) => bar(`NSE-CASH-S${i}`, '1s', '2019-03-05', '09:15:00'));
  assert.equal(filesOnPage(many).files.length, PROV_FILES_MAX);
  assert.equal(filesOnPage(many).over, 2);
});

test('on a page of several series merged by time, a contract\'s rows are one run though others sit between them', () => {
  const ce = 'NSE-FNO-ACME-2019-03-28-100000-CE';
  const pe = 'NSE-FNO-ACME-2019-03-28-100000-PE';
  const rows = [
    bar(ce, '1s', '2019-03-27', '15:29:59'),
    bar(pe, '1s', '2019-03-27', '15:29:59'),
    bar(ce, '1s', '2019-03-27', '15:29:58'),
    bar(pe, '1s', '2019-03-27', '15:29:57')
  ];
  const runs = sourceRuns(rows);
  assert.deepEqual(runs.map((r) => [r.instrument, r.rows]), [[ce, [0, 2]], [pe, [1, 3]]]);
  // Every row of the page is in exactly one run.
  assert.deepEqual(runs.flatMap((r) => r.rows).sort(), [0, 1, 2, 3]);
  // Two contracts imported by one journal line say the same thing: one line, every row.
  const folds = foldRuns(runs, (r) => r.day);
  assert.equal(folds.length, 1);
  assert.deepEqual(folds[0].rows, [0, 1, 2, 3]);
  assert.equal(folds[0].runs.length, 2);
  // Lines that say different things stay apart, in page order.
  assert.deepEqual(foldRuns(runs, (r) => r.instrument).map((f) => f.rows), [[0, 2], [1, 3]]);
});

test('row positions read as the 1-based ranges a reader counts', () => {
  assert.equal(rowsLabel([0]), 'row 1');
  assert.equal(rowsLabel([0, 1, 2, 6, 8, 9]), 'rows 1–3, 7, 9–10');
  assert.equal(rowsLabel([0, 2, 4, 6, 8, 10, 12, 14], 3), 'rows 1, 3, 5, and 5 more');
  assert.equal(rowsLabel([]), 'no row');
});

/* An invented GDFL month with no sidecar, rebuilt from the import journal. */
/**
 * @param {string} day
 * @param {number} line
 * @param {number} first
 * @param {number} end
 * @param {number | null} [run]
 */
const dayRow = (day, line, first, end, run = null) => ({
  attempts: 1,
  bars: { count: end - first, end, first_index: first },
  closes: 1,
  day,
  kind: 'indices',
  line,
  outcome: 'done',
  run,
  vendor_files: {
    day_folder: `FIXTURE_DAY_${day}`,
    stem: 'FIXTURE INDEX',
    tickstore_day: `cm/FIXTURE/${day}.bts`,
    zip_day: `FIXTURE/${day}.zip`
  }
});
const GDFL_MONTH = {
  feed: 'gdfl',
  instrument: 'NSE-INDEX-NIFTY',
  timeframe: '1s',
  month: '2019-03',
  held: true,
  bars: { path: 'bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin', n_valid: 300, generation: 2, modified: '2026-10-05 09:00:00 IST' },
  bar: null,
  sidecar: { path: 'bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.prv', state: 'absent', records: 0, why: 'fixture: written before D-3334' },
  recorded: [],
  reconstructed: true,
  label: 'reconstructed, member digest unknown',
  reconstruction: {
    source: 'imports/gdfl.journal',
    kind: 'indices',
    journal_days: 2,
    journal_days_done: 2,
    bars_on_no_journal_day: 0,
    archive: {
      recorded: false,
      runs: [{ id: 77, line: 900, fields: { source: 'zip', archive: '/fixture/archive%20dir/2019.zip', archive_bytes: '123456', commit: 'abcdef0123456789', archive_cd_blake3: '00112233445566778899' } }]
    },
    days: [dayRow('2019-03-01', 244, 0, 100), dayRow('2019-03-05', 246, 100, 300, 77)]
  },
  window: null,
  notes: []
};

test('DB-078/DB-G56: a GDFL day names its import line, and its archive only where a run recorded one', () => {
  const unrecorded = sourceOfDay(GDFL_MONTH, '2019-03-01');
  assert.equal(unrecorded.state, 'journal');
  assert.equal(unrecorded.short, 'reconstructed, member digest unknown');
  assert.match(unrecorded.text, /^import day 2019-03-01 · imports\/gdfl\.journal line 244 \(done\)/);
  assert.match(unrecorded.text, /archive not recorded \(its closing line names no run: written before D-3317\); this build would read FIXTURE_DAY_2019-03-01\/FIXTURE INDEX\.csv, in day zip FIXTURE\/2019-03-01\.zip or tick-store day cm\/FIXTURE\/2019-03-01\.bts/);
  const recorded = sourceOfDay(GDFL_MONTH, '2019-03-05');
  assert.match(recorded.text, /run 77 \(journal line 900\): source zip, archive \/fixture\/archive dir\/2019\.zip \(1,23,456 bytes, central directory BLAKE3 0011223344556677…\), build abcdef012345/);
  const none = sourceOfDay(GDFL_MONTH, '2019-03-07');
  assert.match(none.text, /names no indices import of 2019-03-07/);
  assert.match(sourceOfDay(GDFL_MONTH, '').text, /2 indices import day\(s\) in imports\/gdfl\.journal, 2 done; archive not recorded/);
});

test('a sidecar record wins over the reconstruction, and says what it recorded', () => {
  const rec = {
    ordinal: 0,
    source_kind: 'archive_member',
    source_name: 'FIXTURE_DAY/FIXTURE INDEX.csv',
    source_name_truncated: false,
    source_bytes: 4096,
    source_crc32: '0badf00d',
    day: '2019-03-05',
    journal_ordinal: 246,
    commit: 'fedcba9876543210',
    landed_at: '2026-10-11 10:00:00 IST',
    journal: { source: 'imports/gdfl.journal' }
  };
  const s = sourceOfDay({ ...GDFL_MONTH, recorded: [rec], label: 'recorded' }, '2019-03-05');
  assert.equal(s.state, 'recorded');
  assert.equal(s.text, recordText(rec));
  assert.equal(s.text, 'archive member FIXTURE_DAY/FIXTURE INDEX.csv (4,096 bytes, CRC-32 0badf00d) · day 2019-03-05 · imports/gdfl.journal record 246 · landed 2026-10-11 10:00:00 IST · build fedcba987654');
});

/* An invented Zerodha month, rebuilt from the pull journal. */
/**
 * @param {number} ordinal
 * @param {string} started
 * @param {string} [outcome]
 * @param {number} [failures]
 */
const candidate = (ordinal, started, outcome = 'STORED', failures = 0) => ({
  ordinal,
  started,
  outcome,
  failures,
  source: 'https://fixture.invalid/historical/',
  source_cut: false
});
/** @param {boolean} attributed @returns {any} */
const ZERODHA_MONTH = (attributed) => ({
  feed: 'zerodha',
  instrument: 'NSE-CASH-ACME',
  timeframe: '1day',
  month: '2019-05',
  held: true,
  bars: { path: 'bars/zerodha/NSE/CASH/ACME/1day/2019-05.bin', n_valid: 22, generation: 1, modified: '2026-10-06 11:33:50 IST' },
  sidecar: { path: 'bars/zerodha/NSE/CASH/ACME/1day/2019-05.prv', state: 'absent', records: 0 },
  recorded: [],
  label: 'reconstructed, member digest unknown',
  reconstruction: {
    source: 'audit/pull.journal',
    candidates: [candidate(2, '2026-10-06 11:33:42 IST', 'FAILED', 17), candidate(20, '2026-10-06 11:37:12 IST')],
    attributed: attributed ? { run: 2, basis: 'fixture' } : null,
    why: 'fixture: a pull record names its window, never its months'
  },
  window: { from: '2018-01-01', to: '2026-09-30', inside: true, label: null },
  notes: []
});

test('DB-078: a Zerodha month names the pull run that wrote it, and that run is its pull date', () => {
  const s = sourceOfDay(ZERODHA_MONTH(true), '');
  assert.equal(s.state, 'pull');
  assert.equal(s.pullDay, '2026-10-06');
  assert.match(s.text, /^pulled by run 2 \(audit\/pull\.journal record 2\), started 2026-10-06 11:33:42 IST, FAILED \(the run ended FAILED on 17 other request\(s\); this month was written inside it\)/);
  assert.match(s.text, /; named because fixture$/);
  // No writer named, but every candidate began on the day the file was last
  // written: that day is the pull date, and the line says why.
  const unnamed = sourceOfDay(ZERODHA_MONTH(false), '');
  assert.equal(unnamed.pullDay, '2026-10-06');
  assert.match(unnamed.text, /^2 pull run\(s\) could have written this month and none is named as its writer: fixture: a pull record names its window, never its months\. Candidates: run 2 \(2026-10-06 11:33:42 IST, FAILED\); run 20 \(2026-10-06 11:37:12 IST, STORED\)\./);
  assert.match(unnamed.text, /Every write fell on 2026-10-06: no candidate began before it and the file was last written on it, so that is the pull date\.$/);
  // A candidate that began the day before leaves the pull date open, and says so.
  const span = ZERODHA_MONTH(false);
  span.reconstruction.candidates.unshift(candidate(1, '2026-10-05 23:50:00 IST'));
  const open = sourceOfDay(span, '');
  assert.equal(open.pullDay, null);
  assert.match(open.text, /written between 2026-10-05 and 2026-10-06, so no one pull date is claimed\.$/);
  assert.equal(priceBadge(ZERODHA, 'CASH', '1day', open.pullDay).text, 'back-adjusted by Zerodha as of the pull date');
});

test('a refused or absent month is said, never shown as a source', () => {
  assert.equal(sourceOfDay({ error: 'fixture refusal' }, '').state, 'error');
  assert.match(sourceOfDay({ held: false, path: 'bars/x.bin' }, '').text, /no month file at bars\/x\.bin/);
  assert.equal(sourceOfDay(null, '').state, 'unread');
});

/* ── the drawer, on the verification's selections ─────────────────────── */

test('the drawer on a GDFL NIFTY 1s row names its import day and its archive', () => {
  const row = bar('NSE-INDEX-NIFTY', '1s', '2019-03-05', '09:15:00');
  const answer = { ...GDFL_MONTH, bar: { index: 100 }, t: row.ts / 1000, reconstruction: { ...GDFL_MONTH.reconstruction, days: [GDFL_MONTH.reconstruction.days[1]] } };
  const facts = drawerFacts({ feed: GDFL, row, answer, bar: { t: row.ts / 1000, o: 100000, h: 100000, l: 100000, c: 100000, v: 0, v_why: 'index_no_volume', phase: 'continuous' } });
  const get = (/** @type {string} */ k) => facts.find((f) => f.k === k)?.v ?? '';
  assert.match(get('Bar'), /NSE-INDEX-NIFTY · 1s · 05 Mar 2019 09:15:00 IST/);
  assert.match(get('As served'), /close ₹1,000\.00 · volume 0 \(index_no_volume\)/);
  assert.equal(get('Price basis'), 'as published, an index level');
  assert.equal(get('Close'), '1s: the close is the last print inside the bar');
  assert.equal(get('Session phase'), 'continuous trading');
  assert.match(get('Source'), /import day 2019-03-05 · imports\/gdfl\.journal line 246 \(done\) · run 77/);
  assert.match(get('In its file'), /bar 100 of 300 \(0-based\) in bars\/gdfl\/NSE\/INDEX\/NIFTY\/1s\/2019-03\.bin/);
  assert.match(get('Month file'), /generation 2/);
  assert.match(get('Sidecar'), /absent — fixture: written before D-3334/);
  assert.equal(facts.some((f) => f.k === 'Closing auction'), false);
  // An index bar has no strike, so no spot line.
  assert.equal(facts.some((f) => f.k === 'Spot'), false);
});

test('DB-G58: the drawer on an option bar says its spot is its own feed\'s, never another\'s', () => {
  const row = bar('NSE-FNO-NIFTY-2019-03-28-1150000-CE', '1s', '2019-03-27', '15:29:59');
  const facts = drawerFacts({ feed: GDFL, row, answer: null });
  assert.match(facts.find((f) => f.k === 'Spot')?.v ?? '', /never from another feed's: a GDFL strike is never priced against a Zerodha spot/);
  assert.equal(facts.find((f) => f.k === 'Price basis')?.v, 'raw, as traded');
});

test('the drawer on a Zerodha day bar says official close; on a Zerodha share, back-adjusted as of its pull date', () => {
  const index = drawerFacts({ feed: ZERODHA, row: bar('NSE-INDEX-NIFTY', '1day', '2019-05-30', '09:15:00'), answer: { ...ZERODHA_MONTH(true), instrument: 'NSE-INDEX-NIFTY' } });
  assert.equal(index.find((f) => f.k === 'Close')?.v, '1day close: the official close, not a fold of its minutes, not the last print');
  assert.equal(index.find((f) => f.k === 'Session phase')?.v, 'none: a day bar spans the session');
  // A day bar is stamped at its IST midnight and names no clock of its own.
  assert.match(index.find((f) => f.k === 'Bar')?.v ?? '', /· 1day · 30 May 2019, a day bar \(t=\d+\) · month file 2019-05$/);
  const share = drawerFacts({ feed: ZERODHA, row: bar('NSE-CASH-ACME', '1day', '2019-05-30', '09:15:00'), answer: ZERODHA_MONTH(true) });
  const price = share.find((f) => f.k === 'Price basis');
  assert.deepEqual([price?.v, price?.tone], ['back-adjusted by Zerodha as of the pull date 2026-10-06', 'adjusted']);
  assert.match(share.find((f) => f.k === 'Volume basis')?.v ?? '', /nothing on this page derives turnover or VWAP/);
  // A month outside the operator's window says so (D-3338).
  const outside = drawerFacts({ feed: ZERODHA, row: bar('NSE-INDEX-NIFTY', '1day', '2016-05-30', '09:15:00'), answer: { ...ZERODHA_MONTH(true), window: { from: '2018-01-01', to: '2026-09-30', inside: false, label: 'outside your window' } } });
  assert.equal(outside.find((f) => f.k === 'Window')?.v, 'outside your window (2018-01-01 to 2026-09-30)');
});

test('the drawer on a GDFL share at the auction\'s second labels it the closing auction print', () => {
  const row = bar('NSE-CASH-ACME', '1s', CAS_DAY, '15:28:07');
  const facts = drawerFacts({
    feed: GDFL,
    row,
    answer: null,
    bar: { t: lastT, o: 50000, h: 50000, l: 50000, c: 50000, v: 1000, v_why: null, phase: 'cas' },
    tail,
    dayFacts: noSidecar
  });
  assert.equal(facts.find((f) => f.k === 'Session phase')?.v, 'closing-auction session');
  const auction = facts.find((f) => f.k === 'Closing auction');
  assert.equal(auction?.tone, 'auction');
  assert.match(auction?.v ?? '', /^closing auction print: the closing-auction session's last print/);
  assert.equal(facts.find((f) => f.k === 'Price basis')?.v, 'raw, as traded');
  assert.match(facts.find((f) => f.k === 'Source')?.v ?? '', /source not read/);
});

test('one page and its drawer fit in the answer cache, so re-asking a missing answer settles', () => {
  // A page reads at most a month file each up to the cap, and two answers per
  // closing-auction day; the drawer at most four. The page re-asks any of
  // its answers the cache lets go, which settles only if they all fit.
  assert.ok(PROV_FILES_MAX + 2 * CAS_DAYS_MAX + 4 < PROV_CACHE_MAX);
});

test('the answer cache keeps the newest entries and drops the oldest first', () => {
  let m = new Map();
  for (let i = 0; i < 5; i++) m = remember(m, `u${i}`, i, 3);
  assert.deepEqual([...m.keys()], ['u2', 'u3', 'u4']);
  m = remember(m, 'u2', 9, 3);
  assert.deepEqual([...m.entries()], [['u3', 3], ['u4', 4], ['u2', 9]]);
});

/* ── the components are mounted and read only these helpers ───────────── */

test('DB-A40: the drawer and the badge are no longer placeholders that render nothing', () => {
  const prov = readFileSync(new URL('../src/routes/db/DbProvenance.svelte', import.meta.url), 'utf8');
  const badge = readFileSync(new URL('../src/routes/db/DbBasisBadge.svelte', import.meta.url), 'utf8');
  assert.doesNotMatch(prov, /A PLACEHOLDER/);
  assert.doesNotMatch(badge, /A PLACEHOLDER/);
  for (const name of ['provenanceUrl', 'sourceRuns', 'sourceOfDay', 'drawerFacts', 'auctionMarks']) assert.match(prov, new RegExp(`\\b${name}\\(`));
  for (const name of ['priceBadge', 'volumeBadge', 'closeBadge', 'auctionNote', 'spotNote']) assert.match(badge, new RegExp(`\\b${name}\\(`));
});

/* ── R3-13 goal 4: a Zerodha share's adjusted or quantised run, by its days ── */

/** One `/sources/day.json` day with an invented `basis` (`sources.rs` serves it under that key). */
const basisDay = (/** @type {string} */ day, /** @type {Record<string, any> | null} */ b) => ({ day, basis: b });
/** A day `/sources/day.json` names and neither feed measured: a weekend, a holiday. */
const unmeasured = (/** @type {string} */ day) => basisDay(day, { factor: null, basis_differs: false, quantised: false, grid_paisa: { zerodha_minutes: null, gdfl_seconds: null } });

test('R3-13 goal 4: a share run whose prices are adjusted or quantised is named with its first and last day', () => {
  assert.equal(
    shareBasisUrl('NSE-CASH-ACME', '2019-06-01', '2019-06-30'),
    '/sources/day.json?exchange=NSE&segment=CASH&symbol=ACME&from=2019-06-01&to=2019-06-30'
  );
  assert.equal(shareBasisUrl('NSE-INDEX-NIFTY', '2019-06-01', '2019-06-30'), null);
  assert.equal(shareBasisUrl('NSE-CASH-ACME', '', '2019-06-30'), null);
  const body = {
    days: [
      basisDay('2019-06-05', { basis_differs: true, quantised: false, factor: 4.1, grid_paisa: { zerodha_minutes: 5, gdfl_seconds: 5 } }),
      basisDay('2019-06-03', { basis_differs: true, quantised: false, factor: 4, grid_paisa: { zerodha_minutes: 5, gdfl_seconds: 5 } }),
      unmeasured('2019-06-01'), // a Saturday before the first measured day: the run's edge stays open
      unmeasured('2019-06-04'), // inside a run: joins and breaks nothing
      basisDay('2019-06-02', null),
      unmeasured('2019-06-08'), // a weekend inside the second run
      unmeasured('2019-06-09'),
      basisDay('2019-06-06', { basis_differs: false, quantised: false, factor: 1 }),
      basisDay('2019-06-07', { basis_differs: false, quantised: true, factor: 1.0254, grid_paisa: { zerodha_minutes: 50, gdfl_seconds: 5 } }),
      basisDay('2019-06-10', { basis_differs: true, quantised: true, factor: 1.0254, grid_paisa: { zerodha_minutes: 50, gdfl_seconds: 5 } }),
      unmeasured('2019-06-15') // after the last measured day: the edge stays open
    ]
  };
  const runs = basisRunsOf(body);
  assert.deepEqual(runs.map((r) => [r.from, r.to, r.days, r.adjusted, r.quantised]), [
    ['2019-06-03', '2019-06-05', 2, true, false],
    ['2019-06-07', '2019-06-10', 2, true, true]
  ]);
  assert.deepEqual(runs[0].factors, [4, 4.1]);
  assert.deepEqual(runs[0].open, { before: true, after: false });
  assert.deepEqual(runs[1].open, { before: false, after: true });
  const first = basisRunLabel(runs, '2019-06-04');
  assert.equal(first?.short, 'adjusted run 2019-06-03 → 2019-06-05');
  assert.match(first?.text ?? '', /2019-06-03 to 2019-06-05, 2 day\(s\) measured, factor ×4\.0000 to ×4\.1000 \(it drifts\) \(the run may go on before the days read\)/);
  assert.match(first?.text ?? '', /D-3323's to decide/);
  const second = basisRunLabel(runs, '2019-06-10');
  assert.equal(second?.short, 'adjusted and quantised run 2019-06-07 → 2019-06-10');
  assert.match(second?.text ?? '', /factor ×1\.0254; Zerodha's prices sit on a 50-paisa grid against GDFL's raw 5-paisa grid/);
  assert.equal(basisRunLabel(runs, '2019-06-06'), null, 'a day on the raw basis carries none');
  assert.equal(basisRunLabel(runs, ''), null);
  assert.deepEqual(basisRunsOf(null), []);
  // The key the route serves, not a name of its own: a day under any other key is not read.
  assert.deepEqual(basisRunsOf({ days: [{ day: '2019-07-01', share_basis: { basis_differs: true, factor: 4 } }] }), []);
  // A day with no factor served is not measured, whatever else it says.
  assert.deepEqual(basisRunsOf({ days: [basisDay('2019-07-01', { quantised: true, basis_differs: true, factor: null })] }), []);
  // One measured day alone is a run open both ways.
  const lone = basisRunsOf({ days: [unmeasured('2019-07-01'), basisDay('2019-07-02', { quantised: true, factor: 1.0254, grid_paisa: { zerodha_minutes: 50, gdfl_seconds: null } }), unmeasured('2019-07-03')] });
  assert.match(basisRunLabel(lone, '2019-07-02')?.text ?? '', /quantised against GDFL's raw prints: 2019-07-02 to 2019-07-02, 1 day\(s\) measured, factor ×1\.0254; Zerodha's prices sit on a 50-paisa grid, coarser than an exact adjustment leaves \(the run may go on both ways the days read\)/);
  assert.equal(basisRunLabel(lone, '2019-07-03'), null, 'an unmeasured day past the run is not in it');
  // The drawer names the run of its row's day.
  const row = bar('NSE-CASH-ACME', '1min', '2019-06-04', '10:00:00');
  const facts = drawerFacts({ feed: ZERODHA, row, answer: null, basis: runs });
  assert.match(facts.find((f) => f.k === 'Adjusted run')?.v ?? '', /^Zerodha's prices on this run are adjusted against GDFL's raw prints: 2019-06-03 to 2019-06-05/);
  // The source line and the badge say where the run is named.
  const prov = readFileSync(new URL('../src/routes/db/DbProvenance.svelte', import.meta.url), 'utf8');
  assert.match(prov, /\{#if l\.basis\}<span class="bb \{l\.basis\.tone\}" title=\{l\.basis\.text\}>\{l\.basis\.short\}<\/span>\{\/if\}/);
  const badge = readFileSync(new URL('../src/routes/db/DbBasisBadge.svelte', import.meta.url), 'utf8');
  assert.match(badge, /adjusted or quantised run is named, with its first and last day/);
});

/* ── R3-13 goal 3: the bar definition that built a row's day ─────────────── */

test('R3-13 goal 3: an option day names its bar definition on its rows\' label and in the drawer', () => {
  const answer = {
    reconstruction: {
      kind: 'options',
      source: 'imports/gdfl.journal',
      days: [
        { day: '2019-03-05', line: 246, outcome: 'done', attempts: 1, definition: 2, run: null, vendor_files: null },
        { day: '2019-03-06', line: 247, outcome: 'done', attempts: 1, definition: 4, run: null, recounted: true, vendor_files: null },
        { day: '2019-03-07', line: 248, outcome: 'done', attempts: 1, definition: null, run: null, vendor_files: null }
      ],
      archive: { runs: [] }
    }
  };
  assert.match(sourceOfDay(answer, '2019-03-05').text, / · bar definition 2$/);
  assert.doesNotMatch(sourceOfDay(answer, '2019-03-07').text, /bar definition/);
  const old = { definition: { value: 2, current: false, thisBuild: 4, means: 'fixture means' } };
  assert.deepEqual(definitionFact(answer, '2019-03-05', old), {
    k: 'Bar definition',
    v: "definition 2: fixture means (from the import journal's line 246); this build builds definition 4, so the day is labelled until it is restated",
    tone: 'warn'
  });
  assert.equal(
    definitionFact(answer, '2019-03-06', { definition: { value: 4, current: true, thisBuild: 4, means: '' } })?.v,
    "definition 4 (from the import journal's line 247, closed by the census recount); this build builds the same bars"
  );
  assert.match(definitionFact(answer, '2019-03-05')?.v ?? '', /whether this build builds the same bars is the day band's to say, and it was not read/);
  // No journal definition: the band's, where it has one.
  assert.match(definitionFact(answer, '2019-03-07', old)?.v ?? '', /^definition 2: fixture means \(from the day band\)/);
  assert.equal(definitionFact(answer, '2019-03-07'), null);
  assert.equal(definitionFact(null, '2019-03-05'), null);
  // A band that names another definition than the journal's is not borrowed.
  assert.match(definitionFact(answer, '2019-03-05', { definition: { value: 3, current: true, thisBuild: 4, means: '' } })?.v ?? '', /was not read for this day/);
  const row = bar('NSE-FNO-ACME-2019-03-28-100000-CE', '1s', '2019-03-05', '10:00:00');
  const facts = drawerFacts({ feed: GDFL, row, answer, band: old });
  assert.equal(facts.find((f) => f.k === 'Bar definition')?.tone, 'warn');
});

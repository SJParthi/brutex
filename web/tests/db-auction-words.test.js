// THE CLOSING AUCTION'S WORDS FOLLOW THE DAY'S RECORD (AH-03,
// g-ui-dom-vs-json-3; D-3320 items 3-4).
//
// `node --test web/tests/` — the band's auction sentence, the bar's
// closing-auction verdict and the extremes panel's flag, over `/feeds.json`
// and `/sources/day.json` shapes (`crates/api/src/feeds_route.rs`,
// `crates/api/src/sources.rs`). Every key, price and count below is invented
// (a share named FIXTURESHARE at prices no real share printed); the SHAPES are
// the routes'.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { auctionVerdict, auctionBandSentence, extremeAuctionFlag } from '../src/lib/db/notes-labels.js';

const GDFL = {
  wire: 'gdfl',
  display: 'GDFL',
  closing_auction: {
    from: '2026-08-03',
    eligible_continuous_close: '15:15',
    handling: 'print_kept_apart',
    label: 'fixture label',
    source: 'fixture source'
  }
};
const ZERODHA = {
  wire: 'zerodha',
  display: 'Zerodha',
  closing_auction: { from: '2026-08-03', eligible_continuous_close: '15:15', handling: 'minutes_end_before_auction', label: 'fixture', source: 'fixture' }
};

/** An IST instant as epoch seconds. @param {string} day @param {string} clock */
const ist = (day, clock) => Date.parse(`${day}T${clock}+05:30`) / 1000;

const DAY = '2026-09-28';
const KEY = 'NSE-CASH-FIXTURESHARE';
const lastT = ist(DAY, '15:29:19');
const earlierT = ist(DAY, '15:16:02');
const tail = { bars: [{ t: lastT, c: 777700, phase: 'cas' }, { t: earlierT, c: 777710, phase: 'cas' }] };

/** A `/sources/day.json` day: its auction record and the official-close comparison. */
function facts(/** @type {any} */ auction, bps = 0) {
  return { day: DAY, auction, close: { official_vs_gdfl_last_print: { of: 777700, against: 777700, bps, flag: false } } };
}

test('the_band_sentence_follows_the_days_auction_record', () => {
  // UNREAD: the run's sentence never claims the print is kept apart.
  const unread = auctionBandSentence(GDFL, null, { sessions: 41 });
  assert.equal(unread.state, 'unread');
  assert.match(unread.text, /^41 session\(s\) of this share are closing-auction days: an eligible share trades continuously until 15:15/);
  assert.match(unread.text, /that day's own auction record \(\/sources\/day\.json\), not this run's: open a day to read it/);
  assert.doesNotMatch(unread.text, /is kept apart from trades \(/);

  // KEPT APART WITH PRINTS: names them.
  const kept = auctionBandSentence(GDFL, { state: 'kept_apart', prints: [{ at: '15:29:19', price: 777700, quantity: 4321 }] }, { day: DAY });
  assert.equal(kept.state, 'kept_apart');
  assert.match(kept.text, /^On 28 Sep 2026 the auction's print is kept apart from trades, never in a one-second bar: 15:29:19 at ₹7,777\.00 × 4,321 \(D-3320 item 3\)\.$/);

  // KEPT APART, NONE ON THIS DAY: says none is filed, never "kept apart".
  const none = auctionBandSentence(GDFL, { state: 'kept_apart', prints: [] }, { day: DAY });
  assert.equal(none.state, 'kept_apart_none');
  assert.match(none.text, /none is filed for this day: if the closing-auction session's last print is the auction's, it sits among the one-second bars/);

  // NO SIDECAR: the print is an ordinary one-second bar until re-imported.
  const old = auctionBandSentence(GDFL, { state: 'no_sidecar', says: 'fixture' }, { day: '2026-08-03' });
  assert.equal(old.state, 'no_sidecar');
  assert.match(old.text, /^On 03 Aug 2026 the auction's print is not kept apart: .* held as an ordinary one-second bar until the month is re-imported \(D-3320 item 4\)/);

  // UNREADABLE: says so and claims nothing.
  const broken = auctionBandSentence(GDFL, { state: 'unreadable', why: 'fixture: torn sidecar' }, { day: DAY });
  assert.equal(broken.state, 'unreadable');
  assert.match(broken.text, /the auction record would not read \(fixture: torn sidecar\), so whether the auction's print is kept apart is not known/);

  // A STATE THIS PAGE DOES NOT KNOW is named, never read as one it does.
  assert.match(auctionBandSentence(GDFL, { state: 'brand_new' }, { day: DAY }).text, /state "brand_new" is not one this page knows/);
});

test('the band names an unchecked sidecar and a print also stored as a bar, when the server says so (AG-04 fields)', () => {
  const s = auctionBandSentence(
    GDFL,
    { state: 'kept_apart', version: 1, checked: false, unchecked_why: 'fixture: version 1 carries no checksum', prints: [{ at: '15:29:19', price: 777700, quantity: 10, also_in_bars: true }] },
    { day: DAY }
  );
  assert.match(s.text, /One is also stored as a one-second bar at 15:29:19: counted twice in any sum of the two \(bars-1\)\./);
  assert.match(s.text, /The sidecar is not checksummed \(fixture: version 1 carries no checksum\)/);
});

test('a_kept_apart_day_without_a_print_never_denies_the_auction_print', () => {
  const empty = { state: 'kept_apart', prints: [] };
  // The session's last print at the official close: possibly the auction's.
  const last = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail, dayFacts: facts(empty, 0) });
  assert.equal(last?.short, 'possibly the auction print');
  assert.match(last?.text ?? '', /^possibly the auction print, not filed apart for this day/);
  assert.match(last?.text ?? '', /equals the official day close \(\/sources\/day\.json: 0 bps\)/);
  // An earlier bar of the session: undecided, never "not the auction's print".
  const earlier = auctionVerdict({ feed: GDFL, segment: 'CASH', t: earlierT, phase: 'cas', tail, dayFacts: facts(empty, 0) });
  assert.match(earlier?.text ?? '', /none is filed for this day, so whether this bar is the auction's print is not decided here \(the session's last print is at 15:29:19/);
  // The last print but away from the official close: undecided, with the distance.
  const off = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail, dayFacts: facts(empty, 12) });
  assert.match(off?.text ?? '', /it is the session's last print, and it is 12 bps from the official day close/);
  // No tail read: says so.
  const blind = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', dayFacts: facts(empty, 0) });
  assert.match(blind?.text ?? '', /the session's last print was not read/);
  for (const v of [last, earlier, off, blind]) assert.doesNotMatch(v?.text ?? '', /not the auction's print/);
});

test('a filed print also stored as this second is the auction print twice, never "not the auction\'s print"', () => {
  const doubled = { state: 'kept_apart', prints: [{ at: '15:29:19', price: 777700, quantity: 10, also_in_bars: true }] };
  const v = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail, dayFacts: facts(doubled) });
  assert.equal(v?.short, 'auction print, also a bar');
  assert.match(v?.text ?? '', /filed apart AND also stored as this one-second bar: counted twice/);
  // The same record, the other bar: an ordinary session bar, as before.
  const other = auctionVerdict({ feed: GDFL, segment: 'CASH', t: earlierT, phase: 'cas', tail, dayFacts: facts(doubled) });
  assert.match(other?.text ?? '', /not the auction's print: this month files the auction's print apart \(15:29:19 at ₹7,777\.00 × 10\)/);
});

test('a no_sidecar day whose last cas bar is not at the official close still names the last print, and says the comparison is not 0 bps', () => {
  const v = auctionVerdict({ feed: GDFL, segment: 'CASH', t: lastT, phase: 'cas', tail, dayFacts: facts({ state: 'no_sidecar' }, 9) });
  assert.equal(v?.short, 'closing auction print');
  assert.doesNotMatch(v?.text ?? '', /as it does on this day/);
});

test('an_extreme_in_the_closing_auction_session_is_flagged', () => {
  const at = (/** @type {string} */ day, /** @type {string} */ clock) => ({ v: 1, t: ist(day, clock) });
  // Inside [15:15, 15:30) on an auction day, a share, a second rung: flagged.
  const f = extremeAuctionFlag(at(DAY, '15:29:19'), GDFL, KEY, 1);
  assert.equal(f?.short, 'closing-auction session');
  assert.match(f?.text ?? '', /closing-auction session's window \(15:15–15:30, D-3320 item 2\)/);
  assert.match(f?.text ?? '', /Its own phase decides/);
  // Both edges: 15:15:00 is in, 15:14:59 and 15:30:00 are out.
  assert.ok(extremeAuctionFlag(at(DAY, '15:15:00'), GDFL, KEY, 1));
  assert.equal(extremeAuctionFlag(at(DAY, '15:14:59'), GDFL, KEY, 1), null);
  assert.equal(extremeAuctionFlag(at(DAY, '15:30:00'), GDFL, KEY, 1), null);
  // A minute bar opening at 15:15 is in it; a day bar never is.
  assert.ok(extremeAuctionFlag(at(DAY, '15:15:00'), GDFL, KEY, 60));
  assert.equal(extremeAuctionFlag(at(DAY, '00:00:00'), GDFL, KEY, 86_400), null);
  // Before the auction began, an index, a contract, Zerodha, no feed note: never.
  assert.equal(extremeAuctionFlag(at('2026-07-31', '15:29:19'), GDFL, KEY, 1), null);
  assert.equal(extremeAuctionFlag(at(DAY, '15:29:19'), GDFL, 'NSE-INDEX-NIFTY', 1), null);
  assert.equal(extremeAuctionFlag(at(DAY, '15:29:19'), GDFL, 'NSE-FNO-FIXTURESHARE-2026-09-29-100000-CE', 1), null);
  assert.equal(extremeAuctionFlag(at(DAY, '15:29:19'), ZERODHA, KEY, 60), null);
  assert.equal(extremeAuctionFlag(at(DAY, '15:29:19'), { closing_auction: null }, KEY, 1), null);
  // Garbage: no instant, no key, an unparsable cut.
  assert.equal(extremeAuctionFlag({ v: 1, t: NaN }, GDFL, KEY, 1), null);
  assert.equal(extremeAuctionFlag(at(DAY, '15:29:19'), GDFL, null, 1), null);
  assert.equal(extremeAuctionFlag(at(DAY, '15:29:19'), { closing_auction: { ...GDFL.closing_auction, eligible_continuous_close: 'soon' } }, KEY, 1), null);
});

test('a day record read with no auction claims nothing, and a feed whose minutes end before the auction says so', () => {
  // READ, `auction: null` (not a share, or before the auction runs).
  const na = auctionBandSentence(GDFL, null, { day: DAY, read: true });
  assert.equal(na.state, 'not_applicable');
  assert.match(na.text, /^On 28 Sep 2026 the day's record carries no auction/);
  assert.doesNotMatch(na.text, /is kept apart from trades/);
  // ZERODHA: its minutes end before the auction; no day record is read.
  const z = auctionBandSentence(ZERODHA, null, { sessions: 3 });
  assert.equal(z.state, 'minutes_end');
  assert.match(z.text, /^3 session\(s\) of this share are closing-auction days: .*This feed's minutes end before the auction \(fixture\), so no auction print is among its bars\.$/);
  // An unread GDFL run is still the run sentence, never the Zerodha one.
  assert.equal(auctionBandSentence(GDFL, null, { sessions: 3 }).state, 'unread');
});

test('the day band speaks the auction sentence and counts sessions from the calendar (DbDayFlags.svelte wiring)', async () => {
  const { readFileSync } = await import('node:fs');
  const src = readFileSync(new URL('../src/routes/db/DbDayFlags.svelte', import.meta.url), 'utf8');
  // The unconditional run sentence is gone.
  assert.doesNotMatch(src, /the auction print is kept apart from trades \(charter/);
  assert.match(src, /auctionBandSentence\(feed, null, \{ sessions: casAll \}\)/);
  assert.match(src, /auctionBandSentence\(feed, got\.value\.auction, \{ day: got\.day, read: true \}\)/);
  // The session count is the calendar's, never the lead kind's.
  assert.match(src, /const sessions = \$derived\(sessionCount\(db\.dayBandCells\)\)/);
  assert.doesNotMatch(src, /c\.kind !== 'closed' && c\.kind !== 'outside'/);
  // The stored-life-cut note reads the supersession before blaming a lag.
  const sel = readFileSync(new URL('../src/lib/db/selection.svelte.js', import.meta.url), 'utf8');
  const life = sel.slice(sel.indexOf('const lifeNote = $derived.by'), sel.indexOf("This contract's stored life is cut"));
  assert.match(life, /lifeCutBySupersession\(supersessionOf\(db\.barState\?\.answer\)\)/);
  assert.ok(life.indexOf('lifeCutBySupersession(') < life.indexOf('it may not have traded after that day'), 'the supersession is read before the lag wording');
});

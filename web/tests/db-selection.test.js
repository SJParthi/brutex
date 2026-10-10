// /db's SELECTION MODEL (WP-50; D-3337, D-3338), DRIVEN.
//
// Every fixture row below is INVENTED: names follow the store's key grammar
// (`crates/core/src/instrument.rs`) and prices are absent or round numbers no
// vendor printed. Nothing here is a vendor row.
//
// The pure rules live in `$lib/db/selection-model.js`; the seeds are the
// effects in `selection.svelte.js`, run through the seam fixture exactly as
// the page runs them.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  DEFAULT_INSTRUMENT,
  SEGMENT_KEYS,
  segmentOf,
  contractRungsOf,
  foldInstruments,
  defaultInstrument,
  defaultSegment,
  heldSpan,
  eraOf,
  outsideWindow,
  calendarOf,
  dayCode,
  epochDayOf,
  isoOfEpoch,
  sessionGrid,
  timeBound,
  rebound,
  SECONDS,
  weekdayOf,
  expiryTags,
  keepOffered,
  linkOfKey
} from '../src/lib/db/selection-model.js';
import { SEG_VIEW } from '../src/lib/db/notes-controls.js';
import { databasePage, dbSource } from './database-page-fixture.js';

/** An invented decorated census row. @param {string} instrument @param {object} [more] */
function row(instrument, more = {}) {
  const tail = instrument.match(/-(\d{4}-\d{2}-\d{2})-(?:(\d+)-(CE|PE)|FUT)$/);
  const parts = instrument.split('-');
  return {
    instrument,
    underlying: parts[2],
    expiry: tail ? tail[1] : null,
    strike: tail && tail[2] ? Number(tail[2]) : null,
    side: tail && tail[3] ? tail[3] : null,
    segRung: tail ? (tail[3] ? 'option' : 'future') : 'spot',
    kind: parts[1],
    month: '2019-03',
    timeframe: '1s',
    rows: 100,
    short: 0,
    firstAt: null,
    lastAt: null,
    ...more
  };
}

/* ── DB-001: a picked instrument is one key, matched by equality ─────────── */

test('THE REGRESSION: NIFTY does not bring NIFTY100, NIFTYIT or NIFTYCPSE, and LT not LTF or LTM', () => {
  const rows = [
    row('NSE-INDEX-NIFTY'),
    row('NSE-INDEX-NIFTY100'),
    row('NSE-FNO-NIFTY-2019-03-28-1150000-CE'),
    row('NSE-FNO-NIFTYIT-2019-03-28-1540000-CE'),
    row('NSE-FNO-NIFTYCPSE-2019-03-28-300000-PE'),
    row('NSE-CASH-LT'),
    row('NSE-CASH-LTF'),
    row('NSE-CASH-LTM')
  ];
  const by = foldInstruments(rows);
  assert.deepEqual(
    by.get('NIFTY')?.rows.map((r) => r.instrument),
    ['NSE-INDEX-NIFTY', 'NSE-FNO-NIFTY-2019-03-28-1150000-CE']
  );
  assert.deepEqual(by.get('LT')?.rows.map((r) => r.instrument), ['NSE-CASH-LT']);
  assert.equal(by.get('NIFTY')?.spotMonths, 1);
  assert.equal(by.get('NIFTY')?.contractMonths, 1);
});

test('the shortfall of an instrument counts spot months only, never contract-months', () => {
  const by = foldInstruments([
    row('NSE-INDEX-NIFTY', { short: 7 }),
    row('NSE-FNO-NIFTY-2019-03-28-1150000-CE', { short: 400_000 })
  ]);
  assert.equal(by.get('NIFTY')?.short, 7);
  assert.equal(by.get('NIFTY')?.months, 2);
});

test('the page matches the picked instrument through the index, and every rung reads that set', () => {
  const src = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(src, /const instrumentIndex = \$derived\(foldInstruments\(universed\)\)/);
  assert.match(src, /picked \? \(instrumentIndex\.get\(picked\)\?\.rows \?\? \[\]\)/);
  // No rung of the cascade reads the prefix match any more.
  assert.doesNotMatch(src, /db\.textMatched/);
  assert.match(src, /const segmented = \$derived\(kind \? instrumentMatched\.filter/);
});

/* ── D-3337 item 2: the opening selection ─────────────────────────────── */

test('the page opens on NIFTY where the feed holds it, never on the alphabet', () => {
  const offered = [
    { key: '360ONE', months: 45 },
    { key: 'BANKNIFTY', months: 97 },
    { key: 'NIFTY', months: 97 }
  ];
  assert.equal(DEFAULT_INSTRUMENT, 'NIFTY');
  assert.equal(defaultInstrument(offered), 'NIFTY');
  // A feed whose store holds no NIFTY opens on the first name with rows.
  assert.equal(defaultInstrument([{ key: 'A', months: 0 }, { key: 'B', months: 3 }, { key: 'NIFTY', months: 0 }]), 'B');
  assert.equal(defaultInstrument([]), null);
});

test('the instrument SEED effect itself prefers NIFTY, and keeps a pick', () => {
  const page = databasePage();
  const offered = [{ key: '360ONE', months: 45 }, { key: 'NIFTY', months: 97 }];
  assert.equal(page.seedInstrument('', [{}], offered), 'NIFTY');
  assert.equal(page.seedInstrument('LT', [{}], offered), 'LT');
  assert.equal(page.seedInstrument('', [{}], [{ key: '360ONE', months: 45 }]), '360ONE');
});

test('the segment SEED prefers Spot, and an option-only underlying opens on what it holds', () => {
  const page = databasePage();
  const spotHeld = [{ key: 'spot' }, { key: 'futures', why: 'x' }, { key: 'options' }];
  assert.equal(page.seedSegment('', [{}], 'NIFTY', spotHeld), 'spot');
  assert.equal(defaultSegment(spotHeld), 'spot');
  const optionsOnly = [{ key: 'spot', why: 'x' }, { key: 'futures', why: 'x' }, { key: 'options' }];
  assert.equal(page.seedSegment('', [{}], 'ACC', optionsOnly), 'options');
  assert.equal(defaultSegment(optionsOnly), 'options');
});

test('the timeframe SEED takes the finest held rung', () => {
  const page = databasePage();
  assert.equal(page.seedTimeframe('', 'NIFTY', 'spot', [['1min', 105], ['5min', 105], ['1day', 141]]), '1min');
  assert.equal(page.seedTimeframe('', 'NIFTY', 'spot', [['1s', 97]]), '1s');
});

/* ── DB-G67, DB-050: segments against today and the archive end ──────── */

test('THE REGRESSION: a contract expiring today or later is not "already settled", and one cut by the archive end is apart', () => {
  const today = '2026-10-10';
  const end = '2026-09-24';
  assert.equal(segmentOf(row('NSE-INDEX-NIFTY'), today, end), 'spot');
  assert.equal(segmentOf(row('NSE-FNO-NIFTY-2019-03-28-1150000-CE'), today, end), 'options');
  assert.equal(segmentOf(row('NSE-FNO-NIFTY-2026-09-24-2300000-PE'), today, end), 'options');
  assert.equal(segmentOf(row('NSE-FNO-NIFTY-2026-09-29-2300000-PE'), today, end), 'options-cut');
  assert.equal(segmentOf(row('NSE-FNO-NIFTY-2026-10-10-2300000-CE'), today, end), 'options-live');
  assert.equal(segmentOf(row('NSE-FNO-NIFTY-2026-10-13-FUT'), today, end), 'futures-live');
  assert.equal(segmentOf(row('NSE-FNO-NIFTY-2026-09-29-FUT'), today, end), 'futures-cut');
  // A feed with no archive (Zerodha) splits on today only.
  assert.equal(segmentOf(row('NSE-FNO-NIFTY-2026-09-29-FUT'), today, null), 'futures');
});

test('every segment key has a row, and no segment note claims a 503 or a missing transport', () => {
  assert.deepEqual(SEG_VIEW.map((s) => s.key), SEGMENT_KEYS);
  for (const s of SEG_VIEW) assert.doesNotMatch(s.note, /503|no transport/);
  assert.deepEqual(SEG_VIEW.filter((s) => s.always).map((s) => s.key), ['spot', 'futures', 'options']);
});

test('a segment has the contract rungs its contracts have, cut and unexpired alike', () => {
  assert.deepEqual(contractRungsOf('spot'), { expiry: false, strike: false, money: false, side: false });
  assert.deepEqual(contractRungsOf('futures-live'), { expiry: true, strike: false, money: false, side: false });
  assert.deepEqual(contractRungsOf('options-cut'), { expiry: true, strike: true, money: true, side: true });
});

/* ── DB-019, DB-020: what a change drops ─────────────────────────────── */

test('THE REGRESSION: a GDFL option pick does not survive onto a feed that holds no contract', () => {
  const kept = keepOffered(
    { expiry: '2019-03-28', strikes: new Set(['1150000']), side: 'CE', money: new Set(['ATM']) },
    { rungs: contractRungsOf('spot'), expiries: [], strikes: [], sides: [] }
  );
  assert.deepEqual(kept, { expiry: '', strikes: new Set(), side: '', money: new Set() });
});

test('an instrument change keeps only the contract picks the new underlying offers', () => {
  const picks = { expiry: '2020-01-02', strikes: new Set(['1150000', '3100000']), side: 'PE', money: new Set() };
  const kept = keepOffered(picks, {
    rungs: contractRungsOf('options'),
    expiries: ['2020-01-02'],
    strikes: ['3100000', '3150000'],
    sides: ['CE', 'PE']
  });
  assert.equal(kept.expiry, '2020-01-02');
  assert.deepEqual([...kept.strikes], ['3100000']);
  assert.equal(kept.side, 'PE');
  // An expiry the new underlying lacks takes the strikes and side with it.
  const gone = keepOffered(picks, { rungs: contractRungsOf('options'), expiries: ['2020-01-09'], strikes: ['1150000'], sides: ['PE'] });
  assert.deepEqual(gone, { expiry: '', strikes: new Set(), side: '', money: new Set() });
});

test('the page re-seeds on a feed change and drops hidden rungs on a segment change', () => {
  const src = dbSource('../src/lib/db/selection.svelte.js');
  // A segment change empties the rungs the segment hides, at once.
  assert.match(src, /if \(segMoved\) \{\s*const rungs = contractRungsOf\(k\);\s*if \(!rungs\.expiry\) db\.expiry = '';/);
  // A feed change clears an unheld rung so the finest-rung seed runs again.
  assert.match(src, /if \(tf !== '' && !tfs\.some\(\(\/\*\* @type \{any\} \*\/ \[t\]\) => t === tf\)\) \{\s*db\.timeframe = '';/);
  // The stranded list names a leftover strike (DB-020).
  assert.match(src, /rung: gone\.length === 1 \? 'Strike' : 'Strikes'/);
});

test('each view keeps its own page, and every rung that changes which rows exist returns to page one', () => {
  const src = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(src, /let pages = \$state\(\{ census: 1, bars: 1 \}\)/);
  const reset = src.slice(src.indexOf('export function selectionPaging'));
  for (const rung of ['feeds.active', 'db.timeframe', 'db.expiry', 'db.side', 'db.fromDay', 'db.toDay', 'db.fromTime', 'db.toTime', 'db.oow']) {
    assert.ok(reset.slice(0, reset.indexOf('untrack')).includes(rung), `${rung} does not reset the page`);
  }
});

/* ── DB-044, DB-045: days from the census ─────────────────────────────── */

test('THE REGRESSION: the held span is the census span, not the rows on screen', () => {
  // 03 Sep 2018 09:15 IST and 24 Sep 2026 15:29:59 IST, in milliseconds.
  const first = Date.UTC(2018, 8, 3, 3, 45);
  const last = Date.UTC(2026, 8, 24, 9, 59, 59);
  const span = heldSpan([
    { firstAt: Date.UTC(2020, 2, 2, 3, 45), lastAt: Date.UTC(2020, 2, 31, 9, 59) },
    { firstAt: first, lastAt: Date.UTC(2018, 8, 28, 9, 59) },
    { firstAt: Date.UTC(2026, 8, 1, 3, 45), lastAt: last },
    { firstAt: null, lastAt: null }
  ]);
  assert.deepEqual(span, { lo: '2018-09-03', hi: '2026-09-24' });
  assert.deepEqual(heldSpan([]), { lo: '', hi: '' });
});

test('a bar after 18:30 UTC is filed on the next IST day', () => {
  assert.deepEqual(heldSpan([{ firstAt: Date.UTC(2020, 2, 1, 19, 0), lastAt: Date.UTC(2020, 2, 1, 19, 0) }]), {
    lo: '2020-03-02',
    hi: '2020-03-02'
  });
});

test("each feed's era comes from its own descriptor", () => {
  const gdfl = { wire: 'gdfl', display: 'Global Datafeeds', archive: { from: '2018-09-03', to: '2026-09-24' }, window: null };
  const zerodha = { wire: 'zerodha', display: 'Zerodha', archive: null, window: { from: '2018-01-01', to: '2026-09-30' } };
  assert.deepEqual(eraOf(gdfl), { from: '2018-09-03', to: '2026-09-24', kind: 'archive', label: "Global Datafeeds's archive" });
  assert.equal(eraOf(zerodha).from, '2018-01-01');
  assert.equal(eraOf(zerodha, true).from, '');
  assert.deepEqual(eraOf(null), { from: '', to: '', kind: 'none', label: '' });
});

test('a month outside the operator window is named so, inside it is not (D-3338)', () => {
  const zerodha = { window: { from: '2018-01-01', to: '2026-09-30' } };
  assert.equal(outsideWindow('2017-12', zerodha), true);
  assert.equal(outsideWindow('2015-01', zerodha), true);
  assert.equal(outsideWindow('2018-01', zerodha), false);
  assert.equal(outsideWindow('2026-09', zerodha), false);
  assert.equal(outsideWindow('2026-10', zerodha), true);
  assert.equal(outsideWindow('2015-01', { window: null }), false);
});

test('the calendar of record reads one code per day, and nothing past its ends', () => {
  const from = epochDayOf('2020-03-06');
  // Fri, Sat, Sun, Mon, Tue (10 Mar 2020, a holiday in this invented string), Wed.
  const cal = calendarOf({ from, days: 'R..R.R' });
  assert.ok(cal);
  assert.equal(cal.fromDate, '2020-03-06');
  assert.equal(cal.toDate, '2020-03-11');
  assert.equal(dayCode(cal, '2020-03-07'), '.');
  assert.equal(dayCode(cal, '2020-03-10'), '.');
  assert.equal(dayCode(cal, '2020-03-09'), 'R');
  assert.equal(dayCode(cal, '2020-03-12'), null);
  assert.equal(dayCode(cal, '2020-03-05'), null);
  assert.equal(calendarOf({ reason: 'withheld' }), null);
  assert.equal(isoOfEpoch(epochDayOf('2024-02-29')), '2024-02-29');
});

/* ── DB-074: the session grid ─────────────────────────────────────────── */

test('THE REGRESSION: a 1s page at 15:29 still offers 09:15, and every rung lists its own grid', () => {
  const one = sessionGrid(1, ['15:29']);
  assert.equal(one.length, 375);
  assert.equal(one[0].key, '09:15');
  assert.equal(one.at(-1)?.key, '15:29');
  assert.equal(sessionGrid(60).length, 375);
  assert.equal(sessionGrid(300).length, 75);
  assert.deepEqual(sessionGrid(3600).map((r) => r.key), ['09:15', '10:15', '11:15', '12:15', '13:15', '14:15', '15:15']);
  assert.deepEqual(sessionGrid(86_400), []);
  assert.deepEqual(sessionGrid(undefined), []);
});

test('a minute outside the regular session is listed only where a loaded bar holds it, and says so', () => {
  const grid = sessionGrid(60, ['09:07', '15:45', '10:00']);
  assert.equal(grid.length, 377);
  const pre = grid.find((r) => r.key === '09:07');
  assert.equal(pre?.detail, 'before 09:15');
  assert.match(pre?.why ?? '', /charter §3/);
  assert.equal(grid.find((r) => r.key === '15:45')?.detail, 'after 15:30');
});

test('a 1s bound carries its second; a minute rung never compares seconds', () => {
  assert.equal(timeBound('09:15', 1, 'from'), '09:15:00');
  assert.equal(timeBound('09:16', 1, 'to'), '09:16:59');
  assert.equal(timeBound('09:15', 1, 'from', '30'), '09:15:30');
  assert.equal(timeBound('09:15', 60, 'to'), '09:15');
  assert.equal(timeBound('9:15', 1, 'from'), '');
  assert.equal(rebound('09:15:30', 60, 'from'), '09:15');
  assert.equal(rebound('09:15', 1, 'to'), '09:15:59');
  assert.equal(rebound('09:15:30', 1, 'from'), '09:15:30');
  assert.equal(rebound('nonsense', 1, 'from'), '');
  assert.equal(SECONDS.length, 60);
  assert.equal(SECONDS[59], '59');
});

/* ── DB-076: expiry tags ──────────────────────────────────────────────── */

test('THE REGRESSION: 20 Mar 2019 is a weekly moved from Thursday 21 Mar, and 28 Mar is the monthly', () => {
  // The calendar of record marks 21 Mar 2019 closed in this invented string.
  const closed = new Set(['2019-03-21']);
  const tags = expiryTags(
    ['2019-03-07', '2019-03-14', '2019-03-20', '2019-03-28', '2019-02-28', '2019-04-04', '2019-04-11', '2019-04-25'],
    (iso) => (closed.has(iso) ? '.' : 'R')
  );
  assert.deepEqual(tags.get('2019-03-28'), { weekday: 'Thu', cadence: 'monthly', shift: null });
  assert.equal(tags.get('2019-03-07')?.cadence, 'weekly');
  const shifted = tags.get('2019-03-20');
  assert.equal(shifted?.weekday, 'Wed');
  assert.equal(shifted?.cadence, 'weekly');
  assert.equal(shifted?.shift?.from, '2019-03-21');
  assert.match(shifted?.shift?.why ?? '', /moved from Thu 2019-03-21, a day the exchange calendar of record marks closed/);
});

test('an off-weekday expiry with no closed day behind it is named, never given a cause', () => {
  const tags = expiryTags(['2023-08-31', '2023-09-07', '2023-09-13', '2023-09-21'], () => 'R');
  const odd = tags.get('2023-09-13');
  assert.equal(odd?.shift?.from, null);
  assert.match(odd?.shift?.why ?? '', /not a closed day, so this is not a holiday shift/);
  const beyond = expiryTags(['2026-09-29', '2026-10-06', '2026-10-12', '2026-10-20'], () => null);
  assert.match(beyond.get('2026-10-12')?.shift?.why ?? '', /does not reach/);
  assert.equal(weekdayOf('2026-10-10'), 6);
});

/* ── DB-077: links into /db ───────────────────────────────────────────── */

test('an instrument key from another page names the rungs it carries', () => {
  assert.deepEqual(linkOfKey('NSE-FNO-NIFTY-2019-03-28-1150000-CE'), {
    sym: 'NIFTY',
    contract: 'options',
    expiry: '2019-03-28',
    strike: '1150000',
    side: 'CE'
  });
  assert.deepEqual(linkOfKey('NSE-INDEX-NIFTY'), { sym: 'NIFTY', contract: 'spot', expiry: '', strike: '', side: '' });
  assert.equal(linkOfKey('NSE-FNO-NIFTY-2019-03-28-FUT')?.contract, 'futures');
  assert.equal(linkOfKey('NIFTY'), null);
});

/* ── DB-046, DB-G89: the words say the window narrows a contract ─────── */

test('no /db control says the day window is "spot only"', () => {
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.doesNotMatch(controls, /spot only, since a contract/);
  assert.doesNotMatch(controls, /title="The window in days\. Spot only/);
  assert.match(controls, /It narrows a spot series and a contract alike/);
  assert.match(dbSource('../src/lib/db/selection.svelte.js'), /The window narrows a contract too/);
});

/* ── DB-G98, DB-N09: strikes are multi-select; side counts follow them ── */

test('the strike picker is multi-select and the side counts are taken after the strike pick', () => {
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  const at = controls.indexOf('label="strikes"');
  const strike = controls.slice(controls.lastIndexOf('<Picker', at), at);
  assert.doesNotMatch(strike, /\bsingle\b/);
  const src = dbSource('../src/lib/db/selection.svelte.js');
  assert.match(src, /const sideBase = \$derived\.by\(\(\) => \{\s*let out = expiryGated;\s*if \(strikePick\.size > 0\)/);
  assert.match(src, /for \(const r of sideBase\) \{\s*if \(r\.side !== null\) c\.set/);
});

/* ── DB-N03, DB-N04, DB-N13, DB-N08: the controls themselves ─────────── */

test('THE REGRESSION: a typed day that was not applied is never silently thrown away', () => {
  const day = dbSource('../src/lib/DayField.svelte');
  // Blur settles only text that parsed and is in range; anything else stays, named.
  assert.match(day, /function settle\(\) \{\s*if \(typing === null\) return;\s*const iso = parse\(typing\);\s*if \(typing\.trim\(\) === '' \|\| \(iso !== '' && inRange\(iso\)\)\) typing = null;/);
  assert.match(day, /Not applied: the field still holds <b>\{render\(value\) \|\| 'no bound'\}<\/b>/);
  // A click selects the shown day, so typing replaces it rather than appending.
  assert.match(day, /function onFocus\(e\) \{\s*e\.currentTarget\.select\(\);/);
  assert.match(day, /onkeydown=\{onInputKey\}/);
});

test('the calendar hangs from the right edge when it would overflow, and strikes closed days', () => {
  const day = dbSource('../src/lib/DayField.svelte');
  assert.match(day, /alignRight = host\.left \+ panel\.offsetWidth > window\.innerWidth - 12;/);
  assert.match(day, /class:right=\{alignRight\}/);
  assert.match(day, /shut: closed \? closed\(iso\) === true : false/);
  assert.match(dbSource('../src/routes/db/DbControls.svelte'), /closed=\{db\.dayClosed\}/);
});

test('a filtered list takes the keyboard when it opens, and forgets its last search when it closes', () => {
  const picker = dbSource('../src/lib/Picker.svelte');
  assert.match(picker, /if \(open && pq\) pq\.focus\(\{ preventScroll: true \}\);/);
  assert.match(picker, /if \(!open\) q = '';/);
  assert.match(picker, /bind:this=\{pq\}/);
});

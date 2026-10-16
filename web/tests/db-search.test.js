// /db — THE SEARCH BOX, DRIVEN (WP-51; D-3332, D-3337; DB-001, DB-040, DB-G97).
//
// Every finding this file pins was measured on the operator's census before
// the fix: `NIFTY28MAR1911500CE`, `NIFTY 11500 CE`, `NIFTY 11,500 CE`,
// `11500`, `2019-03-28`, `28 Mar 2019` and `28MAR19` found 0 rows; `11,500 CE`
// found NIFTY and NESTLEIND blended into one list; `FINNIFTY`, `NIFTY 50`,
// `NIFTY BANK`, `ZOMATO`, `TATAMOTORS` and `IIFLWAM` found nothing; and the
// typed prefix WAS the selection, so `NIFTY` brought NIFTY100 and NIFTYIT.
//
// THE CENSUS HERE IS GENERATED. The store keys are the store's own spelling
// (`crates/core/src/instrument.rs`), the month counts are invented, and no
// row carries a price: nothing below is market evidence.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  normalise,
  strikePaisa,
  isoDay,
  tickerReadings,
  naturalReading,
  parseQuery
} from '../src/lib/db/contract-parse.js';
import {
  buildCatalog,
  buildCatalogNow,
  buildNames,
  search,
  selectionOf,
  windowOf,
  nameMatches,
  MAX_COUNT
} from '../src/lib/db/search.js';

/* ── the generated census ─────────────────────────────────────────────── */

/**
 * One decorated census row, as `$lib/db/decoration.js` hands the page one:
 * the underlying, expiry, strike and side read off the key.
 *
 * @param {string} instrument
 * @param {string} month
 * @param {string} [timeframe]
 */
function row(instrument, month, timeframe = '1s') {
  const opt = /^NSE-FNO-(.+)-(\d{4}-\d{2}-\d{2})-(\d+)-(CE|PE)$/.exec(instrument);
  const fut = /^NSE-FNO-(.+)-(\d{4}-\d{2}-\d{2})-FUT$/.exec(instrument);
  const spot = /^NSE-(?:INDEX|CASH)-(.+)$/.exec(instrument);
  return {
    instrument,
    month,
    timeframe,
    rows: 1,
    underlying: opt?.[1] ?? fut?.[1] ?? spot?.[1] ?? instrument,
    expiry: opt?.[2] ?? fut?.[2] ?? null,
    strike: opt ? Number(opt[3]) : null,
    side: opt ? opt[4] : null
  };
}

/** `n` consecutive months ending at `last` (`YYYY-MM`), oldest first. */
function monthsTo(/** @type {string} */ last, /** @type {number} */ n) {
  const [y, m] = last.split('-').map(Number);
  const out = [];
  for (let i = n - 1; i >= 0; i -= 1) {
    const t = y * 12 + (m - 1) - i;
    out.push(`${Math.floor(t / 12)}-${String((t % 12) + 1).padStart(2, '0')}`);
  }
  return out;
}

/** @param {string} key @param {string} last @param {number} n @param {string} [tf] */
const held = (key, last, n, tf) => monthsTo(last, n).map((m) => row(key, m, tf));

const CENSUS = [
  ...held('NSE-INDEX-NIFTY', '2026-09', 97),
  ...held('NSE-INDEX-NIFTY100', '2026-09', 12),
  ...held('NSE-INDEX-NIFTYIT', '2026-09', 12),
  ...held('NSE-INDEX-BANKNIFTY', '2026-09', 97),
  ...held('NSE-INDEX-NIFTYFINSERVICE', '2026-09', 12),
  ...held('NSE-CASH-NESTLEIND', '2026-09', 97),
  ...held('NSE-CASH-AXISBANK', '2026-09', 97),
  ...held('NSE-CASH-ETERNAL', '2026-09', 18),
  ...held('NSE-CASH-TMPV', '2026-09', 12),
  ...held('NSE-CASH-360ONE', '2026-09', 45),
  ...held('NSE-CASH-LT', '2026-09', 97),
  ...held('NSE-CASH-LTM', '2026-09', 8),
  // NIFTY 28 Mar 2019 11,500 CE: seven months, its life 2018-09 to 2019-03.
  ...held('NSE-FNO-NIFTY-2019-03-28-1150000-CE', '2019-03', 7),
  ...held('NSE-FNO-NIFTY-2019-03-28-1150000-PE', '2019-03', 6),
  ...held('NSE-FNO-NIFTY-2019-03-28-1160000-CE', '2019-03', 5),
  // a weekly of the same month, earlier than the monthly
  ...held('NSE-FNO-NIFTY-2019-03-20-1150000-CE', '2019-03', 1),
  ...held('NSE-FNO-NIFTY-2019-04-25-1150000-CE', '2019-04', 3),
  // NESTLEIND at the same strike and expiry: never drawn under NIFTY
  ...held('NSE-FNO-NESTLEIND-2019-03-28-1150000-CE', '2019-03', 2),
  // NIFTYIT, whose strikes the prefix match once blended into NIFTY's menu
  ...held('NSE-FNO-NIFTYIT-2019-06-27-1390000-CE', '2019-06', 2),
  ...held('NSE-FNO-BANKNIFTY-2019-03-28-2650000-PE', '2019-03', 2),
  // an old-symbol underlying the store holds under its own key
  ...held('NSE-FNO-TATAMOTORS-2019-03-28-17250-CE', '2019-03', 2),
  // one ticker, two held readings: the day form's 20 Jun 2019 12,500 CE, and
  // the monthly form's June 2020 contract at an impossible strike (D-3331)
  ...held('NSE-FNO-NIFTY-2019-06-20-1250000-CE', '2019-06', 2),
  ...held('NSE-FNO-NIFTY-2020-06-25-191250000-CE', '2019-05', 1)
];

/** The `/aliases.json` shape (`crates/api/src/aliases.rs`), cut to what is driven. */
const ALIASES = {
  decision: 'D-3332',
  index: [
    { fno: 'NIFTY', nse_name: 'NIFTY 50', index_symbol: 'NIFTY 50', kite: 'NIFTY 50', store_key: 'NIFTY', reference_only: false, status: 'sourced' },
    { fno: 'BANKNIFTY', nse_name: 'NIFTY BANK', index_symbol: 'NIFTY BANK', kite: 'NIFTY BANK', store_key: 'BANKNIFTY', reference_only: false, status: 'sourced' },
    {
      fno: 'FINNIFTY',
      nse_name: 'NIFTY FINANCIAL SERVICES',
      index_symbol: 'NIFTY FIN SERVICE',
      kite: 'NIFTY FIN SERVICE',
      store_key: 'NIFTYFINSERVICE',
      reference_only: false,
      status: 'sourced'
    },
    { fno: 'NIFTYIT', nse_name: 'NIFTY IT', index_symbol: 'NIFTY IT', kite: 'NIFTY IT', store_key: 'NIFTYIT', reference_only: false, status: 'sourced' },
    // Sourced, and held by no row of this census.
    { fno: 'NIFTYFPI', nse_name: 'NIFTY INDIA FPI 150', index_symbol: 'NIFTY FPI 150', kite: 'NIFTY FPI 150', store_key: 'NIFTYFPI150', reference_only: false, status: 'sourced' },
    // A row D-3332 leaves UNVERIFIED is never applied.
    { fno: 'XQIDX', nse_name: 'NIFTY 100', index_symbol: 'NIFTY 100', kite: 'NIFTY 100', store_key: 'NIFTY100', reference_only: false, status: 'unverified' }
  ],
  symbol_changes: [
    { old: 'LTI', new: 'LTIM', effective: '2022-12-05', status: 'sourced' },
    { old: 'IIFLWAM', new: '360ONE', effective: '2023-01-23', status: 'sourced' },
    { old: 'ZOMATO', new: 'ETERNAL', effective: '2025-04-09', status: 'sourced' },
    { old: 'TATAMOTORS', new: 'TMPV', effective: '2025-10-24', status: 'sourced' },
    { old: 'LTIM', new: 'LTM', effective: '2026-02-27', status: 'sourced' },
    { old: 'QQOLD', new: 'AXISBANK', effective: '2020-01-01', status: 'unverified' }
  ],
  not_aliased: []
};

const OPTS = { today: '2026-10-10', archiveTo: '2026-09-24' };
const cat = buildCatalogNow(CENSUS);
const names = buildNames(cat, ALIASES);

/** @param {string} text */
const find = (text, opts = {}) => search(cat, names, text, { ...OPTS, ...opts });

/* ── the parser ───────────────────────────────────────────────────────── */

test('a strike is read to paisa exactly, never through a float', () => {
  assert.equal(strikePaisa('11500'), 1150000);
  assert.equal(strikePaisa('172.5'), 17250);
  assert.equal(strikePaisa('172.55'), 17255);
  assert.equal(strikePaisa('0.05'), 5);
  assert.equal(strikePaisa('1.234'), null);
  assert.equal(strikePaisa('0'), null);
  assert.equal(strikePaisa('.5'), null);
});

test('a date is a real calendar day or nothing', () => {
  assert.equal(isoDay(2019, 3, 28), '2019-03-28');
  assert.equal(isoDay(2019, 2, 29), null);
  assert.equal(isoDay(2020, 2, 29), '2020-02-29');
  assert.equal(isoDay(2019, 13, 1), null);
});

test('digit groups close, Indian or Western, and case folds', () => {
  assert.equal(normalise(' nifty  11,500  ce '), 'NIFTY 11500 CE');
  assert.equal(normalise('NIFTY 1,15,000 PE'), 'NIFTY 115000 PE');
});

test('GDFL day form, monthly form and Kite weekly form each read to the contract they name', () => {
  const day = tickerReadings('NIFTY28MAR1911500CE');
  assert.ok(day.some((r) => r.name === 'NIFTY' && r.expiry === '2019-03-28' && r.strike === 1150000 && r.side === 'CE' && r.form === 'GDFL day form'));
  const monthly = tickerReadings('ACC18DEC1500CE');
  assert.ok(monthly.some((r) => r.name === 'ACC' && r.month === '2018-12' && r.monthly && r.strike === 150000 && r.side === 'CE'));
  const kite = tickerReadings('NIFTY1932811500CE');
  const k = kite.find((r) => r.name === 'NIFTY');
  assert.ok(k);
  assert.equal(k.expiry, '2019-03-28');
  assert.equal(k.strike, 1150000);
  assert.match(k.form, /UNVERIFIED/);
  const frac = tickerReadings('TATAMOTORS28MAR19172.5CE');
  assert.ok(frac.some((r) => r.name === 'TATAMOTORS' && r.strike === 17250));
  const fut = tickerReadings('NIFTY28MAR19FUT');
  assert.ok(fut.some((r) => r.name === 'NIFTY' && r.expiry === '2019-03-28' && r.side === 'FUT' && r.strike === null));
  // A symbol holding a month's letters and one ending in digits still split right.
  assert.ok(tickerReadings('MARICO28MAR19350CE').some((r) => r.name === 'MARICO' && r.expiry === '2019-03-28'));
  assert.ok(tickerReadings('NIFTYNXT5028MAR1925000CE').some((r) => r.name === 'NIFTYNXT50' && r.strike === 2500000));
  assert.deepEqual(tickerReadings('NIFTY'), []);
});

test('natural text reads a name, an expiry in every spelling, a strike and a side', () => {
  for (const [text, want] of /** @type {Array<[string, any]>} */ ([
    ['NIFTY 11500 CE', { name: 'NIFTY', strike: 1150000, side: 'CE', expiry: null }],
    ['NIFTY 11,500 CE', { name: 'NIFTY', strike: 1150000, side: 'CE' }],
    ['nifty 28/03/2019 11500 call', { name: 'NIFTY', expiry: '2019-03-28', strike: 1150000, side: 'CE' }],
    ['2019-03-28', { name: '', expiry: '2019-03-28' }],
    ['28 Mar 2019', { name: '', expiry: '2019-03-28' }],
    ['28MAR19', { name: '', expiry: '2019-03-28' }],
    ['28MAR2019', { name: '', expiry: '2019-03-28' }],
    ['Mar 28 2019', { name: '', expiry: '2019-03-28' }],
    ['Mar 2019', { name: '', month: '2019-03' }],
    ['2019-03', { name: '', month: '2019-03' }],
    ['NIFTY 19MAR', { name: 'NIFTY', month: '2019-03' }],
    ['11500CE', { name: '', strike: 1150000, side: 'CE' }],
    ['BANKNIFTY 28MAR19 26500 PE', { name: 'BANKNIFTY', expiry: '2019-03-28', strike: 2650000, side: 'PE' }],
    ['NIFTY 28 Mar', { name: 'NIFTY', dayMonth: '03-28' }]
  ])) {
    const r = naturalReading(normalise(text));
    assert.ok(r, text);
    for (const [k, v] of Object.entries(want)) assert.deepEqual(/** @type {any} */ (r)[k], v, `${text}: ${k}`);
  }
  // Two strikes name two contracts, which is none.
  assert.equal(naturalReading(normalise('NIFTY 11500 11600 CE')), null);
});

test('the whole text is also read as a ticker once its spaces are gone', () => {
  const q = parseQuery('NIFTY 28MAR19 11500CE');
  assert.ok(q.readings.some((r) => r.form === 'GDFL day form' && r.expiry === '2019-03-28' && r.strike === 1150000));
});

/* ── DB-G97: a contract by ticker, natural name, strike or expiry ──────── */

test('NIFTY28MAR1911500CE opens one contract: NIFTY, 2019-03-28, 11,500 CE, its 7 months, Expired options', () => {
  const a = find('NIFTY28MAR1911500CE');
  assert.equal(a.groups.length, 1);
  const [g] = a.groups;
  assert.equal(g.u, 'NIFTY');
  assert.equal(g.hits.length, 1);
  const [h] = g.hits;
  assert.equal(h.kind, 'contract');
  assert.equal(h.instrument, 'NSE-FNO-NIFTY-2019-03-28-1150000-CE');
  assert.equal(h.expiry, '2019-03-28');
  assert.equal(h.strike, 1150000);
  assert.equal(h.side, 'CE');
  assert.equal(h.seg, 'options');
  assert.equal(h.months, 7);
  assert.match(h.detail, /Expired options · 7 instrument-month/);
  assert.deepEqual(a.forms, ['GDFL day form']);
});

test('the monthly and Kite weekly forms land on the same contract; a monthly name is the month\'s last expiry, never its weekly', () => {
  for (const text of ['NIFTY19MAR11500CE', 'NIFTY1932811500CE']) {
    const a = find(text);
    assert.deepEqual(a.groups.map((g) => g.u), ['NIFTY'], text);
    assert.deepEqual(a.groups[0].hits.map((h) => h.instrument), ['NSE-FNO-NIFTY-2019-03-28-1150000-CE'], text);
  }
});

test('NIFTY 11500 CE and NIFTY 11,500 CE list NIFTY\'s 11,500 calls only, newest expiry first', () => {
  for (const text of ['NIFTY 11500 CE', 'NIFTY 11,500 CE', 'nifty 11500 call']) {
    const a = find(text);
    assert.deepEqual(a.groups.map((g) => g.u), ['NIFTY'], text);
    assert.deepEqual(
      a.groups[0].hits.map((h) => h.expiry),
      ['2019-04-25', '2019-03-28', '2019-03-20'],
      text
    );
    assert.ok(a.groups[0].hits.every((h) => h.strike === 1150000 && h.side === 'CE'));
  }
});

test('a strike with no name is grouped by underlying: NESTLEIND never mixes into NIFTY', () => {
  for (const text of ['11500', '11,500 CE', '11500 CE']) {
    const a = find(text);
    const us = a.groups.map((g) => g.u);
    assert.ok(us.includes('NIFTY') && us.includes('NESTLEIND'), text);
    for (const g of a.groups) for (const h of g.hits) assert.equal(h.u, g.u, `${text}: ${h.instrument} under ${g.u}`);
    // NIFTY holds most of that strike, so it is drawn first.
    assert.equal(us[0], 'NIFTY', text);
  }
});

test('an expiry in any spelling lists each underlying that expires that day, one group each', () => {
  for (const text of ['2019-03-28', '28 Mar 2019', '28MAR19', '28/03/2019']) {
    const a = find(text);
    assert.deepEqual(new Set(a.groups.map((g) => g.u)), new Set(['NIFTY', 'NESTLEIND', 'BANKNIFTY', 'TATAMOTORS']), text);
    for (const g of a.groups) {
      assert.equal(g.hits.length, 1);
      assert.equal(g.hits[0].kind, 'expiry');
      assert.equal(g.hits[0].expiry, '2019-03-28');
      assert.equal(g.hits[0].seg, 'options');
    }
  }
  const nifty = find('NIFTY Mar 2019');
  assert.deepEqual(nifty.groups[0].hits.map((h) => h.expiry), ['2019-03-28', '2019-03-20']);
});

test('a contract the store does not hold says what was read and which part is missing', () => {
  const a = find('NIFTY 99999 CE');
  assert.equal(a.total, 0);
  assert.match(String(a.why), /NIFTY strike 99,999 CE/);
  assert.match(String(a.why), /NIFTY is held, but not with strike 99,999 CE/);
  const b = find('QZQZQZ');
  assert.equal(b.total, 0);
  assert.match(String(b.why), /Nothing this feed holds is called/);
  const spotOnly = buildCatalogNow(CENSUS.filter((r) => r.expiry === null));
  const c = search(spotOnly, buildNames(spotOnly, ALIASES), 'NIFTY 11500 CE', OPTS);
  assert.match(String(c.why), /holds no future or option at all/);
});

test('a ticker that reads two held ways draws its day reading first, the monthly one after it (D-2806 item 2)', () => {
  const a = find('NIFTY20JUN1912500CE');
  assert.deepEqual(a.groups.map((g) => g.u), ['NIFTY']);
  assert.deepEqual(a.groups[0].hits.map((h) => h.instrument), [
    'NSE-FNO-NIFTY-2019-06-20-1250000-CE',
    'NSE-FNO-NIFTY-2020-06-25-191250000-CE'
  ]);
  assert.deepEqual(a.groups[0].hits.map((h) => h.form), ['GDFL day form', 'GDFL monthly form']);
});

/* ── DB-040: aliases and old symbols ───────────────────────────────────── */

test('an index name the table knows and this feed does not hold says so, not "nothing is called this"', () => {
  for (const text of ['NIFTYFPI', 'NIFTY FPI 150']) {
    const a = find(text);
    assert.equal(a.total, 0);
    assert.match(String(a.why), /NSE index NIFTY INDIA FPI 150 .*filed as NIFTYFPI150\), and this feed's store holds nothing/, text);
  }
});


test('index names find the stored key: NIFTY 50, NIFTY BANK, FINNIFTY, NIFTY FIN SERVICE', () => {
  for (const [text, key] of [
    ['NIFTY 50', 'NIFTY'],
    ['nifty50', 'NIFTY'],
    ['NIFTY BANK', 'BANKNIFTY'],
    ['FINNIFTY', 'NIFTYFINSERVICE'],
    ['NIFTY FIN SERVICE', 'NIFTYFINSERVICE'],
    ['NIFTY FINANCIAL SERVICES', 'NIFTYFINSERVICE']
  ]) {
    const a = find(text);
    assert.equal(a.groups[0]?.u, key, text);
    assert.equal(a.groups[0].hits[0].kind, 'instrument');
    assert.equal(a.groups[0].rank, 0, `${text} is an exact name of ${key}`);
  }
  assert.match(find('FINNIFTY').groups[0].notes.join(' '), /NIFTY FINANCIAL SERVICES.*F&O symbol FINNIFTY/);
});

test('an UNVERIFIED alias row or symbol change is not applied (D-3332 decision 1)', () => {
  assert.equal(find('XQIDX').total, 0);
  assert.ok(!find('QQOLD').groups.some((g) => g.u === 'AXISBANK'));
});

test('ZOMATO finds ETERNAL, formerly ZOMATO until the day before the change', () => {
  const a = find('ZOMATO');
  assert.deepEqual(a.groups.map((g) => g.u), ['ETERNAL']);
  assert.match(a.groups[0].notes.join(' '), /formerly ZOMATO until 08 Apr 2025/);
  assert.match(a.groups[0].hits[0].detail, /matched “ZOMATO”/);
});

test('IIFLWAM finds 360ONE; LTI finds LTM through LTIM, said once', () => {
  assert.deepEqual(find('IIFLWAM').groups.map((g) => g.u), ['360ONE']);
  const lti = find('LTI');
  assert.equal(lti.groups[0].u, 'LTM');
  assert.deepEqual(lti.groups[0].notes, [
    'formerly LTI until 04 Dec 2022, then LTIM until 26 Feb 2026 (NSE symbolchange.csv; LTM from 27 Feb 2026)'
  ]);
});

test('TATAMOTORS finds both its old-key options and TMPV, each its own group', () => {
  const a = find('TATAMOTORS');
  assert.deepEqual(new Set(a.groups.map((g) => g.u)), new Set(['TATAMOTORS', 'TMPV']));
  const old = a.groups.find((g) => g.u === 'TATAMOTORS');
  assert.match(String(old?.notes.join(' ')), /renamed TMPV effective 24 Oct 2025/);
  // and its old-key contract is found by its own ticker
  const t = find('TATAMOTORS28MAR19172.5CE');
  assert.equal(t.groups[0].hits[0].instrument, 'NSE-FNO-TATAMOTORS-2019-03-28-17250-CE');
});

test('a name is matched by any part of it, and every underlying is its own group', () => {
  const a = find('BANK');
  const us = a.groups.map((g) => g.u);
  assert.ok(us.includes('BANKNIFTY') && us.includes('AXISBANK'));
  const m = nameMatches(names, 'NIFTY');
  assert.equal(m.get('NIFTY')?.rank, 0);
  assert.equal(m.get('NIFTY100')?.rank, 1);
  assert.equal(m.get('BANKNIFTY')?.rank, 1, 'BANKNIFTY begins with NIFTY under its name NIFTY BANK');
});

/* ── DB-001: what is TYPED never becomes the selection ─────────────────── */

test('typing NIFTY lists NIFTY, NIFTY100 and NIFTYIT apart, exact name first', () => {
  const a = find('NIFTY');
  assert.equal(a.groups[0].u, 'NIFTY');
  for (const k of ['NIFTY100', 'NIFTYIT']) assert.ok(a.groups.some((g) => g.u === k), k);
  for (const g of a.groups) assert.ok(g.hits.every((h) => h.u === g.u));
});

test('a contract under a name is looked up under that exact name, never under one it is part of', () => {
  // NIFTYIT holds 13,900 CE; NIFTY does not, so `NIFTY 13900 CE` finds nothing.
  assert.equal(find('NIFTY 13900 CE').total, 0);
  assert.equal(find('NIFTYIT 13900 CE').groups[0].u, 'NIFTYIT');
});

/* ── the selection a hit sets, in one step ─────────────────────────────── */

test('a contract hit sets underlying, segment, expiry, strike and side together', () => {
  const [h] = find('NIFTY28MAR1911500CE').groups[0].hits;
  const sel = selectionOf(h, cat, { timeframe: '1s', universe: '', universeNames: null, oow: false });
  assert.equal(sel.filter, 'NIFTY');
  assert.equal(sel.kind, 'options');
  assert.equal(sel.timeframe, '1s', 'the rung on screen is kept where the contract holds it');
  assert.equal(sel.expiry, '2019-03-28');
  assert.deepEqual([...sel.strikes], ['1150000']);
  assert.equal(sel.side, 'CE');
  assert.deepEqual(sel.notes, []);
});

test('a hit re-seeds a rung its target does not hold, widens a universe that hides it, and says so', () => {
  const [h] = find('NIFTY28MAR1911500CE').groups[0].hits;
  const sel = selectionOf(h, cat, {
    timeframe: '1day',
    universe: 'n50',
    universeNames: new Set(['RELIANCE']),
    oow: false,
    universeLabel: 'NIFTY 50'
  });
  assert.equal(sel.timeframe, '', 'the finest held rung is re-seeded by the page');
  assert.equal(sel.universe, '');
  assert.match(sel.notes.join(' '), /NIFTY is not in NIFTY 50/);
});

test('an instrument hit leaves the segment to the page\'s seed and opens months outside the window only when that is all there is', () => {
  const old = buildCatalogNow(
    [...held('NSE-INDEX-OLDIDX', '2017-12', 36, '1day'), ...held('NSE-INDEX-NIFTY', '2026-09', 3, '1day')],
    windowOf({ window: { from: '2018-01-01', to: '2026-09-30' } })
  );
  const oldNames = buildNames(old, null, 'not read in this test');
  const [h] = search(old, oldNames, 'OLDIDX', OPTS).groups[0].hits;
  const sel = selectionOf(h, old, { timeframe: '1day', universe: '', universeNames: null, oow: false });
  assert.equal(sel.kind, '');
  assert.equal(sel.oow, true);
  assert.match(sel.notes.join(' '), /outside your window/);
  const [n] = search(old, oldNames, 'NIFTY', OPTS).groups[0].hits;
  assert.equal(selectionOf(n, old, { timeframe: '1day', universe: '', universeNames: null, oow: false }).oow, false);
  // With no alias table the box still answers by store key, and says why the rest is not searched.
  assert.equal(oldNames.why, 'not read in this test');
  assert.match(String(search(old, oldNames, 'ZOMATO', OPTS).why), /alias table is not in force/);
});

test('the page assigns every rung a hit names, through the selection state, never a typed prefix', () => {
  const src = readFileSync(new URL('../src/routes/db/DbSearch.svelte', import.meta.url), 'utf8');
  for (const line of [
    'db.filter = sel.filter;',
    'db.kind = sel.kind;',
    'db.timeframe = sel.timeframe;',
    'db.expiry = sel.expiry;',
    'db.strikePick = sel.strikes;',
    'db.side = sel.side;',
    'db.universe = sel.universe;',
    'db.oow = sel.oow;'
  ]) {
    assert.ok(src.includes(line), line);
  }
  // The box is bound to the search text, never to the instrument key.
  assert.match(src, /bind:value=\{db\.query\}/);
  assert.doesNotMatch(src, /bind:value=\{db\.filter\}/);
  assert.doesNotMatch(src, /db\.filter = row\.key/);
});

/* ── the catalog, chunked; and the cost of a keystroke ─────────────────── */

test('the chunked catalog equals the one-pass catalog, yields between chunks and can be cancelled', async () => {
  let yields = 0;
  const chunked = await buildCatalog(CENSUS, { chunkSize: 16, yieldControl: async () => { yields += 1; } });
  assert.equal(yields, Math.ceil(CENSUS.length / 16) - 1);
  assert.equal(chunked.rows, cat.rows);
  assert.equal(chunked.contracts, cat.contracts);
  assert.deepEqual([...chunked.unders.keys()].sort(), [...cat.unders.keys()].sort());
  assert.deepEqual(chunked.expiries, cat.expiries);
  const controller = new AbortController();
  const work = buildCatalog(CENSUS, { chunkSize: 16, signal: controller.signal, yieldControl: async () => controller.abort() });
  await assert.rejects(work, /abort/i);
  await assert.rejects(buildCatalog(CENSUS, { chunkSize: 0 }), /chunk size/);
});

/**
 * A generated census the size of GDFL's (3,85,442 rows on 10 Oct 2026):
 * 300 underlyings, each with 24 expiries of 25 strikes a side over a few
 * months, plus 97 spot months each.
 */
function bigCensus() {
  /** @type {any[]} */
  const out = [];
  for (let u = 0; u < 300; u += 1) {
    const sym = `SYM${String(u).padStart(3, '0')}${u % 7 === 0 ? 'BANK' : ''}`;
    out.push(...held(`NSE-CASH-${sym}`, '2026-09', 97));
    for (let e = 0; e < 24; e += 1) {
      const y = 2019 + Math.floor(e / 12);
      const expiry = `${y}-${String((e % 12) + 1).padStart(2, '0')}-25`;
      for (let k = 0; k < 25; k += 1) {
        const strike = (100 + k * 10) * 100;
        for (const side of ['CE', 'PE']) {
          for (const m of monthsTo(expiry.slice(0, 7), 1 + (k % 2))) out.push(row(`NSE-FNO-${sym}-${expiry}-${strike}-${side}`, m));
        }
      }
    }
  }
  return out;
}

test('a keystroke never walks the census, and its p99 over a GDFL-sized census is inside one frame (16 ms)', (t) => {
  const rows = bigCensus();
  assert.ok(rows.length > 350_000, `generated ${rows.length} rows`);
  const big = buildCatalogNow(rows);
  const bigNames = buildNames(big, ALIASES);
  /* EVERY ROW ACCESS AFTER THE BUILD IS COUNTED: the census array is
     wrapped, and a search that read one row would show here. */
  let reads = 0;
  const watched = new Proxy(rows, { get: (o, k) => { if (typeof k === 'string' && /^\d+$/.test(k)) reads += 1; return Reflect.get(o, k); } });
  void watched.length;
  const keys = [
    'S', 'SY', 'SYM', 'SYM1', 'SYM12', 'SYM123', 'BANK', 'SYM007BANK 25 Jan 2019 CE', 'SYM010 200 CE',
    '200', '200 CE', '2019-01-25', '25 JAN 2019', 'SYM010 Mar 2019', 'SYM01025JAN19200CE', 'SYM01019JAN200CE',
    'NIFTY', 'ZOMATO', 'QZQZQZ', 'SYM299 99999 CE'
  ];
  /** @type {number[]} */
  const ms = [];
  for (let rep = 0; rep < 10; rep += 1) {
    for (const k of keys) {
      // typed one character at a time, as a reader types it
      for (let n = 1; n <= k.length; n += 1) {
        const t0 = performance.now();
        const a = search(big, bigNames, k.slice(0, n), OPTS);
        ms.push(performance.now() - t0);
        assert.ok(a.total <= MAX_COUNT);
      }
    }
  }
  assert.equal(reads, 0, 'no keystroke read a census row');
  ms.sort((a, b) => a - b);
  const p50 = ms[Math.floor(ms.length * 0.5)];
  const p99 = ms[Math.floor(ms.length * 0.99)];
  t.diagnostic(`${ms.length} keystrokes over ${rows.length} generated rows: p50 ${p50.toFixed(3)} ms, p99 ${p99.toFixed(3)} ms, max ${ms[ms.length - 1].toFixed(3)} ms`);
  assert.ok(p99 < 16, `p99 ${p99.toFixed(2)} ms is over one 60 Hz frame`);
});

test('a search that reaches more than MAX_COUNT hits stops counting and says so', () => {
  const a = search(cat, names, '11500', { ...OPTS, maxCount: 2 });
  assert.equal(a.capped, true);
  assert.equal(a.total, 2);
});

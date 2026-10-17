// COVERAGE AGAINST THE CALENDAR OF RECORD (WP-54; D-3336, D-3320, D-3319).
//
// `node --test web/tests/` — node's own runner, no dependency. This drives the
// module the /db coverage grid imports, `$lib/completeness.js`, over ledgers
// built by `$lib/calendar-owed.js` from synthetic `/calendar.json` bodies in
// the server's own shape (`crates/api/src/calendar_record.rs`). The day strings
// and counts are invented; the ARITHMETIC they exercise is the one each
// finding measured on the operator's store:
//
//   DB-014 / DB-G34  a hole every series shares read "full" against the
//                    fullest peer; a CAS-eligible share read short against the
//                    index; NIFTY 1s 2018-09 read "full 100.00%" at 4,00,200
//                    bars against 18 x 22,500 = 4,05,000 owed.
//   DB-G63           a trade-driven option read 0.0065% covered.
//   DB-012           absent months were never drawn.
//   DB-013 / DB-G35  a derived rung behind its 1min source read as ordinary.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  denominators,
  denomKey,
  isSole,
  judgeRow,
  absentMonths,
  rollUpMonths,
  monthVerdict,
  isTradeDriven,
  segmentOfKey,
  derivedPartial,
  istDayOf,
  peersAtMost,
  STORE_WIDE_SHARE,
  coverPct,
  owedKind
} from '../src/lib/completeness.js';
import { owedLedger, epochDayOfIso, ledgerSpan, derivativeEras } from '../src/lib/calendar-owed.js';

/**
 * A synthetic `/calendar.json` body over `from` with the given day codes.
 *
 * @param {{ from: string, days: string, regular: number, cas?: number | null, special?: any[], held?: string | null, series?: { symbol: string, segment: string } | null, rung?: string }} o
 */
function body(o) {
  return {
    status: 'ok',
    feed: 'zerodha',
    rung: o.rung ?? '1min',
    from: epochDayOfIso(o.from),
    days: o.days,
    held: o.held ?? null,
    heldBy: o.series ? [o.series.symbol] : ['BANKNIFTY', 'NIFTY'],
    target: o.series ? { kind: 'series', exchange: 'NSE', ...o.series } : { kind: 'exchange' },
    owed: { rung: o.rung ?? '1min', regular: o.regular, casEligible: o.cas ?? null },
    special: o.special ?? [],
    halts: []
  };
}

/** A census row of one instrument-month. */
function row(/** @type {Partial<import('../src/lib/completeness.js').Judgeable> & { instrument: string, month: string, rows: number }} */ r) {
  return {
    timeframe: '1min',
    firstAt: null,
    lastAt: null,
    expiry: null,
    strike: null,
    underlying: r.instrument.split('-').slice(2).join('-'),
    ...r
  };
}

// 2024-11-01 (a Friday) to 2024-11-30: the 1st a Muhurat (P), weekends
// closed, two weekday holidays — 19 sessions, as the record has it.
const NOV_CODES = 'P..RRRRR..RRRR...RRRR..RRRRR..';

test('a hole every series shares is a store-wide hole, never "full" (DB-012, DB-014, DB-G34)', () => {
  const held = NOV_CODES.split('').map((c, i) => (c !== '.' && i >= 3 && i < 8 ? '1' : '0')).join('');
  const L = owedLedger(
    body({
      from: '2024-11-01',
      days: NOV_CODES,
      regular: 375,
      cas: 360,
      special: [{ day: epochDayOfIso('2024-11-01'), code: 'P', owed: 60, indexOwed: 60 }],
      held,
      series: { symbol: 'NIFTY', segment: 'INDEX' }
    })
  );
  const nifty = row({ instrument: 'NSE-INDEX-NIFTY', month: '2024-11', rows: 1875 });
  const bank = row({ instrument: 'NSE-INDEX-BANKNIFTY', month: '2024-11', rows: 1875 });
  // BEFORE: the peer denominator. Both rows are the fullest at (2024-11,
  // 1min), so each was "short by 0" — "full".
  const peer = denominators([nifty, bank]);
  assert.equal(peer.fullest.get(denomKey('2024-11', '1min')), 1875);
  // AFTER: the calendar owes 18 x 375 + 60 = 6,810 over 19 sessions.
  const c = judgeRow(nifty, { ledger: L, fullest: 1875, support: 2, peers: { n: 2, atMost: 2 } });
  assert.equal(c.basis, 'calendar');
  assert.equal(c.sessions, 19);
  assert.equal(c.owed, 6810);
  assert.equal(c.missing, 6810 - 1875);
  assert.equal(c.state, 'gap');
  assert.ok(c.notes.includes('store_wide'), 'the fullest peer is short too: the hole is the store\'s');
  assert.ok(c.notes.includes('special'), 'the Muhurat is owed by its window');
  // The series' own held digits: the days, not the bars.
  assert.equal(c.heldExact, true);
  assert.equal(c.heldSessions, 5);
  // A second series not short of the calendar keeps the hole from being store-wide.
  const alone = judgeRow(nifty, { ledger: L, fullest: 6810, support: 2 });
  assert.ok(!alone.notes.includes('store_wide'));
  // One series alone has nothing to share a hole with.
  assert.ok(!judgeRow(nifty, { ledger: L, fullest: 1875, support: 1 }).notes.includes('store_wide'));
  // ONE LATE-PULLED SERIES DOES NOT HIDE IT: 271 of 272 series hold no more
  // than 1,875 (Zerodha 2024-11, where one share pulled later holds 4,093).
  const sorted = [...Array(271).fill(1875), 4093];
  assert.deepEqual(peersAtMost(sorted, 1875), { n: 272, atMost: 271 });
  const outlier = judgeRow(nifty, { ledger: L, fullest: 4093, support: 272, peers: peersAtMost(sorted, 1875) });
  assert.ok(outlier.notes.includes('store_wide'));
  assert.ok(271 >= STORE_WIDE_SHARE * 272);
  // A row short by more than the minute every series lacks owns the rest.
  const own = judgeRow({ ...nifty, rows: 6000 }, { ledger: L, fullest: 6809, support: 3, peers: { n: 3, atMost: 1 } });
  assert.ok(own.notes.includes('store_wide_part'));
  assert.equal(own.sharedMissing, 1);
});

test('a CAS-eligible share owes 360 a session from 2026-08-03 and reads full at 7,560 (DB-G34, D-3320)', () => {
  // 21 auction days in a 30-day month (weekends closed).
  const days = 'XX..XXXXX..XXXXX..XXXXX..XXXX.';
  const L = owedLedger(body({ from: '2026-09-01', days, regular: 375, cas: 360 }));
  const share = row({ instrument: 'NSE-CASH-RELIANCE', month: '2026-09', rows: 21 * 360 });
  // AN ELIGIBLE SHARE, BY ITS DATED FLAG, owes 360 a session and is full at 7,560.
  const c = judgeRow(share, { ledger: L, casFlag: true });
  assert.equal(c.sessions, 21);
  assert.equal(c.auctionDays, 21);
  assert.equal(c.owed, 7560);
  assert.equal(c.owedAlt, null, 'a known flag decides; no other count stands beside it');
  assert.equal(c.state, 'full');
  assert.ok(c.notes.includes('cas'));
  // An index on the same days owes 375: it has no auction phase.
  const index = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2026-09', rows: 7875 }), { ledger: L });
  assert.equal(index.owed, 7875);
  assert.equal(index.state, 'full');
  assert.ok(!index.notes.includes('cas'));
  // A share at the ineligible count is full under either flag.
  assert.equal(judgeRow({ ...share, rows: 7875 }, { ledger: L }).state, 'full');
  // Short of the eligible count is short.
  const short = judgeRow({ ...share, rows: 7560 - 360 }, { ledger: L, casFlag: true });
  assert.equal(short.missing, 360);
  assert.equal(short.state, 'near', 'one session short is under one session of 375');
});

test('a share on auction days is judged by its dated flag, and a flag the census lacks is never "full" between the counts (SC-CLOSE-17)', () => {
  const days = 'XX..XXXXX..XXXXX..XXXXX..XXXX.';
  const L = owedLedger(body({ from: '2026-09-01', days, regular: 375, cas: 360 }));
  const share = row({ instrument: 'NSE-CASH-ZZFLAGZERO', month: '2026-09', rows: 21 * 360 });
  // BEFORE: every share on an auction day owed 360, so a flag-0 share holding
  // 360 a session read "full" though it owes 375 and lacks 15 a session.
  // AFTER, FLAG 0: owed the regular count, short by 21 x 15.
  const flagZero = judgeRow(share, { ledger: L, casFlag: false });
  assert.equal(flagZero.owed, 7875);
  assert.equal(flagZero.missing, 315);
  assert.equal(flagZero.state, 'near');
  assert.ok(!flagZero.notes.includes('cas'), 'an ineligible share is not judged at the auction count');
  // AFTER, NO FLAG (the census carries none): neither verdict is claimed.
  const unread = judgeRow(share, { ledger: L });
  assert.equal(unread.state, 'unmeasured');
  assert.equal(unread.missing, null);
  assert.equal(unread.ratio, null, 'no percentage for a count the flag decides');
  assert.equal(unread.owedAlt, 7875, 'the ineligible count is named beside the eligible one');
  assert.ok(unread.notes.includes('cas_unresolved'));
  assert.notEqual(unread.state, 'full');
  // Under both counts it is short whatever the flag, by at least the eligible shortfall.
  const under = judgeRow({ ...share, rows: 7000 }, { ledger: L });
  assert.equal(under.missing, 560);
  assert.ok(under.notes.includes('cas_unresolved'));
  assert.equal(under.state, 'gap');
  // Which count each kind of row owes.
  assert.deepEqual(owedKind('CASH', true, null), { owed: 'cas', unknown: ['unknownCas', 'unknownShare'], unheld: 'unheldCas', cas: true, unresolved: true });
  assert.equal(owedKind('CASH', true, false).owed, 'share');
  assert.equal(owedKind('CASH', false, null).owed, 'share', 'no auction day, nothing to resolve');
  assert.equal(owedKind('FNO', true, null).owed, 'deriv');
  assert.equal(owedKind('INDEX', true, true).owed, 'index', 'an index has no auction phase, whatever the flag');
  // The roll-up keeps an unresolved month out of every sum.
  const month = rollUpMonths([{ ...share, cover: unread }])[0];
  assert.equal(month.unmeasured, 1);
  assert.equal(month.owed, 0);
  assert.equal(month.pct, null);
});

test('a future is owed the derivatives venue\'s own count, era by era, never an index\'s (SC-CLOSE-17)', () => {
  // 2026-07-30 .. 2026-08-05: Thu, Fri regular; weekend; Mon 3rd onward auction days.
  const days = 'RR..XXX';
  const withDerivs = {
    ...body({ from: '2026-07-30', days, regular: 375, cas: 360 }),
    owed: {
      rung: '1min',
      regular: 375,
      casEligible: 360,
      derivatives: [
        { from: '2018-01-01', to: '2026-08-02', regular: 375, tradeDriven: false },
        { from: '2026-08-03', to: '2026-10-04', regular: 385, tradeDriven: false }
      ]
    }
  };
  const L = owedLedger(withDerivs);
  const fut = row({ instrument: 'NSE-FNO-NIFTY-2026-08-27-FUT', month: '2026-07', rows: 0 });
  // July's two days owe 375 each; August's first three owe 385 each (the 15:40 close).
  const july = judgeRow({ ...fut, rows: 750 }, { ledger: L });
  assert.equal(july.owed, 750);
  assert.equal(july.state, 'full');
  const aug = judgeRow({ ...fut, month: '2026-08', rows: 3 * 375 }, { ledger: L });
  assert.equal(aug.owed, 3 * 385, 'BEFORE it owed the index count, 3 x 375, and read full');
  assert.equal(aug.missing, 30);
  assert.equal(aug.state, 'near');
  // An index on the same days still owes 375.
  assert.equal(judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2026-08', rows: 1125 }), { ledger: L }).owed, 1125);
  // An answer that serves no derivatives row judges no future: named, never the index count.
  const old = judgeRow({ ...fut, month: '2026-08', rows: 1125 }, { ledger: owedLedger(body({ from: '2026-07-30', days, regular: 375, cas: 360 })) });
  assert.equal(old.basis, 'none');
  assert.match(old.why, /derivatives/);
  // A special day owes a future no derivatives count: unknown, never the index's.
  const special = owedLedger({
    ...withDerivs,
    days: 'RRP',
    special: [{ day: epochDayOfIso('2026-08-01'), code: 'P', owed: 60, indexOwed: 60 }]
  });
  const s = judgeRow({ ...fut, month: '2026-08', rows: 60 }, { ledger: special });
  assert.equal(s.unknownDays, 1);
  assert.equal(s.owed, 0, 'never the 60 an index owes that Saturday');
  assert.equal(s.state, 'unmeasured');
  assert.ok(s.notes.includes('unknown_days'));
  // At one second a future is trade-driven: the derivatives figure there is a ceiling.
  assert.equal(isTradeDriven('1s', 'FNO', null), true);
  assert.equal(judgeRow({ ...fut, timeframe: '1s', rows: 9 }, { ledger: L }).basis, 'trade');
});

test('a malformed derivatives era is refused, never skipped; an era with no count owes nothing', () => {
  assert.equal(derivativeEras(undefined), null);
  assert.throws(() => derivativeEras('375'), /not a list/);
  assert.throws(() => derivativeEras([{ from: '2026-08-03', to: '2026-08-01', regular: 385 }]), /malformed era/);
  assert.throws(() => derivativeEras([{ from: 20000, to: '2026-08-01' }]), /malformed era/);
  const eras = derivativeEras([
    { from: '2026-08-03', to: '2026-10-04', regular: 385 },
    { from: '2018-01-01', to: '2026-08-02', regular: null, why: 'no verified hours' }
  ]);
  assert.deepEqual(eras?.map((e) => e.regular), [null, 385], 'sorted by their first day');
  const L = owedLedger({
    ...body({ from: '2026-07-31', days: 'R..R', regular: 375 }),
    owed: { rung: '1min', regular: 375, casEligible: null, derivatives: [{ from: '2026-08-03', to: '2026-10-04', regular: 385 }] }
  });
  const span = ledgerSpan(L, L.from, L.to);
  assert.equal(span.deriv, 385, 'the day before the first era owes a future nothing');
  assert.equal(span.unknownDeriv, 1, 'and is named unknown');
});

test('a_surplus_day_never_hides_a_short_day: a session held with no bar is missing whatever the other days hold (R3-04, DB-014)', () => {
  // 2022-03: 21 sessions at one second; the 7th (a Monday) is held with no bar.
  // The 1st (Mahashivratri) and the 18th (Holi) closed, weekends closed.
  const codes = '.RRR..RRRRR..RRRR...RRRRR..RRRR';
  const held = codes
    .split('')
    .map((c, i) => (c === '.' ? '0' : i === 6 ? '0' : '1'))
    .join('');
  const own = owedLedger(
    body({ from: '2022-03-01', days: codes, regular: 22500, rung: '1s', held, series: { symbol: 'BANKNIFTY', segment: 'INDEX' } })
  );
  const exchange = owedLedger(body({ from: '2022-03-01', days: codes, regular: 22500, rung: '1s' }));
  const sessions = codes.split('').filter((c) => c === 'R').length;
  assert.equal(sessions, 21);
  // Twenty held days, each with 1,125 seconds more than a session owes: the
  // month total reaches what 21 sessions owe, though one session has nothing.
  const total = 20 * (22500 + 1125);
  assert.equal(total, 21 * 22500);
  const bank = row({ instrument: 'NSE-INDEX-BANKNIFTY', month: '2022-03', timeframe: '1s', rows: total });
  // BEFORE: the month total covered the hole — "full", 100.00%.
  const blind = judgeRow(bank, { ledger: exchange });
  assert.equal(Math.max(0, /** @type {number} */ (blind.owed) - bank.rows), 0, 'the month total sees nothing missing');
  // AFTER: the 7th's 22,500 seconds are missing whatever the other days hold.
  const c = judgeRow(bank, { ledger: exchange, held: own });
  assert.equal(c.missing, 22500);
  assert.equal(c.state, 'gap', 'a whole session missing is a gap, never full');
  assert.ok(c.ratio !== null && c.ratio < 1);
  assert.equal(coverPct(c.ratio), '95.23%');
  assert.ok(c.notes.includes('unheld_sessions'));
  assert.equal(c.heldSessions, 20);
  // The ledger's own floor: what the unheld session owes, in every kind.
  const span = ledgerSpan(own, epochDayOfIso('2022-03-01'), epochDayOfIso('2022-03-31'));
  assert.equal(span.unheldIndex, 22500);
  assert.equal(span.unheldShare, 22500);
  // A `-` digit (not read) owes no floor.
  const unread = owedLedger(body({ from: '2022-03-01', days: codes, regular: 22500, rung: '1s', held: held.replace(/0/g, '-'), series: { symbol: 'BANKNIFTY', segment: 'INDEX' } }));
  assert.equal(ledgerSpan(unread, unread.from, unread.to).unheldIndex, 0);
  // A SURPLUS NO MONTH TOTAL CAN PLACE, with no held day read: never full.
  const over = judgeRow({ ...bank, rows: 21 * 22500 + 400 }, { ledger: exchange });
  assert.equal(over.state, 'over');
  assert.equal(over.missing, null, 'nothing is known missing, and nothing is known whole');
  assert.equal(over.ratio, null);
  assert.equal(over.extra, 400);
  assert.ok(over.notes.includes('surplus_unplaced'));
  // ...and the month it stands in is "over", never "full".
  const month = rollUpMonths([
    { ...bank, cover: over },
    { ...bank, instrument: 'NSE-INDEX-NIFTY', rows: 21 * 22500, cover: judgeRow({ ...bank, rows: 21 * 22500 }, { ledger: exchange }) }
  ])[0];
  assert.equal(month.over, 1);
  assert.equal(month.state, 'over');
  assert.deepEqual(monthVerdict({ n: 1, owed: 0, bars: 0, missing: 0, trade: 0, unmeasured: 0, absent: 0, over: 1 }), { pct: null, state: 'over' });
});

test('a_month_missing_one_bar_is_never_100_percent (R3-04, DB-014)', () => {
  const days = '.RRR..RRRRR..RRRR...RRRRR..RRRR';
  const L = owedLedger(body({ from: '2022-03-01', days, regular: 22500, rung: '1s' }));
  const r = row({ instrument: 'NSE-INDEX-NIFTY', month: '2022-03', timeframe: '1s', rows: 21 * 22500 - 1 });
  const c = judgeRow(r, { ledger: L });
  assert.equal(c.missing, 1);
  assert.equal(c.state, 'near');
  // BEFORE: two decimals rounded half up, so one bar of 4,72,500 read 100.00%.
  assert.equal(`${(/** @type {number} */ (c.ratio) * 100).toFixed(2)}%`, '100.00%');
  // AFTER: rounded down.
  assert.equal(coverPct(c.ratio), '99.99%');
  assert.equal(coverPct(1), '100.00%');
  assert.equal(coverPct(0.29), '29.00%', 'the float error of 0.29 x 10,000 is absorbed');
  assert.equal(coverPct(0.00001), '<0.01%', 'something held never reads as nothing');
  assert.equal(coverPct(0), '0.00%');
  assert.equal(coverPct(null), '—');
  assert.equal(coverPct(Number.NaN), '—');
  assert.equal(coverPct(1 - 1e-15), '99.99%', 'a ratio under 1 never reaches 100.00%');
  // The month card rounds the same way and is never "full".
  const card = rollUpMonths([{ ...r, cover: c }])[0];
  assert.equal(card.state, 'near');
  assert.equal(coverPct(card.pct), '99.99%');
});

test('NIFTY 1s 2018-09 is 98.81% of 18 x 22,500, not "full 100.00%" (DB-014)', () => {
  // 18 sessions in September 2018 from the 3rd (the archive's first day).
  const days = 'RRRRR..RRRR...RRRR..RRRR..R...';
  const L = owedLedger(body({ from: '2018-09-03', days, regular: 22500, rung: '1s' }));
  const r = row({ instrument: 'NSE-INDEX-NIFTY', month: '2018-09', timeframe: '1s', rows: 400200 });
  const c = judgeRow(r, { ledger: L });
  assert.equal(c.sessions, 18);
  assert.equal(c.owed, 405000);
  assert.equal(c.missing, 4800);
  assert.equal(c.ratio?.toFixed(4), '0.9881');
  assert.equal(c.state, 'near', '4,800 missing bars are less than one 22,500-bar session');
});

test('trade-driven series owe no count: a one-second share or an option at any rung (DB-G63, D-3336 §2)', () => {
  const L = owedLedger(body({ from: '2019-06-03', days: 'RRRRR..RRRRR..RRRRR..RRRRR', regular: 22500, rung: '1s' }));
  for (const r of [
    row({ instrument: 'NSE-FNO-NIFTY-2023-12-28-1270000-PE', month: '2019-06', timeframe: '1s', rows: 11, strike: 1270000, expiry: '2023-12-28' }),
    row({ instrument: 'NSE-CASH-RELIANCE', month: '2019-06', timeframe: '1s', rows: 200000 })
  ]) {
    const c = judgeRow(r, { ledger: L, fullest: 405647, support: 9 });
    assert.equal(c.basis, 'trade');
    assert.equal(c.state, 'trade');
    assert.equal(c.owed, null, 'never 0.0065% of an index every second');
    assert.equal(c.missing, null);
    assert.equal(c.sessions, 20);
    assert.ok(c.notes.includes('trade_driven'));
  }
  // A MONTH A TRADE-DRIVEN SERIES DOES NOT HOLD is a month nothing traded, not a hole.
  const quiet = judgeRow(
    row({ instrument: 'NSE-FNO-NIFTY-2023-12-28-1270000-PE', month: '2019-06', timeframe: '1s', rows: 0, strike: 1270000, absent: true }),
    { ledger: L }
  );
  assert.equal(quiet.basis, 'trade');
  assert.equal(quiet.state, 'absent');
  assert.equal(quiet.missing, null, 'owed nothing, so missing nothing');
  assert.ok(quiet.notes.includes('no_trade_month'));
  const hole = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2019-06', timeframe: '1s', rows: 0, absent: true }), { ledger: L });
  assert.equal(hole.missing, 450000, 'an index month with no file misses everything owed');
  assert.ok(!hole.notes.includes('no_trade_month'));
  assert.equal(isTradeDriven('1s', 'INDEX', null), false, 'an index is computed every second');
  assert.equal(isTradeDriven('1min', 'FNO', 1270000), true);
  assert.equal(isTradeDriven('1min', 'FNO', null), false, 'a future is not an option');
  assert.equal(segmentOfKey('NSE-FNO-NIFTY-2023-12-28-1270000-PE'), 'FNO');
});

test("a series' first month is owed from its first bar, a halt's minutes neither owed nor missing", () => {
  // March 2020: the 13th is a suspected halt day owing 316 minutes.
  const days = '.RRRRR..RRRRH..RRRRR..RRRR...RR';
  const L = owedLedger(
    body({
      from: '2020-03-01',
      days,
      regular: 375,
      special: [{ day: epochDayOfIso('2020-03-13'), code: 'H', owed: 316, indexOwed: 316 }]
    })
  );
  const full = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2020-03', rows: 21 * 375 - 59 }), { ledger: L });
  assert.equal(full.owed, 21 * 375 - 59);
  assert.equal(full.state, 'full');
  assert.ok(full.notes.includes('halt'));
  // A share listed on the 16th owes from the 16th only, and says so.
  const listed = judgeRow(row({ instrument: 'NSE-CASH-NEWCO', month: '2020-03', rows: 11 * 375 }), {
    ledger: L,
    seriesFirstDay: epochDayOfIso('2020-03-16')
  });
  assert.equal(listed.sessions, 11);
  assert.equal(listed.state, 'full');
  assert.ok(listed.notes.includes('series_start'));
});

test('a day of unknown length is owed nothing and named; an outage day has no index count (D-0420)', () => {
  const days = 'RRUR';
  const L = owedLedger(
    body({
      from: '2021-02-22',
      days: 'RRPR',
      regular: 375,
      special: [{ day: epochDayOfIso('2021-02-24'), code: 'P', owed: 220, indexOwed: null }]
    })
  );
  const index = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-02', rows: 3 * 375 }), { ledger: L });
  assert.equal(index.owed, 3 * 375, 'the outage day owes the index no count');
  assert.equal(index.unknownDays, 1);
  assert.ok(index.notes.includes('unknown_days'));
  const share = judgeRow(row({ instrument: 'NSE-CASH-AXISBANK', month: '2021-02', rows: 3 * 375 + 220 }), { ledger: L });
  assert.equal(share.owed, 3 * 375 + 220);
  assert.equal(share.state, 'full');
  const U = owedLedger(body({ from: '2021-02-22', days, regular: 375 }));
  assert.equal(judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-02', rows: 1125 }), { ledger: U }).unknownDays, 1);
});

test('no ledger is "not measured" with its reason, never a verdict', () => {
  const c = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2024-11', rows: 1875 }), {
    ledger: null,
    ledgerWhy: 'the server is still building this calendar'
  });
  assert.equal(c.basis, 'none');
  assert.equal(c.state, 'unmeasured');
  assert.equal(c.owed, null);
  assert.match(c.why, /still building/);
});

test('a derived rung behind or short of its 1min source says so (DB-013, DB-G12, DB-G35)', () => {
  const days = 'RR..RRRRR..RRRRR..RRRRR..RRRR.';
  const one = owedLedger(body({ from: '2026-09-01', days, regular: 375 }));
  const five = owedLedger(body({ from: '2026-09-01', days, regular: 75, rung: '5min' }));
  const ms = (/** @type {string} */ iso) => Date.parse(`${iso}T09:59:00Z`);
  const src = row({ instrument: 'NSE-INDEX-NIFTY', month: '2026-09', rows: 21 * 375, lastAt: ms('2026-09-30') });
  const srcCover = judgeRow(src, { ledger: one });
  const derived = row({ instrument: 'NSE-INDEX-NIFTY', month: '2026-09', timeframe: '5min', rows: 4 * 75, lastAt: ms('2026-09-04') });
  const c = judgeRow(derived, {
    ledger: five,
    derived: true,
    source: { rows: src.rows, lastAt: src.lastAt, cover: srcCover },
    rungMs: 300_000
  });
  assert.ok(c.notes.includes('derived'));
  assert.ok(c.notes.includes('derived_behind'));
  assert.ok(c.notes.includes('derived_partial'));
  assert.equal(c.owed, 21 * 75);
  assert.equal(derivedPartial(c, srcCover), true);
  const caught = judgeRow({ ...derived, rows: 21 * 75, lastAt: ms('2026-09-30') }, {
    ledger: five,
    derived: true,
    source: { rows: src.rows, lastAt: src.lastAt, cover: srcCover },
    rungMs: 300_000
  });
  assert.ok(!caught.notes.includes('derived_behind'));
  assert.ok(!caught.notes.includes('derived_partial'));
  assert.ok(judgeRow(derived, { ledger: five, derived: true, source: null }).notes.includes('derived_no_source'));
  // DAYS HELD CANNOT SEE A WITHHELD BUCKET (DB-G36): 360ONE 60min 2023-02
  // holds 126 of 140 hourly buckets on all 20 days while its 1min month misses
  // 30 of 7,500 minutes — 840 minutes withheld against 30 lost.
  const cover = (/** @type {number} */ owed, /** @type {number} */ missing) =>
    /** @type {any} */ ({ owed, missing, heldSessions: 20 });
  assert.equal(derivedPartial(cover(140, 14), cover(7500, 30)), true);
  assert.equal(derivedPartial(cover(1500, 6), cover(7500, 30)), false, 'within one bucket of what the source lost');
  assert.equal(derivedPartial(cover(1500, 8), cover(7500, 30)), true);
  assert.equal(derivedPartial(/** @type {any} */ ({ owed: null, missing: null }), cover(7500, 30)), false);
});

test('absent months between a series\' two ends are listed and owed in full (DB-012)', () => {
  assert.deepEqual(absentMonths('2020-10', '2021-01', new Set(['2020-10', '2021-01'])), ['2020-11', '2020-12']);
  assert.deepEqual(absentMonths('2024-12', '2025-01', new Set(['2024-12', '2025-01'])), []);
  assert.deepEqual(absentMonths('', '2025-01', new Set()), []);
  const L = owedLedger(body({ from: '2023-10-01', days: '.RRRRR..RRRRR..RRRRR..RRRRR..RR', regular: 375 }));
  const c = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2023-10', rows: 0, absent: true }), { ledger: L });
  assert.equal(c.state, 'absent');
  assert.equal(c.missing, c.owed);
  assert.equal(c.ratio, 0);
});

test('a month outside the operator window is marked, and judged all the same (DB-052)', () => {
  const L = owedLedger(body({ from: '2015-01-01', days: 'RR..RRRRR', regular: 1, rung: '1day' }));
  const c = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2015-01', timeframe: '1day', rows: 7 }), {
    ledger: L,
    window: { from: '2018-01-01', to: '2026-09-30' }
  });
  assert.ok(c.notes.includes('outside_window'));
  assert.ok(c.notes.includes('outside_span'), 'January past the 9th is outside this ledger');
  assert.equal(c.state, 'full');
});

test('the month roll-up sums calendar owed per row, order-independent, and leaves trade rows unmeasured', () => {
  const L = owedLedger(body({ from: '2021-08-02', days: 'RRRRR', regular: 375 }));
  const D = owedLedger(body({ from: '2021-08-02', days: 'RRRRR', regular: 1, rung: '1day' }));
  const minute = row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-08', rows: 5 * 375 });
  const daily = row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-08', timeframe: '1day', rows: 5 });
  const items = [
    { ...minute, cover: judgeRow(minute, { ledger: L }) },
    { ...daily, cover: judgeRow(daily, { ledger: D }) }
  ];
  const a = rollUpMonths(items)[0];
  const b = rollUpMonths([...items].reverse())[0];
  assert.deepEqual(a, b);
  assert.equal(a.owed, 5 * 375 + 5);
  assert.equal(a.state, 'full');
  assert.equal(a.pct, 1);
  const opt = row({ instrument: 'NSE-FNO-NIFTY-2021-08-26-1600000-CE', month: '2021-08', timeframe: '1s', rows: 9, strike: 1600000 });
  const t = rollUpMonths([{ ...opt, cover: judgeRow(opt, { ledger: L }) }])[0];
  assert.equal(t.state, 'trade');
  assert.equal(t.pct, null);
  assert.deepEqual(monthVerdict({ n: 2, owed: 750, bars: 375, missing: 375, trade: 0, unmeasured: 0, absent: 1 }), { pct: 0.5, state: 'gap' });
});

test('the peer helpers that stay are corroboration only', () => {
  const rows = [
    { month: '2024-11', timeframe: '1min', rows: 1875 },
    { month: '2024-11', timeframe: '1min', rows: 1800 }
  ];
  const d = denominators(rows);
  assert.equal(d.support.get(denomKey('2024-11', '1min')), 2);
  assert.equal(isSole(d.support, rows[0]), false);
  assert.equal(istDayOf(Date.parse('2024-11-04T18:29:59Z')), epochDayOfIso('2024-11-04'));
  assert.equal(istDayOf(Date.parse('2024-11-04T18:30:00Z')), epochDayOfIso('2024-11-05'));
});

test("the store's reach bounds the month; a series that stops short of it owes the rest; one that ended earlier says so", () => {
  // 2026-09: 21 regular sessions (weekends closed), the exchange's calendar.
  const days = 'RR..RRRRR..RRRRR..RRRRR..RRRR.';
  const exchange = owedLedger(body({ from: '2026-09-01', days, regular: 375 }));
  // NIFTY's own calendar at this rung holds the days it holds, to its last bar (the 4th).
  const mine = owedLedger(
    body({ from: '2026-09-01', days: days.slice(0, 4), regular: 375, held: '1100', series: { symbol: 'NIFTY', segment: 'INDEX' } })
  );
  const at = (/** @type {string} */ iso) => epochDayOfIso(iso);
  const stale = row({ instrument: 'NSE-INDEX-NIFTY', month: '2026-09', rows: 2 * 375 });
  // BEFORE: measured over the series' own calendar alone, a pull that stopped on the 2nd read full.
  const own = judgeRow(stale, { ledger: mine });
  assert.equal(own.state, 'full', 'the series-span calendar cannot see the tail');
  // AFTER: the exchange owes the month to the store's reach (the 30th); the series' digits count its days.
  const c = judgeRow(stale, { ledger: exchange, held: mine, seriesLastDay: at('2026-09-02'), storeLastDay: at('2026-09-30') });
  assert.equal(c.sessions, 21);
  assert.equal(c.owed, 21 * 375);
  assert.equal(c.missing, 19 * 375);
  assert.equal(c.state, 'gap');
  assert.equal(c.heldSessions, 2, 'two days held, counted from its own digits');
  assert.ok(!c.notes.includes('series_end'), 'the store reaches this month: the tail is a hole');
  // GDFL's archive ends on the 24th: the days after it are not owed.
  const archive = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2026-09', rows: 17 * 375 }), {
    ledger: exchange,
    seriesLastDay: at('2026-09-24'),
    storeLastDay: at('2026-09-24')
  });
  assert.equal(archive.sessions, 17);
  assert.equal(archive.state, 'full');
  // A series whose last month is earlier than the store's last month ENDED: measured to its last bar, and named.
  const ended = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2026-09', rows: 2 * 375 }), {
    ledger: exchange,
    seriesLastDay: at('2026-09-02'),
    storeLastDay: at('2026-10-30')
  });
  assert.equal(ended.sessions, 2);
  assert.equal(ended.state, 'full');
  assert.ok(ended.notes.includes('series_end'));
});

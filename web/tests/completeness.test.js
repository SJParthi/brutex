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
import { owedLedger, epochDayOfIso, ledgerSpan, derivativeEras, foldMinuteOwed, denseOwed, ledgerDay, readLedger, askCalendar, readCalendar, emptyCalendar, withheldDays, isoOfEpochDay, monthsBetween, createCalendarLoader } from '../src/lib/calendar-owed.js';

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
  // EVERY SESSION HELD: no floor, and nothing is named for it — a surplus
  // month with its own days read is `over`, never "short of a session".
  const allHeld = owedLedger(
    body({ from: '2022-03-01', days: codes, regular: 22500, rung: '1s', held: held.replace(/^(.{6})0/, '$11'), series: { symbol: 'BANKNIFTY', segment: 'INDEX' } })
  );
  const surplus = judgeRow({ ...bank, rows: 21 * 22500 + 60 }, { ledger: exchange, held: allHeld });
  assert.equal(surplus.state, 'over');
  assert.ok(!surplus.notes.includes('unheld_sessions'), 'no session with no bar, so none is named');
  assert.equal(surplus.heldSessions, 21);
  // Short by the month total more than the floor: the total decides, the floor is not named.
  const totalShort = judgeRow({ ...bank, rows: 19 * 22500 }, { ledger: exchange, held: own });
  assert.equal(totalShort.missing, 2 * 22500);
  assert.ok(!totalShort.notes.includes('unheld_sessions'));
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
  assert.equal(isTradeDriven('1min', 'FNO', undefined), false, 'nor is a contract whose strike was never read');
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

/* ======================================================================
   THE EDGES (R3-04): every absent, malformed or exhausted input the ledger,
   its reader and the verdict meet is named or left absent, never guessed.
   ====================================================================== */

/** A response as `fetch` hands it back, for the reader's own loop. */
const answer = (/** @type {number} */ status, /** @type {any} */ json) => /** @type {any} */ ({ ok: status >= 200 && status < 300, status, json: async () => json });

test('the ledger reads absent fields as absent, and refuses a special or halt entry with no day', () => {
  const from = epochDayOfIso('2024-01-01');
  const add = (/** @type {string} */ iso, /** @type {number} */ n) => new Date(Date.parse(`${iso}T00:00:00Z`) + n * 86_400_000).toISOString().slice(0, 10);
  for (const [a, b] of [['', '2024-01-02'], ['2024-01-01', ''], ['2024-01-03', '2024-01-02']]) {
    assert.equal(foldMinuteOwed(a, b, new Map([['2024-01-02', 375]]), new Map(), add).exchange.size, 0, `${a}..${b} folds nothing`);
  }
  const dense = denseOwed({ days: 'R', from, minuteOwed: { regular: '375' }, special: 'not a list' });
  assert.equal(dense.owed.get('2024-01-01'), null, 'a regular count that is not a whole number is unknown, never 375');
  const L = owedLedger({ days: 'RH', from, owed: { regular: 375 }, special: 'x', halts: [{ day: from + 1 }], heldBy: 'x', target: null, feed: 7, rung: null });
  assert.equal(L.halts.size, 1);
  assert.throws(() => owedLedger({ days: 'RH', from, halts: [{ day: from + 1 }, { day: 'x' }] }), /`halts` carries an entry with no day: \{"day":"x"\}/);
  assert.throws(() => owedLedger({ days: 'RH', from, special: [null] }), /`special` carries an entry with no day: null/);
  assert.deepEqual(L.heldBy, []);
  assert.equal(L.series, null);
  assert.equal(L.feed, '');
  assert.equal(L.rung, '');
  assert.equal(L.derivatives, null, 'no derivatives row served');
  assert.equal(ledgerDay(L, from)?.held, null, 'no held string, no digit');
  assert.equal(ledgerDay(L, from)?.special, null);
  assert.equal(ledgerDay(L, from - 1), null, 'outside the span');
  const H = owedLedger({ days: 'RR', from, held: '-1', owed: { regular: 375 } });
  assert.equal(ledgerDay(H, from)?.held, null, 'a day nobody read');
  assert.equal(ledgerDay(H, from + 1)?.held, 1);
  assert.throws(() => owedLedger({ days: 'RR', from, held: '1' }), /held string is 1 days against 2/);
});

test('the reader names a calendar still building or withheld without a reason, rethrows an abort, and names a thrown non-error', async () => {
  const signal = new AbortController().signal;
  const building = await readLedger('/calendar.json', async () => answer(200, { status: 'building' }), signal);
  assert.equal(building.ledger, null);
  assert.match(building.why, /still building this calendar: no reason given/);
  // An answer with no body is a calendar with no day string, said as such — never a TypeError.
  assert.equal((await readLedger('/calendar.json', async () => answer(200, null), signal)).why, 'the calendar could not be read (/calendar.json carried no day string)');
  const withheld = await readLedger('/calendar.json', async () => answer(200, { status: 'withheld' }), signal);
  assert.match(withheld.why, /withheld: no reason given/);
  const thrown = await readLedger('/calendar.json', async () => { throw 'a string, not an Error'; }, signal);
  assert.match(thrown.why, /could not be read \(a string, not an Error\)/);
  const aborted = new AbortController();
  aborted.abort(new Error('left the page'));
  await assert.rejects(readLedger('/calendar.json', async () => { throw new Error('cut'); }, aborted.signal), /cut/);
  // An older answer: a list of days, or none at all, read as given and never guessed.
  const listed = await readCalendar('zerodha', async () => answer(200, { days: [{ day: epochDayOfIso('2024-01-01') }], firstDay: epochDayOfIso('2024-01-01'), lastDay: epochDayOfIso('2024-01-01') }), signal);
  assert.equal(listed.owed.get('2024-01-01'), null, 'a day with no count is unknown');
  assert.equal(listed.indexOwed.get('2024-01-01'), null);
  const bare = await readCalendar('zerodha', async () => answer(200, {}), signal);
  assert.equal(bare.owed.size, 0);
  assert.equal(bare.first, '');
  assert.equal(bare.last, '');
  assert.equal(bare.why, 'the store holds no bars for this feed');
  // A 202 asked again at the server's hint, then answered.
  let asked = 0;
  const settled = await askCalendar('/calendar.json', async () => (++asked < 3 ? answer(202, { status: 'building', retry_after_ms: 'soon' }) : answer(200, { status: 'ok', days: 'R' })), signal, async () => {});
  assert.equal(asked, 3);
  assert.equal(settled.body.status, 'ok');
});

test('the wait between asks ends on an abort, with the abort\'s reason or a named one', async () => {
  const building = async () => answer(202, { status: 'building', retry_after_ms: 50 });
  /** A signal whose abort the test fires. @param {boolean} aborted @param {unknown} reason */
  const fake = (aborted, reason) => {
    /** @type {(() => void)[]} */
    const heard = [];
    return {
      signal: /** @type {any} */ ({ aborted, reason, addEventListener: (/** @type {string} */ _type, /** @type {() => void} */ fn) => heard.push(fn) }),
      fire: () => heard.forEach((fn) => fn())
    };
  };
  await assert.rejects(askCalendar('/c', building, fake(true, null).signal), /aborted/);
  await assert.rejects(askCalendar('/c', building, fake(true, new Error('gone')).signal), /gone/);
  const later = fake(false, null);
  const waiting = askCalendar('/c', building, later.signal);
  await new Promise((resolve) => setTimeout(resolve, 5));
  later.fire();
  await assert.rejects(waiting, /aborted/);
  const reasoned = fake(false, new Error('reasoned'));
  const pending = askCalendar('/c', building, reasoned.signal);
  await new Promise((resolve) => setTimeout(resolve, 5));
  reasoned.fire();
  await assert.rejects(pending, /reasoned/);
  // The wait listens once: a listener left behind would fire on a later abort.
  /** @type {any[]} */
  const options = [];
  const once = { aborted: false, reason: null, addEventListener: (/** @type {string} */ _t, /** @type {() => void} */ fn, /** @type {any} */ o) => { options.push(o); setTimeout(fn, 1); } };
  await assert.rejects(askCalendar('/c', building, /** @type {any} */ (once)), /aborted/);
  assert.deepEqual(options, [{ once: true }]);
  // With no signal at all the wait simply runs out and the ask repeats.
  let n = 0;
  const done = await askCalendar('/c', async () => (++n < 2 ? answer(202, { status: 'building', retry_after_ms: 50 }) : answer(200, { status: 'ok' })), /** @type {any} */ (undefined));
  assert.equal(done.body.status, 'ok');
});

test('the verdict at its edges: a contract ends at its expiry, a trade-driven series counts its own days, nothing owed is no ratio', () => {
  // An option expiring mid-month: its span ends on the expiry.
  const L = owedLedger(body({ from: '2019-03-01', days: 'R..RRRRR..RRRRR..RRRRR..RRRRR..', regular: 375 }));
  const opt = row({ instrument: 'NSE-FNO-NIFTY-2019-03-28-1160000-CE', month: '2019-03', rows: 40, strike: 1160000, expiry: '2019-03-28' });
  assert.equal(judgeRow(opt, { ledger: L }).spanTo, epochDayOfIso('2019-03-28'));
  // A one-second share with its own held days: the days it holds, exactly.
  const own = owedLedger(body({ from: '2019-06-03', days: 'RRRRR', regular: 22500, rung: '1s', held: '11011', series: { symbol: 'RELIANCE', segment: 'CASH' } }));
  const share = judgeRow(row({ instrument: 'NSE-CASH-RELIANCE', month: '2019-06', timeframe: '1s', rows: 9000 }), { ledger: own });
  assert.equal(share.basis, 'trade');
  assert.equal(share.heldExact, true);
  assert.equal(share.heldSessions, 4);
  // An absent month in which nothing is owed has no ratio.
  const shut = owedLedger(body({ from: '2020-01-01', days: '.'.repeat(31), regular: 375 }));
  const none = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2020-01', rows: 0, absent: true }), { ledger: shut });
  assert.equal(none.owed, 0);
  assert.equal(none.ratio, null);
  // One session at the span's own average where the ledger has no regular count.
  const odd = owedLedger({ ...body({ from: '2021-11-04', days: 'P', regular: 375 }), owed: { rung: '1min', regular: null, casEligible: null }, special: [{ day: epochDayOfIso('2021-11-04'), code: 'P', owed: 60, indexOwed: 60 }] });
  const short = judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-11', rows: 30 }), { ledger: odd });
  assert.equal(short.owed, 60);
  assert.equal(short.state, 'near', '30 of a 60-minute session is under one session');
  assert.equal(judgeRow(row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-11', rows: 0 }), { ledger: odd }).state, 'gap');
  // The peer helpers on nothing.
  assert.equal(denominators(null).fullest.size, 0);
  assert.equal(isSole(new Map(), row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-11', rows: 1 })), true);
  assert.equal(segmentOfKey('NIFTY'), '');
  assert.equal(derivedPartial(/** @type {any} */ ({ owed: 0, missing: 0 }), /** @type {any} */ ({ owed: 375, missing: 0 })), false);
});

test('the roll-up counts absent and store-wide rows, breaks a tie on the row itself, and an all-absent month is absent', () => {
  assert.deepEqual(rollUpMonths(null), []);
  const L = owedLedger(body({ from: '2021-08-02', days: 'RRRRR', regular: 375 }));
  const a = row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-08', rows: 0, absent: true });
  const absent = rollUpMonths([{ ...a, cover: judgeRow(a, { ledger: L }) }])[0];
  assert.equal(absent.absent, 1);
  assert.equal(absent.state, 'absent');
  const short = row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-08', rows: 1800 });
  const part = judgeRow(short, { ledger: L, fullest: 1850, support: 3, peers: { n: 3, atMost: 1 } });
  assert.ok(part.notes.includes('store_wide_part'));
  const tie = { month: '2021-08', timeframe: '1min', rows: 1800, cover: part };
  const rolled = rollUpMonths([{ ...short, cover: part }, tie])[0];
  assert.equal(rolled.storeWide, 2);
  // A tie is broken on the rows' own identity (a row with no instrument reads as an empty name), never on arrival order.
  assert.equal(rolled.worst.instrument, 'NSE-INDEX-NIFTY');
  assert.equal(rollUpMonths([tie, { ...short, cover: part }])[0].worst.instrument, 'NSE-INDEX-NIFTY');
});

/* ======================================================================
   THE VERDICT PINNED (R3-04): each boundary a surviving mutant found,
   asserted. March 2021 at one minute: 23 weekday sessions.
   ====================================================================== */

/** Day codes for `n` days from `iso`: weekdays `R`, weekends `.`. */
function weekdays(/** @type {string} */ iso, /** @type {number} */ n) {
  const d0 = epochDayOfIso(iso);
  return Array.from({ length: n }, (_, i) => ([0, 6].includes(new Date((d0 + i) * 86_400_000).getUTCDay()) ? '.' : 'R')).join('');
}
const MAR = weekdays('2021-03-01', 31);
const MAR_L = owedLedger(body({ from: '2021-03-01', days: MAR, regular: 375 }));
const at = (/** @type {string} */ iso) => epochDayOfIso(iso);
const nifty = (/** @type {any} */ r = {}) => row({ instrument: 'NSE-INDEX-NIFTY', month: '2021-03', rows: 23 * 375, ...r });

test('the keys, the fullest peer and the segment read exactly', () => {
  assert.equal(denomKey('2024-11', '1min'), '2024-11\u00001min');
  assert.notEqual(denomKey('2024-1', '1min'), denomKey('2024-11', 'min'), 'the separator keeps two keys apart');
  for (const order of [[1875, 1800], [1800, 1875]]) {
    const d = denominators(order.map((rows) => ({ month: '2024-11', timeframe: '1min', rows })));
    assert.equal(d.fullest.get(denomKey('2024-11', '1min')), 1875);
  }
  assert.equal(segmentOfKey('NSE-NIFTY'), '');
  assert.deepEqual(owedKind('FNO', false, null), { owed: 'deriv', unknown: ['unknownDeriv'], unheld: 'unheldDeriv', cas: false, unresolved: false });
  assert.deepEqual(owedKind('INDEX', false, null), { owed: 'index', unknown: ['unknownIndex'], unheld: 'unheldIndex', cas: false, unresolved: false });
  assert.deepEqual(owedKind('CASH', false, null), { owed: 'share', unknown: ['unknownShare'], unheld: 'unheldShare', cas: false, unresolved: false });
  assert.deepEqual(owedKind('CASH', true, true), { owed: 'cas', unknown: ['unknownCas'], unheld: 'unheldCas', cas: true, unresolved: false });
});

test('a whole month carries no note, no reason and its own average session', () => {
  const c = judgeRow(nifty(), { ledger: MAR_L });
  assert.deepEqual(c.notes, []);
  assert.equal(c.why, '');
  assert.equal(c.state, 'full');
  assert.equal(c.sessions, 23);
  assert.equal(c.heldExact, false);
  assert.equal(c.heldSessions, 23);
  assert.equal(c.ratio, 1);
  assert.equal(judgeRow(nifty({ rows: 10 * 375 }), { ledger: MAR_L }).heldSessions, 10);
  assert.equal(judgeRow(nifty(), { ledger: null }).why, 'no calendar of record was read for this feed and rung');
});

test('the operator window holds a month its first or last day touches, and no other', () => {
  const note = (/** @type {string} */ month, /** @type {{ from: string, to: string }} */ window) =>
    judgeRow(nifty({ month }), { ledger: MAR_L, window }).notes.includes('outside_window');
  assert.equal(note('2021-03', { from: '2021-03-01', to: '2021-03-01' }), false, 'a window ending on the 1st holds the month');
  assert.equal(note('2021-04', { from: '2021-03-01', to: '2021-03-01' }), true);
  assert.equal(note('2021-03', { from: '2021-03-31', to: '2026-09-30' }), false, 'a window starting on the 31st holds the month');
  assert.equal(note('2021-02', { from: '2021-03-31', to: '2026-09-30' }), true);
});

test('the span: a first bar on the 1st is no late start, an expiry on the 1st ends the span there, an end on the last day is no early end', () => {
  const onFirst = judgeRow(nifty(), { ledger: MAR_L, seriesFirstDay: at('2021-03-01') });
  assert.ok(!onFirst.notes.includes('series_start'));
  assert.equal(onFirst.spanFrom, at('2021-03-01'));
  const earlier = judgeRow(nifty(), { ledger: MAR_L, seriesFirstDay: at('2020-01-02') });
  assert.equal(earlier.spanFrom, at('2021-03-01'));
  assert.ok(!earlier.notes.includes('series_start'));
  const opt = row({ instrument: 'NSE-FNO-NIFTY-2021-03-01-1500000-CE', month: '2021-03', rows: 3, strike: 1500000, expiry: '2021-03-01' });
  assert.equal(judgeRow(opt, { ledger: MAR_L }).spanTo, at('2021-03-01'));
  const lastDay = judgeRow(nifty(), { ledger: MAR_L, seriesLastDay: at('2021-03-31'), storeLastDay: at('2021-06-30') });
  assert.ok(!lastDay.notes.includes('series_end'), 'a series ending on the month\'s last day did not end early');
  assert.equal(lastDay.spanTo, at('2021-03-31'));
});

test('the series\' own held days count only when the answer is that series, in that segment, with digits', () => {
  const held = MAR.split('').map((c) => (c === 'R' ? '1' : '0')).join('');
  const own = (/** @type {{ symbol: string, segment: string } | null} */ series, /** @type {string | null} */ digits = held) =>
    owedLedger(body({ from: '2021-03-01', days: MAR, regular: 375, held: digits, series }));
  assert.equal(judgeRow(nifty(), { ledger: MAR_L, held: own({ symbol: 'NIFTY', segment: 'INDEX' }) }).heldExact, true);
  for (const [why, ledger] of /** @type {[string, any][]} */ ([
    ['the exchange\'s answer', own(null)],
    ['another symbol', own({ symbol: 'BANKNIFTY', segment: 'INDEX' })],
    ['another segment', own({ symbol: 'NIFTY', segment: 'CASH' })],
    ['no held digits', own({ symbol: 'NIFTY', segment: 'INDEX' }, null)]
  ])) {
    assert.equal(judgeRow(nifty(), { ledger: MAR_L, held: ledger }).heldExact, false, why);
  }
  // A trade-driven series without its own days counts none, and a month it holds is no quiet month.
  const t = judgeRow(row({ instrument: 'NSE-CASH-ZZTRADE', month: '2021-03', timeframe: '1s', rows: 40 }), { ledger: MAR_L });
  assert.equal(t.heldExact, false);
  assert.equal(t.heldSessions, null);
  assert.ok(!t.notes.includes('no_trade_month'));
});

test('a share outside auction days owes the regular count; one whose two counts agree names no other', () => {
  const share = judgeRow(row({ instrument: 'NSE-CASH-ZZSHARE', month: '2021-03', rows: 23 * 375 }), { ledger: MAR_L });
  assert.ok(!share.notes.includes('cas'));
  assert.equal(share.owedAlt, null);
  assert.equal(share.state, 'full');
  const same = owedLedger(body({ from: '2026-09-01', days: 'XX..XXXXX..XXXXX..XXXXX..XXXX.', regular: 375, cas: 375 }));
  const short = judgeRow(row({ instrument: 'NSE-CASH-ZZSHARE', month: '2026-09', rows: 21 * 375 - 10 }), { ledger: same });
  assert.equal(short.owedAlt, null);
  assert.ok(!short.notes.includes('cas_unresolved'), 'nothing for a flag to decide');
  assert.equal(short.missing, 10);
  // At the ineligible count exactly: full under either flag, nothing unresolved.
  const days = 'XX..XXXXX..XXXXX..XXXXX..XXXX.';
  const L = owedLedger(body({ from: '2026-09-01', days, regular: 375, cas: 360 }));
  const at375 = judgeRow(row({ instrument: 'NSE-CASH-ZZSHARE', month: '2026-09', rows: 21 * 375 }), { ledger: L });
  assert.equal(at375.state, 'full');
  assert.equal(at375.missing, 0);
  assert.ok(!at375.notes.includes('cas_unresolved'));
  // Between the counts with a session held at 0: short by that session whatever the flag.
  const heldDigits = days.split('').map((c, i) => (c === '.' ? '0' : i === 0 ? '0' : '1')).join('');
  const own = owedLedger(body({ from: '2026-09-01', days, regular: 375, cas: 360, held: heldDigits, series: { symbol: 'ZZSHARE', segment: 'CASH' } }));
  const floored = judgeRow(row({ instrument: 'NSE-CASH-ZZSHARE', month: '2026-09', rows: 21 * 365 }), { ledger: L, held: own });
  assert.equal(floored.missing, 360);
  assert.equal(floored.state, 'near');
  assert.ok(floored.notes.includes('cas_unresolved'));
});

test('an average session where nothing sized one, and no count owed at all', () => {
  // Every session of unrecorded length: the ledger's regular count stands in.
  const U = owedLedger(body({ from: '2021-03-01', days: MAR.replace(/R/g, 'U'), regular: 375 }));
  const u = judgeRow(nifty({ rows: 750 }), { ledger: U });
  assert.equal(u.heldSessions, 2);
  assert.equal(u.state, 'unmeasured');
  // A month with an unsized session: the average is over the sized ones, and bars on the
  // unsized day are no surplus (nobody can say what it owed).
  const mixed = owedLedger(body({ from: '2021-03-01', days: 'RUR', regular: 375 }));
  const m = judgeRow(nifty({ rows: 750 }), { ledger: mixed });
  assert.equal(m.heldSessions, 2);
  const extraOnU = judgeRow(nifty({ rows: 750 + 200 }), { ledger: mixed });
  assert.equal(extraOnU.extra, 0);
  assert.equal(extraOnU.state, 'full');
  // No regular count and nothing sized: no session to divide by.
  const none = owedLedger({ ...body({ from: '2021-03-01', days: MAR.replace(/R/g, 'U'), regular: 375 }), owed: { rung: '1min', regular: null } });
  assert.equal(judgeRow(nifty({ rows: 750 }), { ledger: none }).heldSessions, null);
  // A session owed nothing: owed 0, a bar there is over (no surplus to place), none is full.
  const zero = owedLedger({ ...body({ from: '2021-03-06', days: 'P', regular: 375 }), special: [{ day: at('2021-03-06'), code: 'P', owed: 0, indexOwed: 0 }] });
  const over = judgeRow(nifty({ rows: 5 }), { ledger: zero });
  assert.equal(over.owed, 0);
  assert.equal(over.state, 'over');
  assert.equal(over.heldSessions, 5 / 375);
  assert.ok(over.notes.includes('extra'));
  assert.ok(!over.notes.includes('surplus_unplaced'), 'nothing owed, so no day short to hide');
  const nothing = judgeRow(nifty({ rows: 0 }), { ledger: zero });
  assert.equal(nothing.state, 'full');
  assert.equal(nothing.ratio, null, 'nothing owed has no ratio, never NaN');
});

test('the per-day floor is named only where it decides, at the boundary too', () => {
  const held = MAR.split('').map((c, i) => (c === 'R' ? (i === 1 ? '0' : '1') : '0')).join('');
  const own = owedLedger(body({ from: '2021-03-01', days: MAR, regular: 375, held, series: { symbol: 'NIFTY', segment: 'INDEX' } }));
  // A session with no bar AND a surplus on others: the floor is the shortfall, never unknown.
  const both = judgeRow(nifty({ rows: 23 * 375 + 500 }), { ledger: MAR_L, held: own });
  assert.equal(both.missing, 375);
  assert.equal(both.extra, 500);
  assert.ok(both.notes.includes('surplus_unplaced'));
  assert.ok(both.notes.includes('unheld_sessions'));
  assert.equal(both.state, 'gap');
  // Short by exactly the unheld session: the month total already says it.
  const exact = judgeRow(nifty({ rows: 22 * 375 }), { ledger: MAR_L, held: own });
  assert.equal(exact.missing, 375);
  assert.ok(!exact.notes.includes('unheld_sessions'));
});

test('a store-wide hole needs a known shortfall, two series at least, and nine in ten of them', () => {
  const short = nifty({ rows: 23 * 375 - 100 });
  const wide = (/** @type {any} */ r, /** @type {any} */ ctx) => judgeRow(r, { ledger: MAR_L, ...ctx }).notes.filter((n) => n.startsWith('store_wide'));
  assert.deepEqual(wide(nifty({ rows: 23 * 375 + 5 }), { peers: { n: 3, atMost: 3 } }), [], 'an unplaced surplus shares no hole');
  assert.deepEqual(wide(nifty(), { peers: { n: 3, atMost: 3 } }), [], 'a whole month shares no hole');
  assert.deepEqual(wide(short, { support: 2, fullest: 23 * 375 - 100 }), ['store_wide'], 'two series are enough');
  assert.deepEqual(wide(short, { peers: { n: 2, atMost: 2 } }), ['store_wide']);
  assert.deepEqual(wide(short, { peers: { n: 1, atMost: 1 } }), [], 'one series has nothing to share a hole with');
  assert.deepEqual(wide(short, { peers: { n: 10, atMost: 9 } }), ['store_wide'], 'nine in ten is enough');
  assert.deepEqual(wide(short, { peers: { n: 10, atMost: 8 } }), []);
  assert.deepEqual(wide(short, { peers: { n: 10, atMost: 1 }, support: 10, fullest: 23 * 375 }), [], 'the fullest whole: nothing shared');
});

test('a derived month is behind only when its last bar is a whole bucket short of a known source bar', () => {
  const five = owedLedger(body({ from: '2021-03-01', days: MAR, regular: 75, rung: '5min' }));
  const t = Date.parse('2021-03-31T09:55:00Z');
  const behind = (/** @type {number | null} */ lastAt, /** @type {number | null} */ sourceAt) =>
    judgeRow(nifty({ timeframe: '5min', rows: 23 * 75, lastAt }), { ledger: five, derived: true, source: { rows: 1, lastAt: sourceAt }, rungMs: 300_000 }).notes.includes('derived_behind');
  assert.equal(behind(t - 300_000, t), false, 'exactly one bucket is not behind');
  assert.equal(behind(t - 300_001, t), true);
  assert.equal(behind(null, t), false, 'no last bar, nothing to compare');
  assert.equal(behind(t, null), false, 'no source bar, nothing to compare');
  assert.ok(!judgeRow(nifty({ timeframe: '5min', rows: 23 * 75 }), { ledger: five, derived: true, source: { rows: 1, lastAt: null } }).notes.includes('derived_no_source'));
  // An unknown source shortfall is never read as nothing lost.
  assert.equal(derivedPartial(/** @type {any} */ ({ owed: 140, missing: 14 }), /** @type {any} */ ({ owed: 7500, missing: null })), false);
  assert.equal(derivedPartial(/** @type {any} */ ({ owed: 140, missing: null }), /** @type {any} */ ({ owed: 7500, missing: 0 })), false);
  assert.equal(derivedPartial(/** @type {any} */ ({ owed: null, missing: 14 }), /** @type {any} */ ({ owed: 7500, missing: 0 })), false);
  assert.equal(derivedPartial(/** @type {any} */ ({ owed: 140, missing: 14 }), /** @type {any} */ ({ owed: 0, missing: 0 })), false);
  assert.deepEqual(absentMonths('2025-01', '', new Set()), []);
  assert.deepEqual(absentMonths('2025-02', '2025-01', new Set()), []);
});

test('the month roll-up names its rung, counts full and store-wide rows, and keeps the worst row on a tie', () => {
  const minute = nifty();
  const daily = nifty({ timeframe: '1day', rows: 23 });
  const D = owedLedger(body({ from: '2021-03-01', days: MAR, regular: 1, rung: '1day' }));
  const two = rollUpMonths([{ ...minute, cover: judgeRow(minute, { ledger: MAR_L }) }, { ...daily, cover: judgeRow(daily, { ledger: D }) }])[0];
  assert.equal(two.tf, false, 'two rungs in a month name none');
  assert.equal(two.full, 2);
  assert.equal(two.storeWide, 0, 'no row shares a hole');
  const mixedFull = rollUpMonths([{ ...minute, cover: judgeRow(minute, { ledger: MAR_L }) }, { ...nifty({ rows: 23 * 375 - 9 }), cover: judgeRow(nifty({ rows: 23 * 375 - 9 }), { ledger: MAR_L }) }])[0];
  assert.equal(mixedFull.full, 1, 'a short row is not counted full');
  const one = rollUpMonths([{ ...minute, cover: judgeRow(minute, { ledger: MAR_L }) }, { ...minute, cover: judgeRow(minute, { ledger: MAR_L }) }])[0];
  assert.equal(one.tf, '1min');
  const short = nifty({ rows: 23 * 375 - 100 });
  const wideCover = judgeRow(short, { ledger: MAR_L, peers: { n: 2, atMost: 2 } });
  assert.equal(rollUpMonths([{ ...short, cover: wideCover }])[0].storeWide, 1);
  assert.equal(rollUpMonths([{ ...short, cover: wideCover }, { ...short, cover: wideCover }])[0].missing, 200, 'the month misses what its rows miss');
  // A later row missing fewer bars never takes the worst place, whatever its name.
  const big = { ...nifty({ instrument: 'NSE-INDEX-ZZZ', rows: 0 }), cover: judgeRow(nifty({ rows: 0 }), { ledger: MAR_L }) };
  const small = { ...nifty({ instrument: 'NSE-INDEX-AAA', rows: 23 * 375 - 1 }), cover: judgeRow(nifty({ rows: 23 * 375 - 1 }), { ledger: MAR_L }) };
  assert.equal(rollUpMonths([big, small])[0].worst, big);
  // Two rows of one identity: the first stays.
  const twin = { ...big };
  assert.equal(rollUpMonths([big, twin])[0].worst, big);
  // The verdict: what is judged, its percentage and the near band.
  assert.deepEqual(monthVerdict({ n: 2, owed: 100, bars: 100, missing: 0, trade: 1, unmeasured: 1, absent: 0 }), { pct: null, state: 'trade' });
  assert.deepEqual(monthVerdict({ n: 1, owed: 100, bars: 100, missing: 0, trade: 0, unmeasured: 1, absent: 0 }), { pct: null, state: 'unmeasured' });
  assert.deepEqual(monthVerdict({ n: 1, owed: 100, bars: 100, missing: 0, trade: 0, unmeasured: 0, absent: 0, over: 1 }), { pct: null, state: 'over' });
  assert.deepEqual(monthVerdict({ n: 1, owed: 0, bars: 0, missing: 0, trade: 0, unmeasured: 0, absent: 0 }), { pct: null, state: 'full' });
  assert.deepEqual(monthVerdict({ n: 1, owed: 100, bars: 99, missing: 1, trade: 0, unmeasured: 0, absent: 0 }), { pct: 0.99, state: 'near' });
});

/* ======================================================================
   THE LEDGER PINNED (R3-04): every prefix sum of one mixed calendar, and
   each edge of the reader a surviving mutant found.
   ====================================================================== */

test('every prefix sum of a mixed calendar, day by day', () => {
  // 2026-07-29 (Wed) .. 2026-08-07 (Fri): R R R . . X H U P X, with held digits.
  const from = epochDayOfIso('2026-07-29');
  const days = 'RRR..XHUPX';
  const held = '10-1110100';
  const L = owedLedger({
    status: 'ok',
    feed: 'zerodha',
    rung: '1min',
    from,
    days,
    held,
    heldBy: ['ZZS'],
    target: { kind: 'series', symbol: 'ZZS', segment: 'CASH' },
    owed: {
      regular: 375,
      casEligible: 360,
      derivatives: [
        { from: '2018-01-01', to: '2026-07-30', regular: 375 },
        { from: '2026-08-03', to: '2026-10-04', regular: 385, tradeDriven: true }
      ]
    },
    special: [
      { day: from + 6, code: 'H', owed: 316, indexOwed: 300 },
      { day: from + 8, code: 'P', owed: 60, indexOwed: 60 }
    ],
    halts: []
  });
  const span = ledgerSpan(L, L.from, L.to);
  assert.deepEqual(
    { ...span },
    {
      asked: 10, inside: 10, outside: 0,
      sessions: 8,
      index: 375 * 3 + 375 + 300 + 60 + 375,
      share: 375 * 3 + 375 + 316 + 60 + 375,
      cas: 375 * 3 + 360 + 316 + 60 + 360,
      // 07-31 is a session after the first era ends (07-30) and before the second: unknown to a future.
      deriv: 375 * 2 + 385 * 2,
      unknownIndex: 1, unknownShare: 1, unknownCas: 1,
      unknownDeriv: 1 + 1 + 1 + 1,
      // Sessions held at 0: 07-30 (R), 08-04 (H), 08-06 (P) and 08-07 (X);
      // '-' on 07-31 is unread, never unheld.
      unheldIndex: 375 + 300 + 60 + 375, unheldShare: 375 + 316 + 60 + 375, unheldCas: 375 + 316 + 60 + 360, unheldDeriv: 375 + 385,
      auction: 2, specials: 1, halted: 1, unmeasured: 1,
      haltBars: 375 - 316,
      // Held: 07-29, both weekend days, 08-03 and 08-05; of them sessions 07-29, 08-03, 08-05.
      heldDays: 5, heldSessions: 3, closedHeld: 2
    }
  );
  // A one-day span is that day, not an empty one.
  assert.equal(ledgerSpan(L, from, from).sessions, 1);
  assert.equal(ledgerSpan(L, from, from).inside, 1);
  assert.equal(ledgerSpan(L, from, from).outside, 0);
  assert.equal(ledgerSpan(L, from - 5, from - 1).inside, 0);
  // A halt day with no record owes no halted bars.
  const bare = owedLedger({ days: 'H', from, owed: { regular: 375 } });
  assert.equal(ledgerSpan(bare, from, from).haltBars, 0);
  assert.equal(ledgerSpan(bare, from, from).sessions, 1, 'a halt day is a session');
  // The answer's own words.
  assert.equal(L.feed, 'zerodha');
  assert.equal(L.rung, '1min');
  assert.deepEqual(L.series, { symbol: 'ZZS', segment: 'CASH' });
  assert.deepEqual(L.heldBy, ['ZZS']);
  assert.equal(owedLedger({ days: 'R', from, target: { kind: 'series', symbol: 7 } }).series, null, 'a symbol that is not a name');
  assert.deepEqual(owedLedger({ days: 'R', from, target: { kind: 'series', symbol: 'ZZS' } }).series, { symbol: 'ZZS', segment: '' });
  assert.deepEqual(L.derivatives?.map((e) => e.tradeDriven), [false, true]);
});

test('the reader and its helpers at their edges', async () => {
  const signal = new AbortController().signal;
  // The calendar that measures nothing.
  assert.deepEqual(emptyCalendar('why'), { first: '', last: '', owed: new Map(), indexOwed: new Map(), withheld: new Set(), withheldMonths: new Set(), from: [], clashes: 0, why: 'why' });
  assert.throws(() => withheldDays('x', isoOfEpochDay), /is not a list/);
  // Dense days: none, not a string, an auction day, a special day's own index count, a special entry that is null.
  const from = epochDayOfIso('2026-08-03');
  assert.equal(denseOwed(null).owed.size, 0);
  assert.equal(denseOwed({ days: ['R'], from }).owed.size, 0);
  const dense = denseOwed({ days: 'XP', from, minuteOwed: { regular: 375 }, special: [null, { day: from + 1, minuteOwed: 60, minuteIndexOwed: 55 }] });
  assert.equal(dense.owed.get('2026-08-03'), 375, 'an auction day is a regular day here');
  assert.equal(dense.indexOwed.get('2026-08-04'), 55);
  assert.equal(denseOwed({ days: 'R', from }).owed.get('2026-08-03'), null, 'no regular count');
  // A backwards month span holds nothing; a day string that is no string, or empty, is refused.
  assert.deepEqual(monthsBetween('2025-01', '2024-03'), []);
  assert.throws(() => owedLedger(null), /carried no day string/);
  assert.throws(() => owedLedger({ days: ['R'], from }), /carried no day string/);
  assert.throws(() => owedLedger({ days: '', from, reason: 'held back' }), /carried no day string \(held back\)/);
  assert.equal(owedLedger({ days: 'R', from }).regular, null, 'an answer with no owed table owes no count');
  assert.equal(derivativeEras(null), null);
  assert.throws(() => derivativeEras([{ from: '2026-08-03junk', to: '2026-10-04' }]), /malformed era/);
  assert.throws(() => derivativeEras([null]), /malformed era/);
  assert.throws(() => derivativeEras([{ from: 'x2026-08-03', to: '2026-10-04' }]), /malformed era/, 'a day is the whole field, not its tail');
  assert.deepEqual(derivativeEras([{ from: '2026-08-03', to: '2026-08-03', regular: 385 }])?.map((e) => [e.from, e.to]), [[epochDayOfIso('2026-08-03'), epochDayOfIso('2026-08-03')]], 'a one-day era stands');
  // The day after the last era is unknown to a future, and the era's last day is its own.
  const eraEnd = owedLedger({ days: 'RR..R', from: epochDayOfIso('2026-07-30'), owed: { regular: 375, derivatives: [{ from: '2018-01-01', to: '2026-07-30', regular: 375 }] } });
  const e = ledgerSpan(eraEnd, eraEnd.from, eraEnd.to);
  assert.equal(e.deriv, 375);
  assert.equal(e.unknownDeriv, 2);
  // The reader: what it asks with, the hint's floor, a null building body, a refusal named by route.
  /** @type {any[]} */
  const asked = [];
  /** @type {number[]} */
  const waits = [];
  let n = 0;
  const reply = await askCalendar('/calendar.json?x', async (url, init) => {
    asked.push(init);
    return ++n === 1 ? answer(202, { status: 'building', retry_after_ms: 50 }) : n === 2 ? answer(202, null) : answer(200, {});
  }, signal, async (ms) => { waits.push(ms); });
  assert.equal(asked.length, 2, 'a building answer with no body ends the asking');
  for (const init of asked) assert.deepEqual(init, { cache: 'no-store', signal }, 'every ask, the first and each again, bypasses the cache');
  assert.deepEqual(waits, [50], 'a hint of exactly 50 ms is taken as given');
  assert.equal(reply.body, null, 'a building answer with no body is answered as it came');
  const refusal = /** @type {any} */ ({ ok: false, status: 503, text: async () => '' });
  assert.equal((await readLedger('/calendar.json', async () => refusal, signal)).why, '/calendar.json answered HTTP 503 and named no reason');
  assert.equal((await readCalendar('gdfl', async () => refusal, signal)).why, '/calendar.json answered HTTP 503 and named no reason');
  assert.equal((await readCalendar('gdfl', async () => answer(202, { status: 'building' }), signal, async () => {})).why, 'the server is still building this calendar: no reason given');
  assert.equal((await readCalendar('gdfl', async () => answer(200, null), signal)).why, 'the store holds no bars for this feed');
  assert.equal((await readCalendar('gdfl', async () => answer(200, { reason: '' }), signal)).why, 'the store holds no bars for this feed');
  assert.equal((await readCalendar('gdfl', async () => { throw 'not an Error'; }, signal)).why, 'the calendar request failed');
  const read = await readCalendar('gdfl', async () => answer(200, { days: [{ day: from, owed: 375 }], firstDay: from, lastDay: from }), signal);
  assert.equal(read.why, '');
  assert.equal(read.clashes, 0, 'no disagreements listed, none counted');
  assert.equal((await readCalendar('gdfl', async () => answer(200, { days: [], disagreements: ['a', 'b'] }), signal)).clashes, 2);
  assert.equal(read.first, '2026-08-03');
  assert.deepEqual(read.from, []);
  // The loader asks the held span unless told otherwise.
  /** @type {string[]} */
  const urls = [];
  const loader = createCalendarLoader({
    request: async (url) => { urls.push(url); return answer(200, {}); },
    apply: () => {},
    requests: { run: async (work) => work({ signal, current: () => true }), cancel: () => {} }
  });
  await loader.load('gdfl');
  assert.deepEqual(urls, ['/calendar.json?feed=gdfl']);
});

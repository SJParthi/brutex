// THE /db DAY BAND (WP-54; D-3335 §3, DB-016, DB-025, DB-G37, DB-G87, ...).
//
// `node --test web/tests/` — the module the band, the bar grid's empty state
// and the day fields read, over `/dayflags.json` records in the route's own
// shape (`crates/api/src/dayflags.rs`). Every value below is invented; the
// SHAPES are the route's, and each case is a day kind the audit found shown as
// an unexplained hole.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  DAYFLAGS_PAGE_DAYS,
  DAY_KINDS,
  bandOf,
  bandSpan,
  bandInstrument,
  bandExpiry,
  casCounts,
  cellOf,
  definitionOf,
  unknownFieldsOf,
  dayflagsUrl,
  emptyWindowWhy,
  kindsOf,
  lineageOf,
  deriveOf,
  tally
} from '../src/lib/db/dayflags.js';

/** One record, every key present as the route sends it. */
function rec(/** @type {Record<string, any>} */ over = {}) {
  return {
    feed: 'gdfl',
    instrument: 'NSE-INDEX-NIFTY',
    day: '2024-03-04',
    calendar: { code: 'R', kind: 'regular', owed_minutes: 375 },
    events: { halt: null, phases: [] },
    import: { outcome: 'done', reached: true, late: 0, max_back_s: 0, refusals: { this: [], this_count: 0 } },
    quality: { indices: null, options: null, stocks: null },
    degraded: [],
    gaps: null,
    junk: null,
    refused_front: null,
    held: { feed: 'gdfl', rung: '1s', bars: 22500, why: null },
    peer: { feed: 'zerodha', rung: '1min', bars: 375, why: null },
    says: ['Zerodha holds 375 bar(s)'],
    ...over
  };
}

/** A label a reader sees: the ISO day, so the assertions read plainly. */
const label = (/** @type {string} */ iso) => `[${iso}]`;

test('a drill day the import left incomplete names why and the other feed\'s count (DB-016, DB-025, DB-G87)', () => {
  const drill = rec({
    day: '2024-03-02',
    calendar: { code: 'P', kind: 'special' },
    import: { outcome: 'incomplete', stage: 'file', reason: 'filing', reached: true, late: 0, refusals: { this: [], this_count: 1 } },
    held: { feed: 'gdfl', rung: '1s', bars: 0, why: null },
    peer: { feed: 'zerodha', rung: '1min', bars: 105, why: null },
    says: [
      'disaster-recovery drill session, 09:15-09:59 and 11:30-12:29',
      'incomplete at import (file): filing',
      'no GDFL bar on this session',
      'Zerodha holds 105 bar(s)'
    ]
  });
  const c = cellOf(drill);
  assert.deepEqual(c.kinds, ['incomplete', 'special']);
  assert.equal(c.kind, 'incomplete');
  assert.equal(c.tone, 'bad');
  assert.equal(c.held, 0);
  assert.equal(c.peer, 105);
  assert.ok(c.flagged);
  const why = emptyWindowWhy([c], '2024-03-02', '2024-03-02', label);
  assert.equal(
    why,
    'No bar on [2024-03-02]: disaster-recovery drill session, 09:15-09:59 and 11:30-12:29; incomplete at import (file): filing; no GDFL bar on this session; Zerodha holds 105 bar(s).'
  );
  assert.doesNotMatch(why ?? '', /disagreement|manifest/, 'never an integrity accusation');
});

test('a weekend or holiday window says closed; a window with no record says nothing yet', () => {
  const sat = cellOf(rec({ day: '2024-03-02', calendar: { code: '.' }, held: { bars: 0 }, peer: { feed: 'zerodha', bars: 0 }, says: ['closed: a weekend'] }));
  const sun = cellOf(rec({ day: '2024-03-03', calendar: { code: '.' }, held: { bars: 0 }, peer: { feed: 'zerodha', bars: 0 }, says: ['closed: a weekend'] }));
  assert.equal(sat.kind, 'closed');
  assert.equal(sat.flagged, false);
  assert.equal(emptyWindowWhy([sat], '2024-03-02', '2024-03-02', label), 'No bar on [2024-03-02]: closed: a weekend.');
  assert.equal(emptyWindowWhy([sat, sun], '', '', label), 'No bar in 2 days: every one is closed (closed: a weekend).');
  assert.equal(emptyWindowWhy([sat], '2024-04-01', '2024-04-02', label), null);
});

test('a session with no bar and no import refusal is a store gap; the other feed\'s count is beside it (DB-G37)', () => {
  const z = cellOf(
    rec({
      feed: 'zerodha',
      day: '2024-11-04',
      import: null,
      held: { feed: 'zerodha', rung: '1min', bars: 0, why: null },
      peer: { feed: 'gdfl', rung: '1s', bars: 21594, why: null },
      says: ['no Zerodha bar on this session', 'GDFL holds 21594 bar(s)']
    })
  );
  assert.equal(z.kind, 'store_gap');
  assert.equal(z.peer, 21594);
  assert.equal(z.peerFeed, 'gdfl');
  const many = [z, { ...z, day: '2024-11-05' }, { ...z, day: '2024-11-06' }];
  assert.match(emptyWindowWhy(many, '2024-11-04', '2024-11-06', label, 2) ?? '', /^No bar on any of 3 session day\(s\) — \[2024-11-04\]: .* · and 1 more, each in the day band above\.$/);
});

test('every vendor-fault and exchange-event field becomes its kind, in a fixed severity order', () => {
  assert.deepEqual(kindsOf(rec({ import: { outcome: 'refused', reached: true }, held: { bars: 0 } })), ['refused']);
  assert.deepEqual(kindsOf(rec({ refused_front: true, junk: ['zero_price'] })), ['front', 'junk']);
  assert.deepEqual(kindsOf(rec({ degraded: [{ kind: 'indices', says: 'ss_distinct 1' }] })), ['degraded']);
  assert.deepEqual(kindsOf(rec({ gaps: { missing: 14, why: 'vendor_empty' } })), ['gap']);
  assert.deepEqual(kindsOf(rec({ quality: { indices: { holes_5s: 1, dual_spread_s: 0 }, stocks: null } })), ['gap']);
  assert.deepEqual(kindsOf(rec({ quality: { indices: { holes_5s: 0, dual_spread_s: 3 }, stocks: null } })), ['gap'], 'DB-G96: dual-value seconds');
  assert.deepEqual(kindsOf(rec({ import: { outcome: 'done', reached: true, late: 703, max_back_s: 126 } })), ['late']);
  assert.deepEqual(kindsOf(rec({ calendar: { code: 'H' }, events: { halt: { suspected: true }, phases: [] } })), ['halt']);
  assert.deepEqual(kindsOf(rec({ calendar: { code: 'U' } })), ['unmeasured']);
  assert.deepEqual(kindsOf(rec({ calendar: { code: '-' }, held: { bars: null } })), ['outside']);
  assert.deepEqual(kindsOf(rec({ import: { outcome: 'none', reached: false }, held: { bars: null } })), ['not_reached']);
  assert.deepEqual(kindsOf(rec()), ['session']);
  // The auction phase is a share's: an index on the same day has none (D-3320).
  // SC-CLOSE-9: an auction only on a day the share's dated flag is 1; flag 0 is a session; a flag no master holds is said as that.
  assert.deepEqual(kindsOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X', cas: { eligible: true } } })), ['cas']);
  assert.deepEqual(kindsOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X', cas: { eligible: false } } })), ['session']);
  assert.deepEqual(kindsOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X', cas: { eligible: null, why: 'no master' } } })), ['cas']);
  assert.deepEqual(kindsOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X' } })), ['cas'], 'an older record, which names no flag');
  const unheld = cellOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X', cas: { eligible: null } } }));
  assert.equal(unheld.word, 'closing-auction window, flag not held');
  const eligibleCell = cellOf(rec({ instrument: 'NSE-CASH-RELIANCE', day: '2026-08-05', calendar: { code: 'X', cas: { eligible: true } } }));
  assert.equal(eligibleCell.word, 'closing auction from 15:15');
  // The legend counts the two apart, by the cell's own word.
  assert.deepEqual(tally([unheld, eligibleCell, { ...unheld, day: '2026-08-06' }]).map((k) => [k.kind, k.word, k.n]), [['cas', 'closing-auction window, flag not held', 2], ['cas', 'closing auction from 15:15', 1]]);
  assert.deepEqual(kindsOf(rec({ instrument: 'NSE-INDEX-NIFTY', calendar: { code: 'X' } })), ['session']);
  // A null ledger field is not measured, never clean and never a fault.
  assert.deepEqual(kindsOf(rec({ quality: null, gaps: null, junk: null })), ['session']);
  assert.equal(DAY_KINDS[0].kind, 'refused');
});

test('a band is the days list sorted oldest first; a body without one is refused by name', () => {
  const band = bandOf({
    days: [rec({ day: '2024-03-04' }), rec({ day: '2024-03-01' })],
    page: 0,
    pages: 1,
    page_from: '2024-03-01',
    page_to: '2024-03-04',
    notes: ['No GDFL day ledger yet'],
    rungs: { held: { feed: 'gdfl', rung: '1s' } }
  });
  assert.deepEqual(band.cells.map((c) => c.day), ['2024-03-01', '2024-03-04']);
  assert.deepEqual(band.notes, ['No GDFL day ledger yet']);
  assert.throws(() => bandOf({ error: 'unknown instrument: NSE-INDEX-NOPE' }), /without a days list: unknown instrument: NSE-INDEX-NOPE/);
  assert.deepEqual(
    tally(band.cells).map((t) => [t.kind, t.n]),
    [['session', 2]]
  );
});

test('the band asks for the window when it fits one page, else its newest page, and says which', () => {
  const add = (/** @type {string} */ iso, /** @type {number} */ n) =>
    new Date(Date.parse(`${iso}T00:00:00Z`) + n * 86_400_000).toISOString().slice(0, 10);
  assert.deepEqual(bandSpan('2024-03-02', '2024-03-02', add), { from: '2024-03-02', to: '2024-03-02', clipped: false });
  assert.deepEqual(bandSpan('2018-01-01', '2026-09-30', add), { from: add('2026-09-30', -(DAYFLAGS_PAGE_DAYS - 1)), to: '2026-09-30', clipped: true });
  assert.equal(bandSpan('', '2026-09-30', add), null);
  assert.equal(bandSpan('2026-10-01', '2026-09-30', add), null);
  assert.equal(
    dayflagsUrl('gdfl', 'NSE-FNO-NIFTY-2019-06-27-1150000-CE', '2019-06-01', '2019-06-30', 1),
    '/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY-2019-06-27-1150000-CE&from=2019-06-01&to=2019-06-30&order=asc&page=1'
  );
});

test('a share day of zero-volume minutes is flagged; an earlier run\'s refusals are history (DB-B16, DB-B45)', () => {
  const fills = cellOf(
    rec({
      instrument: 'NSE-CASH-ZZFILL',
      feed: 'zerodha',
      import: null,
      held: { feed: 'zerodha', rung: '1min', bars: 375, why: null, zero_volume: 300 },
      says: ['300 of 375 Zerodha bar(s) carry zero volume: held, but no trade in that minute']
    })
  );
  assert.deepEqual(fills.kinds, ['fills']);
  assert.equal(fills.fills, 300);
  assert.equal(cellOf(rec({ held: { bars: 375, zero_volume: 10 } })).kind, 'session', 'a few no-trade minutes are not a flag');
  assert.equal(cellOf(rec()).fills, null, 'not counted is null, never zero');
  const drill = cellOf(
    rec({
      day: '2024-03-02',
      calendar: { code: 'P' },
      import: {
        outcome: 'incomplete',
        reached: true,
        run: 1791602895,
        refusals: {
          this: [{ about: 'NIFTY 50.NSE_IDX.csv', why: 'filing' }],
          this_count: 1,
          superseded: [{ run: null, commit: null, at_ms: 1791171132660, about: 'calendar', why: 'not a regular session' }],
          superseded_count: 1
        }
      },
      held: { bars: 0 }
    })
  );
  assert.equal(drill.superseded.length, 1);
  assert.equal(drill.superseded[0].about, 'calendar');
  assert.equal(drill.kind, 'incomplete');
});

test('a contract renamed at an ex-date says where it continues; no ledger is said, never "never renamed" (DB-G72)', () => {
  const band = bandOf({
    days: [rec({ instrument: 'NSE-FNO-ZZBRIT-2018-12-27-600000-CE', day: '2018-11-28' })],
    lineage: {
      present: true,
      records: [{ kind: 'continues_as', key: 'NSE-FNO-ZZBRIT-2018-12-27-300000-CE', ex: '2018-11-29', factor: '2/1' }],
      why: null
    }
  });
  assert.deepEqual(band.lineage?.lines, [
    'renamed at the 2018-11-29 ex-date, continues as NSE-FNO-ZZBRIT-2018-12-27-300000-CE (factor 2/1)'
  ]);
  assert.deepEqual(lineageOf({ present: true, records: [{ kind: 'renamed_from', key: 'K', ex: '2018-11-29', factor: '2/1' }] })?.lines, [
    'renamed at the 2018-11-29 ex-date from K (factor 2/1)'
  ]);
  assert.match(lineageOf({ present: true, records: [{ kind: 'stitched', ex: '2018-11-29', after: 'K' }] })?.lines[0] ?? '', /two exchange contracts/);
  const none = lineageOf({ present: false, records: [], why: 'no lineage ledger yet' });
  assert.equal(none?.present, false);
  assert.equal(none?.why, 'no lineage ledger yet');
  assert.deepEqual(none?.lines, []);
  assert.equal(lineageOf(null), null, 'an index or a share carries no lineage');
  assert.equal(bandOf({ days: [] }).lineage, null);
});

test('a trade-driven series with no bar on a session traded nothing there: never a store gap (D-3336 §2)', () => {
  const option = cellOf(
    rec({ instrument: 'NSE-FNO-ZZIDX-2019-06-27-1130000-PE', held: { feed: 'gdfl', rung: '1s', bars: 0, why: null }, says: ['no GDFL bar on this session'] })
  );
  assert.deepEqual(option.kinds, ['no_trade']);
  assert.equal(option.flagged, false, 'no trade is not a fault');
  const share = cellOf(rec({ instrument: 'NSE-CASH-ZZSHR', held: { feed: 'gdfl', rung: '1s', bars: 0, why: null } }));
  assert.deepEqual(share.kinds, ['no_trade']);
  // A Zerodha minute share sends a candle for every minute: none on a session is a gap.
  const minute = cellOf(rec({ instrument: 'NSE-CASH-ZZSHR', feed: 'zerodha', held: { feed: 'zerodha', rung: '1min', bars: 0, why: null } }));
  assert.deepEqual(minute.kinds, ['store_gap']);
  assert.equal(minute.flagged, true);
  assert.equal(
    emptyWindowWhy([option], '2024-03-04', '2024-03-04', label),
    'No bar on [2024-03-04]: no trade printed that session — this series holds a bar only where a trade printed (D-3336).'
  );
});

test("a day incomplete for another contract's file warns; it is not this contract's refusal (DB-G69)", () => {
  const other = cellOf(
    rec({
      instrument: 'NSE-FNO-ZZIDX-2018-11-29-1050000-CE',
      import: { outcome: 'incomplete', reached: true, refusals: { this: [], this_count: 0, other_count: 1 } },
      held: { feed: 'gdfl', rung: '1s', bars: 0, why: null }
    })
  );
  assert.equal(other.kind, 'incomplete_other');
  assert.equal(other.tone, 'warn');
  assert.ok(other.kinds.includes('no_trade'), 'and the contract simply did not trade that day');
  const own = cellOf(rec({ import: { outcome: 'incomplete', reached: true, refusals: { this: [{ about: 'x', why: 'y' }], this_count: 1, other_count: 0 } }, held: { bars: 0 } }));
  assert.equal(own.kind, 'incomplete');
  assert.ok(!own.kinds.includes('store_gap'), 'the refusal explains the empty day');
});

test('an empty window whose days all hold bars names the time-of-day window, not the days', () => {
  const held = [cellOf(rec({ day: '2024-03-04' })), cellOf(rec({ day: '2024-03-05' }))];
  assert.equal(
    emptyWindowWhy(held, '2024-03-04', '2024-03-05', label),
    'Every one of the 2 session day(s) holds bars (45000 in all); none falls inside the time-of-day window.'
  );
  const mixed = [cellOf(rec({ day: '2024-03-04' })), cellOf(rec({ day: '2024-03-05', held: { bars: 0 }, says: ['no GDFL bar on this session'] }))];
  assert.equal(emptyWindowWhy(mixed, '2024-03-04', '2024-03-05', label), 'No bar on [2024-03-05]: no GDFL bar on this session.');
});

test("a derived rung's withheld buckets are counted per month; an unwritten ledger is null, never none (DB-G36)", () => {
  const lines = deriveOf([
    { rung: '60min', month: '2023-02', days: 20, buckets: 126, withheld: 14, partial: 0, run: 3, line: 9 },
    { rung: '5min', month: '2023-02', days: 20, buckets: 1500, withheld: 0, partial: 0, run: 3, line: 10 }
  ]);
  assert.deepEqual(lines, ['60min 2023-02: 126 bucket(s) folded over 20 day(s), 14 withheld (a minute missing in the bucket, D-3321)']);
  assert.equal(deriveOf(null), null);
  assert.equal(bandOf({ days: [], derive: null }).derive, null);
  assert.deepEqual(bandOf({ days: [], derive: [] }).derive, []);
});

test("a day past the vendor archive's end is outside it: never a store gap, never 'not imported yet' (DB-G67, DB-G68)", () => {
  const archive = { from: '2018-09-03', to: '2026-09-24' };
  const after = cellOf(
    rec({ day: '2026-09-25', import: { outcome: 'none', reached: false }, held: { feed: 'gdfl', rung: '1s', bars: 0, why: null } }),
    { archive }
  );
  assert.deepEqual(after.kinds, ['outside_archive']);
  assert.equal(after.flagged, false);
  // Without the archive span the same record reads as a store gap: the span is what tells them apart.
  assert.ok(cellOf(rec({ day: '2026-09-25', import: { outcome: 'none', reached: false }, held: { bars: 0 } })).kinds.includes('store_gap'));
  const inside = cellOf(rec({ day: '2026-09-24' }), { archive });
  assert.equal(inside.kind, 'session');
  const band = bandOf({ days: [rec({ day: '2026-09-25', held: { bars: 0 } }), rec({ day: '2026-09-24' })] }, { archive });
  assert.deepEqual(band.cells.map((c) => c.kind), ['session', 'outside_archive']);
  assert.equal(emptyWindowWhy([after], '2026-09-25', '2026-09-25', label), 'No bar on [2026-09-25]: not imported.');
});

test("R3-05 repair 3: a day before a binding archive start is outside the vendor's archive, never 'not imported'", () => {
  const archive = { from: '2018-09-03', to: '2026-09-24', from_binds: true, to_binds: false };
  const early = cellOf(rec({ day: '2018-08-31', import: { outcome: 'none', reached: false }, held: { feed: 'gdfl', rung: '1s', bars: 0, why: null } }), { archive });
  assert.deepEqual(early.kinds, ['before_archive']);
  assert.equal(early.flagged, false);
  assert.equal(emptyWindowWhy([early], '2018-08-31', '2018-08-31', label), "No bar on [2018-08-31]: before the vendor's archive.");
  // PAST THE MEASURED LAST DAY (a lower bound) it is still not imported.
  const late = cellOf(rec({ day: '2026-09-25', import: { outcome: 'none', reached: false }, held: { bars: 0 } }), { archive });
  assert.deepEqual(late.kinds, ['outside_archive']);
  // A FIRST DAY THE SERVER SAYS DOES NOT BIND is not imported, not outside.
  const loose = cellOf(rec({ day: '2018-08-31', import: { outcome: 'none', reached: false }, held: { bars: 0 } }), { archive: { ...archive, from_binds: false } });
  assert.deepEqual(loose.kinds, ['outside_archive']);
});

// ── R3-13: the band at the grid's rung, frozen index values, late rows, the
// bar definition of a day, and fields the page has never seen ─────────────

test('R3-13: the band counts the asked timeframe, and a missing day bar is a hole', () => {
  // The URL carries the grid's rung and a family's expiry, or neither.
  assert.equal(
    dayflagsUrl('zerodha', 'NSE-INDEX-NIFTY', '2024-10-01', '2024-10-04', 0, { timeframe: '1day' }),
    '/dayflags.json?feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-10-01&to=2024-10-04&order=asc&timeframe=1day'
  );
  assert.equal(
    dayflagsUrl('gdfl', 'NSE-FNO-NIFTY', '2024-10-01', '2024-10-31', 1, { expiry: '2019-03-28' }),
    '/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY&from=2024-10-01&to=2024-10-31&order=asc&page=1&expiry=2019-03-28'
  );
  assert.equal(dayflagsUrl('gdfl', 'NSE-INDEX-NIFTY', '2024-10-01', '2024-10-01'), '/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-10-01&to=2024-10-01&order=asc');
  // A Zerodha 1day session holding minutes and no day bar is the day bar missing.
  const missing = cellOf(
    rec({
      feed: 'zerodha',
      held: { feed: 'zerodha', rung: '1day', bars: 0, why: null, zero_volume: null },
      finest: { feed: 'zerodha', rung: '1min', bars: 375, why: null, zero_volume: null },
      peer: { feed: 'gdfl', rung: '1s', bars: 0, why: null },
      says: ['no Zerodha 1day bar on this session, while Zerodha holds 375 1min bar(s) that day: the 1day bar is missing']
    })
  );
  assert.equal(missing.kind, 'store_gap');
  assert.deepEqual([missing.held, missing.rung, missing.finest, missing.finestRung], [0, '1day', 375, '1min']);
  // A day bar held, with no minutes, is a session: never "no Zerodha bar".
  const shown = cellOf(
    rec({
      feed: 'zerodha',
      held: { feed: 'zerodha', rung: '1day', bars: 1, why: null },
      finest: { feed: 'zerodha', rung: '1min', bars: 0, why: null },
      says: []
    })
  );
  assert.equal(shown.kind, 'session');
  // At the finest rung there is no second count.
  assert.equal(cellOf(rec()).finest, null);
});

test('R3-13: one contract is asked about itself, one expiry at any strike is the family narrowed to it', () => {
  const contract = 'NSE-FNO-NIFTY-2019-03-28-1150000-CE';
  const row = (/** @type {Record<string, any>} */ o) => ({ underlying: 'NIFTY', timeframe: '1s', expiry: '2019-03-28', ...o });
  // One contract, at two rungs: the contract, at either rung or both.
  const one = [row({ instrument: contract }), row({ instrument: contract, timeframe: '1min' })];
  assert.equal(bandInstrument(one), contract);
  assert.equal(bandInstrument(one, '1s'), contract);
  assert.equal(bandExpiry(one, contract), '', 'a contract carries its own expiry');
  // Another contract at another rung than the grid's is not the grid's.
  const mixed = [row({ instrument: contract }), row({ instrument: 'NSE-FNO-NIFTY-2019-03-28-1160000-PE', timeframe: '1min' })];
  assert.equal(bandInstrument(mixed, '1s'), contract);
  assert.equal(bandInstrument(mixed), 'NSE-FNO-NIFTY');
  // Any strike of one expiry: the family, narrowed to that expiry.
  const strikes = [row({ instrument: contract }), row({ instrument: 'NSE-FNO-NIFTY-2019-03-28-1160000-PE' })];
  assert.equal(bandInstrument(strikes), 'NSE-FNO-NIFTY');
  assert.equal(bandExpiry(strikes, 'NSE-FNO-NIFTY'), '2019-03-28');
  // Two expiries, or a row with none: no one expiry.
  assert.equal(bandExpiry([...strikes, row({ instrument: 'NSE-FNO-NIFTY-2019-04-25-1160000-PE', expiry: '2019-04-25' })], 'NSE-FNO-NIFTY'), '');
  assert.equal(bandExpiry([row({ instrument: 'NSE-FNO-NIFTY-2019-03-28-FUT', expiry: null }), ...strikes], 'NSE-FNO-NIFTY'), '');
  // A share or an index never sends one; an absent row is passed over.
  assert.equal(bandExpiry(strikes, 'NSE-INDEX-NIFTY'), '');
  assert.equal(bandInstrument([{ instrument: 'NSE-INDEX-NIFTY', absent: true }, { instrument: 'NSE-INDEX-BANKNIFTY' }]), 'NSE-INDEX-BANKNIFTY');
  assert.equal(bandExpiry([{ ...strikes[0], absent: true }, strikes[1]], 'NSE-FNO-NIFTY'), '2019-03-28');
  assert.equal(bandInstrument([{ instrument: 'NSE-INDEX-NIFTY' }, { instrument: 'NSE-CASH-RELIANCE' }]), '');
});

test('SC-CLOSE-12: a day after expiry is expired, owing nothing — never a hole or "not imported yet"', () => {
  const after = cellOf(
    rec({
      instrument: 'NSE-FNO-NIFTY-2026-08-04-2700000-CE',
      day: '2026-08-05',
      calendar: { code: 'R', owed_minutes: 0, expired: true },
      contract: { expiry: '2026-08-04', expired: true },
      import: { outcome: 'none', reached: false, late: null },
      held: { feed: 'gdfl', rung: '1s', bars: 0, why: null },
      says: ["expired: the contract's last session was 2026-08-04, so this day owes nothing"]
    })
  );
  assert.deepEqual(after.kinds, ['expired']);
  assert.equal(after.tone, 'quiet');
  assert.equal(after.flagged, false);
  // A family narrowed to an expiry reads the same.
  assert.deepEqual(kindsOf(rec({ instrument: 'NSE-FNO-NIFTY', contract: { expiry: '2019-03-28', expired: true }, held: { bars: null } })), ['expired']);
});

test('DB-B20, DB-A15, goal 3: a frozen index, a value outside the day range, a closed day the peer traded, an older definition', () => {
  assert.deepEqual(kindsOf(rec({ index: { frozen: { flagged: true }, outside_day_range: { flagged: true } } })), ['frozen', 'outside_range']);
  assert.deepEqual(kindsOf(rec({ index: { frozen: { flagged: false }, outside_day_range: null } })), ['session']);
  assert.deepEqual(kindsOf(rec({ calendar: { code: '.' }, held: { bars: 0 }, peer: { feed: 'gdfl', bars: 21594 } })), ['peer_on_closed', 'closed']);
  assert.deepEqual(kindsOf(rec({ calendar: { code: '.' }, held: { bars: 0 }, peer: { feed: 'gdfl', bars: 0 } })), ['closed']);
  const old = cellOf(rec({ bar_definition: { value: 2, current: false, means: 'fixture', from: 'import_journal', this_build: 4 } }));
  assert.deepEqual(old.kinds, ['old_definition']);
  assert.deepEqual(old.definition, { value: 2, current: false, means: 'fixture', from: 'import_journal', thisBuild: 4 });
  assert.equal(cellOf(rec({ bar_definition: { value: 4, current: true } })).kind, 'session');
  assert.equal(definitionOf(null), null);
  assert.equal(definitionOf({ value: 'two' }), null);
  assert.deepEqual(definitionOf({ value: 3 }), { value: 3, current: false, means: '', from: '', thisBuild: null });
  // Every new kind has a word and a tone.
  for (const kind of ['frozen', 'outside_range', 'peer_on_closed', 'old_definition', 'expired']) {
    assert.ok(DAY_KINDS.some((k) => k.kind === kind && k.word && k.tone), kind);
  }
});

test('R3-13: a day field the reader has never seen is served and drawn', () => {
  const quality = {
    indices: { holes_5s: 0, frozen_runs: 2, frozen_note: 'vendor_flat', unknown: ['frozen_runs', 'frozen_note'] },
    stocks: { unknown: ['gone'] },
    options: null
  };
  assert.deepEqual(unknownFieldsOf(quality), [
    { kind: 'indices', name: 'frozen_runs', value: '2' },
    { kind: 'indices', name: 'frozen_note', value: 'vendor_flat' }
  ]);
  assert.deepEqual(cellOf(rec({ quality })).unknown.map((f) => f.name), ['frozen_runs', 'frozen_note']);
  assert.deepEqual(unknownFieldsOf({ indices: { unknown: ['n'], n: null } }), [{ kind: 'indices', name: 'n', value: 'null' }]);
  assert.deepEqual(unknownFieldsOf(null), []);
  assert.deepEqual(unknownFieldsOf({ indices: { unknown: 'not a list' } }), []);
  // The band component draws them, under their own names.
  const svelte = readFileSync(new URL('../src/routes/db/DbDayFlags.svelte', import.meta.url), 'utf8');
  assert.match(svelte, /\{#each c\.unknown as f/);
  assert.match(svelte, /<code>\{f\.kind\}\.\{f\.name\}<\/code> = \{f\.value\}/);
});

test('SC-CLOSE-9: the auction line claims "kept apart" only where a sidecar keeps the print apart', () => {
  const c = cellOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X', cas: { eligible: true } }, auction_sidecar: false }));
  assert.equal(c.cas, true);
  assert.equal(c.auctionSidecar, false);
  assert.equal(cellOf(rec()).auctionSidecar, null);
  const svelte = readFileSync(new URL('../src/routes/db/DbDayFlags.svelte', import.meta.url), 'utf8');
  assert.doesNotMatch(svelte, /at 15:15 and the auction print is kept apart from trades \(charter/);
  assert.match(svelte, /no month holds an auction sidecar, so the print is an ordinary bar here/);
});

test('R3-13 review: a share on a coarser grid is trade-driven by its finest rung, and its fills are the finest rung\'s', () => {
  // A GDFL share asked at 1min: `held` counts the grid's rung, `finest` the feed's 1s.
  const coarse = rec({
    instrument: 'NSE-CASH-ZZSHR',
    held: { feed: 'gdfl', rung: '1min', bars: 0, why: null },
    finest: { feed: 'gdfl', rung: '1s', bars: 0, why: null }
  });
  assert.deepEqual(kindsOf(coarse), ['no_trade'], 'a share that holds nothing at one second traded nothing: never a store gap');
  // A Zerodha share at 1day: the finest rung is its minutes, which are not trade-driven.
  const day = rec({
    instrument: 'NSE-CASH-ZZSHR',
    feed: 'zerodha',
    held: { feed: 'zerodha', rung: '1day', bars: 0, why: null },
    finest: { feed: 'zerodha', rung: '1min', bars: 0, why: null }
  });
  assert.deepEqual(kindsOf(day), ['store_gap']);
  // Zero-volume minutes ride on `finest` when the grid counts the day bar.
  const fills = cellOf(
    rec({
      instrument: 'NSE-CASH-ZZFILL',
      feed: 'zerodha',
      import: null,
      held: { feed: 'zerodha', rung: '1day', bars: 1, why: null, zero_volume: 0 },
      finest: { feed: 'zerodha', rung: '1min', bars: 375, why: null, zero_volume: 300 }
    })
  );
  assert.deepEqual(fills.kinds, ['fills']);
  assert.equal(fills.fills, 300);
  // At the finest rung itself, `held` is still read.
  assert.equal(cellOf(rec({ instrument: 'NSE-CASH-ZZFILL', held: { feed: 'zerodha', rung: '1min', bars: 375, zero_volume: 300 } })).fills, 300);
});

test('R3-13 review: a closing-auction day whose flag no master holds is never counted as kept apart', () => {
  const day = (/** @type {boolean | undefined} */ eligible, /** @type {boolean} */ sidecar) =>
    cellOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X', cas: eligible === undefined ? undefined : { eligible } }, auction_sidecar: sidecar }));
  const cells = [day(true, true), day(true, false), day(undefined, true), day(false, true)];
  assert.deepEqual(cells.map((c) => c.kind), ['cas', 'cas', 'cas', 'session'], 'flag 0 is an ordinary session');
  assert.deepEqual(casCounts(cells), { days: 2, unheld: 1, apart: 1 });
  // A day with another fault beside the auction is listed with its fault, not counted here.
  const both = cellOf(rec({ instrument: 'NSE-CASH-RELIANCE', calendar: { code: 'X', cas: { eligible: true } }, auction_sidecar: true, degraded: ['x'] }));
  assert.deepEqual(casCounts([both]), { days: 0, unheld: 0, apart: 0 });
  // The component counts with the function.
  const svelte = readFileSync(new URL('../src/routes/db/DbDayFlags.svelte', import.meta.url), 'utf8');
  assert.match(svelte, /const cas = \$derived\(casCounts\(flagged\)\);/);
});

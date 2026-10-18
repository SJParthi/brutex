// A CLOSED DAY IS CLOSED, A SESSION IS THE CALENDAR'S, AND A SECOND IS NOT
// A MINUTE (AH-03, g-ui-dom-vs-json-6 and g-ui-dom-vs-json-7; D-5837, D-5838).
//
// `node --test web/tests/` — the day band's kinds, its session count and the
// empty grid's reason, over `/dayflags.json` records in the route's own shape
// (`crates/api/src/dayflags.rs`). The archive span is `/feeds.json`'s shape;
// every count below is invented (22,222 bars, 7 prints, 3 minutes).

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { DAY_KINDS, bandOf, cellOf, emptyWindowWhy, kindsOf, sessionCount, isSessionCode, isHoleDay, tally } from '../src/lib/db/dayflags.js';

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
    held: { feed: 'gdfl', rung: '1s', bars: 22222, why: null },
    peer: { feed: 'zerodha', rung: '1min', bars: 375, why: null },
    says: [],
    ...over
  };
}

const label = (/** @type {string} */ iso) => `[${iso}]`;
/** The archive span as `/feeds.json` serves it (R3-05). */
const ARCHIVE = { from: '2018-09-03', to: '2026-09-24', from_binds: true, to_binds: false };
const weekend = (/** @type {string} */ day) =>
  rec({ day, calendar: { code: '.', kind: 'closed' }, import: { outcome: 'none', reached: false }, held: { feed: 'gdfl', rung: '1s', bars: 0, why: null }, says: ['closed: a weekend'] });
const unimported = (/** @type {string} */ day) => rec({ day, import: { outcome: 'none', reached: false }, held: { feed: 'gdfl', rung: '1s', bars: 0, why: null } });

test('a_closed_day_outside_the_archive_is_closed_not_a_session', () => {
  // AFTER THE SPAN: the weekend of 26-27 Sep 2026 read "not imported" and
  // was counted as a session.
  const sat = cellOf(weekend('2026-09-26'), { archive: ARCHIVE });
  assert.deepEqual(sat.kinds, ['closed']);
  assert.equal(sat.kind, 'closed');
  assert.equal(sat.word, 'closed');
  assert.equal(sat.flagged, false);
  // BEFORE THE SPAN, where the first day binds: a 2018-08 weekend is closed,
  // never "before the vendor's archive".
  const early = cellOf(weekend('2018-08-04'), { archive: ARCHIVE });
  assert.deepEqual(early.kinds, ['closed']);
  // A SESSION on either side still says where it lies.
  assert.deepEqual(cellOf(unimported('2018-08-06'), { archive: ARCHIVE }).kinds, ['before_archive']);
  assert.deepEqual(cellOf(unimported('2026-09-28'), { archive: ARCHIVE }).kinds, ['outside_archive']);
  // A closed day INSIDE the span is closed, as it always was.
  assert.deepEqual(cellOf(weekend('2024-03-02'), { archive: ARCHIVE }).kinds, ['closed']);
  // NO ARCHIVE SPAN on the feed (absent from /feeds.json): nothing is
  // outside it, and a closed day is closed.
  assert.deepEqual(cellOf(weekend('2026-09-26')).kinds, ['closed']);
  assert.deepEqual(cellOf(weekend('2026-09-26'), { archive: null }).kinds, ['closed']);
  // A day past the calendar's span is outside it, never outside the archive.
  assert.deepEqual(cellOf(rec({ day: '2031-01-01', calendar: { code: '-' }, held: { bars: null } }), { archive: ARCHIVE }).kinds, ['outside']);
});

test('a session past the measured last day that HOLDS bars was imported: never "not imported"', () => {
  // The archive's last day is a measured lower bound (R3-05); the store
  // held 22,500 bars on each of 25-30 Sep 2026 while the band said "not
  // imported" on them.
  const held = cellOf(rec({ day: '2026-09-25' }), { archive: ARCHIVE });
  assert.deepEqual(held.kinds, ['session']);
  // The same day with none held is not imported.
  assert.deepEqual(cellOf(unimported('2026-09-25'), { archive: ARCHIVE }).kinds, ['outside_archive']);
});

test('the_session_count_equals_the_calendars_sessions', () => {
  // 14-30 Sep 2026: weekends closed, the archive's measured end on the 24th,
  // the days after it held (25, 28-30) and the weekend between (26, 27).
  /** @type {any[]} */
  const days = [];
  for (let d = 14; d <= 30; d++) {
    const day = `2026-09-${String(d).padStart(2, '0')}`;
    const dow = new Date(`${day}T00:00:00Z`).getUTCDay();
    days.push(dow === 0 || dow === 6 ? weekend(day) : rec({ day }));
  }
  const band = bandOf({ days }, { archive: ARCHIVE });
  const calendarSessions = days.filter((d) => isSessionCode(d.calendar.code)).length;
  assert.equal(calendarSessions, 13);
  assert.equal(sessionCount(band.cells), calendarSessions);
  // The legend agrees: four closed weekend days, no day "not imported".
  const legend = tally(band.cells);
  assert.deepEqual(
    legend.map((k) => [k.kind, k.n]),
    [
      ['closed', 4],
      ['session', 13]
    ]
  );
  // A SPECIAL SESSION ON A HOLIDAY (code P) is a session; a halt day (H) and
  // an unmeasured day (U) are sessions; a closed day and a day past the
  // calendar are not.
  const odd = bandOf({
    days: [
      rec({ day: '2024-11-01', calendar: { code: 'P', kind: 'special' }, says: ['special session'] }),
      rec({ day: '2024-11-02', calendar: { code: 'H' } }),
      rec({ day: '2024-11-03', calendar: { code: 'U' } }),
      weekend('2024-11-09'),
      rec({ day: '2031-01-01', calendar: { code: '-' }, held: { bars: null } })
    ]
  });
  assert.equal(sessionCount(odd.cells), 3);
  assert.equal(odd.cells.find((c) => c.day === '2024-11-01')?.kind, 'special');
  for (const c of ['R', 'X', 'P', 'H', 'U']) assert.equal(isSessionCode(c), true, c);
  for (const c of ['.', '-', '', null, undefined, 'r']) assert.equal(isSessionCode(c), false, String(c));
  assert.equal(sessionCount([]), 0);
});

test('an_empty_weekend_window_says_every_day_is_closed', () => {
  const cells = [weekend('2026-09-26'), weekend('2026-09-27')].map((r) => cellOf(r, { archive: ARCHIVE }));
  assert.equal(emptyWindowWhy(cells, '2026-09-26', '2026-09-27', label), 'No bar in 2 days: every one is closed (closed: a weekend).');
  // One closed day before the span says closed, never "before the archive".
  const early = [cellOf(weekend('2018-08-04'), { archive: ARCHIVE })];
  assert.equal(emptyWindowWhy(early, '2018-08-04', '2018-08-04', label), 'No bar on [2018-08-04]: closed: a weekend.');
  // A window of a closed day and a day past the calendar is not "every one
  // closed": the second is not known to be.
  const mixed = [cellOf(weekend('2030-12-28')), cellOf(rec({ day: '2031-01-01', calendar: { code: '-' }, held: { bars: null }, says: [] }))];
  assert.match(emptyWindowWhy(mixed, '', '', label) ?? '', /^No bar in 2 days: none is a session \(closed: a weekend; outside the calendar's span\)\.$/);
  // A window with a session among the weekend names only the session.
  const withSession = [weekend('2026-09-26'), weekend('2026-09-27'), unimported('2026-09-28')].map((r) => cellOf(r, { archive: ARCHIVE }));
  assert.equal(emptyWindowWhy(withSession, '2026-09-26', '2026-09-28', label), 'No bar on [2026-09-28]: not imported.');
  // 2018-08 BEFORE THE SPAN: sessions and weekends — only the sessions are
  // counted as session days.
  const aug = [weekend('2018-08-04'), weekend('2018-08-05'), unimported('2018-08-06'), unimported('2018-08-07')].map((r) => cellOf(r, { archive: ARCHIVE }));
  assert.equal(
    emptyWindowWhy(aug, '2018-08-04', '2018-08-07', label),
    "No bar on any of 2 session day(s) — [2018-08-06]: before the vendor's archive · [2018-08-07]: before the vendor's archive."
  );
});

test('a_two_valued_second_is_not_a_minute_gap', () => {
  const dual = cellOf(
    rec({
      day: '2026-09-25',
      quality: { indices: { holes_5s: 0, hole_max_s: 0, dual_spread_s: 1, seconds: 22222, session_s: 22222 }, stocks: null, options: null },
      says: ['1 second(s) holding two values at least 10 bps apart, up to 14 bps']
    })
  );
  assert.deepEqual(dual.kinds, ['dual']);
  assert.equal(dual.word, 'two values in one second');
  assert.doesNotMatch(dual.word, /minute|gap/);
  assert.equal(dual.flagged, true);
  // A DAY WITH BOTH a real minute hole (Zerodha's `gaps.missing`) and a
  // two-valued second carries both kinds and both words.
  const both = cellOf(
    rec({ gaps: { missing: 3, why: 'vendor_empty' }, quality: { indices: { holes_5s: 0, dual_spread_s: 2 }, stocks: null, options: null } })
  );
  assert.deepEqual(both.kinds, ['gap', 'dual']);
  assert.equal(both.word, 'minute gap');
  const words = new Map(DAY_KINDS.map((k) => [k.kind, k.word]));
  assert.equal(words.get('gap'), 'minute gap', "Zerodha's missing minutes keep their word");
  assert.equal(words.get('dual'), 'two values in one second');
  // The stocks file's holes shared by most shares are their own kind too.
  const shared = cellOf(rec({ quality: { indices: null, stocks: { shared_holes: 1 }, options: null } }));
  assert.deepEqual(shared.kinds, ['shared_hole']);
  assert.doesNotMatch(shared.word, /minute/);
  // The chart draws hole bands only where time is missing: a minute gap, a
  // hole of seconds, a shared hole — never on a day whose only fault is a
  // two-valued second (chart.js holeBands).
  assert.equal(isHoleDay(dual), false);
  assert.equal(isHoleDay(both), true);
  assert.equal(isHoleDay(shared), true);
  assert.equal(isHoleDay(cellOf(rec({ quality: { indices: { holes_5s: 1 }, stocks: null, options: null } }))), true);
  assert.equal(isHoleDay(cellOf(rec())), false);
});

test('a_5_second_hole_is_not_called_a_minute', () => {
  const hole = cellOf(rec({ quality: { indices: { holes_5s: 2, hole_max_s: 7, dual_spread_s: 0 }, stocks: null, options: null } }));
  assert.deepEqual(hole.kinds, ['hole']);
  assert.equal(hole.word, 'holes of 5 s or more');
  assert.doesNotMatch(hole.word, /minute/);
  // All three quality counts on one day: three kinds, in the fixed order.
  assert.deepEqual(
    kindsOf(rec({ quality: { indices: { holes_5s: 1, dual_spread_s: 1 }, stocks: { shared_holes: 1 }, options: null } })),
    ['hole', 'shared_hole', 'dual']
  );
  // A null ledger is not measured: no kind at all.
  assert.deepEqual(kindsOf(rec({ quality: { indices: null, stocks: null, options: null } })), ['session']);
});

test("the stocks file's auction count on an INDEX record is labelled as the whole stocks file's", () => {
  const phrase = '7 closing-auction print(s), kept apart from trades';
  const idx = cellOf(rec({ quality: { indices: null, stocks: { auction_prints: 7 }, options: null }, says: [phrase, 'stocks: 3 LTQ=0 row(s) carry a new price (flagged; bars unchanged)'] }));
  assert.deepEqual(idx.says, [`stocks file, all shares: ${phrase}`, 'stocks: 3 LTQ=0 row(s) carry a new price (flagged; bars unchanged)']);
  // On a share it is that share's run's line and stays as the server wrote it.
  const share = cellOf(rec({ instrument: 'NSE-CASH-FIXTURESHARE', quality: { indices: null, stocks: { auction_prints: 7 }, options: null }, says: [phrase] }));
  assert.deepEqual(share.says, [phrase]);
  // A phrase that does not match the count is not touched.
  const other = cellOf(rec({ quality: { indices: null, stocks: { auction_prints: 8 }, options: null }, says: [phrase] }));
  assert.deepEqual(other.says, [phrase]);
});

test('a closed day whose lead kind is a vendor fault is still closed: the session count and the empty window read the calendar code', () => {
  // AH-03 REPAIR 1 (the second agent's mutants M02 and M03): every closed
  // day above led with 'closed', so the count and the empty-window reason
  // could fall back to the lead-kind rule unseen. A weekend that carries
  // junk rows, or late rows the import deferred, leads with that fault.
  const junky = (/** @type {string} */ day) =>
    rec({ ...weekend(day), junk: [{ kind: 'fixture_junk', rows: 2 }], says: ['closed: a weekend', 'junk rows: 2 (fixture)'] });
  const late = (/** @type {string} */ day) => rec({ ...weekend(day), import: { outcome: 'none', reached: false, late: 3 } });
  const cells = [junky('2026-09-26'), late('2026-09-27')].map((r) => cellOf(r, { archive: ARCHIVE }));
  assert.equal(cells[0].kind, 'junk');
  assert.ok(cells[0].kinds.includes('closed'));
  assert.equal(cells[1].kind, 'late');
  assert.ok(cells[1].kinds.includes('closed'));
  assert.equal(sessionCount(cells), 0);
  assert.equal(emptyWindowWhy(cells, '2026-09-26', '2026-09-27', label), 'No bar in 2 days: every one is closed (closed: a weekend).');
  // Beside a real session the count is the calendar's, one.
  const withSession = [...cells, cellOf(unimported('2026-09-28'), { archive: ARCHIVE })];
  assert.equal(sessionCount(withSession), 1);
  assert.equal(emptyWindowWhy(withSession, '2026-09-26', '2026-09-28', label), 'No bar on [2026-09-28]: not imported.');
});

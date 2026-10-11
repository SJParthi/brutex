// WP-61: how /ingest judges a month file and a series (DB-G20, DB-G23,
// DB-B38, DB-B46). Every fixture is invented; no vendor row is copied.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  VERDICT_ORDER, VERDICTS, defaultWindow, emptySeen, feedSpan, monthVerdict, outsideSpan,
  refusalBatches, refusalIndex, refusalSentence, refusedMonth, seriesVerdict, unsettledSpans
} from '../src/lib/ingest-months.js';

const lastDay = (/** @type {string} */ ym) => {
  const [y, m] = ym.split('-').map(Number);
  return `${ym}-${String(new Date(Date.UTC(y, m, 0)).getUTCDate()).padStart(2, '0')}`;
};

// The shape `/ingest/refusals.json` sends (crates/api/src/ingest_refusals.rs).
const refusals = {
  feed: 'zerodha',
  rung_note: 'a pull journal record carries no timeframe',
  symbols: [
    {
      symbol: 'AAA',
      count: 5,
      months: {
        '2020-11': { count: 4, days: ['2020-11-14'], more_day_records: 0, note: '2020-11-14: 60 minute row(s) refused: the exchange calendar ', cut: true, record: 901, at_unix_secs: 1_791_000_000, window: { from: '2018-01-01', to: '2026-09-30' } }
      },
      whole: [{ window: { from: '2023-03-01', to: '2023-04-30' }, note: 'the whole window was refused', cut: false, record: 950, at_unix_secs: 1_791_000_100 }],
      whole_count: 1,
      whole_more: 0
    },
    { symbol: 'BBB', count: 0, months: {}, whole: [], whole_count: 0, whole_more: 0 }
  ],
  errors: [{ symbol: 'CCC', error: 'the journal is being walked at start' }]
};

test('every verdict has a tone, a label and a meaning, in one worst-first order', () => {
  assert.deepEqual(Object.keys(VERDICTS).sort(), [...VERDICT_ORDER].sort());
  assert.deepEqual(Object.keys(emptySeen()).sort(), [...VERDICT_ORDER].sort());
  for (const k of VERDICT_ORDER) {
    assert.equal(VERDICTS[k].length, 3);
    assert.ok(VERDICTS[k][2].length > 40, k);
  }
  assert.equal(VERDICTS.flight[1], 'in flight', 'DB-G20: never "retrying"');
  assert.equal(VERDICTS.refused[1], 'requested and refused', 'DB-G23');
  assert.equal(VERDICTS.partial[1], 'partly never pulled', 'DB-B38');
  assert.match(VERDICTS.short[2], /^Every month file in this window has been written/);
  assert.ok(VERDICT_ORDER.indexOf('refused') < VERDICT_ORDER.indexOf('never'));
});

test('DB-G23: a month that is missing or short and refused is "requested and refused", never "never pulled"', () => {
  const { bySymbol, errors, rungNote } = refusalIndex(refusals);
  assert.equal(rungNote, 'a pull journal record carries no timeframe');
  assert.equal(errors.get('CCC'), 'the journal is being walked at start');
  assert.equal(bySymbol.has('CCC'), false, 'an unread symbol is not an empty answer');
  const named = refusedMonth(bySymbol, 'AAA', '2020-11');
  assert.equal(named?.count, 4);
  assert.equal(named?.whole, false);
  assert.equal(refusedMonth(bySymbol, 'AAA', '2020-12'), null);
  assert.equal(refusedMonth(bySymbol, 'BBB', '2020-11'), null, 'another symbol’s refusal is not this one’s');
  assert.equal(refusedMonth(bySymbol, 'ZZZ', '2020-11'), null);
  // A whole-window refusal covers every month of its window and no other.
  assert.equal(refusedMonth(bySymbol, 'AAA', '2023-03')?.whole, true);
  assert.equal(refusedMonth(bySymbol, 'AAA', '2023-04')?.whole, true);
  assert.equal(refusedMonth(bySymbol, 'AAA', '2023-05'), null);

  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 7185, refused: true }), 'refused');
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 7185, refused: false }), 'never');
  assert.equal(monthVerdict({ beyond: false, bars: 7124, expect: 7185, refused: true }), 'refused');
  assert.equal(monthVerdict({ beyond: false, bars: 7124, expect: 7185, refused: false }), 'short');
  // A refusal never spoils a month that holds everything, or one out of reach.
  assert.equal(monthVerdict({ beyond: false, bars: 7185, expect: 7185, refused: true }), 'ok');
  assert.equal(monthVerdict({ beyond: true, bars: null, expect: null, refused: true }), 'beyond');
  assert.equal(monthVerdict({ beyond: false, bars: 12, expect: null, refused: true }), 'unknown');
  // Zero bars in a written file is zero, and is short of any expectation.
  assert.equal(monthVerdict({ beyond: false, bars: 0, expect: 1, refused: false }), 'short');
  assert.equal(monthVerdict({ beyond: false, bars: 0, expect: 0, refused: false }), 'ok');

  const said = refusalSentence(/** @type {any} */ (named), (secs) => `T${secs}`);
  assert.match(said, /^requested and refused — 4 refusal record\(s\) name 2020-11-14, newest written T1791000000: “2020-11-14: 60 minute row\(s\) refused: the exchange calendar …” \(journal record 901;/);
  assert.match(refusalSentence(/** @type {any} */ (refusedMonth(bySymbol, 'AAA', '2023-03'))), /a whole window that covers this month was refused/);
});

test('the refusals fold tolerates an empty, a null and a malformed body', () => {
  for (const body of [null, undefined, {}, { symbols: 'x' }, { symbols: [null, { months: {} }] }]) {
    const out = refusalIndex(body);
    assert.equal(out.bySymbol.size, 0);
    assert.equal(out.errors.size, 0);
  }
});

test('a table’s symbols are asked in batches the route accepts, each symbol once', () => {
  assert.deepEqual(refusalBatches([]), []);
  assert.deepEqual(refusalBatches(['A', 'A', '', 'B']), [['A', 'B']]);
  const many = Array.from({ length: 250 }, (_, i) => `S${i}`);
  const batches = refusalBatches(many);
  assert.deepEqual(batches.map((b) => b.length), [120, 120, 10]);
  assert.deepEqual(batches.flat(), many);
  assert.deepEqual(refusalBatches(many, 250).map((b) => b.length), [250]);
});

test('DB-B38: a series with bars and unwritten months is "partly never pulled", not "short"', () => {
  const seen = (/** @type {Partial<ReturnType<typeof emptySeen>>} */ over) => ({ ...emptySeen(), ...over });
  const v = (/** @type {any} */ s) => seriesVerdict({ months: 142, got: 0, inFlight: false, halted: false, ...s });
  assert.equal(v({ seen: seen({ never: 36, ok: 79, short: 27 }), got: 20_664_881 }), 'partial');
  assert.equal(v({ seen: seen({ never: 142 }), got: 0 }), 'never');
  assert.equal(v({ seen: seen({ short: 3, ok: 139 }), got: 500 }), 'short');
  assert.equal(v({ seen: seen({ ok: 142 }), got: 500 }), 'ok');
  assert.equal(v({ seen: seen({ unknown: 142 }), got: 500 }), 'unknown');
  assert.equal(v({ seen: seen({ beyond: 142 }), got: 0 }), 'beyond');
  // A refusal outranks never, partial and short: it is the one with a reason to read.
  assert.equal(v({ seen: seen({ refused: 5, never: 3, ok: 134 }), got: 500 }), 'refused');
  assert.equal(v({ seen: seen({ refused: 5, never: 137 }), got: 0 }), 'refused');
  assert.equal(v({ seen: seen({ refused: 1, short: 2, ok: 139 }), got: 500 }), 'refused');
  // A halt makes anything missing "failed"; nothing missing stays what it is.
  assert.equal(v({ seen: seen({ never: 1, ok: 141 }), got: 5, halted: true }), 'fail');
  assert.equal(v({ seen: seen({ refused: 1, ok: 141 }), got: 5, halted: true }), 'fail');
  assert.equal(v({ seen: seen({ ok: 142 }), got: 5, halted: true }), 'ok');
  // DB-G20: in flight outranks everything but out-of-reach.
  assert.equal(v({ seen: seen({ never: 142 }), inFlight: true }), 'flight');
  assert.equal(v({ seen: seen({ beyond: 142 }), inFlight: true }), 'beyond');
  // No months at all (an empty window) is not "out of reach".
  assert.equal(seriesVerdict({ seen: seen({}), months: 0, got: 0, inFlight: false, halted: false }), 'ok');
});

test('DB-B38: a row’s pull asks each run of unsettled months, never the settled months between', () => {
  const months = [
    ['2018-01', 'ok'], ['2018-02', 'never'], ['2018-03', 'never'], ['2018-04', 'ok'],
    ['2018-05', 'beyond'], ['2018-06', 'refused'], ['2018-07', 'ok'], ['2018-08', 'short']
  ].map(([month, k]) => ({ month, k: /** @type {any} */ (k) }));
  assert.deepEqual(unsettledSpans(months, '2018-01-01', '2018-08-31', lastDay), [
    { from: '2018-02-01', to: '2018-03-31', months: 2 },
    { from: '2018-06-01', to: '2018-06-30', months: 1 },
    { from: '2018-08-01', to: '2018-08-31', months: 1 }
  ]);
  // Clipped to the window the operator chose, at both ends.
  assert.deepEqual(unsettledSpans(months, '2018-02-10', '2018-08-05', lastDay), [
    { from: '2018-02-10', to: '2018-03-31', months: 2 },
    { from: '2018-06-01', to: '2018-06-30', months: 1 },
    { from: '2018-08-01', to: '2018-08-05', months: 1 }
  ]);
  assert.deepEqual(unsettledSpans([], '2018-01-01', '2018-12-31', lastDay), []);
  assert.deepEqual(unsettledSpans([{ month: '2018-01', k: 'ok' }], '2018-01-01', '2018-01-31', lastDay), []);
  assert.deepEqual(unsettledSpans([{ month: '2018-01', k: 'never' }], '2018-01-01', '2018-01-31', lastDay), [
    { from: '2018-01-01', to: '2018-01-31', months: 1 }
  ]);
  // Every month unsettled is one span, the whole window.
  const all = ['2019-11', '2019-12', '2020-01'].map((month) => ({ month, k: /** @type {any} */ ('never') }));
  assert.deepEqual(unsettledSpans(all, '2019-11-01', '2020-01-31', lastDay), [{ from: '2019-11-01', to: '2020-01-31', months: 3 }]);
  // Two unsettled months that are NOT consecutive in the list are never joined.
  const gapped = [{ month: '2020-11', k: /** @type {any} */ ('refused') }, { month: '2024-11', k: /** @type {any} */ ('refused') }];
  assert.deepEqual(unsettledSpans(gapped, '2018-01-01', '2026-09-30', lastDay), [
    { from: '2020-11-01', to: '2020-11-30', months: 1 },
    { from: '2024-11-01', to: '2024-11-30', months: 1 }
  ]);
});

test('DB-B46, DB-B38: a feed’s stated span bounds the window; an archive’s binds both ends', () => {
  const gdfl = feedSpan({ archive: { from: '2018-09-03', to: '2026-09-24', source: 'measured' }, window: null });
  assert.deepEqual([gdfl.kind, gdfl.from, gdfl.to, gdfl.binds], ['archive', '2018-09-03', '2026-09-24', true]);
  const zerodha = feedSpan({ archive: null, window: { from: '2018-01-01', to: '2026-09-30' } });
  assert.deepEqual([zerodha.kind, zerodha.from, zerodha.to, zerodha.binds], ['window', '2018-01-01', '2026-09-30', false]);
  const silent = feedSpan({ archive: null, window: null });
  assert.equal(silent.kind, null);
  assert.equal(feedSpan(null).kind, null);
  assert.equal(feedSpan({ archive: { from: 'soon', to: '2026-09-24' } }).kind, null, 'a span that is not two days is no span');

  // 44 months before the archive and the month after it are out of reach.
  assert.equal(outsideSpan(gdfl, '2015-01', lastDay), true);
  assert.equal(outsideSpan(gdfl, '2018-08', lastDay), true);
  assert.equal(outsideSpan(gdfl, '2018-09', lastDay), false, 'the archive opens inside this month');
  assert.equal(outsideSpan(gdfl, '2026-09', lastDay), false);
  assert.equal(outsideSpan(gdfl, '2026-10', lastDay), true);
  // The operator's window opens the form and binds nothing: Zerodha answers for 2015.
  assert.equal(outsideSpan(zerodha, '2015-01', lastDay), false);
  assert.equal(outsideSpan(silent, '2015-01', lastDay), false);

  const bounds = { floor: '2015-01-01', ceiling: '2026-10-09' };
  assert.deepEqual(defaultWindow(zerodha, bounds), { from: '2018-01-01', to: '2026-10-09', why: 'your window for this feed opens 2018-01-01' });
  const g = defaultWindow(gdfl, bounds);
  assert.deepEqual([g.from, g.to], ['2018-09-03', '2026-09-24']);
  assert.match(g.why, /archive this feed was imported from spans 2018-09-03 to 2026-09-24/);
  assert.deepEqual([defaultWindow(silent, bounds).from, defaultWindow(silent, bounds).to], ['2015-01-01', '2026-10-09']);
  // A span older than the page's own floor never opens before the floor.
  assert.equal(defaultWindow(feedSpan({ window: { from: '2010-01-01', to: '2026-09-30' } }), bounds).from, '2015-01-01');
  // A ceiling older than the span's first day cannot produce a backwards window.
  const early = defaultWindow(zerodha, { floor: '2015-01-01', ceiling: '2017-06-30' });
  assert.ok(early.from <= early.to, JSON.stringify(early));
});

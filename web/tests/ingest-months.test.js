// WP-61: how /ingest judges a month file and a series (DB-G20, DB-G23,
// DB-B38, DB-B46). Every fixture is invented; no vendor row is copied.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  VERDICT_ORDER, VERDICTS, archiveLane, defaultWindow, emptySeen, feedSpan, monthVerdict, outsideSpan,
  refusalBatches, refusalErrorsSentence, refusalIndex, refusalJoin, refusalSentence, refusedMonth,
  seriesVerdict, unsettledSpans, verdictWords, wholeUnlisted
} from '../src/lib/ingest-months.js';
import { readFileSync } from 'node:fs';

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
  // No months at all (an empty window) is not "out of reach", and it is not
  // "verified" either: nothing was judged, so the verdict is withheld.
  assert.equal(seriesVerdict({ seen: seen({}), months: 0, got: 0, inFlight: false, halted: false }), 'unknown');
  assert.equal(seriesVerdict({ seen: seen({}), months: /** @type {any} */ (NaN), got: 0, inFlight: false, halted: false }), 'unknown');
  assert.equal(seriesVerdict({ seen: seen({ ok: 1 }), months: 1, got: 1, inFlight: false, halted: false }), 'ok');
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

// ---------------------------------------------------------------------------
// WP-61 repair: a refusals answer that could not be read for an instrument is
// never "never pulled". The server answers 200 with errors:[{symbol,error}];
// the page stored that list and read only `symbols`, so the month fell to
// "never pulled — nothing has asked the vendor for it".
// ---------------------------------------------------------------------------
/** The page's reading of one answer, as `+page.svelte` holds it. */
const reading = (/** @type {any} */ body, /** @type {any} */ over = {}) => {
  const folded = refusalIndex(body);
  return { feed: 'zerodha', state: 'ready', bySymbol: folded.bySymbol, errors: folded.errors, why: null, ...over };
};

test('a per-symbol read error is "refusals unread", never "never pulled"', () => {
  // The verifier's body: the journal answered for no symbol and named one error.
  const held = reading({ symbols: [], errors: [{ symbol: 'QQQ', error: 'the refusals index is being walked' }] });
  assert.equal(refusedMonth(held.bySymbol, 'QQQ', '2020-11'), null, 'no refusal is on hand for it');
  const join = refusalJoin(held, 'zerodha', 'QQQ');
  assert.equal(join.read, false);
  assert.match(join.why, /could not read this instrument’s refusals: the refusals index is being walked/);
  // The month with no file, and the month that is short: both withheld.
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 100, refused: false, unread: !join.read }), 'unread');
  assert.equal(monthVerdict({ beyond: false, bars: 40, expect: 100, refused: false, unread: !join.read }), 'unread');
  // What does not depend on the journal is unchanged.
  assert.equal(monthVerdict({ beyond: true, bars: null, expect: 100, refused: false, unread: true }), 'beyond');
  assert.equal(monthVerdict({ beyond: false, bars: 100, expect: 100, refused: false, unread: true }), 'ok');
  assert.equal(monthVerdict({ beyond: false, bars: 5, expect: null, refused: false, unread: true }), 'unknown');
  // A refusal on hand still wins: it is the stronger statement.
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 100, refused: true, unread: true }), 'refused');
  // And an instrument the journal DID answer for is judged as before.
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 100, refused: false, unread: false }), 'never');
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 100, refused: false }), 'never');
});

test('refusalJoin: read only when this feed\'s answer lists the instrument and no error', () => {
  const ok = reading({ symbols: [{ symbol: 'AAA', count: 0, months: {} }], errors: [{ symbol: 'BBB', error: 'torn index' }] });
  assert.deepEqual(refusalJoin(ok, 'zerodha', 'AAA'), { read: true, why: '' });
  assert.equal(refusalJoin(ok, 'zerodha', 'BBB').read, false);
  // Listed in BOTH (a server that contradicts itself): the error wins.
  const both = reading({ symbols: [{ symbol: 'AAA', count: 0, months: {} }], errors: [{ symbol: 'AAA', error: 'x' }] });
  assert.equal(refusalJoin(both, 'zerodha', 'AAA').read, false);
  // In NEITHER list: the answer did not cover it, and that is said.
  assert.match(refusalJoin(ok, 'zerodha', 'CCC').why, /neither among its symbols nor among its errors/);
  // Another feed's answer is not this feed's.
  assert.match(refusalJoin(ok, 'gdfl', 'AAA').why, /not been read for this feed/);
  assert.equal(refusalJoin(null, 'zerodha', 'AAA').read, false);
  assert.equal(refusalJoin(ok, null, 'AAA').read, false);
  // The whole read failed, was refused, is in flight, or was never asked.
  const empty = { symbols: [], errors: [] };
  assert.equal(refusalJoin(reading(empty, { state: 'error', why: '/ingest/refusals.json could not be read: HTTP 500' }), 'zerodha', 'AAA').why,
    '/ingest/refusals.json could not be read: HTTP 500');
  assert.match(refusalJoin(reading(empty, { state: 'unsupported', why: 'no pull journal for gdfl' }), 'zerodha', 'AAA').why,
    /refused to answer for this feed’s refusals: no pull journal for gdfl/);
  assert.match(refusalJoin(reading(empty, { state: 'reading' }), 'zerodha', 'AAA').why, /still being read/);
  assert.match(refusalJoin(reading(empty, { state: 'idle' }), 'zerodha', 'AAA').why, /were not asked for/);
  // A RE-READ KEEPS THE LAST ANSWER: an instrument already answered for stays read.
  assert.equal(refusalJoin(reading({ symbols: [{ symbol: 'AAA', count: 0, months: {} }] }, { state: 'reading' }), 'zerodha', 'AAA').read, true);
});

test('a series with unread refusals is "refusals unread", before "never pulled" and "short"', () => {
  const seen = (/** @type {any} */ over) => ({ ...emptySeen(), ...over });
  assert.equal(seriesVerdict({ seen: seen({ unread: 3 }), months: 3, got: 0, inFlight: false, halted: false }), 'unread');
  assert.equal(seriesVerdict({ seen: seen({ unread: 1, ok: 2 }), months: 3, got: 900, inFlight: false, halted: false }), 'unread');
  // A halted ladder still reads failed, and a cell in flight in flight.
  assert.equal(seriesVerdict({ seen: seen({ unread: 1, ok: 2 }), months: 3, got: 900, inFlight: false, halted: true }), 'fail');
  assert.equal(seriesVerdict({ seen: seen({ unread: 1 }), months: 1, got: 0, inFlight: true, halted: false }), 'flight');
  assert.ok(VERDICT_ORDER.indexOf('unread') < VERDICT_ORDER.indexOf('never'));
  assert.equal(VERDICTS.unread[1], 'refusals unread');
  assert.match(VERDICTS.unread[2], /NOT KNOWN/);
});

test('the notice names how many instruments are unread and the first few, in the server\'s words', () => {
  assert.equal(refusalErrorsSentence(new Map()), '');
  assert.equal(refusalErrorsSentence(/** @type {any} */ (null)), '');
  const one = refusalErrorsSentence(new Map([['QQQ', 'torn index']]));
  assert.match(one, /could not be read for 1 instrument\(s\) — QQQ: torn index\. /);
  assert.match(one, /“never pulled” is not claimed/);
  const many = refusalErrorsSentence(new Map(Array.from({ length: 1_250 }, (_, i) => [`S${i}`, 'x'])));
  assert.match(many, /for 1,250 instrument\(s\) — S0: x; S1: x; S2: x; and 1,247 more\./);
});

test('the page judges a month through the join and draws every unread state', () => {
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /const join = isBroker \? refusalJoin\(refusals, feedNow, m\.symbol\) : \{ read: true, why: '' \}/);
  assert.match(page, /monthVerdict\(\{ beyond, bars, expect, sessions, refused: why !== null, unread: s\.key === 'spot' && unreadWhy !== '' \}\)/);
  assert.match(page, /const unreadWhy = !join\.read\s+\? join\.why\s+: unlisted > 0/);
  assert.match(page, /unread: k === 'unread' \? unreadWhy : ''/);
  // The per-symbol errors are read, and each unread state has its own notice.
  assert.match(page, /refusals\.state === 'ready' && refusals\.errors\.size > 0/);
  assert.match(page, /data-ingest-refusals="unread-symbols"[\s\S]{0,200}refusalErrorsSentence\(refusals\.errors\)/);
  assert.match(page, /data-ingest-refusals="unsupported"/);
  // The old notice said the rows would read "never pulled" anyway.
  assert.doesNotMatch(page, /reads "never pulled" below without the journal/);
});

test('a month that owes no bar is not "never pulled"; a refusal in it is still shown', () => {
  // No file, and the days inside the window hold no session: nothing is owed.
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 0, sessions: 0, refused: false }), 'ok');
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 0, sessions: 0, refused: false, unread: true }), 'ok');
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 0, sessions: 0, refused: true }), 'refused');
  // An expectation of 0 with sessions in the month (a denominator that was not
  // found), or with no session count stated, proves nothing owed: still a gap.
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 0, sessions: 21, refused: false }), 'never');
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 0, refused: false }), 'never');
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 0, sessions: 21, refused: false, unread: true }), 'unread');
  // One bar owed and no file is still a gap; no yardstick is still withheld.
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 1, refused: false }), 'never');
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: null, refused: false }), 'never');
  assert.equal(monthVerdict({ beyond: false, bars: 0, expect: 0, refused: false }), 'ok');
  assert.equal(monthVerdict({ beyond: false, bars: 0, expect: 1, refused: false }), 'short');
});

test('a whole-window refusal the server counted and did not list leaves the month unread', () => {
  // The route lists the newest 4 whole-window refusals and counts the rest.
  const body = {
    symbols: [
      { symbol: 'AAA', count: 9, months: {}, whole_count: 6, whole_more: 2,
        whole: [
          { window: { from: '2024-01-01', to: '2024-01-31' }, note: 'n', cut: false, record: 4, at_unix_secs: 4 },
          { window: { from: '2024-02-01', to: '2024-02-29' }, note: 'n', cut: false, record: 3, at_unix_secs: 3 },
          { window: { from: '2024-03-01', to: '2024-03-31' }, note: 'n', cut: false, record: 2, at_unix_secs: 2 },
          { window: { from: '2024-04-01', to: '2024-04-30' }, note: 'n', cut: false, record: 1, at_unix_secs: 1 }
        ] },
      { symbol: 'BBB', count: 1, months: {}, whole_count: 1, whole_more: 0,
        whole: [{ window: { from: '2024-01-01', to: '2024-01-31' }, note: 'n', cut: false, record: 9, at_unix_secs: 9 }] },
      // An answer that names no count is every listed one, never a guess.
      { symbol: 'CCC', count: 1, months: {}, whole: [{ window: { from: '2024-01-01', to: '2024-01-31' }, note: 'n' }] }
    ]
  };
  const { bySymbol } = refusalIndex(body);
  assert.equal(wholeUnlisted(bySymbol, 'AAA'), 2);
  assert.equal(wholeUnlisted(bySymbol, 'BBB'), 0);
  assert.equal(wholeUnlisted(bySymbol, 'CCC'), 0);
  assert.equal(wholeUnlisted(bySymbol, 'ZZZ'), 0, 'an instrument the answer does not name');
  assert.equal(wholeUnlisted(null, 'AAA'), 0);
  assert.equal(wholeUnlisted(/** @type {any} */ ({}), 'AAA'), 0);
  // A month a LISTED refusal covers is joined; one none covers is the case
  // the count is for: before, it read "never pulled".
  assert.equal(refusedMonth(bySymbol, 'AAA', '2024-02')?.whole, true);
  assert.equal(refusedMonth(bySymbol, 'AAA', '2019-06'), null);
  assert.equal(monthVerdict({ beyond: false, bars: null, expect: 8250, refused: false, unread: wholeUnlisted(bySymbol, 'AAA') > 0 }), 'unread');
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /const unlisted = why === null && join\.read && refused !== null && s\.key === 'spot' \? wholeUnlisted\(refused, m\.symbol\) : 0/);
  assert.match(page, /more whole-window refusal\(s\) of this instrument than \/ingest\/refusals\.json lists/);
});

test('an imported feed’s rows are worded as imports, with the same keys, tones and order', () => {
  assert.equal(verdictWords(false), VERDICTS, 'a pulled feed reads the one table');
  const words = verdictWords(true);
  assert.deepEqual(Object.keys(words).sort(), [...VERDICT_ORDER].sort());
  for (const k of VERDICT_ORDER) {
    assert.equal(words[k][0], VERDICTS[k][0], `${k}: the tone is the table's`);
    assert.equal(words[k].length, 3);
  }
  assert.equal(words.never[1], 'never imported');
  assert.equal(words.partial[1], 'partly never imported');
  // No sentence of an imported feed tells the reader to pull.
  for (const k of /** @type {const} */ (['never', 'partial', 'short'])) {
    assert.match(words[k][2], /imported from an archive, not pulled/, k);
    assert.doesNotMatch(words[k][2], /row’s Pull|A pull is warranted|pull of those months is warranted/, k);
  }
  assert.equal(words.ok, VERDICTS.ok);
  assert.equal(words.beyond, VERDICTS.beyond);
  // The pulled feed's own words no longer claim what the journal cannot show.
  assert.doesNotMatch(VERDICTS.never[2], /nothing has asked the vendor for it\. A pull IS warranted/);
  assert.match(VERDICTS.never[2], /cannot tell them apart/);
  assert.match(VERDICTS.never[2], /before the instrument was listed/);
  assert.match(VERDICTS.short[2], /a member failure that is not a refusal[^.]*is NOT joined here/);
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /const VERDICT = \$derived\(verdictWords\(isImported\)\)/);
  assert.doesNotMatch(page, /const VERDICT = VERDICTS;/);
});

test('an archive’s measured span is its reach on the strip, never "reach not stated"', () => {
  const gdflLike = feedSpan({ archive: { from: '2018-09-03', to: '2026-09-24', source: 'measured' } });
  // The window equals the archive: all of it is reachable.
  assert.deepEqual(archiveLane(gdflLike, '2018-09-03', '2026-09-24'), { from: '2018-09-03', to: '2026-09-24', beforePct: 0, livePct: 100, afterPct: 0 });
  // A window that opens 10 days early and closes 10 days late: 10 + 80 + 10.
  const tenth = archiveLane(feedSpan({ archive: { from: '2024-01-11', to: '2024-03-31' } }), '2024-01-01', '2024-04-10');
  assert.equal(tenth?.beforePct, 10);
  assert.equal(tenth?.livePct, 80);
  assert.equal(Math.round(/** @type {number} */ (tenth?.afterPct)), 10);
  // A window wholly before or after the archive reaches none of it.
  assert.deepEqual(archiveLane(gdflLike, '2015-01-01', '2016-01-01'), { from: '2018-09-03', to: '2026-09-24', beforePct: 100, livePct: 0, afterPct: 0 });
  assert.equal(archiveLane(gdflLike, '2027-01-01', '2027-06-01')?.livePct, 0);
  // A single-day archive inside the window is a zero-width reach, not NaN.
  const one = archiveLane(feedSpan({ archive: { from: '2024-01-06', to: '2024-01-06' } }), '2024-01-01', '2024-01-11');
  assert.equal(one?.livePct, 0);
  assert.equal(one?.beforePct, 50);
  // A pulled feed, no span, a garbage window or a backwards one: no lane.
  assert.equal(archiveLane(feedSpan({ window: { from: '2018-01-01', to: '2026-09-30' } }), '2018-01-01', '2026-09-30'), null);
  assert.equal(archiveLane(feedSpan({}), '2018-01-01', '2026-09-30'), null);
  assert.equal(archiveLane(/** @type {any} */ (null), '2018-01-01', '2026-09-30'), null);
  assert.equal(archiveLane(gdflLike, 'garbage', '2026-09-30'), null);
  assert.equal(archiveLane(gdflLike, '2026-09-30', '2018-01-01'), null);
  assert.equal(archiveLane(gdflLike, '2020-01-01', '2020-01-01'), null);
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /archiveLane\(feedSpan\(\/\*\* @type \{any\} \*\/ \(feeds\.all\.find\(\(f\) => f\.wire === leg\.vendor\)\)\), from, to\)/);
  assert.match(page, /data-lane-reach=\{lane\.archive \? 'archive'/);
  // The folder halt no longer denies the store's rows drawn under it.
  assert.doesNotMatch(page, /nothing below is a census\s+of anything/);
  assert.match(page, /they are the bars the STORE already holds/);
});

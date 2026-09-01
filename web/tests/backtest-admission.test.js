import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');

/** @param {string} source @param {string} from @param {string} to */
function between(source, from, to) {
  const start = source.indexOf(from);
  assert.notEqual(start, -1, `missing start marker: ${from}`);
  const end = source.indexOf(to, start);
  assert.notEqual(end, -1, `missing end marker: ${to}`);
  return source.slice(start, end);
}

const vocabSource = between(page, 'function validateVocabEnvelope(payload)', 'async function fetchVocab()');
// This executes the route's actual pure admission function, not a test copy.
const validateVocabEnvelope = Function(
  'MASK_CAPACITY',
  `${vocabSource}; return validateVocabEnvelope;`
)(384);

const barsSource = between(
  page,
  'const integer = (value)',
  "/** @type {{ phase: 'idle'|'loading'|'ready'|'failed', bars: any[]"
);
const { validateBarsWindow, runMonthCount } = Function(
  `${barsSource}; return { validateBarsWindow, runMonthCount };`
)();

/** @param {number} i @param {string} [name] @param {unknown} [live] */
const bit = (i, name = `rule_${i}`, live = true) => ({ i, name, live });

test('vocabulary admission is atomic, dense, bounded, and type exact', () => {
  const good = { vocab_version: 7, count: 3, bits: [bit(0), bit(1), bit(2)] };
  const admitted = validateVocabEnvelope(good);
  assert.equal(admitted.ok, true);
  assert.deepEqual([...admitted.bits.keys()], [0, 1, 2]);

  for (const malformed of [
    null,
    [],
    { ...good, vocab_version: 0 },
    { ...good, vocab_version: 1.5 },
    { ...good, count: 2 },
    { ...good, count: 385, bits: Array.from({ length: 385 }, (_, i) => bit(i)) },
    { ...good, bits: [bit(0), bit(2), bit(1)] },
    { ...good, bits: [bit(-0), bit(1), bit(2)] },
    { ...good, bits: [bit(0), bit(1, 'rule_0'), bit(2)] },
    { ...good, bits: [bit(0), bit(1, ''), bit(2)] },
    { ...good, bits: [bit(0), bit(1, 'rule_1', 1), bit(2)] }
  ]) {
    const refused = validateVocabEnvelope(malformed);
    assert.equal(refused.ok, false, JSON.stringify(malformed)?.slice(0, 160));
    assert.equal(refused.bits.size, 0, 'a malformed table must publish no partial prefix');
  }
});

/** @param {number} t @param {Record<string, unknown>} [overrides] */
function row(t, overrides = {}) {
  return {
    t,
    o: 100,
    h: 110,
    l: 90,
    c: 105,
    v: 0,
    oi: null,
    chg: null,
    chg_why: 'first_bar_in_file',
    oichg: null,
    oichg_why: 'oi_null',
    ...overrides
  };
}

/** @param {any[]} bars @param {Record<string, unknown>} [overrides] */
function windowOf(bars, overrides = {}) {
  return {
    total: bars.length,
    months_read: 2,
    months_missing: 1,
    scanned: false,
    extremes: null,
    faults: null,
    bars,
    ...overrides
  };
}

test('one complete bar-window door rejects truncation, corruption, and wrong order', () => {
  const goodBars = [
    row(100),
    row(200, { chg: 500, chg_why: null, oichg: 0, oichg_why: null })
  ];
  assert.equal(validateBarsWindow(windowOf(goodBars), 'asc', 1000, 3).ok, true);
  assert.equal(validateBarsWindow(windowOf([...goodBars].reverse()), 'desc', 1000, 3).ok, true);

  const malformed = [
    windowOf(goodBars, { total: 3 }),
    windowOf(goodBars, { months_missing: 0 }),
    windowOf(goodBars, { scanned: true }),
    windowOf(goodBars, { extremes: { range: 20, volume: 0 } }),
    windowOf(goodBars, { faults: 'record 1 failed CRC' }),
    windowOf([row(100), row(100)]),
    windowOf([row(200), row(100)]),
    windowOf([row(100, { t: Number.MAX_SAFE_INTEGER + 1 })]),
    windowOf([row(-0)]),
    windowOf([row(100, { h: 99 })]),
    windowOf([row(100, { v: -1 })]),
    windowOf([row(100, { oi: -1 })]),
    windowOf([row(100, { chg: null, chg_why: null })]),
    windowOf([row(100, { chg: 0, chg_why: 'first_bar_in_file' })])
  ];
  for (const body of malformed) {
    assert.equal(validateBarsWindow(body, 'asc', 1000, 3).ok, false);
  }
});

test('month-span arithmetic is inclusive and refuses inverted or malformed ranges', () => {
  assert.equal(
    runMonthCount({ from_year: 2024, from_month: 12, to_year: 2025, to_month: 2 }),
    3
  );
  assert.equal(runMonthCount({ from_year: 2025, from_month: 2, to_year: 2024, to_month: 12 }), null);
  assert.equal(runMonthCount({ from_year: 2025, from_month: 0, to_year: 2025, to_month: 2 }), null);
});

test('series and benchmark both cross the same bar admission door', () => {
  const series = between(page, 'async function loadSeries(run, rung)', '// OPENING A RUN RESETS');
  const benchmark = between(page, 'async function loadBenchmark(run)', 'Current-store buy-and-hold over');
  assert.match(series, /validateBarsWindow\(body, 'desc', MAX_WINDOW_LIMIT, expectedMonths\)/);
  assert.match(series, /&sort=ts&dir=desc&limit=\$\{MAX_WINDOW_LIMIT\}/);
  assert.match(benchmark, /timeframe=\$\{encodeURIComponent\(run\.timeframe\)\}/);
  assert.match(benchmark, /\$\{base\}&dir=asc&limit=1/);
  assert.match(benchmark, /\$\{base\}&dir=desc&limit=1/);
  assert.match(benchmark, /validateBarsWindow\(firstBody, 'asc', 1, expectedMonths\)/);
  assert.match(benchmark, /validateBarsWindow\(lastBody, 'desc', 1, expectedMonths\)/);
  assert.doesNotMatch(benchmark, /chartRung|function loadBenchmark\(run, rung\)/);
  assert.match(page, /exactIntegerDelta\(bench\.close, bench\.open\)/);
});

test('every drill loader drops stale success, failure, catch, and close-panel writes', () => {
  const series = between(page, 'async function loadSeries(run, rung)', '// OPENING A RUN RESETS');
  const rungs = between(page, 'async function loadRungs(run)', 'BUY & HOLD');
  const benchmark = between(page, 'async function loadBenchmark(run)', 'Current-store buy-and-hold over');
  assert.match(series, /catch \(error\) \{\s*if \(seq !== seriesSeq\) return;/);
  assert.match(rungs, /catch \{\s*if \(seq !== rungsSeq\) return;/);
  assert.match(benchmark, /catch \(error\) \{\s*if \(seq !== benchSeq\) return;/);
  assert.match(page, /if \(!run\) \{\s*seriesSeq \+= 1;/);
  assert.match(page, /if \(!run\) \{\s*benchSeq \+= 1;/);
  assert.match(page, /if \(!run\) \{\s*rungsSeq \+= 1;/);
});

test('the ledger loader revokes every stale success, failure, catch, and teardown write', () => {
  const ledger = between(page, 'async function fetchLedger()', '$effect(() =>');
  assert.match(ledger, /const seq = \+\+ledgerSeq/);
  assert.ok(
    (ledger.match(/if \(seq !== ledgerSeq\) return;/g) ?? []).length >= 5,
    'every response, schema, success, failure, and catch publication stays generation-guarded'
  );
  assert.match(ledger, /validateLedgerPayload\(body\)/);
  const initial = between(page, '$effect(() =>', 'STARTING A SWEEP FROM HERE');
  assert.match(initial, /return \(\) => \{\s*ledgerSeq \+= 1;/);
});

test('frontier board slots are comparison questions and expose their context', () => {
  const board = between(page, 'async function fetchBoard(rowsIn)', '/** @param {string} rung e.g.');
  const question = between(page, 'function comparisonQuestionKey(r)', '/** @param {any[]} rowsIn');
  assert.match(question, /supportRatioKey\(r\)/);
  assert.doesNotMatch(question, /supportBasisPoints\(r\)/);
  assert.doesNotMatch(question, /Math\.round|\/ r\.bars/);
  assert.match(board, /comparisonQuestionKey\(r\)/);
  assert.match(board, /\$\{comparisonQuestionKey\(r\)\}\\u001f\$\{r\.timeframe\}/);
  assert.doesNotMatch(board, /newest\.get\(r\.timeframe\)/);
  assert.match(page, /\{#each board10 as g \(g\.key\)\}/);
  assert.match(page, /\{g\.run\.feed\} · \{g\.run\.underlying\} · \{span\(g\.run\)\} · \{g\.rung\}/);
});

test('current-store prices are never presented as data-digest-bound run evidence', () => {
  assert.doesNotMatch(page, /the same files the sweep\s+read/i);
  assert.match(page, /Current-store reference; not bound to this run's data digest/);
  assert.match(page, /Strategy return · current-store normalized/);
  assert.match(page, /Buy and hold return · current store/);
  assert.match(page, /PnL \/ current-store open/);
});

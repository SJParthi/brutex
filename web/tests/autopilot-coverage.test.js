import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

const source = readFileSync(new URL('../src/routes/autopilot/+page.svelte', import.meta.url), 'utf8');
const ast = parse(source);
/** @param {string} name */
function declaration(name) {
  const node = ast.instance?.content.body.find((/** @type {any} */ value) =>
    value.type === 'FunctionDeclaration' ? value.id?.name === name
      : value.type === 'VariableDeclaration' && value.declarations.some((/** @type {any} */ item) => item.id?.name === name));
  assert.ok(node, `actual Autopilot ${name} declaration is required`);
  return node;
}
const names = ['MON', 'MKEY', 'DKEY', 'MAX_SPAN', 'cmp', 'MEMBERSHIP_UNKNOWN',
  'monthKey', 'monthLastDay', 'span', 'monthLabel', 'inr', 'inrs', 'istTime',
  'censusDay', 'foldTargetCensus', 'targetCoverageVerdict', 'classOf'];
const code = names.map((name) => { const node = declaration(name); return source.slice(node.start, node.end); }).join('\n');
const verdictNode = /** @type {any} */ (declaration('verdict'));
const arrow = verdictNode.declarations[0].init.arguments[0];
assert.equal(arrow.type, 'ArrowFunctionExpression');
const actualVerdict = source.slice(arrow.start, arrow.end);
const page = new Function(`${code}
  const feedName = value => value;
  function view(snapshot, target, state='waiting') {
    const ap={target,state}, link={kind:'ok'}, census=foldTargetCensus(snapshot,target,target.feed);
    const counted=census.kind==='ok', notCounted=census.why ?? 'Census not measured';
    const scope={same:true}, ladder=span(target.from,target.to);
    const rows=[...census.months].map(([key,value])=>({key,...value}));
    const verdict=(${actualVerdict})();
    return {census,rows,verdict};
  }
  return {view,foldTargetCensus,span,classOf};`)();

/** @param {string} day */
const micros = (day) => Date.parse(`${day}T09:15:00+05:30`) * 1000;
/** @param {string} instrument @param {string} month @param {number} rows
 * @param {{timeframe?:string,first?:number|null,last?:number|null}} [extra] */
const cell = (instrument, month, rows, extra = {}) => ({ instrument, month, timeframe: '1day', rows, first: null, last: null, ...extra });
/** @param {ReturnType<typeof cell>[]} readable */
const snapshot = (readable) => ({ state: 'ready', error: null, at: 1, feed: 'example', readable,
  // Deliberately hostile global totals: these must never enter target coverage.
  cells: 148222, bars: 900000000, byMonth: new Map() });
const target = { feed: 'example', timeframe: '1day', from: '2026-09-01', to: '2026-09-07', instruments: 906 };
/** @param {{claim:string,sub:string}} verdict */
function noCoverageAssurance(verdict) {
  assert.doesNotMatch(verdict.claim, /\bAll\b|\bComplete\b|\d%|of \d.*target/i);
  assert.match(verdict.sub, /no exact instrument list/);
  assert.match(verdict.sub, /completion percentage is unavailable/);
}

test('actual coverage filters the declared feed, timeframe and window before counting distinct records', () => {
  const rows = [
    cell('A', '2026-09', 2, { first: micros('2026-09-02'), last: micros('2026-09-05') }),
    cell('A', '2026-09', 10000, { timeframe: '1min' }),
    cell('B', '2026-08', 500),
    cell('B', '2026-09', 100, { first: micros('2026-09-08'), last: micros('2026-09-30') }),
    cell('C', '2026-10', 700)
  ];
  const result = page.view(snapshot(rows), target);
  assert.equal(result.census.kind, 'ok');
  assert.equal(result.census.cells, 1);
  assert.equal(result.census.bars, 2);
  assert.equal(result.census.months.size, 1);
  assert.equal(result.rows[0].cells, 1);
  assert.equal(result.rows[0].bars, 2);
  noCoverageAssurance(result.verdict);
  const otherFeed = page.foldTargetCensus({ ...snapshot(rows), feed: 'other' }, target, 'other');
  assert.equal(otherFeed.kind, 'unscoped');
  assert.match(otherFeed.why, /different feeds/);
});

test('the live-shaped 141-month span keeps all 59 empty months even when global counts exceed a reported denominator', () => {
  const currentTarget = { ...target, from: '2015-01-01', to: '2026-09-07' };
  const months = page.span(currentTarget.from, currentTarget.to).months;
  const entries = months.slice(59).flatMap((/** @type {string} */ month) => [
    cell('A', month, 5, month === '2026-09' ? { first: micros('2026-09-01'), last: micros('2026-09-07') } : {}),
    cell('A', month, 5000, { timeframe: '1min' })
  ]);
  const result = page.view(snapshot(entries), currentTarget, 'complete');
  assert.equal(result.rows.length, 141);
  assert.equal(result.rows.filter((/** @type {any} */ row) => row.cells === 0).length, 59);
  assert.equal(result.census.cells, 82);
  assert.equal(result.census.bars, 410);
  assert.match(result.verdict.claim, /59 months have no stored records/);
  assert.ok(result.verdict.facts.some((/** @type {any} */ fact) => fact.v === '82 · 59 · 0'));
  assert.equal(result.verdict.tone, 'warn');
  noCoverageAssurance(result.verdict);
});

test('an excess of different stored instruments in one month cannot fill an empty month or prove target membership', () => {
  const currentTarget = { ...target, from: '2026-08', to: '2026-09', instruments: 1 };
  const entries = Array.from({ length: 20 }, (_, n) => cell(`extra-${n}`, '2026-09', 2));
  const result = page.view(snapshot(entries), currentTarget, 'complete');
  assert.equal(result.census.cells, 20);
  assert.equal(result.rows[0].cells, 0);
  assert.equal(result.rows[1].cells, 20);
  assert.match(result.verdict.claim, /1 months have no stored records/);
  assert.equal(page.classOf(result.rows[1]), 'nod');
  noCoverageAssurance(result.verdict);
});

test('identical census records are counted once and conflicting duplicates withhold the actual headline totals', () => {
  const a = cell('A|B', '2026-09', 2, { first: micros('2026-09-01'), last: micros('2026-09-07') });
  const result = page.view(snapshot([a, { ...a }, { ...a, timeframe: '1min' }]), target);
  assert.equal(result.census.cells, 1);
  assert.equal(result.census.bars, 2);
  assert.equal(result.census.duplicates, 1);
  const conflict = page.view(snapshot([a, { ...a, rows: 3 }]), target);
  assert.equal(conflict.census.kind, 'broken');
  assert.match(conflict.census.why, /Conflicting census entries/);
  assert.doesNotMatch(conflict.verdict.claim, /\d.*instrument-months/);
});

test('partial months exclude outside bars and distinguish confirmed presence from unknown overlap', () => {
  const currentTarget = { ...target, from: '2026-09-03' };
  const result = page.view(snapshot([
    cell('inside', '2026-09', 2, { first: micros('2026-09-03'), last: micros('2026-09-07') }),
    cell('starts-before', '2026-09', 4, { first: micros('2026-09-01'), last: micros('2026-09-04') }),
    cell('ends-after', '2026-09', 7, { first: micros('2026-09-06'), last: micros('2026-09-30') }),
    cell('brackets', '2026-09', 30, { first: micros('2026-09-01'), last: micros('2026-09-30') }),
    cell('unstamped', '2026-09', 50),
    cell('before', '2026-09', 10, { first: micros('2026-09-01'), last: micros('2026-09-02') }),
    cell('after', '2026-09', 20, { first: micros('2026-09-08'), last: micros('2026-09-20') })
  ]), currentTarget);
  assert.equal(result.census.cells, 3);
  assert.equal(result.census.bars, 2);
  assert.equal(result.census.unknownCells, 2);
  assert.equal(result.census.unmeasuredBars, 4);
  assert.deepEqual(result.rows[0], { key: '2026-09', cells: 3, bars: 2, unknownCells: 2, unmeasuredBars: 4 });
  assert.match(result.verdict.sub, /4 boundary files are unmeasured/);
  const uncertainOnly = page.view(snapshot([cell('unstamped', '2026-09', 50)]), currentTarget);
  assert.equal(page.classOf(uncertainOnly.rows[0]), 'part');
  assert.ok(uncertainOnly.verdict.facts.some((/** @type {any} */ fact) => fact.v === '0 · 0 · 1'));
});

test('missing scope, malformed ranges, failed reads and unsafe counts cannot publish a measured target total', () => {
  const fullMonth = { ...target, from: '2026-09', to: '2026-09' };
  for (const badTarget of [{ ...target, feed: null }, { ...target, timeframe: null }, { ...target, to: '2026-02-31' }]) {
    assert.equal(page.view(snapshot([]), badTarget).census.kind, 'unscoped');
  }
  for (const state of ['reading', 'error']) {
    const result = page.view({ ...snapshot([]), state, error: 'named failure' }, target);
    assert.notEqual(result.census.kind, 'ok');
    assert.doesNotMatch(result.verdict.claim, /\d.*instrument-months/);
  }
  assert.equal(page.view(snapshot([cell('zero', '2026-09', 0)]), fullMonth).census.cells, 0);
  for (const bad of [cell('bad', '2026-09', -1), cell('bad', '2026-09', 1.5), cell('bad', '2026-09', Number.MAX_SAFE_INTEGER + 1)]) {
    assert.equal(page.view(snapshot([bad]), fullMonth).census.kind, 'broken');
  }
  const overflow = page.view(snapshot([cell('A', '2026-09', Number.MAX_SAFE_INTEGER), cell('B', '2026-09', 1)]), fullMonth);
  assert.equal(overflow.census.kind, 'broken');
  assert.match(overflow.census.why, /integer precision/);
  for (const bad of [
    cell('wrong-month', '2026-09', 2, { first: micros('2026-08-31'), last: micros('2026-09-07') }),
    cell('reversed', '2026-09', 2, { first: micros('2026-09-07') + 1000, last: micros('2026-09-07') })
  ]) {
    const result = page.view(snapshot([bad]), fullMonth);
    assert.equal(result.census.kind, 'broken');
    assert.match(result.census.why, /timestamps contradict/);
  }
});

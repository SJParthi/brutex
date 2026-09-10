import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

// Execute the actual page reader and coverage functions. No copied date
// arithmetic can make this pass while the live page still rejects its API.
const source = readFileSync(new URL('../src/routes/autopilot/+page.svelte', import.meta.url), 'utf8');
const ast = parse(source);
const names = ['MON', 'MKEY', 'DKEY', 'MAX_SPAN', 'cmp', 'STATES', 'MUST_EXPLAIN',
  'monthKey', 'monthLabel', 'span', 'str', 'num', 'readState'];
const code = names.map((name) => {
  const node = ast.instance?.content.body.find((/** @type {any} */ value) =>
    value.type === 'FunctionDeclaration' ? value.id?.name === name
      : value.type === 'VariableDeclaration' && value.declarations.some((/** @type {any} */ item) => item.id?.name === name));
  assert.ok(node, `the actual Autopilot ${name} declaration must exist`);
  return source.slice(node.start, node.end);
}).join('\n');
const page = new Function('inrs', `${code}\nreturn {monthKey,monthLabel,span,readState};`)(String);

test('the native civil-day target produces monthly coverage without changing its reported day bounds', () => {
  const raw = { state: 'waiting', why: 'Waiting for the next check', now: null, failures: [],
    target: { from: '2015-01-01', to: '2026-09-07', instruments: 2, timeframe: '1min', feed: 'example' } };
  const read = page.readState(raw);
  assert.equal(read.ok, true);
  assert.deepEqual(read.value.target, raw.target);
  const result = page.span(read.value.target.from, read.value.target.to);
  assert.equal(result.ok, true);
  assert.equal(result.months.length, 141);
  assert.equal(result.months[0], '2015-01');
  assert.equal(result.months.at(-1), '2026-09');
  assert.equal(page.monthLabel(read.value.target.from), 'Jan 2015');
  assert.equal(page.monthLabel(read.value.target.to), 'Sep 2026');
  assert.deepEqual(read.value.target, raw.target, 'drawing coverage must not widen the stored day target');
});

test('legacy month keys and mixed day/month endpoints still describe their inclusive months', () => {
  for (const [from, to] of [
    ['2023-12', '2024-02'], ['2023-12-31', '2024-02'], ['2023-12', '2024-02-01']
  ]) {
    assert.deepEqual(page.span(from, to), { ok: true, months: ['2023-12', '2024-01', '2024-02'] });
  }
  for (const [from, to] of [['2024-02-29', '2024-02'], ['2024-02', '2024-02-01']]) {
    assert.deepEqual(page.span(from, to), { ok: true, months: ['2024-02'] });
  }
  assert.equal(page.monthLabel('2024-02'), 'Feb 2024');
});

test('reported instrument counts must be exact non-negative integers, not rounded display estimates', () => {
  for (const instruments of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
    const result = page.readState({ state: 'waiting', now: null, failures: [],
      target: { from: '2026-09-01', to: '2026-09-07', instruments } });
    assert.equal(result.ok, false);
    assert.match(result.why, /target.instruments.*exact non-negative integer/);
  }
});

test('date projection rejects impossible calendar days and accepts Gregorian leap days exactly', () => {
  for (const valid of ['2000-02-29', '2024-02-29', '2400-02-29', '2026-01-31', '2026-04-30', '2026-12-31']) {
    assert.equal(page.monthKey(valid), valid.slice(0, 7), valid);
  }
  for (const invalid of ['1900-02-29', '2023-02-29', '2100-02-29', '2024-02-30', '2026-02-31',
    '2026-04-31', '2026-06-31', '2026-09-31', '2026-11-31', '0000-01-01']) {
    assert.equal(page.monthKey(invalid), null, invalid);
    assert.equal(page.monthLabel(invalid), null, invalid);
  }
});

test('malformed or non-exact dates refuse the offending endpoint and preserve its raw value', () => {
  for (const raw of [null, '', '2026', '2026-00', '2026-13', '2026-9', '2026-09-7', '2026-09-00',
    '2026-09-32', '2026-13-01', '2026-09-07T00:00:00Z', '2026-09-07Z', '2026-09-07 ', ' 2026-09-07',
    '2026-09\n', '2026-09-07\n', '2026-09-07\r\n']) {
    for (const [from, to, field] of [[raw, '2026-09', 'target.from'], ['2026-09', raw, 'target.to']]) {
      const result = page.span(from, to);
      assert.equal(result.ok, false, `${String(raw)} at ${field}`);
      assert.equal(result.field, field);
      assert.equal(result.raw, raw);
      assert.match(result.why, /valid calendar date.*month key/);
      assert.equal(result.months, undefined);
    }
  }
  for (const raw of [42, {}, [], new Date('2026-09-07')]) assert.equal(page.monthKey(raw), null);
});

test('month projection cannot hide reversed exact days and the coverage row limit remains bounded', () => {
  for (const [from, to] of [
    ['2026-09-07', '2026-09-06'], ['2026-09-01', '2026-08-31'], ['2026-09', '2026-08']
  ]) {
    const result = page.span(from, to);
    assert.equal(result.ok, false);
    assert.match(result.why, /ends before it begins/);
    assert.equal(result.months, undefined);
  }
  assert.deepEqual(page.span('2026-09-07', '2026-09-07'), { ok: true, months: ['2026-09'] });
  assert.equal(page.span('2000-01-01', '2049-12-31').months.length, 600);
  const tooWide = page.span('2000-01-01', '2050-01-01');
  assert.equal(tooWide.ok, false);
  assert.match(tooWide.why, /601 month files/);
  assert.equal(tooWide.months, undefined);
});

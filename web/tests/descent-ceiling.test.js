import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

// D-4740. D-1732 made both api doors read a zero stop ceiling as no ceiling,
// as `cli` does, and left the browser refusing it: `wholeNumber` (n > 0) read
// the ceiling box, so the page could not ask for a run the route accepts, and
// its comment still quoted the refusal D-1732 removed. The ceiling now has its
// own reading, which accepts 0, while the row count keeps refusing it.

const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');

/** The source of `function name(...) { ... }` in the page, by brace depth. */
const functionSource = (name) => {
  const start = page.indexOf(`function ${name}(`);
  assert.notEqual(start, -1, `the page defines ${name}`);
  let depth = 0;
  for (let at = page.indexOf('{', start); at < page.length; at += 1) {
    if (page[at] === '{') depth += 1;
    if (page[at] === '}') {
      depth -= 1;
      if (depth === 0) return page.slice(start, at + 1);
    }
  }
  assert.fail(`${name} never closes`);
};

/** The page's own function, evaluated as written. */
const load = (name) => new Function(`${functionSource(name)}; return ${name};`)();

/** The slice of `source` from `from` up to the next `to`. */
const between = (source, from, to) => {
  const start = source.indexOf(from);
  assert.notEqual(start, -1, `missing start marker: ${from}`);
  const end = source.indexOf(to, start);
  assert.notEqual(end, -1, `missing end marker: ${to}`);
  return source.slice(start, end);
};

test('a stop ceiling of 0 is read as no ceiling, and nonsense is still refused', () => {
  const stopCeiling = load('stopCeiling');
  assert.equal(stopCeiling('0'), 0);
  assert.equal(stopCeiling(' 0 '), 0);
  assert.equal(stopCeiling('120'), 120);
  for (const wrong of ['', ' ', '-1', '-0', '1.5', '1e3', 'abc', '12abc', '9007199254740993']) {
    assert.equal(stopCeiling(wrong), null, `${JSON.stringify(wrong)} is not a ceiling`);
  }
  assert.equal(stopCeiling(undefined), null);
});

test('the row count still refuses 0', () => {
  const wholeNumber = load('wholeNumber');
  assert.equal(wholeNumber('0'), null, 'a listing of no rows is no answer');
  assert.equal(wholeNumber('25'), 25);
  assert.equal(wholeNumber(''), null);
  assert.equal(wholeNumber('-3'), null);
});

test('the descent reads the ceiling with stopCeiling and the rows with wholeNumber', () => {
  const start = functionSource('startDescent');
  assert.match(start, /const ceiling = stopCeiling\(descent\.points\);/);
  assert.match(start, /const listRows = wholeNumber\(descent\.top\);/);
  assert.match(start, /max_points: ceiling,/);
  assert.doesNotMatch(start, /wholeNumber\(descent\.points\)/);
  assert.match(page, /const descentPoints = \$derived\(stopCeiling\(descent\.points\)\);/);
  assert.match(page, /const descentRows = \$derived\(wholeNumber\(descent\.top\)\);/);
  assert.doesNotMatch(page, /wholeNumber\(descent\.points\)/);
});

test('no text on the page repeats the refusal D-1732 removed', () => {
  assert.doesNotMatch(page, /a ceiling of zero\s+admits no trade/);
  assert.doesNotMatch(page, /INDEX POINTS above zero/);
  const doc = between(page, 'A whole number above zero', 'function wholeNumber(');
  assert.match(doc, /D-1732/);
  const summary = between(page, "{:else if descentStop === 'points'}", '{/if}');
  assert.match(summary, /0 means no ceiling/);
  assert.match(summary, /descentPoints === 0[\s\S]*?no stop ceiling/);
});

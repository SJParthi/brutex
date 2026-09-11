import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const page = readFileSync(
  new URL('../src/routes/backtest/+page.svelte', import.meta.url),
  'utf8'
);

test('every backtest input has a stable submitted-control name', () => {
  const inputs = page.match(/<input\b[\s\S]*?\/>/g) ?? [];
  assert.equal(inputs.length, 6, 'update this audit when another input shape is added');
  for (const input of inputs) {
    assert.match(input, /\bname\s*=/, `unnamed input:\n${input}`);
  }

  const names = inputs.map((input) => input.match(/\bname\s*=\s*([^\s>]+)/)?.[1]);
  assert.equal(new Set(names).size, names.length, 'every static or loop-derived name expression is unique');
  assert.match(page, /name=\{`engine-\$\{k\.key\}`\}/, 'every engine knob derives its own name');
  assert.match(page, /name=\{`frontier-weight-\$\{key\}`\}/, 'every ranking weight derives its own name');
});

test('the drill-down heading tree advances one level at a time', () => {
  assert.match(page, /<h2 class="bh">\{openRun\.underlying\}[\s\S]*?<h3 class="tt-h">Key stats<\/h3>/);
  assert.doesNotMatch(page, /<h4 class="tt-h">/, 'tester sections are direct children of the h2 drill-down');
  assert.doesNotMatch(page, /<h5 class="tt-h5">/, 'tester subsections are one level below their h3');
});

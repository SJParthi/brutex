import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { cashIdentityFor, DEFAULT_ZERODHA_CASH_IDENTITY } from '../src/lib/cash-identity.js';
import { codeOf, templateExpressions } from './page-code-fixture.js';

test('Zerodha form defaults to separate token and corroborated ISIN evidence, never token-only', () => {
  assert.equal(DEFAULT_ZERODHA_CASH_IDENTITY, 'zerodha_cross_checked');
  assert.equal(cashIdentityFor('zerodha', DEFAULT_ZERODHA_CASH_IDENTITY), 'zerodha_cross_checked');
  assert.equal(cashIdentityFor('dhan', DEFAULT_ZERODHA_CASH_IDENTITY), 'isin');
});

test('ingest form wires the cross-check default and never renders an unknown denominator as zero', () => {
  // CODE, NOT TEXT (P19-05, D-2564): comments are blanked before matching, so
  // an old expression left in `<!-- … -->` no longer satisfies these.
  const source = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  const page = codeOf(source);
  assert.ok(page.includes('$state(DEFAULT_ZERODHA_CASH_IDENTITY)'));
  assert.ok(page.includes("month.exp === null && month.k !== 'beyond'"));
  assert.ok(!source.includes('Vendor-supplied ISIN (strict default)'));

  // AND THE DENOMINATOR CELL IS EVALUATED. The finding's mutation rendered
  // `/ {n(r.exp)}` and kept the guarded expression in an HTML comment; that
  // comment is not markup, so it is not found here and the test fails.
  const cells = templateExpressions(source).filter(
    (expression) => expression.includes("'unverified'") && expression.includes('n(r.exp)')
  );
  assert.equal(cells.length, 1, 'exactly one rendered denominator cell guards the expected total');
  const cell = new Function('c', 'r', 'n', `return (${cells[0]});`);
  const n = (/** @type {number} */ value) => `#${value}`;
  for (const c of [null, undefined, {}, { expectedKnown: false }, { expectedKnown: 0 }]) {
    assert.equal(cell(c, { exp: 0 }, n), 'unverified', `${JSON.stringify(c)}: an unknown total is never a zero`);
  }
  assert.equal(cell({ expectedKnown: true }, { exp: 0 }, n), '#0', 'a known zero total is a number');
  assert.equal(cell({ expectedKnown: true }, { exp: 1129875 }, n), '#1129875');
});

test('cash identity is an explicit Zerodha-only choice with separate assurance levels', () => {
  assert.equal(cashIdentityFor('zerodha', 'zerodha_symbol'), 'zerodha_symbol');
  assert.equal(cashIdentityFor('zerodha', 'zerodha_cross_checked'), 'zerodha_cross_checked');
  for (const value of [true, false, undefined, null, '', 'true', 1, 'auto', 'isin']) {
    assert.equal(cashIdentityFor('zerodha', value), 'isin');
  }
  for (const feed of ['dhan', 'groww', 'truedata', 'gdfl', '', undefined, null]) {
    assert.equal(cashIdentityFor(feed, 'zerodha_cross_checked'), 'isin');
    assert.equal(cashIdentityFor(feed, 'zerodha_symbol'), 'isin');
  }
});

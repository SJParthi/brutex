import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { cashIdentityFor, DEFAULT_ZERODHA_CASH_IDENTITY } from '../src/lib/cash-identity.js';

test('Zerodha form defaults to separate token and corroborated ISIN evidence, never token-only', () => {
  assert.equal(DEFAULT_ZERODHA_CASH_IDENTITY, 'zerodha_cross_checked');
  assert.equal(cashIdentityFor('zerodha', DEFAULT_ZERODHA_CASH_IDENTITY), 'zerodha_cross_checked');
  assert.equal(cashIdentityFor('dhan', DEFAULT_ZERODHA_CASH_IDENTITY), 'isin');
});

test('ingest form wires the cross-check default and never renders an unknown denominator as zero', () => {
  const page = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.ok(page.includes('$state(DEFAULT_ZERODHA_CASH_IDENTITY)'));
  assert.ok(page.includes("month.exp === null && month.k !== 'beyond'"));
  assert.ok(page.includes("? 'unverified' : n(r.exp)"));
  assert.ok(!page.includes('Vendor-supplied ISIN (strict default)'));
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

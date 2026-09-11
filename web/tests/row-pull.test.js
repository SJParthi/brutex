import test from 'node:test';
import assert from 'node:assert/strict';
import { singleMemberBody } from '../src/lib/row-pull.js';

test('row retry replaces every basket member while preserving request bounds and verification', () => {
  const base = 'target=fno&vendor=zerodha&cash_identity=zerodha_cross_checked&from=2019-12-01&to=2026-09-04&granularity=1min&member=ABB&member=INFY';
  for (const symbol of ['ABB', 'M&M', 'BAJAJ-AUTO', 'GVT&D']) {
    const result = new URLSearchParams(singleMemberBody(base, symbol));
    assert.deepEqual(result.getAll('member'), [symbol]);
    for (const [key, value] of new URLSearchParams(base)) {
      if (key !== 'member') assert.equal(result.get(key), value);
    }
  }
  assert.deepEqual(new URLSearchParams(singleMemberBody('target=fno', 'ABB')).getAll('member'), ['ABB']);
  assert.throws(() => singleMemberBody(base, ''), /requires a symbol/);
  assert.throws(() => singleMemberBody(base, '  '), /requires a symbol/);
});

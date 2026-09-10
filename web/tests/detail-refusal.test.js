import test from 'node:test';
import assert from 'node:assert/strict';
import { detailRefusal } from '../src/lib/detail-refusal.js';
import { fetchExpressionSearch } from '../src/lib/expression-search.js';
import { fetchCandidatePage } from '../src/lib/candidate-trades.js';

const identity = 'ab'.repeat(32);
const refusal = 'Search page byte admission exhausted; no prefix exposed';
/** @param {any} body */
const response = (body) => ({ ok: false, status: 503, json: async () => body });
const body = () => ({ schema_version: 1, status: 'refused', rows: [], refusal });

test('search and candidate failures retain exact bounded admission and busy reasons', async () => {
  await assert.rejects(fetchExpressionSearch({ identity }, async () => response(body())), /HTTP 503.*byte admission exhausted/);
  await assert.rejects(fetchCandidatePage({ identity, attempt: '1' }, async () => response({ ...body(), refusal: 'Candidate detail is busy; retry this exact saved page' })), /HTTP 503.*detail is busy/);
});

test('refusal detail never accepts partial rows, unknown schema or unbounded messages', async () => {
  for (const changed of [
    { ...body(), rows: [{ identity }] },
    { ...body(), status: 'saved' },
    { ...body(), schema_version: 2 },
    { ...body(), refusal: '' },
    { ...body(), refusal: ' '.repeat(10) },
    { ...body(), refusal: 'x'.repeat(4097) },
    { ...body(), rows: null },
    [], null,
  ]) {
    assert.equal(await detailRefusal(response(changed), 'unavailable'), 'unavailable');
  }
  assert.equal(await detailRefusal(response({ ...body(), refusal: 'x'.repeat(4096) }), 'unavailable'), 'unavailable ' + 'x'.repeat(4096));
});

test('proxy HTML, malformed JSON and absent response preserve the fallback', async () => {
  assert.equal(await detailRefusal({ json: async () => { throw new SyntaxError('HTML'); } }, 'HTTP 502'), 'HTTP 502');
  assert.equal(await detailRefusal(undefined, 'connection unavailable'), 'connection unavailable');
  assert.equal(await detailRefusal({ status: 502 }, 'HTTP 502'), 'HTTP 502');
  await assert.rejects(fetchExpressionSearch({ identity }, async () => ({ ok: false, status: 502, json: async () => { throw new SyntaxError('HTML'); } })), /HTTP 502/);
});

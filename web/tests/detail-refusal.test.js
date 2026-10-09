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

// F1 (OBSV-19, D-3218): every route in `crates/api/src/operation_audit.rs`
// `AUDITED` can be refused by the audit layer itself, before or after its
// handler, with `{"schema_version":1,"refusal":…,"code":"invocation_audit_
// unavailable","handler_completed":bool,"why":…}` (429 when the journal is busy,
// 503 otherwise) or the read twin `invocation_audit_read_unavailable`. That body
// is not the detail envelope, so every detail reader of an audited route
// (/candidate-trades.json, /sweep-evidence.json, /boolean-*.json,
// /selection-v6.json, /expression-search.json) printed the status alone and
// dropped whether the handler had run.
const audit = (/** @type {boolean} */ completed) => ({ schema_version: 1,
  refusal: 'bounded request audit capacity is full; retry this exact request',
  code: 'invocation_audit_unavailable', handler_completed: completed,
  why: completed ? 'The handler already ran. Its work may still be running or saved; inspect the exact invocation before retrying a write.'
    : 'The handler was not dispatched because its required audit start was unavailable.' });

test('an audit-layer refusal of an audited detail route keeps its refusal and whether the handler ran (F1)', async () => {
  const busy = { ...response(audit(false)), status: 429 };
  assert.equal(await detailRefusal(busy, 'HTTP 429.'),
    'HTTP 429. bounded request audit capacity is full; retry this exact request The handler was not dispatched because its required audit start was unavailable.');
  await assert.rejects(fetchCandidatePage({ identity, attempt: '1' }, async () => busy), /HTTP 429.*audit capacity is full.*not dispatched/);
  await assert.rejects(fetchExpressionSearch({ identity }, async () => response(audit(true))), /HTTP 503.*audit capacity is full.*handler already ran/);
  const read = { schema_version: 1, code: 'invocation_audit_read_unavailable', refusal: 'audit read unavailable: Saturated',
    why: 'No audit snapshot was published. Reading this endpoint does not start an engine task or append an audit record.' };
  assert.equal(await detailRefusal(response(read), 'x'), `x audit read unavailable: Saturated ${read.why}`);
  for (const damage of [{ schema_version: 2 }, { code: 'other' }, { handler_completed: 'false' }, { refusal: ' ' },
    { refusal: 'x'.repeat(4097) }, { why: 7 }, { why: 'x'.repeat(4097) }]) {
    assert.equal(await detailRefusal(response({ ...audit(false), ...damage }), 'fallback'), 'fallback', JSON.stringify(damage));
  }
  assert.equal(await detailRefusal(response({ ...read, handler_completed: false }), 'fallback'), 'fallback', 'a read failure never claims a dispatch');
});

// The readers of the one status rule (D-7550): every identity route answers an
// identity that names nothing with ONE not-found, HTTP 404 and one body shape,
// and a page shows "nothing is saved" only for exactly that answer. No body
// here is market data: identities are repeated bytes, sentences are invented.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { detailRefusal, notFoundWhy, REFUSAL_STATUSES } from '../src/lib/detail-refusal.js';
import { fetchExpressionSearch } from '../src/lib/expression-search.js';
import { fetchCandidatePage } from '../src/lib/candidate-trades.js';
import { fetchSweepEvidence } from '../src/lib/sweep-evidence.js';
import { fetchCompleteFrontier } from '../src/lib/frontier-pages.js';

const identity = 'cd'.repeat(32);
const other = 'ef'.repeat(32);
const why = 'Thing not held: one probe of x/' + identity + ' found nothing (NotFound)';

/** The one not-found body the server writes. @param {any} [changes] */
const body = (changes = {}) => ({
  schema_version: 1, status: 'not_found', identity, refusal: why, rows: [],
  path: 'x/' + identity, error_kind: 'NotFound', rule: 'D-7550', ...changes
});
/** A response of `status` whose JSON is `value`. @param {number} status @param {any} value */
const answer = (status, value) => ({ ok: status >= 200 && status < 300, status, json: async () => value });

test('the not-found sentence is read only from the exact not-found answer', () => {
  assert.equal(notFoundWhy(answer(404, body()), body(), identity), why);
  // EXTREMES, each refused: another run's identity, a 200 carrying the words,
  // rows beside the refusal, an empty, a blank and an over-long sentence, an
  // unknown schema, a body that is not an object at all.
  for (const [response, value] of [
    [answer(404, null), body({ identity: other })],
    [answer(200, null), body()],
    [answer(503, null), body()],
    [answer(404, null), body({ rows: [{ rank: 1 }] })],
    [answer(404, null), body({ rows: null })],
    [answer(404, null), body({ refusal: '' })],
    [answer(404, null), body({ refusal: '   ' })],
    [answer(404, null), body({ refusal: 'x'.repeat(4097) })],
    [answer(404, null), body({ schema_version: 2 })],
    [answer(404, null), body({ status: 'missing' })],
    [answer(404, null), []],
    [answer(404, null), null],
    [undefined, body()],
  ]) {
    assert.equal(notFoundWhy(response, value, identity), null, JSON.stringify(value)?.slice(0, 80));
  }
  assert.equal(notFoundWhy(answer(404, null), body({ refusal: 'y'.repeat(4096) }), identity), 'y'.repeat(4096));
});

test('refusal detail quotes the rule classes and a route refusal, never a result word', async () => {
  // GENERATED, NEVER TYPED: the class words are read from the classifier's own
  // `Class::word`, so a class the server gains and this reader lacks fails here.
  const rule = readFileSync(new URL('../../crates/api/src/status_rule.rs', import.meta.url), 'utf8');
  const arms = rule.slice(rule.indexOf('pub const fn word'), rule.indexOf('// ══ Parameters'));
  const words = [...arms.matchAll(/Self::[A-Za-z]+ => "([a-z_]+)",/g)].map((arm) => arm[1]);
  assert.equal(words.length, 7, 'the seven classes of D-7550');
  assert.deepEqual(REFUSAL_STATUSES, ['refused', ...words]);
  for (const status of REFUSAL_STATUSES) {
    assert.equal(await detailRefusal(answer(503, body({ status })), 'HTTP 503.'), 'HTTP 503. ' + why);
  }
  for (const status of ['saved', 'observed', 'missing', 'complete', 'NOT_FOUND', 'not found', '']) {
    assert.equal(await detailRefusal(answer(503, body({ status })), 'HTTP 503.'), 'HTTP 503.');
  }
});

test('an unknown search is shown as missing, and a pinned page is never replaced by it', async () => {
  const missing = await fetchExpressionSearch({ identity }, async () => answer(404, body()));
  assert.deepEqual(missing, { schema_version: 1, status: 'missing', identity, why, rows: [], refusal: null });
  // COMBINATION: a pinned snapshot and the not-found at once is the pin no
  // longer held, refused by name, never a missing search.
  const snapshot = { sequence: '9', seal: 'ab'.repeat(32) };
  await assert.rejects(fetchExpressionSearch({ identity, snapshot }, async () => answer(404, body())), /cannot replace a pinned page\. Thing not held/);
  // Another identity's not-found, or a 404 with no application body (a proxy's
  // HTML), names the HTTP status and shows nothing missing.
  await assert.rejects(fetchExpressionSearch({ identity }, async () => answer(404, body({ identity: other }))), /HTTP 404/);
  await assert.rejects(fetchExpressionSearch({ identity }, async () => ({ ok: false, status: 404, json: async () => { throw new SyntaxError('HTML'); } })), /HTTP 404/);
  // OUT OF THE BOX: the old 200 "missing" is no answer the server gives now;
  // a reader that still took it would hide a server that regressed to it.
  await assert.rejects(fetchExpressionSearch({ identity }, async () => answer(200, {
    schema_version: 1, status: 'missing', identity, why: 'old', rows: [], refusal: null
  })), /invalid/);
});

test('an attempt with no saved capture is missing, and a pinned catalog is never replaced by it', async () => {
  const missing = await fetchCandidatePage({ identity, attempt: '7' }, async () => answer(404, body()));
  assert.deepEqual(missing, { schema_version: 1, status: 'missing', model: 'and-mask', identity, attempt: '7', rows: [], refusal: null, why });
  await assert.rejects(fetchCandidatePage({ identity, attempt: '7', digest: other, offset: '256' }, async () => answer(404, body())), /cannot replace a pinned page\. Thing not held/);
  await assert.rejects(fetchCandidatePage({ identity, attempt: '7' }, async () => answer(404, body({ rows: [{}] }))), /HTTP 404/);
});

test('a run with no saved evidence is missing, and a later page losing its attempt is refused', async () => {
  const missing = await fetchSweepEvidence(identity, async () => answer(404, body()));
  assert.deepEqual(missing, { status: 'missing', evidence: null, rows: [], why });
  await assert.rejects(fetchSweepEvidence(identity, async () => answer(404, body({ identity: other }))), /HTTP 404/);
  await assert.rejects(fetchSweepEvidence(identity, async () => answer(404, body({ refusal: 'x'.repeat(5000) }))), /HTTP 404/);
});

test('a run with no committed receipt is the explicit absence, on page zero only', async () => {
  const absent = await fetchCompleteFrontier(identity, async () => answer(404, body()));
  assert.equal(absent.identity, identity);
  assert.equal(absent.total_count, 0);
  assert.deepEqual(absent.rows, []);
  assert.equal(absent.refusal, why);
  assert.equal(absent.complete, true);
  await assert.rejects(fetchCompleteFrontier(identity, async () => answer(404, body({ identity: other }))), /answered 404 on page 0/);
  await assert.rejects(fetchCompleteFrontier(identity, async () => answer(404, null)), /answered 404 on page 0/);
});

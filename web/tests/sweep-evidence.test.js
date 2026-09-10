import test from 'node:test';
import assert from 'node:assert/strict';
import { fetchSweepEvidence, evidenceComparison, canInspectCandidateTrades } from '../src/lib/sweep-evidence.js';

const identity = 'ab'.repeat(32);
const summary = (/** @type {number} */ count) => ({
  attempt: '9007199254740993', operation: 'audit', completion: 'completed',
  started_micros: '1788680000000000', updated_micros: '1788680000000001',
  depth_rows: String(count), ranked_rows: '0', ranked_available: true, validation_requested: false
});
const level = (/** @type {number} */ k) => ({
  k: String(k), generated: '1', duplicates: '0', excluded: '0', pruned: '0',
  infrequent: '0', frequent: '1', admitted: String(k), pairs: String(k), reconciles: true
});
function pageOf(/** @type {number} */ count, page = 0) {
  const start = page * 256;
  return {
    schema_version: 1, identity, status: 'saved', evidence: summary(count),
    kind: 'depth', page, limit: 256, total_count: String(count),
    next_page: start + 256 < count ? page + 1 : null, page_complete: true,
    rows: Array.from({ length: Math.min(256, count - start) }, (_, at) => level(start + at + 1)),
    refusal: null
  };
}
const response = (/** @type {any} */ body) => ({ ok: true, status: 200, json: async () => body });

test('later-period lifecycle never proves campaign completion, trades or promotion',async()=>{
  const body=pageOf(0);body.evidence.operation='boolean-oos';
  const result=await fetchSweepEvidence(identity,async()=>response(body));
  const rows=evidenceComparison(result);
  assert.equal(rows[0].status,'Later-period comparison only');
  assert.equal(rows.find(row=>row.area==='Historical financial rules')?.status,'Not established in this summary');
  assert.equal(rows.find(row=>row.area==='Exact trade replay')?.status,'Not verified in this summary');
  assert.match(rows.find(row=>row.area==='Exact trade replay')?.detail??'',/does not authenticate a trade page/);
  assert.equal(rows.find(row=>row.area==='Institutional admission')?.status,'Authority not exposed here');
});

test('loads all saved depths and pins subsequent pages to exact u64 attempt', async () => {
  const calls = /** @type {string[]} */ ([]);
  const result = await fetchSweepEvidence(identity, async (url) => {
    calls.push(url);
    return response(pageOf(257, Number(new URL(url, 'http://fixture').searchParams.get('page'))));
  });
  assert.equal(result.rows.length, 257);
  assert.equal(result.rows.at(-1).k, '257');
  assert.equal(calls.length, 2);
  assert.ok(calls[1].includes('attempt=9007199254740993'));
  assert.equal(result.evidence.attempt, '9007199254740993');
  assert.equal(evidenceComparison(result).find((r) => r.area === 'Walk-forward and overfitting checks')?.status, 'Explicitly disabled');
});

test('keeps huge counters exact and distinguishes measured mismatch from forged balance', async () => {
  const body = pageOf(1);
  body.rows[0].generated = '18446744073709551615';
  body.rows[0].frequent = '0';
  body.rows[0].infrequent = '18446744073709551615';
  const exact = await fetchSweepEvidence(identity, async () => response(body));
  assert.equal(exact.rows[0].generated, '18446744073709551615');
  body.rows[0].reconciles = false;
  await assert.rejects(fetchSweepEvidence(identity, async () => response(body)), /contradicts/);
  body.rows[0].generated = '18446744073709551614';
  const mismatch = await fetchSweepEvidence(identity, async () => response(body));
  assert.equal(evidenceComparison(mismatch)[1].status, 'Mismatch');
});

test('missing evidence, empty published rankings and absent rankings remain distinct', async () => {
  const missing = await fetchSweepEvidence(identity, async () => response({
    schema_version: 1, identity, status: 'missing', evidence: null, rows: [], refusal: null, why: 'Older run has no saved depths.'
  }));
  assert.equal(missing.status, 'missing');
  const empty = await fetchSweepEvidence(identity, async () => response(pageOf(0)));
  assert.equal(empty.evidence.ranked_available, true);
  const unpublished = pageOf(0);
  unpublished.evidence.ranked_available = false;
  const result = await fetchSweepEvidence(identity, async () => response(unpublished));
  assert.equal(result.evidence.ranked_available, false);
  assert.equal(evidenceComparison(result)[1].status, 'Not recorded');
});

test('refuses partial, changing, looping and incorrectly bound evidence', async () => {
  for (const mutate of [
    (/** @type {any} */ body) => { body.identity = 'cd'.repeat(32); },
    (/** @type {any} */ body) => { body.next_page = null; },
    (/** @type {any} */ body) => { body.next_page = 0; },
    (/** @type {any} */ body) => { body.page_complete = false; },
    (/** @type {any} */ body) => { body.rows.pop(); },
    (/** @type {any} */ body) => { body.rows[0].k = '2'; },
    (/** @type {any} */ body) => { body.rows[0].generated = 1; },
    (/** @type {any} */ body) => { body.evidence.depth_rows = '4097'; },
    (/** @type {any} */ body) => { body.evidence.validation_requested = 'false'; },
    (/** @type {any} */ body) => { body.refusal = 'file corrupt'; }
  ]) {
    const body = pageOf(257);
    mutate(body);
    await assert.rejects(fetchSweepEvidence(identity, async () => response(body)));
  }
  await assert.rejects(fetchSweepEvidence(identity, async (url) => {
    const page = Number(new URL(url, 'http://fixture').searchParams.get('page'));
    const body = pageOf(257, page);
    if (page) body.evidence.attempt = '9007199254740994';
    return response(body);
  }), /changed/);
  await assert.rejects(fetchSweepEvidence(identity, async () => ({ ok: false, status: 503 })), /HTTP 503/);
  await assert.rejects(fetchSweepEvidence(identity, async () => { throw new Error('offline'); }), /offline/);
  await assert.rejects(fetchSweepEvidence(identity, async () => ({ ok: true, json: async () => { throw new Error('invalid JSON'); } })), /invalid JSON/);
});

test('requested validation and Boolean mode never become institutional admission', () => {
  const body = { status: 'saved', evidence: { ...summary(0), validation_requested: true, operation: 'expression' }, rows: [] };
  const rows = evidenceComparison(body);
  assert.equal(rows[0].status, 'Different candidate model');
  assert.match(rows.find((row) => row.area === 'Walk-forward and overfitting checks')?.status ?? '', /outcome not stored/);
  assert.equal(rows.find((row) => row.area === 'Institutional admission')?.status, 'Authority not exposed here');
  assert.match(rows.find((row) => row.area === 'Exact trade replay')?.detail ?? '', /each actually priced candidate/);
  assert.match(rows.find((row) => row.area === 'Exact trade replay')?.detail ?? '', /unpriced signal rows remain explicit; no winner trace is substituted/);
  assert.equal(rows.find((row) => row.area === 'Exact trade replay')?.status, 'Not verified in this summary');
  assert.equal(evidenceComparison(null)[0].status, 'Not established');
  body.evidence.operation = 'auto-search';
  assert.equal(evidenceComparison(body)[0].status, 'Orchestration only');
  body.evidence.operation = 'preparation';
  assert.equal(evidenceComparison(body)[0].status, 'Preparation only');
  body.evidence.operation = 'expression-search';
  assert.equal(evidenceComparison(body)[0].status, 'Different candidate model');
});

test('a completed checksum audit never becomes a completed sweep or financial result', async () => {
  const page = { ...pageOf(0), evidence: {
    ...summary(0), operation: 'checksum-audit', ranked_available: false, validation_requested: null
  } };
  const result = await fetchSweepEvidence(identity, async () => response(page));
  const rows = evidenceComparison(result);
  assert.equal(result.evidence.operation, 'checksum-audit');
  assert.equal(rows[0].status, 'Input integrity only');
  assert.equal(rows.find((row) => row.area === 'Historical financial rules')?.status, 'Not a pricing operation');
  assert.equal(rows.find((row) => row.area === 'Exact trade replay')?.status, 'No trades in checksum audit');
  assert.equal(rows.find((row) => row.area === 'Institutional admission')?.status, 'Authority not exposed here');
});

test('complete Boolean catalog evidence never becomes exhaustive search or institutional admission', async () => {
  const page = { ...pageOf(0), evidence: {
    ...summary(0), operation: 'boolean-candidates', ranked_available: false, validation_requested: null
  } };
  const result = await fetchSweepEvidence(identity, async () => response(page));
  const rows = evidenceComparison(result);
  assert.equal(rows[0].status, 'Explicit program catalog only');
  assert.equal(rows.find(row => row.area === 'Historical financial rules')?.status, 'Not established in this summary');
  assert.match(rows.find(row => row.area === 'Historical financial rules')?.detail ?? '', /does not certify exhaustive Boolean grammar search/);
  assert.equal(rows.find(row => row.area === 'Institutional admission')?.status, 'Authority not exposed here');
  assert.equal(canInspectCandidateTrades(result), false);
  for (const [operation, label] of [['boolean-statistics', 'Recorded statistics only'], ['boolean-admission', 'Recorded research admission only']]) {
    page.evidence.operation = operation;
    const next = await fetchSweepEvidence(identity, async () => response(page));
    assert.equal(evidenceComparison(next)[0].status, label);
    assert.equal(evidenceComparison(next).find(row => row.area === 'Institutional admission')?.status, 'Authority not exposed here');
    assert.equal(canInspectCandidateTrades(next), false);
  }
});

test('missing and failed summary reads cannot certify trade availability or offer an absent detail view', async () => {
  const missing = await fetchSweepEvidence(identity, async () => response({
    schema_version: 1, identity, status: 'missing', evidence: null, rows: [], refusal: null, why: 'Older run has no saved attempt.'
  }));
  assert.equal(canInspectCandidateTrades(missing), false);
  assert.equal(evidenceComparison(missing).find((row) => row.area === 'Exact trade replay')?.status, 'Not recorded');
  await assert.rejects(fetchSweepEvidence(identity, async () => ({ ok: false, status: 503 })), /HTTP 503/);
  // SweepEvidence discards its result on failure instead of retaining an older one.
  assert.equal(canInspectCandidateTrades(null), false);
  const failed = evidenceComparison(null).find((row) => row.area === 'Exact trade replay');
  assert.equal(failed?.status, 'Not established');
  assert.match(failed?.detail ?? '', /failed summary reads cannot establish trades/);
});

test('completed operations without a candidate reader cannot advertise an exact trace above', () => {
  for (const operation of ['sweep', 'auto-search', 'auto-probe', 'preparation', 'expression-search', 'global-replay', 'global-replay-stream']) {
    const result = { status: 'saved', evidence: { ...summary(0), operation }, rows: [] };
    assert.equal(canInspectCandidateTrades(result), false, operation);
    const trade = evidenceComparison(result).find((row) => row.area === 'Exact trade replay');
    assert.equal(trade?.status, 'No candidate capture in this view', operation);
    assert.match(trade?.detail ?? '', /does not establish a captured candidate trace here/);
    assert.doesNotMatch(trade?.detail ?? '', /above/);
  }
});

test('Boolean catalog navigation requires separate authenticated detail for every lifecycle state', () => {
  for (const completion of ['running', 'completed', 'halted', 'refused']) {
    const result = {status:'saved', evidence:{...summary(0),operation:'boolean-candidates',completion},rows:[]};
    assert.equal(canInspectCandidateTrades(result),false);
    const trade=evidenceComparison(result).find(row=>row.area==='Exact trade replay');
    assert.equal(trade?.status,'Not verified in this summary');
    assert.match(trade?.detail??'',/authenticate its own completion/);
  }
});

test('audit and expression summaries offer a reader without claiming a capture was verified', () => {
  for (const operation of ['audit', 'expression']) {
    for (const completion of ['running', 'completed', 'halted', 'refused']) {
      // Includes a completed signal-only expression with zero retained rows:
      // lifecycle completion says nothing about whether pricing was requested.
      const result = { status: 'saved', evidence: { ...summary(0), operation, completion }, rows: [] };
      assert.equal(canInspectCandidateTrades(result), true, `${operation}/${completion}`);
      const trade = evidenceComparison(result).find((row) => row.area === 'Exact trade replay');
      assert.equal(trade?.status, 'Not verified in this summary', `${operation}/${completion}`);
      assert.match(trade?.detail ?? '', /Open candidate detail above to verify/);
      assert.match(trade?.detail ?? '', /unpriced signal rows remain explicit/);
    }
  }
});

test('index consistency lifecycle is an assessment record, not a completed search or passing strategy', async () => {
 const body = pageOf(0); body.evidence.operation = 'index-consistency';
 const result = await fetchSweepEvidence(identity, async () => response(body));
 assert.equal(result.evidence.operation, 'index-consistency');
 const rows = evidenceComparison(result);
 assert.equal(rows.find(row => row.area === 'AND combination search')?.status, 'Recorded index day/week assessment only');
 assert.equal(rows.find(row => row.area === 'Historical financial rules')?.status, 'Not established in this summary');
 assert.equal(canInspectCandidateTrades(result), false);
});

test('single-stop execution and qualification lifecycle require their own authenticated results', async () => {
 for (const [operation,label] of [['index-stop','Recorded single-stop executions only'],['index-stop-qualification','Recorded single-stop qualification only']]) {
  const body=pageOf(0);body.evidence.operation=operation;
  const result=await fetchSweepEvidence(identity,async()=>response(body)),rows=evidenceComparison(result);
  assert.equal(rows.find(row=>row.area==='AND combination search')?.status,label);
  assert.equal(rows.find(row=>row.area==='Historical financial rules')?.status,'Not established in this summary');
  assert.equal(rows.find(row=>row.area==='Institutional admission')?.status,'Authority not exposed here');
  assert.equal(canInspectCandidateTrades(result),false);
 }
});

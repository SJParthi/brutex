import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { auditUrl, validateInvocationAudit, fetchInvocationAudit, auditTime } from '../src/lib/invocation-audit.js';

const BASE = 1n << 63n, MAX = (1n << 64n) - 1n;

/** Generated protocol fixture, not a real command or market result.
 * @param {bigint} invocation @returns {any} */
function record(invocation = BASE + 1n) {
  return { invocation: String(invocation), origin: 'cli', operation: 'generated-audit-fixture',
    status: 'completed', terminal: true, at_millis: '946684800000', elapsed_micros: '0',
    completed_boundaries: '0', total_boundaries: null, response_status: null,
    meaning: 'Invocation only; no strategy or sweep admission is implied.' };
}

/** @param {bigint[]} [ids] @returns {any} */
function page(ids = [BASE + 3n, BASE + 2n, BASE + 1n]) {
  const tail = ids.at(-1);
  return { schema_version: 1, refusal: null, claim: 'Generated durable audit protocol fixture.',
    records: ids.map(record), next_before: tail && tail > BASE + 1n ? String(tail) : null };
}

/** @param {any} body @param {number} [status] */
const response = (body, status = 200) => new Response(JSON.stringify(body), { status });

/** @param {any} value */
function freeze(value) {
  if (value && typeof value === 'object') { Object.values(value).forEach(freeze); Object.freeze(value); }
  return value;
}

test('audit URLs preserve exact upper-half u64 cursors and always request only 32 records', () => {
  assert.equal(auditUrl(), '/backtest/audit.json?limit=32');
  assert.equal(auditUrl(null), '/backtest/audit.json?limit=32');
  for (const cursor of [BASE + 1n, BASE + 2n, MAX - 1n, MAX]) {
    assert.equal(auditUrl(String(cursor)), '/backtest/audit.json?limit=32&before=' + cursor);
  }
  assert.notEqual(auditUrl(String(BASE + 1n)), auditUrl(String(BASE + 2n)), 'adjacent IDs must not collapse to one JS number');
});

test('malformed, low-half, overflow and unsafe numeric cursors refuse before making a request', async () => {
  const invalid = ['', '0', '1', String(BASE), String(MAX + 1n), '0' + String(BASE + 1n),
    ' 9223372036854775809', '9223372036854775809 ', '9.223372036854776e18', '+9223372036854775809',
    '-1', '9223372036854775809.0', '٩٢٢٣٣٧٢٠٣٦٨٥٤٧٧٥٨٠٩', '9'.repeat(1000),
    1, Number(BASE + 1n), NaN, Infinity, BASE + 1n, true, {}, [], new String(BASE + 1n)];
  let requests = 0;
  for (const cursor of invalid) {
    assert.throws(() => auditUrl(cursor));
    await assert.rejects(fetchInvocationAudit(/** @type {any} */ (cursor), async () => { requests += 1; return response(page()); }));
  }
  assert.equal(requests, 0);
});

test('upper-half identities and counters retain every digit without rewriting frozen input', () => {
  const body = page([MAX, MAX - 1n, MAX - 2n]), before = structuredClone(body);
  body.records[0].elapsed_micros = String(MAX);
  body.records[0].completed_boundaries = String(MAX);
  before.records[0].elapsed_micros = String(MAX); before.records[0].completed_boundaries = String(MAX);
  assert.equal(validateInvocationAudit(freeze(body)), body);
  assert.deepEqual(body, before);
  assert.deepEqual(body.records.map((/** @type {any} */ row) => row.invocation),
    ['18446744073709551615', '18446744073709551614', '18446744073709551613']);
  assert.equal(body.records[0].completed_boundaries, '18446744073709551615');
  assert.equal(body.records[0].total_boundaries, null, 'a recorded count has no fabricated total');
});

test('newest-first pages are strictly below their exact cursor and reject repeats or reorderings', () => {
  const cursor = String(MAX - 1n);
  assert.equal(validateInvocationAudit(page([MAX - 2n, MAX - 3n]), cursor).records.length, 2);
  for (const ids of [[MAX - 1n], [MAX], [MAX - 2n, MAX - 2n], [MAX - 3n, MAX - 2n],
    [MAX - 2n, MAX - 4n, MAX - 3n]]) {
    assert.throws(() => validateInvocationAudit(page(ids), cursor), /identity, order/);
  }
  const first = page([MAX, MAX - 1n]), next = page([MAX - 2n, MAX - 3n]);
  validateInvocationAudit(first); validateInvocationAudit(next, first.next_before);
  const all = [...first.records, ...next.records].map(row => row.invocation);
  assert.equal(new Set(all).size, all.length);
  assert.throws(() => validateInvocationAudit(page([MAX - 1n, MAX - 2n]), first.next_before));
});

test('continuation is the exact oldest row even for a short page, and BASE+1 is the terminal boundary', () => {
  const short = page([BASE + 9n]);
  assert.equal(validateInvocationAudit(short).next_before, String(BASE + 9n), 'short does not establish total history');
  assert.equal(validateInvocationAudit(page([BASE + 1n])).next_before, null);
  assert.equal(validateInvocationAudit(page([]), String(BASE + 1n)).records.length, 0);
  for (const next of [null, String(BASE + 8n), String(BASE + 10n)]) {
    const body = page([BASE + 9n]); body.next_before = next;
    assert.throws(() => validateInvocationAudit(body), /continuation/);
  }
  const end = page([BASE + 1n]); end.next_before = String(BASE + 1n);
  assert.throws(() => validateInvocationAudit(end), /continuation/);
  const empty = page([]); empty.next_before = String(BASE + 3n);
  assert.throws(() => validateInvocationAudit(empty), /continuation/);
});

test('the 32-record page limit accepts its boundary and refuses larger or missing record arrays', () => {
  const ids = Array.from({ length: 32 }, (_, i) => MAX - BigInt(i));
  assert.equal(validateInvocationAudit(page(ids)).records.length, 32);
  assert.throws(() => validateInvocationAudit(page([...ids, MAX - 32n])), /unsupported/);
  for (const records of [undefined, null, {}, '[]', 0]) {
    const body = page(); body.records = records;
    assert.throws(() => validateInvocationAudit(body));
  }
});

test('missing or malformed schema, refusal and claim cannot masquerade as an empty history', () => {
  for (const body of [undefined, null, {}, [], '', { records: [] }]) {
    assert.throws(() => validateInvocationAudit(body));
  }
  /** @type {((body:any)=>void)[]} */ const damage = [
    b => { b.schema_version = 0; }, b => { b.schema_version = 2; }, b => { b.schema_version = '1'; },
    b => { delete b.refusal; }, b => { b.refusal = ''; }, b => { b.refusal = 'journal corrupt'; },
    b => { delete b.claim; }, b => { b.claim = {}; }, b => { b.next_before = '0'; },
    b => { b.next_before = String(BASE); }, b => { b.next_before = Number(MAX); },
    b => { b.next_before = String(MAX + 1n); }, b => { delete b.next_before; }
  ];
  for (const mutate of damage) { const body = page([]); mutate(body); assert.throws(() => validateInvocationAudit(body)); }
});

test('each supported origin and terminal status keeps its exact recorded meaning', () => {
  for (const origin of ['cli', 'browser', 'http']) {
    for (const status of ['unconfirmed', 'completed', 'refused', 'failed', 'cancelled']) {
      const body = page([BASE + 1n]), row = body.records[0];
      Object.assign(row, { origin, status, terminal: status !== 'unconfirmed' });
      assert.equal(validateInvocationAudit(body).records[0].status, status);
      assert.equal(validateInvocationAudit(body).records[0].terminal, status !== 'unconfirmed');
    }
  }
});

test('terminal-state contradictions and invented outcome/origin values are refused', () => {
  for (const status of ['unconfirmed', 'completed', 'refused', 'failed', 'cancelled']) {
    for (const terminal of [status === 'unconfirmed', undefined, null, 0, 1, 'true']) {
      const body = page([BASE + 1n]); Object.assign(body.records[0], { status, terminal });
      assert.throws(() => validateInvocationAudit(body), /recorded outcome/);
    }
  }
  for (const [key, value] of [['origin', 'worker'], ['origin', 'CLI'], ['origin', null],
    ['status', 'running'], ['status', 'admitted'], ['status', 'profitable'], ['status', null]]) {
    const body = page([BASE + 1n]); body.records[0][/** @type {string} */ (key)] = value;
    assert.throws(() => validateInvocationAudit(body));
  }
});

test('invocations and all recorded u64 quantities reject coercion, overflow and noncanonical values', () => {
  for (const field of ['invocation', 'at_millis', 'elapsed_micros', 'completed_boundaries']) {
    for (const value of [undefined, null, 0, Number(MAX), MAX, NaN, Infinity, '', '-1', '01', '1.0', '1e3', '+1',
      ' 1', '1 ', String(MAX + 1n), '1'.repeat(1000)]) {
      const body = page([BASE + 1n]); body.records[0][field] = value;
      assert.throws(() => validateInvocationAudit(body));
    }
  }
  for (const invocation of ['0', String(BASE)]) {
    const body = page([BASE + 1n]); body.records[0].invocation = invocation;
    assert.throws(() => validateInvocationAudit(body));
  }
  const zeroCounters = page([BASE + 1n]);
  zeroCounters.records[0].at_millis = '0';
  assert.equal(validateInvocationAudit(zeroCounters).records[0].at_millis, '0', 'an unknown saved time is retained, not rewritten');
});

test('unknown totals stay null and malformed HTTP status fields cannot invent response success', () => {
  for (const total of [undefined, 0, '0', '32', String(MAX), {}, false]) {
    const body = page(); body.records[0].total_boundaries = total;
    assert.throws(() => validateInvocationAudit(body));
  }
  for (const status of [null, 100, 200, 299, 400, 503, 599]) {
    const body = page(); body.records[0].response_status = status;
    assert.equal(validateInvocationAudit(body).records[0].response_status, status);
  }
  for (const status of [undefined, 0, 99, 600, -1, 200.5, '200', NaN, Infinity, true, {}]) {
    const body = page(); body.records[0].response_status = status;
    assert.throws(() => validateInvocationAudit(body));
  }
});

test('operation labels reject control bytes or excessive length without truncating recorded labels', () => {
  for (const value of ['', undefined, null, 1, 'x'.repeat(161), 'line\nfeed', 'tab\there', 'nul\0byte',
    'escape\u001bsequence', 'delete\u007fbyte']) {
    const body = page(); body.records[0].operation = value;
    assert.throws(() => validateInvocationAudit(body));
  }
  const body = page(); body.records[0].operation = 'x'.repeat(160);
  assert.equal(validateInvocationAudit(body).records[0].operation.length, 160);
  body.records[0].meaning = null;
  assert.throws(() => validateInvocationAudit(body));
  const sparse = page(); delete sparse.records[1];
  assert.throws(() => validateInvocationAudit(sparse));
});

test('the fetcher requests one exact bounded page and does not follow its continuation automatically', async () => {
  const cursor = String(MAX), body = page([MAX - 1n, MAX - 2n]);
  /** @type {string[]} */ const urls = [];
  const got = await fetchInvocationAudit(cursor, async url => { urls.push(url); return response(body); });
  assert.deepEqual(got, body);
  assert.deepEqual(urls, ['/backtest/audit.json?limit=32&before=18446744073709551615']);
  assert.equal(got.records[0].invocation, '18446744073709551614');
  assert.equal(got.next_before, '18446744073709551613');
});

test('non-success HTTP, network failures and invalid JSON never substitute a zero-record history', async () => {
  for (const status of [400, 403, 404, 429, 500, 503]) {
    await assert.rejects(fetchInvocationAudit(null, async () => response(page([]), status)), new RegExp('HTTP ' + status));
  }
  await assert.rejects(fetchInvocationAudit(null, async () => null), /HTTP undefined/);
  await assert.rejects(fetchInvocationAudit(null, async () => { throw new Error('socket failed'); }), /socket failed/);
  await assert.rejects(fetchInvocationAudit(null, async () => new Response('<html>proxy error</html>', { status: 200 })), SyntaxError);
  await assert.rejects(fetchInvocationAudit(null, async () => new Response(null, { status: 204 })), SyntaxError);
  await assert.rejects(fetchInvocationAudit(null, async () => response({ schema_version: 1, records: [] })), /unsupported/);
  const empty = await fetchInvocationAudit(null, async () => response(page([])));
  assert.deepEqual(empty.records, []); assert.equal(empty.next_before, null);
  assert.equal(Object.hasOwn(empty, 'total'), false);
});

test('a production-shaped journal failure preserves its bounded refusal for diagnosis', async () => {
  const refusal = { schema_version: 1, refusal: 'saved invocation checksum differs',
    code: 'invocation_audit_unavailable', handler_completed: false,
    why: 'The handler was not dispatched because its required audit start was unavailable.' };
  await assert.rejects(fetchInvocationAudit(null, async () => response(refusal, 503)), /saved invocation checksum differs/);
});

test('malformed or oversized refusal details remain failures without echoing arbitrary response bodies', async () => {
  for (const refusal of [null, {}, 'x'.repeat(4097)]) {
    const body = { schema_version: 1, refusal, code: 'invocation_audit_unavailable', handler_completed: false,
      why: 'Recorded journal read failed.' };
    await assert.rejects(fetchInvocationAudit(null, async () => response(body, 503)), error => {
      assert.match(String(error), /HTTP 503/);
      assert.ok(String(error).length < 256);
      return true;
    });
  }
  const wrong = { schema_version: 999, refusal: 'do-not-display-this-unverified-body',
    code: 'invocation_audit_unavailable', handler_completed: false, why: 'Recorded journal read failed.' };
  await assert.rejects(fetchInvocationAudit(null, async () => response(wrong, 503)), error => {
    assert.doesNotMatch(String(error), /do-not-display/); return true;
  });
});

test('production refusal metadata must validate independently before its explanation is displayed', async () => {
  /** @type {((body:any)=>void)[]} */ const damage = [
    b => { b.handler_completed = 'false'; }, b => { b.handler_completed = 0; },
    b => { delete b.handler_completed; }, b => { b.code = 'another_error'; },
    b => { b.why = {}; }, b => { b.why = 'x'.repeat(4097); },
    b => { delete b.why; }, b => { b.refusal = '   '; }
  ];
  for (const mutate of damage) {
    const body = { schema_version: 1, refusal: 'unvalidated-marker', code: 'invocation_audit_unavailable',
      handler_completed: false, why: 'Recorded journal read failed.' };
    mutate(body);
    await assert.rejects(fetchInvocationAudit(null, async () => response(body, 503)), error => {
      assert.match(String(error), /HTTP 503/);
      assert.doesNotMatch(String(error), /unvalidated-marker|Recorded journal/);
      assert.ok(String(error).length < 256);
      return true;
    });
  }
  const completed = { schema_version: 1, refusal: 'terminal audit did not settle',
    code: 'invocation_audit_unavailable', handler_completed: true,
    why: 'The handler already ran. Inspect its exact invocation before retrying.' };
  await assert.rejects(fetchInvocationAudit(null, async () => response(completed, 503)), /handler already ran/);
});

test('a standard bounded refusal also remains a failure with its reason preserved', async () => {
  const body = { schema_version: 1, status: 'refused', rows: /** @type {any[]} */ ([]), refusal: 'bounded reader capacity is full' };
  await assert.rejects(fetchInvocationAudit(null, async () => response(body, 429)), /HTTP 429.*bounded reader capacity is full/);
  body.rows = /** @type {any[]} */ ([record(BASE + 1n)]);
  await assert.rejects(fetchInvocationAudit(null, async () => response(body, 429)), error => {
    assert.match(String(error), /HTTP 429/); assert.doesNotMatch(String(error), /bounded reader capacity/); return true;
  });
});

test('typed read failures preserve the storage reason without claiming a write was attempted', async () => {
  for (const status of [429, 503]) {
    const body = { schema_version: 1, code: 'invocation_audit_read_unavailable',
      refusal: 'invocation index is busy',
      why: 'No audit snapshot was published. Reading this endpoint does not start an engine task or append an audit record.' };
    await assert.rejects(fetchInvocationAudit(null, async () => response(body, status)), error => {
      assert.match(String(error), /invocation index is busy/);
      assert.match(String(error), /does not start an engine task/);
      assert.doesNotMatch(String(error), /required audit start/);
      return true;
    });
    for (const damage of [
      { schema_version: 2 }, { refusal: '' }, { refusal: 'x'.repeat(4097) },
      { why: 'x'.repeat(4097) }, { handler_completed: false }
    ]) {
      await assert.rejects(fetchInvocationAudit(null, async () => response({ ...body, ...damage }, status)), error => {
        assert.doesNotMatch(String(error), /invocation index is busy/);
        return true;
      });
    }
  }
});

test('date display is IST for known times and explicitly unavailable for zero, invalid or out-of-range time', () => {
  assert.match(auditTime('946684800000'), /1 Jan 2000/);
  assert.match(auditTime('946684800000'), /5:30:00/);
  for (const value of ['0', '', '-1', '01', 'NaN', '1e9', '8640000000000001', String(MAX),
    String(MAX + 1n), null, undefined, 946684800000, NaN, Infinity]) {
    assert.equal(auditTime(/** @type {any} */ (value)), 'Time unavailable');
  }
  assert.notEqual(auditTime('8640000000000000'), 'Time unavailable');
  const unknownTime = page([BASE + 1n]); unknownTime.records[0].at_millis = String(MAX);
  assert.equal(validateInvocationAudit(unknownTime).records[0].at_millis, String(MAX));
  assert.equal(auditTime(unknownTime.records[0].at_millis), 'Time unavailable', 'valid stored u64 does not imply a valid display date');
});

test('the panel links exact strings, separates request return from search acceptance and exposes failures', () => {
  const view = readFileSync(new URL('../src/lib/InvocationAudit.svelte', import.meta.url), 'utf8');
  assert.match(view, /invocation=' \+ record\.invocation/);
  assert.match(view, /ticket\.current\(\)/);
  assert.match(view, /loaded = \{ phase: 'failed', body: null/);
  assert.match(view, /No empty history or completion is substituted/);
  assert.match(view, /does not prove the process is still running/);
  assert.match(view, /separate from a completed search and a passing strategy/);
  assert.match(view, /loaded\.body\.next_before === null/);
  assert.match(view, /record\.completed_boundaries/);
  assert.doesNotMatch(view, /Number\(record\.invocation\)|parseInt\(record\.invocation\)|\{@html/);
});

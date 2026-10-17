// R3-08, NEW-21: a month being written is asked again by itself, on every
// page read that can meet a writer. Every fixture is invented; no row here is
// market data, and the prices cannot be mistaken for real ones.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  BEING_WRITTEN_TRIES, askAgainDelay, askAgainWhileWritten, askWhileBeingWritten, isBeingWritten
} from '../src/lib/being-written.js';
import { refusalFrom } from '../src/lib/refusal.js';
import { readSpots } from '../src/lib/moneyness.js';
import { readGreeks } from '../src/lib/greeks.js';

const SENTENCE = 'being written, ask again: another writer holds /s/bars/x.lock (held by this process (pid 42, thread import-3))';
/** @param {Record<string, string>} [headers] */
const busy = (headers = {}) => /** @type {any} */ ({
  status: 503,
  headers: { get: (/** @type {string} */ name) => headers[name] ?? null },
  json: async () => ({ error: SENTENCE, being_written: true })
});

test('a 503 that says being written is asked again after the server’s delay, at most the tries in all', async () => {
  /** @type {number[]} */
  const waited = [];
  let calls = 0;
  const request = async () => { calls += 1; return busy({ 'Retry-After': '5' }); };
  const { res, body } = await askWhileBeingWritten(request, '/spot.json?x', new AbortController().signal,
    async (ms) => { waited.push(ms); });
  assert.equal(calls, BEING_WRITTEN_TRIES);
  assert.equal(waited.length, BEING_WRITTEN_TRIES - 1);
  assert.ok(waited.every((ms) => ms >= 2000 && ms < 2250), `Retry-After is capped at two seconds: ${waited}`);
  assert.equal(res.status, 503);
  assert.equal(body.error, SENTENCE, 'past the tries the server’s sentence, naming the holder, is the answer');
});

test('only a 503 with the field is a come-back; any other answer returns at once', async () => {
  for (const answer of [
    { status: 503, json: async () => ({ error: 'the census did not read' }) },
    { status: 200, json: async () => ({ being_written: true }) },
    { status: 429, json: async () => ({ error: 'busy' }) }
  ]) {
    let calls = 0;
    await askWhileBeingWritten(async () => { calls += 1; return /** @type {any} */ (answer); }, '/x',
      new AbortController().signal, async () => { throw new Error('must not wait'); });
    assert.equal(calls, 1, `status ${answer.status}`);
  }
  assert.equal(isBeingWritten({ status: 503 }, { being_written: 'true' }), false, 'the field is a boolean');
  assert.equal(askAgainDelay({}, () => 0), 1000, 'one second when the server names none');
  assert.equal(askAgainDelay({ headers: { get: () => 'soon' } }, () => 0), 1000, 'a garbage header is none');
  assert.equal(askAgainDelay({ headers: { get: () => '1' } }, () => 0.999), 1249);
});

test('a reader who leaves stops the asking', async () => {
  const gone = new AbortController();
  let calls = 0;
  const request = async () => { calls += 1; gone.abort(); return busy(); };
  await assert.rejects(askWhileBeingWritten(request, '/x', gone.signal, async () => {}));
  assert.equal(calls, 1);
});

const OPTION = 'NSE-FNO-NIFTY-2099-01-28-9900000-CE';
const at = 4_070_000_000;
const row = { instrument: OPTION, tf: '1s', month: '2099-01', ts: at * 1000, side: 'CE' };

test('the spot cells ask a month being written again, then file the answer', async () => {
  let calls = 0;
  const request = async () => {
    calls += 1;
    if (calls < 3) return busy({ 'Retry-After': '1' });
    return /** @type {any} */ ({ status: 200, json: async () => ({ asof: 'exact', segment: 'INDEX', key: 'NIFTY', faults: null,
      spots: [{ t: at, close: 123_456_789, spot_t: at, spot_age_s: 0, why: null }] }) });
  };
  const out = await readSpots('gdfl', [row], new AbortController().signal, request, 'exact', async () => {});
  assert.equal(calls, 3);
  const [spot] = [...out.values()];
  assert.equal(spot.close, 123_456_789);
  assert.equal(spot.why, null);
});

test('the Greeks cells ask a month being written again; past the tries the holder is the detail', async () => {
  let calls = 0;
  const request = async () => { calls += 1; return busy(); };
  const { cells } = await readGreeks('gdfl', [row], new AbortController().signal, request, 'exact', async () => {});
  assert.equal(calls, BEING_WRITTEN_TRIES);
  const [cell] = [...cells.values()];
  assert.equal(cell.why, 'greeks_request_failed');
  assert.match(String(cell.detail), /being written, ask again.*pid 42/);
  assert.doesNotMatch(String(cell.detail), /unreadable/);
});

// THE SHAPE THE SERVER SENDS: `crate::detail::being_written_answer`'s body,
// `{"error": <the holders' sentences>, "being_written": true}`, with the
// `Retry-After: 1` that `count_route` adds. `/sources/bars.json`,
// `/dayflags.json` and the window all answer a writer with it (R3-08, RA-117).
const written = () => Response.json({ error: SENTENCE, being_written: true },
  { status: 503, headers: { 'Retry-After': '1' } });

test('a folded rung or the day flags beside a writer are asked again, and the response comes back unread (RA-117)', async () => {
  /** @type {number[]} */
  const waited = [];
  let calls = 0;
  const request = async () => {
    calls += 1;
    return calls < 3 ? written() : Response.json({ days: [] }, { status: 200 });
  };
  const res = await askAgainWhileWritten(request, '/dayflags.json?x', { cache: 'no-store' },
    async (ms) => { waited.push(ms); });
  assert.equal(calls, 3, 'asked again by itself until the writer let go');
  assert.equal(waited.length, 2);
  assert.ok(waited.every((ms) => ms >= 1000 && ms < 1250), `the server's own delay: ${waited}`);
  assert.equal(res.status, 200);
  assert.equal(res.bodyUsed, false, 'the caller reads the body itself');
  assert.deepEqual(await res.json(), { days: [] });
});

test('past the tries the last 503 is handed back unread, and its refusal names the holder (RA-117)', async () => {
  let calls = 0;
  const res = await askAgainWhileWritten(async () => { calls += 1; return written(); }, '/sources/bars.json?x',
    { signal: new AbortController().signal }, async () => {});
  assert.equal(calls, BEING_WRITTEN_TRIES, 'bounded: never an endless retry');
  assert.equal(res.status, 503);
  const said = await refusalFrom('/sources/bars.json', res);
  assert.match(said, /being written, ask again.*pid 42, thread import-3/);
  assert.doesNotMatch(said, /unreadable|named no reason/);
});

test('any other answer is handed back at once, unread, and a reader who leaves stops the asking', async () => {
  for (const answer of [
    () => Response.json({ error: 'the census did not read' }, { status: 503, headers: { 'Retry-After': '1' } }),
    () => new Response('not json', { status: 503 }),
    () => Response.json({ being_written: true }, { status: 200 }),
    () => Response.json({ error: 'busy' }, { status: 429, headers: { 'Retry-After': '1' } })
  ]) {
    let calls = 0;
    const res = await askAgainWhileWritten(async () => { calls += 1; return answer(); }, '/x', {},
      async () => { throw new Error('must not wait'); });
    assert.equal(calls, 1, `status ${res.status}`);
    assert.equal(res.bodyUsed, false);
  }
  const gone = new AbortController();
  let calls = 0;
  await assert.rejects(askAgainWhileWritten(async () => { calls += 1; gone.abort(); return written(); }, '/x',
    { signal: gone.signal }, async (_ms, signal) => signal.throwIfAborted()));
  assert.equal(calls, 1);
});

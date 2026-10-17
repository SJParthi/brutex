// R3-08, NEW-21: a month being written is asked again by itself, on every
// page read that can meet a writer. Every fixture is invented; no row here is
// market data, and the prices cannot be mistaken for real ones.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  BEING_WRITTEN_TRIES, askAgainDelay, askWhileBeingWritten, isBeingWritten
} from '../src/lib/being-written.js';
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

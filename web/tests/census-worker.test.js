// WP-56, D-3327; DB-G84, DB-N07: the census worker's protocol, the page's
// side of it, and the timed slices every census pass on the page runs in.
// Every census below is invented for the fixture.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { answerMessage, createCensusWorker, inSlices, yieldTask, packStrings, unpackStrings, heldNames } from '../src/lib/census-worker.js';
import { decodeCensus, CENSUS_ENCODING } from '../src/lib/store-census-wire.js';

const census = () => ({ schema: 1, encoding: CENSUS_ENCODING, feed: 'example',
  instruments: ['NSE-INDEX-SAMPLE', 'NSE-FNO-SAMPLE-2020-01-30-1234500-PE', 'NSE-CASH-ÄBC'], months: ['2020-01', '2020-02'],
  timeframes: ['1s', '1min'], reasons: ['previous_not_stored'],
  rows: [[0, 0, 0, 3, 10, 20, 15, null, null, 0], [1, 1, 0, 1, 30, 30, null, 0, -4, null], [2, 0, 1, 0, 0, 0, 0, null, 0, null]] });
const bytesOf = (/** @type {unknown} */ value) => new TextEncoder().encode(JSON.stringify(value)).buffer;
const rowsOf = async (/** @type {unknown} */ value) => decodeCensus(value, 'example', new AbortController().signal, async () => {});

test('the worker answers a census as columns, every number column moved without a copy', async () => {
  const { reply, transfer } = answerMessage({ op: 'decode', id: 7, feed: 'example', buffer: bytesOf(census()) });
  assert.equal(reply.op, 'decoded');
  assert.equal(reply.id, 7);
  assert.equal(reply.cols.n, 3);
  assert.deepEqual(unpackStrings(reply.cols.instruments), census().instruments);
  assert.equal(transfer.length, 11, 'ten number columns and the instrument ends');
  assert.ok(transfer.every((buffer) => buffer instanceof ArrayBuffer));
});

test('the worker refuses a malformed census in the page decoder\'s own words, and a legacy array as itself', async () => {
  const bad = census();
  bad.rows[0][0] = 9;
  const { reply } = answerMessage({ op: 'decode', id: 1, feed: 'example', buffer: bytesOf(bad) });
  await assert.rejects(rowsOf(bad), (why) => /** @type {Error} */ (why).message === reply.message);
  assert.equal(reply.op, 'error');
  const legacy = answerMessage({ op: 'decode', id: 2, feed: 'example', buffer: bytesOf([{ instrument: 'NSE-INDEX-SAMPLE' }]) });
  assert.deepEqual(legacy.reply.legacy, [{ instrument: 'NSE-INDEX-SAMPLE' }]);
  assert.equal(answerMessage({ op: 'later', id: 3 }).reply.message, 'unknown census worker request "later"');
  assert.match(answerMessage({ op: 'decode', id: 4, feed: 'example', buffer: bytesOf('').slice(0, 0) }).reply.message, /JSON/);
});

test('strings packed as one text and its ends come back exactly, empty and non-ASCII ones too', () => {
  const list = ['', 'NIFTY', 'Ä€😀', '', 'a-b-c'];
  const packed = packStrings(list);
  assert.equal(packed.ends.length, list.length);
  assert.deepEqual(unpackStrings(packed), list);
  assert.deepEqual(unpackStrings(packStrings([])), []);
});

test('names held counts distinct underlyings, the unit every universe row counts (DB-N07)', () => {
  assert.equal(heldNames([{ underlying: 'SAMPLE' }, { underlying: 'SAMPLE' }, { underlying: 'OTHER' }]), 2);
  assert.equal(heldNames([]), 0);
});

/** A worker that runs `answerMessage` a turn later, as a real one would. */
class FakeWorker {
  /** @param {{die?: boolean}} [opts] */
  constructor(opts = {}) {
    this.opts = opts;
    /** @type {any} */ this.onmessage = null;
    /** @type {any} */ this.onerror = null;
    /** @type {any} */ this.onmessageerror = null;
    this.terminated = false;
    /** @type {any[]} */ this.sent = [];
  }
  /** @param {any} message */
  postMessage(message) {
    this.sent.push(message);
    setImmediate(() => {
      if (this.opts.die) return this.onerror?.({ message: 'out of memory', preventDefault() {} });
      const { reply } = answerMessage(structuredClone(message));
      this.onmessage?.({ data: structuredClone(reply) });
    });
  }
  terminate() { this.terminated = true; }
}

const immediate = { yieldControl: async () => {} };

test('the page rebuilds the worker\'s columns into the rows decodeCensus builds, and keeps its bytes', async () => {
  const worker = new FakeWorker();
  const client = createCensusWorker(() => /** @type {any} */ (worker), immediate);
  const bytes = bytesOf(census());
  const rows = await client.decode(bytes, 'example');
  assert.deepEqual(rows, await rowsOf(census()));
  assert.equal(bytes.byteLength > 0, true, 'the bytes were copied, not moved: still readable for a fallback');
  assert.equal(client.broken, null);
  const legacy = await client.decode(bytesOf([{ instrument: 'x' }]), 'example');
  assert.deepEqual(legacy, [{ instrument: 'x' }]);
});

test('a worker that dies is named broken, every waiting read is refused with the reason, and no worker is started again', async () => {
  let spawned = 0;
  const client = createCensusWorker(() => { spawned += 1; return /** @type {any} */ (new FakeWorker({ die: true })); }, immediate);
  await assert.rejects(client.decode(bytesOf(census()), 'example'), /the census worker failed: out of memory/);
  assert.equal(client.broken, 'the census worker failed: out of memory');
  await assert.rejects(client.decode(bytesOf(census()), 'example'), /out of memory/);
  assert.equal(spawned, 1);
  const unstartable = createCensusWorker(() => { throw new Error('no workers here'); });
  await assert.rejects(unstartable.decode(bytesOf(census()), 'example'), /could not start: no workers here/);
  assert.match(unstartable.broken ?? '', /no workers here/);
});

test('a cancelled read is refused at once and its late answer is dropped', async () => {
  const worker = new FakeWorker();
  const client = createCensusWorker(() => /** @type {any} */ (worker), immediate);
  const controller = new AbortController();
  const read = client.decode(bytesOf(census()), 'example', controller.signal);
  controller.abort(new Error('superseded'));
  await assert.rejects(read, /superseded/);
  await new Promise((done) => setImmediate(done));
  assert.equal(client.broken, null, 'a late answer to a cancelled read breaks nothing');
  const next = await client.decode(bytesOf(census()), 'example');
  assert.equal(next.length, 3);
});

test('slices stop at the budget and yield one task between them; a cancelled pass stops at the next slice', async () => {
  let now = 0;
  let yields = 0;
  /** @type {[number, number][]} */
  const steps = [];
  await inSlices(10, (start, end) => { steps.push([start, end]); now += 3; }, undefined,
    { yieldControl: async () => { yields += 1; }, budgetMs: 8, step: 2, clock: () => now });
  // 3 ms per step of 2 items: three steps reach 9 ms, past the 8 ms budget.
  assert.deepEqual(steps, [[0, 2], [2, 4], [4, 6], [6, 8], [8, 10]]);
  assert.equal(yields, 1);
  const controller = new AbortController();
  let seen = 0;
  await assert.rejects(inSlices(100, () => { seen += 1; now += 10; }, controller.signal,
    { yieldControl: async () => controller.abort(), budgetMs: 8, step: 10, clock: () => now }), /abort/i);
  assert.equal(seen, 1);
  await inSlices(0, () => { throw new Error('nothing to do'); }, undefined);
});

test('yieldTask ends the task: a timer that fell due while the work ran runs before the work resumes', async () => {
  /** @type {string[]} */
  const order = [];
  setTimeout(() => order.push('timer'), 0);
  const began = performance.now();
  while (performance.now() - began < 5) { /* the work: the timer falls due meanwhile */ }
  order.push('work');
  await yieldTask();
  order.push('resumed');
  // A microtask-only "yield" (`await Promise.resolve()`) resumes before the
  // timer, which is the starvation DB-G84 measured as a two-second stall.
  assert.deepEqual(order, ['work', 'timer', 'resumed']);
});

// WP-56b (the WP-56 second agent's follow-up 1): a worker that takes the
// message and never answers left the page on "Reading /store.json" for good.
test('a worker that never answers is given up on at the ceiling, named, and no timer is left behind an answer', async () => {
  /** @type {{work: () => void, ms: number, cleared: boolean}[]} */
  const timers = [];
  const clock = {
    setTimer: (/** @type {() => void} */ work, /** @type {number} */ ms) => { const t = { work, ms, cleared: false }; timers.push(t); return t; },
    clearTimer: (/** @type {any} */ t) => { t.cleared = true; }
  };
  const mute = { sent: /** @type {any[]} */ ([]), terminated: false, postMessage(/** @type {any} */ m) { this.sent.push(m); }, terminate() { this.terminated = true; } };
  const client = createCensusWorker(() => /** @type {any} */ (mute), { ...immediate, ...clock, ceilingMs: 4_000 });
  const first = client.decode(bytesOf(census()), 'example');
  const second = client.decode(bytesOf(census()), 'example');
  assert.equal(mute.sent.length, 2);
  assert.deepEqual(timers.map((t) => t.ms), [4_000, 4_000], 'one ceiling per read, of the length asked');
  let settled = false;
  first.then(() => { settled = true; }, () => { settled = true; });
  await new Promise((done) => setImmediate(done));
  assert.equal(settled, false, 'nothing gives up before the ceiling');
  timers[0].work();
  await assert.rejects(first, /the census worker did not answer within 4 s/);
  await assert.rejects(second, /did not answer within 4 s/, 'every read waiting on that worker is refused with it');
  assert.equal(client.broken, 'the census worker did not answer within 4 s');
  assert.equal(mute.terminated, true);
  assert.equal(timers.every((t) => t.cleared), true, 'no ceiling outlives its read');
  await assert.rejects(client.decode(bytesOf(census()), 'example'), /did not answer/, 'and the worker is not tried again');

  // An answer in time clears its ceiling; a ceiling that fires late breaks nothing.
  const timers2 = /** @type {typeof timers} */ ([]);
  const answering = createCensusWorker(() => /** @type {any} */ (new FakeWorker()), {
    ...immediate, clearTimer: clock.clearTimer,
    setTimer: (work, ms) => { const t = { work, ms, cleared: false }; timers2.push(t); return t; }
  });
  assert.equal((await answering.decode(bytesOf(census()), 'example')).length, 3);
  assert.equal(timers2.length, 1);
  assert.equal(timers2[0].cleared, true);
  timers2[0].work();
  assert.equal(answering.broken, null, 'a ceiling of a read already answered is not a failure');

  // A cancelled read clears its ceiling too.
  const timers3 = /** @type {typeof timers} */ ([]);
  const quiet = createCensusWorker(() => /** @type {any} */ ({ postMessage() {}, terminate() {} }), {
    ...immediate, clearTimer: clock.clearTimer,
    setTimer: (work, ms) => { const t = { work, ms, cleared: false }; timers3.push(t); return t; }
  });
  const controller = new AbortController();
  const read = quiet.decode(bytesOf(census()), 'example', controller.signal);
  controller.abort(new Error('superseded'));
  await assert.rejects(read, /superseded/);
  assert.equal(timers3[0].cleared, true);
  assert.equal(quiet.broken, null);
});

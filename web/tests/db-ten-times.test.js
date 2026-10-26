// R3-12 (RA-005, RA-026, RA-027; the fix plan's gate): /db LOADS AT TEN TIMES
// TODAY'S LIST WITHOUT A LONG TASK.
//
// Today's list is the 12 Oct acceptance case: GDFL's census held 602,732 rows,
// NIFTY 404 expiries, and its three segments some 200 underlyings each. This
// gate builds a fake server at TEN TIMES that (6,027,320 census rows behind
// /store.json, 4,040 expiries, 2,000 underlyings a segment), drives the store
// module's own read of every /db view through it, and the page's own list
// rules over its answers, while a turn of the event loop is timed between
// every task: no stretch of the page's work may hold the loop past 50 ms, the
// whole census is never asked, and an open of the spot view stays under the
// fix plan's 400 KB on the wire. Every number is invented.
//
// It is a node measurement of the page's code, not a browser's: the browser
// targets (first content, memory, a picker click drawn) are the Mac's run
// (R3-12 report, NEEDS THE MAC, M17).
//
// A TURN IS TIMED IN THE MAIN THREAD'S OWN CPU TIME (R3-12 merge-back, 25 Oct
// 2026). Timed by the wall clock, the gate failed on the Mac for time the OS
// gave other processes: under a load average of 28 to 32 on 14 cores, three
// runs in five read 50.4 to 101.7 ms on stretches that hold the loop 3 to
// 34 ms on their own. A long task is the page's work; a slice of the machine
// handed to another build is not, so the bound holds on the thread's own time
// (`process.threadCpuUsage`, Node 23.9 and later; CI pins 24), and the wall
// time of each stretch is written beside it on the run's diagnostic line.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { storeModule, rootBody, rowsBody, ORDER, HEADERS } from './store-module.js';
import { rowsUrl, facetsUrl, scopeUnderlyings, viewSegments } from '../src/lib/store-census.js';
import { listedOf, segmentOf } from '../src/lib/db/selection-model.js';

const TIMES = 10;
const CENSUS_ROWS = 602_732 * TIMES;
const EXPIRIES = 404 * TIMES;
const UNDERLYINGS = 200 * TIMES;
const LONG_TASK_MS = 50;

/** `n` weekly expiries from 2018-09-06, as ISO days. */
const weekly = (/** @type {number} */ n) => Array.from({ length: n }, (_, at) => new Date(Date.UTC(2018, 8, 6) + at * 7 * 86_400_000).toISOString().slice(0, 10));

/** A fake server at ten times today's list; every byte it sends is counted. */
function tenTimes() {
  const feed = 'example';
  /** @type {string[]} */
  const asked = [];
  let bytes = 0;
  const expiries = weekly(EXPIRIES);
  const symbols = Array.from({ length: UNDERLYINGS }, (_, at) => `NAME${String(at).padStart(5, '0')}`);
  const underlyings = (/** @type {string} */ segment) => symbols.map((symbol) => ({ exchange: 'NSE', symbol, months: 96, expiries: segment === 'FNO' ? 400 : 0,
    contracts: segment === 'FNO' ? 30_000 : 0, unread_contracts: 0, rows: { '1min': 700_000 }, first_ts: 1, last_ts: 2 }));
  const listing = (/** @type {string} */ segment, /** @type {string | null} */ underlying) => ({ schema: 1, feed, segment, underlying, expiry: null,
    generation: segment === 'INDEX' ? 7 : segment === 'CASH' ? 5 : 9, since: null, state: 'held', degraded: null, order: ORDER,
    underlyings: underlying ? [{ ...underlyings(segment)[0], symbol: underlying }] : underlyings(segment), months: [], strikes: [], futures: [],
    expiries: underlying && segment === 'FNO' ? expiries.map((expiry) => ({ expiry, weekday: 'Thu', contracts: 120, strikes: 60, future: true, rows: { '1s': 9_000_000 }, first_ts: 1, last_ts: 2 })) : [] });
  /** Ninety-six months of one spot underlying at three rungs. */
  const spotRows = (/** @type {string} */ segment, /** @type {string} */ underlying) => {
    /** @type {Array<[string, string, string, number]>} */
    const list = [];
    for (let m = 0; m < 96; m++) {
      const month = `${2018 + Math.floor(m / 12)}-${String((m % 12) + 1).padStart(2, '0')}`;
      for (const rung of ['1min', '5min', '1day']) list.push([`NSE-${segment}-${underlying}`, month, rung, 7_000]);
    }
    return list;
  };
  /** @param {any} url */
  const fetch = async (url) => {
    const text = String(url);
    asked.push(text);
    const query = new URL(text, 'http://fake').searchParams;
    const segment = query.get('segment');
    const underlying = query.get('underlying');
    const expiry = query.get('expiry');
    /** @type {[number, any]} */
    let answer = [404, { error: 'not_here', why: `the fake server answers no ${text}` }];
    if (text === facetsUrl({ feed, segment: /** @type {any} */ (null) })) answer = [200, rootBody(feed)];
    else if (text.startsWith('/store.json')) answer = [200, { error: 'never', why: `${CENSUS_ROWS} rows would have been sent` }];
    else if (segment && underlying && text === rowsUrl({ feed, segment, underlying, ...(expiry ? { expiry } : {}) })) {
      answer = segment !== 'FNO'
        ? [200, rowsBody(feed, segment, underlying, segment === 'INDEX' ? 7 : 5, spotRows(segment, underlying))]
        : [200, rowsBody(feed, segment, underlying, 9, Array.from({ length: 120 }, (_, n) =>
          [`NSE-FNO-${underlying}-${expiry}-${1_000_000 + n * 5_000}-${n % 2 ? 'PE' : 'CE'}`, String(expiry).slice(0, 7), '1s', 3_000]), expiry)];
    } else if (segment) answer = [200, listing(segment, underlying)];
    const out = JSON.stringify(answer[1]);
    bytes += out.length;
    return new Response(out, { status: answer[0], headers: HEADERS });
  };
  return { fetch, asked, bytes: () => bytes, symbols };
}

/** The main thread's own CPU time so far, in ms (user and system). */
function threadMs() {
  // REFUSED BY NAME, NEVER MEASURED ANOTHER WAY: a node without the call
  // cannot time the page's own work, and the wall clock is what failed.
  assert.equal(typeof process.threadCpuUsage, 'function', 'process.threadCpuUsage is missing: this gate needs Node 23.9 or later (CI pins 24)');
  const used = process.threadCpuUsage();
  return (used.user + used.system) / 1_000;
}

/**
 * The longest gap between two turns of the event loop while `work` runs, as
 * the main thread's own CPU time (`cpu`, the bound) and as the wall clock
 * (`wall`, said beside it).
 * @param {() => Promise<void>} work
 * @returns {Promise<{cpu: number, wall: number}>}
 */
async function longestTask(work) {
  let cpu = 0;
  let wall = 0;
  let lastCpu = threadMs();
  let lastWall = performance.now();
  let running = true;
  const beat = () => {
    const nowCpu = threadMs();
    const nowWall = performance.now();
    cpu = Math.max(cpu, nowCpu - lastCpu);
    wall = Math.max(wall, nowWall - lastWall);
    lastCpu = nowCpu;
    lastWall = nowWall;
    if (running) setImmediate(beat);
  };
  setImmediate(beat);
  await work();
  // The beat queued before this turn runs first, so the stretch that ends the
  // work is timed too: a work with no await of its own (the list rules) was
  // otherwise never timed at all and read 0 ms.
  await new Promise((done) => setImmediate(done));
  running = false;
  return { cpu, wall };
}

test('db_loads_at_ten_times_the_list_without_a_long_task', async (t) => {
  const server = tenTimes();
  const real = globalThis.fetch;
  globalThis.fetch = /** @type {any} */ (server.fetch);
  try {
    const { store, syncStore } = await storeModule();
    /** @param {any} wanted */
    const open = async (wanted) => {
      syncStore('example', wanted);
      for (let i = 0; i < 2_000 && store.state === 'reading'; i++) await new Promise((done) => setImmediate(done));
      assert.equal(store.state, 'ready', String(store.error));
    };
    // ONE UNTIMED OPEN OF ANOTHER INSTRUMENT FIRST: the first call compiles
    // the module's code (40 to 75 ms measured here), which in a browser is
    // the page script's own parse, the page-shell cost NEW-08 measures, and
    // the same at any list size. Every timed open below reads its own data.
    await open({ underlying: server.symbols[1], expiry: null, segments: [] });
    // THE OPENING VIEW (spot), its wire, and its longest task.
    const spotBytesFrom = server.bytes();
    const spot = await longestTask(() => open({ underlying: server.symbols[0], expiry: null, segments: ['spot'] }));
    const spotBytes = server.bytes() - spotBytesFrom;
    assert.ok(spot.cpu < LONG_TASK_MS, `the spot view held the loop ${spot.cpu.toFixed(1)} ms of its own (${spot.wall.toFixed(1)} ms by the wall)`);
    assert.ok(spotBytes < 400_000, `the spot view moved ${spotBytes} bytes`);
    assert.equal(store.rows.length, 2 * 96 * 3, 'its rows, both spot segments');
    // THE OPTIONS VIEW WITH NO EXPIRY: 4,040 expiries offered from the listing.
    const options = await longestTask(() => open({ underlying: server.symbols[0], expiry: null, segments: ['options'] }));
    assert.ok(options.cpu < LONG_TASK_MS, `the options view held the loop ${options.cpu.toFixed(1)} ms of its own (${options.wall.toFixed(1)} ms by the wall)`);
    assert.equal(store.scope.expiries.length, EXPIRIES);
    // AN EXPIRY CHOSEN: its chain, beside the listing.
    const chain = await longestTask(() => open({ underlying: server.symbols[0], expiry: store.scope.expiries[7].expiry, segments: ['options'] }));
    assert.ok(chain.cpu < LONG_TASK_MS, `the chain held the loop ${chain.cpu.toFixed(1)} ms of its own (${chain.wall.toFixed(1)} ms by the wall)`);
    // THE PAGE'S LIST RULES OVER THE ANSWERS: the instrument list of 2,000
    // names a segment, and each segment key's expiries out of 4,040.
    const scope = store.scope;
    const segmentsAnswers = await Promise.all(['INDEX', 'CASH', 'FNO'].map(async (segment) =>
      ({ ok: true, body: await (await server.fetch(facetsUrl({ feed: 'example', segment }))).json() })));
    const lists = await longestTask(async () => {
      const every = { ...scope, segments: ['INDEX', 'CASH', 'FNO'] };
      const { rows, why } = scopeUnderlyings(every, segmentsAnswers);
      assert.equal(why, null);
      assert.equal(rows.length, UNDERLYINGS);
      for (const key of ['spot', 'futures', 'options', 'futures-cut', 'options-cut', 'futures-live', 'options-live']) {
        listedOf(scope.expiries, key, (/** @type {any} */ e, /** @type {string} */ family) =>
          segmentOf({ expiry: e.expiry, instrument: family === 'futures' ? 'x-FUT' : 'x-CE' }, '2026-10-24', '2026-09-24'));
      }
      viewSegments(scope.listed, []);
    });
    assert.ok(lists.cpu < LONG_TASK_MS, `the page's list rules held the loop ${lists.cpu.toFixed(1)} ms of its own (${lists.wall.toFixed(1)} ms by the wall)`);
    // The run's numbers, for the report's n = 20 table (a TAP diagnostic line).
    t.diagnostic(`ten-times spot_ms=${spot.cpu.toFixed(2)} options_ms=${options.cpu.toFixed(2)} chain_ms=${chain.cpu.toFixed(2)} lists_ms=${lists.cpu.toFixed(2)}` +
      ` wall spot_ms=${spot.wall.toFixed(2)} options_ms=${options.wall.toFixed(2)} chain_ms=${chain.wall.toFixed(2)} lists_ms=${lists.wall.toFixed(2)} spot_bytes=${spotBytes}`);
    // THE WHOLE CENSUS WAS NEVER ASKED, at any size.
    assert.deepEqual(server.asked.filter((url) => url.startsWith('/store.json')), []);
  } finally {
    globalThis.fetch = real;
  }
});

// The gate's own timer: a stretch with no await of its own is a long task too.
// Before the last beat was awaited, this read 0 ms, and the list-rules phase
// above asserted nothing.
test('the_timer_times_a_stretch_with_no_await', async () => {
  const held = await longestTask(async () => {
    const until = threadMs() + 2 * LONG_TASK_MS;
    while (threadMs() < until);
  });
  assert.ok(held.cpu >= 2 * LONG_TASK_MS, `a ${2 * LONG_TASK_MS} ms stretch read ${held.cpu.toFixed(1)} ms of the thread's own`);
  assert.ok(held.wall >= held.cpu, `the wall read ${held.wall.toFixed(1)} ms, less than the thread's own ${held.cpu.toFixed(1)}`);
});

// THE BOUND IS THE PAGE'S OWN WORK, NOT THE WALL (R3-12 merge-back): a turn
// in which the thread is held without working (here it sleeps 100 ms, as it
// would while the OS runs another process) reads that time on the wall and
// almost none of its own. Before, the wall was the bound, and the Mac's load
// failed the gate.
test('the_timer_counts_the_threads_own_time_not_the_wall', async () => {
  const held = await longestTask(async () => {
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 2 * LONG_TASK_MS);
  });
  assert.ok(held.wall >= 2 * LONG_TASK_MS - 1, `a ${2 * LONG_TASK_MS} ms sleep read ${held.wall.toFixed(1)} ms by the wall`);
  assert.ok(held.cpu < LONG_TASK_MS, `a sleep read ${held.cpu.toFixed(1)} ms of the thread's own`);
});

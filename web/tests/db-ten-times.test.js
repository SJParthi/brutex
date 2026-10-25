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

/** The longest gap between two turns of the event loop while `work` runs. @param {() => Promise<void>} work */
async function longestTask(work) {
  let longest = 0;
  let last = performance.now();
  let running = true;
  const beat = () => {
    const now = performance.now();
    longest = Math.max(longest, now - last);
    last = now;
    if (running) setImmediate(beat);
  };
  setImmediate(beat);
  await work();
  // The beat queued before this turn runs first, so the stretch that ends the
  // work is timed too: a work with no await of its own (the list rules) was
  // otherwise never timed at all and read 0 ms.
  await new Promise((done) => setImmediate(done));
  running = false;
  return longest;
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
    assert.ok(spot < LONG_TASK_MS, `the spot view held the loop ${spot.toFixed(1)} ms`);
    assert.ok(spotBytes < 400_000, `the spot view moved ${spotBytes} bytes`);
    assert.equal(store.rows.length, 2 * 96 * 3, 'its rows, both spot segments');
    // THE OPTIONS VIEW WITH NO EXPIRY: 4,040 expiries offered from the listing.
    const options = await longestTask(() => open({ underlying: server.symbols[0], expiry: null, segments: ['options'] }));
    assert.ok(options < LONG_TASK_MS, `the options view held the loop ${options.toFixed(1)} ms`);
    assert.equal(store.scope.expiries.length, EXPIRIES);
    // AN EXPIRY CHOSEN: its chain, beside the listing.
    const chain = await longestTask(() => open({ underlying: server.symbols[0], expiry: store.scope.expiries[7].expiry, segments: ['options'] }));
    assert.ok(chain < LONG_TASK_MS, `the chain held the loop ${chain.toFixed(1)} ms`);
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
    assert.ok(lists < LONG_TASK_MS, `the page's list rules held the loop ${lists.toFixed(1)} ms`);
    // The run's numbers, for the report's n = 20 table (a TAP diagnostic line).
    t.diagnostic(`ten-times spot_ms=${spot.toFixed(2)} options_ms=${options.toFixed(2)} chain_ms=${chain.toFixed(2)} lists_ms=${lists.toFixed(2)} spot_bytes=${spotBytes}`);
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
    const until = performance.now() + 2 * LONG_TASK_MS;
    while (performance.now() < until);
  });
  assert.ok(held >= 2 * LONG_TASK_MS, `a ${2 * LONG_TASK_MS} ms stretch read ${held.toFixed(1)} ms`);
});

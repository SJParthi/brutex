// ONE PAGE OF THE /db BAR GRID IS ONE CHECKED REQUEST (D-3326, WP-52).
//
// `readWindowPage` replaced a reader that asked each month FILE for its own
// slice at offsets taken from the census, which left bars past a stale census
// count unreachable (DB-G02, DB-G61), stacked CE and PE instead of merging
// them (DB-002, DB-G92), and cost every month's first bar its change (DB-011).
// These tests drive the real module against a fake transport; no row here is
// market data.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readWindowPage, readAllRows, SATURATED_TRIES, pause } from '../src/lib/database-pages.js';
import { windowQuery, seriesPlan } from '../src/lib/db/bars-plan.js';

const signal = () => new AbortController().signal;
/** @param {unknown[]} bars @param {number} [total] @param {Record<string, unknown>} [other] @param {number} [status] */
const answer = (bars, total = 10, other = {}, status = 200) =>
  Response.json({ bars, total, offset: 0, series_totals: [], faults: null, months_read: 1, months_missing: 0, scanned: false, ...other }, { status });

/** Invented census rows of one rung. @param {string} instrument @param {string} month @param {number} rows */
const census = (instrument, month, rows) => ({ instrument, month, timeframe: '1s', rows });

test('a CE and PE pair is one request naming both series, never one per file', async () => {
  const plan = seriesPlan([
    census('NSE-FNO-NIFTY-2019-03-28-1150000-CE', '2019-02', 10),
    census('NSE-FNO-NIFTY-2019-03-28-1150000-CE', '2019-03', 10),
    census('NSE-FNO-NIFTY-2019-03-28-1150000-PE', '2019-03', 10)
  ]);
  /** @type {string[]} */
  const urls = [];
  const body = await readWindowPage(windowQuery('gdfl', plan, { sort: 'ts', desc: true, offset: 50, limit: 50 }), 50, signal(), async (url, options) => {
    assert.ok(options?.signal);
    urls.push(String(url));
    return answer(Array.from({ length: 50 }, (_, i) => ({ t: 1000 - i, key: i % 2 ? plan.keys[0] : plan.keys[1] })), 172_350, { offset: 50 });
  });
  assert.equal(urls.length, 1, 'one request a page');
  const q = new URL(urls[0], 'http://local').searchParams;
  assert.deepEqual(q.getAll('key'), ['NSE-FNO-NIFTY-2019-03-28-1150000-CE', 'NSE-FNO-NIFTY-2019-03-28-1150000-PE']);
  assert.equal(q.get('from'), '2019-02');
  assert.equal(q.get('to'), '2019-03');
  assert.equal(q.get('offset'), '50');
  assert.equal(q.get('extremes'), '0', 'a page turn folds no extremes (DB-G83)');
  assert.equal(q.get('contract'), null);
  assert.equal(body.total, 172_350, 'the server’s total, every series summed');
  assert.equal(body.bars.length, 50);
});

test('a saturated scan pool is retried with a growing pause, and past the tries the server’s sentence is the failure', async () => {
  /** @type {number[]} */
  const waits = [];
  const wait = async (/** @type {number} */ ms) => { waits.push(ms); };
  let calls = 0;
  const body = await readWindowPage('feed=gdfl', 1, signal(), async () => {
    calls += 1;
    return calls < 3 ? answer([], 0, { error: 'a bar window scan not admitted (Saturated): at most 2 run at once' }, 429) : answer([{ t: 1 }], 1);
  }, wait);
  assert.equal(calls, 3);
  assert.equal(body.bars.length, 1);
  assert.equal(waits.length, 2);
  assert.ok(waits[1] >= 500 && waits[0] >= 250 && waits[0] < 500, `waits ${waits}`);
  calls = 0;
  await assert.rejects(
    readWindowPage('feed=gdfl', 1, signal(), async () => {
      calls += 1;
      return Response.json({ error: 'a bar window scan not admitted (Saturated): at most 2 run at once' }, { status: 429 });
    }, async () => {}),
    /Saturated/
  );
  assert.equal(calls, SATURATED_TRIES);
});

test('a refusal carries the server’s own sentence, and a malformed answer is a failure, not a page', async () => {
  await assert.rejects(
    readWindowPage('q', 50, signal(), async () => Response.json({ error: 'scan_budget_exceeded: ordering by Close reads all 43942544 kept rows' }, { status: 400 })),
    /scan_budget_exceeded/
  );
  for (const bad of [
    new Response('broken'),
    Response.json({ bars: [], total: -1, offset: 0, series_totals: [] }),
    Response.json({ bars: [{}, {}], total: 9, offset: 0, series_totals: [] }),
    Response.json({ bars: [], total: 9, offset: 0 }),
    Response.json({ bars: [], total: 9, offset: 0, series_totals: [], faults: null })
  ]) {
    await assert.rejects(readWindowPage('q', 1, signal(), async () => bad), Error);
  }
  // A PARTIAL PAGE WITH ITS FAULT NAMED IS A PAGE (206).
  const partial = await readWindowPage('q', 2, signal(), async () => answer([{ t: 1 }], 2, { faults: 'record 7 failed its checksum' }, 206));
  assert.equal(partial.faults, 'record 7 failed its checksum');
});

test('an aborted read stops at once, waiting or not', async () => {
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(readWindowPage('q', 1, controller.signal, async () => answer([])), /abort/i);
  const later = new AbortController();
  const waiting = pause(10_000, later.signal);
  later.abort();
  await assert.rejects(waiting, /abort/i);
});

test('a whole read is refused before any request past its budget, and otherwise reads ceil(total / page) slices', async () => {
  let calls = 0;
  await assert.rejects(
    readAllRows(() => 'q', 20_001, 20_000, 1000, signal(), async () => { calls += 1; return answer([]); }),
    /over_budget: the query keeps 20001 rows/
  );
  assert.equal(calls, 0, 'nothing is read past the budget (DB-B50)');
  /** @type {number[]} */
  const offsets = [];
  /** @type {number[]} */
  const seen = [];
  const got = await readAllRows(
    (offset, limit) => `offset=${offset}&limit=${limit}`,
    2_500, 20_000, 1000, signal(),
    async (url) => {
      const q = new URL(String(url), 'http://local').searchParams;
      const offset = Number(q.get('offset'));
      offsets.push(offset);
      const n = Math.min(1000, 2_500 - offset);
      return Response.json({ bars: Array.from({ length: n }, (_, i) => ({ t: offset + i })), total: 2_500, offset, series_totals: [], faults: null });
    },
    (read) => seen.push(read)
  );
  assert.deepEqual(offsets, [0, 1000, 2000]);
  assert.equal(got.bars.length, 2_500);
  assert.deepEqual(seen, [1000, 2000, 2500]);
  assert.equal(got.bars[2_499].t, 2_499);
});

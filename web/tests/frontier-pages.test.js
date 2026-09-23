import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fetchCompleteFrontier, MAX_FRONTIER_ROWS, FRONTIER_PAGE_ROWS } from '../src/lib/frontier-pages.js';
import { validateFrontierPayload } from '../src/lib/frontier-analytics.js';

const identity = 'ab'.repeat(32);
/** @param {number} total */
const rules = (total) => ({
  min_win_rate_bp: 5_000, min_rr_bp: 125, min_ret_over_dd_bp: 400,
  min_trades: 4, min_assurance_bp: 3_000, max_mae_ppm: 0, top: Math.max(1, total)
});
/** @param {number} rank */
const row = (rank) => ({
  rank, direction: 'long', mask_words: [String(rank), '0', '0', '0', '0', '0'],
  hits: 6, n: 5, mean_milli_paisa: 12_500, t_milli: 2_000, payoff_bp: 150,
  edge_wins: 4, priced: true, trades: 4, wins: 3, losses: 1, pessimistic: 200,
  worst_trade: -50, max_drawdown: 50, min_win: 150, win_rate_bp: 7_500,
  reward_to_risk_bp: 300, return_over_drawdown: 400, avg_win: 100, avg_loss: -100,
  gross_win: 300, gross_loss: -100,
  meets: {
    win_rate: true, reward_to_risk: true, return_over_drawdown: true,
    trades: true, assurance: true, all: true, stop_unchecked: true,
    protective_exits_unchecked: true
  }
});

/** @param {number} total @param {number} page @returns {{ok: boolean, status: number, body: any}} */
function responseFor(total, page) {
  const offset = page * FRONTIER_PAGE_ROWS;
  const end = Math.min(total, offset + FRONTIER_PAGE_ROWS);
  const complete = page === 0 && end === total;
  return {
    ok: true,
    status: complete ? 200 : 206,
    body: {
      identity, rows: Array.from({ length: end - offset }, (_, i) => row(offset + i + 1)),
      count: end - offset, total_count: total, admitted: end - offset, total_admitted: total,
      rules: total ? rules(total) : null, page, limit: FRONTIER_PAGE_ROWS,
      page_complete: true, complete, next_page: end < total ? page + 1 : null,
      refusal: complete ? null : `partial page only: page ${page} returns rows ${offset}..${end} of ${total}. Fetch every page and reconcile \`total_count\`; this response is not a complete frontier`
    }
  };
}

/**
 * @param {number} total
 * @param {(response: {ok: boolean, status: number, body: any}, page: number) => void} [mutate]
 */
function server(total, mutate = () => {}) {
  /** @type {number[]} */
  const calls = [];
  /** @param {string} url */
  const request = async (url) => {
    const query = new URL(url, 'http://localhost').searchParams;
    assert.equal(query.get('identity'), identity);
    assert.equal(query.get('limit'), String(FRONTIER_PAGE_ROWS));
    const page = Number(query.get('page'));
    assert.equal(page, calls.length, 'pages must be requested once in ascending order');
    calls.push(page);
    const response = responseFor(total, page);
    mutate(response, page);
    return { ...response, json: async () => response.body };
  };
  return { request, calls };
}

test('257 and 4096 combinations cross every page and retain exact analytics and admissions', async () => {
  for (const total of [257, MAX_FRONTIER_ROWS]) {
    const source = server(total);
    const body = await fetchCompleteFrontier(identity, source.request);
    assert.equal(source.calls.length, Math.ceil(total / FRONTIER_PAGE_ROWS));
    assert.equal(body.count, total);
    assert.equal(body.total_count, total);
    assert.equal(body.admitted, total);
    assert.equal(body.total_admitted, total);
    assert.equal(body.rows.at(-1).rank, total);
    assert.equal(body.complete, true);
    assert.equal(body.refusal, null);
    assert.equal(body.next_page, null);
    const checked = validateFrontierPayload(body, identity);
    assert.equal(checked.ok, true, checked.why);
    assert.equal(checked.rows.length, total);
  }
});

test('empty, one-row, and exactly full-page results require one request', async () => {
  for (const total of [0, 1, FRONTIER_PAGE_ROWS]) {
    const source = server(total);
    const body = await fetchCompleteFrontier(identity, source.request);
    assert.deepEqual(source.calls, [0]);
    assert.equal(body.rows.length, total);
    assert.equal(validateFrontierPayload(body, identity).ok, true);
  }
  const missing = server(0, ({ body }) => { body.refusal = 'No committed result-set receipt exists.'; });
  const body = await fetchCompleteFrontier(identity, missing.request);
  const checked = validateFrontierPayload(body, identity);
  assert.equal(checked.ok, true);
  assert.equal(checked.empty, true);
  assert.equal(checked.why, 'No committed result-set receipt exists.');
});

test('failed HTTP and JSON responses and transport failures never publish a preceding page', async () => {
  let parsed = false;
  await assert.rejects(fetchCompleteFrontier(identity, async () => ({
    ok: false, status: 503, json: async () => { parsed = true; throw new Error('HTML body'); }
  })), /answered 503/);
  assert.equal(parsed, false, 'an HTTP failure must retain its status even for an HTML response');
  for (const failure of ['transport', 'JSON']) {
    const source = server(257);
    let calls = 0;
    await assert.rejects(fetchCompleteFrontier(identity, async (url) => {
      const response = await source.request(url);
      calls += 1;
      if (calls === 2) {
        if (failure === 'transport') throw new Error('transport failed');
        return { ...response, json: async () => { throw new Error('JSON failed'); } };
      }
      return response;
    }), new RegExp(`${failure} failed`));
    assert.equal(calls, 2);
  }
});

test('missing, invalid, repeated, skipped, and prematurely terminal cursors are refused at their first occurrence', async () => {
  for (const next of [undefined, '1', -1, 0, 2, null, NaN, Number.MAX_SAFE_INTEGER + 1]) {
    const source = server(257, ({ body }) => { body.next_page = next; });
    await assert.rejects(fetchCompleteFrontier(identity, source.request), /next_page/);
    assert.deepEqual(source.calls, [0]);
  }
  const loop = server(257, ({ body }, page) => { if (page === 1) body.next_page = 0; });
  await assert.rejects(fetchCompleteFrontier(identity, loop.request), /next_page/);
  assert.deepEqual(loop.calls, [0, 1]);
});

test('later pages cannot change identity, counts, rules, coordinates, completeness, or ranks', async () => {
  /** @type {((body: any) => void)[]} */
  const mutations = [
    (body) => { body.identity = 'cd'.repeat(32); },
    (body) => { body.total_count = 258; },
    (body) => { body.total_admitted = 256; },
    (body) => { body.rules.top = 258; },
    (body) => { body.rules = null; },
    (body) => { body.page = 0; },
    (body) => { body.limit = 128; },
    (body) => { body.page_complete = false; },
    (body) => { body.complete = true; },
    (body) => { body.rows[0].rank = 1; },
    (body) => { body.rows.pop(); },
    (body) => { body.count = 0; },
    (body) => { body.admitted = 0; }
  ];
  for (const mutate of mutations) {
    const source = server(257, ({ body }, page) => { if (page === 1) mutate(body); });
    await assert.rejects(fetchCompleteFrontier(identity, source.request), /refused/);
    assert.deepEqual(source.calls, [0, 1]);
  }
});

test('transport ceilings and canonical metadata refuse oversized or malformed success payloads', async () => {
  /** @type {((response: {ok: boolean, status: number, body: any}) => void)[]} */
  const mutations = [
    ({ body }) => { body.total_count = MAX_FRONTIER_ROWS + 1; },
    ({ body }) => { body.total_count = 1.5; },
    ({ body }) => { body.total_count = -1; },
    ({ body }) => { body.total_count = undefined; },
    ({ body }) => { body.total_admitted = Number.MAX_SAFE_INTEGER + 1; },
    ({ body }) => { body.total_admitted = 2; },
    ({ body }) => { body.rows = null; },
    ({ body }) => { body.rows[0] = null; },
    ({ body }) => { body.refusal = undefined; },
    (response) => { response.body = null; },
    (response) => { response.status = 201; },
    (response) => { response.status = 206; }
  ];
  for (const mutate of mutations) {
    const source = server(1, mutate);
    await assert.rejects(fetchCompleteFrontier(identity, source.request));
    assert.deepEqual(source.calls, [0]);
  }
  let requested = false;
  await assert.rejects(fetchCompleteFrontier('bad', async () => {
    requested = true;
    throw new Error('unexpected request');
  }), /canonical/);
  assert.equal(requested, false);
});

test('real refusals and altered pagination notices survive instead of being cleared', async () => {
  for (const refusal of ['seal verification failed', 'partial page only: different count']) {
    const source = server(257, ({ body }, page) => { if (page === 1) body.refusal = refusal; });
    await assert.rejects(fetchCompleteFrontier(identity, source.request), new RegExp(refusal));
    assert.deepEqual(source.calls, [0, 1]);
  }
  const mismatch = server(257, ({ body }) => { body.total_admitted = 256; });
  await assert.rejects(fetchCompleteFrontier(identity, mismatch.request), /committed totals/);
});

test('rule key order may differ while the recorded values stay identical', async () => {
  const source = server(257, ({ body }, page) => {
    if (page === 1) body.rules = Object.fromEntries(Object.entries(body.rules).reverse());
  });
  assert.equal((await fetchCompleteFrontier(identity, source.request)).rows.length, 257);
});

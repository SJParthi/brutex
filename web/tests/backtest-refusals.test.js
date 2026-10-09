// F2 (OBSV-22, D-3220): the backtest page's own readers dropped the server's
// stated reason on a non-2xx, each in its own way:
//
//   /trades.json, /engine/top.json  parsed the body BEFORE the status check, so
//                                    a plain-text refusal became a JSON parse
//                                    error, and a JSON one lost route and status
//   /backtest.json                  "answered 429" alone; an audit-layer 503
//                                    (not ledger-shaped) became "`runs` is not
//                                    an array"
//   /vocab.json                     "answered 404" alone
//   /bars/window.json (chart and    "answered 400" / "answered 400/200", while
//   buy-and-hold)                    the route sends `{"error":…}`
//   /store.json (rung switcher)     `if (!response.ok) return;`: the switcher
//                                    vanished and nothing said why
//
// Each reader here is the page's real function, extracted from the component
// and run over stubbed replies.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import * as refusal from '../src/lib/refusal.js';
import { createRequestGate } from '../src/lib/request-gate.js';
import { fetchWithBusyRetry } from '../saved-backtest/requests.js';
import { validateLedgerPayload } from '../src/lib/comparison.js';

const source = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source));
/** @param {string[]} names */
function functions(names) {
  return names.map((name) => {
    const found = ast.instance.content.body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === name);
    assert.ok(found, `backtest: actual ${name} function is required`);
    return source.slice(found.start, found.end);
  }).join('\n');
}
/** The refusal helpers by name, so the extracted code finds whichever it calls. */
const helpers = Object.keys(refusal);
const helperValues = Object.values(refusal);

const audit = { schema_version: 1, refusal: 'bounded request audit capacity is full; retry this exact request',
  code: 'invocation_audit_unavailable', handler_completed: false,
  why: 'The handler was not dispatched because its required audit start was unavailable.' };
const plain = (/** @type {number} */ status, /** @type {string} */ text) =>
  new Response(text, { status, headers: { 'content-type': 'text/plain; charset=utf-8' } });
function deferred() {
  /** @type {(v?:any)=>void} */ let resolve = () => {};
  const promise = new Promise((yes) => { resolve = yes; });
  return { promise, resolve };
}
/** A non-2xx whose body is read only when `release` is called. @param {number} status @param {string} text */
function slow(status, text) {
  const body = deferred();
  return { response: { ok: false, status, headers: new Headers(), text: async () => { await body.promise; return text; },
    json: async () => { await body.promise; return JSON.parse(text); } }, release: () => body.resolve() };
}
const flush = () => new Promise((done) => setImmediate(done));

test('a refused /trades.json page names the route, status and reason, including a plain-text one (F2)', async () => {
  /** @param {() => any} reply */
  const mount = (reply) => new Function('ask_', 'createRequestGate', ...helpers, `
    const tradeGate = createRequestGate();
    let openRun = { identity: 'ab' }, tradeList = { phase: 'idle', rows: [], periods: null, policy: null, direction: null, why: '' };
    const validateTradePayload = () => { throw new Error('a refusal never reaches validation'); };
    ${functions(['fetchTrades'])}
    return { fetchTrades, list: () => tradeList, close: () => { openRun = { identity: 'cd' }; } };
  `)(reply, createRequestGate, ...helperValues);
  const refused = mount(async () => Response.json({ policy: null, direction: null, trades: null, periods: null, count: 0,
    total_count: null, page_complete: false, complete: false, refusal: 'identity must be 64 lowercase hex digits' }, { status: 400 }));
  await refused.fetchTrades('ab');
  assert.equal(refused.list().phase, 'failed');
  assert.equal(refused.list().why, '/trades.json answered HTTP 400: identity must be 64 lowercase hex digits');
  const proxied = mount(async () => plain(502, 'upstream connect error'));
  await proxied.fetchTrades('ab');
  assert.equal(proxied.list().why, '/trades.json answered HTTP 502: upstream connect error', 'not a JSON parse error');
  const busy = mount(async () => Response.json(audit, { status: 429 }));
  await busy.fetchTrades('ab');
  assert.equal(busy.list().why, `/trades.json answered HTTP 429: ${audit.refusal} ${audit.why}`);
  const late = slow(400, '{"refusal":"late"}');
  const closed = mount(async () => late.response);
  const read = closed.fetchTrades('ab');
  await flush(); closed.close(); late.release(); await read;
  assert.equal(closed.list().phase, 'loading', 'a refusal read after its run closed is not published');
});

test('a refused /engine/top.json read names the route, status and reason before reading a report (F2)', async () => {
  /** @param {() => any} reply */
  const mount = (reply) => new Function('ask_', ...helpers, `
    let top = { phase: 'idle', report: '', why: '' };
    ${functions(['fetchTop'])}
    return { fetchTop, top: () => top };
  `)(reply, ...helperValues);
  const refused = mount(async () => Response.json({ report: null, refusal: 'no committed sweep for dhan NIFTY' }, { status: 503 }));
  await refused.fetchTop('dhan', 'NIFTY');
  assert.equal(refused.top().phase, 'failed');
  assert.match(refused.top().why, /^\/engine\/top\.json answered HTTP 503: no committed sweep for dhan NIFTY\. /);
  const proxied = mount(async () => plain(502, 'Bad Gateway'));
  await proxied.fetchTop('dhan', 'NIFTY');
  assert.match(proxied.top().why, /^\/engine\/top\.json answered HTTP 502: Bad Gateway\. /);
  const empty = mount(async () => Response.json({ report: null, refusal: 'nothing ranked yet' }));
  await empty.fetchTop('dhan', 'NIFTY');
  assert.equal(empty.top().why, 'nothing ranked yet', 'a 200 refusal is still the body refusal alone');
  const ready = mount(async () => Response.json({ report: 'ranked text', refusal: null }));
  await ready.fetchTop('dhan', 'NIFTY');
  assert.deepEqual(ready.top(), { phase: 'ready', report: 'ranked text', why: '' });
});

test('a refused /backtest.json read names the audit refusal, not a status or a schema complaint (F2)', async () => {
  /** @param {(url:string,options:any)=>Promise<any>} request */
  const mount = (request) => new Function('ask_', 'fetchWithBusyRetry', 'validateLedgerPayload', ...helpers, `
    let ledgerSeq = 0, ledgerAbort = null;
    let load = { phase: 'idle', body: null, why: '' };
    const LIMIT = 500;
    ${functions(['fetchLedger', 'cancelLedger'])}
    return { fetchLedger, cancelLedger, state: () => load };
  `)(request, (/** @type {string} */ url, /** @type {any} */ options, /** @type {any} */ transport) =>
    fetchWithBusyRetry(url, options, transport, { wait: async () => {} }), validateLedgerPayload, ...helperValues);
  const busy = mount(async () => Response.json(audit, { status: 429, headers: { 'Retry-After': '1' } }));
  await busy.fetchLedger();
  assert.equal(busy.state().phase, 'failed');
  assert.match(busy.state().why, new RegExp(`^/backtest\\.json answered HTTP 429: ${audit.refusal}.*not dispatched`));
  const unavailable = mount(async () => Response.json({ ...audit, handler_completed: true, why: 'The handler already ran.' }, { status: 503 }));
  await unavailable.fetchLedger();
  assert.equal(unavailable.state().phase, 'failed');
  assert.match(unavailable.state().why, /^\/backtest\.json answered HTTP 503: bounded request audit capacity is full; retry this exact request The handler already ran\./);
  const middleware = mount(async () => plain(431, 'REFUSED — request headers exceed the bound'));
  await middleware.fetchLedger();
  assert.match(middleware.state().why, /^\/backtest\.json answered HTTP 431: REFUSED — request headers exceed the bound/);
  const proxied503 = mount(async () => plain(503, 'upstream down'));
  await proxied503.fetchLedger();
  assert.equal(proxied503.state().why, '/backtest.json answered HTTP 503: upstream down. Nothing from it was ranked or opened.');
  const garbled = mount(async () => plain(200, '<html>not json</html>'));
  await garbled.fetchLedger();
  assert.equal(garbled.state().phase, 'failed');
  assert.match(garbled.state().why, /JSON/, 'a 200 that is not JSON still fails as a parse error');
  const missing = mount(async () => plain(404, 'Not Found'));
  await missing.fetchLedger();
  assert.match(missing.state().why, /^This server answered 404 for \/backtest\.json\./, 'the 404 sentence is unchanged');
  const late = slow(500, '{"error":"late"}');
  const replaced = mount(async () => late.response);
  const read = replaced.fetchLedger();
  await flush(); replaced.cancelLedger(); late.release(); await read;
  assert.equal(replaced.state().phase, 'loading', 'a refusal read after cancellation is not published');
});

test('a refused /vocab.json read names the route, status and reason (F2)', async () => {
  const app = new Function('ask_', ...helpers, `
    let vocab = { phase: 'loading', version: 0, bits: new Map(), why: '' }, vocabCommit = '';
    const validateVocabEnvelope = () => { throw new Error('a refusal never reaches validation'); };
    ${functions(['fetchVocab'])}
    return { fetchVocab, vocab: () => vocab };
  `)(async () => plain(431, 'REFUSED — request headers exceed the bound'), ...helperValues);
  await app.fetchVocab();
  assert.equal(app.vocab().phase, 'failed');
  assert.match(app.vocab().why, /^\/vocab\.json answered HTTP 431: REFUSED — request headers exceed the bound\. Masks below are shown as raw words/);
});

const run = { feed: 'dhan', underlying: 'NIFTY', timeframe: '5min', from_year: 2024, from_month: 1, to_year: 2024, to_month: 2 };

test('a refused bar window names the route, status and the error the route sent, for the chart and buy-and-hold (F2)', async () => {
  /** @param {(url:string)=>Promise<any>} reply */
  const mount = (reply) => new Function('ask_', ...helpers, `
    let seriesSeq = 0, benchSeq = 0, series = { phase: 'idle' }, bench = { phase: 'idle' };
    const catalogue = { ready: true, feed: 'dhan', rows: [] };
    const place = () => ({ exchange: 'NSE', segment: 'INDEX' });
    const runMonthCount = () => 2, MAX_WINDOW_LIMIT = 1000;
    const validateBarsWindow = () => { throw new Error('a refusal never reaches validation'); };
    ${functions(['loadSeries', 'loadBenchmark'])}
    return { loadSeries, loadBenchmark, series: () => series, bench: () => bench, bump: () => { seriesSeq += 1; benchSeq += 1; } };
  `)(reply, ...helperValues);
  const error = '`from` 2024-03 is after `to` 2024-02';
  const chart = mount(async () => Response.json({ error }, { status: 400 }));
  await chart.loadSeries(run, '5min');
  assert.equal(chart.series().phase, 'failed');
  assert.match(chart.series().why, new RegExp(`^/bars/window\\.json answered HTTP 400: ${error.replace(/[`]/g, '.')}\\. The run's own figures above are unaffected`));
  const half = mount(async (url) => url.includes('dir=asc') ? Response.json({ error }, { status: 400 }) : Response.json({ bars: [] }));
  await half.loadBenchmark(run);
  assert.equal(half.bench().phase, 'failed');
  assert.match(half.bench().why, new RegExp(`^The first span endpoint: /bars/window\\.json answered HTTP 400: ${error.replace(/[`]/g, '.')}\\.`));
  assert.doesNotMatch(half.bench().why, /last span endpoint/, 'the endpoint that answered is not blamed');
  const both = mount(async (url) => url.includes('dir=asc') ? plain(502, 'first down') : plain(503, 'last down'));
  await both.loadBenchmark(run);
  assert.match(both.bench().why, /first span endpoint: \/bars\/window\.json answered HTTP 502: first down; the last span endpoint: \/bars\/window\.json answered HTTP 503: last down/);
  const lastOnly = mount(async (url) => url.includes('dir=desc') ? Response.json({ error }, { status: 400 }) : Response.json({ bars: [] }));
  await lastOnly.loadBenchmark(run);
  assert.match(lastOnly.bench().why, /^The last span endpoint: \/bars\/window\.json answered HTTP 400: /);
  assert.doesNotMatch(lastOnly.bench().why, /first span endpoint/);
  const lateBench = slow(400, JSON.stringify({ error }));
  const staleBench = mount(async (url) => url.includes('dir=asc') ? lateBench.response : Response.json({ bars: [] }));
  const benchRead = staleBench.loadBenchmark(run);
  await flush(); staleBench.bump(); lateBench.release(); await benchRead;
  assert.equal(staleBench.bench().phase, 'loading', 'a benchmark refusal read after a newer request is not published');
  const late = slow(400, JSON.stringify({ error }));
  const stale = mount(async () => late.response);
  const read = stale.loadSeries(run, '5min');
  await flush(); stale.bump(); late.release(); await read;
  assert.equal(stale.series().phase, 'loading', 'a refusal read after a newer request is not published');
});

test('a refused /store.json census read hides the switcher and says why (F2)', async () => {
  /** @param {() => Promise<any>} reply */
  const mount = (reply) => new Function('ask_', ...helpers, `
    let rungsSeq = 0, storeRungs = [], rungsWhy = '';
    const sweptSymbolOf = (s) => s.split('-').at(-1), byRung = (a, b) => a.localeCompare(b);
    ${functions(['loadRungs'])}
    return { loadRungs, rungs: () => storeRungs, why: () => rungsWhy, bump: () => { rungsSeq += 1; } };
  `)(reply, ...helperValues);
  const unreadable = mount(async () => Response.json([], { status: 503,
    headers: { 'x-brutex-census-state': 'unreadable', 'x-brutex-census-note': 'dhan: UNREADABLE - manifest checksum differs' } }));
  await unreadable.loadRungs(run);
  assert.deepEqual(unreadable.rungs(), []);
  assert.equal(unreadable.why(), '/store.json answered HTTP 503: the store census is unreadable: dhan: UNREADABLE - manifest checksum differs');
  const unknownFeed = mount(async () => Response.json({ refused: 'this build reads no feed called dhan2', feed: 'dhan2' }, { status: 400 }));
  await unknownFeed.loadRungs(run);
  assert.equal(unknownFeed.why(), '/store.json answered HTTP 400: this build reads no feed called dhan2');
  const thrown = mount(async () => { throw new Error('No response from /store.json within 30 s'); });
  await thrown.loadRungs(run);
  assert.equal(thrown.why(), 'No response from /store.json within 30 s');
  const held = mount(async () => Response.json([{ instrument: 'NSE-INDEX-NIFTY', timeframe: '5min' }]));
  await held.loadRungs(run);
  assert.deepEqual(held.rungs(), [{ name: '5min', months: 1 }]);
  assert.equal(held.why(), '');
  const late = slow(503, '[]');
  const stale = mount(async () => late.response);
  const read = stale.loadRungs(run);
  await flush(); stale.bump(); late.release(); await read;
  assert.equal(stale.why(), '', 'a refusal read after a newer request is not published');
  assert.match(source, /\{:else if rungsWhy\}[\s\S]*?\{rungsWhy\}/, 'the reason is rendered where the switcher would be');
});

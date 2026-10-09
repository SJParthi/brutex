// F4 (OBSV-23, D-3221): every route can be refused before its handler runs, in
// PLAIN TEXT: `crates/api/src/server.rs` `request_bounds_refusal` answers 414
// (target too long), 431 (headers too large) and 400 (a repeated query field)
// with "REFUSED — … Nothing was read or run.", and the cross-site admission
// layer answers 403 with its reason, all `text/plain`. Readers that printed
// only "HTTP 431", or parsed every reply as JSON first, replaced that sentence
// with a status or a parse error; one (Autopilot's control) blamed the dev
// proxy for a refusal the API itself had made, and three (`/pull/run.json` on
// resume, `/masters/refresh`, `/folder.json`) dropped it entirely.
//
// Each reader here is the real one: a module's export, or the page's own
// function extracted from its component and run over stubbed replies.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { parse } from 'svelte/compiler';
import * as refusal from '../src/lib/refusal.js';
import { createFeedStartup } from '../src/lib/feed-startup.js';
import { createInspectionReader } from '../src/lib/runtime-inspection.js';
import { readDatabasePage } from '../src/lib/database-pages.js';

const BOUND = 'REFUSED — the request headers are 70000 bytes and this server reads at most 65536. Nothing was read or run.';
const plain = (/** @type {number} */ status, text = BOUND) =>
  new Response(`${text}\n`, { status, headers: { 'content-type': 'text/plain; charset=utf-8' } });
const helpers = Object.keys(refusal);
const helperValues = Object.values(refusal);
const flush = () => new Promise((done) => setImmediate(done));

/** @param {string} route @param {string[]} names */
function functions(route, names) {
  const source = readFileSync(new URL(`../src/routes/${route}`, import.meta.url), 'utf8');
  const ast = /** @type {any} */ (parse(source));
  return names.map((name) => {
    const found = ast.instance.content.body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === name);
    assert.ok(found, `${route}: actual ${name} function is required`);
    return source.slice(found.start, found.end);
  }).join('\n');
}

test('the feed list and the inspection mode name a plain-text refusal (F4)', async () => {
  const state = { all: [], active: null, error: /** @type {string|null} */ (null) };
  const startup = createFeedStartup(state, /** @type {any} */ (async () => plain(431)), () => /** @type {any} */ ({ getItem: () => null, setItem: () => {} }));
  await startup.load(true);
  assert.equal(state.error, `Error: /feeds.json answered HTTP 431: ${BOUND}`);

  /** @type {any[]} */ const published = [];
  const reader = createInspectionReader(async () => plain(431), (s) => published.push(s));
  const settled = await reader.load();
  assert.equal(settled.phase, 'failed');
  assert.equal(settled.canSweep, false);
  assert.equal(settled.why, `Execution mode could not be verified: /inspection.json answered HTTP 431: ${BOUND}. Sweep controls remain disabled.`);
  const legacy = await createInspectionReader(async () => plain(404, 'Not Found'), () => {}).load();
  assert.equal(legacy.phase, 'legacy', 'a 404 is still a legacy server');
});

test('the database page and the terminal name a refused bar window, plain or JSON (F4)', async (t) => {
  const file = { key: 'k', instrument: 'NSE-INDEX-NIFTY', timeframe: '1min', month: '2024-01', rows: 10 };
  const signal = new AbortController().signal;
  const [plainPage] = await readDatabasePage('dhan', [file], 0, 5, 0, false, signal, /** @type {any} */ (async () => plain(414)));
  assert.equal(plainPage.error, `/bars/window.json answered HTTP 414: ${BOUND}`);
  const [jsonPage] = await readDatabasePage('dhan', [file], 0, 5, 0, false, signal,
    /** @type {any} */ (async () => Response.json({ error: 'path segment symbol is 31 bytes, max 24' }, { status: 400 })));
  assert.equal(jsonPage.error, '/bars/window.json answered HTTP 400: path segment symbol is 31 bytes, max 24');

  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  globalThis.fetch = /** @type {any} */ (async () => plain(414));
  const source = await readFile(new URL('../src/lib/terminal.svelte.js', import.meta.url), 'utf8');
  // Only the module's own import lines are rewritten: its prose strings also
  // contain "from '…'".
  const code = source.replace(/^(import [^;]*? from )(["'])([^"']+)\2/gm, (_m, head, q, path) =>
    `${head}${q}${path.startsWith('$lib/') ? new URL(`../src/lib/${path.slice(5)}`, import.meta.url).href : import.meta.resolve(path)}${q}`);
  const terminal = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`);
  const quoted = await terminal.quote('dhan', 'NSE-INDEX-NIFTY', '1min', '2024-01');
  assert.equal(quoted.close, null);
  assert.equal(quoted.why, `/bars/window.json answered HTTP 414: ${BOUND}`);
  const month = await terminal.monthBars('dhan', 'NSE-INDEX-NIFTY', '1min', '2024-02');
  assert.deepEqual(month.bars, []);
  assert.equal(month.why, `/bars/window.json answered HTTP 414: ${BOUND}`);
  globalThis.fetch = /** @type {any} */ (async () => Response.json({ error: 'no such month' }, { status: 400 }));
  const named = await terminal.quote('dhan', 'NSE-INDEX-NIFTY', '1min', '2024-03');
  assert.equal(named.why, '/bars/window.json answered HTTP 400: no such month');
});

test('the console probe and the Autopilot status read name a plain-text refusal (F4)', async () => {
  const probe = new Function('ask', ...helpers, `
    let api = { state: 'checking', ms: 0, why: null, at: 0 };
    const performance = { now: () => 0 };
    ${functions('+layout.svelte', ['probe'])}
    return { probe, api: () => api };
  `)(async () => plain(431), ...helperValues);
  await probe.probe({ current: () => true, signal: new AbortController().signal });
  assert.equal(probe.api().state, 'down');
  assert.equal(probe.api().why, `/feeds.json answered HTTP 431: ${BOUND}`);

  /** @type {any[]} */ const links = [];
  const tick = new Function('ask', ...helpers, `
    let ap = null;
    const performance = { now: () => 0 };
    const readState = (value) => ({ ok: true, value }), adopt = () => {}, setLink = (link) => links.push(link);
    const links = [];
    ${functions('autopilot/+page.svelte', ['tick'])}
    return { tick, links };
  `)(async () => plain(431), ...helperValues);
  await tick.tick({ current: () => true, signal: new AbortController().signal });
  links.push(...tick.links);
  assert.equal(links.at(-1).kind, 'broken');
  assert.equal(links.at(-1).why, `GET /autopilot.json answered HTTP 431: ${BOUND}`);
});

test('a plain-text refusal of the Autopilot control is the API refusing, not a missing proxy route (F4)', async () => {
  const send = (/** @type {() => Promise<any>} */ reply) => new Function('ask', ...helpers, `
    const CONTROL = '/autopilot/control';
    let control = { busy: false }, receipt = null, code = 0, link = { ms: 0 }, poller = null;
    const notes = [], note = (line) => notes.push(line), str = (v) => typeof v === 'string' && v.trim() !== '' ? v : null;
    const classify = () => 'accepted', readState = () => ({ ok: false }), adopt = () => {}, refresh = () => {};
    ${functions('autopilot/+page.svelte', ['send'])}
    return { send, receipt: () => receipt, notes };
  `)(reply, ...helperValues);
  const refused = send(async () => plain(403, 'cross-site request refused: the Origin header names another site'));
  await refused.send('pause');
  assert.equal(refused.receipt()?.tone, 'unknown');
  assert.equal(refused.receipt()?.why, 'POST /autopilot/control answered HTTP 403: cross-site request refused: the Origin header names another site');
  assert.doesNotMatch(refused.receipt()?.why, /proxy list/);
  const html = send(async () => new Response('<!doctype html><title>dev</title>', { status: 200, headers: { 'content-type': 'text/html' } }));
  await html.send('pause');
  assert.match(html.receipt()?.why, /answered text\/html, not JSON — the API is not behind this route/, 'a 2xx HTML page is still the dev-proxy fallback');
});

/** The ingest page's run readers, with the state they write. @param {(url:string, options?:any)=>Promise<any>} request */
function ingest(request) {
  return new Function('request', ...helpers, `
    let pollError = null, runState = null, phase = 'idle', startedAt = 0, lastGrowthAt = 0, finishedAt = 0;
    let baseline = null, live = null, seenRead = 0, sent = null, passSummary = null, netError = null, releaseWatch = () => {};
    const store = { reads: 0 }, POLL_MS = 2000, snapshot = async () => ({}), watchStore = () => () => {}, watchRun = async () => {};
    const AMBIGUOUS = /measured column layouts/i, FOLDER_SEGMENTS = ['INDEX', 'EQUITY'];
    ${functions('ingest/+page.svelte', ['resumeRunCurrent', 'pollRunCurrent', 'readFolder'])}
    return { resumeRunCurrent, pollRunCurrent, readFolder, state: () => ({ pollError, runState, phase }) };
  `)(request, ...helperValues);
}

test('the ingest run readers name a refused status read, including at resume (F4)', async () => {
  const ticket = { current: () => true, signal: new AbortController().signal };
  const resumed = ingest(async () => plain(431));
  await resumed.resumeRunCurrent(ticket);
  assert.equal(resumed.state().phase, 'idle', 'a status it could not read is not a run it claims');
  assert.equal(resumed.state().pollError, `Whether a pull run is already going could not be read: /pull/run.json answered HTTP 431: ${BOUND}. Pull stays offered; the server refuses a second run by name.`);
  const thrown = ingest(async () => { throw new Error('No response from /pull/run.json within 15 s'); });
  await thrown.resumeRunCurrent(ticket);
  assert.match(thrown.state().pollError ?? '', /could not be read: No response from \/pull\/run\.json within 15 s\./);
  const idle = ingest(async () => Response.json({ running: false, feeds: [] }));
  await idle.resumeRunCurrent(ticket);
  assert.equal(idle.state().pollError, null, 'a status that was read and is idle says nothing');
  let current = true;
  const late = ingest(async () => { current = false; return plain(431); });
  await late.resumeRunCurrent({ current: () => current, signal: new AbortController().signal });
  assert.equal(late.state().pollError, null, 'a replaced resume read publishes nothing');

  const polled = ingest(async () => plain(431));
  await polled.pollRunCurrent(ticket);
  assert.equal(polled.state().pollError, `Run status is unknown; this page could not read it: Error: /pull/run.json answered HTTP 431: ${BOUND}. Retrying every 2s while this page is visible.`);
});

test('a refused /folder.json read keeps its reason, plain text or JSON (F4)', async () => {
  const plainFolder = await ingest(async () => plain(414)).readFolder('folderfeed');
  assert.equal(plainFolder.ok, false);
  assert.equal(plainFolder.why, `/folder.json answered HTTP 414: ${BOUND}`);
  assert.equal(plainFolder.data, null);
  const json = await ingest(async () => Response.json({ feed: 'x', refused: 'this build reads no feed called that.' }, { status: 400 })).readFolder('x');
  assert.equal(json.why, '/folder.json answered HTTP 400: this build reads no feed called that.');
  assert.equal(json.data.refused, 'this build reads no feed called that.', 'the body is still carried for its path');
  const read = await ingest(async () => Response.json({ path: '/data', reach: { state: 'blank', files: 0, rows: 0 } })).readFolder('x');
  assert.equal(read.ok, true);
  assert.equal(read.why, null);
  /** @type {string[]} */ const asked = [];
  const ambiguous = await ingest(async (url) => {
    asked.push(url);
    return url.includes('segment=') ? plain(414) : Response.json({ refused: 'two measured column layouts; name a segment' }, { status: 409 });
  }).readFolder('x');
  assert.equal(asked.length, 3, 'the ambiguity is still retried once per segment');
  assert.match(ambiguous.why, /^\/folder\.json answered HTTP 409: two measured column layouts/, 'the ambiguity refusal is what is returned');
  await assert.rejects(ingest(async () => new Response('<html>', { status: 200 })).readFolder('x'), SyntaxError, 'a 200 that is not JSON still fails loudly');
  const source = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
  assert.match(source, /\.then\(\(\{ ok, data, segment, why \}\) =>[\s\S]*?state: 'halted', body: data, why, segment/);
});

test('the markets and database month reads name a plain-text refusal (F4)', async () => {
  const markets = new Function('ask', ...helpers, `
    ${functions('markets/+page.svelte', ['readMonth'])}
    return { readMonth };
  `)(async () => plain(414), ...helperValues);
  await assert.rejects(markets.readMonth({ exchange: 'NSE', segment: 'INDEX', symbol: 'NIFTY' }, '2024-01', 'dhan', '1min', undefined),
    { message: `2024-01: /bars.json answered HTTP 414: ${BOUND}` });
  const named = new Function('ask', ...helpers, `
    ${functions('markets/+page.svelte', ['readMonth'])}
    return { readMonth };
  `)(async () => Response.json({ error: 'dhan/NSE/INDEX/NIFTY/1min/2024-01.bin does not exist, opening it' }, { status: 400 }), ...helperValues);
  await assert.rejects(named.readMonth({ exchange: 'NSE', segment: 'INDEX', symbol: 'NIFTY' }, '2024-01', 'dhan', '1min', undefined),
    { message: '2024-01: /bars.json answered HTTP 400: dhan/NSE/INDEX/NIFTY/1min/2024-01.bin does not exist, opening it' });

  const db = (/** @type {() => Promise<any>} */ reply) => new Function('ask', 'pooled', ...helpers, `
    const IN_FLIGHT = 2, fromDay = '', toDay = '', barSortKey = 'ts', barDesc = false;
    ${functions('db/+page.svelte', ['fetchBarFile', 'readWindow'])}
    return { fetchBarFile, readWindow };
  `)(reply, async (/** @type {any[]} */ items, /** @type {number} */ _n, /** @type {(x:any)=>any} */ work) => Promise.all(items.map(work)), ...helperValues);
  const row = { key: 'k', instrument: 'NSE-INDEX-NIFTY', timeframe: '1min', month: '2024-01' };
  const open = new AbortController().signal;
  const file = await db(async () => plain(431)).fetchBarFile('dhan', row, open);
  assert.equal(file.error, `/bars.json answered HTTP 431: ${BOUND}`);
  const windowed = await db(async () => plain(431)).readWindow('dhan', [row], 0, 10, open);
  assert.equal(windowed[0].error, `/bars/window.json answered HTTP 431: ${BOUND}`);
  const partial = await db(async () => Response.json({ bars: [{ t: 1 }], faults: 'record 3 would not read' }, { status: 206 })).fetchBarFile('dhan', row, open);
  assert.equal(partial.error, null, 'a 206 partial read is still read, with its faults');
  assert.equal(partial.faults, 'record 3 would not read');
});

test('a refused masters refresh names the refusal and does not claim the refresh is still running (F4)', async () => {
  const refresh = (/** @type {() => Promise<any>} */ reply) => new Function('ask', ...helpers, `
    let masters = { phase: 'idle', rows: [], why: '', universe: '' }, reread = 0;
    const readMasters = async () => { reread += 1; }, fetchJoin = async () => {}, feeds = { active: 'dhan' };
    ${functions('mapping/+page.svelte', ['refreshMasters'])}
    return { refreshMasters, masters: () => masters, reread: () => reread };
  `)(reply, ...helperValues);
  const bounded = refresh(async () => plain(431));
  await bounded.refreshMasters();
  assert.equal(bounded.masters().phase, 'done');
  assert.equal(bounded.masters().why, `/masters/refresh answered HTTP 431: ${BOUND}`);
  assert.doesNotMatch(bounded.masters().why, /keeps running on the server/);
  assert.equal(bounded.reread(), 1, 'what is on disk is re-read after a refusal too');
  const stated = refresh(async () => Response.json({ landed: [], refusal: 'neither BRUTEX_MASTERS nor HOME is set' }, { status: 503 }));
  await stated.refreshMasters();
  assert.equal(stated.masters().why, 'neither BRUTEX_MASTERS nor HOME is set', 'a JSON refusal is read as before');
  const timedOut = refresh(async () => { throw new Error('No response from /masters/refresh within 90 s'); });
  await timedOut.refreshMasters();
  assert.match(timedOut.masters().why, /^No response from \/masters\/refresh within 90 s The refresh keeps running on the server/);
  await flush();
});

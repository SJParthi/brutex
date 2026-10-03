// OPENING /masters MUST NOT SPEND A VENDOR REQUEST, AND THIS RUNS THE PAGE TO SEE.
//
// Gate W6 checked `web/masters.js` by text: a `refresh();` or `verify();` at
// column zero was refused, and nothing else was. An indented call, a call with
// no semicolon, `setTimeout(refresh)`, a `load` or `DOMContentLoaded` listener
// and an `onload` assignment all spend the request the operator's standing
// rule forbids, and every one of them passed (LATE gates-and-ci #11, D-1118).
//
// So this executes the script in a `node:vm` context against a stub document,
// plays the page's load (every load-time event, every timer, every settled
// promise) and records every `fetch`. Opening the page may ask for
// `/masters/status.json`, which stats four files and opens no socket, and for
// nothing else. No server is started and no route is called.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const SOURCE = readFileSync(new URL('../masters.js', import.meta.url), 'utf8');
const LOAD_EVENTS = ['readystatechange', 'DOMContentLoaded', 'load', 'pageshow'];

/** @typedef {(event: any) => any} Listener */
/** @typedef {Record<string, Listener[]>} Listeners */

/** @param {string} id */
function element(id) {
  /** @type {Listeners} */
  const listeners = {};
  return {
    id,
    listeners,
    textContent: '',
    innerHTML: '',
    className: '',
    disabled: false,
    /** @param {string} type @param {Listener} fn */
    addEventListener(type, fn) {
      (listeners[type] ||= []).push(fn);
    },
    appendChild() {},
  };
}

/** Let every promise the page started settle, including chains of them. */
async function settle() {
  for (let round = 0; round < 20; round += 1) {
    await new Promise((resolve) => setImmediate(resolve));
  }
}

/**
 * Run `source` as the page would, then play its load. Returns every request
 * the page made and the elements it bound, so a test can press them.
 */
/**
 * @param {string} source
 * @param {Record<string, {status: number, body: any}>} [answers] what each URL answers
 */
async function open(source, answers = {}) {
 * @param {(url: string) => unknown} [respond] the JSON body each request reads
 */
async function open(source, respond = () => ({})) {
  /** @type {string[]} */
  const requests = [];
  /** @type {Map<string, ReturnType<typeof element>>} */
  const elements = new Map();
  /** @type {ReturnType<typeof element>[]} */
  const created = [];
  /** @param {string} id */
  const byId = (id) => {
    if (!elements.has(id)) elements.set(id, element(id));
    return elements.get(id);
  };
  /** @type {Listeners} */
  const documentListeners = {};
  /** @type {Listeners} */
  const windowListeners = {};
  /** @type {Function[]} */
  const timers = [];
  /** @param {unknown} fn */
  const schedule = (fn) => {
    if (typeof fn === 'function') timers.push(fn);
    return timers.length;
  };
  const document = {
    readyState: 'loading',
    getElementById: byId,
    querySelector: (/** @type {string} */ selector) => byId(`?${selector}`),
    querySelectorAll: () => [],
    createElement: () => {
      const made = element('');
      created.push(made);
      return made;
    },
    /** @param {string} type @param {Listener} fn */
    addEventListener(type, fn) {
      (documentListeners[type] ||= []).push(fn);
    },
  };
  const context = vm.createContext({
    document,
    console,
    URL,
    Date,
    fetch: async (/** @type {unknown} */ url) => {
      requests.push(String(url));
      const answer = answers[String(url)];
      const status = answer ? answer.status : 200;
      return { ok: status >= 200 && status < 300, status, json: async () => (answer ? answer.body : {}) };
      const body = respond(String(url));
      return { ok: true, status: 200, json: async () => body };
    },
    setTimeout: schedule,
    setInterval: schedule,
    requestAnimationFrame: schedule,
    queueMicrotask: (/** @type {unknown} */ fn) => schedule(fn),
    /** @param {string} type @param {Listener} fn */
    addEventListener(type, fn) {
      (windowListeners[type] ||= []).push(fn);
    },
  });
  context.window = context;
  context.self = context;
  context.globalThis = context;

  vm.runInContext(source, context, { filename: 'masters.js' });
  await settle();

  document.readyState = 'complete';
  for (const type of LOAD_EVENTS) {
    const event = { type, target: document };
    for (const fn of [...(documentListeners[type] || []), ...(windowListeners[type] || [])]) {
      await fn(event);
    }
    for (const holder of /** @type {Record<string, unknown>[]} */ ([document, context])) {
      const handler = holder[`on${type.toLowerCase()}`];
      if (typeof handler === 'function') await handler(event);
    }
  }
  // A timer may schedule another; a page that keeps rescheduling forever is
  // refused rather than followed.
  for (let at = 0; at < timers.length; at += 1) {
    assert.ok(at < 100, 'the page keeps scheduling work on load');
    await timers[at]();
  }
  await settle();
  return { requests, elements, created };
}

/** @param {Map<string, ReturnType<typeof element>>} elements @param {string} id */
async function press(elements, id) {
  const target = elements.get(id);
  assert.ok(target, `#${id} was looked up by the script`);
  const handlers = target.listeners.click || [];
  assert.ok(handlers.length > 0, `#${id} has a click handler`);
  for (const fn of handlers) await fn({ type: 'click', target });
  await settle();
}

test('opening the page asks only for the socket-free status', async () => {
  const { requests } = await open(SOURCE);
  assert.deepEqual(requests, ['/masters/status.json']);
});

test('refresh and verify are reached by a press, and each spends its request', async () => {
  const { requests, elements } = await open(SOURCE);
  const before = requests.length;
  await press(elements, 'go');
  assert.ok(requests.slice(before).includes('/masters/refresh'), requests.join(' '));
  const afterRefresh = requests.length;
  await press(elements, 'verify');
  assert.ok(
    requests.slice(afterRefresh).some((url) => url.startsWith('/indexmap.json?feed=')),
    requests.join(' '),
  );
});

test('every way to call refresh or verify on load is caught', async () => {
  // The shapes the text check let through, each appended to the real script.
  const spellings = [
    '  refresh();',
    'verify()',
    'void refresh();',
    ';(async () => { await verify(); })();',
    'setTimeout(refresh, 0);',
    'requestAnimationFrame(() => verify());',
    "addEventListener('load', refresh);",
    "window.addEventListener('load', () => verify());",
    "document.addEventListener('DOMContentLoaded', refresh);",
    'onload = verify;',
    'window.onload = () => refresh();',
    'Promise.resolve().then(verify);',
  ];
  for (const spelling of spellings) {
    const { requests } = await open(`${SOURCE}\n${spelling}\n`);
    const spent = requests.filter((url) => url !== '/masters/status.json');
    assert.ok(spent.length > 0, `not caught: ${spelling}`);
  }
});

// audit-20261003 webcontract-3, D-1585: the refresh's own answer is read. A
// 502 whose re-parse was refused must not be reported as "All four are on
// disk." and nothing else; a 200 that re-parsed must not ask for a restart.
test('a refused re-parse is said, and a successful one is not called stale', async () => {
  const refused = await open(SOURCE, {
    '/masters/refresh': {
      status: 502,
      body: { landed: [], missing: [], reloaded: false, universe: 'nse-indices.csv would not parse', restart_required: false },
    },
  });
  await press(refused.elements, 'go');
  const said = refused.elements.get('say')?.textContent || '';
  assert.match(said, /did NOT re-parse/, said);
  assert.match(said, /nse-indices\.csv would not parse/, said);

  const reloaded = await open(SOURCE, {
    '/masters/refresh': {
      status: 200,
      body: { landed: [], missing: [], reloaded: true, universe: 'ok', restart_required: false },
    },
  });
  await press(reloaded.elements, 'go');
  const fine = reloaded.elements.get('say')?.textContent || '';
  assert.match(fine, /re-parsed them/, fine);
  assert.doesNotMatch(fine, /Restart|did NOT/, fine);
});

// audit-20261003 webcontract-2, D-1585: vendor and server text never becomes
// markup. A vendor body quoted in `detail`, a refusal and a symbol are shown as
// text: the `<img onerror>` they carry is escaped, not parsed.
test('vendor text reaching the page is escaped, never parsed as markup', async () => {
  const hostile = '<img src=x onerror=alert(1)>';
  const page = await open(SOURCE, {
    '/masters/refresh': {
      status: 502,
      body: {
        landed: [{ file: 'zerodha.csv', written: false, refusal: hostile,
          attempts: [{ number: 1, got: hostile, status: 403, waited_ms: 0, detail: hostile }], waited_ms: 0 }],
        missing: [], reloaded: false, universe: '',
      },
    },
    '/indexmap.json?feed=dhan': { status: 200, body: { error: hostile } },
    '/indexmap.json?feed=groww': { status: 200, body: { published: 1, listed: 1, resolved: 0, refused: 1, rows: [{ symbol: hostile, nse: null }] } },
  });
  await press(page.elements, 'go');
  await press(page.elements, 'verify');
  const markup = [
    page.elements.get('?tr[data-file="zerodha.csv"] .out')?.innerHTML || '',
    ...page.created.map((made) => made.innerHTML),
    page.elements.get('xverify')?.innerHTML || '',
  ].join('\n');
  assert.ok(markup.includes('&lt;img src=x onerror=alert(1)&gt;'), markup);
  assert.ok(!markup.includes('<img'), markup);
// MR-23, audit-20261003 testgaps-3 (D-1606). The row said the join "names the
// symbols it could not resolve" and cited a test that never existed; the two
// that did only found `id="xverify"` in the HTML. This presses Verify against
// a join that refuses two symbols and reads what the page rendered.
test('a press of verify names every symbol the exchange did not confirm', async () => {
  const join = {
    published: 4,
    listed: 3,
    resolved: 1,
    refused: 2,
    rows: [
      { symbol: 'NIFTY PVT BANK', nse: 'NIFTY PRIVATE BANK', basis: 'abbreviation' },
      { symbol: 'NIFTY100QLTY30', nse: null, why: 'ambiguous', candidates: 2 },
      { symbol: 'INDIA VIX', nse: null, why: 'absent' },
    ],
  };
  const { elements } = await open(SOURCE, (url) =>
    url.startsWith('/indexmap.json?feed=') ? join : {},
  );
  await press(elements, 'verify');
  const box = elements.get('xverify');
  assert.ok(box, '#xverify was looked up by the script');
  for (const feed of ['dhan', 'groww', 'zerodha']) {
    assert.ok(box.innerHTML.includes(`<b>${feed}</b>`), `${feed} has no row`);
  }
  assert.ok(box.innerHTML.includes('NIFTY100QLTY30, INDIA VIX'), box.innerHTML);
  assert.ok(!box.innerHTML.includes('NIFTY PVT BANK'), 'a resolved symbol is not unconfirmed');
});

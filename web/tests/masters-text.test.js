// EVERY STRING /masters SHOWS FROM THE SERVER IS TEXT, NEVER MARKUP.
//
// A master host's first bytes are quoted in a refusal, and the page assigned
// that refusal to `innerHTML`: a host answering `x,<img src=x onerror=…>` ran
// script in the operator's console (CE-25, D-1768). And a refresh whose files
// the server refused to re-parse answered 502 with `reloaded:false` while the
// page said "All four are on disk" (CE-26). This runs the page in a `node:vm`
// context against a stub document that records every `innerHTML` write and
// every text node, presses Refresh and Verify against hostile answers, and
// reads what the operator would see.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const SOURCE = readFileSync(new URL('../masters.js', import.meta.url), 'utf8');
const HOSTILE = 'x,<img src=x onerror=alert(1)>';

function harness(answers) {
  /** @type {string[]} */
  const markup = [];
  const make = () => {
    const node = {
      children: [],
      listeners: {},
      className: '',
      disabled: false,
      colSpan: 1,
      _text: '',
      get textContent() {
        return node._text + node.children.map((c) => c.textContent).join('');
      },
      set textContent(v) {
        node._text = String(v);
        node.children = [];
      },
      set innerHTML(v) {
        markup.push(String(v));
      },
      get innerHTML() {
        return '';
      },
      addEventListener(type, fn) {
        (node.listeners[type] ||= []).push(fn);
      },
      appendChild(child) {
        node.children.push(child);
        return child;
      },
      replaceChildren(...kids) {
        node._text = '';
        node.children = kids;
      },
    };
    return node;
  };
  const byId = new Map();
  const cells = new Map();
  const document = {
    getElementById: (id) => {
      if (!byId.has(id)) byId.set(id, make());
      return byId.get(id);
    },
    querySelector: (sel) => {
      if (!cells.has(sel)) cells.set(sel, make());
      return cells.get(sel);
    },
    querySelectorAll: () => [],
    createElement: () => make(),
  };
  const context = vm.createContext({
    document,
    Date,
    String,
    fetch: async (url) => {
      const key = Object.keys(answers).find((k) => String(url).startsWith(k));
      const [status, body] = answers[key] ?? [200, {}];
      return { ok: status < 300, status, json: async () => body };
    },
  });
  vm.runInContext(SOURCE, context, { filename: 'masters.js' });
  return { markup, byId, cells };
}

const settle = async () => {
  for (let i = 0; i < 20; i += 1) await new Promise((r) => setImmediate(r));
};

test('a host-quoted refusal is shown as text and never parsed as markup', async () => {
  const page = harness({
    '/masters/refresh': [
      502,
      {
        landed: [
          {
            file: 'a.csv',
            refusal: HOSTILE,
            attempts: [{ number: 1, got: 'refused', status: 200, detail: HOSTILE }],
          },
        ],
        missing: ['a.csv'],
        reloaded: false,
        universe: 'the new master was refused on re-parse',
      },
    ],
    '/masters/status.json': [200, { masters: [{ file: HOSTILE, present: true, bytes: 1 }] }],
    '/indexmap.json': [200, { error: HOSTILE }],
  });
  await settle();
  for (const fn of page.byId.get('go').listeners.click) await fn();
  for (const fn of page.byId.get('verify').listeners.click) await fn();
  await settle();

  assert.deepEqual(
    page.markup.filter((m) => m.includes('<')),
    [],
    'no server string reaches innerHTML'
  );
  const shown = [...page.cells.values()].map((c) => c.textContent).join(' ');
  assert.ok(shown.includes(HOSTILE), `the refusal is shown verbatim as text: ${shown}`);
  assert.ok(page.byId.get('ledger').textContent.includes(HOSTILE));
  assert.ok(page.byId.get('xverify').textContent.includes(HOSTILE));

  // CE-26: a refresh the server did not reload is not reported as success.
  const said = page.byId.get('say').textContent;
  assert.ok(!said.startsWith('All four are on disk'), said);
  assert.ok(said.includes('HTTP 502') && said.includes('refused on re-parse'), said);
});

test('a refresh that reloaded says so', async () => {
  const page = harness({
    '/masters/refresh': [200, { landed: [], missing: [], reloaded: true, universe: 'parsed 4' }],
  });
  await settle();
  for (const fn of page.byId.get('go').listeners.click) await fn();
  await settle();
  assert.ok(page.byId.get('say').textContent.startsWith('All four are on disk and the server reloaded them'));
});

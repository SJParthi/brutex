// W2 (OBSV-13, D-3212): the /ingest "measure what this feed is missing" press
// reads `/instruments.json` once per feed and printed only the status of a
// refusal. A 503 for an unreadable census carries its reason in
// `x-brutex-census-note`, and an unknown feed's 400 in its body's `refused`.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import * as refusal from '../src/lib/refusal.js';

const source = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source, { modern: true }));
/** @param {string} name */
function declared(name) {
  const found = ast.instance.content.body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === name);
  assert.ok(found, `ingest: actual ${name} function is required`);
  return source.slice(found.start, found.end);
}

/** The page's real `measureRoster`, over one stubbed reply per feed. @param {(wire:string)=>Response} reply */
function roster(reply) {
  const create = new Function('request', 'refusal', `
    const { headerRefusalFrom } = refusal;
    const feeds = { all: [{ wire: 'dhan', display: 'Dhan' }, { wire: 'groww', display: 'Groww' }] };
    const rosterKey = 'key';
    let roster = { at: 0, key: null, byKey: new Map(), error: null, busy: false };
    ${declared('measureRoster')}
    return { measureRoster, roster: () => roster };
  `);
  return create(async (/** @type {string} */ url) => reply(new URL(url, 'http://x').searchParams.get('feed') ?? ''), refusal);
}

test('a refused feed in the roster measurement names the server reason, not only the status (W2)', async () => {
  const app = roster((wire) => wire === 'dhan'
    ? Response.json([{ key: 'NSE:NIFTY', symbol: 'NIFTY' }])
    : Response.json([], { status: 503, headers: {
      'x-brutex-master-state': 'read', 'x-brutex-master-note': 'groww: master read; 812 instrument(s) in the merged universe',
      'x-brutex-census-state': 'unreadable', 'x-brutex-census-note': 'groww: UNREADABLE ? manifest checksum failed' } }));
  await app.measureRoster();
  assert.equal(app.roster().busy, false);
  assert.equal(app.roster().error, 'Error: Groww: /instruments.json answered HTTP 503: the store census is unreadable: groww: UNREADABLE ? manifest checksum failed');
  const unknown = roster(() => Response.json({ refused: 'this build reads no feed called groww', feed: 'groww' }, { status: 400 }));
  await unknown.measureRoster();
  assert.equal(unknown.roster().error, 'Error: Dhan: /instruments.json answered HTTP 400: this build reads no feed called groww');
});

test('every feed answering is measured as before', async () => {
  const app = roster((wire) => Response.json([{ key: `NSE:${wire}`, symbol: wire }]));
  await app.measureRoster();
  assert.equal(app.roster().error, null);
  assert.deepEqual([...app.roster().byKey.keys()].sort(), ['NSE:dhan', 'NSE:groww']);
});

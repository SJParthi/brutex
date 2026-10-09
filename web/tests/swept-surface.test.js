// W1 (OBSV-12, D-3211): `/universes.json` answers `"counted_from":"no master"`
// with every count `null` for a feed that publishes no instrument master
// (`crates/api/src/coverage.rs`, `target_json`). The backtest page read that
// `null` as `Number(null ?? 0)`, held `matched: 0`, and its cover sentence
// said "0 of N are swept" -- a measurement of a file the feed does not have.
// A refused read set the surface to null and dropped the server's reason.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { refusalFrom } from '../src/lib/refusal.js';
import { exact } from '../src/lib/money.js';
import { coverScope } from '../src/lib/charge-scope.js';

const source = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source));
const body = ast.instance.content.body;
/** @param {string} name */
function declared(name) {
  const found = body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === name);
  assert.ok(found, `backtest: actual ${name} function is required`);
  return source.slice(found.start, found.end);
}
/** The arrow inside `const name = $derived.by(arrow)`. @param {string} name */
function derivedBy(name) {
  const found = body.flatMap((/** @type {any} */ n) => n.type === 'VariableDeclaration' ? n.declarations : [])
    .find((/** @type {any} */ d) => d.id?.name === name);
  assert.ok(found, `backtest: actual ${name} declaration is required`);
  const arrow = found.init.arguments[0];
  return source.slice(arrow.start, arrow.end);
}

/** The page's real `loadSurface` and `coverNote`, over one stubbed reply. @param {() => Promise<Response>} reply */
function page(reply, admits = () => true) {
  const create = new Function('ask_', 'refusalFrom', 'exact', 'coverScope', 'admits', `
    let sweptSurface = null, activeFeed = 'truedata';
    const catalogGate = { admits };
    const catalog = { held: [{ leaf: 'NIFTY' }, { leaf: 'BANKNIFTY' }, { leaf: 'INDIAVIX' }] };
    const heldNow = { daily: [] }, sweepMonths = 12, pickedSymbols = new Set(['NIFTY']);
    ${declared('loadSurface')}
    const coverNote = () => (${derivedBy('coverNote')})();
    return { load: () => loadSurface('truedata', {}), surface: () => sweptSurface, note: () => coverNote() };
  `);
  return create(reply, refusalFrom, exact, coverScope, admits);
}

/** @param {any} target */
const universes = (target) => async () => Response.json({ targets: [target] });

test('a swept target counted from no master keeps its null count and never reads "0 of N" (W1)', async () => {
  const app = page(universes({ target: 'swept', label: 'Swept', note: 'the engine surface', universe: null,
    counted_from: 'no master', published: null, matched: null, lacks: null, unresolved: [] }));
  await app.load();
  assert.equal(app.surface()?.matched, null, 'null on the wire stays null in the page');
  assert.equal(app.surface()?.countedFrom, 'no master');
  assert.doesNotMatch(app.note(), /\b0 of 3 are swept/);
  assert.match(app.note(), /swept count not measured \(no master\)/);
});

test('a measured swept count is still shown exactly, including a real zero', async () => {
  for (const matched of [0, 2]) {
    const app = page(universes({ target: 'swept', label: 'Swept', note: '', counted_from: 'master', matched }));
    await app.load();
    assert.equal(app.surface()?.matched, matched);
    assert.match(app.note(), new RegExp(`${matched} of 3 are swept`));
  }
  for (const matched of [-1, 1.5, '2', Number.MAX_SAFE_INTEGER + 1, undefined]) {
    const app = page(universes({ target: 'swept', counted_from: 'master', matched }));
    await app.load();
    assert.equal(app.surface()?.matched, null, `${String(matched)} is not a count`);
    assert.match(app.note(), /swept count not measured \(master\)/);
  }
});

test('a refused /universes.json read names the route, status and server reason in the note', async () => {
  const app = page(async () => Response.json({ refused: 'this build reads no feed called nope', feed: 'nope' }, { status: 400 }));
  await app.load();
  assert.equal(app.surface()?.matched, null);
  assert.match(app.note(), /swept count not measured \(\/universes\.json answered HTTP 400: this build reads no feed called nope\)/);
  const thrown = page(async () => { throw new Error('No response from /universes.json within 15 s'); });
  await thrown.load();
  assert.match(thrown.note(), /swept count not measured \(No response from \/universes\.json within 15 s\)/);
});

test('a refusal whose feed was replaced while its reason was read is not published', async () => {
  let asked = 0;
  const app = page(async () => Response.json({ refused: 'gone' }, { status: 400 }), () => ++asked === 1);
  await app.load();
  assert.equal(asked, 2, 'the gate is asked again after the reason is read');
  assert.equal(app.surface(), null);
});

// F6 (OBSV-25, D-3223): a pull run this page picked up on load (`resumeRunCurrent`)
// keeps going when its "before" snapshot of the store cannot be read: the
// comment there says the card "cannot show a difference". The card showed one
// anyway. `share` and `unitsDone` fell back to 0 without a baseline, so the
// panel read "0% · 0/N", the foot said "nothing landed yet", and `unitsLeft`
// counted every unit as still to go, feeding an ETA. None of it was measured.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

const source = readFileSync(new URL('../src/routes/ingest/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source));
/** The expression inside `const name = $derived(expression)`. @param {string} name */
function derived(name) {
  const found = ast.instance.content.body.flatMap((/** @type {any} */ n) => n.type === 'VariableDeclaration' ? n.declarations : [])
    .find((/** @type {any} */ d) => d.id?.name === name);
  assert.ok(found, `ingest: actual ${name} declaration is required`);
  const arg = found.init.arguments[0];
  return source.slice(arg.start, arg.end);
}

/** @param {{units:number,rows:number}|null} baseline @param {{units:number,rows:number}|null} live @param {number|null} rate */
function progress(baseline, live, rate, expectedUnits = 10) {
  return new Function('baseline', 'live', 'rate', 'expectedUnits', `
    const unitsDone = ${derived('unitsDone')};
    const rowsGained = ${derived('rowsGained')};
    const unitsLeft = ${derived('unitsLeft')};
    const share = ${derived('share')};
    const etaSecs = ${derived('etaSecs')};
    const everGrew = ${derived('everGrew')};
    return { unitsDone, rowsGained, unitsLeft, share, etaSecs, everGrew };
  `)(baseline, live, rate, expectedUnits);
}

test('a run with no baseline has no measured progress, remaining count or ETA (F6)', () => {
  const unmeasured = progress(null, { units: 7, rows: 700 }, 0.5);
  assert.equal(unmeasured.share, null, 'no share without a baseline, not 0%');
  assert.equal(unmeasured.unitsLeft, null, 'no remaining count without a baseline');
  assert.equal(unmeasured.etaSecs, null, 'no ETA extrapolated from an unknown remaining count');
  const measured = progress({ units: 2, rows: 200 }, { units: 5, rows: 500 }, 0.5);
  assert.equal(measured.unitsDone, 3);
  assert.equal(measured.share, 0.5);
  assert.equal(measured.unitsLeft, 5);
  assert.equal(measured.etaSecs, 10);
  assert.equal(measured.everGrew, true);
  const finished = progress({ units: 2, rows: 200 }, { units: 12, rows: 900 }, 0.5);
  assert.equal(finished.share, 1);
  assert.equal(finished.unitsLeft, 0);
  assert.equal(finished.etaSecs, null);
  assert.equal(progress({ units: 0, rows: 0 }, null, null, 0).share, 0, 'an empty request is 0, as before');
});

test('the progress card says an unmeasured run is unmeasured instead of drawing 0% (F6)', () => {
  const card = source.slice(source.indexOf('{#if phase !== \'idle\' && expectedUnits > 0}'), source.indexOf('WHAT THE MULTI-PASS RUN DID'));
  assert.match(card, /\{#if share === null\}[\s\S]*?progress not measured[\s\S]*?\{:else\}[\s\S]*?class="prog-n mono"[\s\S]*?\{#if share === null\}[\s\S]*?Progress is not measured[\s\S]*?\{:else\}[\s\S]*?class="meter tall"[\s\S]*?nothing landed yet/);
  assert.doesNotMatch(card, /baseline\?\.units \?\? 0/, 'no rendered count stands a zero in for the baseline');
});

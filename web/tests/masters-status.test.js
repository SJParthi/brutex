// CE-82, D-1788: /mapping swallowed the /masters/status.json 503 refusal and
// kept a stale restart flag.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { mastersStatusRefused } from '../src/lib/masters-status.js';
import { refusalFrom } from '../src/lib/refusal.js';

const page = readFileSync(new URL('../src/routes/mapping/+page.svelte', import.meta.url), 'utf8');

test('the 503 refusal is read and the restart flag becomes unknown, not stale', async () => {
  const why = await refusalFrom('/masters/status.json', Response.json(
    { masters: [], refusal: 'neither BRUTEX_MASTERS nor HOME is set, so there is no masters folder' }, { status: 503 }));
  const state = mastersStatusRefused(why);
  assert.deepEqual(state.onDisk, []);
  assert.equal(state.restartNeeded, null);
  assert.equal(state.statusWhy, '/masters/status.json answered HTTP 503: neither BRUTEX_MASTERS nor HOME is set, so there is no masters folder');
  assert.equal(mastersStatusRefused('').statusWhy, '/masters/status.json failed and named no reason');
});

test('the page no longer returns silently on a refusal, and renders the reason', () => {
  assert.doesNotMatch(page, /if \(!response\.ok\) return;/);
  assert.match(page, /refusalFrom\('\/masters\/status\.json', response\)/);
  assert.equal(page.split('mastersStatusRefused(').length - 1, 2, 'the refusal and the failed read both reset the state');
  assert.match(page, /\{#if statusWhy\}\s*<div class="refusal" role="alert">/);
});

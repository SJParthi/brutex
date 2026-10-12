import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fetchAdmitted, JITTER_MS } from '../src/lib/admitted-fetch.js';

test('a 429 is asked again after the server delay plus jitter, and a 200 ends it (DB-B51)', async () => {
  /** @type {number[]} */ const waited = [];
  let calls = 0;
  const request = async () => ++calls === 1
    ? new Response('{"error":"busy"}', { status: 429, headers: { 'retry-after': '1' } })
    : Response.json({ bars: [] });
  const response = await fetchAdmitted('/bars/window.json?x=1', { signal: new AbortController().signal }, request,
    { random: () => 0.5, wait: async (delayMs) => { waited.push(delayMs); } });
  assert.equal(response.status, 200);
  assert.equal(calls, 2);
  assert.deepEqual(waited, [1000 + Math.floor(0.5 * JITTER_MS)]);
});

test('a refusal that persists is returned after three asks, and a plain 503 is never retried', async () => {
  let calls = 0;
  const busy = await fetchAdmitted('/bars.json', { signal: new AbortController().signal },
    async () => { calls++; return new Response('{}', { status: 429 }); },
    { random: () => 0, wait: async () => {} });
  assert.equal(busy.status, 429);
  assert.equal(calls, 3);
  calls = 0;
  const failed = await fetchAdmitted('/bars.json', { signal: new AbortController().signal },
    async () => { calls++; return new Response('{}', { status: 503 }); },
    { random: () => 0, wait: async () => {} });
  assert.equal(failed.status, 503);
  assert.equal(calls, 1);
});

test('every /db bar read asks again when the server is busy, and so does the census HEAD survey (DB-B51)', () => {
  // The grid reads bars only through `readWindowPage`, whose own loop asks a
  // 429 again after a jittered pause (WP-52); it holds no window read of its own.
  const grid = readFileSync(new URL('../src/routes/db/DbBarsGrid.svelte', import.meta.url), 'utf8');
  assert.match(grid, /import \{ readWindowPage, readAllRows(, [A-Za-z]+)* \} from '\$lib\/database-pages\.js'/);
  assert.doesNotMatch(grid, /ask\(`\/bars/, 'no bar read in the grid bypasses the retrying reader');
  const pages = readFileSync(new URL('../src/lib/database-pages.js', import.meta.url), 'utf8');
  assert.match(pages, /response\.status === 429 && attempt < SATURATED_TRIES/);
  assert.match(pages, /Math\.floor\(Math\.random\(\) \* 250\)/, 'the pause is jittered');
  // The coverage grid's previous-close read met 429 as "no previous close".
  const coverage = readFileSync(new URL('../src/routes/db/DbCoverageGrid.svelte', import.meta.url), 'utf8');
  assert.match(coverage, /fetchAdmitted\(`\/bars\/window\.json\?\$\{q\}`/);
  assert.doesNotMatch(coverage, /ask\(`\/bars/);
  const survey = readFileSync(new URL('../src/lib/feed-summary.js', import.meta.url), 'utf8');
  assert.match(survey, /fetchAdmitted\(/);
});

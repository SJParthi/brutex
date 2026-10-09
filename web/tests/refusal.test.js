// CE-83, D-1789: four readers printed only the HTTP status and dropped the
// api's named reason; the unknown-feed refusal's `refused` key was read by none.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { auditRefusal, headerRefusal, headerRefusalFrom, reasonOf, refusalFrom, refusalOf, refusalSentence } from '../src/lib/refusal.js';
import { readCalendar } from '../src/lib/calendar-owed.js';
import { censusFailure, createCensusLoader } from '../src/lib/store-census.js';

/** @param {string} path */
const source = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('all three refusal keys on the wire are read, in one order', () => {
  assert.equal(refusalOf({ error: 'calendar derivation not admitted; retry' }), 'calendar derivation not admitted; retry');
  assert.equal(refusalOf({ refused: 'this build reads no feed called that', feed: 'nope' }), 'this build reads no feed called that');
  assert.equal(refusalOf({ masters: [], refusal: 'neither BRUTEX_MASTERS nor HOME is set' }), 'neither BRUTEX_MASTERS nor HOME is set');
  assert.equal(refusalOf({ error: 'first', refused: 'second' }), 'first');
  for (const body of [null, undefined, [], 'text', 7, {}, { error: '' }, { error: '   ' }, { error: 3 }]) {
    assert.equal(refusalOf(body), null, JSON.stringify(body));
  }
});

test('a body is read for its reason and never throws', async () => {
  assert.equal(await reasonOf(Response.json({ refused: 'unknown feed' }, { status: 400 })), 'unknown feed');
  assert.equal(await reasonOf(new Response('upstream said no', { status: 502 })), 'upstream said no');
  assert.equal(await reasonOf(new Response('', { status: 503 })), null);
  assert.equal(await reasonOf(Response.json([], { status: 503 })), null);
  const long = 'x'.repeat(2000);
  assert.equal((await reasonOf(new Response(long, { status: 500 })))?.length, 501);
  const used = new Response('gone', { status: 500 });
  await used.text();
  assert.equal(await reasonOf(used), null);
});

test('the sentence carries the route, the status and the reason, or says none was named', async () => {
  assert.equal(refusalSentence('/logs.json', 503, 'logging is not installed'), '/logs.json answered HTTP 503: logging is not installed');
  assert.equal(refusalSentence('/logs.json', 503, null), '/logs.json answered HTTP 503 and named no reason');
  assert.equal(await refusalFrom('/indexmap.json', Response.json({ refused: 'no feed called x', feed: 'x' }, { status: 400 })),
    '/indexmap.json answered HTTP 400: no feed called x');
});

test('/calendar.json: a refusal is an empty calendar that names the server reason', async () => {
  const calendar = await readCalendar('dhan', async () => Response.json(
    { error: 'calendar derivation not admitted: too many in flight; retry' }, { status: 429 }), new AbortController().signal);
  assert.equal(calendar.owed.size, 0);
  assert.equal(calendar.why, '/calendar.json answered HTTP 429: calendar derivation not admitted: too many in flight; retry');
  const unknown = await readCalendar('nope', async () => Response.json({ refused: 'no such feed', feed: 'nope' }, { status: 400 }),
    new AbortController().signal);
  assert.match(unknown.why, /HTTP 400: no such feed/);
});

test('/store.json: a refused census names the body error, else the census note header', async () => {
  const busy = createCensusLoader(async () => Response.json({ error: 'census read unavailable: Saturated' }, { status: 429 }));
  const saturated = await busy.load('dhan', 0);
  assert.equal(saturated.ok, false);
  assert.equal(censusFailure(saturated), '/store.json answered HTTP 429: census read unavailable: Saturated');
  const damaged = createCensusLoader(async () => Response.json([], {
    status: 503, headers: { 'x-brutex-census-state': 'unreadable', 'x-brutex-census-note': 'dhan: UNREADABLE manifest checksum failed' }
  }));
  const unreadable = await damaged.load('dhan', 0);
  assert.equal(censusFailure(unreadable), '/store.json answered HTTP 503: dhan: UNREADABLE manifest checksum failed');
  const silent = createCensusLoader(async () => new Response('', { status: 500 }));
  assert.equal(censusFailure(await silent.load('dhan', 0)), '/store.json answered HTTP 500 and named no reason');
});

test('each of the four readers goes through the named-reason helper', () => {
  const store = source('../src/lib/store.svelte.js');
  assert.match(store, /Promise\.reject\(new Error\(censusFailure\(r\)\)\)/);
  assert.doesNotMatch(store, /`HTTP \$\{r\.status\} from \/store\.json`/);
  assert.match(source('../src/lib/calendar-owed.js'), /refusalFrom\('\/calendar\.json', response\)/);
  const backtest = source('../src/routes/backtest/+page.svelte');
  assert.match(backtest, /refusalFrom\('\/logs\.json', response\)/);
  assert.match(backtest, /censusFailure\(response\)/);
  const mapping = source('../src/routes/mapping/+page.svelte');
  assert.match(mapping, /refusalFrom\('\/indexmap\.json', response\)/);
  assert.doesNotMatch(mapping, /body\?\.error/);
});

test('a stamped refusal names the failed half only: an unreadable census, a master that did not read (W2)', async () => {
  const headers = (/** @type {Record<string, string>} */ h) => new Headers(h);
  assert.equal(headerRefusal(null), null);
  assert.equal(headerRefusal(headers({})), null);
  assert.equal(headerRefusal(headers({ 'x-brutex-master-state': 'read', 'x-brutex-master-note': 'dhan: master read; 5 instrument(s)',
    'x-brutex-census-state': 'held', 'x-brutex-census-note': 'dhan: 1 month(s), 2 row(s), generation 3' })), null);
  assert.equal(headerRefusal(headers({ 'x-brutex-census-state': 'absent', 'x-brutex-census-note': 'dhan: UNAVAILABLE ? no manifest' })), null);
  assert.equal(headerRefusal(headers({ 'x-brutex-census-state': 'unreadable', 'x-brutex-census-note': '  dhan: UNREADABLE  ' })),
    'the store census is unreadable: dhan: UNREADABLE');
  assert.equal(headerRefusal(headers({ 'x-brutex-census-state': 'unreadable', 'x-brutex-census-note': '   ' })),
    'the store census is unreadable and the response carried no census note');
  assert.equal(headerRefusal(headers({ 'x-brutex-master-state': 'UNAVAILABLE' })),
    'the instrument master is UNAVAILABLE and the response carried no master note');
  assert.equal(await headerRefusalFrom('/instruments.json', Response.json({ refused: 'no feed called x', feed: 'x' }, { status: 400 })),
    '/instruments.json answered HTTP 400: no feed called x');
  assert.equal(await headerRefusalFrom('/instruments.json', Response.json([], { status: 503, headers: { 'x-brutex-census-state': 'unreadable', 'x-brutex-census-note': 'n' } })),
    '/instruments.json answered HTTP 503: the store census is unreadable: n');
});

// W6 (OBSV-17, D-3216): `/backtest/run.json` answers an unreadable attempt with
// 503 (or a malformed `?attempt=` with 400) and the reason nested in
// `{"running":{"status":"unknown","why":…}}` (`crates/api/src/sweeprun.rs`
// `browser_attempt_unknown`, `unknown_status`). None of the three top-level keys
// carries it, so every reader printed the status alone.
test('an unknown running status is read for its why, and only an unknown one (W6)', async () => {
  const why = 'persistent invocation read unavailable: Saturated';
  assert.equal(refusalOf({ running: { where: 'browser', status: 'unknown', requested_attempt: '9', in_flight: false, why, refusal: null, report: null } }), why);
  assert.equal(refusalOf({ running: { where: 'cli', status: 'unknown', in_flight: false, why: `  ${why}  `, refusal: null } }), why);
  assert.equal(refusalOf({ error: 'top level first', running: { status: 'unknown', why } }), 'top level first');
  for (const body of [{ running: null }, { running: { status: 'running', why } }, { running: { status: 'unknown', why: '' } },
    { running: { status: 'unknown', why: 7 } }, { running: { status: 'unknown' } }, { running: [why] }, { running: why }]) {
    assert.equal(refusalOf(body), null, JSON.stringify(body));
  }
  assert.equal(await refusalFrom('/backtest/run.json', Response.json({ running: { status: 'unknown', why } }, { status: 503 })),
    `/backtest/run.json answered HTTP 503: ${why}`);
});

// F1 (OBSV-19, D-3218): the audit layer refuses any audited route with its own
// envelope, whose `why` says whether the handler ran. `refusalOf` read only
// `refusal` from it, so /live.json, /frontier.json, /backtest/run.json and
// /engine/boolean-launch.json readers dropped that half.
test('an audit-layer envelope is read for its refusal and its why, and only a validated one (F1)', async () => {
  const body = { schema_version: 1, refusal: 'bounded terminal audit could not settle: Busy', code: 'invocation_audit_unavailable',
    handler_completed: true, why: 'The handler already ran. Its work may still be running or saved; inspect the exact invocation before retrying a write.' };
  assert.equal(refusalOf(body), `${body.refusal} ${body.why}`);
  assert.equal(await refusalFrom('/live.json', Response.json(body, { status: 503 })), `/live.json answered HTTP 503: ${body.refusal} ${body.why}`);
  const read = { schema_version: 1, refusal: 'invocation index is busy', code: 'invocation_audit_read_unavailable', why: 'No audit snapshot was published.' };
  assert.equal(refusalOf(read), 'invocation index is busy No audit snapshot was published.');
  for (const damage of [{ schema_version: 2 }, { code: 'invocation_audit' }, { handler_completed: 1 }, { why: null }]) {
    assert.equal(refusalOf({ ...body, ...damage }), body.refusal, `unvalidated ${JSON.stringify(damage)} falls back to the bare refusal key`);
  }
  assert.equal(refusalOf({ ...read, handler_completed: false }), read.refusal);
  assert.equal(auditRefusal({ ...read, why: '  ' }), 'invocation index is busy', 'an empty why adds nothing');
  assert.equal(auditRefusal({ ...read, refusal: 'r'.repeat(4096), why: 'w'.repeat(4096) }), `${'r'.repeat(4096)} ${'w'.repeat(4096)}`);
  assert.equal(auditRefusal({ ...read, refusal: 'r'.repeat(4097) }), null);
  assert.equal(auditRefusal({ ...read, why: 'w'.repeat(4097) }), null);
  for (const bad of [null, [], 'text', {}]) assert.equal(auditRefusal(bad), null);
});

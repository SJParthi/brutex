import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFeedSummary, readFeedHeader, surveyFeedHeaders } from '../src/lib/feed-summary.js';

/** @param {Record<string, string>} [overrides] */
function held(overrides = {}) {
  return new Response(null, { headers: {
    'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '',
    'x-brutex-census-note': 'zerodha: 148222 month(s), 282845758 row(s), generation 210950',
    ...overrides
  } });
}

test('exact live census headers preserve counts and absent manifests are explicitly zero', () => {
  assert.deepEqual(readFeedSummary('zerodha', held()), { cells: 148222, bars: 282845758 });
  assert.deepEqual(readFeedSummary('dhan', held({
    'x-brutex-census-state': 'absent', 'x-brutex-census-note': 'dhan: UNAVAILABLE ? no manifest at fixture'
  })), { cells: 0, bars: 0 });
});

test('malformed, wrong-feed, rounded, negative, or missing counters never become empty', () => {
  for (const note of ['', 'foreign: 1 month(s), 2 row(s), generation 3',
    'zerodha: 1 month(s), -2 row(s), generation 3', 'zerodha: 1 month(s), 1.5 row(s), generation 3',
    'zerodha: 1 month(s), 9007199254740992 row(s), generation 3',
    'zerodha: 1 month(s), 2 row(s), generation 9007199254740992',
    'zerodha: 1 month(s), 02 row(s), generation 3']) {
    assert.throws(() => readFeedSummary('zerodha', held({ 'x-brutex-census-note': note })),
      /unavailable census summary|exceed exact browser integers/);
  }
});

test('unreadable, degraded, absent-but-foreign, HTTP, and missing-header summaries refuse', () => {
  assert.throws(() => readFeedSummary('zerodha', new Response(null, { status: 503 })), /HTTP 503/);
  assert.throws(() => readFeedSummary('zerodha', new Response(null)), /missing its integrity header/);
  assert.throws(() => readFeedSummary('zerodha', held({ 'x-brutex-census-degraded': 'recovered stale generation' })), /degraded/);
  assert.throws(() => readFeedSummary('zerodha', held({ 'x-brutex-census-state': 'unreadable' })), /unavailable census summary/);
  assert.throws(() => readFeedSummary('zerodha', held({
    'x-brutex-census-state': 'absent', 'x-brutex-census-note': 'foreign: UNAVAILABLE ? no manifest'
  })), /unavailable census summary/);
});

test('simultaneous startup and picker surveys share HEAD reads and keep at most two outstanding', async () => {
  let active = 0, peak = 0, calls = 0;
  const feeds = Array.from({ length: 5 }, (_, index) => ({ wire: `feed${index}`, ready: true }));
  /** @type {import('../src/lib/ask.js').ask} */
  const request = async (url, options) => {
    assert.equal(options?.method, 'HEAD'); calls++; active++; peak = Math.max(peak, active);
    await new Promise((done) => setImmediate(done)); active--;
    const wire = new URL(url, 'http://fixture').searchParams.get('feed');
    return held({ 'x-brutex-census-note': `${wire}: 1 month(s), 2 row(s), generation 3` });
  };
  const [first, second] = await Promise.all([surveyFeedHeaders(feeds, request), surveyFeedHeaders(feeds, request)]);
  assert.equal(calls, 5);
  assert.equal(peak, 2);
  assert.deepEqual(first, second);
  await readFeedHeader('feed0', request);
  assert.equal(calls, 6, 'a later refresh must revalidate rather than reuse stale summary counts');
});

test('a failed shared HEAD is released and the same feed can recover on the next read', async () => {
  let calls = 0;
  /** @type {import('../src/lib/ask.js').ask} */
  const request = async () => ++calls === 1 ? new Response(null, { status: 503 }) : held();
  const first = readFeedHeader('zerodha', request);
  assert.strictEqual(readFeedHeader('zerodha', request), first);
  await assert.rejects(first, /HTTP 503/);
  assert.deepEqual(await readFeedHeader('zerodha', request), { cells: 148222, bars: 282845758 });
  assert.equal(calls, 2);
});

/* WP-56 (D-3327 item 2): `capped` and `behind` are a census that LOADED and
   cannot be called complete. Its counts are its own; refusing them would
   survey a feed holding 3.6 billion bars as unreadable. */
import { readCensusSummary, LOADED_STATES } from '../src/lib/feed-summary.js';

test('a capped or behind census is counted, with its state, generation and refused days beside the counts', () => {
  assert.deepEqual(LOADED_STATES, ['held', 'capped', 'behind']);
  for (const state of ['capped', 'behind']) {
    assert.deepEqual(readFeedSummary('zerodha', held({ 'x-brutex-census-state': state })), { cells: 148222, bars: 282845758 });
  }
  const summary = readCensusSummary('zerodha', held({ 'x-brutex-census-state': 'behind',
    'x-brutex-census-behind-days': '32', 'x-brutex-census-behind-from': '2019-11-14' }));
  assert.equal(summary.generation, 210950);
  assert.equal(summary.lag.state, 'behind');
  assert.equal(summary.lag.behindDays, 32);
  assert.equal(summary.lag.behindFrom, '2019-11-14');
  assert.throws(() => readFeedSummary('zerodha', held({ 'x-brutex-census-state': 'sleepy' })), /unavailable census summary/);
});

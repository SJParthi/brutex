import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { COUNTS, fetchScrub, readScrub, scrubUrl, verdictHeading } from '../src/lib/store-scrub.js';

/** A generated reply in the shape `verify_reading` writes; not a real store.
 * @param {Record<string, unknown>} [over] */
function reply(over = {}) {
  return {
    feed: 'dhan', verified: true, say: 'generated fixture sentence', seen: 2, agreed: 2,
    missing: 0, rows: 0, bounds: 0, unreadable: 0, undrawn: 0, findings: [], refused: null,
    ...over,
  };
}

/** @param {any} body @param {number} [status] */
const response = (body, status = 200) => new Response(JSON.stringify(body), { status });

test('the scrub URL names the feed and escapes it', () => {
  assert.equal(scrubUrl('dhan'), '/verify.json?feed=dhan');
  assert.equal(scrubUrl('a b&c'), '/verify.json?feed=a%20b%26c');
});

test('each verdict is read from the reply and never invented', () => {
  const clean = readScrub(200, reply());
  assert.equal(clean.verdict, 'verified');
  assert.equal(clean.say, 'generated fixture sentence');
  assert.deepEqual(clean.counts.map((c) => c.key), COUNTS.map(([key]) => key));
  assert.equal(clean.counts[0].n, 2);

  const wrong = readScrub(200, reply({ verified: false, agreed: 1, rows: 1, findings: ['NSE-INDEX-X 1day 2025-05 — rows'], undrawn: 3 }));
  assert.equal(wrong.verdict, 'disagrees');
  assert.deepEqual(wrong.findings, ['NSE-INDEX-X 1day 2025-05 — rows']);
  assert.equal(wrong.undrawn, 3);

  const empty = readScrub(200, reply({ verified: false, seen: 0, agreed: 0 }));
  assert.equal(empty.verdict, 'empty', 'an empty counter is not a clean store');

  const refused = readScrub(503, reply({ verified: false, seen: 0, agreed: 0, refused: 'the census could not be read' }));
  assert.equal(refused.verdict, 'refused', 'a refused question is not an answer');
});

test('a question that was never asked is drawn as unanswered with the server words', () => {
  const notFeed = readScrub(400, { refused: 'this build reads no feed called that', feed: 'nope' });
  assert.equal(notFeed.verdict, 'unanswered');
  assert.equal(notFeed.say, 'this build reads no feed called that');
  assert.equal(notFeed.feed, 'nope');
  assert.deepEqual(notFeed.counts, []);
  const busy = readScrub(429, { error: 'scrub not admitted (Saturated): retry' });
  assert.equal(busy.verdict, 'unanswered');
  assert.equal(busy.say, 'scrub not admitted (Saturated): retry');
  assert.equal(busy.feed, '');
});

test('a reply in any other shape is refused by name', () => {
  assert.throws(() => readScrub(200, null), /not an object/);
  assert.throws(() => readScrub(200, []), /not an object/);
  assert.throws(() => readScrub(500, {}), /without a verdict/);
  assert.throws(() => readScrub(200, reply({ verified: 'yes' })), /cannot read/);
  assert.throws(() => readScrub(200, reply({ say: 7 })), /cannot read/);
  assert.throws(() => readScrub(200, reply({ findings: 'none' })), /findings/);
  assert.throws(() => readScrub(200, reply({ findings: [1] })), /findings/);
  assert.throws(() => readScrub(200, reply({ rows: -1 })), /whole count for rows/);
  assert.throws(() => readScrub(200, reply({ seen: 1.5 })), /whole count for seen/);
  assert.throws(() => readScrub(200, reply({ undrawn: null })), /whole count for undrawn/);
});

test('fetchScrub asks the one URL and reads what comes back', async () => {
  /** @type {string[]} */
  const asked = [];
  const read = await fetchScrub('dhan', async (url) => { asked.push(url); return response(reply({ verified: false, rows: 1, agreed: 1 })); });
  assert.deepEqual(asked, ['/verify.json?feed=dhan']);
  assert.equal(read.verdict, 'disagrees');
  await assert.rejects(
    fetchScrub('dhan', async () => new Response('not json', { status: 502 })),
    /answered 502 with a body that is not JSON/
  );
});

test('every verdict has its own heading', () => {
  const headings = ['verified', 'disagrees', 'empty', 'refused', 'unanswered'].map(
    (v) => verdictHeading(/** @type {any} */ (v))
  );
  assert.equal(new Set(headings).size, 5);
  assert.match(verdictHeading('refused'), /Not checked/);
});

test('the database page mounts the scrub panel for the chosen feed and never polls it', () => {
  const page = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /import StoreScrub from '\$lib\/StoreScrub\.svelte';/);
  assert.match(page, /\{#if feeds\.active\}<StoreScrub feed=\{feeds\.active\} \/>\{\/if\}/);
  const panel = readFileSync(new URL('../src/lib/StoreScrub.svelte', import.meta.url), 'utf8');
  assert.match(panel, /fetchScrub\(asked/);
  assert.doesNotMatch(panel, /setInterval|watchVisible/, 'a scrub opens every bar file; it runs on request only');
  assert.match(panel, /href="\/logs\?target=api\.verify" data-sveltekit-reload/);
});

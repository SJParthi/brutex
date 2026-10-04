// conc18-5: the constituents crawl result is feed-specific and was never
// cleared on a feed change, and its feed was never rendered, so feed A's join
// counts and publishable verdict were shown under feed B.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';

const source = readFileSync(new URL('../src/routes/mapping/+page.svelte', import.meta.url), 'utf8');
const ast = /** @type {any} */ (parse(source));
const body = ast.instance.content.body;
const resolve = body.find((/** @type {any} */ n) => n.type === 'FunctionDeclaration' && n.id?.name === 'resolveUniverse');
assert.ok(resolve, 'mapping: actual resolveUniverse function is required');
/** Every top-level `$effect(() => …)` callback, as source. */
const effects = body
  .filter((/** @type {any} */ n) => n.type === 'ExpressionStatement' && n.expression.type === 'CallExpression' && n.expression.callee.name === '$effect')
  .map((/** @type {any} */ n) => source.slice(n.expression.arguments[0].start, n.expression.arguments[0].end));

function deferred() {
  /** @type {(v?:any)=>void} */ let resolve = () => {};
  const promise = new Promise((yes) => { resolve = yes; });
  return { promise, resolve };
}

function page() {
  /** @type {ReturnType<typeof deferred>[]} */ const replies = [];
  const create = new Function('ask', `
    const feeds={active:'dhan'}, untrack=(f)=>f();
    let crawl={phase:'idle',body:null,why:'',feed:null}, crawlSeq=0;
    const effects=[${effects.join(',')}];
    ${source.slice(resolve.start, resolve.end)}
    // Every effect runs on a feed change; only the ones about the crawl touch it.
    const switchFeed=(next)=>{feeds.active=next; for (const e of effects) { try { e(); } catch {} }};
    return {resolveUniverse, switchFeed, crawl:()=>crawl};
  `);
  const app = create((/** @type {string} */ url) => { assert.equal(url, '/universe/resolve'); const reply = deferred(); replies.push(reply); return reply.promise; });
  return { app, replies };
}
const answer = (/** @type {string} */ feed) => Response.json({ ok: true, feed, published: 750, publishable: true, digest: 'd', key: 'k', identity: 'i', day: '2026-10-04' });

test('a crawl result does not survive a feed change', async () => {
  const { app, replies } = page();
  const press = app.resolveUniverse();
  replies[0].resolve(answer('dhan')); await press;
  assert.equal(app.crawl().phase, 'done');
  app.switchFeed('groww');
  assert.equal(app.crawl().phase, 'idle');
  assert.equal(app.crawl().body, null);
});

test('a crawl that lands after the feed changed is dropped', async () => {
  const { app, replies } = page();
  const press = app.resolveUniverse();
  app.switchFeed('groww');
  replies[0].resolve(answer('dhan')); await press;
  assert.equal(app.crawl().body, null, "dhan's crawl must not appear under groww");
});

test('the crawl result names the feed it was checked against', () => {
  assert.match(source, /feed \{crawl\.body\.feed \?\? crawl\.feed\}/);
});

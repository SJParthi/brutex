import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFeedStartup, FEED_PREFERENCE_KEY } from '../src/lib/feed-startup.js';

const descriptors = [{ wire: 'first', ready: false }, { wire: 'ready', ready: true }];
/** @param {unknown} [body] */
const response = (body = descriptors) => Response.json(body);
/** @returns {{all: typeof descriptors, active: string | null, error: string | null}} */
const state = () => ({ all: [], active: null, error: null });
/** @param {string | null} [initial] @returns {Pick<Storage, 'getItem' | 'setItem'>} */
function preferences(initial = null) {
  /** @type {Map<string, string>} */
  const values = new Map(initial === null ? [] : [[FEED_PREFERENCE_KEY, initial]]);
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value)
  };
}
function deferred() {
  /** @type {(value: Response) => void} */
  let resolve = () => { throw new Error('Deferred response was not initialized'); };
  /** @type {Promise<Response>} */
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

/** @param {string} url */
async function ordinary(url) {
  if (url === '/feeds.json') return response();
  const wire = new URL(url, 'http://fixture').searchParams.get('feed');
  return new Response(null, { headers: {
    'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '',
    'x-brutex-census-note': `${wire}: 0 month(s), 0 row(s), generation 0`
  } });
}

test('saved preference selects from descriptors without requesting any store census', async () => {
  const feeds = state();
  /** @type {string[]} */
  const urls = [];
  const loader = createFeedStartup(feeds, async (url) => {
    urls.push(url);
    assert.equal(url, '/feeds.json', 'a census must not precede initial selection');
    return response();
  }, () => preferences('"ready"'));
  const first = loader.load();
  assert.equal(loader.load(true), first, 'even a concurrent retry shares the descriptor read');
  await first;
  assert.equal(feeds.active, 'ready');
  await loader.load();
  assert.deepEqual(urls, ['/feeds.json']);
});

test('a saved available feed or explicit clear wins over the ready-first default', async () => {
  for (const choice of ['first', null]) {
    const feeds = state();
    await createFeedStartup(feeds, async () => response(), () => preferences(JSON.stringify(choice))).load();
    assert.equal(feeds.active, choice);
  }
});

test('invalid, removed, or unavailable browser preferences cannot invent a feed', async () => {
  for (const saved of ['{bad', '"removed"', '17', '{}']) {
    const feeds = state();
    await createFeedStartup(feeds, ordinary, () => preferences(saved)).load();
    assert.equal(feeds.active, 'ready');
  }
  const feeds = state();
  await createFeedStartup(feeds, ordinary, () => { throw new Error('Storage disabled'); }).load();
  assert.equal(feeds.active, 'ready');
});

test('a user choice or clear during a forced descriptor refresh is never overwritten', async () => {
  for (const choice of ['first', null]) {
    const feeds = { all: descriptors, active: 'ready', error: null };
    const storage = preferences('"ready"');
    const pending = deferred();
    const loader = createFeedStartup(feeds, () => pending.promise, () => storage);
    const loading = loader.load(true);
    loader.select(choice);
    pending.resolve(response());
    await loading;
    assert.equal(feeds.active, choice);
    assert.equal(storage.getItem(FEED_PREFERENCE_KEY), JSON.stringify(choice));
  }
});

test('existing selection survives a retry and invalid explicit selection is refused', async () => {
  const feeds = { all: descriptors, active: 'first', error: null };
  const loader = createFeedStartup(feeds, async () => response(), () => preferences('"ready"'));
  await loader.load(true);
  assert.equal(feeds.active, 'first');
  assert.throws(() => loader.select('foreign'), /absent from \/feeds.json/);
  assert.equal(feeds.active, 'first');
});

test('HTTP and malformed descriptor failures stay visible and can retry', async () => {
  for (const bad of [new Response('', { status: 503 }), response({}), response([{ wire: 'x' }]),
    response([{ wire: 'x', ready: true }, { wire: 'x', ready: false }])]) {
    const feeds = state();
    let calls = 0;
    const loader = createFeedStartup(feeds, async () => ++calls === 1 ? bad : response(), () => preferences('"ready"'));
    await loader.load();
    assert.match(feeds.error ?? '', /HTTP 503|valid feed descriptor list/);
    assert.equal(feeds.active, null);
    await loader.load();
    assert.equal(feeds.error, null);
    assert.equal(feeds.active, 'ready');
    assert.equal(calls, 2);
  }
});

test('first visit preserves the held-data default using bounded HEAD headers with no body read', async () => {
  const feeds = state();
  /** @type {string[]} */
  const methods = [];
  const loader = createFeedStartup(feeds, async (url, options = {}) => {
    if (url === '/feeds.json') return response();
    assert.equal(options.method, 'HEAD');
    assert.equal(options.ms, 15_000);
    methods.push(url);
    const wire = new URL(url, 'http://fixture').searchParams.get('feed');
    const result = new Response(null, { headers: {
      'x-brutex-census-state': 'held', 'x-brutex-census-degraded': '',
      'x-brutex-census-note': `${wire}: 1 month(s), ${wire === 'first' ? 8_922 : 0} row(s), generation 1`
    } });
    result.json = async () => { throw new Error('HEAD must not parse an inventory body'); };
    return result;
  }, () => preferences());
  await loader.load();
  assert.equal(feeds.active, 'first', 'held data beats a ready-but-empty descriptor');
  assert.equal(feeds.error, null);
  assert.deepEqual(methods, ['/store.json?feed=first', '/store.json?feed=ready']);
});

test('a user selection or clear wins while first-visit HEAD summaries are pending, including failures', async () => {
  for (const choice of ['first', null]) {
    const feeds = state();
    const pending = deferred();
    const loader = createFeedStartup(feeds, async (url) => url === '/feeds.json' ? response() : pending.promise,
      () => preferences());
    const loading = loader.load();
    // Waiting for the descriptor body permits an actual picker selection.
    await new Promise((resolve) => setImmediate(resolve));
    assert.equal(feeds.all.length, 2);
    loader.select(choice);
    pending.resolve(new Response(null, { status: 503 }));
    await loading;
    assert.equal(feeds.active, choice);
    assert.equal(feeds.error, null);
  }
});

test('missing census headers leave the choice unmeasured and show a named refusal', async () => {
  const feeds = state();
  const loader = createFeedStartup(feeds, async (url) => url === '/feeds.json' ? response() : new Response(null),
    () => preferences());
  await loader.load();
  assert.equal(feeds.active, null);
  assert.match(feeds.error ?? '', /Feed first:.*missing its integrity header/);
});

test('an empty descriptor list has no invented selection', async () => {
  const feeds = state();
  await createFeedStartup(feeds, async () => response([]), () => preferences('"ready"')).load();
  assert.deepEqual(feeds.all, []);
  assert.equal(feeds.active, null);
  assert.equal(feeds.error, null);
});

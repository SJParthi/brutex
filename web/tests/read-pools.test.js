// R3-08: the read pools as tables (/pools.json). Every fixture is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { POOL_FIELDS, ROUTE_FIELDS, poolTables } from '../src/lib/read-pools.js';

const pool = (name, over = {}) => ({ pool: name, slots: 4, running: 0, waiting: 0, answered: 10, busy: 0, cancelled: 0, ...over });
const route = (path, over = {}) => ({ route: path, answered: 5, busy: 0, being_written: 0, cancelled: 0, refused: 0, failed: 0, ...over });
const body = (over = {}) => ({ cores: 14, wait_ms: 250, max_waiting: 64, read_patience_ms: 200, pools: [pool('store-read')], routes: [route('/bars.json')], say: 'since start', ...over });

test('a whole answer becomes rows, with every counter kept as served', () => {
  const t = poolTables(body());
  assert.equal(t.fault, null);
  assert.equal(t.cores, 14);
  assert.equal(t.waitMs, 250);
  assert.equal(t.patienceMs, 200);
  assert.deepEqual(t.pools, [pool('store-read')]);
  assert.deepEqual(t.routes, [{ ...route('/bars.json'), asked: 5 }]);
});

test('the routes turned away come first: busy, being written and cancelled; ties by route', () => {
  const t = poolTables(
    body({
      routes: [route('/a.json'), route('/z.json', { being_written: 2 }), route('/m.json', { busy: 1, cancelled: 1 }), route('/b.json')]
    })
  );
  assert.deepEqual(
    t.routes.map((r) => r.route),
    ['/m.json', '/z.json', '/a.json', '/b.json']
  );
  assert.equal(t.routes[0].asked, 7);
});

test('a missing counter is named, never read as zero', () => {
  for (const field of POOL_FIELDS) {
    const row = pool('census');
    delete row[field];
    assert.equal(poolTables(body({ pools: [row] })).fault, `/pools.json pool census has no count for ${field}.`);
  }
  for (const field of ROUTE_FIELDS) {
    const row = route('/gaps.json');
    row[field] = -1;
    assert.equal(poolTables(body({ routes: [row] })).fault, `/pools.json route /gaps.json has no count for ${field}.`);
  }
});

test('garbage bodies are named by what is wrong', () => {
  assert.equal(poolTables(null).fault, '/pools.json did not answer an object.');
  assert.equal(poolTables([]).fault, '/pools.json did not answer an object.');
  assert.equal(poolTables({ ...body(), pools: 'x' }).fault, '/pools.json carries no pools list.');
  assert.equal(poolTables({ ...body(), routes: null }).fault, '/pools.json carries no routes list.');
  assert.equal(poolTables({ ...body(), cores: 1.5 }).fault, '/pools.json names no core count or wait.');
  assert.equal(poolTables(body({ pools: [{ slots: 1 }] })).fault, '/pools.json pool row 0 has no name.');
  assert.equal(poolTables(body({ routes: [route('bars.json')] })).fault, '/pools.json route row 0 names no route.');
});

// THE BODY A REAL SERVER SENDS ONCE A BROWSER HAS LOADED THE PAGE (R3-08
// repair 2, verify-R3-08-r2 blocking 1): the fallback counted the page and its
// assets under the front end's row before the panel asked, and the reader
// refused that row, so the panel showed only a fault line. Shaped like the
// shadow server's /pools.json after GET /ingest; the counts are invented.
test('the front end row the server names is one more route row, never a fault', () => {
  const served = body({
    front_end: '(front end)',
    pools: [pool('store-read'), pool('verify', { slots: 1 })],
    routes: [route('(front end)', { answered: 31 }), route('/feeds.json'), route('/pools.json', { answered: 1 })]
  });
  const t = poolTables(served);
  assert.equal(t.fault, null);
  assert.deepEqual(
    t.routes.map((r) => r.route),
    ['(front end)', '/feeds.json', '/pools.json']
  );
  assert.equal(t.routes[0].asked, 31);
  // ONLY THE NAME THE SERVER GAVE: another pathless row, or the same row from
  // a body that names no front end, is still a fault.
  assert.equal(
    poolTables({ ...served, routes: [route('(front end)'), route('(other)')] }).fault,
    '/pools.json route row 1 names no route.'
  );
  const { front_end: _, ...unnamed } = served;
  assert.equal(poolTables(unnamed).fault, '/pools.json route row 0 names no route.');
  assert.equal(poolTables({ ...served, front_end: '' }).fault, '/pools.json route row 0 names no route.');
});

test('the server names the front end row it counts, from the same constant', () => {
  const src = readFileSync(new URL('../../crates/api/src/detail.rs', import.meta.url), 'utf8');
  assert.match(src, /"front_end": FRONT_END,/);
  assert.match(src, /map_or\(FRONT_END, axum::extract::MatchedPath::as_str\)/);
});

test('empty lists and an absent patience are kept as they are', () => {
  const { read_patience_ms: _, ...rest } = body({ pools: [], routes: [] });
  const t = poolTables(rest);
  assert.equal(t.fault, null);
  assert.deepEqual(t.pools, []);
  assert.deepEqual(t.routes, []);
  assert.equal(t.patienceMs, null);
});

test('the panel reads /pools.json through the poller and shows both tables', () => {
  const src = readFileSync(new URL('../src/routes/ingest/panels/ReadPools.svelte', import.meta.url), 'utf8');
  assert.match(src, /ask\('\/pools\.json'/);
  assert.match(src, /createPoller\(\{\s*every: 2000/);
  assert.match(src, /poll\.stop\(\)/);
  assert.match(src, /aria-label="Pools"/);
  assert.match(src, /aria-label="Routes"/);
  for (const head of ['Busy', 'Being written', 'Cancelled']) assert.match(src, new RegExp(`>${head}</th>`));
});

test('a pool counts work it ran, never the routes’ answered: the two columns are named apart', () => {
  // R3-08 repair 2 (verifier follow-up): a being-written 503 is work the
  // store_read pool ran to its end, and the route row counts it
  // being_written. Both columns were headed "Answered", so the same request
  // read as answered in one table and being written in the other.
  const src = readFileSync(new URL('../src/routes/ingest/panels/ReadPools.svelte', import.meta.url), 'utf8');
  const [poolTable, routeTable] = src.split('aria-label="Routes"');
  assert.match(poolTable, />Ran<\/th>/);
  assert.doesNotMatch(poolTable, />Answered<\/th>/);
  assert.match(routeTable, />Answered<\/th>/);
  const server = readFileSync(new URL('../../crates/api/src/detail.rs', import.meta.url), 'utf8');
  assert.match(server, /A pool's answered is work it ran to its end/);
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createResearchTester, researchRule, researchSetting } from '../src/lib/research-tester.js';
import { assessment as indexAssessment, sync as syncAssessment } from './index-consistency-fixture.js';

const id = (/** @type {number} */ n) => n.toString(16).padStart(64, '0');
const flush = () => new Promise(resolve => setImmediate(resolve));

/** Generated, already-authenticated detail seam. These fixtures test selection
 * and request lifecycle only; saved-backtest-setting-pages.test.js exercises
 * the production wire validators and the complete coordinate/receipt join.
 * @param {string} [index] @param {string} [coordinate] @param {number} [seed]
 * @returns {any} */
function detail(index = '480', coordinate = '144', seed = 1) {
  const policy = { digest: id(seed + 10), values: [{ name: 'min_trades', value: '200' }] };
  const row = {
    index, identity: id(seed + 11),
    statistics: { identity: id(seed + 12), coordinate, side: 'short',
      source: { identity: id(seed + 13), completion: id(seed + 14), instrument: 'GENERATED-FIXTURE' } },
    qualification: { identity: id(seed + 15), coordinate,
      source: { identity: id(seed + 16), completion: id(seed + 17) } }
  };
  const comparison = { index, identity: row.identity, policy_digest: policy.digest,
    status: 'refused', checks: [{ index: '0', state: 'refused' }], values: [] };
  return { authority: 'authenticated-search-projection', kind: 'rung', child_bodies_checked: true,
    projection_version: 2, identity: id(seed + 20), pin: id(seed + 21), completion: id(seed + 22),
    batch: '9007199254740993', rung: '7', search_policy: policy,
    source: { completion: id(seed + 22), offset: index, rows: [row] }, rows: [{ comparison }] };
}

/** @returns {{promise:Promise<any>,resolve:(value:any)=>void,reject:(error:any)=>void}} */
function deferred() {
  /** @type {(value:any)=>void} */ let resolve = () => { throw new Error('not initialized'); };
  /** @type {(error:any)=>void} */ let reject = () => { throw new Error('not initialized'); };
  const promise = new Promise((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

/** @param {any} row */
function factsFor(row) {
  return { training: { identity: row.statistics.identity, expression: '(0 | !(1))', cell: { trades: '0' } },
    later: { identity: row.qualification.identity, cell: { trades: '0' } },
    grid: { first_micros: '0', last_micros: '60000000' }, laterPeriod: { from: '1970-02', to: '1970-03' } };
}

/** @param {any} selection @param {any[]} [rows] */
function savedPage(selection, rows = []) {
  return { ...selection, rows, total: String(rows.length), next: null };
}

/** @typedef {{request?:(url:string,init:any)=>any,
 * read?:(period:string,selection:any,transport:any)=>any,
 * facts?:(rows:any[],transport:any,signal:AbortSignal)=>any,
 * join?:(body:any,row:any,isLater:boolean,fact:any)=>any}} HarnessOptions */
/** @param {HarnessOptions} [options] */
function harness(options = {}) {
  /** @type {any[]} */ const states = [];
  const calls = { facts: /** @type {any[]} */ ([]), reads: /** @type {any[]} */ ([]),
    joins: /** @type {any[]} */ ([]), requests: /** @type {any[]} */ ([]) };
  const request = async (/** @type {string} */ url, /** @type {any} */ init) => {
    calls.requests.push({ url, init });
    if (!options.request) throw new Error('Unexpected HTTP request from lifecycle fixture');
    return options.request(url, init);
  };
  const read = async (/** @type {string} */ period, /** @type {any} */ selection, /** @type {any} */ transport) => {
    calls.reads.push({ period, selection });
    return options.read ? options.read(period, selection, transport) : savedPage(selection);
  };
  const tester = createResearchTester(state => states.push(state), request, {
    facts: async (rows, transport, signal) => {
      calls.facts.push({ rows, signal });
      return options.facts ? options.facts(rows, transport, signal)
        : new Map(rows.map(row => [row.index, factsFor(row)]));
    },
    training: (selection, transport) => read('training', selection, transport),
    later: (selection, transport) => read('later', selection, transport),
    join: (body, row, isLater, fact) => {
      calls.joins.push({ body, row, isLater, fact });
      return options.join ? options.join(body, row, isLater, fact) : body;
    }
  });
  return { tester, states, calls };
}

/** @param {any} value */
function freeze(value) {
  if (value && typeof value === 'object') { Object.values(value).forEach(freeze); Object.freeze(value); }
  return value;
}

test('the setting address keeps exact search, checkpoint, child, batch and local coordinate', () => {
  const body = detail('9007199254740993', '18446744073709551615');
  const chosen = researchSetting(body, '9007199254740993');
  assert.deepEqual([chosen.search, chosen.pin, chosen.completion, chosen.batch, chosen.rung, chosen.projection, chosen.pageOffset],
    [body.identity, body.pin, body.completion, body.batch, '7', 2, '9007199254740993']);
  assert.equal(chosen.row.index, '9007199254740993');
  assert.equal(chosen.row.statistics.coordinate, '18446744073709551615');
  assert.equal(chosen.row.qualification.coordinate, '18446744073709551615');
  assert.throws(() => researchSetting(body, '0'), /absent or duplicated/);
  body.projection_version = 1;
  assert.equal(researchSetting(body, '9007199254740993').projection, 1, 'historical arithmetic remains historical');
});

test('subset projection version 3 keeps physical timeframe indices and refuses excluded or widened scopes', () => {
  const body = detail();
  body.projection_version = 3; body.timeframes = ['1min', '60min'];
  const chosen = researchSetting(body, '480');
  assert.equal(chosen.rung, '7'); assert.deepEqual(chosen.timeframes, ['1min', '60min']);
  body.timeframes[0] = '5min';
  assert.deepEqual(chosen.timeframes, ['1min', '60min'], 'the selected scope is retained independently of later overview edits');
  for (const timeframes of [undefined, null, [], ['5min'], ['60min', '1min'], ['60min', '60min'],
    ['1min', '2min', '3min', '5min', '10min', '15min', '30min', '60min']]) {
    const invalid = { ...body, timeframes };
    assert.throws(() => researchSetting(invalid, '480'));
  }
  delete body.timeframes;
  assert.throws(() => researchSetting(body, '480'), /timeframe selection/);
  body.projection_version = 2; body.timeframes = ['60min'];
  assert.throws(() => researchSetting(body, '480'), /timeframe selection/);
});

test('unauthenticated, unpinned, malformed and mismatched pages refuse before facts or HTTP', async () => {
  /** @type {((body:any)=>void)[]} */ const damage = [
    b => { b.authority = 'acknowledged-search-history'; }, b => { b.kind = 'overview'; },
    b => { b.child_bodies_checked = false; }, b => { b.projection_version = 5; },
    b => { b.projection_version = '2'; }, b => { b.source.completion = id(99); },
    b => { b.rung = '8'; }, b => { b.rung = 0; }, b => { b.batch = '01'; },
    b => { b.batch = '18446744073709551616'; }, b => { b.source.rows = null; },
    b => { b.rows = []; }, b => { b.rows[0].comparison.index = '481'; },
    b => { b.rows[0].comparison.identity = id(99); },
    b => { b.rows[0].comparison.policy_digest = id(99); },
    b => { b.rows[0].comparison.status = 'profitable'; }
  ];
  for (const field of ['identity', 'pin', 'completion']) for (const value of ['', null, 'AB'.repeat(32), '0'.repeat(64)]) {
    damage.push(b => { b[field] = value; });
  }
  damage.push(b => { b.search_policy.digest = undefined; });
  for (const mutate of damage) {
    const body = detail(); mutate(body);
    const h = harness();
    assert.equal(await h.tester.open(body, '480'), null);
    assert.equal(h.states.at(-1).phase, 'failed');
    assert.equal(h.states.at(-1).body, null);
    assert.equal(h.calls.facts.length, 0);
    assert.equal(h.calls.reads.length + h.calls.requests.length, 0);
  }
});

test('selection refuses duplicate/missing settings and enforces the authenticated overview page ceiling', () => {
  const duplicate = detail();
  duplicate.source.rows.push(structuredClone(duplicate.source.rows[0]));
  duplicate.rows.push(structuredClone(duplicate.rows[0]));
  assert.throws(() => researchSetting(duplicate, '480'), /absent or duplicated/);
  for (const index of [undefined, null, 480, '', '-1', '0480', '1e3', '18446744073709551616']) {
    assert.throws(() => researchSetting(detail(), /** @type {any} */ (index)));
  }
  const large = detail();
  large.source.rows = Array.from({ length: 256 }, (_, i) => ({ ...large.source.rows[0], index: String(i) }));
  large.rows = large.source.rows.map((/** @type {any} */ row) => ({ comparison: { ...large.rows[0].comparison, index: row.index } }));
  assert.equal(researchSetting(large, '255').row.index, '255');
  large.source.rows.push({ ...large.source.rows[0], index: '256' }); large.rows.push(large.rows[0]);
  assert.throws(() => researchSetting(large, '0'));
});

test('deeply frozen caller input is not mutated and later caller edits cannot relabel the saved copy', () => {
  const body = detail(), before = structuredClone(body);
  const chosen = researchSetting(freeze(body), '480');
  assert.deepEqual(body, before);
  assert.notEqual(chosen.row, body.source.rows[0]);
  assert.notEqual(chosen.comparison, body.rows[0].comparison);
  assert.notEqual(chosen.policy, body.search_policy);
  chosen.row.statistics.source.completion = id(90);
  chosen.comparison.checks[0].state = 'passed'; chosen.policy.values[0].value = '0';
  assert.deepEqual(body, before, 'the bounded copy owns its nested source and policy data');
});

test('opening loads one setting then one later trade page using local coordinates, never display rank', async () => {
  const h = harness(), body = detail();
  const result = await h.tester.open(body, '480');
  assert.equal(h.calls.facts.length, 1);
  assert.equal(h.calls.facts[0].rows.length, 1);
  assert.deepEqual(h.calls.reads, [{ period: 'later', selection: {
    identity: body.source.rows[0].qualification.source.identity,
    completion: body.source.rows[0].qualification.source.completion,
    candidate: '144', kind: 'trades', offset: '0', limit: 32
  } }]);
  assert.equal(h.calls.joins.length, 1); assert.equal(h.calls.joins[0].isLater, true);
  assert.equal(h.states.at(-1).body, result);
  assert.deepEqual(h.states.map(state => state.phase), ['loading', 'loading', 'ready']);
  assert.equal(h.states.at(-1).context.search, body.identity);
});

test('training/later and trades/sessions switches keep exact source pins with fixed bounded pages', async () => {
  const h = harness(), body = detail('480', '9007199254740993');
  await h.tester.open(body, '480');
  for (const period of ['training', 'later']) for (const kind of ['trades', 'sessions']) {
    await h.tester.page(period, kind, '9007199254740994');
    const asked = h.calls.reads.at(-1), row = body.source.rows[0];
    const source = period === 'later' ? row.qualification.source : row.statistics.source;
    assert.deepEqual(asked, { period, selection: { identity: source.identity, completion: source.completion,
      candidate: '9007199254740993', kind, offset: '9007199254740994', limit: 32 } });
    assert.equal(h.states.at(-1).period, period); assert.equal(h.states.at(-1).kind, kind);
    assert.equal(h.calls.joins.at(-1).isLater, period === 'later');
  }
  assert.equal(h.calls.facts.length, 1, 'period navigation retains the same authenticated comparison');
});

test('invalid page selectors never read evidence and never reuse an earlier ready body', async () => {
  const h = harness(); await h.tester.open(detail(), '480');
  const start = h.calls.reads.length;
  for (const [period, kind, offset] of [
    ['future', 'trades', '0'], ['later', 'candles', '0'], ['later', 'sessions', '-1'],
    ['training', 'trades', '01'], ['later', 'trades', '18446744073709551616'],
    ['later', 'trades', '1.5'], ['later', 'trades', '1e3'], ['later', 'trades', '']
  ]) {
    assert.equal(await h.tester.page(period, kind, offset), null);
    assert.equal(h.states.at(-1).phase, 'failed'); assert.equal(h.states.at(-1).body, null);
  }
  assert.equal(h.calls.reads.length, start);
  await h.tester.close();
  assert.equal(await h.tester.page(), null);
  assert.equal(h.calls.reads.length, start, 'a closed setting has no page authority');
});

test('a page change clears stale visible rows immediately while its replacement is pending', async () => {
  const pending = deferred(); let reads = 0;
  const h = harness({ read: (_period, selection) => ++reads === 1 ? savedPage(selection, [{ index: '0' }]) : pending.promise });
  await h.tester.open(detail(), '480');
  const next = h.tester.page('training', 'sessions', '32');
  assert.equal(h.states.at(-1).phase, 'loading'); assert.equal(h.states.at(-1).body, null);
  pending.resolve(savedPage(h.calls.reads.at(-1).selection));
  await next;
  assert.equal(h.states.at(-1).phase, 'ready'); assert.equal(h.states.at(-1).period, 'training');
});

test('late facts from an older selection cannot launch a page or replace a newer setting', async () => {
  const old = deferred(); let calls = 0;
  const h = harness({ facts: rows => ++calls === 1 ? old.promise : new Map([[rows[0].index, factsFor(rows[0])]]) });
  const first = detail(), second = detail('600', '0', 50);
  const stale = h.tester.open(first, '480');
  await h.tester.open(second, '600');
  const published = h.states.length;
  assert.equal(h.calls.facts[0].signal.aborted, true);
  old.resolve(new Map([['480', factsFor(first.source.rows[0])]]));
  assert.equal(await stale, null); assert.equal(h.states.length, published);
  assert.equal(h.states.at(-1).context.search, second.identity);
  assert.equal(h.states.at(-1).context.row.index, '600');
  assert.equal(h.calls.reads.length, 1);
});

test('a late old-selection failure never replaces a newer successful result', async () => {
  const old = deferred(); let reads = 0;
  const h = harness({ read: (_period, selection) => ++reads === 1 ? old.promise : savedPage(selection) });
  const first = h.tester.open(detail(), '480'); await flush();
  await h.tester.open(detail('600', '0', 50), '600');
  const published = h.states.length;
  old.reject(new Error('old source disappeared'));
  assert.equal(await first, null); assert.equal(h.states.length, published);
  assert.equal(h.states.at(-1).phase, 'ready'); assert.equal(h.states.at(-1).context.row.index, '600');
});

test('period/kind races ignore late successful replies before the final coordinate join', async () => {
  const old = deferred(); let reads = 0;
  const h = harness({ read: (_period, selection) => ++reads === 2 ? old.promise : savedPage(selection) });
  await h.tester.open(detail(), '480');
  const stale = h.tester.page('training', 'trades', '32');
  await h.tester.page('later', 'sessions', '0');
  const published = h.states.length, joined = h.calls.joins.length;
  old.resolve(savedPage(h.calls.reads[1].selection, [{ index: '32' }]));
  assert.equal(await stale, null); assert.equal(h.states.length, published); assert.equal(h.calls.joins.length, joined);
  assert.equal(h.states.at(-1).period, 'later'); assert.equal(h.states.at(-1).kind, 'sessions');
});

test('close aborts pending facts and neither late success nor failure can leave idle', async () => {
  for (const success of [true, false]) {
    const pending = deferred(), body = detail(); const h = harness({ facts: () => pending.promise });
    const opened = h.tester.open(body, '480'); h.tester.close();
    const published = h.states.length;
    assert.equal(h.calls.facts[0].signal.aborted, true); assert.equal(h.states.at(-1).phase, 'idle');
    if (success) pending.resolve(new Map([['480', factsFor(body.source.rows[0])]]));
    else pending.reject(new Error('late facts failure'));
    assert.equal(await opened, null); assert.equal(h.states.length, published); assert.equal(h.calls.reads.length, 0);
  }
});

test('close aborts the real transport signal and a transport ignoring abort cannot publish or join', async () => {
  const response = deferred();
  const h = harness({ request: () => response.promise,
    read: async (_period, _selection, transport) => (await transport('/generated-lifecycle-fixture')).json() });
  const opened = h.tester.open(detail(), '480'); await flush();
  assert.equal(h.calls.requests.length, 1); assert.equal(h.calls.requests[0].init.cache, 'no-store');
  const signal = h.calls.requests[0].init.signal;
  h.tester.close(); const published = h.states.length;
  assert.equal(signal.aborted, true);
  response.resolve(new Response(JSON.stringify({ total: '0', rows: [] }), { status: 200 }));
  assert.equal(await opened, null); assert.equal(h.states.length, published); assert.equal(h.calls.joins.length, 0);
});

test('caller refresh during facts loading cannot change the selected search or source pins', async () => {
  const pending = deferred(), body = detail(), before = structuredClone(body);
  const h = harness({ facts: () => pending.promise });
  const opened = h.tester.open(body, '480');
  body.identity = id(90); body.pin = id(91); body.source.rows[0].index = '999';
  body.source.rows[0].qualification.source.identity = id(92);
  body.source.rows[0].statistics.source.completion = id(93);
  body.rows[0].comparison.status = 'admitted'; body.search_policy.values[0].value = '0';
  pending.resolve(new Map([['480', factsFor(before.source.rows[0])]]));
  await opened;
  assert.equal(h.states.at(-1).context.search, before.identity);
  assert.equal(h.states.at(-1).context.pin, before.pin);
  assert.equal(h.states.at(-1).context.row.index, '480');
  assert.equal(h.states.at(-1).context.comparison.status, 'refused');
  assert.equal(h.states.at(-1).context.policy.values[0].value, '200');
  assert.equal(h.calls.reads[0].selection.identity, before.source.rows[0].qualification.source.identity);
  await h.tester.page('training', 'trades');
  assert.equal(h.calls.reads.at(-1).selection.completion, before.source.rows[0].statistics.source.completion);
});

test('missing facts and failed fact authentication never fall through to trade reads', async () => {
  for (const facts of [async () => new Map(), async () => new Map([['999', {}]]),
    async () => { throw new Error('facts receipt changed'); }]) {
    const h = harness({ facts });
    assert.equal(await h.tester.open(detail(), '480'), null);
    assert.equal(h.states.at(-1).phase, 'failed'); assert.equal(h.states.at(-1).context, null);
    assert.equal(h.states.at(-1).body, null); assert.ok(h.states.at(-1).why.length > 0);
    assert.equal(h.calls.reads.length + h.calls.joins.length, 0);
  }
});

test('a rejected final join clears the body and retains the exact setting for a same-page retry', async () => {
  let joins = 0;
  const h = harness({ join: body => { if (++joins === 1) throw new Error('foreign selected receipt'); return body; } });
  assert.equal(await h.tester.open(detail(), '480'), null);
  const failed = h.states.at(-1);
  assert.equal(failed.phase, 'failed'); assert.equal(failed.body, null);
  assert.equal(failed.context.row.index, '480'); assert.match(failed.why, /foreign selected receipt/);
  await h.tester.page(failed.period, failed.kind);
  assert.equal(h.states.at(-1).phase, 'ready'); assert.equal(h.calls.facts.length, 1);
  assert.deepEqual(h.calls.reads[0], h.calls.reads[1]);
});

test('retry after a page failure retains the exact nonzero offset, period, kind and source pins', async () => {
  for (const period of ['training', 'later']) for (const kind of ['trades', 'sessions']) {
    let fail = true;
    const offset = period === 'training' ? '32' : '9007199254740993';
    const h = harness({ read: (_period, selection) => {
      if (selection.offset === offset && fail) { fail = false; throw new Error('requested saved page unavailable'); }
      return savedPage(selection);
    } });
    await h.tester.open(detail(), '480');
    assert.equal(await h.tester.page(period, kind, offset), null);
    const failed = h.states.at(-1);
    assert.equal(failed.phase, 'failed'); assert.equal(failed.body, null);
    assert.equal(failed.offset, offset);
    assert.equal(failed.period, period); assert.equal(failed.kind, kind);
    await h.tester.page(failed.period, failed.kind, failed.offset);
    assert.equal(h.states.at(-1).phase, 'ready'); assert.equal(h.states.at(-1).offset, offset);
    assert.equal(h.calls.facts.length, 1);
    assert.deepEqual(h.calls.reads[1], h.calls.reads[2], 'retry must request the failed page, not restart at zero');
  }
});

test('recorded zero remains ready while read failures have no result body or invented zero', async () => {
  for (const kind of ['trades', 'sessions']) {
    const h = harness(); await h.tester.open(detail(), '480'); await h.tester.page('later', kind);
    assert.equal(h.states.at(-1).phase, 'ready'); assert.equal(h.states.at(-1).body.total, '0');
    assert.deepEqual(h.states.at(-1).body.rows, []);
  }
  const h = harness({ read: () => { throw new Error('saved evidence unavailable (HTTP 503)'); } });
  assert.equal(await h.tester.open(detail(), '480'), null);
  assert.equal(h.states.at(-1).phase, 'failed'); assert.equal(h.states.at(-1).body, null);
  assert.match(h.states.at(-1).why, /HTTP 503/); assert.equal(h.calls.joins.length, 0);
});

test('the component exposes saved-only scope, exact paging and distinct loading/error/zero states', () => {
  const view = readFileSync(new URL('../src/lib/ResearchTester.svelte', import.meta.url), 'utf8');
  assert.match(view, /tester\.open\(\$state\.snapshot\(current\.detail\), current\.index\)/);
  assert.match(view, /return \(\) => tester\.close\(\)/);
  assert.match(view, /report\.phase === 'ready' && report\.kind === view/);
  assert.match(view, /body\.total === '0'/); assert.match(view, /recorded zero, not a read failure/);
  assert.match(view, /role="alert"/); assert.match(view, /Saved setting unavailable/);
  assert.match(view, /!report\.context \|\| view === report\.kind/);
  assert.match(view, /page unavailable at row \$\{report\.offset\}/);
  assert.match(view, /body\.next === null/); assert.match(view, /tester\.page\(report\.period, view, body\.next\)/);
  assert.match(view, /tester\.page\(report\.period, report\.kind, report\.offset\)/);
  assert.match(view, /Costs excluded/); assert.match(view, /not a separate untouched final test/);
  assert.match(view, /not a profitability ranking/); assert.doesNotMatch(view, /\{@html/);
});

/** Generated vocabulary/provenance seam. The wire validators authenticate the
 * real Rust table; these checks ensure names never escape that build binding.
 * @returns {{fact:any,vocabulary:any}} */
function ruleFixture() {
  const commit = id(55);
  return { fact: { training: { expression: '53' }, grids: [{ commit }, { commit }] },
    vocabulary: { phase: 'ready', commitDigest: commit,
      bits: new Map([[52, { name: 'close_above_vwap', live: true }], [53, { name: 'close_below_vwap', live: true }]]) } };
}

test('historical rule names require both saved grids to match the exact loaded vocabulary build', () => {
  const { fact, vocabulary } = ruleFixture(), before = structuredClone({ fact, vocabulary });
  assert.equal(researchRule(freeze(fact), freeze(vocabulary)), 'Close below the session VWAP');
  assert.deepEqual({ fact, vocabulary }, before);
  const renamed = ruleFixture(); renamed.vocabulary.bits.set(53, { name: 'other_verified_rule', live: true });
  assert.equal(researchRule(renamed.fact, renamed.vocabulary), 'other verified rule', 'the supplied table owns names, not a copied bit number');
});

test('missing, foreign or incomplete grid provenance suppresses names instead of relabelling history', () => {
  /** @type {((fixture:{fact:any,vocabulary:any})=>void)[]} */ const damage = [
    f => { f.vocabulary = null; }, f => { f.vocabulary.phase = 'loading'; },
    f => { f.vocabulary.phase = 'failed'; }, f => { delete f.vocabulary.commitDigest; },
    f => { f.vocabulary.commitDigest = id(99); }, f => { f.vocabulary.commitDigest = '0'.repeat(64); },
    f => { f.vocabulary.commitDigest = 'AB'.repeat(32); }, f => { f.vocabulary.commitDigest = 'a'.repeat(40); },
    f => { f.vocabulary.bits = []; }, f => { f.vocabulary.bits = new Map(); },
    f => { delete f.fact.grids; }, f => { f.fact.grids = null; }, f => { f.fact.grids = []; },
    f => { f.fact.grids.pop(); }, f => { f.fact.grids.push({ commit: f.vocabulary.commitDigest }); },
    f => { f.fact.grids[0].commit = id(99); }, f => { f.fact.grids[1].commit = id(99); },
    f => { delete f.fact.grids[0].commit; }, f => { delete f.fact.grids[1].commit; },
    f => { f.fact.grids[0] = null; }, f => { f.fact.grids[1] = null; },
    f => { f.fact.grids = new Array(2); }, f => { delete f.fact.grids[0]; }, f => { delete f.fact.grids[1]; }
  ];
  for (const mutate of damage) {
    const fixture = ruleFixture(); mutate(fixture);
    assert.equal(researchRule(fixture.fact, fixture.vocabulary), 'condition 53 (name not loaded)');
  }
});

test('named VWAP NOT stays conditional on availability and cannot turn index Unknown into a trade', () => {
  const { fact, vocabulary } = ruleFixture();
  fact.training = { expression: '!(53)', instrument: 'NSE-NIFTY', cell: { trades: '0', unknown: '17' } };
  const before = structuredClone(fact);
  assert.equal(researchRule(fact, vocabulary), 'Close at or above the session VWAP, when VWAP is available');
  assert.deepEqual(fact, before, 'wording changes no saved unknown count, trade result or availability');
  fact.training.expression = '!52';
  assert.equal(researchRule(fact, vocabulary), 'Close at or below the session VWAP, when VWAP is available');
  fact.training.expression = '!(999)';
  vocabulary.bits = new Map([[999, { name: 'close_below_vwap', live: true }]]);
  assert.equal(researchRule(fact, vocabulary), 'Close at or above the session VWAP, when VWAP is available');
  fact.grids[1].commit = id(99);
  assert.equal(researchRule(fact, vocabulary), 'NOT (condition 999 (name not loaded))');
});

test('version 4 strategy inspection preserves index receipt and separate corrected qualification on full or selected timeframes', () => {
  for (const timeframes of [['1min','2min','3min','5min','10min','15min','30min','60min'], ['60min']]) {
    const body = detail(); body.projection_version = 4; body.timeframes = [...timeframes]; body.source.identity = id(99);
    const row = body.source.rows[0], c = indexAssessment(); row.statistics.source.instrument = 'NSE-NIFTY';
    c.qualification = { identity: body.source.identity, completion: body.source.completion };
    row.status = 'admitted'; row.index_consistency = structuredClone(c);
    body.rows[0].comparison.index_consistency = syncAssessment(structuredClone(c), 'refused');
    const selected = researchSetting(body, '480');
    assert.equal(selected.consistency.state, 'passed'); assert.equal(selected.consistency.combined_qualifies, false);
    assert.equal(selected.consistencyContext.institutional, 'refused'); assert.equal(selected.row.index_consistency.combined_qualifies, true);
    body.rows[0].comparison.index_consistency.receipt.identity = id(88);
    assert.equal(selected.consistency.receipt.identity, c.receipt.identity);
    assert.throws(() => researchSetting(body, '480'), /changed the original saved/);
  }
  const missing = detail(); missing.projection_version = 4; missing.timeframes = ['60min'];
  assert.throws(() => researchSetting(missing, '480'), /required saved index-policy assessment/);
});

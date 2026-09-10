import test from 'node:test';
import assert from 'node:assert/strict';
import { loadSettingFacts, assertTradeSetting } from '../saved-backtest/setting-pages.js';
import { fetchBooleanLater } from '../src/lib/boolean-oos.js';
import { fetchBooleanCatalog } from '../src/lib/boolean-catalog.js';

const id = (/** @type {number} */ n) => n.toString(16).padStart(64, '0');
/** Generated join-only fixture. The injected fetcher replaces transport and
 * shared wire validation; these tests prove the additional setting/source join.
 * @param {number} index @param {number} coordinate @param {number} [family] */
function setting(index, coordinate, family = 1) {
  return { index: String(index), statistics: {
    index: String(index),
    coordinate: String(coordinate), identity: id(1000 + index), side: 'short', run: id(2000 + family), program_index: '1', ordinal: String(coordinate),
    trades: '1395', wins: '278', return_paisa: '-159340', execution_refusal_bits: '2',
    source: { identity: id(family), completion: id(10 + family), instrument: `FIXTURE-${family}`, cash: true,
      index: String(family), coordinates: '18446744073709551615', membership_digest: id(40 + family) }
  }, qualification: { coordinate: String(coordinate), identity: id(3000 + index), source: { identity: id(20 + family), completion: id(30 + family) } } };
}
/** @param {any} asked @param {any[]} rows */
function page(asked, rows) {
  const selected = rows.filter(row => row.qualification.source.identity === asked.identity &&
    BigInt(row.statistics.coordinate) >= BigInt(asked.offset) && BigInt(row.statistics.coordinate) < BigInt(asked.offset) + BigInt(asked.limit));
  selected.sort((a, b) => Number(BigInt(a.statistics.coordinate) - BigInt(b.statistics.coordinate)));
  const first = selected[0].statistics;
  return { ...asked, parent: first.source, instrument: first.source.instrument, cash: true,
    coordinate_count: first.source.coordinates, membership_digest: first.source.membership_digest,
    later: { from: '2025-06', to: '2025-08' }, grids: [{ side: 'long' }, { side: 'short', horizon_bars: '5' }],
    rows: selected.map(row => ({ index: row.statistics.coordinate,
      training: { ...row.statistics, index: row.statistics.coordinate, cell: { trades: row.statistics.trades, wins: row.statistics.wins, pessimistic: row.statistics.return_paisa } },
      later: { index: row.qualification.coordinate, identity: row.qualification.identity, side: row.statistics.side, cell: { trades: '0', pessimistic: '0' } }
    })) };
}
const unusedRequest = async () => { throw new Error('The injected fixture must not make HTTP requests'); };

test('visible coordinates group by pinned family, split gaps, and fetch sequentially', async () => {
  const rows = [setting(480, 144), setting(481, 145), setting(484, 148), setting(500, 0, 2)];
  /** @type {any[]} */ const asked = [];
  let active = 0, maximum = 0;
  const result = await loadSettingFacts(rows, unusedRequest, new AbortController().signal, async selection => {
    asked.push(selection); active += 1; maximum = Math.max(maximum, active);
    await new Promise(resolve => setImmediate(resolve));
    active -= 1; return page(selection, rows);
  });
  assert.equal(maximum, 1);
  assert.deepEqual(asked.map(row => [row.offset, row.limit]), [['144', 2], ['148', 1], ['0', 1]]);
  assert.equal(result.size, 4);
  assert.equal(result.get('480')?.training.cell.trades, '1395');
  assert.equal(result.get('480')?.later.cell.trades, '0');
  assert.equal(result.get('480')?.grid.side, 'short');
  assert.equal(result.get('500')?.laterPeriod.to, '2025-08');
  assert.deepEqual(rows.map(row => row.index), ['480', '481', '484', '500']);
});

test('empty pages need no requests, oversize and duplicate settings refuse before transport', async () => {
  let calls = 0;
  const fetcher = async () => { calls += 1; throw new Error('not expected'); };
  assert.equal((await loadSettingFacts([], unusedRequest, new AbortController().signal, fetcher)).size, 0);
  for (const rows of [Array.from({ length: 33 }, (_, i) => setting(i, i)), [setting(1, 1), setting(1, 2)]]) {
    await assert.rejects(loadSettingFacts(rows, unusedRequest, new AbortController().signal, fetcher), /limited to 32|ambiguous or incomplete/);
  }
  assert.equal(calls, 0);
});

test('a mismatch in any exact ancestor, coordinate or training fact refuses the whole map', async () => {
  /** @type {((body:any)=>void)[]} */
  const damage = [
    body => { body.parent = { ...body.parent, identity: id(99) }; },
    body => { body.parent = { ...body.parent, completion: id(99) }; },
    body => { body.instrument = 'FOREIGN'; }, body => { body.cash = false; },
    body => { body.membership_digest = id(99); }, body => { body.coordinate_count = '1'; },
    body => { body.rows[0].index = '99'; }, body => { body.rows[0].training.index = '99'; },
    body => { body.rows[0].later.index = '99'; }, body => { body.rows[0].training.identity = id(99); },
    body => { body.rows[0].later.identity = id(99); }, body => { body.rows[0].training.side = 'long'; },
    body => { body.rows[0].later.side = 'long'; }, body => { body.rows[0].training.run = id(99); },
    body => { body.rows[0].training.program_index = '0'; }, body => { body.rows[0].training.ordinal = '99'; },
    body => { body.rows[0].training.execution_refusal_bits = '0'; },
    body => { body.rows[0].training.cell.trades = '0'; }, body => { body.rows[0].training.cell.wins = '0'; },
    body => { body.rows[0].training.cell.pessimistic = '0'; }, body => { body.grids = [{ side: 'long' }]; }
  ];
  for (const mutate of damage) {
    const rows = [setting(480, 144)];
    await assert.rejects(loadSettingFacts(rows, unusedRequest, new AbortController().signal, async asked => {
      const body = page(asked, rows); mutate(body); return body;
    }), /does not match/);
  }
});

test('a wrong response pin or partial page cannot provide a partial successful map', async () => {
  const rows = [setting(480, 144), setting(481, 145)];
  for (const change of ['completion', 'identity', 'offset', 'limit', 'rows']) {
    await assert.rejects(loadSettingFacts(rows, unusedRequest, new AbortController().signal, async asked => {
      const body = page(asked, rows);
      if (change === 'rows') body.rows.pop(); else Object.assign(body, { [change]: 'wrong' });
      return body;
    }), /differs from its exact requested source or span/);
  }
});

test('cancellation before transport or after a resolving read never publishes', async () => {
  const rows = [setting(480, 144), setting(484, 148)];
  const cancelled = new AbortController(); cancelled.abort();
  await assert.rejects(loadSettingFacts(rows, unusedRequest, cancelled.signal), { name: 'AbortError' });
  const pending = new AbortController(); let calls = 0;
  await assert.rejects(loadSettingFacts(rows, unusedRequest, pending.signal, async asked => {
    calls += 1; pending.abort(); return page(asked, rows);
  }), { name: 'AbortError' });
  assert.equal(calls, 1, 'the next span must never start after cancellation');
});

test('a later page failure does not publish earlier completed spans', async () => {
  const rows = [setting(480, 144), setting(484, 148)]; let calls = 0, published = false;
  await assert.rejects(loadSettingFacts(rows, unusedRequest, new AbortController().signal, async asked => {
    if (++calls === 2) throw new Error('saved ancestor changed');
    return page(asked, rows);
  }).then(value => { published = true; return value; }), /saved ancestor changed/);
  assert.equal(calls, 2); assert.equal(published, false);
});

test('duplicate local coordinates and conflicting ancestry refuse before any page read', async () => {
  const cases = [[setting(1, 0), setting(2, 2), setting(3, 2)]];
  for (const field of ['identity', 'completion', 'instrument', 'cash', 'index', 'coordinates', 'membership_digest']) {
    const rows = [setting(1, 0), setting(2, 1)];
    Object.assign(rows[1].statistics.source, { [field]: {
      identity: id(90), completion: id(91), instrument: 'ANOTHER-FIXTURE', cash: false,
      index: '7', coordinates: '100', membership_digest: id(92)
    }[/** @type {'identity'|'completion'|'instrument'|'cash'|'index'|'coordinates'|'membership_digest'} */(field)] });
    cases.push(rows);
  }
  let calls = 0;
  for (const rows of cases) {
    await assert.rejects(loadSettingFacts(rows, unusedRequest, new AbortController().signal, async () => {
      calls += 1; throw new Error('preflight should have refused');
    }), /same later coordinate|conflicting original ancestors/);
  }
  assert.equal(calls, 0);
});

test('missing or noncanonical join numbers cannot match other missing values', async () => {
  let calls = 0;
  /** @type {any[]} */ const bad = [undefined, null, '', 0, '-0', '01', '1.5', '1e3', '18446744073709551616'];
  for (const field of ['coordinate', 'program_index', 'ordinal', 'trades', 'wins', 'execution_refusal_bits', 'return_paisa']) {
    for (const value of bad) {
      const row = setting(1, 1);
      Object.assign(row.statistics, { [field]: value });
      if (field === 'coordinate') row.qualification.coordinate = value;
      await assert.rejects(loadSettingFacts([row], unusedRequest, new AbortController().signal, async () => {
        calls += 1; throw new Error('invalid numeric join reached transport');
      }), /ambiguous or incomplete/);
    }
  }
  const zero = setting(0, 0);
  Object.assign(zero.statistics, { trades: '0', wins: '0', return_paisa: '0', execution_refusal_bits: '0' });
  const result = await loadSettingFacts([zero], unusedRequest, new AbortController().signal, async asked => page(asked, [zero]));
  assert.equal(result.get('0')?.training.cell.trades, '0');
  assert.equal(result.get('0')?.training.cell.pessimistic, '0');
  assert.equal(calls, 0);
});

test('unsafe-number coordinates stay exact and input order cannot change the join', async () => {
  const rows = [setting(10, 0), setting(11, 0), setting(12, 0)];
  for (const [index, coordinate] of ['9007199254740994', '9007199254740992', '9007199254740993'].entries()) {
    rows[index].statistics.coordinate = coordinate;
    rows[index].qualification.coordinate = coordinate;
  }
  /** @type {any[]} */ const asked = [];
  const result = await loadSettingFacts(rows, unusedRequest, new AbortController().signal, async value => {
    asked.push(value); return page(value, rows);
  });
  assert.equal(asked.length, 1);
  assert.equal(asked[0].offset, '9007199254740992');
  assert.equal(asked[0].limit, 3);
  assert.equal(result.get('10')?.training.index, '9007199254740994');
  assert.deepEqual(rows.map(row => row.index), ['10', '11', '12']);
});

test('the visible-page cap is exact and sparse pages never request unseen coordinates', async () => {
  const rows = Array.from({ length: 32 }, (_, i) => setting(i, i * 2));
  /** @type {any[]} */ const asked = [];
  const result = await loadSettingFacts(rows, unusedRequest, new AbortController().signal, async value => {
    asked.push(value); return page(value, rows);
  });
  assert.equal(result.size, 32); assert.equal(asked.length, 32);
  assert.deepEqual(asked.map(value => value.offset), rows.map(row => row.statistics.coordinate));
  assert.ok(asked.every(value => value.kind === 'coordinates' && value.limit === 1));
});

test('caller edits during a read cannot rename a result or change its expected receipt', async () => {
  const rows = [setting(480, 144)];
  const original = structuredClone(rows);
  const result = await loadSettingFacts(rows, unusedRequest, new AbortController().signal, async asked => {
    rows[0].index = '999'; rows[0].statistics.source.completion = id(99);
    rows[0].statistics.trades = '0'; rows[0].qualification.identity = id(98);
    return page(asked, original);
  });
  assert.equal(result.has('999'), false);
  assert.equal(result.get('480')?.training.cell.trades, '1395');
  assert.equal(result.get('480')?.later.identity, original[0].qualification.identity);
});

/** Generated HTTP-contract fixture. It passes through the production later
 * validator; it is not a market-data or strategy-profitability fixture.
 * @returns {{ rows: any[], body: any }} */
function wireFixture() {
  const parent = { identity: id(50), completion: id(51) };
  const identity = id(52), completion = id(53), membership = id(54);
  /** @param {string} side */
  const grid = side => ({
    side, resolution: id(55), execution: id(56), feed: id(57), commit: id(58), calendar: id(59), cost_model: id(60),
    bars: '2', cells: '1', horizon_bars: '5', stop_count: '1', target_count: '1', trail_count: '1',
    requested_stop_count: '1', requested_target_count: '1', requested_trail_count: '1', max_pairs: '100',
    max_cells: '100', max_levels_per_axis: '1', max_ambiguous_bars: '0', max_gap_fills: '0',
    first_micros: '0', last_micros: '60000000', ratio_min_hundredths: '100', ratio_max_hundredths: '10000',
    execution_seconds: '60', range_rounding: 'floor', selector: 'pessimistic-total', forced_stop: { kind: 'disabled', ppm: null }
  });
  const pairs = ['long', 'short'].map((side, index) => {
    const training = {
      index: String(index), identity: id(100 + index), run: id(110 + index), program_index: '0', ordinal: '0', side,
      expression: '(0 | !(1))', support_sessions: '1', execution_refusal_bits: '0', execution_refusals: [],
      truth: { evaluated: '2', hits: '1', misses: '0', unknown: '1' },
      cell: { trades: '1', wins: '1', pessimistic: '10', optimistic: '12', fill_cost: '0', stopped: '0', targeted: '0',
        trailed_stop: '0', trailed_profit: '0', timed_out: '1', ambiguous_bars: '0', gapped: '0', stop: null, target: null, tsl: null, ttp: null },
      levels: { stop_ppm: null, target_ppm: null, tsl_ppm: null, ttp_arm_ppm: null, ttp_trail_ppm: null }
    };
    const later = structuredClone(training);
    later.identity = id(120 + index); later.run = id(130 + index);
    later.cell.pessimistic = '20'; later.cell.optimistic = '22';
    return { index: String(index), training, later };
  });
  const rows = pairs.map((pair, index) => ({
    index: String(480 + index), statistics: {
      index: String(480 + index), coordinate: pair.index, identity: pair.training.identity, run: pair.training.run,
      program_index: '0', ordinal: '0', side: pair.training.side, trades: '1', wins: '1', return_paisa: '10', execution_refusal_bits: '0',
      source: { ...parent, index: '0', coordinates: '2', instrument: 'FIXTURE-WIRE', cash: false, membership_digest: membership }
    }, qualification: { coordinate: pair.index, identity: pair.later.identity, source: { identity, completion } }
  }));
  return { rows, body: {
    schema_version: 1, status: 'saved', authority: 'authenticated-later-comparison-observation', identity, completion,
    parent, cohort: id(61), instrument: 'FIXTURE-WIRE', cash: false, membership_digest: membership,
    program_count: '1', coordinate_count: '2', training_session_count: '1', session_count: '1',
    grids: [grid('long'), grid('short')],
    later: { source: id(62), execution: id(63), first_micros: '99900000000', last_micros: '100440000000', bars: '10', from: '1970-01', to: '1970-01' },
    kind: 'coordinates', candidate: null, offset: '0', limit: 2, total: '2', next: null, page_complete: true,
    selected: null, rows: pairs, admitted_bytes: '10000', refusal: null, scope: 'Generated transport-validation fixture'
  } };
}

test('the production fetch path validates the paired wire page before joining both directions', async () => {
  const { rows, body } = wireFixture(); let calls = 0;
  const result = await loadSettingFacts(rows, async url => {
    calls += 1;
    const parsed = new URL(url, 'http://example.invalid');
    assert.equal(parsed.pathname, '/boolean-oos.json');
    assert.equal(parsed.searchParams.get('identity'), body.identity);
    assert.equal(parsed.searchParams.get('completion'), body.completion);
    assert.equal(parsed.searchParams.get('kind'), 'coordinates');
    assert.equal(parsed.searchParams.get('offset'), '0');
    assert.equal(parsed.searchParams.get('limit'), '2');
    return { ok: true, json: async () => body };
  }, new AbortController().signal);
  assert.equal(calls, 1); assert.equal(result.size, 2);
  assert.equal(result.get('480')?.grid.side, 'long');
  assert.equal(result.get('481')?.grid.side, 'short');
  assert.equal(result.get('481')?.later.cell.pessimistic, '20');
});

test('production wire validation refuses changed receipts, swapped identities and malformed later numbers', async () => {
  /** @type {((body:any)=>void)[]} */
  const damage = [
    body => { body.identity = body.completion; }, body => { body.completion = body.parent.completion; },
    body => { body.parent.identity = body.identity; }, body => { body.parent.completion = body.completion; },
    body => { body.rows.reverse(); }, body => { body.rows[0].training.identity = body.rows[0].later.identity; },
    body => { body.rows[0].later.identity = body.rows[0].training.identity; },
    body => { body.rows[0].later.cell.trades = 1; }, body => { body.rows[0].later.cell.trades = undefined; },
    body => { body.rows[0].later.cell.pessimistic = null; }, body => { body.rows[0].later.cell.pessimistic = '9223372036854775808'; },
    body => { body.rows[0].later.expression = '1'; }, body => { body.rows[0].later.levels.stop_ppm = '1'; },
    body => { body.page_complete = false; }, body => { body.next = '2'; },
    body => { body.rows[1].later.cell.optimistic = '-1'; }
  ];
  for (const mutate of damage) {
    const { rows, body } = wireFixture(); mutate(body);
    await assert.rejects(loadSettingFacts(rows, async () => ({ ok: true, json: async () => body }), new AbortController().signal));
  }
  for (const status of [429, 503]) {
    await assert.rejects(loadSettingFacts(wireFixture().rows, async () => ({ ok: false, status }), new AbortController().signal), new RegExp(String(status)));
  }
});

/** @param {boolean} isLater @param {'trades'|'sessions'} [kind] */
async function tradeFixture(isLater, kind = 'trades') {
  const wire = wireFixture();
  const facts = await loadSettingFacts(wire.rows, async () => ({ ok: true, json: async () => wire.body }), new AbortController().signal);
  const row = wire.rows[1], fact = facts.get(row.index);
  assert.ok(fact, 'the fixture setting must pass the real visible-page join');
  const body = structuredClone(wire.body), pair = body.rows[1];
  const source = isLater ? row.qualification.source : row.statistics.source;
  Object.assign(body, { identity: source.identity, completion: source.completion, kind, candidate: '1', limit: 32, total: '1', next: null,
    selected: isLater ? pair : pair.training,
    rows: kind === 'sessions' ? [{ index: '0', day: isLater ? '1' : '0', trades: '1', wins: '1', return_paisa: isLater ? '20' : '10' }]
      : [{ index: '0', signal_bar: '0', entry_bar: '0', exit_bar: '1', best: isLater ? '22' : '12', worst: isLater ? '20' : '10',
        entry_micros: isLater ? '99960000000' : '0', exit_micros: isLater ? '100020000000' : '60000000',
        adverse_ppm: '0', favourable_ppm: '1', adverse_paisa: '0', favourable_paisa: '1' }]
  });
  if (!isLater) Object.assign(body, { authority: 'authenticated-catalog-observation', side: null, axis: null });
  return { row, fact, body };
}

test('trade and zero-inclusive session pages match the exact displayed setting in both periods', async () => {
  for (const isLater of [false, true]) for (const kind of /** @type {const} */(['trades', 'sessions'])) {
    const { row, fact, body } = await tradeFixture(isLater, kind);
    const source = isLater ? row.qualification.source : row.statistics.source;
    const read = await (isLater ? fetchBooleanLater : fetchBooleanCatalog)({ ...source, kind, candidate: '1', offset: '0', limit: 32 }, async () => ({ ok: true, json: async () => body }));
    const before = structuredClone(read);
    assert.equal(assertTradeSetting(read, row, isLater, fact), read);
    assert.deepEqual(read, before, 'the final join must not mutate the saved response');
  }
});

test('trade detail rejects foreign source pins, parents, grid identities and selected coordinates', async () => {
  /** @type {((body:any)=>void)[]} */
  const damage = [
    body => { body.identity = id(999); }, body => { body.completion = id(998); }, body => { body.candidate = '0'; },
    body => { body.instrument = 'FOREIGN'; }, body => { body.cash = true; }, body => { body.membership_digest = id(997); },
    body => { body.coordinate_count = '3'; }, body => { body.grids.reverse(); }, body => { body.grids[0].commit = id(996); },
    body => { body.grids[1].forced_stop.kind = 'include-exact-observed'; }, body => { body.kind = 'coordinates'; },
    body => { body.parent.identity = id(995); }, body => { body.parent.completion = id(994); },
    body => { body.selected.index = '0'; }, body => { body.later.source = id(993); }, body => { body.later.from = '1970-02'; }
  ];
  for (const mutate of damage) {
    const { row, fact, body } = await tradeFixture(true); mutate(body);
    assert.throws(() => assertTradeSetting(body, row, true, fact), /different saved source|lost its exact original/);
  }
});

test('returned trade coordinates cannot change either period identity, expression, exits or measured totals', async () => {
  /** @type {((coordinate:any)=>void)[]} */
  const damage = [
    value => { value.identity = id(990); }, value => { value.run = id(989); }, value => { value.index = '0'; },
    value => { value.side = 'long'; }, value => { value.program_index = '1'; }, value => { value.ordinal = '1'; },
    value => { value.expression = '1'; }, value => { value.support_sessions = '0'; },
    value => { value.execution_refusal_bits = '2'; }, value => { value.execution_refusals = ['Missing target']; },
    value => { value.cell.trades = '0'; }, value => { value.cell.wins = '0'; }, value => { value.cell.pessimistic = '-1'; },
    value => { value.cell.optimistic = '99'; }, value => { value.cell.ttp = { arm: '0', trail: '0' }; },
    value => { value.cell.stop = '0'; }, value => { value.levels.stop_ppm = '13244'; }, value => { value.truth.unknown = '0'; }
  ];
  for (const isLater of [false, true]) for (const period of isLater ? ['training', 'later'] : ['training']) for (const mutate of damage) {
    const { row, fact, body } = await tradeFixture(isLater);
    mutate(isLater ? body.selected[period] : body.selected);
    assert.throws(() => assertTradeSetting(body, row, isLater, fact), /lost its exact original|differs from the visible original/);
  }
});

test('missing or mismatched visible facts and a switched period cannot approve a trade response', async () => {
  const { row, fact, body } = await tradeFixture(true);
  for (const missing of [null, undefined, {}, { training: fact?.training }, { ...fact, grids: [] }]) {
    assert.throws(() => assertTradeSetting(body, row, true, missing), /Load the exact setting comparison/);
  }
  const foreign = structuredClone(fact); foreign.training.identity = id(991);
  assert.throws(() => assertTradeSetting(body, row, true, foreign), /differs from its visible/);
  assert.throws(() => assertTradeSetting(body, { ...row, index: '999' }, true, fact), /differs from its visible/);
  assert.throws(() => assertTradeSetting(body, row, false, fact), /different saved source/);
});

test('matching selected zero-trade evidence stays zero without substituting a missing result', async () => {
  const { row, fact, body } = await tradeFixture(true);
  const zero = { trades: '0', wins: '0', pessimistic: '0', optimistic: '0', fill_cost: '0', timed_out: '0' };
  Object.assign(fact?.later.cell, zero); Object.assign(body.selected.later.cell, zero);
  Object.assign(body, { total: '0', rows: [] });
  const checked = await fetchBooleanLater({ identity: body.identity, completion: body.completion, kind: 'trades', candidate: '1', offset: '0', limit: 32 }, async () => ({ ok: true, json: async () => body }));
  assert.equal(assertTradeSetting(checked, row, true, fact).selected.later.cell.trades, '0');
  body.selected.later.cell.pessimistic = undefined;
  assert.throws(() => assertTradeSetting(body, row, true, fact), /lost its exact original/);
});

test('enabled trailing exits and recorded refusals must match without relying on object key order', async () => {
  const { row, fact, body } = await tradeFixture(true);
  for (const value of [fact.training, fact.later, body.selected.training, body.selected.later]) {
    value.execution_refusal_bits = '2'; value.execution_refusals = ['Missing target'];
    Object.assign(value.cell, { stop: '0', tsl: '0', ttp: { arm: '0', trail: '0' } });
    Object.assign(value.levels, { stop_ppm: '13244', tsl_ppm: '22214', ttp_arm_ppm: '15325', ttp_trail_ppm: '693' });
  }
  row.statistics.execution_refusal_bits = '2';
  body.selected.later.cell.ttp = { trail: '0', arm: '0' };
  body.grids = body.grids.map((/** @type {any} */ grid) => Object.fromEntries(Object.entries(grid).reverse()));
  assert.equal(assertTradeSetting(body, row, true, fact), body);
  body.selected.later.cell.ttp.arm = '1';
  assert.throws(() => assertTradeSetting(body, row, true, fact), /lost its exact original/);
  body.selected.later.cell.ttp.arm = '0'; body.selected.later.execution_refusals = ['Missing stop'];
  assert.throws(() => assertTradeSetting(body, row, true, fact), /lost its exact original/);
});

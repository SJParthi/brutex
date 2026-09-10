import test from 'node:test';
import assert from 'node:assert/strict';
import { savedSelection, savedSettingIndex, latestRequest } from '../saved-backtest/selection.js';

const identity = 'a'.repeat(64), pin = 'b'.repeat(64);
test('saved viewer uses exact URL identity and forbids browser-selected roots and ambiguous queries', () => {
  assert.deepEqual(savedSelection('?identity=' + identity), { identity, pin: null, batch: null, rung: '0', offset: '0', setting: null });
  assert.equal(savedSelection(`?identity=${identity}&pin=${pin}&batch=1&rung=7&offset=480&setting=503`).setting, '503');
  for (const tail of ['&store=/tmp', '&identity=' + identity, '&rung=8', '&batch=-1', '&offset=18446744073709551616', '&setting=32', '&offset=1e2']) {
    assert.throws(() => savedSelection('?identity=' + identity + tail));
  }
  assert.throws(() => savedSelection('?identity=missing'));
});
test('late saved-result requests cannot replace newer selection or survive disposal', async () => {
  /** @type {any[]} */ const states = [];
  /** @type {(value:any)=>void} */ let resolveOld = () => { throw new Error('Old request missing'); };
  /** @type {(value:any)=>void} */ let resolveNew = () => { throw new Error('New request missing'); };
  /** @type {{signal?:AbortSignal}} */ const observed = {};
  const owned = latestRequest(state => states.push(state));
  const old = owned.run(signal => { observed.signal = signal; return new Promise(resolve => resolveOld = resolve); });
  const current = owned.run(() => new Promise(resolve => resolveNew = resolve));
  assert.equal(observed.signal?.aborted, true);
  resolveNew('new'); await current;
  resolveOld('old'); await old;
  assert.equal(states.at(-1).body, 'new');
  /** @type {(value:any)=>void} */ let release = () => { throw new Error('Closing request missing'); };
  const closing = owned.run(() => new Promise(resolve => release = resolve));
  owned.stop(); const before = states.length;
  release('closed'); await closing;
  assert.equal(states.length, before);
});
test('saved-result failures remain visible and never become empty success', async () => {
  let state; const owned = latestRequest(value => state = value);
  await owned.run(async () => { throw new Error('wrong receipt'); });
  assert.deepEqual(state, { phase: 'failed', body: null, why: 'wrong receipt' });
});
test('an exact result in the final short page must exist before it is shown as opened', () => {
  const rows = Array.from({ length: 24 }, (_, index) => ({ index: String(480 + index) }));
  assert.equal(savedSettingIndex(rows, null), -1);
  assert.equal(savedSettingIndex(rows, '480'), 0);
  assert.equal(savedSettingIndex(rows, '503'), 23);
  const selected = savedSelection(`?identity=${identity}&offset=480&setting=509`);
  assert.throws(() => savedSettingIndex(rows, selected.setting), /Setting 509 is absent.*not opened/);
  assert.throws(() => savedSettingIndex([], '0'), /Setting 0 is absent/);
  assert.equal(savedSettingIndex([{ index: '0' }], '0'), 0);
});

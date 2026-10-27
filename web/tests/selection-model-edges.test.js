// R3-12 merge-back (25 Oct 2026): THE EDGES OF `selection-model.js` the merged
// tree's coverage named unrun (`node --test --experimental-test-coverage`,
// lcov BRDA count 0): a missing segment key, a spot row with no short count,
// an offer with nothing held, an empty segment list, a feed named only by its
// wire word, a calendar that does not reach or marks a day outside, a held
// minute before, after and off the rung's grid, a bound that is not text, a
// segment word no picker names, a key read with no key, and a future named by
// its key alone. Every name and number is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { contractRungsOf, foldInstruments, defaultInstrument, defaultSegment, DEFAULT_SEGMENT, eraOf, dayCode, sessionGrid,
  timeBound, rebound, offSegment, keyConflict, linkedContract, linkOfKey, epochDayOf } from '../src/lib/db/selection-model.js';

test('a segment key that is no text has no contract rungs', () => {
  const none = { expiry: false, strike: false, money: false, side: false };
  assert.deepEqual(contractRungsOf(/** @type {any} */ (null)), none);
  assert.deepEqual(contractRungsOf(/** @type {any} */ (undefined)), none);
});

test('a spot row that names no short count adds none to its instrument', () => {
  const by = foldInstruments([{ underlying: 'ALPHA', rows: 5, expiry: null }, { underlying: 'ALPHA', rows: 2, expiry: undefined, short: 3 }]);
  const alpha = by.get('ALPHA');
  assert.ok(alpha);
  assert.deepEqual([alpha.months, alpha.bars, alpha.short, alpha.spotMonths, alpha.contractMonths], [2, 7, 3, 2, 0]);
});

test('the opening instrument and segment fall back in their stated order, to the first offered and to Spot', () => {
  assert.equal(defaultInstrument([{ key: 'BETA', months: 0 }, { key: 'GAMMA', months: 0 }]), 'BETA', 'nothing held: the first offered');
  assert.equal(defaultInstrument([{ key: 'BETA', months: 0 }, { key: 'GAMMA', months: 4 }]), 'GAMMA', 'the first held');
  assert.equal(defaultSegment([]), DEFAULT_SEGMENT, 'no segment row: Spot');
  assert.equal(defaultSegment([{ key: 'options', why: 'nothing held' }]), 'options', 'nothing held: the first row');
});

test('a feed\'s archive is labelled by its wire word when it names no display', () => {
  assert.equal(eraOf({ wire: 'example', archive: { from: '2018-09-03', to: '2026-09-24' } }).label, "example's archive");
});

test('a day the calendar does not reach, a day that is not one, and a day outside are no code', () => {
  const cal = { from: epochDayOf('2019-01-01'), days: 'R-.' };
  assert.equal(dayCode(null, '2019-01-01'), null);
  assert.equal(dayCode(cal, '1 Jan 2019'), null);
  assert.equal(dayCode(cal, '2019-01-01'), 'R');
  assert.equal(dayCode(cal, '2019-01-02'), null, "'-' is outside the calendar");
  assert.equal(dayCode(cal, '2019-01-03'), '.');
});

test('a held minute before the session, after it and off the rung\'s grid is listed and named', () => {
  const grid = sessionGrid(300, ['09:00', '15:35', '09:16', 'xx:yy', '09:15']);
  const of = (/** @type {string} */ t) => grid.find((row) => row.key === t);
  assert.equal(of('09:00')?.detail, 'before 09:15');
  assert.equal(of('15:35')?.detail, 'after 15:30');
  assert.match(String(of('15:35')?.why), /such as a closing-auction print/);
  assert.equal(of('09:16')?.detail, 'held');
  assert.equal(of('09:16')?.why, 'Off this rung’s grid; listed because a loaded bar opens here.');
  assert.equal(of('xx:yy'), undefined, 'a held minute that is no time is not listed');
  assert.deepEqual(grid.map((row) => row.key), [...grid.map((row) => row.key)].sort(), 'in time order');
  const derivative = sessionGrid(60, ['15:45'], { venue: 'derivative', lo: '2026-08-03', hi: '2026-09-01' });
  assert.equal(derivative.find((row) => row.key === '15:45')?.detail, 'after 15:40');
  assert.doesNotMatch(String(derivative.find((row) => row.key === '15:45')?.why), /closing-auction/);
});

test('a bound that is no text is no bound', () => {
  assert.equal(timeBound(/** @type {any} */ (null), 60, 'from'), '');
  assert.equal(timeBound(/** @type {any} */ (undefined), 1, 'to'), '');
  assert.equal(rebound(/** @type {any} */ (null), 1, 'from'), '');
  assert.equal(rebound(/** @type {any} */ (undefined), 60, 'to'), '');
  assert.equal(rebound('09:15:07', 1, 'from'), '09:15:07');
});

test('a segment word no picker names is said as itself when the rungs it lacks are dropped', () => {
  const off = offSegment('weird', { exp: '2019-03-28', strikes: [], mny: [], side: undefined });
  assert.equal(off.exp, true);
  assert.equal(off.why, 'exp=2019-03-28 is not applied: weird (seg=weird) has no such rung');
});

test('a key conflict read with no key in the selection says the empty key as it is', () => {
  const link = linkOfKey('NSE-INDEX-ALPHA');
  assert.ok(link);
  assert.match(String(keyConflict({ sym: 'BETA' }, link)), /^key= and sym=BETA \(the key's is ALPHA\)/);
});

test('a future named by its key alone is filed by its own expiry', () => {
  const link = linkedContract({ key: 'NSE-FNO-ALPHA-2019-03-28-FUT' }, '2026-10-25', '2026-09-24');
  assert.equal(link.sym, 'ALPHA');
  assert.equal(link.exp, '2019-03-28');
  assert.equal(link.seg, 'futures');
  assert.deepEqual(link.notes, []);
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  SPOT_CHUNK, SPOT_WHY, chainsFrom, chainKey, intrinsic, moneyness, optionCells, readSpots, rungOf, spotKey
} from '../src/lib/moneyness.js';

// Generated shapes only; no row here is market data. D-2807.
const K = (strike, side, expiry = '2026-09-29', u = 'NIFTY') => `NSE-FNO-${u}-${expiry}-${strike}-${side}`;
const chain = [2400000, 2405000, 2410000, 2415000, 2420000];

test('rungs count listed strikes from the one nearest spot, and the sign is the side', () => {
  assert.deepEqual(moneyness(chain, 2410000, 'CE', 2410000), { rung: 'ATM', why: null });
  assert.deepEqual(moneyness(chain, 2400000, 'CE', 2410000), { rung: 'ITM-2', why: null });
  assert.deepEqual(moneyness(chain, 2400000, 'PE', 2410000), { rung: 'OTM+2', why: null });
  assert.deepEqual(moneyness(chain, 2420000, 'CE', 2410000), { rung: 'OTM+2', why: null });
  assert.deepEqual(moneyness(chain, 2420000, 'PE', 2410000), { rung: 'ITM-2', why: null });
  // Equidistant: the LOWER strike is the money, every read alike.
  assert.deepEqual(moneyness(chain, 2405000, 'CE', 2407500), { rung: 'ATM', why: null });
  assert.deepEqual(moneyness(chain, 2410000, 'CE', 2407500), { rung: 'OTM+1', why: null });
  assert.deepEqual(moneyness(chain, 2410000, 'CE', 2407501), { rung: 'ATM', why: null });
  assert.equal(rungOf(0, 'PE'), 'ATM');
  assert.equal(rungOf(-3, 'PE'), 'OTM+3');
  assert.equal(rungOf(3, 'PE'), 'ITM-3');
});

test('no rung is named without a ladder, for an unlisted strike, or beyond the chain', () => {
  assert.deepEqual(moneyness([2400000], 2400000, 'CE', 2400000), { rung: null, why: 'one_strike_listed' });
  assert.deepEqual(moneyness(chain, 2401000, 'CE', 2410000), { rung: null, why: 'strike_not_listed' });
  // Half the smallest gap past either end is still the end strike's money.
  assert.deepEqual(moneyness(chain, 2400000, 'CE', 2397500), { rung: 'ATM', why: null });
  assert.deepEqual(moneyness(chain, 2420000, 'CE', 2422500), { rung: 'ATM', why: null });
  assert.deepEqual(moneyness(chain, 2400000, 'CE', 2397499), { rung: null, why: 'spot_outside_chain' });
  assert.deepEqual(moneyness(chain, 2420000, 'CE', 2422501), { rung: null, why: 'spot_outside_chain' });
  // The step is the SMALLEST listed gap, not the first.
  const wide = [2300000, 2400000, 2405000];
  assert.deepEqual(moneyness(wide, 2405000, 'CE', 2407501), { rung: null, why: 'spot_outside_chain' });
  for (const why of ['one_strike_listed', 'strike_not_listed', 'spot_outside_chain', 'no_spot_bar',
    'no_spot_file', 'spot_unreadable', 'spot_order_broken', 'spot_request_failed', 'spot_pending']) {
    assert.equal(typeof SPOT_WHY[why], 'string', why);
  }
});

test('intrinsic is clipped at zero per side and extrinsic is the premium left over, unclipped', () => {
  assert.equal(intrinsic(2400000, 'CE', 2410000), 10000);
  assert.equal(intrinsic(2420000, 'CE', 2410000), 0);
  assert.equal(intrinsic(2420000, 'PE', 2410000), 10000);
  assert.equal(intrinsic(2400000, 'PE', 2410000), 0);
  const chains = chainsFrom(chain.map((s) => ({ instrument: K(s, 'CE') })));
  const row = { instrument: K(2400000, 'CE'), strike: 2400000, side: /** @type {'CE'} */ ('CE'), c: 9000 };
  assert.deepEqual(optionCells(row, { close: 2410000, why: null }, chains),
    { mny: 'ITM-2', mnyWhy: null, intr: 10000, extr: -1000, why: null, spot: 2410000, detail: null });
});

test('a missing spot is the named reason in every cell, never a figure', () => {
  const chains = chainsFrom(chain.map((s) => ({ instrument: K(s, 'PE') })));
  const row = { instrument: K(2410000, 'PE'), strike: 2410000, side: /** @type {'PE'} */ ('PE'), c: 5000 };
  assert.deepEqual(optionCells(row, undefined, chains),
    { mny: null, mnyWhy: 'spot_pending', intr: null, extr: null, why: 'spot_pending', spot: null, detail: null });
  assert.deepEqual(optionCells(row, { close: null, why: 'no_spot_bar' }, chains),
    { mny: null, mnyWhy: 'no_spot_bar', intr: null, extr: null, why: 'no_spot_bar', spot: null, detail: null });
  assert.deepEqual(optionCells(row, { close: null, why: null, detail: 'boom' }, chains),
    { mny: null, mnyWhy: 'spot_request_failed', intr: null, extr: null, why: 'spot_request_failed', spot: null, detail: 'boom' });
  assert.equal(optionCells({ instrument: 'NSE-INDEX-NIFTY', strike: null, side: null, c: 1 }, undefined, chains), null);
  // A different expiry's chain is not this option's ladder.
  const other = chainsFrom([{ instrument: K(2410000, 'PE', '2026-10-27') }, { instrument: K(2415000, 'PE', '2026-10-27') }]);
  assert.equal(optionCells(row, { close: 2410000, why: null }, other)?.mnyWhy, 'strike_not_listed');
});

test('chains are per underlying and expiry, distinct and ascending, and skip non-options', () => {
  const chains = chainsFrom([
    { instrument: K(2410000, 'CE') }, { instrument: K(2400000, 'PE') }, { instrument: K(2410000, 'PE') },
    { instrument: K(150000, 'CE', '2026-09-29', 'RELIANCE') }, { instrument: 'NSE-INDEX-NIFTY' },
    { instrument: 'NSE-FNO-NIFTY-2026-09-29-FUT' }
  ]);
  assert.deepEqual([...chains.keys()].sort(), [chainKey('NSE', 'NIFTY', '2026-09-29'), chainKey('NSE', 'RELIANCE', '2026-09-29')].sort());
  assert.deepEqual(chains.get(chainKey('NSE', 'NIFTY', '2026-09-29')), [2400000, 2410000]);
});

test('spot is asked once per underlying, rung and month, in chunks, and each instant filed exactly', async () => {
  /** @type {string[]} */
  const asked = [];
  /** @param {string} url */
  const request = async (url) => {
    asked.push(url);
    const q = new URL(url, 'http://x').searchParams;
    const t = String(q.get('t')).split(',').map(Number);
    const symbol = q.get('symbol');
    return /** @type {any} */ ({ status: 200, json: async () => symbol === 'TCS'
      ? { error: 'refused by name' }
      : { faults: symbol === 'RELIANCE' ? 'record 3: damaged' : null,
          spots: t.map((s) => (s % 120 === 0 ? { t: s, close: s, why: null } : { t: s, close: null, why: 'no_spot_bar' })) } });
  };
  const base = 1777777920; // divisible by 120
  const rows = [
    ...Array.from({ length: SPOT_CHUNK + 1 }, (_, i) => ({ instrument: K(2400000, 'CE'), tf: '1min', month: '2026-05', ts: (base + 60 * i) * 1000, side: 'CE' })),
    { instrument: K(2400000, 'PE'), tf: '1min', month: '2026-05', ts: base * 1000, side: 'PE' },
    { instrument: K(150000, 'CE', '2026-05-28', 'RELIANCE'), tf: '1min', month: '2026-05', ts: base * 1000, side: 'CE' },
    { instrument: K(400000, 'CE', '2026-05-28', 'TCS'), tf: '1min', month: '2026-05', ts: base * 1000, side: 'CE' },
    { instrument: 'NSE-INDEX-NIFTY', tf: '1min', month: '2026-05', ts: base * 1000, side: null }
  ];
  const out = await readSpots('gdfl', rows, new AbortController().signal, request);
  assert.equal(asked.length, 4);
  assert.ok(asked.every((u) => u.startsWith('/spot.json?feed=gdfl&exchange=NSE&symbol=')));
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('t')?.split(',').length, SPOT_CHUNK);
  assert.equal(new URL(asked[1], 'http://x').searchParams.get('t')?.split(',').length, 1);
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('timeframe'), '1min');
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('month'), '2026-05');
  assert.deepEqual(out.get(spotKey(rows[0])), { close: base, why: null });
  assert.deepEqual(out.get(spotKey(rows[1])), { close: null, why: 'no_spot_bar' });
  assert.deepEqual(out.get(spotKey(rows[SPOT_CHUNK + 2])), { close: base, why: null, detail: 'record 3: damaged' });
  assert.deepEqual(out.get(spotKey(rows[SPOT_CHUNK + 3])), { close: null, why: 'spot_request_failed', detail: 'refused by name' });
  assert.equal(out.size, SPOT_CHUNK + 1 + 2);
  const thrown = await readSpots('gdfl', rows.slice(0, 1), new AbortController().signal,
    async () => { throw 'offline'; });
  assert.deepEqual([...thrown.values()], [{ close: null, why: 'spot_request_failed', detail: 'offline' }]);
  const status = await readSpots('gdfl', rows.slice(0, 1), new AbortController().signal,
    async () => /** @type {any} */ ({ status: 503, json: async () => ({}) }));
  assert.deepEqual([...status.values()], [{ close: null, why: 'spot_request_failed', detail: 'HTTP 503' }]);
  const gone = new AbortController();
  gone.abort();
  await assert.rejects(readSpots('gdfl', rows.slice(0, 1), gone.signal, request));
  assert.equal((await readSpots('gdfl', rows.slice(-1), new AbortController().signal, request)).size, 0);
});

test('the page draws the three columns from the spot read and only for options', () => {
  const page = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /readSpots\(feed, rows, controller\.signal\)/);
  assert.match(page, /optionCells\(b, spotMap\.get\(spotKey\(b\)\), spotChains\)/);
  assert.match(page, /const OPTION_COLS = new Set\(\['side', 'strike', 'mny', 'intr', 'extr'\]\)/);
  assert.match(page, /\.bgrid\.lean\.nooption > tbody > tr > td:nth-child\(n \+ 15\)/);
  assert.match(page, /\.bgrid\.lean > tbody > tr > td:nth-child\(n \+ 20\)/);
  assert.doesNotMatch(page, /none: NO_SPOT\b/);
});

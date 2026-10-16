import test from 'node:test';
import assert from 'node:assert/strict';
import { databaseSource, dbSource } from './database-page-fixture.js';
import {
  LADDER_CHUNK, RUNG_CHUNK, RUNG_WHY, SPOT_CHUNK, SPOT_WHY, intrinsic, ladderKey, optionCells, ownMonth,
  readRungs, readSpots, rungFrom, rungKey, rungOf, spotKey
} from '../src/lib/moneyness.js';

// Generated shapes and invented figures only; no row here is market data.
// D-2807, D-3328, D-3329.
const K = (/** @type {number} */ strike, /** @type {string} */ side, expiry = '2026-09-29', u = 'NIFTY') =>
  `NSE-FNO-${u}-${expiry}-${strike}-${side}`;
/** A bar row the way the grid builds one: `ts` in ms, `month` whatever it was filed under. */
/** @returns {any} */
const bar = (/** @type {string} */ instrument, /** @type {number} */ secs, month = '2099-01', c = 9000) => {
  const p = instrument.match(/-(\d+)-(CE|PE)$/);
  return { instrument, tf: '1s', month, ts: secs * 1000, side: p ? p[2] : null, strike: p ? Number(p[1]) : null, c };
};

// 2018-09-19 14:28:15 IST and 2018-10-03 10:00:00 IST, as unix seconds.
const SEP = 1537347495;
const OCT = 1538541000;

test('a bar is asked in its OWN IST month, whatever month it was filed under (DB-009)', async () => {
  assert.equal(ownMonth({ ts: SEP * 1000, month: '2019-03' }), '2018-09');
  assert.equal(ownMonth({ ts: OCT * 1000, month: '2019-03' }), '2018-10');
  // 18:29:59 UTC on the last day of a month is 23:59:59 IST, the same month;
  // a second later is 00:00:00 IST on the 1st, the next.
  assert.equal(ownMonth({ ts: Date.UTC(2018, 8, 30, 18, 29, 59) }), '2018-09');
  assert.equal(ownMonth({ ts: Date.UTC(2018, 8, 30, 18, 30, 0) }), '2018-10');
  const rows = [bar(K(1150000, 'CE', '2019-03-28'), SEP, '2019-03'), bar(K(1150000, 'CE', '2019-03-28'), OCT, '2019-03')];
  assert.notEqual(spotKey(rows[0]).split('|')[2], '2019-03');
  /** @type {string[]} */
  const asked = [];
  const request = async (/** @type {string} */ url) => {
    asked.push(url);
    const q = new URL(url, 'http://x').searchParams;
    const t = String(q.get('t')).split(',').map(Number);
    return /** @type {any} */ ({ status: 200, json: async () => ({ asof: 'exact', segment: 'INDEX', key: 'NIFTY', faults: null,
      spots: t.map((s) => ({ t: s, close: 1_000_000 + (s % 7), spot_t: s, spot_age_s: 0, why: null })) }) });
  };
  const out = await readSpots('gdfl', rows, new AbortController().signal, request);
  assert.deepEqual(asked.map((u) => new URL(u, 'http://x').searchParams.get('month')).sort(), ['2018-09', '2018-10']);
  assert.equal(out.get(spotKey(rows[0]))?.close, 1_000_000 + (SEP % 7));
  assert.equal(out.get(spotKey(rows[1]))?.close, 1_000_000 + (OCT % 7));
});

test('spot is asked once per underlying, rung and own month, in chunks, each instant filed exactly', async () => {
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
      : { asof: 'exact', segment: symbol === 'NIFTY' ? 'INDEX' : 'CASH', key: symbol, faults: symbol === 'RELIANCE' ? 'record 3: damaged' : null,
          spots: t.map((s) => (s % 120 === 0
            ? { t: s, close: s, spot_t: s, spot_age_s: 0, why: null }
            : { t: s, close: null, spot_t: null, spot_age_s: null, why: 'no_spot_bar' })) } });
  };
  const base = 1777777920; // divisible by 120; 2026-05-03 IST
  const rows = [
    ...Array.from({ length: SPOT_CHUNK + 1 }, (_, i) => ({ ...bar(K(2400000, 'CE'), base + 60 * i), tf: '1min' })),
    { ...bar(K(2400000, 'PE'), base), tf: '1min' },
    { ...bar(K(150000, 'CE', '2026-05-28', 'RELIANCE'), base), tf: '1min' },
    { ...bar(K(400000, 'CE', '2026-05-28', 'TCS'), base), tf: '1min' },
    { ...bar('NSE-INDEX-NIFTY', base), tf: '1min' }
  ];
  const out = await readSpots('gdfl', rows, new AbortController().signal, request);
  assert.equal(asked.length, 4);
  assert.ok(asked.every((u) => u.startsWith('/spot.json?feed=gdfl&exchange=NSE&symbol=')));
  assert.ok(asked.every((u) => !u.includes('asof=')), 'the exact second is the default and is not asked for by name');
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('t')?.split(',').length, SPOT_CHUNK);
  assert.equal(new URL(asked[1], 'http://x').searchParams.get('t')?.split(',').length, 1);
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('timeframe'), '1min');
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('month'), '2026-05');
  assert.deepEqual(out.get(spotKey(rows[0])), { close: base, why: null, spotT: base, age: 0, asof: 'exact', segment: 'INDEX', key: 'NIFTY' });
  assert.deepEqual(out.get(spotKey(rows[1])), { close: null, why: 'no_spot_bar', spotT: null, age: null, asof: 'exact', segment: 'INDEX', key: 'NIFTY' });
  assert.equal(out.get(spotKey(rows[SPOT_CHUNK + 2]))?.detail, 'record 3: damaged');
  assert.deepEqual(out.get(spotKey(rows[SPOT_CHUNK + 3])),
    { close: null, why: 'spot_request_failed', spotT: null, age: null, asof: 'exact', segment: null, key: null, detail: 'refused by name' });
  assert.equal(out.size, SPOT_CHUNK + 1 + 2);
  const thrown = await readSpots('gdfl', rows.slice(0, 1), new AbortController().signal, async () => { throw 'offline'; });
  assert.deepEqual([...thrown.values()].map((s) => [s.why, s.detail]), [['spot_request_failed', 'offline']]);
  const status = await readSpots('gdfl', rows.slice(0, 1), new AbortController().signal,
    async () => /** @type {any} */ ({ status: 503, json: async () => ({}) }));
  assert.deepEqual([...status.values()].map((s) => s.detail), ['HTTP 503']);
  const gone = new AbortController();
  gone.abort();
  await assert.rejects(readSpots('gdfl', rows.slice(0, 1), gone.signal, request));
  assert.equal((await readSpots('gdfl', rows.slice(-1), new AbortController().signal, request)).size, 0);
});

test('the opted-in earlier spot is asked by name and keeps its stamp and age (D-3328, DB-068)', async () => {
  /** @type {string[]} */
  const asked = [];
  const request = async (/** @type {string} */ url) => {
    asked.push(url);
    const t = String(new URL(url, 'http://x').searchParams.get('t')).split(',').map(Number);
    return /** @type {any} */ ({ status: 200, json: async () => ({ asof: 'prev', segment: 'INDEX', key: 'NIFTY', faults: null,
      spots: t.map((s) => (s % 2 === 0
        ? { t: s, close: 2_000_000, spot_t: s - 2, spot_age_s: 2, why: null }
        : { t: s, close: null, spot_t: null, spot_age_s: null, why: 'no_spot_in_session' })) }) });
  };
  const rows = [bar(K(2400000, 'CE'), 1790000000), bar(K(2400000, 'CE'), 1790000001)];
  const out = await readSpots('gdfl', rows, new AbortController().signal, request, 'prev');
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('asof'), 'prev');
  assert.deepEqual(out.get(spotKey(rows[0])), { close: 2_000_000, why: null, spotT: 1789999998, age: 2, asof: 'prev', segment: 'INDEX', key: 'NIFTY' });
  assert.equal(out.get(spotKey(rows[1]))?.why, 'no_spot_in_session');
  // The cells carry the age, so the page can draw it on every cell it priced.
  const oc = optionCells(rows[0], out.get(spotKey(rows[0])), new Map());
  assert.equal(oc?.age, 2);
  assert.equal(oc?.asof, 'prev');
  assert.equal(oc?.intr, 0);
});

/**
 * A `/chain.json` stand-in: every strike `strikes` is stored by `first[strike]`
 * (unix seconds), ATM is the stored strike nearest `spotAt(t)`, rungs named
 * the server's way. It answers both forms and refuses where told to.
 * @param {{strikes: number[], first: Record<number, number>, spotAt: (t: number) => number, refuse?: (t: number) => string|null}} m
 * @param {string[]} asked
 */
const chainServer = (m, asked) => async (/** @type {string} */ url) => {
  asked.push(url);
  const q = new URL(url, 'http://x').searchParams;
  const at = String(q.get('at')).split(',').map(Number);
  const strike = q.get('strike');
  const side = q.get('side');
  const chains = at.map((t) => {
    const stored = m.strikes.filter((s) => m.first[s] <= t);
    const spot = m.spotAt(t);
    let money = 0;
    for (let i = 1; i < stored.length; i += 1) if (Math.abs(stored[i] - spot) < Math.abs(stored[money] - spot)) money = i;
    const refusal = m.refuse?.(t) ?? null;
    const head = { at: t, spot, spot_t: t, spot_age_s: 0, spot_why: null, n_strikes: stored.length,
      step_local: stored.length > 1 ? stored[1] - stored[0] : null, atm: refusal ? null : stored[money], refusal,
      detail: refusal ? `refused: ${refusal}` : null };
    if (strike) {
      const pos = stored.indexOf(Number(strike));
      const k = pos - money;
      return { ...head, pick: pos === -1
        ? { strike: Number(strike), side, k: null, rung: null, why: 'strike_not_stored_by_then' }
        : refusal ? { strike: Number(strike), side, k: null, rung: null, why: null }
          : { strike: Number(strike), side, k, rung: rungOf(k, /** @type {'CE'|'PE'} */ (side)), why: null } };
    }
    return { ...head, strikes: stored.map((s, i) => ({ strike: s, k: refusal ? null : i - money,
      ce: refusal ? null : rungOf(i - money, 'CE'), pe: refusal ? null : rungOf(i - money, 'PE') })) };
  });
  return /** @type {any} */ ({ status: 200, json: async () => ({ generation: 7, census: { state: 'held', behind_days: 0, behind_from: null }, chains }) });
};

test('a rung is the server’s, at the bar’s own instant, over the strikes stored by then (D-3329, DB-037)', async () => {
  // Strike 1,10,000 and 1,15,000 stored from the start; 1,05,000 and
  // 1,20,000 to 1,40,000 first stored later. At t0 the 1,15,000 CE is one
  // strike out; once the later strikes are stored it is still OTM+1 here
  // because ATM moves with spot, and the rung at t0 never sees them.
  const t0 = 1790000000;
  const t1 = t0 + 86_400 * 30;
  const strikes = [1050000, 1100000, 1150000, 1200000, 1250000, 1300000, 1350000, 1400000];
  const first = Object.fromEntries(strikes.map((s) => [s, s === 1100000 || s === 1150000 ? t0 - 10 : t0 + 100]));
  /** @type {string[]} */
  const asked = [];
  const request = chainServer({ strikes, first, spotAt: (t) => (t < t1 ? 1_124_350 : 1_390_000) }, asked);
  const contract = K(1150000, 'CE', '2026-12-31');
  const rows = Array.from({ length: 20 }, (_, i) => bar(contract, i < 10 ? t0 + i : t1 + i));
  const { rungs, ladders } = await readRungs('gdfl', rows, new AbortController().signal, request);
  // One contract over twenty seconds: the per-contract form, one request,
  // and one more for the chain at the newest instant.
  assert.equal(asked.length, 2);
  const q = new URL(asked[0], 'http://x').searchParams;
  assert.deepEqual([q.get('underlying'), q.get('expiry'), q.get('strike'), q.get('side'), q.get('timeframe')],
    ['NIFTY', '2026-12-31', '1150000', 'CE', '1s']);
  assert.equal(q.get('at')?.split(',').length, 20);
  assert.equal(rungs.get(rungKey(rows[0]))?.rung, 'OTM+1', 'at t0 only 1,10,000 and 1,15,000 are stored');
  assert.equal(rungs.get(rungKey(rows[0]))?.n, 2);
  assert.equal(rungs.get(rungKey(rows[19]))?.rung, 'ITM-5', 'a month later the chain holds the strikes stored by then');
  const ladder = /** @type {any} */ (ladders.get(ladderKey(rows[0])));
  assert.equal(ladder.at, t1 + 19);
  assert.equal(ladder.strikes.length, strikes.length);
  assert.equal(ladder.census.state, 'held');
  assert.equal(ladder.generation, 7);
  // The module counts nothing itself: an answer it did not get is pending.
  assert.equal(optionCells(bar(contract, t0 + 999), undefined, rungs)?.mnyWhy, 'rung_pending');
});

test('a page of many contracts over few seconds asks every strike at once, eight instants a request', async () => {
  const t0 = 1790000000;
  const strikes = Array.from({ length: 30 }, (_, i) => 1_000_000 + 5000 * i);
  const first = Object.fromEntries(strikes.map((s) => [s, t0 - 1]));
  /** @type {string[]} */
  const asked = [];
  const request = chainServer({ strikes, first, spotAt: () => 1_072_400, refuse: (t) => (t === t0 + 9 ? 'census_behind' : null) }, asked);
  const rows = [];
  for (let s = 0; s < 10; s += 1) for (const k of strikes.slice(0, 5)) rows.push(bar(K(k, s % 2 ? 'PE' : 'CE'), t0 + s));
  rows.push(bar(K(9_990_000, 'CE'), t0)); // a strike never stored
  const { rungs } = await readRungs('gdfl', rows, new AbortController().signal, request);
  assert.equal(asked.length, Math.ceil(10 / LADDER_CHUNK));
  assert.ok(asked.every((u) => !new URL(u, 'http://x').searchParams.has('strike')));
  assert.equal(new URL(asked[0], 'http://x').searchParams.get('at')?.split(',').length, LADDER_CHUNK);
  // Spot 10,724.00 sits nearest 10,700 (k=0 at index 14): 10,000 is 14 strikes below.
  assert.equal(rungs.get(rungKey(bar(K(1_000_000, 'CE'), t0)))?.rung, 'ITM-14');
  assert.equal(rungs.get(rungKey(bar(K(1_000_000, 'PE'), t0 + 1)))?.rung, 'OTM+14');
  assert.deepEqual(
    [rungs.get(rungKey(bar(K(1_000_000, 'PE'), t0 + 9)))?.rung, rungs.get(rungKey(bar(K(1_000_000, 'PE'), t0 + 9)))?.why],
    [null, 'census_behind']
  );
  assert.equal(rungs.get(rungKey(bar(K(9_990_000, 'CE'), t0)))?.why, 'strike_not_stored_by_then');
  assert.equal(rungs.size, rows.length);
});

test('a refused rung, a failed read and an abort are each named, never a rung', async () => {
  const c = { at: 5, refusal: 'ladder_too_sparse', detail: 'the nearest strike is far', atm: null, step_local: 50000, n_strikes: 3, spot: 1 };
  assert.deepEqual(rungFrom(c, 100, 'CE'), { rung: null, k: null, why: 'ladder_too_sparse', detail: 'the nearest strike is far', step: 50000, atm: null, n: 3, spot: 1 });
  assert.equal(rungFrom({ at: 5, refusal: null, pick: { rung: null, why: 'strike_not_stored_by_then' } }, 100, 'CE').why, 'strike_not_stored_by_then');
  assert.equal(rungFrom({ at: 5, refusal: null, pick: { rung: 'ITM-2', k: -2, why: null } }, 100, 'CE').rung, 'ITM-2');
  assert.equal(rungFrom({ at: 5, refusal: null, strikes: [{ strike: 100, k: 1, ce: 'OTM+1', pe: 'ITM-1' }] }, 100, 'PE').rung, 'ITM-1');
  assert.equal(rungFrom({ at: 5, refusal: null, strikes: [] }, 100, 'PE').why, 'strike_not_stored_by_then');
  const rows = [bar(K(1_000_000, 'CE'), 1790000000)];
  const failed = await readRungs('gdfl', rows, new AbortController().signal,
    async () => /** @type {any} */ ({ status: 400, json: async () => ({ error: 'expiry is required' }) }));
  assert.deepEqual([...failed.rungs.values()].map((r) => [r.rung, r.why, r.detail]), [[null, 'rung_request_failed', 'expiry is required']]);
  const short = await readRungs('gdfl', rows, new AbortController().signal,
    async () => /** @type {any} */ ({ status: 200, json: async () => ({ chains: [] }) }));
  assert.equal([...short.rungs.values()][0].why, 'rung_request_failed');
  const gone = new AbortController();
  gone.abort();
  await assert.rejects(readRungs('gdfl', rows, gone.signal, async () => /** @type {any} */ ({})));
  assert.equal((await readRungs('gdfl', [bar('NSE-INDEX-NIFTY', 1790000000)], new AbortController().signal, async () => { throw 'unreached'; })).rungs.size, 0);
  // asof=prev rides along on the chain too, so the rung and the spot agree.
  /** @type {string[]} */
  const asked = [];
  await readRungs('gdfl', rows, new AbortController().signal, chainServer({ strikes: [1_000_000], first: { 1000000: 0 }, spotAt: () => 1_000_000 }, asked), 'prev');
  assert.ok(asked.every((u) => new URL(u, 'http://x').searchParams.get('asof') === 'prev'));
});

test('intrinsic is clipped at zero per side and extrinsic is the premium left over, unclipped', () => {
  assert.equal(intrinsic(2400000, 'CE', 2410000), 10000);
  assert.equal(intrinsic(2420000, 'CE', 2410000), 0);
  assert.equal(intrinsic(2420000, 'PE', 2410000), 10000);
  assert.equal(intrinsic(2400000, 'PE', 2410000), 0);
  const row = bar(K(2400000, 'CE'), 1790000000, '2099-01', 9000);
  const rungs = new Map([[rungKey(row), { rung: 'ITM-2', k: -2, why: null, detail: null, step: 5000, atm: 2410000, n: 5, spot: 2410000 }]]);
  assert.deepEqual(optionCells(row, { close: 2410000, why: null, spotT: 1790000000, age: 0, asof: 'exact', segment: 'INDEX', key: 'NIFTY' }, rungs),
    { mny: 'ITM-2', mnyWhy: null, mnyDetail: null, step: 5000, atm: 2410000, n: 5,
      intr: 10000, extr: -1000, why: null, spot: 2410000, detail: null, spotT: 1790000000, age: 0, asof: 'exact' });
});

test('a missing spot or rung is the named reason in its cells, never a figure', () => {
  const row = bar(K(2410000, 'PE'), 1790000000, '2099-01', 5000);
  const pending = optionCells(row, undefined, new Map());
  assert.deepEqual([pending?.mny, pending?.mnyWhy, pending?.intr, pending?.extr, pending?.why], [null, 'rung_pending', null, null, 'spot_pending']);
  const noBar = optionCells(row, { close: null, why: 'no_spot_bar', spotT: null, age: null, asof: 'exact', segment: 'INDEX', key: 'NIFTY' }, new Map());
  assert.deepEqual([noBar?.intr, noBar?.extr, noBar?.why], [null, null, 'no_spot_bar']);
  const failed = optionCells(row, /** @type {any} */ ({ close: null, why: null, detail: 'boom' }), new Map());
  assert.deepEqual([failed?.why, failed?.detail], ['spot_request_failed', 'boom']);
  const refused = optionCells(row, undefined, new Map([[rungKey(row), { rung: null, k: null, why: 'census_behind', detail: 'behind 3 days', step: null, atm: null, n: null, spot: null }]]));
  assert.deepEqual([refused?.mny, refused?.mnyWhy, refused?.mnyDetail], [null, 'census_behind', 'behind 3 days']);
  assert.equal(optionCells(bar('NSE-INDEX-NIFTY', 1790000000), undefined, new Map()), null);
  for (const why of ['no_spot_bar', 'no_spot_in_session', 'no_spot_file', 'spot_unreadable', 'spot_order_broken',
    't_outside_month', 't_off_grid', 'spot_request_failed', 'spot_pending']) {
    assert.equal(typeof /** @type {Record<string, string>} */ (SPOT_WHY)[why], 'string', why);
  }
  for (const why of ['census_absent', 'census_state_unknown', 'census_degraded', 'census_behind', 'expiry_not_in_census',
    'no_strike_stored_by_then', 'ladder_too_sparse', 'strike_not_stored_by_then', 'rung_request_failed', 'rung_pending']) {
    assert.equal(typeof /** @type {Record<string, string>} */ (RUNG_WHY)[why], 'string', why);
  }
  assert.equal(RUNG_CHUNK, 400);
});

test('the page asks each bar’s spot, rung and greeks under one spot rule, and counts no ladder itself', () => {
  const page = databaseSource();
  const cells = dbSource('../src/routes/db/DbOptionCells.svelte');
  const money = dbSource('../src/routes/db/DbMoneyness.svelte');
  const grid = dbSource('../src/routes/db/DbBarsGrid.svelte');
  assert.match(cells, /readSpots\(feed, rows, controller\.signal, ask, asof\)/);
  assert.match(cells, /readRungs\(feed, rows, controller\.signal, ask, asof\)/);
  assert.match(cells, /readGreeks\(feed, rows, controller\.signal, ask, asof\)/);
  assert.match(cells, /optionCells\(b, db\.spotMap\.get\(spotKey\(b\)\), db\.rungMap\)/);
  // The rows asked about are the page as the server sent it, before the pick.
  assert.match(cells, /db\.barPageRead \?\? db\.barPage/);
  assert.match(grid, /const barPage = \$derived\(db\.rungKeep \? barPageRead\.filter\(db\.rungKeep\) : barPageRead\)/);
  // No whole-census ladder anywhere on the page (DB-008, DB-037): the fold
  // that built one, and the hard-coded null spot that disabled the control.
  assert.doesNotMatch(page, /chainsFrom\(/);
  assert.doesNotMatch(page, /const spotPaisa = \$derived/);
  assert.doesNotMatch(page, /rungKeyOf/);
  assert.doesNotMatch(money, /for \(const r of db\.deco\) \{\s*if \(r\.strike === null\) continue;/);
  assert.match(dbSource('../src/lib/db/notes-bars.js'), /const OPTION_COLS = new Set\(\[\s*'side',\s*'strike',\s*'mny',\s*'intr',\s*'extr',/);
  assert.match(grid, /\.bgrid\.lean\.nooption > tbody > tr > td:nth-child\(n \+ 15\)/);
  assert.match(cells, /:global\(\.bgrid\.lean\.nooption > tbody > tr\) > td:nth-child\(n \+ 15\)/);
  assert.doesNotMatch(page, /\.bgrid\.lean > tbody > tr > td:nth-child\(n \+ 20\)/);
  assert.match(grid, /\.bgrid\.lean > tbody > tr > td:nth-child\(9\) \{/);
});

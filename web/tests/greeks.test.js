import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  GREEK_CHUNK, GREEK_WHY, cellOf, greekCell, greekKey, greekText, greekWhyText, ivText, readGreeks
} from '../src/lib/greeks.js';
import { barTimeLabel, secondLabel, timeLabel } from '../src/lib/dates.js';

// Generated shapes and invented figures only; no row here is market data. D-2830.
const KEY = 'NSE-FNO-NIFTY-2026-08-27-2500000-CE';
const row = (/** @type {number} */ s, instrument = KEY) => ({
  instrument, tf: '1s', month: '2026-08', ts: s * 1000, side: 'CE'
});
const priced = (/** @type {number} */ t) => ({
  t, premium: 25000, spot: 2500000, tenor_s: 600000, rate: 0.06, auction: '2026-08-19',
  iv: 0.1234, delta: 0.52, gamma: 0.00071, theta: -9.5, vega: 13.25, rho: 2.5,
  iterations: 6, below_band: true, why: null, detail: null
});

test('a priced row carries the server figures and an unpriced row carries its reason, never a figure', () => {
  const c = cellOf(priced(1), null);
  assert.equal(c.why, null);
  assert.deepEqual([c.iv, c.delta, c.gamma, c.theta, c.vega, c.rho], [0.1234, 0.52, 0.00071, -9.5, 13.25, 2.5]);
  const r = cellOf({ t: 1, premium: 25000, spot: 2500000, why: 'rate_unset', detail: 'BRUTEX_TBILL91_SERIES is not set' }, null);
  assert.equal(r.why, 'rate_unset');
  assert.deepEqual([r.iv, r.delta, r.gamma, r.theta, r.vega, r.rho], [null, null, null, null, null, null]);
  assert.match(greekWhyText(r), /no risk-free rate table is configured.*BRUTEX_TBILL91_SERIES is not set/);
  // A server fault sentence rides along with the reason.
  const f = cellOf({ t: 1, why: 'option_unreadable', detail: null }, 'record 3: damaged');
  assert.equal(f.detail, 'record 3: damaged');
  // A row claiming no reason but carrying no volatility is not a figure.
  assert.equal(cellOf({ t: 1, why: null, iv: null }, null).why, 'greeks_request_failed');
  assert.match(greekWhyText({ ...r, why: 'mystery', detail: null }), /^unknown \(mystery\)$/);
  for (const why of ['rate_unset', 'rate_unreadable', 'rate_before_series', 'rate_stale', 'tenor_refused',
    'iv_unsolved', 'no_option_bar', 'no_option_file', 'option_unreadable', 'option_order_broken',
    'no_spot_bar', 'no_spot_file', 'spot_unreadable', 'spot_order_broken', 'greeks_request_failed',
    'greeks_pending']) {
    assert.equal(typeof /** @type {Record<string, string>} */ (GREEK_WHY)[why], 'string', why);
  }
});

test('a non-option row has no greek cell, and an unread option row is pending by name', () => {
  assert.equal(greekCell({ side: null }, undefined), null);
  assert.equal(greekCell({ side: 'PE' }, undefined)?.why, 'greeks_pending');
  const c = cellOf(priced(1), null);
  assert.equal(greekCell({ side: 'CE' }, c), c);
  assert.equal(ivText(0.1234), '12.34');
  assert.equal(ivText(null), '—');
  assert.equal(greekText(-9.5, 2), '-9.50');
  assert.equal(greekText(null, 2), '—');
});

test('greeks are asked once per contract, rung and month, in chunks, and each instant filed exactly', async () => {
  const base = 1787197500;
  const rows = [
    ...Array.from({ length: GREEK_CHUNK + 1 }, (_, i) => row(base + i)),
    row(base, 'NSE-FNO-NIFTY-2026-08-27-2500000-PE'),
    { ...row(base), side: null }
  ];
  /** @type {string[]} */
  const asked = [];
  const request = async (/** @type {string} */ url) => {
    asked.push(url);
    const q = new URL(url, 'http://x').searchParams;
    const ts = String(q.get('t')).split(',').map(Number);
    if (q.get('contract') === '2026-08-27-2500000-PE') {
      return /** @type {any} */ ({ status: 400, json: async () => ({ error: 'refused by name' }) });
    }
    return /** @type {any} */ ({
      status: 200,
      json: async () => ({
        rate: { state: 'loaded' },
        faults: null,
        rows: ts.map((t) => (t === base + 1 ? { t, premium: 1, spot: 2, why: 'iv_unsolved', detail: 'below intrinsic' } : priced(t)))
      })
    });
  };
  const out = await readGreeks('gdfl', rows, new AbortController().signal, request);
  assert.equal(asked.length, 3);
  const first = new URL(asked[0], 'http://x').searchParams;
  assert.equal(first.get('feed'), 'gdfl');
  assert.equal(first.get('exchange'), 'NSE');
  assert.equal(first.get('segment'), 'FNO');
  assert.equal(first.get('symbol'), 'NIFTY');
  assert.equal(first.get('contract'), '2026-08-27-2500000-CE');
  assert.equal(first.get('timeframe'), '1s');
  assert.equal(first.get('month'), '2026-08');
  assert.equal(first.get('t')?.split(',').length, GREEK_CHUNK);
  assert.equal(new URL(asked[1], 'http://x').searchParams.get('t'), String(base + GREEK_CHUNK));
  assert.deepEqual(out.rate, { state: 'loaded' });
  assert.equal(out.cells.get(greekKey(rows[0]))?.iv, 0.1234);
  assert.equal(out.cells.get(greekKey(rows[1]))?.why, 'iv_unsolved');
  assert.equal(out.cells.get(greekKey(rows[1]))?.detail, 'below intrinsic');
  assert.equal(out.cells.get(greekKey(rows[GREEK_CHUNK + 1]))?.why, 'greeks_request_failed');
  assert.equal(out.cells.get(greekKey(rows[GREEK_CHUNK + 1]))?.detail, 'refused by name');
  assert.equal(out.cells.size, GREEK_CHUNK + 2);
  const thrown = await readGreeks('gdfl', rows.slice(0, 1), new AbortController().signal,
    async () => { throw 'offline'; });
  assert.equal([...thrown.cells.values()][0].detail, 'offline');
  const gone = new AbortController();
  gone.abort();
  await assert.rejects(readGreeks('gdfl', rows.slice(0, 1), gone.signal, request));
});

test('a one-second bar shows its second and a minute bar its minute', () => {
  const at = '2026-08-03T03:45:07Z'; // 09:15:07 IST
  assert.equal(secondLabel(at), '09:15:07');
  assert.equal(barTimeLabel(at, 1), '09:15:07');
  assert.equal(barTimeLabel(at, 60), '09:15');
  assert.equal(barTimeLabel(at, 86400), '09:15');
  assert.equal(barTimeLabel(at, undefined), '09:15');
  // The minute half never drifts from `timeLabel`.
  assert.equal(secondLabel(at).slice(0, 5), timeLabel(at));
  assert.equal(secondLabel(/** @type {any} */ (null)), '—');
});

test('the page draws the greeks from the server and computes no model', () => {
  const page = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');
  const lib = readFileSync(new URL('../src/lib/greeks.js', import.meta.url), 'utf8');
  assert.match(page, /readGreeks\(feed, rows, controller\.signal\)/);
  assert.match(page, /greekCell\(b, greekMap\.get\(greekKey\(b\)\)\)/);
  assert.match(page, /barTimeLabel\(b\.ts, TF_SECS\.get\(b\.tf\)\)/);
  assert.match(page, /'extr',\s*'iv',\s*'delta',\s*'gamma',\s*'theta',\s*'vega',\s*'rho'/);
  assert.doesNotMatch(page, /none: NO_GREEK\b/);
  // No Black-Scholes in the browser: the greeks module takes no exp, log or
  // root at all, and neither file holds a normal CDF. (The page's own
  // Math.log is the volume ruler's log scale, not a model.)
  assert.doesNotMatch(lib, /Math\.(exp|log|sqrt)\(/);
  for (const src of [page, lib]) assert.doesNotMatch(src, /\berf\(|normCdf|normalCdf/i);
});

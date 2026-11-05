# pops.mjs.md

Source: `state/strategy-psrb-20261103/work-willr-cci/model/pops.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// pops.mjs (5 Nov 2026): per-deque pop counts of PARTS-WILLR-CCI.md 5.3's rule over EVERY candle (warm or not), on the
// staircase input of wr-cci.mjs (highs fall by one paisa a candle, jump every `period` candles; low = high - 10).
function pops(n, bars) { const hi = [], lo = []; let s = 0; const ph = [], pl = []; let maxLen = 0;
  for (const [h, l] of bars) { if (hi.length && s - hi[0][0] >= n) hi.shift(); if (lo.length && s - lo[0][0] >= n) lo.shift();
    let a = 0; while (hi.length && hi[hi.length - 1][1] <= h) { hi.pop(); a++; } hi.push([s, h]);
    let b = 0; while (lo.length && lo[lo.length - 1][1] >= l) { lo.pop(); b++; } lo.push([s, l]);
    ph.push(a); pl.push(b); maxLen = Math.max(maxLen, hi.length, lo.length); s++; }
  const st = (v) => { const w = [...v].sort((x, y) => x - y); return { total: v.reduce((x, y) => x + y, 0), p99: w[Math.floor(w.length * 0.99)], max: w[w.length - 1], over100: v.filter((x) => x > 100).length }; };
  return { highs: st(ph), lows: st(pl), maxLen }; }
for (const n of [14, 2000]) { const bars = []; for (let k = 0; k < 20000; k++) { const h = (k % n === n - 1) ? 1000000 : 900000 - (k % n); bars.push([h, h - 10]); }
  const r = pops(n, bars); console.log(`period ${n}, 20000 candles: highs ${JSON.stringify(r.highs)} lows ${JSON.stringify(r.lows)} max deque length ${r.maxLen}`); }
`````

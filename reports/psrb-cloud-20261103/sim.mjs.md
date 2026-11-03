# sim.mjs.md

Source: `state/strategy-psrb-20261103/work/sim.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// sim.mjs: exact BigInt models used to derive the PSRB fixtures in PARTS.md (node only; never the Py-language).
// Models: trend.rs Atr + SuperTrend latch at 1ad6e24ef (lines 263-466), the two-band form of PARTS.md 5.3,
// Wilder RSI on trend::SCALE, Bollinger moments. Run: node sim.mjs <case>
const SCALE = 1000000n;
const de = (a, d) => { let q = a / d; if (a % d !== 0n && ((a < 0n) !== (d < 0n))) q -= 1n; return q; }; // div_euclid for d>0
const mid = (h, l) => { const s = h + l; return s / 2n; }; // i128::midpoint rounds toward zero; prices positive

class Atr {
  constructor(n) { this.n = BigInt(n); this.scaled = 0n; this.seed = 0n; this.prev = null; this.folded = 0n; this.seeded = false; }
  tr(c) { const hl = c.h - c.l; if (this.prev === null) return hl; let hc = c.h - this.prev; let lc = this.prev - c.l; if (hc < 0n) hc = -hc; if (lc < 0n) lc = -lc; let w = hl; if (hc > w) w = hc; if (lc > w) w = lc; return w; }
  fold(c) { this.folded += 1n; const tr = this.tr(c) * SCALE; const count = this.folded;
    if (this.seeded && count > this.n) { if (this.n > 0n) { this.scaled += de(tr - this.scaled, this.n); } }
    else { this.seed += tr; this.scaled = de(this.seed, count); this.seeded = true; }
    this.prev = c.c; }
  value() { return this.seeded ? de(this.scaled, SCALE) : null; }
  warm() { return this.folded >= this.n; }
}

class Latch { // trend.rs SuperTrend
  constructor(n, m) { this.atr = new Atr(n); this.m = BigInt(m); this.stop = null; this.trend = 'Up'; }
  fold(c) { this.atr.fold(c); const a = this.atr.value(); if (a === null) return;
    const md = mid(c.h, c.l); const band = de(a * this.m, 1000n); const up = md + band; const lo = md - band; const cl = c.c;
    if (this.stop === null) { if (cl >= md) { this.trend = 'Up'; this.stop = lo; } else { this.trend = 'Down'; this.stop = up; } return; }
    const p = this.stop;
    if (this.trend === 'Up') { if (cl < p) { this.trend = 'Down'; this.stop = up; } else { this.stop = lo > p ? lo : p; } }
    else { if (cl > p) { this.trend = 'Up'; this.stop = lo; } else { this.stop = up < p ? up : p; } } }
}

class TwoBand { // PARTS.md 5.3: both bands ratchet; the previous TREND picks the band judged (D-8602)
  constructor(n, m) { this.atr = new Atr(n); this.m = BigInt(m); this.pu = 0n; this.pl = 0n; this.ptrend = 'Down'; this.had = false; this.pc = null; this.reading = null; }
  fold(c) { this.atr.fold(c); const a = this.atr.value();
    if (!(this.atr.warm() && a !== null)) { this.pc = c.c; return; }
    const band = de(a * this.m, 1000n); const md = mid(c.h, c.l); const bu = md + band; const bl = md - band;
    const upper = (bu < this.pu || (this.pc !== null && this.pc > this.pu)) ? bu : this.pu;
    const lower = (bl > this.pl || (this.pc !== null && this.pc < this.pl)) ? bl : this.pl;
    let trend; if (!this.had) trend = 'Down'; else if (this.ptrend === 'Down') trend = c.c > upper ? 'Up' : 'Down'; else trend = c.c < lower ? 'Down' : 'Up';
    const line = trend === 'Up' ? lower : upper; const settled = this.had;
    this.pu = upper; this.pl = lower; this.ptrend = trend; this.had = true; this.pc = c.c;
    this.reading = { trend, line, upper, lower, settled }; }
}

const C = (h, l, c) => ({ h: BigInt(h), l: BigInt(l), c: BigInt(c) });
function both(n, m, bars) {
  const L = new Latch(n, m), T = new TwoBand(n, m); const rows = [];
  bars.forEach((b, i) => { L.fold(b); T.fold(b); const r = T.reading;
    rows.push(`${i} H${b.h} L${b.l} C${b.c} | atr ${L.atr.value()} | latch ${L.trend} ${L.stop} | two-band ${r ? `${r.trend} line ${r.line} up ${r.upper} lo ${r.lower} ${r.settled ? 'settled' : 'first'}` : 'none'}`); });
  return rows.join('\n');
}

// RSI on SCALE: seed = floor(sum*SCALE/n), step avg += floor((x*SCALE - avg)/n)
function rsi(n, closes) { n = BigInt(n); let prev = null, k = 0n, gs = 0n, ls = 0n, g = 0n, l = 0n; const out = [];
  for (const c0 of closes) { const c = BigInt(c0); if (prev !== null) { const d = c - prev; const gain = d > 0n ? d : 0n; const loss = d < 0n ? -d : 0n; k += 1n;
      if (k <= n) { gs += gain * SCALE; ls += loss * SCALE; if (k === n) { g = de(gs, n); l = de(ls, n); } }
      else { g += de(gain * SCALE - g, n); l += de(loss * SCALE - l, n); } }
    prev = c; const warm = k >= n; let ppm = null;
    if (warm) { if (g + l === 0n) ppm = 1000000n; else if (l === 0n) ppm = 1000000n; else ppm = (1000000n * g) / (g + l); }
    out.push({ c: c0, warm, g, l, ppm }); }
  return out; }

// exact Wilder in rationals (num/den), for the bound check
function rsiExact(n, closes) { n = BigInt(n); let prev = null, k = 0n; let gs = 0n, ls = 0n; let G = null, L = null; const out = [];
  // represent G = gn/gd
  for (const c0 of closes) { const c = BigInt(c0); if (prev !== null) { const d = c - prev; const gain = d > 0n ? d : 0n; const loss = d < 0n ? -d : 0n; k += 1n;
      if (k <= n) { gs += gain; ls += loss; if (k === n) { G = [gs, n]; L = [ls, n]; } }
      else { G = [G[0] * (n - 1n) + gain * G[1], G[1] * n]; L = [L[0] * (n - 1n) + loss * L[1], L[1] * n]; } }
    prev = c; out.push(G ? { G, L } : null); }
  return out; }

function boll(n, closes, zMilli) { // returns per index: S, Q, V, D, above, below, upperFloor, lowerCeil
  const N = BigInt(n); const out = []; const zn = BigInt(zMilli), zd = 1000n;
  const isqrt = (v) => { if (v <= 0n) return 0n; let x = BigInt(Math.floor(Math.sqrt(Number(v)))); while (x * x > v) x -= 1n; while ((x + 1n) * (x + 1n) <= v) x += 1n; return x; };
  for (let i = 0; i < closes.length; i++) { if (i + 1 < n) { out.push(null); continue; }
    const w = closes.slice(i + 1 - n, i + 1).map(BigInt); const S = w.reduce((a, b) => a + b, 0n); const Q = w.reduce((a, b) => a + b * b, 0n);
    const V = N * Q - S * S; const c = w[w.length - 1]; const D = N * c - S;
    const above = D > 0n && (D * zd) ** 2n > zn * zn * V; const below = -D > 0n && (D * zd) ** 2n > zn * zn * V;
    const r = isqrt(zn * zn * V); const uf = de(S * zd + r, N * zd); const lc = -de(r - S * zd, N * zd);
    out.push({ c: closes[i], S, Q, V, D, above, below, uf, lc }); }
  return out; }

const which = (import.meta.url === `file://${process.argv[1]}`) ? (process.argv[2] || "all") : "none";
if (which === 'st-a' || which === 'all') {
  console.log('--- ST fixture A: n=2, m=1000');
  console.log(both(2, 1000, [C(110, 90, 100), C(112, 100, 110), C(115, 105, 114), C(116, 108, 115), C(116, 109, 111), C(112, 96, 98), C(100, 90, 95)]));
}
if (which === 'st-flat' || which === 'all') {
  console.log('--- ST flat: n=3, m=3000');
  console.log(both(3, 3000, [C(100, 100, 100), C(100, 100, 100), C(100, 100, 100), C(100, 100, 100), C(100, 100, 100)]));
}
if (which === 'rsi' || which === 'all') {
  console.log('--- RSI n=3');
  for (const r of rsi(3, [100, 103, 101, 104, 104, 98, 101, 107])) console.log(JSON.stringify(r, (k, v) => typeof v === 'bigint' ? v.toString() : v));
}
if (which === 'boll' || which === 'all') {
  console.log('--- Bollinger n=8 z=2.000');
  for (const b of boll(8, [200, 400, 400, 400, 500, 500, 700, 900], 2000)) console.log(JSON.stringify(b, (k, v) => typeof v === 'bigint' ? v.toString() : v));
}
export { Atr, Latch, TwoBand, rsi, rsiExact, boll, both, C, de };
`````

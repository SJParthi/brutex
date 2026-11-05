# wr-cci.mjs.md

Source: `state/strategy-psrb-20261103/work-willr-cci/model/wr-cci.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// wr-cci.mjs (5 Nov 2026): BigInt models of PARTS-WILLR-CCI.md's integer rules, written apart from the hand
// computations, to re-check every fixture value. Two independent forms each: the part's integer rule and an exact
// rational transcription of the published definition. node only; no file is written.
const B = (x) => BigInt(x);
const fdiv = (a, b) => { const q = a / b; return (a % b !== 0n && ((a < 0n) !== (b < 0n))) ? q - 1n : q; }; // floor
const abs = (a) => (a < 0n ? -a : a);
const gcd = (a, b) => { a = abs(a); b = abs(b); while (b) { [a, b] = [b, a % b]; } return a; };
const Q = (n, d) => { if (d < 0n) { n = -n; d = -d; } const g = gcd(n, d) || 1n; return [n / g, d / g]; };
const qsub = (x, y) => Q(x[0] * y[1] - y[0] * x[1], x[1] * y[1]);
const qadd = (x, y) => Q(x[0] * y[1] + y[0] * x[1], x[1] * y[1]);
const qmul = (x, y) => Q(x[0] * y[0], x[1] * y[1]);
const qdiv = (x, y) => Q(x[0] * y[1], x[1] * y[0]);
const qcmp = (x, y) => { const l = x[0] * y[1], r = y[0] * x[1]; return l > r ? 1 : l < r ? -1 : 0; };
// ---------- Williams %R: integer rule (deques) and brute force + rationals ----------
export function willr(n, bars) {
  const hi = [], lo = []; let seq = 0n; const out = [];
  bars.forEach(([h, l, c]) => { h = B(h); l = B(l); c = B(c); const s = seq;
    if (hi.length && s - hi[0][0] >= B(n)) hi.shift(); if (lo.length && s - lo[0][0] >= B(n)) lo.shift();
    let pops = 0; while (hi.length && hi[hi.length - 1][1] <= h) { hi.pop(); pops++; } hi.push([s, h]);
    while (lo.length && lo[lo.length - 1][1] >= l) { lo.pop(); pops++; } lo.push([s, l]);
    seq++; const warm = seq >= B(n);
    if (!warm) { out.push(null); return; }
    const HH = hi[0][1], LL = lo[0][1], U = HH - c, R = HH - LL;
    out.push({ HH, LL, C: c, U, R, flat: R === 0n, ppm: R === 0n ? null : fdiv(-1000000n * U, R), pops,
      above: (lvl) => R !== 0n && 1000000n * U < -B(lvl) * R, below: (lvl) => R !== 0n && 1000000n * U > -B(lvl) * R });
  });
  return out;
}
export function willrBrute(n, bars) { // max/min by scan, %R as an exact rational
  return bars.map((_, t) => { if (t + 1 < n) return null; const w = bars.slice(t + 1 - n, t + 1).map((b) => b.map(B));
    const HH = w.reduce((m, b) => (b[0] > m ? b[0] : m), w[0][0]), LL = w.reduce((m, b) => (b[1] < m ? b[1] : m), w[0][1]);
    const C = w[w.length - 1][2]; if (HH === LL) return { HH, LL, flat: true };
    const wr = Q(100n * (C - HH), HH - LL); // 100 (close - highest) / (highest - lowest)
    return { HH, LL, flat: false, wr, ppmFloor: fdiv(wr[0] * 10000n, wr[1]) }; });
}
// ---------- CCI: integer rule (thirds, n-multiples) and the published definition in rationals ----------
export function cci(n, bars) {
  const ring = []; let S = 0n; const N = B(n);
  return bars.map(([h, l, c]) => { const T = B(h) + B(l) + B(c);
    if (ring.length === n) S -= ring.shift(); ring.push(T); S += T;
    if (ring.length < n) return null;
    const dev = N * T - S; const D = ring.reduce((a, t) => a + abs(N * t - S), 0n);
    return { T, S, dev, D, flat: D === 0n, milli: D === 0n ? null : fdiv(200000n * N * dev, 3n * D),
      above: (m) => D !== 0n && 200000n * N * dev > 3n * B(m) * D, below: (m) => D !== 0n && 200000n * N * dev < 3n * B(m) * D }; });
}
export function cciPublished(n, bars) { // TP = hlc3, SMA, mean deviation, (TP - SMA) / (0.015 MD)
  return bars.map((_, t) => { if (t + 1 < n) return null; const w = bars.slice(t + 1 - n, t + 1);
    const tp = w.map(([h, l, c]) => Q(B(h) + B(l) + B(c), 3n)); const sma = qdiv(tp.reduce(qadd, [0n, 1n]), [B(n), 1n]);
    const md = qdiv(tp.map((x) => { const d = qsub(x, sma); return [abs(d[0]), d[1]]; }).reduce(qadd, [0n, 1n]), [B(n), 1n]);
    if (md[0] === 0n) return { flat: true }; const v = qdiv(qsub(tp[tp.length - 1], sma), qmul([15n, 1000n], md));
    return { flat: false, cci: v, milliFloor: fdiv(v[0] * 1000n, v[1]) }; });
}
const show = (x) => (x === null || x === undefined ? '-' : x.toString());
if (process.argv[2] === 'run') {
  console.log('### WR-1 (period 3)');
  const wr1 = [[110, 100, 105], [115, 104, 114], [113, 101, 102], [120, 108, 119], [118, 112, 112], [110, 100, 103]];
  const a = willr(3, wr1), b = willrBrute(3, wr1);
  a.forEach((r, i) => { if (!r) { console.log(i, '-'); return; } const q = b[i];
    console.log(i, `HH ${r.HH} LL ${r.LL} C ${r.C} U ${r.U} R ${r.R} ppm ${show(r.ppm)} brute ${q.wr[0]}/${q.wr[1]} floor ${q.ppmFloor}`,
      `above-85 ${r.above(-850000)} below-15 ${r.below(-150000)} above(-850001) ${r.above(-850001)} below(-849999) ${r.below(-849999)}`); });
  console.log('cross above -85 at 3:', !a[2].flat && !a[2].above(-850000) && a[3].above(-850000), ' at 4:', !a[3].above(-850000) && a[4].above(-850000));
  console.log('cross below -15 at 4:', !a[3].below(-150000) && a[4].below(-150000), ' at 5:', !a[4].below(-150000) && a[5].below(-150000));
  console.log('### WR-2 flat then a move (period 3)');
  const wr2 = [[500, 500, 500], [500, 500, 500], [500, 500, 500], [501, 500, 501], [501, 499, 499]];
  willr(3, wr2).forEach((r, i) => console.log(i, r ? `HH ${r.HH} LL ${r.LL} flat ${r.flat} ppm ${show(r.ppm)} above-85 ${r.above(-850000)} below-15 ${r.below(-150000)}` : '-'));
  console.log('### WR period 1'); willr(1, [[110, 90, 100], [100, 100, 100], [120, 100, 120], [120, 100, 100]]).forEach((r, i) => console.log(i, `flat ${r.flat} ppm ${show(r.ppm)}`));
  const M = (1n << 63n) - 1n; const ex = willr(1, [[M, 1n, 1n], [M, 1n, M], [M, 1n, M - 1n]]);
  ex.forEach((r, i) => console.log('extreme', i, `ppm ${r.ppm} 1e6*U bits ${(1000000n * r.U).toString(2).length}`));
  console.log('### CCI-1 (period 4)');
  const c1 = [[110, 100, 105], [112, 104, 110], [108, 98, 100], [120, 106, 118], [125, 115, 124], [126, 120, 121], [118, 100, 101]];
  const x = cci(4, c1), y = cciPublished(4, c1);
  x.forEach((r, i) => { if (!r) { console.log(i, '-'); return; } const p = y[i];
    console.log(i, `T ${r.T} S ${r.S} dev ${r.dev} D ${r.D} milli ${show(r.milli)} published ${p.cci[0]}/${p.cci[1]} floor ${p.milliFloor}`,
      `above100 ${r.above(100000)} below-100 ${r.below(-100000)}`); });
  console.log('### CCI-2 (period 7, the +200 tie) and its mirror, and the 100n/3 ceiling');
  const c2 = [...Array(5).fill([101, 99, 100]), [105, 103, 104], [113, 111, 113]];
  const c2m = [...Array(5).fill([101, 99, 100]), [97, 95, 96], [89, 87, 87]];
  const c3 = [...Array(6).fill([101, 99, 100]), [101, 99, 101]];
  for (const [nm, bars] of [['CCI-2', c2], ['CCI-2 mirror', c2m], ['CCI-3 ceiling', c3]]) { const r = cci(7, bars)[6], p = cciPublished(7, bars)[6];
    console.log(nm, `T ${r.T} S ${r.S} dev ${r.dev} D ${r.D} milli ${r.milli} published ${p.cci[0]}/${p.cci[1]}`,
      `above200 ${r.above(200000)} above199.999 ${r.above(199999)} below200.001 ${r.below(200001)} below-200 ${r.below(-200000)} below-199.999 ${r.below(-199999)} above-200.001 ${r.above(-200001)}`); }
  const c6 = [...Array(5).fill([101, 99, 100]), [101, 99, 101]]; const r6 = cci(6, c6)[5]; console.log('n=6 ceiling', r6.milli, 'above200', r6.above(200000), 'above199.999', r6.above(199999));
  console.log('### CCI flat with different candles, period 1, period 2');
  const fl = cci(3, [[101, 99, 100], [102, 98, 100], [100, 100, 100]]); console.log('flat3', fl[2].D, fl[2].flat);
  console.log('p1', cci(1, [[110, 90, 100], [120, 100, 110]]).map((r) => r.flat).join(' '));
  console.log('p2', cci(2, [[110, 90, 100], [120, 100, 110], [100, 90, 95], [100, 90, 95]]).map((r) => (r ? `${show(r.milli)}` : '-')).join(' '));
  // random cross-check of both indicators against their independent forms
  let seed = 88172645463325252n; const rnd = () => { seed ^= (seed << 13n) & ((1n << 64n) - 1n); seed ^= seed >> 7n; seed ^= (seed << 17n) & ((1n << 64n) - 1n); return seed; };
  let checks = 0, bad = 0, ceilingEq = 0;
  for (let trial = 0; trial < 400; trial++) { const n = Number(rnd() % 30n) + 1; const len = n + Number(rnd() % 40n); let p = 10000n; const bars = [];
    for (let k = 0; k < len; k++) { p += (rnd() % 41n) - 20n; if (p < 50n) p = 50n; const up = rnd() % 15n, dn = rnd() % 15n, flatbar = rnd() % 7n === 0n;
      const h = flatbar ? p : p + up, l = flatbar ? p : p - dn; const c = flatbar ? p : l + (rnd() % (h - l + 1n)); bars.push([h, l, c]); }
    const wa = willr(n, bars), wb = willrBrute(n, bars), ca = cci(n, bars), cb = cciPublished(n, bars);
    for (let t = 0; t < len; t++) { if (!wa[t]) { if (wb[t]) bad++; continue; } checks++;
      if (wa[t].HH !== wb[t].HH || wa[t].LL !== wb[t].LL || wa[t].flat !== wb[t].flat) bad++;
      if (!wa[t].flat && wa[t].ppm !== wb[t].ppmFloor) bad++;
      for (const lv of [-850000, -150000, -500000]) { if (wa[t].flat) continue; const cmp = qcmp(wb[t].wr, Q(B(lv), 10000n)); if (wa[t].above(lv) !== (cmp > 0) || wa[t].below(lv) !== (cmp < 0)) bad++; }
      if (ca[t].flat !== cb[t].flat) bad++; if (!ca[t].flat) { if (ca[t].milli !== cb[t].milliFloor) bad++;
        if (2n * abs(ca[t].dev) > ca[t].D) bad++; if (2n * abs(ca[t].dev) === ca[t].D) ceilingEq++;
        for (const lv of [200000, -200000, 100000, 0]) { const cmp = qcmp(cb[t].cci, Q(B(lv), 1000n)); if (ca[t].above(lv) !== (cmp > 0) || ca[t].below(lv) !== (cmp < 0)) bad++; } } } }
  console.log(`random cross-check: ${checks} warm readings, ${bad} disagreements, ${ceilingEq} readings at the 100n/3 ceiling`);
  // pops: a staircase of falling highs, then a jump, at period 2000
  const n2 = 2000, st = []; for (let k = 0; k < 20000; k++) { const h = (k % n2 === n2 - 1) ? 1000000 : 900000 - (k % n2); st.push([h, h - 10, h - 5]); }
  const pr = willr(n2, st).filter(Boolean).map((r) => r.pops).sort((p, q) => p - q); const tot = pr.reduce((s, v) => s + v, 0);
  console.log(`staircase n=${n2}: ${pr.length} readings, total pops ${tot}, p99 pops ${pr[Math.floor(pr.length * 0.99)]}, max pops ${pr[pr.length - 1]}`);
  const n3 = 14, st3 = []; for (let k = 0; k < 20000; k++) { const h = (k % n3 === n3 - 1) ? 1000000 : 900000 - (k % n3); st3.push([h, h - 10, h - 5]); }
  const pr3 = willr(n3, st3).filter(Boolean).map((r) => r.pops).sort((p, q) => p - q); console.log(`staircase n=14: p99 pops ${pr3[Math.floor(pr3.length * 0.99)]}, max ${pr3[pr3.length - 1]}`);
}
`````

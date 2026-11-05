# repair1.mjs.md

Source: `state/strategy-psrb-20261103/specs-1105-repair1-work/repair1.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// repair1.mjs (Thu 5 Nov 2026): the figures behind repair 1 of the 5 Nov PSRB specs, every one re-derived here.
// node BigInt only (no floating point). Run: node repair1.mjs > repair1.out
// Sections: B1 (WILLR pop counts by the white-box method), B3 (the EMA 200 C8 fixture and the mirror gap),
// N1 (floored R1/S1 and CPR width gaps), N3 (the CCI ceiling's equality clause), N10 (the CCI level bound),
// N12 (the narrow-CPR cut at i64 extremes), N2 (the narrowest-of-5 product bound).
const B = (x) => BigInt(x);
const abs = (a) => (a < 0n ? -a : a);
const divE = (a, b) => { const q = a / b; const r = a % b; return r < 0n ? (b > 0n ? q - 1n : q + 1n) : q; }; // div_euclid, b > 0
const MASK64 = (1n << 64n) - 1n;
const wsub = (a, b) => (a - b) & MASK64; // u64::wrapping_sub
const wadd = (a, b) => (a + b) & MASK64; // u64::wrapping_add

// ---------------------------------------------------------------------------------------------------------------
// B1. The fold of PWC 5.3 with a u64 candle number that may start anywhere (wrapping). Per fold and per deque:
//   back  = the while loop's pop_back count (what pops.mjs counts),
//   lenM  = len_before + 1 - len_after (what the old test text counted: expiry included),
//   white = len_before + 1 - len_after - expired, expired = 1 iff, BEFORE the fold, the deque is non-empty and
//           next.wrapping_sub(front.0) >= period (the repair's method).
function willrFold(st, h, l) {
  const s = st.next, n = st.n;
  const pre = { hl: st.hi.length, ll: st.lo.length,
    eh: st.hi.length > 0 && wsub(s, st.hi[0][0]) >= n ? 1n : 0n,
    el: st.lo.length > 0 && wsub(s, st.lo[0][0]) >= n ? 1n : 0n };
  if (st.hi.length > 0 && wsub(s, st.hi[0][0]) >= n) st.hi.shift();
  if (st.lo.length > 0 && wsub(s, st.lo[0][0]) >= n) st.lo.shift();
  if (st.hi.length > 0 && wsub(s, st.hi[0][0]) >= n) st.secondExpiry = true; // must never happen
  if (st.lo.length > 0 && wsub(s, st.lo[0][0]) >= n) st.secondExpiry = true;
  let bh = 0n; while (st.hi.length > 0 && st.hi[st.hi.length - 1][1] <= h) { st.hi.pop(); bh++; } st.hi.push([s, h]);
  let bl = 0n; while (st.lo.length > 0 && st.lo[st.lo.length - 1][1] >= l) { st.lo.pop(); bl++; } st.lo.push([s, l]);
  st.next = wadd(s, 1n);
  const lenH = B(pre.hl) + 1n - B(st.hi.length), lenL = B(pre.ll) + 1n - B(st.lo.length);
  st.maxLen = Math.max(st.maxLen, st.hi.length, st.lo.length);
  return { bh, bl, lenH, lenL, whiteH: lenH - pre.eh, whiteL: lenL - pre.el, eh: pre.eh, el: pre.el };
}
function staircase(n, start) {
  const st = { n: B(n), next: start, hi: [], lo: [], maxLen: 0, secondExpiry: false };
  const acc = { bh: [], bl: [], lenH: [], lenL: [], eh: 0n, el: 0n, mismatch: 0, removedMaxH: 0n, removedMaxL: 0n,
    jumpsAfterFirst: 0, jumpsAtPeriod: 0, otherAtPeriod: 0, overPeriod: 0 };
  for (let k = 0; k < 20000; k++) {
    const h = B((k % n === n - 1) ? 1000000 : 900000 - (k % n)); const l = h - 10n;
    const r = willrFold(st, h, l);
    const jumpAfterFirst = k % n === n - 1 && k >= 2 * n - 1;
    if (jumpAfterFirst) { acc.jumpsAfterFirst++; if (r.lenH === B(n)) acc.jumpsAtPeriod++; } else if (r.lenH === B(n)) acc.otherAtPeriod++;
    if (r.lenH > B(n) || r.lenL > B(n)) acc.overPeriod++;
    acc.bh.push(r.bh); acc.bl.push(r.bl); acc.lenH.push(r.lenH); acc.lenL.push(r.lenL); acc.eh += r.eh; acc.el += r.el;
    if (r.whiteH !== r.bh || r.whiteL !== r.bl) acc.mismatch++;
    if (r.lenH > acc.removedMaxH) acc.removedMaxH = r.lenH; if (r.lenL > acc.removedMaxL) acc.removedMaxL = r.lenL;
  }
  const stats = (v) => { const w = [...v].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0)); let t = 0n; for (const x of v) t += x;
    const p99 = Number((BigInt(w.length) * 99n) / 100n); // integer index floor(0.99 N), no fractional step
    return `total ${t}, p99 ${w[p99]}, max ${w[w.length - 1]}, candles above 100: ${v.filter((x) => x > 100n).length}`; };
  console.log(`period ${n}, next starts at ${start}: max deque length ${st.maxLen}, a second expiry in one fold: ${st.secondExpiry}`);
  console.log(`  highs back-pops (white-box = while loop): ${stats(acc.bh)}; expiries ${acc.eh}`);
  console.log(`  highs removals in all (len_before + 1 - len_after): ${stats(acc.lenH)}`);
  console.log(`  lows  back-pops (white-box = while loop): ${stats(acc.bl)}; expiries ${acc.el}`);
  console.log(`  lows  removals in all (len_before + 1 - len_after): ${stats(acc.lenL)}`);
  console.log(`  folds where the white-box method differs from the while loop: ${acc.mismatch}`);
  console.log(`  highs: jumps after the first ${acc.jumpsAfterFirst}, of which removing exactly period ${acc.jumpsAtPeriod}; ` +
    `other folds removing period ${acc.otherAtPeriod}; folds removing more than period from either deque ${acc.overPeriod}`);
}
console.log('## B1: WILLR pops on PWC 5.3\'s staircase (20,000 candles; highs fall 1 paisa a candle, jump every period; low = high - 10)');
for (const n of [14, 2000]) { staircase(n, 0n); staircase(n, MASK64 - 5n); }

// ---------------------------------------------------------------------------------------------------------------
// B3. trend::Ema at 1ad6e24ef8e0 (trend.rs:207-233), transcribed: scaled, seed_sum, folded, seeded; SCALE 10^6.
const SCALE = 1000000n;
function emaNew(period) { return { period: B(period), scaled: 0n, seedSum: 0n, folded: 0n, seeded: false }; }
function emaFold(e, price) {
  e.folded += 1n; const target = B(price) * SCALE; const count = e.folded;
  if (!e.seeded || count <= e.period) { e.seedSum += target; e.scaled = divE(e.seedSum, count); e.seeded = true; return; }
  const den = e.period + 1n; const delta = target - e.scaled; e.scaled += divE(delta * 2n, den);
}
const emaValue = (e) => (e.seeded ? divE(e.scaled, SCALE) : null);
const emaWarm = (e) => e.folded >= e.period;
console.log('\n## B3: C8 on the scaled state (period 200)');
{
  const a = emaNew(200), m = emaNew(200); const K = 201n;
  const closes = [...Array(199).fill(101n), 100n, 100n];
  closes.forEach((c, i) => {
    emaFold(a, c); emaFold(m, K - c);
    if (i >= 199) {
      const v = emaValue(a), vm = emaValue(m); const cs = c * SCALE, cms = (K - c) * SCALE;
      const frac = a.scaled - v * SCALE;
      console.log(`fold ${i + 1}: warm ${emaWarm(a)}; close ${c}; scaled ${a.scaled} (e = ${v}.${String(frac).padStart(6, '0')}); value() ${v}`);
      console.log(`   short exact (close*SCALE < scaled) ${cs < a.scaled}; short by value() (close < value()) ${c < v}; long exact ${cs > a.scaled}`);
      console.log(`   mirror K ${K}: close ${K - c}; scaled ${m.scaled}; long exact ${cms > m.scaled}; long by value() ${(K - c) > vm}; short exact ${cms < m.scaled}`);
      console.log(`   mirror gap m = (K*SCALE - scaled) - scaled_mirror = ${K * SCALE - a.scaled - m.scaled}`);
    }
  });
}
// The mirror gap over random walks: m_t = (K*SCALE - S_t) - S'_t must stay in 0..=floor((n+1)/2), and each step
// after the seed must follow m' = m + ceil((r - 2m)/(n + 1)), r = (2 delta) rem_euclid (n + 1), delta = target - S.
{
  let seed = 88172645463325252n; const rnd = () => { seed ^= (seed << 13n) & MASK64; seed ^= seed >> 7n; seed ^= (seed << 17n) & MASK64; return seed; };
  const ceilDiv = (a, b) => -divE(-a, b);
  for (const n of [200, 14, 2, 1]) {
    const bound = (B(n) + 1n) / 2n; // floor((n+1)/2), the tight bound (repair 1, 2.3)
    let maxGap = 0n, minGap = 0n, folds = 0, decide = 0, disagree = 0, near = 0, steps = 0, stepBad = 0, seedGapMax = 0n;
    for (let trial = 0; trial < 40; trial++) {
      const a = emaNew(n), m = emaNew(n); let p = 100000n + (rnd() % 1000n); const K = 300000n;
      const len = n + 400 + Number(rnd() % 400n);
      for (let k = 0; k < len; k++) {
        p += (rnd() % 81n) - 40n; if (p < 1000n) p = 1000n;
        const recursion = a.seeded && a.folded + 1n > a.period; const g0 = K * SCALE - a.scaled - m.scaled;
        const r = recursion ? (((2n * (p * SCALE - a.scaled)) % (B(n) + 1n)) + (B(n) + 1n)) % (B(n) + 1n) : 0n;
        emaFold(a, p); emaFold(m, K - p); folds++;
        const g = K * SCALE - a.scaled - m.scaled; if (g > maxGap) maxGap = g; if (g < minGap) minGap = g;
        if (recursion) { steps++; if (g !== g0 + ceilDiv(r - 2n * g0, B(n) + 1n)) stepBad++; } else if (g > seedGapMax) seedGapMax = g;
        if (emaWarm(a)) { decide++;
          const longA = p * SCALE > a.scaled, shortM = (K - p) * SCALE < m.scaled, shortA = p * SCALE < a.scaled, longM = (K - p) * SCALE > m.scaled;
          const close = abs(p * SCALE - a.scaled) <= bound;
          if (close) near++;
          if ((longA !== shortM || shortA !== longM) && !close) disagree++;
        }
      }
    }
    console.log(`period ${n}: ${folds} folds, mirror gap in ${minGap}..=${maxGap} units of 1/SCALE paisa (bound 0..=${bound}); ` +
      `seed-phase gap at most ${seedGapMax}; ${steps} recursion steps, ${stepBad} off the recursion m' = m + ceil((r - 2m)/(n + 1)); ` +
      `${decide} warm decisions, ${near} within the bound of the state, ${disagree} mirrored decisions that differ outside it`);
  }
}

// ---------------------------------------------------------------------------------------------------------------
// N1. Floored levels (daily.rs:203-211): p = T div 3, b = (H+L) div 2, tc = 2p - b, r1 = 2p - L, s1 = 2p - H.
console.log('\n## N1: floored R1/S1 and the floored CPR width against the exact values (all H, L, C with 1 <= L <= C <= H <= 60)');
{
  let maxR1 = 0n, maxS1 = 0n, gaps = new Set(), wAbove = 0n, wBelow = 0n, overRange = 0n, ex = null;
  // every quantity times 3, so the exact values are integers: R1x3 = 2T - 3L; floored R1 x3 = 3(2p - L).
  for (let H = 1n; H <= 60n; H++) for (let L = 1n; L <= H; L++) for (let C = L; C <= H; C++) {
    const T = H + L + C, p = divE(T, 3n), b = divE(H + L, 2n), tc = 2n * p - b;
    const r1g = (2n * T - 3n * L) - 3n * (2n * p - L), s1g = (2n * T - 3n * H) - 3n * (2n * p - H);
    gaps.add(String(r1g)); if (r1g > maxR1) maxR1 = r1g; if (s1g > maxS1) maxS1 = s1g;
    const wFl3 = 3n * abs(tc - b), wEx3 = abs(2n * C - H - L), rng3 = H - L; // all times 3
    if (wFl3 - wEx3 > wAbove) wAbove = wFl3 - wEx3; if (wEx3 - wFl3 > wBelow) wBelow = wEx3 - wFl3;
    if (wFl3 - rng3 > overRange) { overRange = wFl3 - rng3; ex = `H ${H}, L ${L}, C ${C}: floored width ${abs(tc - b)}, range/3 = ${rng3}/3`; }
  }
  console.log(`exact R1 - floored R1, times 3: values {${[...gaps].join(', ')}}, max ${maxR1} (so up to ${maxR1}/3 paisa); S1 the same, max ${maxS1}`);
  console.log(`floored width - exact width, times 3: from -${wBelow} to +${wAbove}; floored width - range/3, times 3: max ${overRange} (${ex})`);
  const T = 30008n, p = divE(T, 3n);
  console.log(`PIV-2 (10_007, 10_000, 10_001): T mod 3 = ${T % 3n}; exact R1 x3 ${2n * T - 30000n}, floored R1 ${2n * p - 10000n}; exact S1 x3 ${2n * T - 30021n}, floored S1 ${2n * p - 10007n}`);
  const H = 10n, L = 8n, C = 8n, TT = H + L + C, pp = divE(TT, 3n), bb = divE(H + L, 2n);
  console.log(`H 10, L 8, C 8: p ${pp}, b ${bb}, tc ${2n * pp - bb}; floored width ${abs(2n * pp - 2n * bb)}; exact width ${abs(2n * C - H - L)}/3; range/3 = ${H - L}/3`);
}

// ---------------------------------------------------------------------------------------------------------------
// N3. CCI: D >= 2|dev|; for dev != 0 equality iff the current candle is the only one strictly on its side of the
// mean; at dev == 0 equality iff D == 0 (flat).
console.log('\n## N3: the CCI ceiling\'s equality clause (random windows, periods 1..=12)');
{
  let seed = 1234567890123n; const rnd = () => { seed ^= (seed << 13n) & MASK64; seed ^= seed >> 7n; seed ^= (seed << 17n) & MASK64; return seed; };
  let checked = 0, eqNonzero = 0, eqZero = 0, bad = 0, devZeroNonFlat = 0;
  for (let trial = 0; trial < 60000; trial++) {
    const n = Number(rnd() % 12n) + 1; const ts = []; const base = 300n;
    for (let i = 0; i < n; i++) ts.push(base + (rnd() % 4n === 0n ? 0n : (rnd() % 7n) - 3n));
    const N = B(n); let S = 0n; for (const t of ts) S += t; const ds = ts.map((t) => N * t - S); const dev = ds[n - 1];
    let D = 0n; for (const d of ds) D += abs(d); checked++;
    if (2n * abs(dev) > D) bad++;
    const others = ds.slice(0, n - 1);
    const onlyOnItsSide = dev > 0n ? others.every((d) => d <= 0n) : dev < 0n ? others.every((d) => d >= 0n) : null;
    const eq = 2n * abs(dev) === D;
    if (dev !== 0n) { if (eq !== onlyOnItsSide) bad++; if (eq) eqNonzero++; }
    else { if (eq !== (D === 0n)) bad++; if (eq) eqZero++; else devZeroNonFlat++; }
  }
  console.log(`${checked} windows: ${bad} contradictions; equality with dev != 0: ${eqNonzero}; equality with dev == 0 (all flat): ${eqZero}; dev == 0 and not flat (strict): ${devZeroNonFlat}`);
}

// ---------------------------------------------------------------------------------------------------------------
// N10. The level bound: 3 |m| D < 2^127 with D < 2^97 allows |m| <= floor((2^127 - 1) / (3 * 2^97)).
console.log('\n## N10: the CCI level domain against the i128 bound');
{
  const Dmax = (1n << 97n) - 1n; const mMax = ((1n << 127n) - 1n) / (3n * Dmax);
  console.log(`largest |m| (thousandths) with 3 |m| D < 2^127 at D < 2^97: about ${mMax} (${mMax / 1000n} points); chosen MAX_LEVEL_MILLI 10_000_000 (10,000 points)`);
  console.log(`3 x 10^7 x 2^97 < 2^123: ${3n * 10000000n * (1n << 97n) < (1n << 123n)}; the ceiling 100 n / 3 at n = 65,535: ${100n * 65535n / 3n} points (floor)`);
}

// ---------------------------------------------------------------------------------------------------------------
// N12. The narrow-CPR cut at i64 extremes: H = C = 1001 k, L = 998 k gives x = 3k, T = 3000k, a tie at 10 bp.
console.log('\n## N12: the 10 bp cut at the i64 extreme');
{
  const I64 = (1n << 63n) - 1n; const k = I64 / 1001n; const H = 1001n * k, L = 998n * k, C = H;
  const x = 2n * C - H - L, T = H + L + C;
  const lhs = 10000n * abs(x), r10 = 10n * T, r9 = 9n * T;
  console.log(`k = ${k}; H = C = ${H} (<= i64::MAX ${H <= I64}), L = ${L}; x = ${x}, T = ${T}`);
  console.log(`10_000|x| = ${lhs}; 10 T = ${r10}; 9 T = ${r9}: narrow at 10 bp ${lhs <= r10} (tie ${lhs === r10}), at 9 bp ${lhs <= r9}; both sides below 2^79: ${lhs < (1n << 79n) && r10 < (1n << 79n)}`);
  const L2 = L - 1n, x2 = 2n * C - H - L2, T2 = H + L2 + C;
  console.log(`L - 1 = ${L2}: x = ${x2}, T = ${T2}; 10_000|x| = ${10000n * abs(x2)} against 10 T = ${10n * T2}: narrow at 10 bp ${10000n * abs(x2) <= 10n * T2}`);
}

// ---------------------------------------------------------------------------------------------------------------
// N2. The narrowest-of-5 product |x_0| T_j at the extremes, in u128.
console.log('\n## N2: |x| T at the extremes');
{
  const I64 = (1n << 63n) - 1n; const xMax = I64 - 1n; const TMax = 3n * I64;
  console.log(`|x| <= H - L <= ${xMax}; T <= 3 (2^63 - 1) = ${TMax}; product <= ${xMax * TMax}; < 2^128: ${xMax * TMax < (1n << 128n)}; < 2^127 (i128): ${xMax * TMax < (1n << 127n)}`);
}
`````

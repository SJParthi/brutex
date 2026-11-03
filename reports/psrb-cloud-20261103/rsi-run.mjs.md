# rsi-run.mjs.md

Source: `state/strategy-psrb-20261103/repair1-work/rsi-run.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// repair1 (3 Nov 2026): an independent check of check1.md B1 and N1, written from PARTS.md 3.3 alone
// (not from check1-work/*.mjs, not from work/sim.mjs). BigInt only; exact rationals as [num, den] pairs.
const SCALE = 1_000_000n;
const M = 1_000_000n;                       // ppm of the 0..100 scale
const divE = (a, b) => { const q = a / b; return (a % b !== 0n && (a < 0n) !== (b < 0n)) ? q - 1n : q; };
const abs = (x) => (x < 0n ? -x : x);
// exact rational helpers: [n, d] with d > 0, not reduced (BigInt copes)
const radd = ([a, b], [c, d]) => [a * d + c * b, b * d];
const rsub = ([a, b], [c, d]) => [a * d - c * b, b * d];
const rdivk = ([a, b], k) => [a, b * k];
const gcd = (a, b) => { a = abs(a); b = abs(b); while (b) [a, b] = [b, a % b]; return a; };
const red = ([a, b]) => { const g = gcd(a, b) || 1n; return [a / g, b / g]; };

// The integer state of PARTS.md 3.3 and the exact Wilder state, folded side by side.
function fold(n, closes) {
  n = BigInt(n);
  let prev = null, changes = 0n, gs = 0n, ls = 0n, G = 0n, L = 0n;
  let eg = [0n, 1n], el = [0n, 1n], egs = [0n, 1n], els = [0n, 1n];
  const out = [];
  for (const c of closes.map(BigInt)) {
    if (prev === null) { prev = c; out.push(null); continue; }
    const ch = c - prev; prev = c;
    const up = ch > 0n ? ch : 0n, dn = ch < 0n ? -ch : 0n;
    changes += 1n;
    if (changes <= n) {
      gs += up * SCALE; ls += dn * SCALE; egs = radd(egs, [up, 1n]); els = radd(els, [dn, 1n]);
      if (changes < n) { out.push(null); continue; }
      G = divE(gs, n); L = divE(ls, n); eg = red(rdivk(egs, n)); el = red(rdivk(els, n));
    } else {
      G += divE(up * SCALE - G, n); L += divE(dn * SCALE - L, n);
      eg = red(radd(eg, rdivk(rsub([up, 1n], eg), n))); el = red(radd(el, rdivk(rsub([dn, 1n], el), n)));
    }
    // integer reading
    const ppm = L === 0n ? M : (M * G) / (G + L);
    // exact reading: r = g / (g + l) = A / B with A = gn*ld, B = gn*ld + ln*gd (paisa units, not scaled)
    const A = eg[0] * el[1], B = eg[0] * el[1] + el[0] * eg[1];
    const ppmX = el[0] === 0n ? M : (M * A) / B;
    // the ratio bound |G/(G+L) - A/B| < n/(G+L), cross-multiplied with SCALE: G, L are scaled, A/B is a pure ratio
    const bound = (G + L === 0n) ? null : abs(G * B - A * (G + L)) < n * B;
    // the guard proposed: G + L >= 1_000 n  =>  |ppm - ppmX| <= 1_000
    const faithful = G + L >= 1_000n * n;
    out.push({ G, L, ppm, ppmX, bound, faithful, flat: G === 0n && L === 0n });
  }
  return out;
}

// RSI-1 (PARTS.md 3.5): every row, and the digits of the first two states (N1)
{
  const r = fold(3, [100, 103, 101, 104, 104, 98, 101, 107]).filter(Boolean);
  for (const x of r) console.log(`RSI-1 G ${x.G} L ${x.L} ppm ${x.ppm} exact-floor ${x.ppmX}`);
  const digits = (G, L) => { const num = 100n * G * 10n ** 15n / (G + L); const s = num.toString(); return `${s.slice(0, -15)}.${s.slice(-15)}`; };
  console.log(`RSI-1 first state 100G/(G+L) = ${digits(r[0].G, r[0].L)}; second = ${digits(r[1].G, r[1].L)}`);
}

// B1: check1's drift, then a long unchanged run, periods 2, 5, 14 (plus 3 and 20 for the record)
const drift = [100, 103, 101, 104, 102, 105, 103, 106, 104, 107, 105, 108, 106, 109, 107, 110, 108, 111, 109, 112, 110];
function runReport(label, n, head) {
  const len = 20 * n + 60;
  const flat = Array(len).fill(head.at(-1));
  const r = fold(n, [...head, ...flat]);
  const start = head.length - 1;             // index of the last drift candle (the run's start state)
  const s = r[start];
  let firstUnfaithful = null, firstDepart = null, firstFlat = null, worst = 0n, boundAll = true, guardAll = true;
  for (let t = start + 1; t < r.length; t++) {
    const x = r[t]; const d = abs(x.ppm - x.ppmX); if (d > worst) worst = d;
    if (x.bound === false) boundAll = false;
    if (x.faithful && d > 1_000n) guardAll = false;
    if (firstUnfaithful === null && !x.faithful) firstUnfaithful = t - start;
    if (firstDepart === null && (x.G === 0n || x.L === 0n) && x.ppm !== x.ppmX) firstDepart = { k: t - start, G: x.G, L: x.L, ppm: x.ppm, ppmX: x.ppmX, flat: x.flat, faithful: x.faithful };
    if (firstFlat === null && x.flat) firstFlat = t - start;
    if (x.ppmX !== s.ppmX) { console.log(`STOP: exact ppm moved in the run at +${t - start}`); }
  }
  console.log(`${label} period ${n}: start G ${s.G} L ${s.L} ppm ${s.ppm} exact ${s.ppmX} faithful ${s.faithful}; ` +
    `first unfaithful +${firstUnfaithful}; first 0/100 departure +${firstDepart?.k} (G ${firstDepart?.G} L ${firstDepart?.L} ppm ${firstDepart?.ppm} vs exact ${firstDepart?.ppmX}; flat ${firstDepart?.flat} faithful ${firstDepart?.faithful}); ` +
    `first flat +${firstFlat}; worst |ppm-exact| ${worst}; ratio bound held ${boundAll}; faithful=>within 1000 ppm ${guardAll}`);
}
for (const n of [2, 3, 5, 14, 20]) runReport('drift', n, drift);

// A price-like head: 100_000 paisa (Rs 1000), 5-paisa ticks, a deterministic xorshift walk of 200 candles
function walk(seed, len, base, tick) {
  let x = BigInt(seed) | 1n; const m64 = (1n << 64n) - 1n; const out = [base]; let p = base;
  for (let i = 1; i < len; i++) { x ^= (x << 13n) & m64; x ^= x >> 7n; x ^= (x << 17n) & m64; const k = Number(x % 5n) - 2; p += k * tick; if (p < tick) p = tick; out.push(p); }
  return out;
}
for (const seed of [7, 11, 13]) for (const n of [2, 5, 14]) runReport(`walk seed ${seed} (Rs 1000, 5 paisa ticks)`, n, walk(seed, 200, 100_000, 5));

// The bound and the guard over many random series with unchanged runs mixed in (not only one long run)
{
  let x = 0x9E3779B97F4A7C15n; const m64 = (1n << 64n) - 1n; const rnd = () => { x ^= (x << 13n) & m64; x ^= x >> 7n; x ^= (x << 17n) & m64; return x; };
  let checked = 0, boundBad = 0, guardBad = 0, unfaithful = 0;
  for (let trial = 0; trial < 300; trial++) {
    const n = 1 + Number(rnd() % 20n); const closes = []; let p = 1_000 + Number(rnd() % 100_000n);
    for (let i = 0; i < 120; i++) { const r = rnd() % 10n; if (r >= 6n) { /* unchanged */ } else { p += Number(rnd() % 21n) - 10; if (p < 1) p = 1; } closes.push(p); }
    for (const y of fold(n, closes)) { if (!y) continue; checked++; if (y.bound === false) boundBad++; if (!y.faithful) unfaithful++; if (y.faithful && abs(y.ppm - y.ppmX) > 1_000n) guardBad++; }
  }
  console.log(`random: ${checked} readings, ratio bound broken ${boundBad}, faithful-but-off-by-more-than-1000-ppm ${guardBad}, unfaithful readings ${unfaithful}`);
}
`````

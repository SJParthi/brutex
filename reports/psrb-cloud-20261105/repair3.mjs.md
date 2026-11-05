# repair3.mjs.md

Source: `state/strategy-psrb-20261103/specs-1105-repair3-work/repair3.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// repair3.mjs (Thu 5 Nov 2026, repair 3 of the 5 Nov PSRB specs; node BigInt, every price, count and product a BigInt).
// Written for repair 3 from the rules as the files state them: PARTS.md 5.3 (the two-band fold), trend.rs:265-358 (Atr)
// and daily.rs:181-253 (the ladder) at 1ad6e24ef8e0, PSRB-STV's fold at its tip (dual_band_supertrend.rs:190-246),
// PWC 4.3, 5.3, 6.3 and AD 2 and 5. It shares no code with repair1.mjs, repair2.mjs, check2.mjs or check3.mjs.
// Run: node repair3.mjs > repair3.out

const B = BigInt;
const SCALE = 1_000_000n;
const I64_MAX = (1n << 63n) - 1n;
const U64 = 1n << 64n;
let bad = 0;
const show = (v) => JSON.stringify(v, (_, x) => (typeof x === 'bigint' ? x.toString() : x));
function ok(name, got, want) {
  const g = show(got), w = show(want);
  if (g === w) console.log(`ok   ${name}: ${g}`);
  else { bad++; console.log(`FAIL ${name}: got ${g}, want ${w}`); }
}
const divE = (a, d) => { let q = a / d; if (a % d !== 0n && (a % d < 0n) !== (d < 0n)) q -= 1n; return q; }; // floor for d > 0
const abs = (a) => (a < 0n ? -a : a);
const bits = (a) => abs(a).toString(2).length;

// ---------------------------------------------------------------- trend::Atr, transcribed (trend.rs:265-358)
class Atr {
  constructor(p) { this.period = B(p); this.scaled = 0n; this.seed = 0n; this.pc = null; this.folded = 0n; this.seeded = false; }
  copy() { const a = new Atr(1); Object.assign(a, this); return a; }
  fold(h, l, c) {
    this.folded += 1n;
    let tr = h - l;
    if (this.pc !== null) { const hc = abs(h - this.pc), lc = abs(this.pc - l); if (hc > tr) tr = hc; if (lc > tr) tr = lc; }
    tr *= SCALE;
    if (this.seeded && this.folded > this.period) this.scaled += divE(tr - this.scaled, this.period);
    else { this.seed += tr; this.scaled = divE(this.seed, this.folded); this.seeded = true; }
    this.pc = c;
  }
  value() { return this.seeded ? divE(this.scaled, SCALE) : null; }
  warm() { return this.folded >= this.period; }
}

// ---------------------------------------------------------------- PSRB-STV's fold (PARTS 5.3; STV tip :190-246)
class Stv {
  constructor(p, f) { this.atr = new Atr(p); this.f = B(f); this.pc = null; this.up = 0n; this.lo = 0n; this.trend = 'Down'; this.seen = false; this.reading = null; this.ties = []; }
  fold(h, l, c) {
    const a = this.atr.copy(); a.fold(h, l, c);
    const v = a.value();
    if (v === null || !a.warm()) { this.atr = a; this.pc = c; this.reading = null; this.ties = []; return; }
    const mid = (h + l) / 2n; // midpoint of two positive prices: the floor
    const band = divE(v * this.f, 1000n);
    const bu = mid + band, bl = mid - band;
    const pc = this.pc;
    const up = (bu < this.up || (pc !== null && pc > this.up)) ? bu : this.up;
    const lo = (bl > this.lo || (pc !== null && pc < this.lo)) ? bl : this.lo;
    let trend = 'Down';
    if (this.seen) {
      if (this.trend === 'Down' && c > up) trend = 'Up';
      else if (this.trend === 'Up' && c < lo) trend = 'Down';
      else trend = this.trend;
    }
    // every comparison the fold makes, as a distance from its tie (for the mirror check)
    this.ties = [bu - this.up, bl - this.lo, c - up, c - lo];
    if (pc !== null) this.ties.push(pc - this.up, pc - this.lo);
    this.reading = { trend, line: trend === 'Up' ? lo : up, upper: up, lower: lo, settled: this.seen };
    this.atr = a; this.pc = c; this.up = up; this.lo = lo; this.trend = trend; this.seen = true;
  }
}
const fmt = (r) => (r === null ? '-' : `${r.trend} ${r.line} (${r.upper}, ${r.lower})`);
function runStv(p, f, bars) { const s = new Stv(p, f); return bars.map(([h, l, c]) => { s.fold(B(h), B(l), B(c)); return s.reading; }); }

console.log('== ST-5 and ST-2 through PSRB-STV (PARTS 5.4)');
const ST5 = [[121, 101, 118], [118, 102, 105], [106, 101, 103], [107, 101, 106], [120, 104, 118], [138, 109, 111], [113, 104, 104]];
const st5 = runStv(2, 500, ST5);
ok('ST-5, period 2, factor 500, candles 0-6', st5.map(fmt),
  ['-', 'Down 119 (119, 101)', 'Down 108 (108, 101)', 'Down 108 (108, 101)', 'Up 106 (108, 106)', 'Down 133 (133, 113)', 'Down 115 (115, 101)']);
const ST2 = [[126, 104, 106], [118, 104, 115], [120, 109, 110], [115, 108, 113], [118, 101, 101], [123, 100, 119], [135, 118, 130], [139, 126, 136],
  [136, 119, 123], [124, 116, 121], [126, 104, 108]];
const st2 = runStv(2, 1500, ST2);
ok('ST-2, period 2, factor 1_500, candles 0-10', st2.map(fmt),
  ['-', 'Down 138 (138, 84)', 'Down 135 (135, 93)', 'Down 126 (126, 96)', 'Down 126 (126, 96)', 'Down 126 (126, 96)', 'Up 101 (126, 101)',
    'Up 110 (154, 110)', 'Up 110 (151, 110)', 'Up 110 (138, 110)', 'Down 138 (138, 110)']);

// ---------------------------------------------------------------- AD 5.1 for a long, at candle level
// Entry on the signal candle s's close; stop = s's lower band. During candle t the resting stop in force is the one set
// after t - 1; a low at or below it fills the stop inside t. Else the flip: close_t below t's OWN lower band exits after t.
// Else (trailed row only) the stop moves up to t's lower band if higher. `prevBand` reads the flip against the band of t - 1.
function exitLong(bars, readings, s, trailed, prevBand = false) {
  let stop = readings[s].lower;
  for (let t = s + 1; t < bars.length; t++) {
    const [, l, c] = bars[t].map(B);
    if (l <= stop) return `stop ${stop} inside candle ${t}`;
    const band = prevBand ? readings[t - 1].lower : readings[t].lower;
    if (c < band) return `flip after candle ${t} (close ${c} below ${band})`;
    if (trailed && readings[t].lower > stop) stop = readings[t].lower;
  }
  return 'open at the end';
}
console.log('== the flip reads the band the candle itself sets (ST-5, long entered on candle 4)');
ok('ST-5 entry stop (candle 4 lower band)', st5[4].lower, 106n);
ok('ST-5 candle 5: low 109 above the stop 106, close 111 below its own lower band 113', [B(ST5[5][1]) > 106n, B(ST5[5][2]) < st5[5].lower, st5[5].lower], [true, true, 113n]);
ok('ST-5 trailed row', exitLong(ST5, st5, 4, true), 'flip after candle 5 (close 111 below 113)');
{ const r = exitLong(ST5, st5, 4, true, true); console.log(`  read against the previous band instead: ${r}`);
  ok('ST-5 read against the previous band (106): no exit at candle 5, the trade is still open after it closes', /candle 5\b/.test(r), false); }
ok('ST-5 candle 5 is wider than 2 x factor x ATR (range 29, ATR 20, factor 0.5)', [B(ST5[5][0] - ST5[5][1]), 20n * 1000n], [29n, 20000n]);

console.log('== a trailed and a fixed stop exit at different moments (ST-2, long entered on candle 6)');
const trailStops = []; { let stop = st2[6].lower; trailStops.push(`6:${stop}`); for (let t = 7; t <= 9; t++) { if (st2[t].lower > stop) stop = st2[t].lower; trailStops.push(`${t}:${stop}`); } }
ok('ST-2 entry stop and the trailed stop after candles 7-9', trailStops, ['6:101', '7:110', '8:110', '9:110']);
ok('ST-2 trailed row (VIDEO-ORIGINAL and AUTHOR-V2 form)', exitLong(ST2, st2, 6, true), 'stop 110 inside candle 10');
ok('ST-2 fixed row (VIDEO-ORIGINAL-FIXED-STOP form)', exitLong(ST2, st2, 6, false), 'flip after candle 10 (close 108 below 110)');
ok('ST-2 lows of candles 7-10 all above the fixed stop 101', ST2.slice(7).map((b) => B(b[1]) > 101n), [true, true, true, true]);

// ---------------------------------------------------------------- the C3 mirror (AD 1, N6 of check 3)
// Mirror: p -> K - p, the candle's high becomes K - low. Both streams start Down at the first ATR candle (TradingView's
// convention), so they can be alike, not opposite, until one turns alone. Once opposite, with every comparison at least
// 2 paisa from its tie, they stay opposite. A 1-paisa tie can break it.
function xorshift(seed) { let x = B(seed) & (U64 - 1n); return () => { x ^= (x << 13n) & (U64 - 1n); x ^= x >> 7n; x ^= (x << 17n) & (U64 - 1n); return x; }; }
let pairs = 0, synced = 0, keptOpposite = 0, brokeAt2 = 0, brokeAt1 = 0, alikeAfterSettle = 0;
for (let seed = 1; seed <= 400; seed++) {
  const rnd = xorshift(seed * 7919 + 13);
  const p = 1 + Number(rnd() % 12n), f = 500 + Number(rnd() % 4500n);
  const K = 2_000_000n; let price = 1_000_000n; const bars = [];
  for (let i = 0; i < 300; i++) { price += (rnd() % 2001n) - 1000n; const up = rnd() % 900n, dn = rnd() % 900n; const h = price + up, l = price - dn; const c = l + rnd() % (h - l + 1n); bars.push([h, l, c]); }
  const a = new Stv(p, f), m = new Stv(p, f); let inSync = false, minTie = null;
  for (const [h, l, c] of bars) {
    a.fold(h, l, c); m.fold(K - l, K - h, K - c);
    if (!a.reading || !a.reading.settled) continue;
    pairs++;
    const opp = a.reading.trend !== m.reading.trend;
    const tie = [...a.ties, ...m.ties].map(abs).reduce((x, y) => (x < y ? x : y));
    if (!inSync) { if (!opp) alikeAfterSettle++; if (opp) { inSync = true; synced++; minTie = tie; } continue; }
    minTie = tie < minTie ? tie : minTie;
    if (opp) keptOpposite++;
    else if (minTie >= 2n) brokeAt2++;
    else brokeAt1++;
  }
}
console.log(`  400 random series (periods 1-12, factors 500-4_999, 300 candles): ${pairs} settled pairs; ${alikeAfterSettle} settled candles read ALIKE before the two streams first turned opposite; ${synced} series turned opposite`);
ok('once opposite, with every comparison at least 2 paisa from its tie, the trends stay opposite (breaks)', brokeAt2, 0);
console.log(`  pairs kept opposite ${keptOpposite}; breaks after a comparison within 1 paisa of its tie: ${brokeAt1}`);
ok('some settled candles read alike before the streams first turn opposite (the start is not mirrored)', alikeAfterSettle > 0, true);

// ---------------------------------------------------------------- Williams %R pop staircase (PWC 5.3, 5.5; check 3 N1 a)
function staircase(n, start, jumpAt) {
  const N = 20000; n = B(n);
  const hi = [], lo = []; let next = start; let expH = 0, expL = 0, longest = 0;
  const bpH = [], bpL = [], remH = [], remL = []; let jumpsAfterFirst = 0, jumpsAtPeriod = 0, otherAtPeriod = 0;
  for (let k = 0n; k < B(N); k++) {
    const h = (k % n === (jumpAt === 'last' ? n - 1n : 0n)) ? 1_000_000n : 900_000n - (k % n), l = h - 10n;
    const s = next;
    const lenBH = hi.length, lenBL = lo.length;
    const eH = hi.length > 0 && ((s - hi[0][0] + U64) % U64) >= n ? 1 : 0;
    const eL = lo.length > 0 && ((s - lo[0][0] + U64) % U64) >= n ? 1 : 0;
    if (eH) hi.shift(); if (eL) lo.shift(); expH += eH; expL += eL;
    let a = 0; while (hi.length && hi[hi.length - 1][1] <= h) { hi.pop(); a++; } hi.push([s, h]);
    let b = 0; while (lo.length && lo[lo.length - 1][1] >= l) { lo.pop(); b++; } lo.push([s, l]);
    if (lenBH + 1 - hi.length - eH !== a || lenBL + 1 - lo.length - eL !== b) throw new Error('white-box count differs from the loop');
    bpH.push(a); bpL.push(b); remH.push(a + eH); remL.push(b + eL);
    longest = Math.max(longest, hi.length, lo.length);
    const isJumpAfterFirst = jumpAt === 'last' && k % n === n - 1n && k >= 2n * n - 1n;
    if (isJumpAfterFirst) { jumpsAfterFirst++; if (B(a + eH) === n) jumpsAtPeriod++; } else if (B(a + eH) === n) otherAtPeriod++;
    next = (next + 1n) % U64;
  }
  const st = (v) => { const w = [...v].sort((x, y) => x - y); return { total: v.reduce((x, y) => x + y, 0), p99: w[Math.floor(w.length * 0.99)], max: w[w.length - 1], over100: v.filter((x) => x > 100).length }; };
  return { highs: st(bpH), lows: st(bpL), remH: st(remH), remL: st(remL), expH, expL, longest, jumpsAfterFirst, jumpsAtPeriod, otherAtPeriod };
}
console.log('== the pop staircase: h_k = 1_000_000 when k mod n = n - 1, else 900_000 - (k mod n); l_k = h_k - 10; k = 0..19_999');
for (const start of [0n, U64 - 6n]) {
  const r14 = staircase(14, start, 'last'), r2k = staircase(2000, start, 'last');
  ok(`period 14, next from ${start}: highs total/p99/max/over100, lows total/p99/max`, [r14.highs.total, r14.highs.p99, r14.highs.max, r14.highs.over100, r14.lows.total, r14.lows.p99, r14.lows.max], [18564, 13, 13, 0, 18571, 1, 1]);
  ok(`period 14, next from ${start}: removals highs total/max, lows total/max, expiries, longest deque`, [r14.remH.total, r14.remH.max, r14.remL.total, r14.remL.max, r14.expH, r14.expL, r14.longest], [19991, 14, 19998, 2, 1427, 1427, 14]);
  ok(`period 2_000, next from ${start}: highs total/p99/max/over100, lows total/p99/max`, [r2k.highs.total, r2k.highs.p99, r2k.highs.max, r2k.highs.over100, r2k.lows.total, r2k.lows.p99, r2k.lows.max], [19990, 0, 1999, 10, 19989, 1, 1]);
  ok(`period 2_000, next from ${start}: removals highs total/max, lows total/max, expiries, longest deque`, [r2k.remH.total, r2k.remH.max, r2k.remL.total, r2k.remL.max, r2k.expH, r2k.expL, r2k.longest], [19999, 2000, 19998, 2, 9, 9, 2000]);
  ok(`period 14 and 2_000, next from ${start}: the highs remove exactly period at every jump after the first and at no other candle`,
    [r14.jumpsAfterFirst === r14.jumpsAtPeriod, r14.otherAtPeriod, r2k.jumpsAfterFirst === r2k.jumpsAtPeriod, r2k.otherAtPeriod], [true, 0, true, 0]);
}
const v2k = staircase(2000, 0n, 'first');
ok('VARIANT, the jump at k mod n = 0, period 2_000: highs back-pops and candles above 100 (the pins need the jump at n - 1)', [v2k.highs.total, v2k.highs.over100], [17991, 9]);

// ---------------------------------------------------------------- CCI at its ceiling, period u16::MAX (check 3 N1 c)
console.log('== CCI ceiling at period u16::MAX: 65,534 candles (1, 1, 1), then (2, 2, 2)');
{
  const n = 65535n; const ring = []; for (let i = 0; i < 65534; i++) ring.push(3n); ring.push(6n);
  const S = ring.reduce((x, y) => x + y, 0n); const dev = n * 6n - S; const D = ring.reduce((x, t) => x + abs(n * t - S), 0n);
  const milli = divE(200_000n * n * dev, 3n * D);
  ok('S, dev, D, D == 2 dev, milli', [S, dev, D, D === 2n * dev, milli], [196608n, 196602n, 393204n, true, 2184500000n]);
  ok('milli above i32::MAX and equal to 100_000 n / 3 exactly', [milli > 2147483647n, 100_000n * n % 3n === 0n && milli === 100_000n * n / 3n], [true, true]);
}

// ---------------------------------------------------------------- the ladder (daily.rs:181-253) and C1, C2 at the extremes
function ladder(H, L, C) {
  if (H < L) return 'HighBelowLow'; if (C < L || C > H) return 'CloseOutsideRange';
  const range = H - L; if (range > I64_MAX) return 'RangeOverflows';
  const p = divE(H + L + C, 3n), b = divE(H + L, 2n);
  const r1 = 2n * p - L, r2 = p + range, r3 = r1 + range, r4 = r3 + r2 - r1, r5 = r4 + r3 - r2;
  const s1 = 2n * p - H, s2 = p - range, s3 = s1 - range, s4 = s3 + s2 - s1, s5 = s4 + s3 - s2;
  const all = { p, b, tc: 2n * p - b, r1, r2, r3, r4, r5, s1, s2, s3, s4, s5, half: abs(p - b) };
  const out = Object.entries(all).filter(([, v]) => v > I64_MAX || v < -(1n << 63n)).map(([k]) => k);
  return out.length ? `LevelOverflows(${out.join(' ')})` : 'Ok';
}
const narrow = (H, L, C, k) => 10_000n * abs(2n * C - H - L) <= B(k) * (H + L + C);
console.log('== the i64-extreme bar and the flat previous session');
{
  const k = 9_214_157_878_975_800n; const H = 1001n * k, C = 1001n * k, L = 998n * k;
  ok('H = 1001k is 7 below i64::MAX', I64_MAX - H, 7n);
  ok('the extreme bar through the ladder', ladder(H, L, C), 'LevelOverflows(r1 r2 r3 r4 r5)');
  ok('its width test on the three integers: a tie at 10 bp, not narrow at 9', [10_000n * abs(2n * C - H - L), 10n * (H + L + C), narrow(H, L, C, 10), narrow(H, L, C, 9)],
    [276424736369274000000n, 276424736369274000000n, true, false]);
}
for (const P of [1n, 100_000n, I64_MAX]) {
  ok(`flat bar at ${P}: the ladder accepts it; narrow at k = 1, 10, 20, 30, 10_000`, [ladder(P, P, P), [1, 10, 20, 30, 10_000].map((k) => narrow(P, P, P, k))],
    ['Ok', [true, true, true, true, true]]);
}
const n5 = (today, earlier) => earlier.every(([H, L, C]) => abs(2n * today[2] - today[0] - today[1]) * (H + L + C) <= abs(2n * C - H - L) * (today[0] + today[1] + today[2]));
{
  const days = [[100_300n, 99_500n, 100_200n], [100_250n, 99_600n, 100_150n], [100_400n, 99_300n, 100_300n], [100_200n, 99_700n, 100_100n]];
  const flat = [100_000n, 100_000n, 100_000n];
  ok('N5-1 and N5-2 (the source bar is the previous session\'s)', [n5([100_200n, 99_700n, 100_100n], days), n5([100_200n, 99_699n, 100_101n], days)], [true, false]);
  ok('a flat source bar after N5-1\'s four days is the narrowest', n5(flat, days), true);
  ok('N5-1\'s source bar with day 2 flat is not the narrowest', n5([100_200n, 99_700n, 100_100n], [days[0], flat, days[2], days[3]]), false);
  ok('a flat source bar with a flat earlier day: a tie, the narrowest', n5(flat, [days[0], flat, days[2], days[3]]), true);
}
console.log('== k in basis points: a u16 in 1..=10_000');
{
  const refuse = (k) => (k < 1 || k > 10_000 ? `BasisPointsOutOfRange { bp: ${k} }` : 'accepted');
  ok('0, 10_001 and u16::MAX refused; 1 and 10_000 accepted', [0, 10_001, 65_535, 1, 10_000].map(refuse),
    ['BasisPointsOutOfRange { bp: 0 }', 'BasisPointsOutOfRange { bp: 10001 }', 'BasisPointsOutOfRange { bp: 65535 }', 'accepted', 'accepted']);
  const Tmax = 3n * I64_MAX;
  ok('10_000 |x| < 2^77 and k T < 2^79 at k = 10_000; at u16::MAX k T passes 2^79 (below 2^81), so the bound needs the refusal above 10_000', [10_000n * I64_MAX < (1n << 77n), 10_000n * Tmax < (1n << 79n), 65_535n * Tmax < (1n << 79n), 65_535n * Tmax < (1n << 81n)], [true, true, false, true]);
}
console.log('== C2 bounds (AD 2.2; check 3 N4)');
{
  const M = I64_MAX;
  // over every corner of 1 <= L <= C <= H <= i64::MAX and the close in [1, i64::MAX]
  const corners = []; for (const H of [1n, M]) for (const L of [1n, M]) for (const C of [1n, M]) if (L <= C && C <= H) corners.push([H, L, C]);
  let max3c = 3n * M, max2T = 0n, minR = null, maxR = null, minS = null, maxS = null;
  for (const [H, L, C] of corners) { const T = H + L + C; if (2n * T > max2T) max2T = 2n * T; const r = 2n * T - 3n * L, s = 2n * T - 3n * H;
    minR = minR === null || r < minR ? r : minR; maxR = maxR === null || r > maxR ? r : maxR; minS = minS === null || s < minS ? s : minS; maxS = maxS === null || s > maxS ? s : maxS; }
  ok('|3 close| < 2^65; 2T < 2^66; 2T - 3L in (0, 2^65); 2T - 3H in (-2^63, 2^65)', [max3c < (1n << 65n), max2T < (1n << 66n), minR > 0n && maxR < (1n << 65n), minS > -(1n << 63n) && maxS < (1n << 65n)], [true, true, true, true]);
  ok('bits: 3 close, 2T, max 2T - 3L, max 2T - 3H', [bits(max3c), bits(max2T), bits(maxR), bits(maxS)], [65, 66, 65, 65]);
  ok('C8: |close| x SCALE < 2^83', M * SCALE < (1n << 83n), true);
}

console.log(bad === 0 ? '== ALL OK' : `== ${bad} FAILED`);
`````

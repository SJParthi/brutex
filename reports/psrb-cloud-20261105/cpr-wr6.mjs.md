# cpr-wr6.mjs.md

Source: `state/strategy-psrb-20261103/work-willr-cci/model/cpr-wr6.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// cpr-wr6.mjs (5 Nov 2026): re-checks WR-1's seventh candle, the narrow-CPR and R1/S1 fixtures of the addendum.
import { willr } from './wr-cci.mjs';
const wr1 = [[110, 100, 105], [115, 104, 114], [113, 101, 102], [120, 108, 119], [118, 112, 112], [110, 100, 103], [121, 105, 120]];
const a = willr(3, wr1); const r6 = a[6], r5 = a[5];
console.log('WR-1 c6', `HH ${r6.HH} LL ${r6.LL} C ${r6.C} U ${r6.U} R ${r6.R} ppm ${r6.ppm} above-85 ${r6.above(-850000)} below-15 ${r6.below(-150000)}`,
  'crossed_above(-85)', !r5.flat && !r5.above(-850000) && r6.above(-850000), 'crossed_below(-15)', !r5.below(-150000) && r6.below(-150000));
const B = BigInt; const abs = (x) => (x < 0n ? -x : x);
const cpr = (h, l, c) => { h = B(h); l = B(l); c = B(c); const x = 2n * c - h - l, T = h + l + c; return { x, T, ax: abs(x) }; };
const narrow = (s, bp) => 10000n * s.ax <= B(bp) * s.T;
for (const [nm, bar] of [['CPR-1 edge', [100100, 99800, 100100]], ['CPR-2 just narrow', [101000, 99000, 100150]], ['CPR-2b just wide', [101000, 99000, 100151]]]) {
  const s = cpr(...bar); console.log(nm, bar.join(','), `x ${s.x} T ${s.T}`, [9, 10, 20, 30].map((k) => `${k}bp:${narrow(s, k)}`).join(' '),
    `width*3 ${s.ax} P*3 ${s.T} lhs ${10000n * s.ax} rhs10 ${10n * s.T}`); }
// R1 / S1 exact against the floored daily.rs ladder
const fdiv = (a, b) => { const q = a / b; return (a % b !== 0n && ((a < 0n) !== (b < 0n))) ? q - 1n : q; };
for (const bar of [[100100, 99800, 100100], [10007, 10000, 10001]]) { const [h, l, c] = bar.map(B); const T = h + l + c; const p = fdiv(T, 3n);
  const r1f = 2n * p - l, s1f = 2n * p - h; const r1x = `(${2n * T - 3n * l})/3`, s1x = `(${2n * T - 3n * h})/3`;
  const tests = [r1f - 1n, r1f, r1f + 1n, r1f + 2n].map((cl) => `c=${cl}: exact>${3n * cl > 2n * T - 3n * l} floored>${cl > r1f}`).join('; ');
  const testsS = [s1f - 1n, s1f, s1f + 1n].map((cl) => `c=${cl}: exact<${3n * cl < 2n * T - 3n * h} floored<${cl < s1f}`).join('; ');
  console.log('PIV', bar.join(','), `T ${T} p_floor ${p} R1 exact ${r1x} floored ${r1f}; S1 exact ${s1x} floored ${s1f}`); console.log('   R1:', tests); console.log('   S1:', testsS); }
// narrowest of the last five: relative widths |x| / T compared by cross-multiplication (ties count)
const days = [[100300, 99500, 100200], [100250, 99600, 100150], [100400, 99300, 100300], [100200, 99700, 100100], [100200, 99700, 100100]];
const s = days.map((d) => cpr(...d)); s.forEach((v, i) => console.log('day', i, days[i].join(','), `x ${v.x} T ${v.T} bp*100 ${(1000000n * v.ax) / v.T}`));
const today = s[4]; console.log('narrowest-5 (ties count):', s.slice(0, 4).every((v) => today.ax * v.T <= v.ax * today.T));
const d2 = [...days.slice(0, 4), [100200, 99699, 100101]]; const s2 = d2.map((d) => cpr(...d)); const t2 = s2[4];
console.log('variant last day', d2[4].join(','), `x ${t2.x} T ${t2.T}`, 'narrowest-5:', s2.slice(0, 4).every((v) => t2.ax * v.T <= v.ax * t2.T));
const M = (1n << 63n) - 1n; console.log('u128 bound bits: |x| T at i64 extremes', ((M - 1n) * (3n * M)).toString(2).length);
`````

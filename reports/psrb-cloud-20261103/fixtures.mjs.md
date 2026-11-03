# fixtures.mjs.md

Source: `state/strategy-psrb-20261103/work/fixtures.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// fixtures.mjs: prints the exact expected values of the PSRB fixtures in PARTS.md (node only).
import { Latch, TwoBand, C, rsi, rsiExact, boll } from './sim.mjs';
function show(name, n, m, raw) { const L = new Latch(n, m), T = new TwoBand(n, m); console.log(`### ${name}: atr period ${n}, factor ${m} thousandths`);
  raw.forEach((b, i) => { const c = C(...b); L.fold(c); T.fold(c); const r = T.reading;
    console.log(`| ${i} | ${b.join(', ')} | ${L.atr.value()} | ${L.trend} ${L.stop} | ${r ? `${r.trend} ${r.line} (${r.upper}, ${r.lower})${r.settled ? '' : ' first'}` : '-'} |`); }); }
show('ST-1 the seed', 2, 1000, [[110,90,100],[112,100,110],[115,105,114],[116,108,115],[116,109,111],[112,96,98],[100,90,95]]);
show('ST-2 a tighter line at a flip down', 2, 1500, [[126,104,106],[118,104,115],[120,109,110],[115,108,113],[118,101,101],[123,100,119],[135,118,130],[139,126,136],[136,119,123],[124,116,121],[126,104,108]]);
show('ST-3 a tighter line at a flip up, then an earlier flip down', 2, 1000, [[143,138,142],[142,126,126],[128,119,124],[144,123,140],[142,119,124],[137,123,137],[141,119,119],[123,114,116]]);
show('ST-4 a flat series', 3, 3000, [[100,100,100],[100,100,100],[100,100,100],[100,100,100],[100,100,100]]);
show('ST-5 equal histories, a wide bar', 2, 500, [[121,101,118],[118,102,105],[106,101,103],[107,101,106],[120,104,118],[138,109,111],[113,104,104]]);
show('ST-6 period 1, the first bar', 1, 1000, [[110,90,100],[106,100,105],[109,104,108]]);
show('ST-7 a gap through the upper band', 2, 1000, [[101,99,100],[101,99,100],[102,100,101],[121,118,120],[122,119,121]]);
show('ST-8 a gap through the lower band', 2, 1000, [[101,99,100],[101,99,100],[102,100,101],[121,118,120],[122,119,121],[96,93,94]]);
console.log('### RSI-1 (period 3)'); for (const r of rsi(3, [100,103,101,104,104,98,101,107])) console.log(`| ${r.c} | ${r.warm} | ${r.g} | ${r.l} | ${r.ppm ?? '-'} |`);
console.log('### RSI exact (period 3)'); rsiExact(3, [100,103,101,104,104,98,101,107]).forEach((x, i) => console.log(i, x ? `G=${x.G[0]}/${x.G[1]} L=${x.L[0]}/${x.L[1]}` : '-'));
console.log('### RSI alternating (period 2)'); for (const r of rsi(2, [100,101,100,101,100,101])) console.log(`| ${r.c} | ${r.warm} | ${r.g} | ${r.l} | ${r.ppm ?? '-'} |`);
console.log('### BOLL-1 n=8'); for (const z of [1999, 2000, 2001]) { const b = boll(8, [200,400,400,400,500,500,700,900], z)[7]; console.log(`z ${z}: S ${b.S} Q ${b.Q} V ${b.V} D ${b.D} above ${b.above} below ${b.below} upper_floor ${b.uf} lower_ceil ${b.lc}`); }
console.log('### BOLL-2 n=20 steady trend 100..119'); const tr = Array.from({ length: 20 }, (_, k) => 100 + k); for (const z of [1647, 1648]) { const b = boll(20, tr, z)[19]; console.log(`z ${z}: S ${b.S} Q ${b.Q} V ${b.V} D ${b.D} above ${b.above} upper_floor ${b.uf}`); }
const dn = Array.from({ length: 20 }, (_, k) => 119 - k); for (const z of [1647, 1648]) { const b = boll(20, dn, z)[19]; console.log(`down z ${z}: S ${b.S} V ${b.V} D ${b.D} below ${b.below} lower_ceil ${b.lc}`); }
`````

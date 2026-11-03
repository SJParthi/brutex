# fix2.mjs.md

Source: `state/strategy-psrb-20261103/work/fix2.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
import { Latch, TwoBand, C } from './sim.mjs';
function show(name, n, m, raw) { const L = new Latch(n, m), T = new TwoBand(n, m); console.log(`### ${name}: atr period ${n}, factor ${m} thousandths`);
  raw.forEach((b, i) => { const c = C(...b); L.fold(c); T.fold(c); const r = T.reading;
    console.log(`| ${i} | ${b.join(', ')} | ${L.atr.value()} | ${L.trend} ${L.stop} | ${r ? `${r.trend} ${r.line} (${r.upper}, ${r.lower})${r.settled ? '' : ' first'}` : '-'} |`); }); }
show('ST-9 equal bands while up', 2, 500, [[119,108,108],[119,105,119],[129,116,128],[139,128,136],[156,130,151],[151,138,138],[142,137,141],[171,124,155]]);
`````

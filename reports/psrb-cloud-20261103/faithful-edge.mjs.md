# faithful-edge.mjs.md

Source: `state/strategy-psrb-20261103/repair1-work/faithful-edge.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// repair1: the fold sequence named in PARTS.md 3.5 `the_faithful_edge_is_one_thousand_units_per_period`, period 14:
// 15 closes of 100, then 101, then closes of 101; prints each reading's G, L, faithful, flat (the integer rule of 3.3).
const SCALE = 1_000_000n, divE = (a, b) => { const q = a / b; return (a % b !== 0n && (a < 0n) !== (b < 0n)) ? q - 1n : q; };
const n = 14n; let prev = null, k = 0n, gs = 0n, ls = 0n, G = 0n, L = 0n; const rows = [];
const closes = [...Array(15).fill(100), 101, ...Array(25).fill(101)];
closes.forEach((c0, i) => { const c = BigInt(c0); if (prev === null) { prev = c; return; } const ch = c - prev; prev = c; const u = ch > 0n ? ch : 0n, d = ch < 0n ? -ch : 0n; k++;
  if (k <= n) { gs += u * SCALE; ls += d * SCALE; if (k < n) return; G = divE(gs, n); L = divE(ls, n); } else { G += divE(u * SCALE - G, n); L += divE(d * SCALE - L, n); }
  rows.push(`close #${i + 1} (${c0}): G ${G} L ${L} faithful ${G + L >= 1000n * n} flat ${G === 0n && L === 0n}`); });
rows.forEach(r => console.log(r));
`````

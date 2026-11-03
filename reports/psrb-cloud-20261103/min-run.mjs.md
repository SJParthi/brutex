# min-run.mjs.md

Source: `state/strategy-psrb-20261103/repair1-work/min-run.mjs` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````js
// repair1: the fewest unchanged closes after a nonzero change before the RSI state turns unfaithful (G + L < 1_000 n)
// or departs (G == 0 or L == 0 with the other nonzero), at period n. A nonzero change is at least 1 paisa = SCALE units;
// its smallest effect is G' = floor(((n-1) G + SCALE) / n) >= floor(SCALE / n) with L' = floor((n-1) L / n). The
// smallest state after it is G = floor(SCALE / n), L = 0 (L already 0); then each unchanged close maps X -> floor((n-1) X / n).
// For "departs" the smallest case needs BOTH nonzero: the smallest is G = floor(SCALE/n) and L = 1.
const SCALE = 1_000_000n;
const step = (x, n) => ((n - 1n) * x) / n;   // x >= 0: floor
for (const nn of [1, 2, 3, 5, 10, 14, 20, 30, 31, 32, 50, 100]) {
  const n = BigInt(nn); let G = SCALE / n, L = 0n, k = 0;
  const startFaithful = G + L >= 1000n * n;
  while (G + L >= 1000n * n) { G = step(G, n); L = step(L, n); k++; if (k > 100000) break; }
  // fewest unchanged closes to G == 0 from G = floor(SCALE / n)
  let g = SCALE / n, z = 0; while (g > 0n) { g = step(g, n); z++; }
  console.log(`n ${nn}: smallest state after a 1-paisa move G ${SCALE / n}; faithful then ${startFaithful}; unfaithful after >= ${startFaithful ? k : 0} unchanged closes; that G reaches 0 after ${z}`);
}
`````

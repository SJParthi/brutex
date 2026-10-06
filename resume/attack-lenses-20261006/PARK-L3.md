# PARK — lens L3 permutations — 2026-10-06 12:3x UTC

Branch `attack/permutations`, head `8635409` (pushed). Base `origin/final/all-fixes` 969493e (no newer commits; nothing to merge).

## Pushed and validated at 8635409
Validation: cargo fmt --check; cargo clippy --workspace --all-targets -D warnings; tests as non-root for every touched crate (indicators, vocab, runner; cli and api green on the D-3403/D-3407 heads; pull green on D-3404); static gates 0–30 except 1e (full build, CI's); Gate 18 pre-run: indicators diff 81 mutants (76 caught, 3 unviable, 2 missed → killed, 2/2 caught), runner D-3407 diff 6/6 caught. D-3408 and the XPERM-04/05/06 doc tests are test/doc-only (no production mutants).

| item | fixed? | commit | evidence |
|---|---|---|---|
| F-25A074 candlestick midpoints floored (162/164/212/226) | yes | f9ab789 | XPERM-02; failed before (`prior 100->103, bar 104->101`) |
| F-3B4D5A SuperTrend stop from three floors (64/65 wrong side) | yes | 30a8cde | XPERM-03; failed before (`100 < 100.7`, `minute 10 ...`) |
| F-109F13 charter/evaluator said pull keeps unmeasured-Muhurat minutes | yes | 364ce11 | XPERM-04; failed before on base charter |
| F-36228D bit table defined 34 crossings by previous bar | yes | 3c5c237 | XPERM-05; failed before (`row 280 still says`) |
| F-8D5073 vocabulary counts/kinds not the table's | yes | 3c5c237 | XPERM-06; failed before (`row 235 ... says —`) |
| F-0486DA worst_reward_risk_bp n=1 → i64::MAX | yes | 24c7e3a (+62f815e) | XPERM-07; failed before (`[-30]: worst 9223372036854775807 above payoff 0`); gate18 6/6 |
| F-1D5275 trades_needed_for doc values/threshold | yes (doc) | bc1e1f4 | XPERM-08; failed before (`still says: coin-flip floor: eleven trades`) |
| F-DBC24E EMA sides | withdrawn | cee4ccb | duplicate of ind1-2 (wip/zero/numeric-edges de48df5), D-3400 |

Refuted (recorded in D-3401/D-3404/D-3407): VWAP floor, daily pivot floor, gap ladder floor (documented floors); SuperTrend seed past warm (D-1542); SuperTrend seed tie; OverlapWindow exit > i+H (latent, test-only path).
Already tracked (not touched): ind1-2 (EMA sides, gap mid 66/67, bit 68), STO-2, D-3130/31, p9num-2, c4a-7/D-1496, D-1439, D-1497, slice08 F2, numeric-pass18 Note A / p18num-1.

## Open
- Round 6 (the "documented number/example the code does not reproduce" class sweep over the whole scope) was started and STOPPED at the park; no results. Rounds 1–5 each found ≥1 defect, so the lens has NOT reached a zero round.
- Needs the owner (UNVERIFIED): VWAP exact vs floored mean; daily pivot and gap-ladder floors (IF-23/D-1861) vs exact; SuperTrend seed tie rule; cli `statistical_floor_ppm` doc premise ("no combination passes" below the floor holds only at exactly the sizing rate).

## Resume
`git fetch origin attack/permutations final/all-fixes && git checkout attack/permutations && git merge origin/final/all-fixes`; re-run round 6 (two read-only agents: scope A indicators/engine/vocab/core, scope B runner/costs/greeks/pull fold-calendar-session; brief: every doc-stated measured value / worked example / always-never property recomputed against the code, skipping test-pinned ones), then rounds until one finds zero.

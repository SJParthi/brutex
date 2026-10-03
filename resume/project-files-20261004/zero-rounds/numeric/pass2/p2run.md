# Pass 2 numeric audit: SLICE p2run

Commit 5140aca (origin/final/all-fixes-zero). This was an audit only. No repository file was changed.
Scope: runner/src/report.rs, topn.rs, bound.rs, excursion.rs, trade.rs, and api/src/livejson.rs, plus the grid.rs `Cell` ratio helpers that bound.rs compares against.

## Verdict
One new finding, and it is low: the report text contradicts its own numbers in a degenerate case. I found no new precision, overflow, look-ahead or complexity defect in this slice. The livejson `clears_bar` issues are already logged as xcut-1 and run1-3, and the code still matches that description. The stop placed at a floored level is already logged as run2-1.

## Findings

### p2run-1 (low): with one effective hypothesis, SIGNIFICANCE says there is no threshold while FINDINGS judges rows against 1.96
- report.rs:329-352 and 383-411 (`significance`, `significance_counts`): `if n < 2 { row(out, "threshold", "-", "too few hypotheses to have a noise floor"); return; }`
- report.rs:556 (`render_findings_at`): `let bar = crate::significance::bonferroni_t(n);`. Then report.rs:605: `let clears = judgeable && s.edge.t.abs() >= bar;`
- Why it is wrong: both sections use the same effective trial count (the D-comment at :552-555 makes that point). At n = 1 the page states there is no threshold. The section directly below it then prints "bar every row must clear 1.96" and marks a row with n >= 30 and |t| >= 1.96 as "clears". The page therefore gives two opposite statements about the same bar.
- Probe (ran: $SCRATCH/probes/p2run, path dependency on crates/runner):
```
n=0 bonferroni_t=0.0000 bar_milli=0 expected_max_bailey=0.0000
n=1 bonferroni_t=1.9600 bar_milli=1959 expected_max_bailey=0.0000
n=2 bonferroni_t=2.2414 bar_milli=2241 expected_max_bailey=0.5198
```
  At n = 1, SIGNIFICANCE prints `threshold -` and FINDINGS prints `bar every row must clear 1.96`. I did not render the full page; the two branches above are what produce those lines.
- Reachability: `effective_trials = trials - redundant` (runner/src/lib.rs:519, :646). It equals 1 only when a sweep produced exactly one non-redundant hypothesis, so this case is degenerate. n = 0 together with non-empty rows (bar 0.00, so every judgeable row clears) is not reachable through `RankedOutcome`, because any kept row makes trials at least 1.
- Fix: make the two sections agree. Either print the n = 1 Bonferroni bar in SIGNIFICANCE, or render rows as unjudged when n < 2.

## Checked and clean
- bound.rs `Verdict` against grid.rs `win_rate_bp` (:394), `reward_to_risk_bp` (:592) and `guaranteed_floor` (:637). The ratios floor and are compared `<` against an integer threshold. Because floor(x) >= T exactly when x >= T for integer T, flooring cannot hide a breach. `guaranteed_floor` uses i128 and saturates. Flat trades count as losses in `losses = trades - wins` but subtract 0 because of `max(0)`, which is conservative. Zero trades and zero winners are refused before the ratios are read. `Hundredths` handles negative values through `unsigned_abs`.
- topn.rs: `Range::normalise` uses i128, refuses values outside the first-pass extrema, returns NEUTRAL when `low == high`, and its floor division is monotone. `score` uses u128 with `checked_div`. `Ord` is total: admitted, score, pessimistic_profit, assurance, mask, direction, digest. Insertion uses strict `>`, so equal rows keep their arrival order, and the held list is capped at 25. The population proof hashes every field, including the `Option` discriminants. Scoring None at the neutral midpoint is the documented V1 policy.
- report.rs: `permille` is integer math, floors, and guards whole == 0. `paisa()` saturates. A NaN mean would print 0, but no reachable path produces one: when n == 0 the row is already "TOO FEW". A NaN t gives "BELOW". The MISPAIRED verdict is checked first, and REFUSED EXITS are summed with saturation.
- excursion.rs: `ppm_of` widens to i128, truncates, and guards non-positive moves and entries. A crossing test `floor(ppm) >= rung` is exact for an integer rung. `from_excursions`, the nested select from the deepest position down, matches the sorted order statistics, and `rung_position`'s `checked_shl` handles 64 and above. Arming takes effect on the bar after the crossing, and the since-entry peak equals the since-arming peak after arming, so it does not inflate the armed give-back. A refused first bar leaves the low and high runs at their sentinels, which `ppm_of` turns into 0.
- trade.rs: entry is signal + 1 on a Signal column with an exact one-step timestamp check, and fills use only the entry and exit bars' open, high and low. The horizon exit uses an exact timestamp HashMap lookup, which is expected O(1). The prefix arrays `refused_within`, `path_accepts`, `first_*_within` and `next_marked` are O(1) per query and their indices check out. `actual_square_off` does its day arithmetic in i128. `pnl` saturates, and `worst <= best` holds by construction of the anchors. One per-trade scan remains, `walk_core`, which is O(rows) per candidate and already documented (o1eng2 r-1/r-2).
- livejson.rs: `clears_bar` still ignores n < 30 and MISPAIRED, and still compares a rounded t against a truncated bar, exactly as xcut-1 and run1-3 describe. Not new.

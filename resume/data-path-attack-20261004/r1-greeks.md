# Attacker GREEKS, round 1 (`greeks`)

## 1. Attacks
All test paths are in crates/greeks/tests/attack_greeks.rs.

| attack | cases tried | failures | fixed? | evidence |
|---|---|---|---|---|
| Price bounds max(0,F-DK) <= C <= F (puts too), price >= 0, delta in [0,e^-qT] / [-e^-qT,0], gamma >= 0, vega >= 0, finite. Random stream: moneyness 0.01..100, tenor 1 minute..2 years, vol 0.1%..500%, r in [-6%,14%], q in [-3%,7%] | 200,000 | 0 | n/a | `price_and_greek_signs_hold_on_a_random_stream_of_contracts` |
| Extreme grid: spot/strike in {5e-324, MIN_POSITIVE, 1e-300, EPS, 1e-6, 1, 25851.19, 1e6, 1e11, 1e12, 1e12+ulp, MAX, inf, NaN, -0.0, 0, -1}, 11 tenors (incl. 5e-324, 0, -0.0, NaN, 100+ulp), 13 vols (incl. subnormal, 0, negative, NaN, 10+), 8 rates, 3 carries, call+put. Every Ok is checked against the bounds above and round-tripped through the solver | 1,983,696 evaluated / 132,712 Ok / 3,740 solved | 1 (subnormal scale) | yes, D-3101 | `extreme_inputs_never_produce_a_non_finite_or_out_of_bounds_ok`, `a_subnormal_scale_reports_the_last_place_its_quote_really_has`; fix at crates/greeks/src/solver.rs:345-346 (+ const at :184) |
| IV round trip, price -> IV -> price, same grid as row 1. Each Ok must satisfy rel err <= 1e-9 + 10*uncertainty, reproduce the price to 1e-9, and use <= 86 evaluations. Any refusal must be a named one | 120,000 (12,713 solved, 4,129 by Newton) | 0 | n/a | `implied_volatility_round_trips_within_its_stated_uncertainty` |
| Exact bounds: price = intrinsic, intrinsic-ulp, -0.0, 0, -1, = maximum, max+ulp, 1e300, NaN, +-inf, intrinsic+ulp, maximum-ulp, the price at exactly vol 5.0, and at 4.99 | 2 kinds x 15 | 0 | n/a | `prices_at_and_beyond_the_no_arbitrage_bounds_are_refused_by_name` |
| Normal CDF/PDF: monotone on a 1e-4 grid over [-40,40] and ulp by ulp across +-SPLIT and +-37; N(x)+N(-x)==1 exactly; pdf symmetric; +-inf, NaN, +-0 | ~800,000 + 16,000 ulp steps | 0 | n/a | `the_normal_distribution_is_monotone_symmetric_and_total` |
| Moneyness, on-grid strikes built as atm + k*interval with k in -20000..20000, call+put, position/distance flip | 482,032 | before the fix, (1e9,0.03) refused 19,200/40,001 on-grid strikes as OffGrid; (1e8,0.007) refused 2,560; (1.5e5,1e-5) refused 12,520; (1e5,3e-6) refused 23,508 | yes, D-3100 | `an_on_grid_strike_is_never_refused_however_large_the_level`; fix at crates/greeks/src/moneyness.rs:135-150 |
| Moneyness: ladder finer than f64 can resolve; a genuine half step is still refused; MAX_STEPS still enforced | 6 | 1 (atm=strike=1e12, interval 1e-3 answered ATM when no rung can be resolved) | yes, D-3100 | `a_ladder_too_fine_to_resolve_is_refused_by_name_and_a_half_step_still_is` |
| Solver cost bound: arbitrary prices inside the bounds (not prices the model produced), iterations <= MAX_ITERATIONS == 86 | 20,000 | 0 | n/a | `the_solver_cost_is_a_constant_independent_of_the_input` |
| Price monotone in vol, spot and strike (relative bump 1e-6), exploratory only, not kept as a test | 400,000 | 340 vol inversions, each 1-2 ulp in deep-ITM prices where vega*dvol is below one ulp | not a defect | rounding. The solver documents this already ("cannot see that the computed price is not strictly monotone", docs/06-limits.md §29) |

Both regression tests were run against the unmodified HEAD solver.rs and moneyness.rs (I swapped the files in temporarily, then restored them) and FAILED there:
- the subnormal test: uncertainty 1.55e-10 against an actual error of 8.6e-9
- the on-grid test: `atm 1e9 interval 0.03 k -19998: ... -19998.00000190735 steps ... not a whole number within 0.000001`
- the too-fine test: Ok(ATM)
- the extremes test: the same subnormal case

All of them pass with the fix.

## 2. Fixes: draft decisions and invariants

**D-3100: The moneyness step tolerance widens to the strike's representational slack, and a ladder too fine to resolve is refused by name.**
`Moneyness::from_ladder` compared the step count against a fixed 1e-6. That count carries the rounding of the strike, of the at-the-money level and of their difference, about one ulp of the larger operand divided by the interval. When the level is large against the interval, that slack exceeds 1e-6 and strikes built on the grid are refused as `OffGrid`: 19,200 of 40,001 at level 1e9 and interval 0.03. The tolerance is now `max(STEP_TOLERANCE, REPRESENTATION_ULPS(=4) * EPSILON * max(strike, atm) / interval)`. At ordinary levels it is still exactly 1e-6, so the existing `OffGrid { steps: 0.5, tolerance: STEP_TOLERANCE }` assertions are unchanged. When the slack would reach a quarter step (level/interval > `MAX_LEVEL_TO_INTERVAL` = 0.25/(4*EPS), about 2.8e14), neighbouring rungs cannot be told apart in f64. That input is refused as `OutOfRange { field: "level_to_interval" }` instead of being answered by a guess. Realistic NSE ladders (rupee levels up to about 1e5, intervals of 0.5 and up) are far from both edges. The API is unchanged; two public consts are added.

| DPG-01 | `Moneyness::from_ladder(atm + k*interval, atm, interval, kind)` returns `steps == k` with the correct position for every positive on-grid strike, at any level whose level/interval ratio is <= `MAX_LEVEL_TO_INTERVAL` | `an_on_grid_strike_is_never_refused_however_large_the_level` |
| DPG-02 | A ladder with level/interval above `MAX_LEVEL_TO_INTERVAL` is refused as `OutOfRange{level_to_interval}`; a half step is still `OffGrid`; the tolerance at ordinary levels is exactly `STEP_TOLERANCE` | `a_ladder_too_fine_to_resolve_is_refused_by_name_and_a_half_step_still_is` |

**D-3101: The implied-volatility uncertainty screen floors the quote's granularity at the subnormal spacing.**
The solver's `uncertainty` is defined as how far one unit in the last place of the quote moves the answer. It was computed from `price_scale * EPSILON`. Below `f64::MIN_POSITIVE`, doubles are spaced uniformly at 2^-1074, so that product understated the real last place by up to the full subnormal range. Measured case: S = K = `f64::MIN_POSITIVE`, T = 100, r = 0.0946, q = 0.05, sigma = 1. It was answered with uncertainty 1.55e-10 while the answer was wrong by 8.6e-9 of itself, so the stated last place was about 150x finer than any double at that magnitude. The numerator is now `max(scale*EPSILON/vega, 2^-1074/vega)`. That is identical at every normal scale, and neither quotient can be NaN (the existing argument still holds), so `max` hides nothing. The input is absurd for a market, but it is accepted by `Contract::check`, and the field's own definition was wrong there.

| DPG-03 | For every `Ok` from `implied_volatility`, `uncertainty >= 2^-1074 / (vega * volatility)`, and the subnormal witness's error is within its reported uncertainty | `a_subnormal_scale_reports_the_last_place_its_quote_really_has` |
| DPG-04 | Every `Ok` greeks value is finite, priced inside the no-arbitrage bounds within 1e-12 of scale, non-negative, with delta inside [0,e^-qT] or [-e^-qT,0] and gamma, vega >= 0, over a 1.98M-point extreme grid and 200k random contracts | `extreme_inputs_never_produce_a_non_finite_or_out_of_bounds_ok`, `price_and_greek_signs_hold_on_a_random_stream_of_contracts` |
| DPG-05 | The IV solve costs at most `MAX_ITERATIONS` = 86 model evaluations for any input, including arbitrary prices inside the bounds | `the_solver_cost_is_a_constant_independent_of_the_input` |

## 3. Commands
- `CARGO_BUILD_JOBS=2 cargo test -p greeks --locked`: lib 43 passed; attack_greeks 9 passed, 1 ignored (timing); standalone 1; vendor_anchor 11; doctests 4. All green.
- The same attack_greeks run against HEAD solver.rs/moneyness.rs: 5 passed, 4 failed (the regressions listed above).
- `CARGO_BUILD_JOBS=2 cargo clippy -p greeks --all-targets --locked -- -D warnings`: clean.
- `cargo fmt -p greeks --check`: clean.
- `cargo test -p greeks --locked --release --test attack_greeks per_call_latency_report -- --ignored --nocapture`: passed.

Not run: `cargo deny`, coverage, mutants. Downstream `pull` (crates/pull/src/pricing.rs calls `Moneyness::from_ladder`) was not rebuilt by me. The API is unchanged; behaviour differs only where 4*EPS*level/interval > 1e-6, i.e. level/interval > ~1.1e9.

## 4. Per-call latency (release, this shared box, Instant per call, includes ~20-30 ns of timer overhead)
| op | n=1e3 p50/p99/max | 1e4 | 1e5 | 1e6 |
|---|---|---|---|---|
| price | 195/316/664 ns | 135/308/36,715 | 135/282/41,580 | 162/385/1,124,324 |
| greeks | 192/286/685 | 139/275/21,697 | 140/331/696,599 | 146/373/3,305,756 |
| solve_iv | 905/10,847/91,115 | 632/11,022/4,158,983 | 605/9,260/4,086,882 | 631/10,190/3,085,673 |

p50 and p99 are flat across N. The solver p99 of about 10 us is the bisection path: 86 evaluations at about 120 ns each. The ms-scale maxima appear at every N and look like scheduler preemption on a shared machine; I did not confirm that. The cost per solve is constant by construction: at most 2 + 8 + 75 + 1 = 86 evaluations, asserted.

## 5. Needs real vendor data
- Whether real NIFTY/BANKNIFTY/stock option quotes land in the Indeterminate band, and how often. The round-trip test's worst accepted relative IV error was 7.5e-4: the screen accepts up to its 1e-3 bound, as documented in limits §29. Quote frequencies need GDFL/Zerodha option chains.
- The real NSE strike intervals per underlying are still UNVERIFIED (moneyness.rs header). D-3100's edges are far from any plausible rupee ladder, but that is not confirmed.

## 6. Files modified
- crates/greeks/src/moneyness.rs (D-3100: tolerance, two new public consts, Errors doc)
- crates/greeks/src/solver.rs (D-3101: SUBNORMAL_GAP const, uncertainty floor)
- crates/greeks/tests/attack_greeks.rs (new)

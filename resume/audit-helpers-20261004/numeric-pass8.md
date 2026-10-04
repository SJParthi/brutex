# Numeric / statistical-correctness audit, pass 8: the out-of-sample and multiple-testing machinery

Head: `/home/claude/wt/zero3` @ **1f4de71** (origin/final/all-fixes-zero). Audit only, nothing edited.
Scope: `crates/runner` bootstrap, bootstrap_family_pass, bootstrap_zero_v2, significance, pbo, rank, split,
family/search allocation, validate (walk-forward), expression_oos, outcome's t; the cli callers
(index-stop qualification, Boolean admission/qualification, Selection V6, live board, report).

## Verdict

**1 new finding (p8num-1, medium).** Every resampling procedure matches its published definition
or departs from it in a way the repo already records (D-0465, D-1448, D-1549, D-0742/D-1990, pst-3,
p2stat). The new defect sits beside them. The n >= 30 floor that is meant to make a NORMAL Bonferroni bar
valid for a t-statistic does not do so at this engine's trial counts. Under the module's own premise
(`Edge::t` is Student-t with n-1 df), a row with n = 30 that "CLEARS" has a per-row false-positive rate
11x to 1,300x the budgeted one.

## Table

| statistic | file:line | definition used | matches literature? | edge handling | verdict |
|---|---|---|---|---|---|
| White Reality Check (legacy) | runner/src/bootstrap.rs:420-514 | `sqrt(n)·max_k mean_k` vs bootstrap `max_k sqrt(n)(mean*_k - mean_k)`. Non-studentized. p = `(#{>=}+1)/(B+1)`, stated at :488-497 | yes (White 2000; Davison-Hinkley +1) | draws=0, periods<2 or all-constant family give p=1. block>MAX_BLOCK or block>periods returns None (D-1990) | OK |
| White receipt / fused | bootstrap.rs:698-743; bootstrap_family_pass.rs:233-258,306-330 | same, exact `matched_or_exceeded+1 / draws+1` (D-0465) | yes | refuses draws=0, block=0, periods<2, positive statistic on a point-mass null (D-0972) | OK |
| Hansen SPA (legacy + receipt + fused) | bootstrap.rs:563-696, 762-816; family_pass :222-230,248-256 | `T = max(0, max_k mean_k/se_k)`. Recentre kept rows `mean*-mean`, leave gated rows `mean*` (= SPA_c, `g_c`). Gate `-sqrt(2 ln ln n)`. Bootstrap floored at 0. Same `>=`+1 rule | yes except: se is i.i.d. not long-run (D-1549, documented). se held fixed across resamples, as Hansen's omega-hat | n<=3: gate -inf (keep all, conservative). se=0 gives 0 contribution. All-negative family gives p=1 | OK (known deviation) |
| Romano-Wolf stepdown (legacy) | bootstrap.rs:1188-1211, 1466-1590 | studentized by full-sample se. One B x n matrix. Reject iff `(1+#{max>t})/(B+1) <= alpha`, `admissible=floor((B+1)alpha)` | count rule = RW 2016 Alg 4.1 (charter :590). RW 2005 §4.2 re-studentizes each resample by its own sigma*; here sigma-hat is fixed (valid weighted max-type test; same scale class as D-1549) | zero-se never rejected, stops the walk. alpha>1 or draws=0 gives empty. Ties: caller position | OK (note) |
| Romano-Wolf adjusted p | bootstrap.rs:1276-1430; family_pass :400-520 | suffix maxima + cumulative max of `(strict+1)/(B+1)` | yes (RW 2016 Alg 4.1 + monotone step) | any zero-se row refuses. Strict `>` vs White/SPA `>=` is a charter decision (D-0460/D-0465) | OK |
| RW zero-v2 | runner/src/bootstrap_zero_v2.rs:258-330, 470-500 | all-zero rows p=1 (not in RW family). Nonpositive statistic p=1. Positive rows take the shared adjusted p | sound: `max(0,M)>t <=> M>t` for t>0 | nonzero constant refuses. Bounds on work/bytes checked u128 | OK |
| Stationary bootstrap + RNG | bootstrap.rs:80-121, 1687-1711 | Politis-Romano wrap-around, restart prob `floor(1e6/b)` ppm. SplitMix64 seeded only by the caller. Modulo bias stated | yes | block 0/1 = iid. block>periods refused (D-1990). ppm quantization documented (D-0742). One seed reproduces one matrix for all three procedures by design. Families need no independence (union bound) | OK |
| Bonferroni bar | runner/src/significance.rs `bonferroni_t` | `z_{1-0.05/(2N)}`, NORMAL quantile, two-sided on `abs(t)` (direction is the sign, so it is charged) | Bonferroni yes. The reference distribution is the problem: see **p8num-1** | N=0 gives 0. N = this run's duplicate-deflated mask count. Exit grid printed as a separate ceiling (cli/src/lib.rs:5604-5633). Instruments/rungs not charged per run (CLAUDE.md §1 states in-sample pool results mean nothing) | **p8num-1** |
| Expected max (sqrt(2 ln N), Bailey E[max]) | significance.rs `expected_max_t`, `expected_max_bailey` | `(1-gamma)Z^-1(1-1/N) + gamma Z^-1(1-1/(Ne))` | yes (DSR paper's E[max] term only) | N<2 gives 0. Display only | OK |
| Deflated Sharpe Ratio | none | not implemented: no skew/kurtosis terms, no variance of trial SRs. Only the E[max] term above exists | n/a | n/a | absent (not claimed as a computed DSR) |
| t-statistic (Newey-West) | runner/src/outcome.rs:2160-2510 | Bartlett weight `1-d/H` on bar distance, `sum/(n-1)/n` | NW with bandwidth H: yes | m2<=0 or one window holding all hits gives t=0. Non-finite se gives 0 | OK |
| Wilson lower 95% | cli/src/population_statistics_v2.rs:5037-5058; institutional_evidence.rs:2073; v3 :3275 | `(p + z²/2n - z·sqrt(p(1-p)/n + z²/4n²))/(1+z²/n)`, z=1.959964 | yes | trades=0 gives 0 (v2) or refuses (inst.). wins>trades: v2 clamps `min`, inst. refuses. Floor to ppm is the conservative direction for a minimum gate | OK |
| CSCV PBO (cli) | cli/src/population_observations_v1.rs:1245-1330, 1521-1590 | equal blocks, S even <=16 dividing T, `C(S-1,S/2)` splits (segment 0 always test), doubled midrank, overfit iff rank > median | partial: one orientation per pair (D-1448, charter :588). Median tie not overfit vs Bailey's lambda<=0 (pst-3) | no even divisor refuses. Unrankable splits counted. PBO ppm ceiling (D-0743 fixed) | known |
| Anchored bottom-half "PBO" (runner) | runner/src/pbo.rs:193-212, 401-411 | IS first strict max; OOS doubled midrank | labelled not-CSCV. Legacy `place` floors half-ranks (p2stat), still used by cli/src/lib.rs:20259-20279 | <2 candidates unrankable | known |
| Benjamini-Hochberg | significance.rs `benjamini_hochberg` | step-up `p_(k) <= k/m·alpha` | yes | refuses p outside [0,1]. No production caller (gaps-3) | OK |
| Family allocation (Bonferroni) | runner/src/family_allocation_v1.rs | `alpha/(batches·rungs)`, exact u128, `<=` | yes (union bound) | scope/alpha validated. Unexecuted cells keep their share | OK |
| Countable alpha spending | runner/src/search_allocation_v1.rs | `alpha/(8(b+1)(b+2))`, telescopes to alpha | yes | `minimum_draws = ceil(d/n)-1` exact. Index-stop qualification scales RW/White/SPA by it (cli/src/index_stop_qualification_numeric.rs:612-620) | OK |
| Walk-forward / purge | runner/src/split.rs; validate.rs:1740-1790 | anchored/rolling, purge h bars. min_hits scaled by ceil. Side and exit chosen on TRAIN only | no leakage found | `bars < splits+1` gives no folds | OK |
| Expression OOS | runner/src/expression_oos.rs:255-266 | later slice must start a strictly later IST session than the training's last bar. Training grid frozen | no leakage | empty column refuses | OK |
| Qualification split | cli/src/index_stop_qualification_numeric.rs:277-302 | CSCV on `training`, bootstrap/RW/White/SPA on `later`, the whole family (no selection on later data) | yes | n/a | OK |

## New findings

### p8num-1 (medium): the n >= 30 floor does not make the normal Bonferroni bar valid for a t-statistic at the trial counts this engine produces

- **Where.** `crates/runner/src/report.rs:249-264` (`MIN_OBSERVATIONS`) and its use at `report.rs:608-609`.
  The same applies to `/live.json` `clears_bar` via `cli/src/live.rs:214` and `api/src/livejson.rs:244-245`.
  The bar comes from `runner/src/significance.rs` `bonferroni_t` (normal quantile).
  ```rust
  /// `Edge::t` is Student-t with `n - 1` degrees of freedom; every threshold in
  /// `crate::significance` is a NORMAL quantile. They converge as `n` grows and
  /// diverge sharply below about thirty ...
  pub const MIN_OBSERVATIONS: u64 = 30;
  ...
  let judgeable = s.edge.n >= MIN_OBSERVATIONS;
  let clears = judgeable && s.edge.t.abs() >= bar;
  ```
- **Why it is wrong.** "Converge by about thirty" holds for 5% tails. A Bonferroni bar sits at tail
  probability `0.05/(2N)`, and there the Student-t and normal quantiles separate badly at 29 df. Take the
  doc's own premise. At n = 30 the per-row false-positive probability of `abs(t) >= bonferroni_t(N)`, as a
  multiple of the budgeted `0.05/N`, is:

  | N (trials) | normal bar | t(29) bar | t(29) tail at normal bar / budget | union bound on FWER |
  |---|---|---|---|---|
  | 316 | 3.778 | 4.339 | 4.6x | 0.23 |
  | 3,689 | 4.351 | 5.225 | 11.3x | 0.57 |
  | 1,000,000 | 5.451 | 7.289 | 145x | >1 |
  | 61,125,295 | 6.141 | 8.924 | 1,322x | >1 |

  The doc's worked example, 729x at n = 13, is the reason the floor exists. At n = 30 and N = 61M the
  overspend is larger still. The ranker orders by `abs(t)` with no n preference, so rows just over the floor
  with large `abs(t)` are exactly the rows that reach the top and print `CLEARS` / `clears_bar: true`. The
  web counts `clears_bar` as "proved" (xcut-1). Newey-West overlap lowers the effective df further, so these
  figures are a lower bound on the gap.
- **Repro.** Ran. Numerical integration of the t(29) density (Simpson, 4e5 steps) and normal bisection in
  awk, script `scratchpad/t.awk`. The normal bar reproduces the crate's own test value
  (`bonferroni_t(316) = 3.78`, significance.rs test `the_published_bonferroni_figure_is_reproduced`).
  No cargo run was needed: the defect is a property of the two reference distributions, and both inputs are
  the crate's published constants.
- **Minimal fix.** Judge each row against the Student-t quantile at its own `n-1` df,
  `t_{n-1}^{-1}(1 - 0.05/(2N))`. That needs an inverse incomplete beta, Rust only, no dependency. The
  alternative is to keep the normal bar and refuse a verdict until the t(n-1) and normal quantiles at that
  tail agree within a stated tolerance, which makes the floor a function of N rather than the constant 30.
  Correct the `MIN_OBSERVATIONS` doc either way, and add a decision entry and an invariant with a test at
  (n = 30, N = 3,689).

## Notes (not findings)

- RW (both APIs) and SPA hold the full-sample i.i.d. se fixed across resamples, where RW 2005 §4.2
  re-studentizes. The test stays a valid weighted max-type test. This belongs to D-1549's documented class
  and is noted so a future doc edit can name it as well.
- Per-run Bonferroni charges neither the 210 instruments nor the 8 rungs. The live board marks every run
  `ranked_only`, and CLAUDE.md §1 says pooled in-sample results need OOS validation. The qualification path
  does charge batches x rungs (allocations above), so no new item.
- Known items confirmed unchanged at 1f4de71: D-1448 (half-orientation CSCV), pst-3 (median tie), p2stat
  legacy `pbo::place` half-rank floor (cli/src/lib.rs:20275), D-1549, D-0742 ppm quantization, gaps-3 (BH
  has no production caller).

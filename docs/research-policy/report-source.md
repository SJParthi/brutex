# What the 37 research settings mean

Prepared 7 September 2026. These are automated **historical research acceptance
checks**, not 37 things the operator must calculate manually. There are 39
policy settings in the existing engine: 37 values in the supplied profile and
two settings resolved outside it (the run's maximum single-trade loss and the
inherited worst-case reward/risk threshold, whose default is 3:1).
That default is a research choice, not a threshold supplied by the weekly
winning-day requirement. Intraday-only execution and the fixed **15:10 IST deadline** are
additional mandatory execution rules, not numbers supplied by this profile.

The operator explicitly delegated selecting the missing values. The supplied
`config/intraday-research-v1.toml` therefore contains **assistant-selected
research choices**, not an invented pre-existing institutional approval.
“Institutional” is the repository's name for its evidence pipeline. It is not
certification by an institution, exchange, regulator, or independent assessor.

The profile is fixed before inspecting sweep winners. Editing it creates a new
research configuration; repeatedly changing thresholds until something passes
is another search and must not be represented as untouched confirmation.
No costs are added by this change. A cost-excluded result remains cost-excluded.
No threshold here is a position size, leverage recommendation, or account loss
budget. Price-based quantities are per price unit, not cash-account returns.

## Quick comparison

| Group | Plain question | Example of a result it challenges |
|---|---|---|
| Sample | Is there enough evidence? | Four profitable trades across two days |
| Loss and return | Are the gains worth the measured setbacks? | A positive total with a very large drawdown |
| Concentration | Does one lucky event dominate? | One trade supplies most of the profit |
| Later periods | Does it hold up outside the fitting window? | Excellent training results and losing test periods |
| Statistical evidence | Did we account for searching many alternatives? | The best of a million weak candidates looks impressive by chance |
| Data and execution | Can the candles actually support the claimed outcome? | A bar touches both stop and target and the event order is unknown |

## The complete 37-setting table

**R** means the existing run's maximum loss in price units (`MAX_POINTS × 100`
paisa). For illustration only, if a run specifies 50 points, R is 5,000 paisa,
10R is 500 points, and −R is −50 points. This example does not select 50 points
for a campaign. Percentages below use the engine's exact parts-per-million
representation; 50,000 ppm is 5%, and 1,500,000 ppm as a ratio is 1.5×.

| Group | Setting | Plain-language check | Research limit | Purpose and boundary |
|---|---|---|---|---|
| Sample | min_support_hits | Enough matching signals | At least 40 | A matching signal is not necessarily an entered or completed trade. At 200 this floor demanded the combination fire on about seven percent of all bars over the swept window, which mandates a grinder rather than filtering one. |
| Sample | min_independent_sessions | Enough trading days | At least 30 | Counts distinct qualifying sessions; that alone cannot prove statistical independence. |
| Sample | min_trades | Enough completed trades | At least 40 | Avoids qualifying a strategy on a handful of outcomes. This is a screening floor, not a proof of adequate statistical power. At 200 it forced about 2.4 trades a week, which a setup meant to fire seldom cannot meet. |
| Loss and return | max_mae_paisa | Worst movement against a trade | At most R | Measures adverse excursion even if the price later recovers. |
| Loss and return | min_win_rate_ppm | Winning trade share | At least 25% | Counts completed trades, alongside the effective loss and reward/risk limits. The index workflow separately requires at least 60% winning eligible days. |
| Loss and return | min_wilson_win_rate_ppm | Cautious win-rate estimate | At least 14% | Requires the Wilson lower bound to clear the floor; the method’s assumptions still matter. At 35%, and even at 20%, this bound was unsatisfiable at the profile’s own minimum trade count: forty trades at the 25% point floor give a Wilson lower bound of 141,871 ppm. |
| Loss and return | min_return_drawdown_ppm | Return compared with drawdown | At least 2× | The pessimistic return must be at least twice the measured drawdown. |
| Loss and return | min_weakest_period_return_paisa | Worst required period | At least −R | Allows a bounded losing period without hiding it behind a strong total. |
| Statistical evidence | max_pbo_ppm | Estimated backtest overfitting | At most 20% | Limits the pipeline's PBO estimate. It does not state an 80% probability of future success. |
| Statistical evidence | max_fwer_p_value_ppm | Search-wide adjusted evidence | At most 5% | Requires the family-wise adjusted p-value to clear the research threshold. |
| Statistical evidence | max_spa_p_value_ppm | Superior predictive ability | At most 5% | Tests against the recorded benchmark while accounting for alternatives; not a win probability. |
| Later periods | min_decided_folds | Usable walk-forward windows | At least 6 | Enough historical train/test windows must produce an actual decision. |
| Data and execution | max_ambiguous_fill_rate_ppm | Unclear candle event order | 0% | No admitted trade may rely on an ambiguous fill outcome. The sweep may still record rejected candidates. |
| Data and execution | max_gap_affected_rate_ppm | Gap-affected outcomes | 0% | No admitted trade may be marked as affected by a gap. This does not replace source completeness checks. |
| Concentration | max_session_concentration_ppm | Dependence on one day | At most 25% | One session cannot contribute more than a tenth of the trade count. |
| Concentration | max_largest_trade_profit_share_ppm | Dependence on one lucky trade | At most 50% | Limits the measured share of GROSS WIN attributable to the largest trade. At 10%, admitting a winner N times the typical one required 9N+1 winning trades, so a tenfold outlier needed 91 winners and the best trade was capped near the average win. |
| Loss and return | max_drawdown_paisa | Largest accumulated setback | At most 3R | Caps the peak-to-trough loss in price units; not an account-percentage limit. |
| Loss and return | max_losing_trade_rate_ppm | Losing trade share | At most 75% | A rate remains comparable across study lengths. |
| Loss and return | max_losing_trades | Total losing-trade count | No extra absolute cap | Explicit u64 maximum. The 60% rate and losing-streak limits still apply; long studies are not penalized only for being long. |
| Loss and return | min_pessimistic_profit_paisa | Conservative total result | At least 5R | Requires a positive result with a margin relative to the run's risk unit. Excluded costs remain excluded. |
| Sample | min_winning_trades | Enough winning trades | At least 10 | Adds an absolute sample floor alongside the win-rate rule. |
| Loss and return | min_average_win_paisa | Average winning amount | At least 1 paisa | Requires a positive measured average; stronger size checks come from reward/risk and profit factor. |
| Loss and return | max_average_loss_paisa | Average losing amount | At most R | Keeps average loss within the stated run risk unit. |
| Loss and return | min_profit_factor_ppm | Total winning amounts versus losing amounts | At least 1.5× | Requires total gains to exceed total losses with a margin. Missing or undefined evidence must not be invented. |
| Loss and return | max_consecutive_losing_streak | Longest losing streak | At most 10 | Challenges prolonged sequences of losses in the recorded trade order. |
| Loss and return | min_consecutive_winning_streak | Required winning streak | 0; none required | A lucky streak is not a condition for admission. This is an explicit neutral value, not a missing setting. |
| Statistical evidence | min_bootstrap_draws | Statistical resamples | At least 1,000 | Minimum procedure resolution. Resampling existing observations does not create new market history. |
| Statistical evidence | min_bootstrap_strategies | Compared strategy count | At least 2 | A family comparison needs alternatives. This floor never authorizes omitting searched candidates. |
| Statistical evidence | min_bootstrap_periods | Aligned observation periods | At least 60 | Requires a common history for the compared strategies and resampling tests. |
| Statistical evidence | min_pbo_contributing_folds | Usable overfitting splits | At least 6 | Requires valid ranking placements to contribute to the estimate. |
| Statistical evidence | max_pbo_unrankable_folds | Unrankable overfitting splits | At most 0 | A broken or unusable ranking split cannot disappear from the assessment. |
| Later periods | min_profitable_oos_folds | Profitable later test windows | At least 4 | Requires repeated positive results in walk-forward test windows; also keep a strictly later untouched holdout. |
| Later periods | min_oos_pessimistic_return_paisa | Conservative later-period total | At least 1 paisa | Requires a positive aggregate out-of-sample result, not just positive training results. |
| Statistical evidence | max_white_reality_p_value_ppm | White Reality Check evidence | At most 5% | A family-level check for data snooping against the recorded benchmark. |
| Statistical evidence | require_white_reality_rejection | White test actually reached rejection | Required | A missing result or decorative p-value is insufficient. |
| Statistical evidence | max_romano_wolf_p_value_ppm | Candidate-specific adjusted evidence | At most 5% | Uses the candidate's adjusted result after multiple comparisons. |
| Statistical evidence | require_romano_wolf_rejection | Candidate test actually reached rejection | Required | Requires an explicit Romano–Wolf decision for that candidate. |

The two settings outside this profile are **maximum single-trade loss = R**,
and **smallest positive pessimistic trade / largest losing trade ≥ the effective
reward/risk threshold**. The inherited default ratio is 3; an explicit named
override can change it. No observed loss leaves this ratio unmeasured, not an
automatic pass. Backtest shows the effective value when the policy resolves.
The supplied file intentionally cannot overwrite those two fields. The separate
NIFTY/BANKNIFTY winning-day, weekly and loss-streak requirements still apply.

## Why these values, and what the sources actually support

The exact numbers above are **research design choices made under delegation**.
There is no paper establishing 200 trades, 60 sessions, 10R drawdown, 10% profit
concentration, 20% PBO, or 5% p-values as universally correct for this strategy
universe. The choices emphasize enough observations, limited concentration,
explicit loss limits, and more than one kind of statistical evidence. Zero
ambiguity and zero gap-affected trades intentionally favor refusal over an
uncertain execution claim. Returning no admitted strategy is a valid result.

Wilson intervals incorporate sample size into an interval for a binomial
proportion. Applying that interval to dependent trading outcomes still needs
care; 60 distinct days do not magically make trades independent.
[NIST confidence intervals](https://www.itl.nist.gov/div898/handbook/prc/section2/prc241.htm).

Searching many configurations raises the danger of selecting noise. The PBO
paper describes a framework based on in-sample selection and out-of-sample
ranking. This supports keeping the complete compared population and valid
splits; it does not certify our chosen 20% cutoff.
[Bailey et al., The Probability of Backtest Overfitting](https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf).

White's Reality Check addresses data snooping across a specification search.
Hansen's SPA modifies that approach using studentization and a sample-dependent
null distribution. These are methods with assumptions, not guarantees that a
selected strategy will remain profitable.
[White (2000)](https://users.ssc.wisc.edu/~behansen/718/White2000.pdf),
[Hansen (2005)](https://www.tandfonline.com/doi/abs/10.1198/073500105000000063).

Romano–Wolf methods address multiple testing and dependence among test
statistics. Their guarantees have stated assumptions and asymptotic limits.
Correctly retaining the entire candidate population matters more than merely
displaying a method name.
[Romano and Wolf (2005)](https://users.ssc.wisc.edu/~behansen/718/RomanoWolf2005.pdf).

A p-value is not the probability a strategy is true, safe, or profitable, and
decisions should not rest on crossing one numerical threshold alone. We retain
the measured values and separate loss, sample, concentration, and later-period
checks instead of presenting a 5% cutoff as assurance.
[American Statistical Association statement](https://www.amstat.org/asa/files/pdfs/p-valuestatement.pdf).

## What is automated, and what each status means

| Stage | Automatic behavior | Honest outcome |
|---|---|---|
| Read the profile | Load one bounded regular file, validate all 37 fields and the runner's full policy domains | Malformed, duplicate, unknown, absent, changed-during-read or overflowing inputs refuse |
| Explain it | `cli policy-check FILE MAX_POINTS` resolves all 39 rules and prints their units and meaning | Configuration checked; no strategy evaluated |
| Start an institutional run | `BRUTEX_ADMISSION_POLICY_FILE` selects the server-owned file before historical sizing | Existing strict data, clean-build and resource preconditions still apply |
| Override a setting | Existing explicit `BRUTEX_ADMIT_...` knobs take precedence | Override source is visible; a malformed override cannot silently fall back |
| Evaluate candidates | Existing admission machinery compares real observed evidence with the resolved policy | Passed, failed, unmeasured and refused remain distinct |
| Save evidence | Existing canonical admission records bind the resolved numeric policy to the evidence and decision; reports include the selected file digest | Policy file provenance is not itself a completed campaign or proof that a particular record was written |
| Show this guide | Regenerate the searchable table from this document and the actual profile file | A guide has no invented strategy pass/fail results and is not a live monitor |

The live service has not been restarted to select this profile. Boolean/cash
successor evidence work and full release verification are separate remaining
tasks. A new profile does not silently extend the two-index Selection V6 format.

## Edge cases we must distinguish

| Situation | Required behavior |
|---|---|
| Too little history for the selected floors | Record insufficient evidence or rejection; do not lower the floor automatically |
| No profitable candidate | Complete with no admitted strategy if the search itself completed correctly |
| Missing or corrupt history | Refuse source admission; never generate replacement candles |
| Unknown indicator used with NOT | Preserve unknown; absence is not a true opposite signal |
| Same bar touches stop and target | Preserve execution ambiguity and apply the explicit policy |
| Missing 15:09 one-minute evidence or exit after 15:10 | Refuse the claimed fixed-deadline execution |
| A single trade dominates results | Apply the concentration rule even when total profit looks attractive |
| A statistical split cannot rank candidates | Count or refuse the unrankable evidence; never remove it silently |
| File typo, duplicate field, zero required count or out-of-range rate | Refuse configuration before market computation |
| A risk multiple overflows the price domain | Refuse arithmetic; never wrap or truncate |
| Memory, disk or candidate budget is exhausted | Record interruption/refusal and preserve resumable evidence; never label partial enumeration complete |
| Thresholds are retuned after observing winners | Treat the edited profile as another research trial, preserving earlier results |

These are specified and testable classes, not a claim that every possible error
or market history has been exhaustively proven. Total enumeration, history
storage and I/O cannot all have constant time and constant space. Fixed-size
per-operation paths and bounded memory are separate, measurable contracts.

## Research trace

Scope: explain the actual 37 fields, choose an explicit research profile under
the operator's delegation, and distinguish method evidence from threshold
choices. Primary sources were preferred; the existing runner policy schema and
validators are the authority for field names and units. Search batches covered
PBO, Wilson intervals, White, Hansen, Romano–Wolf and p-value interpretation.
The first Hansen PDF fetch failed; the primary publisher abstract was then
read. No inaccessible text is claimed as inspected. No new source established a
universal acceptance threshold. The planning tool was searched for and was not
available; the sequence used was schema discovery, primary-source checks,
profile implementation, explanation generation, then verification.

This file is the canonical prose and comparison source for the generated guide.

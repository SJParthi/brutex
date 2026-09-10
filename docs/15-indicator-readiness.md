# Sweep input and indicator readiness evidence

This is a code and finite-test inventory, not a guarantee over every possible
market history or a profitability claim. The vocabulary is the authority for
bit names. Indicator functions produce those bits, the forward column preserves
their source bars, and the runner joins that column to the engine. The engine
never reads candles or recomputes indicators.

Earlier focused verification on 2026-09-06 passed **483 tests, zero failures and
zero ignored tests** across engine and indicators, including all seven new
`sweep_predicate_readiness` tests, the complete 62-exemplar translation/session
reset test, exact gap-overlay availability, and the three engine readiness
oracle tests. Both crates also passed all-target Clippy with warnings denied.
The exact logs are `/private/tmp/brutex-sweep-final-engine-indicator-tests-20260906.log`
and `/private/tmp/brutex-sweep-final-engine-indicator-clippy-20260906.log`.
These are local evidence paths, not portable CI artifacts. Later edits require
fresh relevant verification. Complete coverage and whole-module mutation
verification remain unestablished; the measured mutation limitations are below.

## Comparison a reader can check

| Need | Implemented behavior | Evidence and honest boundary |
|---|---|---|
| Every condition has an owner | 328 live positions: 234 emitted by indicator modules, 4 aggregate state conditions, 85 crossing/ordinal conditions, 5 weekday conditions | `evaluator::the_position_set_is_the_union_of_the_modules` checks completeness, uniqueness and vocabulary agreement. 39 forming-day pivot slots remain void, 3 old slots retired; no invented emitters. |
| Wide masks | Six `u64` words, 384 positions, unchanged identities | `engine/tests/sweep_readiness_oracle.rs` compares an independent exhaustive subset oracle across bits63/64,127/128,192,274,320,369. |
| Search through extinction | Apriori joins and subset pruning over conjunctions; exact documented implication pruning | 378 finite matrix cells compare every frequent output and accounting. This does not certify arbitrary OR/NOT trees as an Apriori search space. |
| Caller asks for impossibly many workers | Requested support lanes are a scheduling upper bound, limited by process parallelism | `a_maximum_requested_lane_count_is_a_scheduling_bound_and_preserves_the_answer` compares `usize::MAX` with one lane. No depth cap or mask truncation. |
| Recoverable allocation failure | Engine input copy, singleton staging, prior-frontier indices, pending batches, support-count storage, output growth and level-record capacity reserve fallibly | Allocation refusal produces `Breach::Memory`; OS thread creation refusal produces `Breach::Workers`; a halted sweep does not claim extinction. OS overcommit, standard-library internal allocation and process abort are not recoverable guarantees. |
| Honest missing data for NOT | Parallel `Column::known()` certifies truth and selected available false predicates | Unknown never means false. All emitted truths are known. Negative availability is deliberately conservative; family details below. |
| Cold rollover | A regular bar enters the column only when the evaluator was warm before folding and remains warm after rollover | `audit_readiness::a_session_rollover_cannot_admit_a_mask_after_its_daily_anchor_becomes_unusable` uses a positive valid candle whose derived daily ladder overflows. |
| Bad direct prices | Structurally valid zero/negative candles return `PriceNotPositive`, leaving evaluator state unchanged | `extremes` independently enumerates refusal cases across modules; `sweep_predicate_readiness::nonpositive_prices_refuse_without_poisoning_state_and_positive_extremes_survive` includes minimum/maximum integers and one paisa. |
| Durable audit | The indicator/engine boundary returns masks, source indices, availability, census and explicit halts | These crates intentionally emit no events inside bar/candidate loops. CLI/store own persistence. This document does not claim per-operation DB writes or universal durability. |
| Constant-time claim | Fixed-width bit lookup/evaluation and fixed-size indicator state have constant per-operation bounds | Scanning N bars is O(N), materializing columns is O(N) memory, sorting a frontier is O(F log F), and exhaustive subset search is combinatorial. Hash membership is expected O(1), vector append amortized O(1). No total O(1) latency/space claim. |

## Every indicator family

| Family and owned live positions | Emission and lookback | Existing executable evidence | False-answer availability |
|---|---|---|---|
| Daily pivot/CPR, 44 | `daily::bits`, completed eligible prior daily levels; explicit level-overflow refusal | `the_plan_agrees_with_the_vocabulary`, `the_band_sides_admit_nothing_on_their_own_edge`, `a_rung_past_the_type_refuses_the_whole_ladder`, `the_three_width_classes_fall_where_the_cuts_say` | Plain comparisons known with a usable ladder. Near predicates additionally require a valid base/tolerance. CPR class requires a meaningful range. |
| Previous-day/previous-five Fibonacci, 27 | `fib::prev_day_bits`, `Prev5::bits`; five completed sessions for the latter | `fib_known_readiness`: independent levels, all 27 true/false/NOT positions, cold five-session history, widths, overflow, nonregular sessions and Column handoff | False is known only for each representable level with usable range and matching tolerance. Current daily bars cannot fill the prior five sessions. |
| Current-day Fibonacci, 11 | `CurDayFib::step`, prior bars in current session, ordered extreme leg; emit before fold | `current_day_fib_known_readiness`: independent pre-fold oracle, establishment/flip/erasure, reset, equality/band and overflow boundaries | Uses the same pre-fold state as truth on the same day. A leg first established by this bar cannot certify this bar's false answers. |
| Opening ranges, 20 | `Orb::step`, 5/15/30/60-minute windows, freeze before boundary-bar evaluation | `orb_known_readiness` independently tests all 20 predicates and their negations, boundary bars, holes, resets, widths and positive integer extremes | Exact predicates become known when the same frozen reference exists; near predicates additionally require a valid positive range and matching tolerance. Forming/absent references remain Unknown. |
| Bar/session/time/gap-midpoint, 25 | `SessionState::step`, current bar plus declared same-session/prior-session state | Eight `session_known_readiness` public tests independently cover all25 positions, truth-only API equivalence, prior-three history, session descriptions, gap midpoint, calendar skips, refusals and Boolean/Column handoff; three private ownership/cold/overflow checks | Current descriptions use post-fold day extremes; prior-three sequences use pre-fold same-session history; prior-day/gap predicates require their actual references. A flat opening has no gap midpoint. Near additionally needs a usable range/tolerance. |
| Opening-gap Fibonacci, 11 | `GapFib::step`; previous final3 and current first3 bars; stored path replaces with exact-minute overlay | `gap_known_readiness`: independent eleven-rung true/false/NOT oracle, three-bar boundary, reset, engulfing/no-gap, nonregular anchor, extreme widths/levels and exact-minute projection | Truth and known are paired before folding. The overlay clears coarse truth and availability, then copies the exact-minute pair. Missing legs, wrong widths and overflowing levels remain Unknown. |
| Trend/EMA/SuperTrend/swing/BoS/CHoCH, 14 | `TrendState::step`, emit before current fold; confirmed swings use right-hand confirmation already observed | Seven `trend_known_readiness` public tests: independent first/continuation/reversal oracle, confirmation delay, missing opposite swing, tolerance edges, positive extremes, refusal conservation and history boundaries | EMA/stop gates remain unchanged. Each near answer needs its own confirmed representable band. All four structure events are decidable once both swing latches exist; no prior direction is needed for the defined first BoS. Existing trend history carries across days. |
| Session VWAP and bands, 20 | `Vwap::step`, causal current-session price-volume sums; absent volume never inferred from future bars | Existing accumulator/alias tests plus `all_vwap_predicates_and_negations_match_independent_integer_levels`; complete mapping in `docs/26-vwap-mapping.md` | All 13 exact comparisons known when mean, sigma and their levels exist. Seven near predicates additionally require positive sigma and the correct tolerance family. All 20 remain unknown for absent volume or fewer than two current-session contributors. |
| Candle patterns, 62 | `Patterns::step` then `bits`, fixed five-bar ring, session reset | Complete bit-by-bit inventory below; new finite shape/engulfing and translation/reset checks | Every predicate known once its required same-session lookback exists. Earlier false answers remain unknown. |
| Session-open and structure regime, 4 | Aggregate evaluator, positions276–279 | `the_first_bar_of_every_session_has_no_session_open_to_compare_against`, `the_close_against_the_session_open_sets_at_most_one_bit`, `the_structure_in_force_is_reported_between_breaks_and_not_before_the_first` | Open-side predicates require an already seeded current-session open; structure regime requires an established direction. |
| Crossings/ordinals, 85 | Aggregate evaluator's17 fixed crossing state machines; reset each session | Three public `crossing_known_readiness` tests compare all85 bits and negations with an independent last-definite-side oracle; a private matrix exhausts all17 owners and prior-side/day/reference combinations | Known false needs an earlier same-session definite side and both currently available base comparisons. A known touch/band interior is a non-event and preserves crossing memory. Missing references or first-side seeding remain Unknown; a current truth always stays known. |
| Weekdays, 5 | `weekday_bit`, timestamp only | Weekday/calendar tests in `lib.rs` and `evaluator.rs` | All five known on accepted timestamps; weekend dates make all five false. Special-session admission belongs to the calendar boundary. |

Anchored columns share availability with the regular evaluator after installing
eligible sealed daily records strictly before the signal day. The combined exact-minute
ORB/GapFib overlay carries the exact-minute truth and availability together. Projection copies truth and availability
from the same signal row, duplicate projected source rows are rejected together,
and `clear_before` clears both masks so NOT cannot activate outside a test window.
The additional availability column costs 48 bytes per retained signal row.

The public availability tests explicitly distinguish a false but available
prior-day comparison from a missing anchor. VWAP base comparisons remain unknown
after one contributing bar even though a mean exists: the emission contract
also requires dispersion, which becomes available after the second contributing
bar. Zero-volume bars do not advance that boundary, and a new session resets it.

The D-0528 clock-window tests separately certify every minute across two civil
days and all four half-open windows. An accepted timestamp outside a window
is an available false answer, so `!44` can be True after early morning. Missing
price references still remain Unknown. These tests preserve the low-level
evaluator's existing outside-session acceptance; stored calendar admission is
not widened. The generated warmed-day Column retains315 not-early-morning
signals instead of silently omitting all of them.

Independent actual-minute ORB verification was also run at clean commit
`174435aa640f7e6025ca63ddddaef06ca00d5672`, before D-0528. Every saved row for
`!86 & !87` matched:3,748 NIFTY2min and2,425 NIFTY3min May2025 rows, including57
Unknown outcomes. The reconstructed coarse calculation disagreed on192
expression outcomes; this is not a comparison against historical saved result
files. A single60min month had147 raw bars, below200-bar warm-up, and explicitly
refused. The two-month strict range separately completed with80 warmed hourly
rows at a deliberately zero-survivor support threshold. These are bounded
actual-data checks, without strategy admission or profit assurance. The exact
log is [the saved real-data oracle](../target/sweep-audit-20260906/independent-orb-oracle-final-v2.log).

`a_session_wider_than_the_type_leaves_yesterday_absent` and its outside-crate
counterpart retain legacy names. Their revised fixtures use positive one-paisa
and maximum-price bars: the price span fits, but derived pivot extensions do
not. Oversized signed ORB/GapFib spans are now explicit low-level arithmetic
fixtures; the checked production folds independently refuse negative candles.

## Every live candle-pattern bit

All rows below emit through `Patterns::bits` in `crates/indicators/src/pattern.rs`.
Every row has its own named positive exemplar and a required dark counterpart in
`pattern::exemplars::every_position_fires_on_its_own_exemplar_and_leaves_its_opposite_dark`.
That test checks canonical names and exact completeness against `positions()`;
a never-firing predicate cannot pass. The new
`every_named_exemplar_survives_price_translation_and_session_reset` applies to
each of those same62 rows. The required history counts include the current bar.

| Bit | Canonical condition | Bars | Emitter group | Evidence |
|---:|---|---:|---|---|
|153|`pat_hammer`|1|one-bar|named exemplar + finite shape oracle|
|154|`pat_inverted_hammer`|1|one-bar|named exemplar + finite shape oracle|
|155|`pat_hanging_man`|2|two-bar|named exemplar + translation/reset|
|156|`pat_shooting_star`|2|two-bar|named exemplar + translation/reset|
|157|`pat_bullish_engulfing`|2|two-bar|named exemplar + finite engulfing oracle|
|158|`pat_bearish_engulfing`|2|two-bar|named exemplar + finite engulfing oracle|
|159|`pat_bullish_harami`|2|two-bar|named exemplar + containment boundaries|
|160|`pat_bearish_harami`|2|two-bar|named exemplar + containment boundaries|
|161|`pat_piercing_line`|2|two-bar|named exemplar + translation/reset|
|162|`pat_dark_cloud_cover`|2|two-bar|named exemplar + translation/reset|
|163|`pat_morning_star`|3|three-bar|named exemplar + translation/reset|
|164|`pat_evening_star`|3|three-bar|named exemplar + translation/reset|
|165|`pat_three_white_soldiers`|3|three-bar|named exemplar + translation/reset|
|166|`pat_three_black_crows`|3|three-bar|named exemplar + translation/reset|
|167|`pat_three_inside_up`|3|three-bar|named exemplar + translation/reset|
|168|`pat_three_inside_down`|3|three-bar|named exemplar + translation/reset|
|169|`pat_tweezer_top`|2|two-bar|named exemplar + polarity boundaries|
|170|`pat_tweezer_bottom`|2|two-bar|named exemplar + polarity boundaries|
|171|`pat_spinning_top`|1|one-bar|named exemplar + finite shape oracle|
|172|`pat_marubozu_bullish`|1|one-bar|named exemplar + translation/reset|
|173|`pat_marubozu_bearish`|1|one-bar|named exemplar + translation/reset|
|174|`pat_dragonfly_doji`|1|one-bar|named exemplar + finite shape oracle|
|175|`pat_gravestone_doji`|1|one-bar|named exemplar + finite shape oracle|
|176|`pat_rising_three_methods`|5|five-bar|named exemplar + translation/reset|
|177|`pat_falling_three_methods`|5|five-bar|named exemplar + translation/reset|
|198|`pat_abandoned_baby_bull`|3|three-bar|named exemplar + translation/reset|
|199|`pat_abandoned_baby_bear`|3|three-bar|named exemplar + translation/reset|
|200|`pat_three_line_strike_bull`|4|four-bar|named exemplar + translation/reset|
|201|`pat_three_line_strike_bear`|4|four-bar|named exemplar + translation/reset|
|202|`pat_kicker_bull`|2|two-bar|named exemplar + translation/reset|
|203|`pat_kicker_bear`|2|two-bar|named exemplar + translation/reset|
|204|`pat_belt_hold_bull`|1|one-bar|named exemplar + translation/reset|
|205|`pat_belt_hold_bear`|1|one-bar|named exemplar + translation/reset|
|206|`pat_counterattack_bull`|2|two-bar|named exemplar + translation/reset|
|207|`pat_counterattack_bear`|2|two-bar|named exemplar + translation/reset|
|208|`pat_separating_lines_bull`|2|two-bar|named exemplar + translation/reset|
|209|`pat_separating_lines_bear`|2|two-bar|named exemplar + translation/reset|
|210|`pat_on_neck`|2|two-bar|named exemplar + translation/reset|
|211|`pat_in_neck`|2|two-bar|named exemplar + translation/reset|
|212|`pat_thrusting`|2|two-bar|named exemplar + midpoint boundaries|
|213|`pat_tasuki_gap_up`|3|three-bar|named exemplar + translation/reset|
|214|`pat_tasuki_gap_down`|3|three-bar|named exemplar + translation/reset|
|215|`pat_side_by_side_white`|3|three-bar|named exemplar + translation/reset|
|216|`pat_mat_hold`|5|five-bar|named exemplar + translation/reset|
|217|`pat_stick_sandwich`|3|three-bar|named exemplar + translation/reset|
|218|`pat_ladder_bottom`|5|five-bar|named exemplar + translation/reset|
|219|`pat_ladder_top`|5|five-bar|named exemplar + translation/reset|
|220|`pat_concealing_baby_swallow`|4|four-bar|named exemplar + translation/reset|
|221|`pat_unique_three_river`|3|three-bar|named exemplar + translation/reset|
|222|`pat_breakaway_bull`|5|five-bar|named exemplar + translation/reset|
|223|`pat_breakaway_bear`|5|five-bar|named exemplar + translation/reset|
|224|`pat_long_legged_doji`|1|one-bar|named exemplar + translation/reset|
|225|`pat_four_price_doji`|1|one-bar|named exemplar + finite zero-range oracle|
|226|`pat_rickshaw_man`|1|one-bar|named exemplar + off-centre doji boundary|
|227|`pat_high_wave`|1|one-bar|named exemplar + zero-range boundaries|
|228|`pat_homing_pigeon`|2|two-bar|named exemplar + translation/reset|
|229|`pat_matching_low`|2|two-bar|named exemplar + translation/reset|
|230|`pat_identical_three_crows`|3|three-bar|named exemplar + translation/reset|
|231|`pat_advance_block`|3|three-bar|named exemplar + translation/reset|
|232|`pat_deliberation`|3|three-bar|named exemplar + translation/reset|
|233|`pat_tri_star_bull`|3|three-bar|named exemplar + tri-star polarity|
|234|`pat_tri_star_bear`|3|three-bar|named exemplar + tri-star polarity|

The new independent finite matrix covers 3,311 contained single candles and
2,401 ordered two-body pairs, including dojis, zero ranges, equality at engulfing
boundaries and both polarities. These are finite integer domains, not all `i64`
prices or all five-bar histories. Translation tests preserve price differences
and deliberately include a very large positive offset. Pattern conventions are
tested against their recorded code/vocabulary definitions, not against a claim
that their names have one universally accepted market definition.

## Institutional interpretation and remaining limits

Pivots, CPR, Fibonacci, opening ranges, VWAP, confirmed swings, BoS/CHoCH and
candle shapes are observable OHLC/volume transformations. Their presence does
not show that an institution traded, identify a buyer/seller, prove liquidity
or execution capacity, or validate a strategy. The spot-index volume policy
can make VWAP unavailable. This inventory does not introduce an order-book,
market-depth, institutional-flow or point-in-time constituent feed.

`Thresholds::CLASSICAL`, session shape cuts and CPR class cuts are explicitly
recorded as **UNVERIFIED conventions** in their source documentation. The
charter does not provide a verified external authority for all these numeric
choices. They remain named reproducible inputs; this audit adds no new claimed
exchange fact or trading assurance. Statistical institutional-family checks,
validation splits, multiple testing, transaction costs and durable run identity
belong to the runner/CLI evidence and must be checked there separately.

The convenience `indicators::Column::build`, projection allocation and initial
`Arc` acceptance construction remain infallible allocation APIs. The new
fallible engine copy and exact-column clone do not make every workspace
allocation recoverable. No real out-of-memory event was forced for this audit.
The explicit setup-memory refusal test checks metadata honesty without claiming
that it exhausted RAM. Complete worst-case space/time guarantees are therefore
not established, and unbounded historical storage cannot have O(1) total space.

## Measured speed and fault-test limits

Both existing ratio benchmark gates passed on this host; the log is
`/private/tmp/brutex-sweep-indicator-engine-benches-20260906.log`. These are
measurements of one local run, not latency guarantees under arbitrary load.

| Measurement | Observed result | What it establishes |
|---|---:|---|
| Engine mask support, one required bit versus all 384 bits | 597 versus 562 ps per bar, ratio 0.941 | Fixed six-word evaluation avoids a per-required-bit scan. The full support query still scans its bars. |
| Evaluator step after 1,000 versus 200,000 prior candles | 783,104 versus 778,639 ps, ratio 0.994 | History length does not grow the fixed evaluator state. |
| Full evaluator step, every family | 789,902 ps, about 790 ns | A host measurement for this fixture; no universal latency promise. |
| Column build, 20,000 versus 200,000 bars | 762,804 versus 801,713 ps per bar, ratio 1.051 | Per-bar cost stayed within the existing gate; total build time and retained space still grow with row count. |
| Evaluator state size and positions | 1,792 bytes; 328 live emitted positions | Fixed in-memory state at this source snapshot. |

Mutation checks ran in safe scratch copies, never against the shared working
source. The indicator production diff produced 39 mutants: initially 36 were
caught, two survived, and one did not compile. The survivors exposed missing
false daily-availability and first-contributing-bar VWAP assertions. After
adding those public behavioral checks, a focused follow-up caught both survivors
and the additional selected session-boundary mutant. Across the two runs the
original diff scope therefore has 38 caught, one unviable and no remaining
survivor. This is not a whole-module or whole-crate mutation certificate.

The engine production diff produced 57 mutants: **42 caught, nine survived,
six unviable**. Six survivors adjust spare level-vector capacity without changing
reachable output; one changes the batch scheduling boundary, one changes the
reserve equality boundary, and one removes the depth label on a setup allocator
refusal. The result tests do not certify every resource/scheduling behavior of
those changes. No fabricated allocator state or real OOM was used to manufacture
a passing result. The repository's no-surviving-mutant gate is consequently
**not met** by this audit, and complete line/branch coverage is also unproven.

Raw evidence is under `target/sweep-audit-20260906/indicators-readiness-mutation`,
`target/sweep-audit-20260906/indicators-availability-followup`, and
`target/sweep-audit-20260906/engine-readiness-mutation`, each in `mutants.out`.

### Later evidence and historical checkpoints

The engine mutation counts immediately above describe an earlier checkpoint.
D-0526's later campaign reconciled 70 unique diff mutants to 63 caught and
seven unviable, with both initial timeouts caught by focused replay. See the
[dated comparison](14-sweep-readiness-20260906.md#later-d-0526-repair-evidence)
and its exact engine coverage limits. That result applies to its source;
subsequent indicator availability changes need their own verification.

The ORB follow-up added six independent generated-fixture tests and passed
398 indicator tests plus strict all-target Clippy at that checkpoint. All
13 mutations of the new `Orb::known` function were caught in a scratch copy.
The positive-price public fixtures do not reach its defensive checked-span
overflow arm; this is not full touched-crate coverage. Logs are
`orb-known-indicators-tests.log`, `orb-known-clippy.log`,
`orb-known-mutation.log` and `orb-known-function-coverage.log` under
`target/sweep-audit-20260906/`.

The later combined exact-minute bridge replaces both ORB positions 86..105
and GapFib positions 132..142 at stored signal boundaries. An independent
oracle checked 846 signal rows across all eight intraday rungs and all 20 ORB
truth/known/NOT outcomes. The 2-, 3- and 60-minute fixtures also demonstrate the
old coarse-window error explicitly. Missing windows clear stale answers;
late missing, mismatched, corrupt or off-grid minute evidence refuses without
partial replacement. These are generated adversarial fixtures, not market
performance evidence or an exchange-calendar completeness certificate.

After this bridge, 419 indicator tests and all-target Clippy passed. All
11 targeted mutations of that bridge diff were caught. Evidence:
`exact-minute-orb-indicators-tests.log`, `exact-minute-orb-clippy.log` and
`exact-minute-orb-final-mutations.log` under the same local evidence directory.
Previous-session Fib added eight public oracles and caught three new-function
mutants; current-day Fib caught four plus its same-day guard. Opening-gap
known/projection checks caught six selected mutants, with two invalid Rust
let-chain mutations disclosed. These are scoped campaigns, not full-crate
mutation closure.

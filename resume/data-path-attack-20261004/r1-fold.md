# FOLD attacker, round 1

## 1. Attacks (tests in crates/pull/tests/attack_fold.rs)
| attack | cases | failures on unmodified code | fixed? | evidence |
|---|---|---|---|---|
| fold vs naive reference (O first, H max, L min, C last, V sum, OI last non-null), random sessions with holes, pre-open/post-close minutes, 2-day inputs across month/year/leap | 100,000 sessions (>1M bars); 2,000 of them at all 9 rungs | 0 | n/a | random_sessions_fold_equals_the_naive_reference_and_the_ladder_is_associative |
| ladder associativity 1m->a->b == 1m->b | 26 aligned pairs: 1 random pair per session plus all 26 on 2,000 sessions | 0, except the 4 day pairs below | n/a | same test |
| complete_minutes vs reference completeness (holes, duplicates, off-grid +17s, pre-open, calendar/venue hours) | 20,000 sessions | 0 | n/a | random_sessions_complete_minutes_certifies_exactly_the_whole_buckets |
| width that does not divide a day | 1,440 whole-minute widths x 4 days | **FAIL**: 420s on 2025-07-01 opened 09:11 with 3 min (leading stub before the open) | yes, D-3130 | every_accepted_width_opens_every_session_at_the_open; fold.rs `Bucket::of_secs` |
| fold_from_bars repeated source bar | 1 + complete_minutes control | **FAIL**: [09:15, 09:15] -> one bar, volume 2 | yes, D-3131 | fold_from_bars_refuses_a_repeated_or_off_grid_source_bar |
| fold_from_bars off-grid source bar (09:15:30 at 1m; 09:16 at 5m) | 2 | **FAIL**: silently bucketed | yes, D-3131 | same |
| day from 2/10/30/60m source (midnight inside a source bar) | 8 widths | **FAIL**: 60m->DAY accepted (a 23:15 bar covers 00:00-00:15 of the next day) | yes, D-3132 | a_day_from_a_source_grid_that_straddles_midnight_is_refused |
| closing stubs 2/3/5/10/15/30/60m, hour stamps 09:15..15:15 | 7 rungs | 0 | n/a | the_last_bucket_of_each_rung_is_its_scheduled_stub |
| bucket holding only its last minute | 1 | 0 | n/a | a_bucket_holding_only_its_last_minute |
| OI null sentinel (never summed or maxed, survives later nulls, through the ladder) | 6 patterns x 2 rungs | 0 | n/a | open_interest_is_the_last_that_carried_one_and_the_null_is_never_arithmetic |
| volume overflow at the coarse rung only, exact i64::MAX | 4 | 0 (refused) | n/a | volume_overflow_is_refused_at_whichever_rung_it_happens |
| unsorted input | 9 rungs x 2 APIs | 0 (OutOfOrder) | n/a | unsorted_input_is_refused_not_sorted |
| month/year/leap boundary day bars | 4 pairs x 9 rungs | 0 | n/a | day_bars_across_month_year_and_leap_boundaries_land_on_their_own_dates |
| Muhurat 2025-10-21 | 9 rungs | 0 (folded, never certified, named) | n/a | the_2025_muhurat_is_folded_but_never_certified |
| overnight 23:50 + 00:05 in one 60m bucket | 1 | 0 (not certified) | n/a | an_overnight_bucket_is_never_certified |
| Day <-> epoch day round trip | all 2,932,897 days 1970..9999 | 0 | n/a | every_civil_day_round_trips_and_succ_agrees |
| IST edges 09:14:59/09:15/15:29:59/15:30, i64 extremes, calendar extremes | 12 | 0 | n/a | ist_session_edges_and_extremes |
| daily from minutes vs vendor daily | design check | not compared (see 5) | no | the_day_rung_is_not_derived_from_minutes |
| every Timeframe::KNOWN still builds a Bucket (coordinator request) | 10 | 0 | n/a | every_known_timeframe_still_builds_a_bucket |

## 2. Draft decisions and invariants
**D-3130 - A fold width must divide a day.** `Bucket::of_secs` accepted every width from 1s to a day. The intraday grid is counted from 09:15 IST on 1970-01-01, so it lands on a later day's 09:15 only when the width divides 86,400. A 420s bucket over the 2025-07-01 session opened at 09:11 IST holding three minutes. That is the leading stub stamped before the open, which fold's own comment calls a lie, and it happens on most days for every such width. `of_secs` now returns None for a width that does not divide 86,400. All 10 `Timeframe::KNOWN` widths and all 36 whole-minute divisors of a day are still accepted. anchor.rs:596 pinned `of_secs(7) == Some(7)`; the coordinator approved changing it to 50.
`| DPF-01 | Every width Bucket::of_secs accepts divides 86,400, so every intraday rung's first bar of every IST day starts at 09:15 and the day rung's at IST midnight | every_accepted_width_opens_every_session_at_the_open |`
`| DPF-02 | Every Timeframe::KNOWN width builds a Bucket | every_known_timeframe_still_builds_a_bucket |`

**D-3131 - fold_from_bars refuses input that is not bars at the source width.** `fold` merges rows that share a bucket because snapshots legitimately share a second. `fold_from_bars` passed bars straight to it, so a bar repeated at its own width was merged and its volume counted twice, and a bar off the source grid was silently put into whichever bucket its stamp fell in. It now refuses `RepeatedBar {at, ts_micros}` and `OffSourceGrid {at, ts_micros, source_secs}`, with one O(1) check per bar. `complete_minutes_with_calendar` no longer goes through `fold_from_bars`. It keeps its documented behaviour: withhold the affected bucket with a diagnostic rather than refuse the batch.
`| DPF-03 | fold_from_bars refuses a repeated or off-grid source bar by row; complete_minutes withholds only the affected bucket | fold_from_bars_refuses_a_repeated_or_off_grid_source_bar |`

**D-3132 - A day bar needs a source grid that lands on IST midnight.** The intraday grid is counted from 09:15 and the day grid from midnight, 555 minutes earlier. At 2, 10, 30 and 60 minutes, midnight falls inside a source bar. `fold_from_bars(60m, DAY)` was accepted although its 23:15 bar covers the next day's first 15 minutes, which is exactly the edge-inside-a-source-bar case `NarrowerThanSource` exists to refuse. This is now `GridMisaligned {want_secs, source_secs}`. It is unreachable with in-session NSE bars and is refused anyway, because a width check cannot know the data is in-session. A day from 1, 3, 5 or 15 minutes is still allowed.
`| DPF-04 | A day is folded only from a source width dividing 555 minutes; others are refused GridMisaligned | a_day_from_a_source_grid_that_straddles_midnight_is_refused |`
`| DPF-05 | fold equals the naive OHLCV/OI reference and 1m->a->b equals 1m->b on every aligned ladder pair (100k seeded sessions) | random_sessions_fold_equals_the_naive_reference_and_the_ladder_is_associative |`
`| DPF-06 | complete_minutes certifies exactly the buckets whose scheduled minutes are all present once, on grid, on a full verified session | random_sessions_complete_minutes_certifies_exactly_the_whole_buckets |`

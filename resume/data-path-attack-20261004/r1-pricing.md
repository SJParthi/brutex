# Attacker PRICING, round 1

## 1. Attacks
| attack | cases tried | failures (unmodified) | fixed? | evidence |
|---|---|---|---|---|
| SpotBook: exact stamp vs ±1µs/±1ms/same-second-other-µs/30s/minute−1µs, i64 extremes | 3,750 offsets + 5 extremes | 0 | n/a | dpp_spot_book_never_answers_a_sub_minute_neighbour (tests/attack_pricing.rs:112) |
| SpotBook: unsorted input (distinct stamps) | 1,000 seeded shuffles × 375 probes | 0 | n/a | dpp_spot_book_is_order_independent_for_distinct_stamps (:149) |
| SpotBook: duplicate stamps with DIFFERENT closes | 4 hand + 20,000 seeded months | 1 (last bar silently wins; existing unit test pinned it) | YES, D-3110 | dpp_spot_book_refuses_a_stamp_with_two_disagreeing_closes (:178), dpp_spot_book_duplicates_property (:231); fix pricing.rs:385 (of), :429 (lookup), :517 (SpotAmbiguous) |
| SpotBook: zero/negative/i64::MIN close, i64::MIN/MAX stamps, empty book | 8 | 0 (refused downstream as NotPositive "spot") | n/a | dpp_spot_book_extreme_closes_are_refused_downstream_by_name (:266) |
| Rate: ±0, ±1 edges, 1+ε, subnormal, MIN_POSITIVE, ±MAX, NaN, ±inf, blank citation | 17 | 0 | n/a | dpp_rate_screen_at_its_edges (:333) |
| Tenor: pre-open same day, 1µs before close, at close, 1µs after, 1 year after, i64 extremes, leap day, year end | 12 | 0 | n/a | dpp_tenor_edges (:373) |
| Tenor: Saturday expiry | 1 | accepted, +2 days of tenor | NO (needs sourced rule) — pinned, proposal D-3113 | dpp_tenor_edges |
| Tenor monotone + exact second count | 200,000 seeded pairs | 0 | n/a | dpp_tenor_is_monotone_and_exactly_seconds (:434) |
| price: i64::MAX spot/strike/premium, strike 0, i64::MIN spot, premium 1 paisa ATM, deep ITM below intrinsic, deep OTM 1 paisa, spot 1 paisa, off-ladder; × call/put × solved/vendor | 40 | 0 (no panic, no NaN Ok) | n/a | dpp_extreme_quotes_refuse_by_name_and_never_report_nan (:464) |
| vendor vol NaN/±inf/0/−0/neg/10.000001/MAX | 8 | 0 (all Model refusals) | n/a | dpp_vendor_volatility_is_screened_by_the_model (:547) |
| side flip: moneyness mirror + put-call parity, 13 rungs | 13 | 0 | n/a | dpp_side_flip_mirrors_moneyness (:578) |
| price_all reason dedupe | 10-row run | 1 (5 slots filled by one reason at 5 premiums; zero-premium + off-ladder reasons counted but never named) | YES, D-3111 | dpp_price_all_keeps_one_sentence_per_reason_class (:625); fix pricing.rs:969, :985 |
| price_all counts balance, solved+vendor=rows, below-band, rerun equality | 4,000 seeded rows | 0 | n/a | dpp_price_all_counts_balance_and_rerun_identically (:663) |
| vendor path vs premium below intrinsic | 1 | divergence (vendor path prices it, solved path refuses) | NO — pinned, decision needed | dpp_vendor_path_does_not_read_the_premium (:718) |
| empty price_all | 1 | 0 | n/a | dpp_price_all_on_nothing (:739) |
| chain::iso_expiry malformed strings | 17 bad + 2 good | 1 ("2024-+1-25" read as 2024-01-25: str::parse accepts '+') | YES, D-3112 | chain.rs:867 dpp_a_signed_or_non_digit_field_is_not_an_expiry; fix chain.rs:407 |

## 2. Draft decisions / invariants
D-3110 — A contradicted spot stamp answers nothing. SpotBook::of let a later bar at a stamp silently replace an earlier one with a different close: deterministic, but a definite level for a minute whose level is unknown, chosen by arrival order (§4 hidden fallback). Now a stamp with two different closes is removed and recorded; `at` returns None, `lookup` returns PricingError::SpotAmbiguous, `ambiguous()` counts them. Exact repeats (same close) still answer. BarFile keeps rows strictly ascending, so well-formed months are unaffected; the old unit test that pinned last-wins was updated because it encoded the defect.
| DPP-01 | SpotBook answers a stamp iff every bar at it carried one close, and then that close; len()+ambiguous() = distinct stamps; never a neighbour | dpp_spot_book_duplicates_property, dpp_spot_book_refuses_a_stamp_with_two_disagreeing_closes, dpp_spot_book_never_answers_a_sub_minute_neighbour |

D-3111 — price_all keeps one sentence per reason CLASS. The dedupe compared whole sentences, which embed the row's numbers, so five rows refused for one reason at five premiums filled all REASONS_KEPT slots and a later, different reason was counted in `refused` and never named. The key is now (PricingError arm, wrapped GreeksError arm, named field), and the first sentence of each class is kept verbatim. The probe is bounded by REASONS_KEPT, so per-row cost is unchanged. Counts (refused, offered) are unchanged.
| DPP-02 | price_all keeps at most one verbatim sentence per refusal class, so a second class is never hidden by the first one's values | dpp_price_all_keeps_one_sentence_per_reason_class |

D-3112 — iso_expiry demands digits. `str::parse::<u8>` accepts a leading '+', so "2024-+1-25" read as 2024-01-25 and its contracts were keyed to a real expiry. Every byte other than positions 4 and 7 must now be an ASCII digit, else the expiry is filed as unreadable.
| DPP-03 | chain::iso_expiry accepts exactly YYYY-MM-DD in ASCII digits naming a real Expiry | dpp_a_signed_or_non_digit_field_is_not_an_expiry |

D-3113 (PROPOSAL, not implemented) — Tenor::between accepts an expiry on a non-trading day. Venue::hours_on is an hours table, not a calendar, so a Saturday expiry gets a 15:40 close and two extra days of tenor. Refusing needs a charter-sourced rule (NSE has held Saturday special sessions) or calendar::kind_of, whose Closed is observational ("absent observations do not prove closure"). Pinned in dpp_tenor_edges.

## 3. Commands
- `CARGO_BUILD_JOBS=2 cargo test -p pull --locked --test attack_pricing` — before fixes: 15 pass / 1 fail (reason class); after: 17/17 pass.
- `cargo test -p pull --locked --lib attack_pricing` — before fix: chain test FAILED ("2024-+1-25"); after: pass.
- `cargo test -p pull --locked --lib -- pricing:: tenor:: chain::` — 35 passed, 0 failed.
- `cargo clippy -p pull --lib --test attack_pricing --locked -- -D warnings -A clippy::too_many_lines` — clean. `--lib --tests --keep-going` — lib unit-test target clean; failures only in OTHER attackers' files (http.rs:1247 and ingest.rs:1759 too_many_lines; tests/attack_ingest.rs, tests/attack_decoder.rs pedantic lints).
- `cargo check -p api -p cli --locked` — Finished (dependents compile with the new variant/API).
- `rustfmt --edition 2024 --check` on my four files — clean.

## 4. Measured (opt-level 3 test profile, this box, one Instant per op incl. ~20-40ns timer overhead)
SpotBook::at, 20,000 probes (half hit, half 1µs miss):
- n=10^3: p50 42ns p99 139ns max 33,514ns
- n=10^4: p50 47ns p99 81ns max 526ns
- n=10^5: p50 72ns p99 656ns max 2,412,325ns (one descheduling outlier)
- n=10^6: p50 202ns p99 601ns max 41,247ns
One hash probe at every size (O(1) in operations); wall clock p50 rises ~4.8x from 10^3 to 10^6 — the map outgrows cache (memory-hierarchy, not algorithmic). Not flat in nanoseconds.
price (single call, 20,000 samples): solved p50 932ns p99 1,186ns; vendor p50 303ns p99 464ns (first run: 640/1,133 and 186/281).
price_all amortised per row (vendor vol): 10^3 311ns, 10^4 447ns, 10^5 395ns, 10^6 415ns.

## 5. Needs real vendor data
- Whether any real stored index month ever has two bars at one stamp (D-3110 only matters for a malformed month).
- Whether Dhan's vendor IV ever arrives with a premium below discounted intrinsic (the vendor-path divergence) and how often.
- Whether a vendor ever publishes an expiry on a non-trading day (D-3113).

## 6. Files modified
- crates/pull/src/pricing.rs (SpotBook ambiguity, lookup, SpotAmbiguous, ReasonClass dedupe, updated unit test a_repeated_stamp_resolves_deterministically)
- crates/pull/src/chain.rs (iso_expiry digit check + cfg(test) mod attack_pricing)
- crates/pull/tests/attack_pricing.rs (new)
- tenor.rs: not modified.
CAUTION: early on I ran `cargo fmt -p pull` once, which may have reformatted other attackers' in-progress pull files (fold.rs, ingest.rs, http.rs, tests). Content is unchanged, only formatting; after that I used rustfmt on my files only.

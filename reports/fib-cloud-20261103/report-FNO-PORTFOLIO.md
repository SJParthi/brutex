# report-FNO-PORTFOLIO: the F&O path's money rules and order-rate rules

Cloud build session, Tue 3 Nov 2026. Branch `cloud/fib-fno-portfolio-20261103`,
cut from `cloud/fib-kit-20261103b` (`fa9022ba` = FNO-SEAM's passed tip
`1ad6e24ef8e0` plus this folder). Every coverage figure and diff below is
measured against **`1ad6e24ef8e0`**. The code was last changed at `985068e4`;
the commits after it add only this report and `part-FNO-PORTFOLIO.md`.

**Status: built.** The design of PARTS `### FNO-PORTFOLIO (wave 2, F1)` and
its `parts.json` entry, with READINESS ADDENDUM item 5. Not checked: the Mac's
independent check decides whether it counts (CLOUD.md item 4).

## 1. What was built

`runner::fno_portfolio`, one account's money rules and order-rate rules with
exact tie-breaks (SPEC 9.6, 9.7, 11.1, 11.3 to 11.6), as the design lists them:

| Design item | Where it is |
|---|---|
| `Account`: 20 slots in a fixed array | `Account::slots: [Slot; POSITION_SLOTS]` |
| a per-stock lock `[Option<u8>; 256]` | `Account::locks: [Option<u8>; STOCKS]` |
| the trades today (100 at most) | `trades_today`, `TRADES_PER_DAY`, refusal `TradeCap` |
| exposure and margin in use as running sums | `exposure: i128` (paisa), `margin: i128` (millionths of a paisa, exact) |
| capital Rs 20 lakh plus the carried net, worse clock (OP-21), reset-daily shown; capital per window (item 5) | `carried`, `Capital::{Carried, ResetDaily, Unchecked}`, `Account::new`, `begin_window`, `begin_day` |
| order-rate rings 10/s, 250/min, 1,000/h, 7,000/day | `Ring<10>`, `Ring<250>`, `Ring<1000>` (sliding), `orders_today` per session |
| modification counters (25 per order) | `Position::mods: [u8; 5]` per `OrderRole`, `Account::modify` |
| the tie-break order (a)-(e), with the seed for (e) | `TieBreak::{Strength, StopInR, FirstInTop, Symbol, Seeded(u64)}`, `TieKeys`, `Fraction` |
| `fn admit(&mut self, req) -> Admission { Admitted { slot } \| Refused(PositionCap \| TradeCap \| ExposureCap \| MarginCap \| WhileOpen) \| Queued(next_second) }` | `Account::admit(&mut self, at, &EntryRequest) -> Result<Admission, Invalid>` with exactly those variants; `Invalid` refuses a caller defect and changes nothing |
| `fn release(...)` | `Account::release(slot, last_fill, net)` |
| `fn schedule_second(&mut self, intents) -> Ordered`: exits before entries, then by tie-break | `Account::schedule_second(at, &mut [Intent], &mut Vec<Outcome>)`: leaves the batch in the order acted on, one `Outcome` per intent |
| C-FNO-PORT-01 | `measure_admit_release` (library, covered), `crates/runner/benches/ratio/fno_portfolio.rs` (wrapper) |

Also: `send_exit`, `mark` and `resize` (the running sums follow the last
traded price and the quantity, SPEC 11.3), `settle`, `margin(slot)` (the MTM
base FNO-SEC-EXEC reads as plain paise, PARTS FNO-SEC-EXEC), `capital_bound()`
(item 5), `counts()`, and the accessors.

## 2. Files and functions

Touched set against `1ad6e24ef8e0` (`git diff --stat 1ad6e24ef8e0 985068e4 -- crates docs`):

| File | Change |
|---|---|
| `crates/runner/src/fno_portfolio.rs` | new, 3,102 lines: 1,563 of library, then its `#[cfg(test)] mod tests` from line 1,564 |
| `crates/runner/src/lib.rs` | `pub mod fno_portfolio;` under `// slot FNO-PORTFOLIO (fno_portfolio)` |
| `crates/runner/benches/ratio/fno_portfolio.rs` | new, the bench wrapper |
| `crates/runner/benches/ratio.rs` | `#[path = "ratio/fno_portfolio.rs"] mod fno_portfolio;` under `// slot bench-mod FNO-PORTFOLIO`; `fno_portfolio::row(&["C-FNO-PORT-01"]),` under `// slot bench-row FNO-PORTFOLIO` |
| `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md` | rows directly under `<!-- slot FNO-PORTFOLIO -->` and nowhere else |

Public items of `runner::fno_portfolio`: constants `CAPITAL_PAISA`,
`EXPOSURE_CAP_PAISA`, `POSITION_SLOTS`, `TRADES_PER_DAY`, `MARGIN_FLOOR_PPM`,
`STOCKS`, `MODIFICATIONS_PER_ORDER`, `ORDERS_PER_SECOND`, `ORDERS_PER_MINUTE`,
`ORDERS_PER_HOUR`, `ORDERS_PER_DAY`; types `StockId`, `SlotId`, `Fraction`,
`TieKeys`, `TieBreak`, `Capital`, `EntryRequest`, `Admission`, `Refusal`,
`Dispatch`, `OrderRole`, `Modify`, `Invalid`, `Margin`, `Counts`,
`CapitalBound`, `ExitIntent`, `Intent`, `Outcome`, `Account`, `Samples`;
`Account::{new, begin_window, begin_day, settle, admit, send_exit, modify,
mark, resize, release, schedule_second, clock, capital, carried_net,
exposure, margin_in_use, margin, slots_taken, trades_today, orders_today,
counts, capital_bound}`, `Admission::slot`, `Refusal::name`,
`Margin::{micro_paisa, floor_paisa, ceil_paisa}`, `CapitalBound::holds`,
`Fraction::{new, num, den}`, `StockId::{new, position}`,
`SlotId::{index, ticket}`, `measure_admit_release`. No dependency added;
`Cargo.lock` unchanged.

## 3. Readings this part pins (its D block, D-8180 to D-8183; D-8184 to D-8189 unused)

* **D-8180.** Check order: `while_open`, `position_cap`, `trade_cap`,
  `exposure_cap`, `margin_cap`, then the rings; one name per refusal. The margin
  check is margin in use plus this order's at most Rs 20 lakh plus the carried
  net. A released position stays in force (slot, lock, exposure, margin, net)
  through its last fill second, SPEC 9.6's "strictly after" applied to all of
  it; the first call of a later second settles it. Each window at Rs 20 lakh;
  tickets carry on so old ids stay stale. The bench judges the ratio on the
  cycle.
* **D-8181.** Second, minute and hour rings slide (stricter than calendar
  buckets, and empty at every open either way); the 7,000 is a session's count
  (a sliding day would break item 5's equal rows); a queued order changes
  nothing and is checked whole again; `i64::MAX` once the session's orders are
  spent; 25 modifications in place per resting order, the next change a
  replacement that is one order in every ring. The Dhan figures are UNVERIFIED
  in this repository until FNO-LAW records T09-07 in the charter.
* **D-8182.** One total order per second: exits first, start second, the S81
  order, then symbol, recipe and the request itself, so any arrangement of one
  batch is acted on alike. (a) and (b) are exact fractions; (e) is `SplitMix64`
  over (seed, start second, stock, recipe); an exit is ordered by its
  position's entry keys; a procedure runs under its own one S81 value; a stock
  id is its position in `fno218.txt`, which is in byte order (checked with
  `LC_ALL=C sort -c` on this folder's copy), so id order is symbol order.
* **D-8183.** Margin `max(20%, M/100)`, exact in `i128`; `margin(slot)` the
  MTM base and `floor_paisa` its stricter rounding; `CapitalBound::holds` and
  the argument behind it.

## 4. Tests (44, all in `crates/runner/src/fno_portfolio.rs`)

The design's list, each by name:

| Design test | Test |
|---|---|
| the 20th position admitted and the 21st refused | `the_twentieth_position_is_admitted_and_the_twenty_first_refused` |
| the 100th trade admitted and the 101st refused, with cancelled entries counted | `the_hundredth_trade_is_admitted_and_the_hundred_and_first_refused_with_cancelled_entries_counted` |
| exposure at exactly Rs 1 crore admitted and +1 paisa refused | `exposure_at_exactly_one_crore_is_admitted_and_one_paisa_more_refused` (margin also exactly at Rs 20 lakh there) |
| margin exhausted by carried losses | `margin_exhausted_by_carried_losses_is_refused` (exactly at the capital admitted, one paisa of margin more refused, all capital lost) |
| one trade per stock across recipes in procedure mode | `one_trade_per_stock_across_recipes_in_procedure_mode` |
| every tie-break order | `tie_break_a_takes_the_larger_move_first`, `tie_break_b_takes_the_smaller_stop_in_r_first`, `tie_break_c_takes_the_earlier_first_entry_into_top_n_first`, `tie_break_d_is_symbol_order`, `an_earlier_start_second_goes_first_under_every_order`, `two_entries_tied_on_every_key_are_ordered_by_what_they_are` |
| the seeded spread reproducible from its seed | `the_seeded_spread_is_reproducible_from_its_seed` |
| 20 simultaneous exits over the 10-a-second limit | `twenty_simultaneous_exits_over_the_ten_a_second_limit_queue_to_the_next_second` |
| 25 modifications | `twenty_five_modifications_then_a_replacement_that_counts_in_the_rings`, `each_resting_order_counts_its_own_modifications` |
| the slot taken at send and freed at the last fill or a zero-fill cancel | `the_slot_is_taken_at_send_and_freed_at_the_last_fill_or_a_zero_fill_cancel` |
| the daily reset | `the_daily_reset` |
| Property: after any random sequence, slots <= 20, exposure <= Rs 1 crore, trades <= 100, no stock locked twice | `after_any_random_sequence_slots_exposure_trades_and_locks_hold`: 48 seeded walks (`SplitMix64`, no dependency) of 1,500 random calls over every capital reading and tie-break order. After every call: slots, trades and orders under their caps, one lock per taken slot naming its stock, the running sums equal to the slots'. Exposure under Rs 1 crore after every call in the walks without marks, and after every admission in all of them (a mark may lawfully carry it past, SPEC 11.3). Margin under the capital after every checked admission. All three rings at every send. |

READINESS ADDENDUM item 5, at the account's level (FNO-RUN owns the row-level
halves): `each_window_starts_at_twenty_lakh`,
`a_candidate_under_the_capital_bound_has_equal_rows_under_two_schemes`,
`a_candidate_over_the_capital_bound_is_walked_per_window`,
`a_walk_refused_for_capital_never_shares_its_rows`, and
`the_capital_bound_needs_both_halves`.

The rest: `exits_are_sent_before_entries_and_each_group_by_its_start_second`,
`an_exit_arriving_ahead_of_an_entry_stays_ahead`,
`two_sends_of_one_position_in_one_second_are_both_acted_on`,
`queued_entries_change_nothing_and_are_checked_again_when_offered`,
`the_minute_and_hour_rings_queue_to_the_second_their_oldest_send_expires`,
`the_sessions_orders_stop_every_order_until_the_next_session`,
`the_rings_slide_across_a_session_reset`,
`the_reset_daily_reading_starts_each_session_at_twenty_lakh`,
`the_unchecked_reading_never_refuses_for_margin`,
`the_margin_rate_has_a_twenty_percent_floor` (M below 20%, M = 25%,
M = 22.11%), `a_margin_rounds_down_for_the_mtm_rule_and_up_on_request`,
`marks_and_resizes_move_the_exposure_and_the_margin_and_never_force_a_reduction`,
`fractions_compare_by_value_and_refuse_a_zero_denominator`,
`identities_read_back`, `refusals_carry_the_names_the_spec_counts`,
`time_going_backwards_is_refused_by_name_and_changes_nothing`,
`a_bad_request_is_refused_by_name`, `a_stale_slot_is_refused_everywhere`,
`a_release_before_its_send_is_refused`, `a_bad_batch_changes_nothing`,
`the_bench_fixture_admits_every_entry_it_times`.

THE BAR's extreme cases this part owns (PARTS section 10): the 20-position and
100-trade caps hit exactly, the exposure cap at exactly Rs 1 crore and one
paisa more, and the order-rate queue at the session's end. The queue does not
end at 15:10: queued exits wait for the next allowed second, and once the
session's orders are spent no second allows one, so the position is left to
the broker's square-off (SPEC 9.8). The 15:08 and 15:10 times themselves are
FNO-SEC-EXEC's.

## 5. Gates

All run here, at the tip, with the pinned toolchain `rustc 1.97.1` unless named.

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy -p runner --all-targets --locked -- -D warnings` | clean (lib, tests, benches) |
| `cargo test -p runner --locked` | green: 789 lib tests (the base's 745 and this part's 44), the 12 integration test binaries, 6 doc-tests |
| `cargo test -p core --locked` | green, including `cost_invariants` (C-FNO-PORT-01 is a row and is printed) and `decision_citations` (D-8180 to D-8183 head entries) |
| `cargo test -p cli` on the ten tests that read `docs/05` and `docs/06` | see section 5a |
| `cargo bench -p runner --bench ratio --locked` | exit 0 in the four runs of the final fixture (section 7) |
| `cargo deny check` | NOT RUN: `cargo-deny` is not installed in this container. No dependency was added; `Cargo.lock` is byte-identical to the base |
| `cargo test --workspace --locked` | NOT RUN whole: the touched crates are `runner` and the documents, and every test that reads the touched documents was run (core, and cli's in 5a) |

CI gates 10, 11, 12 and 14, each run here exactly as `.github/workflows/ci.yml`
runs it (tools built with `rustc --edition=2024 -D warnings` from `.github/`),
on the base `1ad6e24ef8e0` and on the tip:

| Gate | Base | Tip | This part |
|---|---|---|---|
| 10, invariants name tests that exist | red: R427-11, STACK1028-03 (part 1), HA-02, AG05-13 (part 2) | red, the same four rows | none of its 12 rows refused; 5,097 rows read against 5,083 |
| 11, banned constructs on the O(1) paths | red, 29 refusals | red, 30 refusals | **one: rule 4, the sort in `Account::schedule_second` (`fno_portfolio.rs`, the `sort_unstable_by` line). Finding F1** |
| 12, every cost claim names its proof | red, the same 85 refusals | red, the same 85 | none: 238 doc blocks added, no cost claim among them |
| 14, every crate that claims a bound re-measures it | green | green | runner still pins 3 ratio points; C-FNO-PORT-01 is not on the cover table, which the main session edits (finding F2) |

### 5a. cli's document tests

`cargo test -p cli --locked` on the ten integration tests that read
`docs/05-decisions.md` or `docs/06-limits.md` by heading or section:
`c4_cli_02_limits`, `ledger_append_lookup_costs`, `ledger_scan_costs`,
`limits_doc_drift`, `limits_o1cli_2` to `limits_o1cli_6`,
`pool_and_escape_docs`. All green, 34 tests. They read each document at compile
time, so they were run again after the last document edit (`985068e4`), with
the same result. This is also why the rows under the slot lines carry no
heading of their own in `docs/04` and `docs/06` (step 6): a `###` there would
end the `###` section before it, and these tests read sections by heading. The
`### D-818x` headings of `docs/05` are what `decision_citations` requires, and
the section before them already ends at the slot section's `##`.

## 6. Coverage: measured, Route B

Toolchain `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)` (`nightly-2026-08-08`,
installed here by rustup), the build `docs/06-limits.md` names for Route B on
the Mac. `RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'`,
`CARGO_PROFILE_DEV_OPT_LEVEL=0`, `cargo test -p runner --lib --locked
fno_portfolio`, then the bench target once under the same instrumentation
(R2's CC-20 precedent for a bench wrapper), the two profiles merged with
`llvm-profdata merge -sparse`, read with `llvm-cov report` and `llvm-cov show`.
The D3 recipe file itself (`d3-recipe.txt`, `d3-route-b.sh`) is on the Mac; this
follows CLOUD.md item 4's description of it.

| File of the touched set | Lines | Branches | Functions | Regions (not the bar) |
|---|---|---|---|---|
| `crates/runner/src/fno_portfolio.rs` | **1,754 of 1,754 (100%)** | **90 of 90 (100%)** | 165 of 165 | 3,165 of 3,173 |
| `crates/runner/benches/ratio/fno_portfolio.rs` | 88 of 91 | 5 of 6 | 9 of 9 | 153 of 156 |
| `crates/runner/benches/ratio.rs`, the three added lines | the row call ran (count 1); the `#[path]` and `mod` lines hold no region | none | - | - |
| `crates/runner/src/lib.rs`, the added line | no region | none | - | - |

* The wrapper's 3 lines and 1 branch are its self-check's failing arm, the only
  thing R2 excuses, named here per file: `row`'s `other => { println!("...
  REFUSED ..."); return false; }` and the false side of its guard
  `Ok(samples) if samples.admitted == expected`. Its `if within { "ok" } else
  { "OVER BUDGET" }` ran both ways only because the instrumented bench is
  slower than the budget for some series; on another machine that arm is also
  a failing-check arm.
* The 8 unexecuted regions of `fno_portfolio.rs` are `?` early returns, on
  lines already counted as executed: `exit_key`'s (unreachable once
  `schedule_second` has checked its batch) and the bench fixture's seven
  (`measure_admit_release` and `bench_session`), which a correct fixture never
  meets.
* Coverage here is advisory; the Mac's independent check re-measures by
  Route B before anything counts. Not speculative: the base is FNO-SEAM's
  passed tip.

## 7. Benches: C-FNO-PORT-01 (advisory here)

Admit and release with the other nineteen slots held, at 10, 100 and 1,000
sessions of history (1x, 10x, 100x), 40 measured sessions of 80 cycles each
(3,200 samples of each call), five interleaved trials, the fastest p99 kept.
The cycle (one release and the admission into its slot) has its p99 ratio held
to runner's 2.5x ceiling; the cycle, the admission and the release each have
their p99 held to 300 ns (PARTS section 11's ESTIMATE).

Four runs of the final fixture (release build, 4 vCPUs): cycle p99 96 to 203 ns, admission 67 to
151 ns, release 28 to 61 ns, cycle ratios 1x to 10x and 1x to 100x between
0.92x and 1.29x; every run exit 0. The last run printed:

```
C-FNO-PORT-01 cycle p99, 1x -> 10x    148000 ->  191000 ps   ratio 1.290x  ok
C-FNO-PORT-01 cycle p99, 1x -> 100x   148000 ->  150000 ps   ratio 1.013x  ok
C-FNO-PORT-01 cycle   p99 at most 191 ns, budget 300 ns   ok
C-FNO-PORT-01 admit   p99 at most 128 ns, budget 300 ns   ok
C-FNO-PORT-01 release p99 at most 54 ns, budget 300 ns   ok
```

One fixture artifact was found and removed, and is recorded (L-FNO-PORT-02,
the doc of `measure_admit_release`). The first fixture wrote its sample buffers
only as samples arrived. Then about one release in eight took over 100 ns in
the sessions that began each 4 KiB page of the buffer, at the same sessions in
every process, and release p99 at 100x read about 130 to 180 ns against 30 to
40 ns at 1x. A plain 40 ms busy wait before the measured sessions made a
10-session history show the same pattern, so the cause was not the account's
history. Written whole just before the measured sessions, the buffers show none
of it. Reading it as a store missing a cold buffer is an inference. No run from
before that fix is counted above; in one of them `C-R-04`, a base row and not
this part's, went over its budget and the bench exited 1.

## 8. Findings for the main session

* **F1, gate 11 rule 4.** `Account::schedule_second` sorts the one second's
  batch (`sort_unstable_by`); SPEC 11.5 orders that batch and the part cannot do
  without it. Gate 11 needs one entry in `ALLOW_SORT` of
  `.github/gates_ledger.rs`: `("crates/runner/src/fno_portfolio.rs", 1)`, with
  the reason in `docs/06-limits.md` L-FNO-PORT-01 (O(n log n) in the second's own
  orders, never the history). A part may not write that file (no slot, no
  anchor; PARTS ADDENDUM 1 Nov, "Every fib part"), so it is not edited here.
  Writing the order by hand without a sort call would pass the gate's text and
  break its purpose, so that was not done either.
* **F2, gate 14's cover table.** The runner row of `ci.yml` gate 14 should gain
  `C-FNO-PORT-01` at F1's family assembly (R2 decision (b): the main session
  edits it, never a part). Its pin of ratio points stays 3: the row call has no
  `ratio(` text.
* **F3, the charter.** The order-rate figures (T09-07) are UNVERIFIED in this
  repository until FNO-LAW records the source in `docs/00-charter.md`
  (CLAUDE.md section 3 rule 1); the constants say so where declared.
* **F4, an open reading.** SPEC 9.7 does not say whether a modification in place
  counts in Dhan's per-second budget; this part counts only the replacement, as
  SPEC 9.7's text says (D-8181, L-FNO-PORT-03). The source line 234 is on the
  Mac and was not read here.
* **F5, item 5's other half.** P6's rows per (candidate, scheme, window) and the
  per-window walk of a candidate over the bound are FNO-RUN's;
  `CapitalBound::holds` is what this part gives it (L-FNO-PORT-04). FNO-RUN's
  tests of the same names will test the rows.
* **F6, callers' contracts.** A caller must number stocks by their position in
  the universe file (tie-break (d)); offer a queued order again at the second
  `Queued` names; pass the largest `M` of the readings it sized the quantity
  under; and send the broker's square-off orders outside the account.
  FNO-SEC-EXEC reads `Account::margin(slot)` as the MTM base, which is exact,
  so the rounding is its own choice: `floor_paisa` is the stricter one.

## 9. Out of scope, and what was not read

* No store, no app, no data: none is needed by this part, and none was touched.
* Mac-only files the rules cite and this folder does not hold:
  `sources-leverage-execution.md` (T09-07 at line 234, line 63 for the margin
  rule), the operator's memory files, `ENGINE.md`, the D3 recipe
  (`d3-recipe.txt`, `d3-route-b.sh`) and `state/decision-blocks.json`. This part
  was built from SPEC.md and PARTS.md as copied here, and says where it leans on
  the cited source.
* Mutation tests are deferred by the operator.
* The drain test and disk floor of RULES-MAC R2 are the Mac's; this container
  has neither path. Disk here stayed above 25 GB free.

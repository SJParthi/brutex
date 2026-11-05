STATUS: DONE

# report-FNO-SIZE: session FNO-SIZE of the 3 Nov 2026 wave (token FNO-SIZE-20261103-1)

- **Branch:** `cloud/fib-fno-size-20261103`, cut from `origin/cloud/fib-kit-size-20261103` at `933113ee` (FNO-SEAM's passed
  tip `1ad6e24ef8e0`, the first kit folder, and this part's three kit files). Last code commit `a75abf9e`; this report and
  `part-FNO-SIZE.md` follow it in one commit.
- **Base:** `1ad6e24ef8e0`. Every figure below is measured against it.
- **Kit check:** `sha256sum CLOUD-SIZE.md PARTS-FNO-SIZE.md` printed `03f12e92025787f8982356edacbff086dba705916e0b38ed062ef4367b7bc4d3`
  and `c6bd88201c004f60cb115fc0e1d0e928d9e5919e9e734f195fcd17dd2edd17f0`, the two values `MANIFEST-SIZE.md` lists.
- **Owner file:** `OWNER-FNO-SIZE.md`, the first commit, holds the token. Before every push the session fetched the branch,
  read origin's owner file (it held this token every time) and pushed only fast-forward. Nothing was force-pushed, merged or
  opened as a pull request, and no other branch was pushed.
- **Machine:** a cloud VM, Intel Xeon at 2.1 GHz, 4 vCPUs under KVM, 15 GiB, Linux 6.18. Disk was never short (28 GiB free at
  the end).

## In brief

- `runner::fno_size` is built as `PARTS-FNO-SIZE.md` sections 3-6 state it. It has the four policies through `plan`, the edge
  table, the risk budget and book, the capital actually needed, the labels and `VERSION`. It also holds the measuring functions
  of its seven bench rows.
- All 75 tests section 7 names exist and pass, and so do the 4 tests this part adds: 79 of 79. The whole `runner` suite passes
  (876 of 876), and so does the whole `core` suite (261 of 261), including `cost_invariants` and `decision_citations`.
- **Route B on the pinned nightly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`.** `crates/runner/src/fno_size/` measures 100%
  in regions, functions, lines and branches: 5,215 regions, 282 functions, 2,934 lines and 152 branches, `tests.rs` included.
  The bench wrapper was measured by CC-20's precedent. Everything in it ran except its own failing-check arm, which is named
  below.
- `cargo fmt --all --check` and `cargo clippy -p runner --all-targets --locked -- -D warnings` are clean.
- Gates 10, 11, 12, 14 and 17, invariant-ids and decision-numbers were run as `ci.yml` runs them, on this branch and on the base.
  The results are identical line for line. Every red is the base's own, and this part adds none.
- **The benches are advisory, and on this VM the runner bench exits 1 because of row C-FNO-SIZE.** Every row is inside the 2.5x
  ratio ceiling in every run. In every run, the four fixed-work rows miss the p99/p50 <= 1.2 steadiness at one or more sizes,
  and C-FNO-SIZE-05 (a) misses its 20 ns estimate (p99 28-82 ns). C-FNO-SIZE-03 missed its budget in one run and -06 in
  another. The budgets are ESTIMATES until the Mac measures them; none was changed here. See "Benches" and NEEDS THE MAC.

## What was built: files and functions

| File | What |
|---|---|
| `crates/runner/src/fno_size/mod.rs` (N) | `VERSION`, `UNIVERSE`, `SESSION_MINUTES`, `MARGIN_FLOOR_PPM`; the eight labels `FIXED_SEEDED_LABEL`, `FIXED_RANKED_LABEL`, `SPLIT_ALL_LABEL`, `RISK_LABEL`, `LIMITS_LABEL`, `CAPITAL_FIXED_LABEL`, `CAPITAL_SHARE_LABEL`, `RETURNS_LABEL`; `Policy` (`ALL`, `name`, `label`, `offer`, `twenty_slots`, `reads_capital`, `canonical_bytes`), `Offer`, `Side`, `StockId`, `StockSet`, `Fraction` (by-value `Ord` and `Eq`), `Candidate`, `AccountView`, `Limits`, `PolicyInput`, `Plan` (`new`, with `UNIVERSE` capacity), `PlanRow`, `Outcome`, `Sized`, `Bound`, `SizeRefusal` (`ALL`, `word`), `Invalid`; `plan` and its validation pass |
| `crates/runner/src/fno_size/share.rs` (N) | SPLIT-ALL's and RISK's figures (`k`, `F`, `X`, `m`, `x`, `b`), every overflow refused before a row is written, and the exact row arithmetic |
| `crates/runner/src/fno_size/order.rs` (N) | `seeded` and `mix` (tie-break (e) restated), RANKED's comparator, the fixed bitonic network `rank`, `arrival` |
| `crates/runner/src/fno_size/edge.rs` (N) | `EdgeTable` (`build`, `score`, `bucket`, `buckets`, `judging`, `whole`, `judged`), `EdgeTrade`, `Edge` (`cmp_mean`), `Level` |
| `crates/runner/src/fno_size/risk.rs` (N) | `RiskBudget` (`new`, `per_trade`, `open_cap`), `RiskBook` (`admit`, `release`, `settle`, `open_risk`, `peak`) |
| `crates/runner/src/fno_size/capital.rs` (N) | `CapitalNeeded` (`observe`, `peak`, `value`) |
| `crates/runner/src/fno_size/measure.rs` (N) | `measure_plan`, `measure_score`, `measure_network`, `measure_risk_cycle`, `measure_risk_release`, `measure_edge_insert`, `measure_edge_reset`, `measure_judging`, `measure_duplicates`; the verdict `steady` and `within`; the nine p99 budgets (ESTIMATES); `Series` |
| `crates/runner/src/fno_size/tests.rs` (N) | 79 tests (`#[cfg(test)] mod tests;` in `mod.rs`) |
| `crates/runner/benches/ratio/fno_size.rs` (N) | `pub(crate) fn row(ids: &[&str]) -> bool`, a thin wrapper that prints p50, p99, max and n at every size and judges each row |
| `crates/runner/src/lib.rs` (E) | `pub mod fno_size;` |
| `crates/runner/benches/ratio.rs` (E) | the bench module (4 lines) and the row call (9 lines, rustfmt's layout) |
| `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md` (E) | one block each |

`git diff --stat 1ad6e24ef8e0 a75abf9e -- crates docs`: 15 files, 5,369 insertions and no deletion, so no base line was edited.

### The choices the design left to the builder

Each is stated here so the Mac's check can judge it.

1. **Every write of a fixed table goes through one helper.** The workspace denies `indexing_slicing`, so array access by a
   runtime index returns an `Option`, and an impossible `None` arm would be an untested branch. Reads therefore use
   `get(..).copied().unwrap_or(..)`, the repository's own idiom. Writes go through one helper, `put`, which holds a
   `for slot in table.iter_mut().skip(at).take(1)` loop. Every table so written is a fixed array indexed by a `u8` stock
   position, a candidate index (at most 256) or a list node (at most 256), so the slot always exists. An experiment on the
   pinned nightly showed that this form leaves no line, branch or region uncovered, while `if let` and `let else` would each
   leave a branch untested.
2. **`out` stays untouched on every `Invalid`.** `plan` runs a validation pass first: structural checks, the duplicate chain,
   the eligible count, and every overflow of the batch's figures. Only then does it write. Each candidate's own overflow,
   FNO-NET's quantity times the price past `i64::MAX`, is checked in that pass. A share row cannot overflow once the exposure
   share is known to be at most `i64::MAX` paise, because its value is at most that share. So the write pass is infallible.
3. **The overflow names.** `"free margin"`, `"exposure room"`, `"exposure share"` (an exposure share past `i64::MAX` paise,
   the largest value a position can hold) and `"value"` (a FIXED value past `i64::MAX`). The edge table adds `"edge value"`
   and `"edge sum"`, the risk book `"open risk"`, and `CapitalNeeded` `"capital in use"`.
4. **Refusals beyond section 3's list.** These use the variants that already exist:
   - `RiskBook::admit` refuses a risk below 0 (`NegativeView("risk")`) and an overflowing open risk.
   - `CapitalNeeded::observe` refuses a margin below 0 (`NegativeView("margin_after")`).
   - `EdgeTable::build` refuses a trade's minute past the session (`MinuteOutOfSession`).
5. **`RiskBook::release` returns how many released rows its walk passed.** It is 0 for a release in last-fill order, which
   takes one comparison, and at most 255 otherwise. That return is what lets the property test assert "a release in last-fill
   order takes one step".
6. **The edge table's sums cannot overflow while judging.** `build` refuses values whose magnitudes sum past `i128::MAX`, so
   every subset sum that `judging` and `whole` can form fits. Neither of them can fail, except that `judging` refuses a month
   outside the learning months.
7. **A minute past the session falls in the last bucket.** It is past every rung, which is the rule "the number of rungs at or
   below the minute" applied as written. `whole()` puts the judged month back for a forward month.
8. **`0 < Rr < k` gives `b = 0`.** Each row then meets its stop checks and is refused `size_below_one_share`, as 4.4 steps 8
   and 9 read. It is not refused `size_no_risk_room`.
9. **`seed_reached` is documented exactly.** It is 0 exactly when no two candidates tie on edge, stop in R and liquidity: a
   sorting network compares every pair that ends adjacent, so whether it is 0 does not depend on the batch's arrangement. The
   count itself can depend on the arrangement, and the arrangement test asserts the zero-ness only.
10. **The measuring functions return `Series { samples, refused }`.** They count a refused call rather than propagate it, so
    the library has no unreachable error path. The wrapper sorts the samples for the percentiles, which a bench file may do.
    The library sorts nothing.

## Anchors used (each insertion directly after its base line, read at `1ad6e24ef8e0`)

| File | Base line inserted after | Inserted |
|---|---|---|
| `crates/runner/src/lib.rs` | :62 `pub mod family_allocation_v1;` | `pub mod fno_size;` |
| `crates/runner/benches/ratio.rs` | :59 `use vocab::ConditionMask;` | a blank line, `// FNO-SIZE (state/fno-size-20261103/PARTS-FNO-SIZE.md): its bench module.`, `#[path = "ratio/fno_size.rs"]`, `mod fno_size;` |
| `crates/runner/benches/ratio.rs` | :589 `        the_equity_column_build_stays_within_its_budget(),` | the row call `fno_size::row(&[` ... `]),`, nine lines with one id each, C-FNO-SIZE-01..07 in order, as `cargo fmt` keeps it |
| `docs/04-invariants.md` | :9111, the F&O section's comment line | a blank line, `<!-- slot FNO-SIZE (D-8630..D-8639; state/fno-size-20261103/PARTS-FNO-SIZE.md) -->`, the table |
| `docs/05-decisions.md` | :87601, the F&O section's comment line | a blank line, the same slot comment, D-8630..D-8638 |
| `docs/06-limits.md` | :22147, the F&O section's comment line | a blank line, the slot comment, L-FNO-SIZE-01..07 |
| `docs/07-plan.md` | :1125, the F&O section's comment line | a blank line, the slot comment, the plan table |

The resulting text is exactly the one section 2 specifies. `git diff` draws three of these hunks one line lower, after the
untouched blank line, only because each inserted block ends where a blank line already stood. The FNO-LAW slot lines and the
lines between are unchanged.

## Tests by name, with results

Run with `cargo test -p runner --lib --locked fno_size`: **79 passed, 0 failed, 0 ignored.** They also passed under Route B's
instrumented build.

**Section 7, all 75 by name, each `ok`:**

- **Batch sizes and caps:**
  - `an_empty_batch_plans_nothing_and_divides_by_nothing`
  - `one_candidate_takes_the_whole_free_margin_and_at_five_x_the_whole_exposure_room`
  - `two_hundred_eighteen_candidates_split_over_the_trade_room`
  - `a_batch_of_two_hundred_fifty_six_is_planned_and_two_hundred_fifty_seven_refused`
  - `trade_room_zero_keeps_quantities_defined`
  - `trade_room_one_with_many_candidates`
  - `two_recipes_on_one_stock_count_the_stock_once`
  - `a_duplicate_stock_and_recipe_is_refused_wherever_it_sits`
  - `two_hundred_fifty_six_candidates_on_one_stock_pass_the_duplicate_check_at_its_bound`
- **RANKED ties:**
  - `a_higher_edge_goes_first`
  - `equal_means_from_different_counts_tie`
  - `no_trade_ranks_after_every_score`
  - `an_edge_tie_goes_to_the_smaller_stop_in_r`
  - `equal_fractions_tie`
  - `a_stop_tie_goes_to_the_larger_liquidity`
  - `a_liquidity_tie_goes_to_the_seeded_key`
  - `a_seeded_tie_falls_to_the_stock_then_the_recipe`
  - `the_network_order_equals_a_reference_sort` (400 seeded batches of 0 to 256 candidates, sizes 0, 1 and 256 asserted
    present)
  - `one_seed_one_order_whatever_the_arrangement`
  - `another_seed_another_order`
  - `the_seeded_key_matches_tie_break_e`: five vectors from the restated formula. Its `mix` is also checked against
    SplitMix64's published stream from state 0: `0xE220A8397B1DCDAF`, `0x6E789E6AA1B965F4` and `0x06C45D188009454F`.
  - `a_negative_edge_is_offered_not_refused`
- **Margin, exposure and risk:**
  - `free_margin_exactly_k_shares_sizes_each_and_leaves_no_remainder`
  - `one_paisa_less_leaves_the_remainder_free`
  - `carried_losses_past_the_capital_refuse_every_row_size_no_free_margin`
  - `margin_exhausted_mid_batch_under_ranked_is_portfolios_margin_cap_in_rank_order`
  - `carried_profit_never_lifts_the_free_margin_past_the_margin_cap`
  - `carried_losses_lower_the_free_margin_below_the_margin_cap`
  - `under_the_operators_limits_the_margin_share_always_binds` (300 seeds)
  - `exposure_room_binds_under_limits_whose_exposure_cap_is_below_five_margin_caps`
  - `no_exposure_room_refuses_every_row`
  - `a_share_that_buys_no_share_is_refused_and_stays_free`
  - `split_all_never_passes_the_free_margin_or_the_exposure_room` (300 seeds)
  - `risk_never_passes_the_open_risk_cap` (300 seeds)
  - `the_risk_room_shrinks_equally_when_the_cap_binds`
  - `no_risk_room_refuses_every_row`
  - `no_learned_risk_refuses_every_row`
  - `a_budget_at_or_below_zero_is_refused`
  - `a_stop_of_zero_distance_is_refused`
  - `a_stop_through_the_price_is_refused`
  - `a_stop_less_lane_is_refused`
  - `a_tie_between_risk_and_share_is_labelled_risk`
- **Leverage, prices, overflow:**
  - `leverage_below_one_x_gives_less_value_than_margin`
  - `a_huge_margin_rate_buys_no_share`
  - `a_rate_below_the_twenty_percent_floor_is_a_caller_defect`
  - `a_one_paisa_price_and_an_i64_max_price`
  - `every_product_is_checked_and_an_overflow_is_refused_by_name`
- **Edge table:**
  - `an_empty_learning_window_scores_every_cell_none`
  - `one_trade_fills_all_three_levels`
  - `judging_a_learning_month_leaves_its_own_trades_out` (120 seeds, every month)
  - `a_forward_month_reads_the_whole_learning_table`
  - `a_trade_outside_the_learning_window_is_refused`
  - `months_out_of_order_are_refused`
  - `rungs_empty_one_and_three_hundred_seventy_five`
  - `unsorted_duplicate_or_late_rungs_are_refused`
  - `the_fallback_goes_cell_then_stock_side_then_side`
  - `a_zero_value_trade_is_refused`
  - `a_negative_net_floors_toward_minus_infinity`: -1 paisa on 3 paise gives -333,334 ppm
- **Risk book and capital:**
  - `open_risk_holds_through_the_last_fill_second_and_settles_after`
  - `a_stock_in_force_cannot_be_admitted_again`
  - `a_release_before_its_admission_and_time_backwards_are_refused`
  - `releases_in_any_order_settle_each_row_after_its_own_last_fill` (200 seeds of 400 steps; walking back and joining at the
    head both asserted to occur)
  - `the_peak_open_risk_is_kept`
  - `capital_needed_for_fixed_is_the_peak_margin_less_the_carried_net`
  - `capital_needed_for_share_policies_is_the_capital`
  - `no_admission_means_no_capital_needed`
  - `capital_needed_never_passes_the_capital_under_the_margin_check` (300 seeds)
- **Whole plan:**
  - `the_plan_does_not_depend_on_the_batch_arrangement` (120 seeds, every policy)
  - `seeded_passes_fixed_quantities_through_untouched`
  - `busy_stocks_are_while_open_under_every_policy`
  - `every_label_is_the_operators_text_byte_for_byte`
  - `canonical_bytes_differ_per_policy_and_carry_the_version`
  - `every_caller_defect_is_refused_by_name_and_changes_nothing`
  - `a_negative_view_figure_is_refused_by_name`
  - `the_measurements_run_at_tiny_sizes`

**Added by this part, 4, each `ok`:**

- `the_edge_tables_overflows_are_refused_by_name`
- `the_risk_books_figures_are_checked`
- `the_policy_table_answers_for_every_policy`
- `a_stock_set_holds_each_word`

They cover refusals and accessors section 7 names no test for. The commit message of `320ce556` says "plus five this part
adds". Four is right, and this report corrects it; history is not rewritten.

A mechanical check compared every backticked name in section 7 against the `#[test]` functions of `tests.rs`. All 75 are
present. The one listed name that is not a test, `margin_in_use`, is a field the section mentions.

## Gates

| Gate | This branch | The base `1ad6e24ef8e0` (detached worktree, the same commands) |
|---|---|---|
| `cargo fmt --all --check` | clean | - |
| `cargo clippy -p runner --all-targets --locked -- -D warnings` (pinned 1.97.1) | clean | - |
| `cargo test -p runner --locked` | 876 passed, 0 failed (824 lib tests plus the integration tests) | - |
| `cargo test -p core --locked` | 261 passed, 0 failed; `cost_invariants` 5 of 5 and `decision_citations` 4 of 4 among them | - |
| Gate 10, `invariant_paths.rs` | rc 1 on rows R427-11 (:8873) and STACK1028-03 (:9103), both the base's | the same two |
| Gate 10, crate-first half (`--middles`, `invariant-tests`), run by hand because CI stops at the line above | 4,060 named tests checked, 2 missing: HA-02 and AG05-13, both the base's | 4,044 checked, the same 2 missing |
| Gate 11, `banned-constructs` | rc 1, 29 refusals, all in `cli`, `api`, `store` and `pull` | the same 29, identical line for line |
| Gate 12, `gate12` | rc 1, 85 refusals, all the base's; this branch adds 4 claiming doc blocks, all proven (1 names `runner::fno_size::tests::the_measurements_run_at_tiny_sizes`, 3 name `crates/runner/benches/ratio.rs`) | the same 85, identical line for line |
| Gate 14, `gate14` | rc 0 | rc 0 |
| Gate 17, `gate17 --swept 'vocab engine indicators runner'` | rc 0 | rc 0 |
| `invariant-ids` | rc 0, 3,498 ids unique | rc 0 |
| Gate 27b, `decision-numbers` | rc 1, D-6247 heads two entries (:81917, :82161), the base's | the same |

Gate 11 on this part's files means: no sort, binary search, `.iter().find(`, `.contains(&`, float, unsized map, panic,
unwrap, expect, assert or local lint allow on a production line. A grep of the seven production files for every banned
pattern also prints nothing.

**Gate 14's cover row for the main session, verified.** The runner row of `ci.yml`'s cover table needs C-FNO-SIZE-01..07
appended, with its point count kept at 3, because the insertions add no `ratio(` call to `ratio.rs`:

```
runner    crates/runner/benches/ratio.rs    3  C-R-01 C-R-02 C-R-03 C-R-04 C-R-05 C-FNO-SIZE-01 C-FNO-SIZE-02 C-FNO-SIZE-03 C-FNO-SIZE-04 C-FNO-SIZE-05 C-FNO-SIZE-06 C-FNO-SIZE-07
```

Gate 14 was run with exactly that row: rc 0, and each of the seven "is a row in docs/04-invariants.md and is printed".
`ci.yml` itself was not edited.

## Coverage: Route B

**Command.** It follows `CLOUD.md` item 4:

```
RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_PROFILE_DEV_OPT_LEVEL=0 \
  cargo +nightly-2026-08-08 test -p runner --lib --locked --no-fail-fast -- fno_size
llvm-profdata merge -sparse *.profraw -o runner.profdata
llvm-cov report <the lib test binary> -instr-profile=runner.profdata --show-branch-summary crates/runner/src/fno_size/*.rs
```

- **Toolchain:** `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)` and `cargo 1.99.0-nightly (c79e8f894 2026-08-04)`, with that
  toolchain's own `llvm-profdata` and `llvm-cov`. There is one target dir, the workspace's `target/`.
- **Filter:** the filter `fno_size` runs the module's 79 tests (745 other runner tests filtered out). No other module of the
  crate calls `fno_size`, so the filter leaves out no test that reaches it. The whole runner suite under Route B was not run
  here; NEEDS THE MAC covers it.

**Result, on the final code (`a75abf9e`):**

| File | Regions | Functions | Lines | Branches |
|---|---|---|---|---|
| `capital.rs` | 38 of 38 | 6 of 6 | 23 of 23 | 4 of 4 |
| `edge.rs` | 331 of 331 | 22 of 22 | 228 of 228 | 36 of 36 |
| `measure.rs` | 555 of 555 | 32 of 32 | 305 of 305 | none |
| `mod.rs` | 384 of 384 | 37 of 37 | 259 of 259 | 28 of 28 |
| `order.rs` | 163 of 163 | 11 of 11 | 97 of 97 | 10 of 10 |
| `risk.rs` | 160 of 160 | 12 of 12 | 130 of 130 | 24 of 24 |
| `share.rs` | 229 of 229 | 13 of 13 | 151 of 151 | 24 of 24 |
| `tests.rs` | 3,355 of 3,355 | 149 of 149 | 1,741 of 1,741 | 26 of 26 |
| **total** | **5,215 (100%)** | **282 (100%)** | **2,934 (100%)** | **152 (100%)** |

The production files alone are at 100% in every column. So is `tests.rs`: its compound `assert!`s were split and outcomes
are read through helpers whose both arms run, so a passing run leaves no line or branch of the test module unexecuted.

**The bench wrapper, by CC-20's precedent.** The bench target was run once under the same instrumentation, with
`CARGO_PROFILE_BENCH_OPT_LEVEL=0`, and its profile was merged with the tests'. Its timings measure the instrumentation and
are not reported as benches.

- `crates/runner/benches/ratio/fno_size.rs` shows 179 of 181 lines (98.90%), 21 of 21 functions, 263 of 267 regions and 1 of
  2 branches.
- **The excused arm.** The only part not executed is the self-checking bench's own failing-check arm,
  `crates/runner/benches/ratio/fno_size.rs:250-252`: the refusal when the row call passes other than seven ids. This is the
  one arm the precedent excuses, named here.
- Every line of the `ratio.rs` insertions that holds a region ran: the row call, :594-602. The module lines :61-63 hold none.
  The lib.rs line holds no region.

## Benches (advisory: one cloud machine, never a best-of-N pick)

Each budget is an ESTIMATE. The runner bench's ceiling is `CEILING_PERMILLE = 2_500`, two-sided.

**Method of each row.** Each prints a line led by its id, then p50, p99, max and n at every size. One untimed pass runs before
sampling.

| Id | Method |
|---|---|
| C-FNO-SIZE-01 | One plan of 218 candidates on distinct stocks per sample, 1,000 samples per size. Sizes are taken after 10, 100 and 1,000 batches planned through the same `Plan`. Unit: ps per candidate. |
| C-FNO-SIZE-02 | 256 scores per sample, 2,000 samples per size. Tables of 1k, 10k and 100k trades on a 16-rung ladder. |
| C-FNO-SIZE-03 | 1, 10 or 100 networks of 256 rows per sample, 100 samples per size. Unit: ps per network. |
| C-FNO-SIZE-04 (a) | 16 cycles per sample (admit, a release in last-fill order, settle), 255 other stocks in force, 2,000 samples per size, after 1k, 10k and 100k cycles of history. |
| C-FNO-SIZE-04 (b) | 1, 10 or 100 worst releases per sample on one book, 200 samples per size. Each release walks past 255 released rows to the head; the settle and the admission that put the book back are inside each repetition. |
| C-FNO-SIZE-05 (a) | Every trade inserted once per sample into a table reset outside the timing, 200 samples per size, 1k, 10k and 100k trades. Unit: ps per trade. |
| C-FNO-SIZE-05 (b) | 1, 10 or 100 resets per sample of 256 x 2 x 376 cells and the minute map (375 rungs), 40 samples per size. |
| C-FNO-SIZE-06 | One month boundary per sample, 300 samples per size, months of 100, 1k and 10k trades. Unit: ps per trade moved. |
| C-FNO-SIZE-07 | 1, 10 or 100 plans per sample of 256 candidates on one stock (32,640 comparisons), 50 samples per size. Unit: ps per plan. |

**The three final runs**, made back to back at 03:49 UTC with load 0.65-0.70 and nothing else running. Each run took about
7 s (no compile). Units are ns per unit:

| Id | Measured | Size | n | run 1 p50 / p99 / max | run 2 p50 / p99 / max | run 3 p50 / p99 / max |
|---|---|---|---|---|---|---|
| C-FNO-SIZE-01 | plan, SIZE-SPLIT-ALL | 10 batches | 1000 | 9.4 / 10.3 / 77.1 | 10.5 / 18.4 / 74.1 | 16.4 / 34.9 / 160.0 |
| C-FNO-SIZE-01 | plan, SIZE-SPLIT-ALL | 100 batches | 1000 | 8.8 / 15.5 / 145.2 | 10.5 / 11.9 / 147.7 | 15.8 / 42.0 / 182.7 |
| C-FNO-SIZE-01 | plan, SIZE-SPLIT-ALL | 1,000 batches | 1000 | 8.7 / 9.6 / 97.7 | 10.5 / 20.6 / 165.1 | 15.2 / 29.9 / 188.4 |
| C-FNO-SIZE-01 | plan, SIZE-RISK | 10 batches | 1000 | 12.5 / 13.9 / 83.7 | 15.3 / 16.8 / 103.7 | 21.5 / 45.5 / 152.2 |
| C-FNO-SIZE-01 | plan, SIZE-RISK | 100 batches | 1000 | 12.5 / 18.6 / 94.5 | 15.2 / 22.3 / 92.4 | 22.2 / 50.0 / 198.3 |
| C-FNO-SIZE-01 | plan, SIZE-RISK | 1,000 batches | 1000 | 12.5 / 21.7 / 290.0 | 15.2 / 25.5 / 122.7 | 20.8 / 44.6 / 167.5 |
| C-FNO-SIZE-01 | plan, SIZE-FIXED-20-SEEDED | 10 batches | 1000 | 8.1 / 11.3 / 72.7 | 8.9 / 15.2 / 72.7 | 12.8 / 26.3 / 164.9 |
| C-FNO-SIZE-01 | plan, SIZE-FIXED-20-SEEDED | 100 batches | 1000 | 7.4 / 8.2 / 75.9 | 9.8 / 14.3 / 127.8 | 14.4 / 36.5 / 185.2 |
| C-FNO-SIZE-01 | plan, SIZE-FIXED-20-SEEDED | 1,000 batches | 1000 | 8.0 / 8.9 / 76.5 | 8.9 / 9.2 / 62.3 | 13.5 / 52.8 / 227.1 |
| C-FNO-SIZE-02 | EdgeTable::score | 1k trades | 2000 | 2.1 / 2.1 / 46.5 | 2.3 / 3.1 / 50.3 | 3.1 / 4.5 / 140.2 |
| C-FNO-SIZE-02 | EdgeTable::score | 10k trades | 2000 | 1.7 / 1.9 / 24.2 | 2.4 / 2.4 / 89.8 | 2.8 / 3.8 / 99.1 |
| C-FNO-SIZE-02 | EdgeTable::score | 100k trades | 2000 | 1.6 / 1.7 / 60.0 | 1.7 / 2.5 / 67.8 | 1.7 / 3.1 / 75.5 |
| C-FNO-SIZE-03 | RANKED network, 256 rows | 1x | 100 | 37959.0 / 54346.0 / 63973.0 | 43968.0 / 71363.0 / 85016.0 | 56602.0 / 90158.0 / 95969.0 |
| C-FNO-SIZE-03 | RANKED network, 256 rows | 10x | 100 | 33636.9 / 40228.1 / 41268.5 | 40861.9 / 49480.8 / 52862.5 | 51468.6 / 72373.1 / 74521.7 |
| C-FNO-SIZE-03 | RANKED network, 256 rows | 100x | 100 | 33252.9 / 40140.2 / 41954.6 | 40246.4 / 49730.7 / 53699.9 | 40251.6 / 68353.3 / 76590.4 |
| C-FNO-SIZE-04 | (a) RiskBook cycle | 1k cycles | 2000 | 22.3 / 32.0 / 12978.1 | 24.9 / 26.0 / 822.5 | 24.8 / 26.3 / 780.1 |
| C-FNO-SIZE-04 | (a) RiskBook cycle | 10k cycles | 2000 | 24.8 / 36.4 / 379.0 | 24.9 / 32.4 / 795.9 | 24.8 / 30.6 / 699.8 |
| C-FNO-SIZE-04 | (a) RiskBook cycle | 100k cycles | 2000 | 25.2 / 35.4 / 1559.9 | 24.9 / 31.6 / 839.5 | 24.8 / 34.6 / 713.2 |
| C-FNO-SIZE-04 | (b) RiskBook worst release | 1x | 200 | 717.0 / 1118.0 / 21642.0 | 786.0 / 950.0 / 15789.0 | 786.0 / 866.0 / 1047.0 |
| C-FNO-SIZE-04 | (b) RiskBook worst release | 10x | 200 | 688.5 / 1146.7 / 2471.1 | 758.3 / 1976.2 / 2544.4 | 758.3 / 1908.6 / 2355.3 |
| C-FNO-SIZE-04 | (b) RiskBook worst release | 100x | 200 | 688.2 / 1098.5 / 1176.0 | 755.7 / 946.4 / 1043.0 | 755.7 / 952.6 / 1012.9 |
| C-FNO-SIZE-05 | (a) EdgeTable insertion | 1k trades | 200 | 19.5 / 41.7 / 67.9 | 22.1 / 34.8 / 42.8 | 22.2 / 32.8 / 40.0 |
| C-FNO-SIZE-05 | (a) EdgeTable insertion | 10k trades | 200 | 21.1 / 82.5 / 144.0 | 23.9 / 34.7 / 38.5 | 24.1 / 28.1 / 29.6 |
| C-FNO-SIZE-05 | (a) EdgeTable insertion | 100k trades | 200 | 25.9 / 47.0 / 48.4 | 27.3 / 46.9 / 77.5 | 27.5 / 34.1 / 37.4 |
| C-FNO-SIZE-05 | (b) EdgeTable reset, 375 rungs | 1x | 40 | 331867.0 / 503485.0 / 503485.0 | 338353.0 / 415148.0 / 415148.0 | 407602.0 / 679306.0 / 679306.0 |
| C-FNO-SIZE-05 | (b) EdgeTable reset, 375 rungs | 10x | 40 | 290656.0 / 396050.3 / 396050.3 | 321333.0 / 412507.9 / 412507.9 | 286805.1 / 597804.0 / 597804.0 |
| C-FNO-SIZE-05 | (b) EdgeTable reset, 375 rungs | 100x | 40 | 261850.9 / 345090.9 / 345090.9 | 311838.1 / 370367.8 / 370367.8 | 307959.9 / 440373.7 / 440373.7 |
| C-FNO-SIZE-06 | EdgeTable::judging | 100 a month | 300 | 10.6 / 14.1 / 114.8 | 8.8 / 14.3 / 117.9 | 10.8 / 16.5 / 146.4 |
| C-FNO-SIZE-06 | EdgeTable::judging | 1k a month | 300 | 12.9 / 22.1 / 24.3 | 10.6 / 55.4 / 66.9 | 13.0 / 25.0 / 63.3 |
| C-FNO-SIZE-06 | EdgeTable::judging | 10k a month | 300 | 13.3 / 16.3 / 27.8 | 13.5 / 19.7 / 29.0 | 13.5 / 17.1 / 19.1 |
| C-FNO-SIZE-07 | plan, the duplicate check's worst batch | 1x | 50 | 73231.0 / 98718.0 / 98718.0 | 60506.0 / 102314.0 / 102314.0 | 73184.0 / 96644.0 / 96644.0 |
| C-FNO-SIZE-07 | plan, the duplicate check's worst batch | 10x | 50 | 74673.4 / 76365.8 / 76365.8 | 62801.8 / 83520.3 / 83520.3 | 74846.0 / 81994.9 / 81994.9 |
| C-FNO-SIZE-07 | plan, the duplicate check's worst batch | 100x | 50 | 74850.9 / 81978.4 / 81978.4 | 74927.4 / 85198.4 / 85198.4 | 74478.4 / 90851.1 / 90851.1 |

Every size of every run refused 0 calls.

**Verdicts across the three final runs.**

- **Ratio, p99 at 1x to 100x within 2.5x:** every row, every run. The widest is 2.009x (-01 SEEDED, run 3).
- **Budget:**
  - Held in all three runs: -01, -02, -04 (a), -04 (b), -05 (b), -07.
  - -03 held in runs 1 and 2, and was OVER in run 3 (p99 90.2 us at 1x against 80 us).
  - -05 (a) was OVER in all three runs. Its p50 is 19.5-27.5 ns and its p99 28.1-82.5 ns, against 20 ns.
  - -06 held in runs 1 and 3, and was OVER in run 2 (p99 55.4 ns at 1k a month against 50 ns).
- **p99/p50 at most 1.2 (the fixed-work rows):** -03, -04 (b), -05 (b) and -07 were OVER at one or more sizes in all three runs.
  Computed from the logs, 7 of the 36 fixed-work sizes held:
  - -03 at 10x in run 1 (1.196);
  - -04 (b) at 1x in run 3 (1.102);
  - -05 (b) at 100x in run 2 (1.188);
  - -07 at 10x in runs 1 and 3 (1.023, 1.096) and at 100x in runs 1 and 2 (1.095, 1.137).

  The worst were -04 (b) at 10x (2.606 in run 2, 2.517 in run 3) and -05 (b) at 10x in run 3 (2.084).
- So the runner bench exits 1 on this VM in every run, through the one row C-FNO-SIZE. The base rows C-R-01..05 printed `ok`
  in every run.

Reading of these figures: the per-unit rows are inside the ceiling, which is what O(1) per decision asks. The steadiness
misses are the VM's noise on single samples of 30-100 us (a 2.1 GHz shared KVM guest; p50 is steady across sizes while p99
moves). The insertion's p50 is near its estimate. This reading is not a measurement on the Mac, and no budget or bound was
changed to make this VM pass.

**The two earlier runs, kept for completeness.**

- **Run A**, the first, came before the measurement fixes. It had no warm-up; the worst release was timed on cloned books; and
  -05 used 30 and 20 samples. In it, -05 (b) "breached" its ratio at 0.211x because its first sample paid the table's one-time
  allocation of 192,512 cells. That was an artefact of the fixture, fixed by one untimed reset.
- **Run B** came after the fixes but before -05's sample counts were raised to 200 and 40.
- Their verdicts were the same as the final runs': every ratio within the ceiling except run A's artefact; -05 (a) OVER; the
  fixed-work rows' steadiness OVER.

| Id | Measured | Size | run A n: p50 / p99 / max | run B n: p50 / p99 / max |
|---|---|---|---|---|
| C-FNO-SIZE-01 | plan, SIZE-SPLIT-ALL | 10 batches | 1000: 10.6 / 17.8 / 136.8 | 1000: 8.3 / 9.5 / 26.7 |
| C-FNO-SIZE-01 | plan, SIZE-SPLIT-ALL | 100 batches | 1000: 10.6 / 17.9 / 75.2 | 1000: 8.3 / 9.0 / 83.8 |
| C-FNO-SIZE-01 | plan, SIZE-SPLIT-ALL | 1,000 batches | 1000: 10.6 / 19.7 / 275.6 | 1000: 8.3 / 9.3 / 131.9 |
| C-FNO-SIZE-01 | plan, SIZE-RISK | 10 batches | 1000: 15.4 / 32.9 / 200.6 | 1000: 12.1 / 12.8 / 64.3 |
| C-FNO-SIZE-01 | plan, SIZE-RISK | 100 batches | 1000: 15.2 / 29.3 / 138.1 | 1000: 12.1 / 20.3 / 1679.7 |
| C-FNO-SIZE-01 | plan, SIZE-RISK | 1,000 batches | 1000: 15.3 / 32.1 / 1257.9 | 1000: 12.1 / 15.6 / 355.9 |
| C-FNO-SIZE-01 | plan, SIZE-FIXED-20-SEEDED | 10 batches | 1000: 9.1 / 16.2 / 163.2 | 1000: 7.0 / 7.2 / 54.6 |
| C-FNO-SIZE-01 | plan, SIZE-FIXED-20-SEEDED | 100 batches | 1000: 9.1 / 17.5 / 168.9 | 1000: 7.0 / 7.1 / 49.0 |
| C-FNO-SIZE-01 | plan, SIZE-FIXED-20-SEEDED | 1,000 batches | 1000: 9.1 / 14.0 / 288.3 | 1000: 7.0 / 7.3 / 89.6 |
| C-FNO-SIZE-02 | EdgeTable::score | 1k trades | 2000: 2.3 / 2.8 / 268.7 | 2000: 1.8 / 1.8 / 39.8 |
| C-FNO-SIZE-02 | EdgeTable::score | 10k trades | 2000: 1.9 / 2.0 / 115.8 | 2000: 1.5 / 1.9 / 97.3 |
| C-FNO-SIZE-02 | EdgeTable::score | 100k trades | 2000: 1.7 / 2.0 / 135.2 | 2000: 1.4 / 1.5 / 82.2 |
| C-FNO-SIZE-03 | RANKED network, 256 rows | 1x | 100: 42266.0 / 70630.0 / 78108.0 | 100: 33949.0 / 47519.0 / 57395.0 |
| C-FNO-SIZE-03 | RANKED network, 256 rows | 10x | 100: 40692.1 / 57922.4 / 59509.9 | 100: 31893.5 / 37041.7 / 40669.2 |
| C-FNO-SIZE-03 | RANKED network, 256 rows | 100x | 100: 38410.6 / 49046.4 / 53231.2 | 100: 31383.8 / 48907.6 / 58721.6 |
| C-FNO-SIZE-04 | (a) RiskBook cycle | 1k cycles | 2000: 26.1 / 26.9 / 894.1 | 2000: 19.4 / 20.8 / 284.6 |
| C-FNO-SIZE-04 | (a) RiskBook cycle | 10k cycles | 2000: 25.9 / 36.7 / 916.1 | 2000: 19.5 / 31.2 / 227.4 |
| C-FNO-SIZE-04 | (a) RiskBook cycle | 100k cycles | 2000: 26.0 / 37.4 / 730.5 | 2000: 19.5 / 32.3 / 609.6 |
| C-FNO-SIZE-04 | (b) RiskBook worst release (run A: on cloned books) | 1x | 200: 783.0 / 789.0 / 1577.0 | 200: 617.0 / 637.0 / 879.0 |
| C-FNO-SIZE-04 | (b) RiskBook worst release (run A: on cloned books) | 10x | 200: 679.9 / 1231.5 / 12122.1 | 200: 595.8 / 918.7 / 1901.5 |
| C-FNO-SIZE-04 | (b) RiskBook worst release (run A: on cloned books) | 100x | 200: 959.6 / 1759.3 / 2057.0 | 200: 593.7 / 728.9 / 763.8 |
| C-FNO-SIZE-05 | (a) EdgeTable insertion | 1k trades | 30: 16.3 / 89.9 / 89.9 | 200: 17.5 / 28.4 / 31.9 |
| C-FNO-SIZE-05 | (a) EdgeTable insertion | 10k trades | 30: 18.0 / 33.8 / 33.8 | 200: 18.5 / 23.0 / 59.8 |
| C-FNO-SIZE-05 | (a) EdgeTable insertion | 100k trades | 30: 20.2 / 41.8 / 41.8 | 200: 27.2 / 34.6 / 37.1 |
| C-FNO-SIZE-05 | (b) EdgeTable reset, 375 rungs | 1x | 20: 336743.0 / 1830366.0 / 1830366.0 | 40: 293062.0 / 380542.0 / 380542.0 |
| C-FNO-SIZE-05 | (b) EdgeTable reset, 375 rungs | 10x | 20: 300946.9 / 429665.4 / 429665.4 | 40: 321334.5 / 389375.0 / 389375.0 |
| C-FNO-SIZE-05 | (b) EdgeTable reset, 375 rungs | 100x | 20: 314328.9 / 386903.8 / 386903.8 | 40: 320030.6 / 404302.4 / 404302.4 |
| C-FNO-SIZE-06 | EdgeTable::judging | 100 a month | 300: 9.0 / 11.8 / 55.7 | 300: 8.3 / 11.3 / 123.1 |
| C-FNO-SIZE-06 | EdgeTable::judging | 1k a month | 300: 10.7 / 21.2 / 24.5 | 300: 10.1 / 17.6 / 19.0 |
| C-FNO-SIZE-06 | EdgeTable::judging | 10k a month | 300: 11.0 / 14.4 / 16.8 | 300: 10.5 / 23.6 / 37.8 |
| C-FNO-SIZE-07 | plan, the duplicate check's worst batch | 1x | 50: 60499.0 / 84661.0 / 84661.0 | 50: 57504.0 / 100727.0 / 100727.0 |
| C-FNO-SIZE-07 | plan, the duplicate check's worst batch | 10x | 50: 61802.1 / 69713.9 / 69713.9 | 50: 58769.6 / 60874.5 / 60874.5 |
| C-FNO-SIZE-07 | plan, the duplicate check's worst batch | 100x | 50: 75598.2 / 84045.2 / 84045.2 | 50: 58663.6 / 65087.1 / 65087.1 |

## Ledger rows written (at the four docs anchors)

- **`docs/04-invariants.md`**, in the form `| Id | Invariant | Proof | |`, each row naming its tests by function and file, last
  cell `✓`:
  - FNO-SIZE-I01..I12, the twelve of section 9;
  - C-FNO-SIZE-01..07, each naming `runner::bench::fno_size::row` in `crates/runner/benches/ratio/fno_size.rs` and the
    library's `measure_*` function, run at tiny sizes by `the_measurements_run_at_tiny_sizes`.
- **`docs/06-limits.md`**: L-FNO-SIZE-01..07, as section 9 lists them. L-FNO-SIZE-04 names this VM.
- **`docs/07-plan.md`**: the `| FNO-SIZE (wave 3, F2) | State |` table, giving what was built and what is next.

**Decisions drafted, from this part's block only, in `docs/05-decisions.md`.** Each is
`### D-86nn — FNO-SIZE: <title> — 2026-11-03` followed by **Context**, **Decision** and **Why**:

| Id | Title |
|---|---|
| D-8630 | the sizing and admission policy is setting S82, four labelled policies each judged on its own |
| D-8631 | the decision instant is one order start second |
| D-8632 | SIZE-SPLIT-ALL splits the free margin and the exposure room equally, in whole shares |
| D-8633 | SIZE-RISK sizes equal risk under an open-risk cap, both budgets learned |
| D-8634 | SIZE-FIXED-20-RANKED orders by a learned edge through a fixed network |
| D-8635 | the liquidity key is the session-to-date traded value to the decision candle |
| D-8636 | the capital actually needed per policy is shown beside the Winner Gate's Rs 20 lakh |
| D-8637 | the labels and the refusal words, counted per policy |
| D-8638 | its own anchors stand for its slot lines |
| D-8639 | spare: no entry written, and nothing cites it |

The code under `crates/` cites D-8630..D-8636 only, and every one heads an entry (`decision_citations` passes). No other
part's D-number is cited. FNO-PORTFOLIO, FNO-NET, FNO-LEVERAGE and FNO-WFT are named in words.

## Evidence the rules cite that this folder lacks

Each is listed here instead of being guessed at. None was needed for this part's code: `CLOUD-SIZE.md` items 2 and 4 state
what the code must keep.

- **`AMENDMENTS.md`** (its R1-R8, T1-T3, P1, W1 and D1) and **`SPEC.md` section 20** (OP-47 to OP-59). These are the main
  session's addenda for FNO-RUN, FNO-WFT, FNO-PORTFOLIO, FNO-SEC-WIRE and FNO-RECORDS, absent on purpose (`CLOUD-SIZE.md`
  item 2). Where `PARTS-FNO-SIZE.md` cites them, its sections 3-9 state the rule this module keeps: the margin limit
  `min(capital, margin_cap)`, the queued-entry readings as `AccountView` fields, and the canonical bytes. What they ask of other
  parts is outside this scope.
- **The code of FNO-PORTFOLIO, FNO-NET, FNO-LEVERAGE and FNO-WFT** (not on the base). Their figures are plain integers here.
  The two formulas used are `CLOUD-SIZE.md` item 4's: the seeded key (restated, its test vectors computed from it) and the
  leverage rate `max(200,000, M)` ppm. The origin branches item 4 names were not fetched or read.
- **`ENGINE.md`, `d3-recipe.txt`, `d3-route-b.sh` and every `sources-*.md`.** Route B was run as `CLOUD.md` item 4 states it.
  The recipe's exact report flags and its step 4 (F)/(U) list form are therefore not known here. No line or branch is
  uncovered, so no (F)/(U) entry arises.
- **Mac-only state files `PARTS-FNO-SIZE.md` cites:**
  - `operator-inputs-20261103.md` (OI);
  - `workflow-fib-3.js` (F3);
  - the memory file `backtest-acceptance-bar.md` (BAR);
  - `state/decision-blocks.json`;
  - `WIRING.md`, `START.md`, `state/resume-kit/NEXT-PROMPT.md`;
  - the spec workflow's `check1.md`, `check2.md`, `repair1.md` and `repair2.md`;
  - `replay-minus-reports.sh`;
  - the PSRB `PARTS.md`;
  - the CC-20 precedent's check report and `tgt/cc20-cov-bench`. The precedent's rule, quoted in `RULES-MAC.md` R2, was
    followed.

## Out of scope (not done, by the task's scope)

- Any other part's file. None was edited: the touched set is the one listed above.
- FNO-RUN's adapters: the view, `busy`, the edge table's source walk and rungs, the risk budgets, the liquidity, the queue and
  the 15:10 cancellation.
- Real data, a store and `:8080`. None was read, written or started.
- `.github`, `ci.yml`, `Cargo.toml` and `Cargo.lock`. None was edited, and no dependency was added. Gate 14's cover row is the
  main session's (above).
- Mutation tests, deferred by the operator.
- The macOS rows of check 1.

## NEEDS THE MAC

1. Check 1 by Route B on the Mac's pin `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, with the macOS rows. That includes a
   Route B run of the whole runner suite if the recipe asks for one: here the module's 79 tests were the instrumented set.
2. **The benches on the Mac** (the cloud figures above are advisory). On this VM the runner bench exits 1 in every run because
   of row C-FNO-SIZE. The four fixed-work rows miss p99/p50 <= 1.2, and C-FNO-SIZE-05 (a) misses its 20 ns estimate. If the
   Mac's run also breaches, the budgets and the steadiness form of section 8 are the main session's to settle before F2 lands:
   gate 8 runs `cargo bench --workspace --locked` and fails on a breached row.
3. Gate 14's `ci.yml` cover rows for C-FNO-SIZE-01..07 at F2's family assembly, by the main session: exactly the runner row
   given under "Gates", verified there with rc 0.
4. FNO-PORTFOLIO's slot cap, margin limit `min(capital, Rs 20 lakh)` and lock reader (AMENDMENTS P1, L-FNO-SIZE-05). This is
   a Mac repair round of that part, needed before SIZE-SPLIT-ALL or SIZE-RISK can be walked and before the FIXED pair keeps the
   Rs 20 lakh limit.
5. FNO-RUN's integration tests that tie FNO-SIZE to the real account, FNO-NET and FNO-WFT (AMENDMENTS R7). Among them, the
   check of `order::seeded` against FNO-PORTFOLIO's real tie-break (e) function.
6. The anchor cross-check against PSRB-STRAT and PSRB-RUN once they have anchors (`PARTS-FNO-SIZE.md` section 11 item 5).

## The operator's question (`PARTS-FNO-SIZE.md` section 12; SPEC section 20 OP-58)

The 3 Nov limits say "at every instant". SPEC 11.3 checks margin in use and exposure at every entry, on positions marked at
their last traded price, and a mark never forces a reduction except under Dhan's MTM loss rule. So a price move after an entry
can carry margin in use above Rs 20,00,000, or exposure above Rs 1,00,00,000, until positions close. No entry is admitted while
either stands above its limit. This part builds SPEC 11.3 as written, as section 12 says to.

If "at every instant" means positions are cut when a mark passes a limit, that is a new SPEC rule for FNO-PORTFOLIO and FNO-RUN
(which position, at what fill, under which clock), and it is the operator's to set. It would change `LIMITS_LABEL`'s words and
nothing else of this part.

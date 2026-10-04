# Pass 12: tests, docs and security — docs/04-invariants.md against the tests (final/all-fixes-zero @ 1f4de71)

Tag: tds12. Checkout: /home/claude/wt/zero3 (detached at 1f4de71). The work was static. No cargo was run, because no finding is medium or high.

## Method and coverage

- **Every row, by name (scripted grep loops).** docs/04 has 3,152 `|` lines. They hold 3,005 backticked `a::b(::c)` references. I resolved each reference against an index of all 23,600 `fn` items in tracked `crates/**/*.rs` and `.github/*.rs`, with multi-line attributes followed. Each one is classed as test, bench, non-test or missing.
  - Result: 2,528 resolve to a `#[test]`, 90 to a bench, 226 to a non-test item and 161 to nothing. Of the 161, most are modules or constants. The real misses are P-03 and X-13 (allowlisted), and RS-03, PS-02 and AFD-15 (already P4-01).
  - I also checked 3,935 bare snake_case names in rows. The only missing ones are RS-08 (already P5-01) and three helper or field words.
  - Crate-first paths were checked against the file or `#[path]` mount. Only V-02, V-03 and V-05 name module paths that do not exist (P12-01).
  - `#[ignore]`: 12 tests. The rows that cite them (L-07 ◐, :4174, :4355) all say so.
  - `cfg(feature)`: none exist. No row cites a test under `cfg(not(unix))`.
  - Native `web/*.rs` tests run under gate 6d, except the two skipped by name (P12-07).
- **Bodies read: 42.** These were chosen for risk: rows marked `—`, golden-rule rows, absolute claims, recent zero-fix rows, and rows whose proof is prose. The tests read:
  - no_lookahead, suffix_independence, daily_mask_clears, differential_vs_naive, a_prefix_of_the_bars_gives_a_prefix_of_the_column
  - kill_between_write_and_commit, ragged_tail_truncates_loudly, unknown_version_refuses, three_geometries_are_told_apart_by_magic_stride_and_version, the_constants_are_the_current_versions_layout, commit_counter_publishes_last, oi_sentinel_distinct, a_second_writer_is_refused_while_the_month_is_held
  - an_unknown_direction_or_extremes_flag_is_refused_not_defaulted, the_ledger_ratio_is_the_cell_ratio, a_t_at_the_rounded_bar_does_not_clear_it_and_one_above_does
  - the_sweep_cannot_compute_a_condition_bit, every_level_appends_its_survivors_through_the_one_primitive, the_type_carries_no_depth_field, depth_is_reached_by_extinction_and_not_by_a_caller, best_has_no_production_caller_and_says_so
  - top_queries_and_unreadable_files_refuse_without_creating_a_store, a_ragged_tail_is_named_and_the_whole_records_are_still_served
  - a_losing_rate_whose_floor_sits_on_the_cap_fails_when_the_exact_rate_is_above, a_hold_does_not_run_across_an_intraday_halt, a_block_longer_than_the_series_is_refused_by_every_entry_point
  - a_rejected_token_whose_re_read_is_unchanged_halts_the_spot_run_with_no_further_request, a_failed_append_barrier_is_never_confirmed_by_a_later_append
  - a_top_is_admitted_up_to_the_reader_bound_and_refused_past_it, the_frontier_writer_refuses_a_top_the_api_cannot_serve, the_elite_descent_refuses_a_top_the_api_cannot_serve_before_any_read
  - the_server_answers_every_route_and_then_shuts_down_gracefully (audit part), the_tally_reconciles_only_when_every_month_is_accounted_for, a_ceiling_breach_is_reported_as_a_floor_and_not_as_a_depth, the_same_store_refusal_twice_halts_rather_than_hammering_the_disk
  - resolve's unlinked-page test (resolve.rs:1048), the_vix_month_catalogue_drops_the_oldest_month_at_its_cap
  - every_exit_grid_selector_round_trips_and_keeps_its_appended_tag, every_saved_selector_decodes_to_the_policy_that_was_saved, an_unknown_selector_word_is_refused
  - vocabulary_comes_from_linked_rust_table_and_foreign_grid_refuses, the_authorization_header_never_carries_the_secret
- **The other way.**
  - CLAUDE.md invariants were grepped against docs/04 (P12-08).
  - Decision ids cited anywhere, 1,131, were checked against the 1,119 distinct `^#+ D-NNNN` headings. Twelve cited ids have no heading:
    - D-0051 and D-0676 are explained by D-0684.
    - D-0928, D-0929 and D-0932 are explained by the gate-27b renumbering.
    - D-1310, D-1312, D-1331, D-1340, D-1361 and D-1363 are explained in docs/05 as unlanded lane duplicates.
    - D-1619 is only the end of a range label, `D-1600..D-1619`, in .github/source_scan.rs:3140. D-1615 to D-1619 do not exist. This is trivial and not filed.
  - The 37 decisions marked `**Status: locked.**` were sampled against rows. D-0100 has one, for example.
  - Duplicate headings: D-0076, D-0077, D-0078, D-0370 and D-0372. These are already known (pass 1, pass 5, and D-0684), so they are not re-filed.

## Findings

### P12-01 low: V-02 to V-05, the look-ahead and evaluator rows, name tests that do not touch the production evaluator, assert neighbouring properties, and carry the "crate does not exist" status

**Where:**
- docs/04-invariants.md:168-171
- crates/indicators/tests/invariants.rs:61-75, :91, :117, :152, :177

**What the rows and the legend say:**
- `| V-02 | At bar *i* the evaluator reads no bar `> i` | `indicators::barrier::no_lookahead` ... | — |`
- `| V-04 | Time-of-day and VWAP bits are cleared on a daily timeframe | `indicators::unit::daily_mask_clears` | — |`
- `| V-05 | The fast evaluator agrees with a naive reference on random input | `indicators::proptest::differential_vs_naive` | — |`
- The legend (docs/04:8) defines `—` as "not yet reachable (the crate does not exist)".

**Why it is wrong:**
- **V-02 and V-03.** Both tests fold `bits_over`. That is a test-local union of three components: `CurDayFib`, `Patterns`, and `SessionState::step(b, None, ..)`. The file says so at :61-66: "crates/indicators has no such function".
  - The production `Evaluator` (evaluator.rs:398-481) also folds `GapFib`, `TrendState`, `Orb`, `Vwap`, the daily levels and the crossings.
  - So a look-ahead introduced into any of those would pass both named tests.
  - What actually binds golden rule 7 on the live path is `indicators::column::tests::a_prefix_of_the_bars_gives_a_prefix_of_the_column` (column.rs:1726). It uses one cut point, and no row names it.
- **V-04.** The test drives `Vwap::for_slice(Availability::Absent)` over 120 one-minute bars. It never builds a daily bar and never reads bits 44-47. Today the daily case holds only incidentally: daily bars are stamped 00:00 IST (pull/src/fold.rs:201-316), so `orb::minutes_since_open` returns None.
- **V-05.** The test checks five fixed tuples against `DailyLevels::from_previous_session`. It covers neither "random input" nor "the fast evaluator".
- **Status and paths.**
  - The crate exists and the tests run, so `—` is false. The same applies to S-04, S-07 and S-09 (docs/04:79, :83, :85), whose tests exist and run.
  - The module paths `indicators::barrier::` and `indicators::proptest::` do not exist. Gate 10 passes because, for a crate-first path, it checks only the crate and the fn name.

**Repro (not run):** read invariants.rs:61-75, then evaluator.rs:470-481.

**Fix:**
- Make V-02 and V-03 name `column::tests::a_prefix_of_the_bars_gives_a_prefix_of_the_column`. Better, extend it to every cut, and drive `Column::build` with a mutated suffix.
- Give V-04 a test that builds 1day-rung bars and asserts bits 44-47 and 52-53 are clear.
- Reword V-05 to "the daily pivot ladder agrees with the document's recurrence on five sessions".
- Set the statuses to ✓ or ◐, and correct the module paths.

### P12-02 low: about 30 ✓ rows name no test at all, and the file's own rule ("an invariant with no test named beside it is deleted") and CLAUDE.md §9 are not enforced for them

**Where:** docs/04-invariants.md:2790-3214.
- Rows that name only code: BT-12, SW-14, SW-29, LG-05, SF-13, SF-14, MR-15, MR-22, MR-27, MR-32, MR-34, RF-02, SD-04, RJ-01, RJ-02, WF-02, WF-03, WF-04, OV-02, OV-03, OV-04, GR-01 to GR-06, HZ-01, HZ-02, AS-04, AS-07, AS-11, AS-14, AS-18.
- MR-31, RF-04, SG-02, LV-02, LV-04, HZ-03, SF-11 and AS-02 say "the same test", meaning the test of the row above.

**Examples:**
- `| SD-04 | ... | `Screened { side: direction_of(side), .. }`, four lines below the `evaluate` call | ✓ |`
- `| LG-05 | ... | `api::logs::both_halves`, and LG-04 ... | ✓ |`. `both_halves` is a production fn, not a test.
- `| AS-07 | ... | `cargo test -p runner --lib` 583 passed / 0 failed ... | ✓ |`

**Why it is wrong:**
- docs/04:3 says "Every row names the test that proves it. An invariant with no test named beside it is deleted from this file".
- Gate 10 reads only `` `a::b::c` `` tokens. A row whose proof is a code pointer or a test count is therefore invisible to it, and it still carries ✓. P12-03 is one such row that has gone false.

**Repro (not run):** a grep loop over ✓ rows that have no backticked identifier resolving to a `#[test]`/bench fn and no `.test.js`/gate mention. It lists the 43 rows above, the ~10 "same test" rows included.

**Fix:**
- Name the test for each row. Several exist unnamed: MR-32's is at resolve.rs:1048, and HZ-02's is `a_hold_does_not_run_across_an_intraday_halt`.
- Mark the rest `—` or `✗` per the legend.
- Extend gate 10 to fail a ✓ row whose proof cell resolves to no test.

### P12-03 low: HZ-01 states the opposite of what `horizon_bar` now does

**Where:**
- docs/04-invariants.md:3183
- crates/runner/src/trade.rs:1466-1490, :2785-2828

**The row:** `| HZ-01 | **A hold ends at the last bar inside the horizon's own DURATION.** ... | `horizon_bar` walks the timestamps against a deadline, not the index | ✓ |`

**The code:**
- trade.rs:1466: "Returns only a bar stamped at the EXACT deadline. Returning the last bar before a gap made the exit retroactive ... Absence is now absence".
- The test at :2816 asserts `horizon_bar(&bars, 42, 15, ..) == None` and says "09:59 is before the 10:12 deadline, not a retroactive exit at it". Under HZ-01's wording, the answer would be index 44 (09:59).

**Why it is wrong:** the row describes a removed behaviour, and it names no test (P12-02), so gate 10 cannot notice.

**Repro (not run):** read the two passages above.

**Fix:**
- Reword HZ-01: "a hold has no horizon exit unless a bar is stamped at the exact deadline; the caller keeps block-only occupancy".
- Name `runner::trade::tests::a_hold_does_not_run_across_an_intraday_halt` and the clock tests in trade_clock_tests.rs:172-190.

### P12-04 low: AS-09 says all six selector codecs' tags are pinned as literals, but the named test pins one, and the identity-bearing runner codec is pinned nowhere

**Where:**
- docs/04-invariants.md:3205
- crates/cli/src/execution_capability.rs:3583
- crates/runner/src/exit_grid_policy.rs:4413-4423 and :1157
- crates/cli/src/boolean_candidate_grid.rs:157-165, :274-280, :333-349
- crates/cli/src/execution_v3.rs:4788 and execution_v4.rs:5798

**The row:** "Six codecs carry a selector — `exit_grid_policy::selector_byte`, ... `execution_capability` (the only one with a DECODE side), `boolean_candidate_grid` ... The tags are asserted as literals rather than merely round-tripped".

**Why it is wrong:**
- The named test calls only cli's own `execution_capability::selector_byte`.
- runner's `selector_byte` feeds `ExitGridPolicyV1::digest` (:1157), and so run identity. No test asserts its literals, and no 64-hex golden digest exists in any file that names `ExitGridPolicyV1`. So renumbering it (§3 rule 8) changes every recorded policy identity and fails nothing that I could find.
- boolean_candidate_grid only round-trips tags 0-2, so a coordinated shift of its encoder and decoder passes. It pins only `3` (:345).
- execution_v3 and execution_v4 pin only `OperatorRule => 4`.
- "the only one with a DECODE side" is false: boolean_candidate_grid.rs:274-280 decodes.

**Repro (not run):** grep `selector_byte(` and `OperatorRule => [0-9]` across crates. No cargo-mutants run was made.

**Fix:**
- Add a literal-tag test in runner, `selector_byte(PessimisticTotal) == 1` … `OperatorRule == 4`.
- Assert all four literals in boolean_candidate_grid, execution_v3 and execution_v4.
- Correct the DECODE clause.

### P12-05 low: S-29 still describes three bar/sidecar geometries at bar version 2; `Layout::KNOWN` holds four, and two of them share a stride by exemption

**Where:**
- docs/04-invariants.md:104
- crates/store/src/layout.rs:192
- crates/store/tests/unit.rs:888-918
- crates/store/tests/geometry.rs:724-746

**The row:** "Three geometries are told apart by magic, stride AND version, pairwise. `.bin` is 56 bytes at version 2 ... All three are IN `KNOWN`".

**Why it is wrong:**
- `KNOWN = &[Self::V2, Self::V3, Self::OVERLAY, Self::GREEKS]`, and `CURRENT = Self::V3` (D-1571).
- The pairwise-stride property now skips the V2/V3 pair (unit.rs:905-911). That means "a shared stride makes two geometries indistinguishable" is deliberately not asserted for the bar pair.
- The second named test (geometry.rs:724) checks no version, and checks neither overlay against bar magic nor overlay against bar stride. It compares only the greeks magic and stride with the other two.

**Repro (not run):** read the cited lines.

**Fix:** reword S-29 to four rows, with bar versions 2 and 3 sharing a geometry under D-1571, and state that magic is the separator for that pair. Then assert pairwise-distinct magic over all of `KNOWN`.

### P12-06 low: BA-05's ✓ proof cites "`cli` declares no `telemetry` dependency", which has been false since D-0226

**Where:**
- docs/04-invariants.md:161
- crates/cli/Cargo.toml:98: `telemetry  = { path = "../telemetry", version = "0.1.0" }`

**Why it is wrong:** CLAUDE.md §5 records that `cli` keeps `telemetry` directly (D-0226), and it emits one event per run and per instrument-month. Half of the stated proof is false, and the other half (gate 17) covers only the four swept crates.

**Repro (not run):** `grep -n '^telemetry' crates/cli/Cargo.toml`.

**Fix:**
- Replace the second clause with gate 17's swept-crate list.
- Name the test proving the `cli` events are per instrument-month, not per bar.

### P12-07 low: a docs/04 proof is a native web test that CI skips by name, and the row does not say so

**Where:**
- docs/04-invariants.md:4939
- .github/workflows/ci.yml, gate 6d (`--skip vocabulary_comes_from_linked_rust_table_and_foreign_grid_refuses`)
- web/saved-backtest/viewer.rs:1015-1020

**The row:** `| Captured historical vocabulary and saved-grid bindings do not invent a current reader source commit | actual captured vocabulary integration in `vocabulary_comes_from_linked_rust_table_and_foreign_grid_refuses`; final `/viewer.json` evidence |`

**Why it is wrong:**
- The test returns `Err` unless `BRUTEX_VIEWER_TEST_VOCABULARY` is set, and gate 6d skips it.
- Row :4910 states this caveat for the same test ("Gate 6d skips it by name and no CI run executes it; D-1607"). Row :4939 does not, so it reads as CI-proven.

**Repro (not run):** read gate 6d's loop.

**Fix:** add the same caveat to :4939, or give it a fixture-backed test that gate 6d runs.

### P12-08 low: invariants that CLAUDE.md states have no row in docs/04

**Where:** CLAUDE.md §1, §4 and §5 against docs/04-invariants.md. Each item below has no row.

- **§1, VIX.** "`NSE-INDIAVIX` ... never enters the condition vocabulary, never enters ranking, and never enters run identity."
  - docs/04 has only LS-02, which says it is not swept (:3349), and SC-07's VWAP note.
  - `cli::global_replay::tests::vix_changes_publication_but_not_selection_pnl_or_replay_identity` (global_replay.rs:4917) exists, but it is named only inside GPR-01 for a replay-receipt claim.
- **§1, equities.** "no equity result may enter Selection V6 or execution authority until a charter-sourced equity charge stack exists."
  - It holds structurally, because `public_winner` maps every family to NIFTY or BANKNIFTY (selection_v6.rs:165-176).
  - No row states it and no test asserts it.
- **§4, writable memory mapping.** It is enforced by CI gate 24 (ci.yml:4254), but no docs/04 row names gate 24. X-03, X-04, X-08 and X-09 each give their gate a row.
- **§5, opposite banners.** "`the_generated_and_stored_banners_make_opposite_claims` fails the build if they ever converge". The test exists at cli/src/lib.rs:22795 and is named only in docs/05:22529.

**Why it is wrong:** CLAUDE.md §9 says "every new invariant appears in docs/04-invariants.md beside the test that proves it".

**Repro (not run):** `grep -c` for each phrase or test name in docs/04 returns 0, or matches only the rows noted above.

**Fix:** add four rows that name the existing tests and gate. For the Selection V6 exclusion, add a test asserting that an equity family cannot be constructed.

## Verification

None were assigned to this pass.

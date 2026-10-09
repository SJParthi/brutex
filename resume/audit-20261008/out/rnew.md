rnew (set audit-new, tree 9b0614be): 100 prior ids re-verified -- FIXED 68, DOCUMENTED 14, PARTIAL 18, NOT-FIXED 0, UNVERIFIED 0; NEW 3 (rnew-1..rnew-3, all low).

## New findings

| id | severity | crate | file:line | what is wrong in plain words | evidence | how to fix |
|---|---|---|---|---|---|---|
| rnew-1 | low | cli | crates/cli/src/execution_v3.rs:4035; population_admission_v2.rs:2662; population_finalization_v2.rs:2618; population_finalization_v3.rs:2829; population_v5.rs:3471; population_v6.rs:2023; selection.rs:3318; selection_v3.rs:1591; selection_v4.rs:2123 | The h-cli-1 rollback covers a write that FAILS, not a process that DIES mid-write (kill, OOM, power loss). Nine of the rolled-back ledgers refuse a ragged length when they open and have no repair, so one crash during an append blocks that ledger for good until someone cuts the file by hand. Their sibling ledgers already repair exactly this tail. The rollback's own cut is also not synced (append_rollback.rs:68). | Each named line is a `!bytes.is_multiple_of(stride)` refusal on the open path. `grep heal_torn_tail` finds none of the nine files, but finds execution_v4.rs:2519, population_observations_v1, results, trades, frontier and others. population_v6.rs test `every_exact_prefix_recovers_but_foreign_and_ragged_prefixes_refuse` writes one stray byte and requires `open_read` to fail. fixed_tail.rs:39-47 argues a sub-record tail is never an acknowledged record. docs/06-limits.md:11438 states this for Global Replay only. | Under the writer's exclusive lock at open, call `fixed_tail::heal_torn_tail(file, path, header, stride, magic)` in these nine, as the sibling ledgers do. Readers keep refusing. Add `sync_all` after the `set_len` in `append_rollback::append_all`. Add one test per ledger: one stray byte, writer open, tail cut and logged. |
| rnew-2 | low | docs | docs/06-limits.md:16342 | The D-2290 table of kept costs still says time lookup is a 14-probe bisection and that an index file "would be a new store format version". D-2329 built that index (`.tix`), and the D-1434 section is marked superseded (limits:12751), so the newest table contradicts the code and the decision. | limits:16342 quoted text; docs/05-decisions.md:62831 `D-2329 -- Bar lookup by time reads a per-month .tix index: one entry, not a bisection`; `cargo test -p store --test tix` 12 passed, including `the_index_answers_every_timestamp_exactly_as_the_bisection_does`. | Restate the row: an indexed month is one `.tix` read (C-TIX-01/02, O1P-02). The bisection is only the legacy path, for a month with no usable index. Add the old sentence to a `stale_claims`-style guard. |
| rnew-3 | low | ci/docs | docs/06-limits.md:16335-16338, 16367-16374 | Every "measured" figure behind the D-2290 DOCUMENTED verdicts (store lookup, fstat, month walk, manifest load) comes from a scratch release harness that is not committed. No gate re-measures them, so they can drift without anyone seeing it. D-2240's rederive cost, by contrast, is a committed bench with a ceiling. | limits:16336 "a scratch release harness that is not committed" and :16338 "not budgets a gate holds"; D-2291 re-ran the same uncommitted binaries. The D-2240 counter-example is crates/pull/benches/ratio.rs:703 with CEILING_PERMILLE 3_000. | Commit the harness as bench rows in the owning crates' `benches/ratio.rs` (store, pull, api), with invariant ids and the 3.0x ceiling, so that gate 8 holds them. |

## Verdicts on every prior id

| id | severity | crate | before | now | now, plainly | evidence |
|---|---|---|---|---|---|---|
| v1-1 | low | store | NEW | **FIXED** | Both sentences corrected and a docs test stops them returning | D-1506; cargo test -p store --test docs: 10 passed incl no_document_says_the_cold_block_verify_is_unmeasured (run here) |
| v2-1 | medium | ci | NEW | **FIXED** | Shared strict step scanner used; every disabling trick tried was refused | D-1503; local gates_bounds gate14 rc=0 'layer 5 present'; source_scan step-runs refused if: false && true, run_attempt==0, continue-on-error expr, job-level if, always() && false; shell 'true {0}' passes step-runs but Gate 0 workflow mode refuses it rc=1 |
| v2-2 | low | ci | NEW | **FIXED** | Pins equal the measured counts and a lowered pin is refused | D-1504; gate14 run: store 14 = pin 14, engine 14 = pin 14; scratch copy with engine pin 13 printed REFUSED 14-measurement-points-but-pinned-at-13 |
| v2-3 | low | runner | NEW | **FIXED** | Replaced by a distinctness check that can actually fail | D-1505; crates/runner/tests/join_answer_is_unchanged.rs:205-235 HashSet distinctness; cargo test -p runner --test join_answer_is_unchanged 2 passed |
| fold-1 | low | runner | NEW | **FIXED** | Doc now names the slice-facts build the walk really pays | D-1486; crates/runner/src/trade.rs:875 walk_with doc cites SliceFacts::of |
| h-api-1 | medium | api | NEW | **FIXED** | Leading blank lines no longer end the head; a partial head gets 408 | D-1511 closed by D-1580; server.rs HeadDeadline::observe skips CR/LF while !partial; prebuilt api lib tests leading_blank_lines_do_not_stop_the_head_clock and _deadline passed (run here) |
| h-api-2 | low | api | NEW | **FIXED** | Repeated body fields refused except the two list fields | D-1512; server.rs:16912 one_value_per_form_field with MULTI_VALUED_FORM_FIELDS member and leg; api lib a_form_body_naming_a_field_twice_is_refused and a_leg_whose_payload_repeats_a_single_value_field_refuses_the_run passed (run here) |
| h-api-3 | low | api | NEW | **FIXED** | Failure lines rationed per window with suppressed counts reported | D-1513; api/src/logs.rs note_request; api lib a_flood_of_failed_requests_writes_a_bounded_number_of_lines and a_same_origin_flood_is_bounded_and_counted_separately_from_cross_site passed (run here) |
| h-cli-1 | medium | cli | NEW | **FIXED** | A failed write truncates back; a crash mid-write is raised as rnew-1 | D-1850; crates/cli/src/append_rollback.rs:57-77 set_len(end) on error naming both errors; each of the 16 ledgers proves it with inject_short_write (read, cli not run) |
| h-cli-2 | low | cli | NEW | **FIXED** | Its banner now comes from the per-instrument provenance helper | D-1851; crates/cli/src/expression.rs:169 crate::stored_provenance_of(self.key); helper at lib.rs:3261 (read) |
| h-cli-3 | low | cli | NEW | **FIXED** | Totals use a wider integer that cannot reach the cap | D-1852; crates/cli/src/pool.rs i128 totals with NEVER_LOST = i128::MAX (pool.rs:184); stability.rs net in i128 (read) |
| h-cli-4 | medium | cli | NEW | **FIXED** | Each rolls back, syncs its rollback, or was deleted | D-1854; population_observations_v1 appends through append_rollback; sweep_evidence truncates then syncs; lineage V2/V3 deleted (D-1832); every other cli seek-to-end append site read has a rollback |
| h-eng-1 | low | indicators | NEW | **FIXED** | Docs state the exact-close rule and a test guards the wording | D-1860; cargo test -p vocab --test stale_claims 12 passed incl the_column_docs_state_the_exact_close_alignment_rule; runner align::tests 10 passed |
| h-eng-2 | low | vocab | NEW | **FIXED** | Claim corrected and multi-rung cases checked against brute force | D-1861; cargo test -p indicators --test fib_rung_rounding 3 passed; -p runner --test fib_rung_sweep 2 passed |
| h-pull-1 | medium | lake | NEW | **FIXED** | Both annotations checked; a mismatch or unknown unit is refused | D-2270; lake/src/schema.rs:172 misread_timestamp(logical, converted); refusals.rs:1613 and next test passed; probe: legacy MILLIS refused, NANOS and non-UTC refused, mismatched pair refused by the writer, stamps i64::MIN and i64::MAX pass through unchanged |
| h-pull-2 | low | pull | NEW | **FIXED** | Impossible lengths and non-zero leftover bits are refused | D-2271; totp.rs:307 TrailingBits; probe 68,000 strings of length 0..16 incl non-alphabet bytes vs a strict RFC 4648 reference: 0 mismatches; lengths 1,3,6,9,11,14 never accepted; 128 chars Ok, 129 TooLong |
| v3a-1 | low | api | NEW | **FIXED** | Both paths consult the dead list before sending | D-1482; credential_law.rs:306, 379, 408 is_dead; api lib a_read_that_returns_a_value_already_rejected_is_never_sent and the contract and rolling rejection walks passed, 15 credential_law tests (run here) |
| v3b-1 | medium | pull | NEW | **FIXED** | Anything but a plain file at the lock path is refused | D-1480; ingest::tests::a_socket_at_the_lock_path_is_refused_rather_than_run_unlocked passed (run here) |
| v3b-2 | low | api | NEW | **FIXED** | A failed stamp releases the lock and refuses loudly | D-1481; server.rs:19197 if let Err(refusal) = stamp_serve_lock(...) then release; api lib a_serve_lock_stamp_that_fails_is_cleared_or_refused_never_left_stale passed (run here) |
| v3b-3 | low | api | NEW | **FIXED** | Stray marker removed | D-1486; grep 'for why///' in crates/api/src/sweeprun.rs finds nothing |
| v4-1 | low | pull | NEW | **FIXED** | Docs now say expected or amortised, with the worst case named | D-1488; crates/pull/src/manifest.rs:288 records the old claim, :2868 'Amortised O(1) ... with an O(n_keys) worst case' |
| v4-2 | low | cli | NEW | **FIXED** | Columns are separated so extreme figures stay apart | D-1487; crates/cli/src/pool.rs pass-2 table (read) |
| v4-3 | low | cli | NEW | **FIXED** | Separator added between the progress columns | D-1487; crates/cli/src/lib.rs:17034 descent_line (read) |
| v4-4 | low | engine | NEW | **FIXED** | Repeats are counted and dropped; evicted entries are forgotten | D-1497; crates/engine/src/keep.rs Best::offer masks.insert, repeated counter, masks.remove on eviction and on rejection; no production caller (production_callers 4 passed) |
| v53-1 | low | pull | NEW | **FIXED** | A wrong separator is refused | D-1483; csv::tests::a_date_whose_separator_is_not_the_declared_byte_is_refused passed (run here) |
| v53-2 | low | cli | NEW | **FIXED** | Output is written with errors handled, no panic | D-1484; crates/cli/src/main.rs:37-49 and :79 cli::deliver writes to stdout and stderr handles (read) |
| audit-root | low | store | PARTIAL | **FIXED** | All catalog tests pass as root | D-1485; as root: cargo test -p store --lib catalog 12 passed, --test catalog 28 passed |
| gate8 | medium | ci | PARTIAL | **FIXED** | CI plants a linear scan and requires the gate to fail | D-1509, D-2314; gates_jobs tests 3 passed; scratch plant at vendor.rs:1431 refused; CI run 37502065146 job 112440187252 'Per-operation cost' success on fbdabaec |
| lookahead | high | runner | PARTIAL | **FIXED** | Exits before a hole are priced; a hole blocks only later exits | D-1514; cargo test -p runner --test hole_after_exit 3 passed; runner lib a_walk_built_stop_before_a_hole_is_priced_by_the_cell_and_the_replay_alike, hole_offset_is_the_earlier_hole_and_only_on_a_path_priced_before_it, unmeasured_and_refused_at_answer_each_cause_on_its_own passed |
| probeapi-1 | medium | api | FIXED | **FIXED** | Body has the same 10 second deadline, answered 408 | D-1510; server.rs:17823 body_deadline; api lib a_dripped_body_is_refused_with_408_at_the_deadline and a_crowd_of_body_drippers_cannot_starve_a_real_request passed (run here) |
| docs-web-01 | low | docs | PARTIAL | **FIXED** | Fourth named and each tied to the check that proves it | D-1516 table names ET-bars-candles-store-6, -11, -13 and ET-vocabulary-conditions-bits-4; store --test docs 10 passed; cargo test -p vocab --lib table:: 18 passed |
| GAP17-33 | medium | process | NOT-FIXED | **FIXED** | Landing checked by ancestry; the one dropped piece landed and tested | D-1515; git: 2c209309 and c0fc71cd are ancestors of HEAD, tree 967a04a2 equals origin/fix/c2-final's tree; store/src/header.rs:684 get_or_insert; cargo test -p store --test fault when_both_slots_decode_and_fail_the_newest_slots_refusal_is_reported passed |
| GAP15-19 | low | cli | NOT-FIXED | **FIXED** | V4 enforces the frozen ceilings the way V1 does | D-1643; StreamQuality and its tests in cli (read) |
| GAP15-17 | low | cli | NOT-FIXED | **FIXED** | Rates round up; the losing rate is gated exactly from counts | D-1644; population_base_evidence_v2.rs MAGIC BTX-BASE-EV-V3, max_rate_ppm (:1627) edge tests at u64::MAX and zero total; other producers use measured_max_rate (read) |
| AC-whp-tb-2 | medium | cli | PARTIAL | **FIXED** | Lens is a command-line word; unknown words are refused | D-1645; crates/cli/src/lib.rs:1416 parse_lens (read) |
| ET-strategies-trades-ranking-costs-7 | low | cli | PARTIAL | **FIXED** | Note prints the fold count the walk-forward uses | D-1646 (read) |
| ET-bars-candles-store-1/-8 | low | pull | DOCUMENTED | **DOCUMENTED** | Argued inherent and held by a committed bench with a ceiling | D-2240; crates/pull/benches/ratio.rs:703 a_month_filled_session_by_session_rederives_linearly, ceiling 3.0x (ratio.rs:71 CEILING_PERMILLE 3_000), run by gate 8 |
| rederive | low | pull | DOCUMENTED | **DOCUMENTED** | Argued inherent and held by a committed bench with a ceiling | D-2240; same bench crates/pull/benches/ratio.rs:703; D-3001 adds a day check, not a cost |
| W1-pull2-3 | low | pull | DOCUMENTED | **DOCUMENTED** | The rerun is the retry; the month walk is measured | D-2290 table docs/06-limits.md:16345; month walk 1.03 ms p50 / 5.12 ms p99, rerun 0.73 / 4.81 ms (D-2291); scratch harness, not committed |
| W1-pull2-0 | low | pull | DOCUMENTED | **DOCUMENTED** | The re-check is the bit-rot guarantee; load time measured | docs/06-limits.md:16347 Manifest::load 15,857 entries 5.64 ms / 15.4 ms, 93,776 entries 70.7 ms / 95.6 ms; scratch harness |
| W1-pull2-6 | low | pull | DOCUMENTED | **DOCUMENTED** | The re-check is the bit-rot guarantee; load time measured | docs/06-limits.md:16347 same Manifest::load row; scratch harness |
| W1-api5-1 | low | api | DOCUMENTED | **DOCUMENTED** | The re-check is the bit-rot guarantee; load time measured | docs/06-limits.md:16347 same row; D-3308 corrected the comment; scratch harness |
| W1-api5-2 | low | api | DOCUMENTED | **DOCUMENTED** | A miss means a manifest changed; per-manifest load measured | docs/06-limits.md:16348 'per manifest as the row above' |
| W1-pull2-5 | low | pull | DOCUMENTED | **DOCUMENTED** | Days come from the records; whole month walk measured | docs/06-limits.md:16344 whole verified month 1.03 ms / 5.12 ms / 5.13 ms |
| W1-pull1-0 | low | pull | DOCUMENTED | **PARTIAL** | Argued inherent but never timed | docs/06-limits.md:16349 'not timed here' |
| W3-store1-0 | low | store | DOCUMENTED | **FIXED** | A per-month time index answers in one read | D-2329 .tix; docs/04-invariants.md C-TIX-01, C-TIX-02, O1P-02; cargo test -p store --test tix 12 passed incl the_index_answers_every_timestamp_exactly_as_the_bisection_does; stale D-2290 row is rnew-2 |
| W3-store1-1 | low | store | DOCUMENTED | **FIXED** | Lookup goes through the time index; reading the batch is inherent | D-2329; store/src/file.rs first_at_or_after dispatches on tix_state, bisection only for unindexed months with a warning (limits:12751) |
| ET-bars-candles-store-3 | low | store | DOCUMENTED | **PARTIAL** | Grid and month enforced; session hours checked only upstream | docs/06-limits.md:16376 'store may not depend on pull's calendar'; fix: move the session table into core so store can check it |
| R9-csr-o1-0 | low | store | DOCUMENTED | **DOCUMENTED** | The check finds records past the commit; measured in nanoseconds | docs/06-limits.md:16343 fstat 386 ns / 488 ns; rerun 245 / 336 ns (D-2291) |
| W3-engine1-1 | low | engine | DOCUMENTED | **PARTIAL** | Argued needed for a stable order, but never timed | docs/06-limits.md:16356 'not timed here' |
| ET-o1-proof-coverage-2 | low | engine | DOCUMENTED | **DOCUMENTED** | Apriori must test every subset; an engine bench times it | docs/06-limits.md:16355 C-E-12 engine bench |
| W3-engine1-0 | low | engine | DOCUMENTED | **DOCUMENTED** | Apriori must test every subset; an engine bench times it | docs/06-limits.md:16355 C-E-12 engine bench |
| o1engine-20 | low | engine | DOCUMENTED | **DOCUMENTED** | No production caller, and a test keeps it that way | docs/06-limits.md:16357 'none: no caller'; cargo test -p engine --test production_callers best_has_no_production_caller_and_says_so passed |
| W3-engine1-2 | low | engine | DOCUMENTED | **DOCUMENTED** | No production caller, and a test keeps it that way | docs/06-limits.md:16357; same production_callers test passed |
| GAP16-26 | low | runner | DOCUMENTED | **PARTIAL** | Sums are exact, but four money totals are still stored as floats | crates/runner/src/outcome.rs:1873 wide() converts i128 sums to f64 once; Edge money fields stay f64 (D-1173), exact only below 2^53 paisa |
| ET-strategies-trades-ranking-costs-9 | low | runner | DOCUMENTED | **FIXED** | Paragraph corrected | D-1448; docs/06-limits.md:4762 (read) |
| W3-runner2-2 | low | cli | DOCUMENTED | **FIXED** | Each shared slice is attested once | D-1838 (read) |
| W3-runner4-1 | low | cli | DOCUMENTED | **FIXED** | Attested once per side | D-1837; TrainingAttestationsV1 and its tests (read) |
| W2-cli8-6 | low | cli | PARTIAL | **FIXED** | Reads are shared; a build pass reads nothing | D-1843; SpanShare, PreparedColumn, SPAN_SHARE_KEYS; exact_minute_withholding_unsourceable_days removed (read) |
| o1cli-2 | low | cli | DOCUMENTED | **FIXED** | A rung prepares once | D-1840, D-1843; measured before and after in D-1849 (read) |
| o1cli-3 | low | cli | DOCUMENTED | **FIXED** | Rungs of one command share their reads | D-1840, D-1843, D-1849 (read) |
| o1cli-4 | low | cli | DOCUMENTED | **FIXED** | Contexts handed back from the build | D-1840, D-1843, D-1849 (read) |
| o1cli-5 | low | cli | DOCUMENTED | **FIXED** | Loaded span passed on to the work | D-1840, D-1843, D-1849 (read) |
| o1cli-6 | low | cli | DOCUMENTED | **FIXED** | Sorts only the rows it measures and prints | D-1842; crates/cli/src/lib.rs:14196 screen_order_key, :14221 least_first; D-1849 (read) |
| AC-whp-o1-1 | low | cli | DOCUMENTED | **PARTIAL** | Fresh runs stream; a resume still decodes the full history | D-1844 walk_checkpointed_streamed, rank_checkpointed_streamed; engine --test resume_readiness a_streamed_checkpointed_walk_hands_on_what_the_retaining_walk_retains passed; resume path still restores the whole history per D-1844 text |
| cli-14 | low | cli | DOCUMENTED | **PARTIAL** | Rescans removed; per-generation whole-file hashes remain, untimed | D-1845 (read); no timing in docs/06-limits.md |
| W2-cli12-3/-4 | low | cli | DOCUMENTED | **PARTIAL** | Row reads test-only; whole-file hashes remain, untimed | D-1845 (read) |
| W2-cli1-2/-3 | low | cli | NOT-FIXED | **FIXED** | Modules deleted | D-1832; crates/cli/src/anchored_search_lineage_v2.rs and _v3.rs absent |
| W2-cli6-0 | medium | cli | PARTIAL | **PARTIAL** | Argued inherent but not timed | D-1846; docs/06-limits.md section says not timed |
| W2-cli16-2 | low | cli | NOT-FIXED | **FIXED** | One writer held while the file is unchanged | D-1841, D-1777 with_cached_handle (read) |
| W2-cli16-3 | low | cli | NOT-FIXED | **FIXED** | Priced over the span sizing already loaded | D-1836, D-1683 SizedNifty and 2 tests (read) |
| W2-cli15-2 | low | cli | NOT-FIXED | **FIXED** | Computed once per population | D-1835 BoundPopulationCompletenessV1 (read) |
| W2-cli7-0 | low | cli | NOT-FIXED | **FIXED** | Derived once per population | D-1835; D-1847 corrects D-1638's heading (read) |
| W2-cli10-2 | low | cli | NOT-FIXED | **FIXED** | The only entry takes the grid's one validation | D-1834; derive_strategy_digest_v1 test-only; test no_strategy_digest_entry_validates_the_whole_grid_per_cell (read) |
| W2-cli2-5 | low | cli | NOT-FIXED | **PARTIAL** | Argued inherent but not timed | D-1847 (read); no timing |
| W2-cli14-1/2/3 | low | cli | NOT-FIXED | **PARTIAL** | One authenticated read; remaining replays argued, not timed | D-1848 (read); no timing |
| W1-api2-1 | low | api | DOCUMENTED | **DOCUMENTED** | Reads what it derives from; the month walk is measured | docs/06-limits.md:16346 same month walk 1.03 ms / 5.12 ms |
| W1-api2-2 | low | api | DOCUMENTED | **FIXED** | Summary held; closing check by file generation | D-2283 (read) |
| W1-api2-3 | low | api | DOCUMENTED | **PARTIAL** | Argued inherent but never timed | docs/06-limits.md:16351 'not timed here' |
| W1-api1-4 | low | api | DOCUMENTED | **PARTIAL** | One pass removed; the remaining passes are untimed | D-2284 (three passes to two, read); no timing |
| W1-api1-6 | low | api | DOCUMENTED | **PARTIAL** | Argued inherent but never timed | docs/06-limits.md:16350 'not timed here' |
| W1-api3-0 | low | api | DOCUMENTED | **PARTIAL** | Argued inherent but never timed | docs/06-limits.md:16353 'not timed here' |
| W1-api3-1 | low | api | DOCUMENTED | **FIXED** | Moved off the runtime worker | D-2282 (read) |
| W1-api5-3 | low | api | DOCUMENTED | **FIXED** | Kept per census snapshot and universe parse | D-2285 (read) |
| W1-api5-5 | low | api | DOCUMENTED | **FIXED** | Kept per snapshot, feed, name and stamp | D-2286 (read) |
| W1-api5-6 | low | api | DOCUMENTED | **FIXED** | Walked once per census snapshot | D-2289 (read) |
| W1-api5-7 | low | api | DOCUMENTED | **FIXED** | The scrub runs on the store-read pool | D-2281 (read) |
| W1-api6-0 | low | api | DOCUMENTED | **FIXED** | The scrub runs on the store-read pool | D-2281 (read) |
| W1-api5-8 | low | api | DOCUMENTED | **FIXED** | Answered by a bounded lookup | D-2280 (read) |
| W1-api5-9 | low | api | DOCUMENTED | **FIXED** | Kept per catalogue stamp and parse | D-2287 (read) |
| W1-api2-7 | low | api | DOCUMENTED | **FIXED** | Kept per catalogue stamp and parse | D-2287 (read) |
| W1-api5-11 | low | api | DOCUMENTED | **FIXED** | Listed once per parse | D-2288 (read) |
| W1-api6-3 | low | api | DOCUMENTED | **PARTIAL** | Argued inherent but never timed | docs/06-limits.md:16352 'not timed here' |
| o1api-4 | low | api | DOCUMENTED | **PARTIAL** | Bounded by the capped query length, but never timed | docs/06-limits.md:16354 'not timed here' |
| o1api-33 | low | pull | DOCUMENTED | **PARTIAL** | Memory multiple measured, but not inherent; streaming decode not built | D-2291; docs/06-limits.md:16359-16365 12x Dhan, 7x Zerodha, 17x hostile; pull --test allocation a_json_decodes_peak_memory_is_measured_against_its_body passed; fix: typed or streaming decode |
| rustonly-4 | low | api | DOCUMENTED | **PARTIAL** | Still launched by default; only an opt-out exists | crates/api/src/server.rs:19717 xdg-open; :19701 BRUTEX_NO_OPEN opt-out; fix: print the address, open only on opt-in |
| W2-cli5-4 | medium | cli | NOT-FIXED | **FIXED** | One rule check decides; the browser reads rules from the server | D-1810; win_rate_holds, avg_payoff_holds_for, VERDICT_CHECKED and tests (read) |
| W3-runner2-8 | low | runner | NOT-FIXED | **FIXED** | Digest V2 binds known values; V1 kept for old records | D-1812 column_digest_v2, column_digest_codec; runner lib exit_grid_policy::tests::column_digest_v1_never_moves_and_v2_binds_known passed |
| W3-runner2-7 | medium | runner | PARTIAL | **FIXED** | Closed by the hole-after-exit fix | D-1813 points to D-1514; cargo test -p runner --test hole_after_exit 3 passed; runner lib a_walk_built_stop_before_a_hole_is_priced_by_the_cell_and_the_replay_alike, hole_offset_is_the_earlier_hole_and_only_on_a_path_priced_before_it, unmeasured_and_refused_at_answer_each_cause_on_its_own passed |
| W3-runner5-0 | medium | runner | PARTIAL | **FIXED** | Attested once; pending candidates replayed in parallel | D-1811 OosReplaySliceV1; runner lib validate::tests::the_oos_replay_loop_attests_once_per_fold_and_replays_in_parallel and replays_on_one_oos_slice_equal_the_per_call_replay_for_every_fault passed |

## PARTIAL: the real fix

| ids | what is still missing | real fix |
|---|---|---|
| W1-pull1-0, W1-api1-6, W1-api2-3, W1-api6-3, W1-api3-0, o1api-4, W3-engine1-1 | These are argued inherent, but the D-2290 table says "not timed here" (limits:16349-16356), so the measured-bound half of DOCUMENTED is missing. | Add a committed `benches/ratio.rs` row for each kept cost, with a ceiling, or remove the cost. |
| W2-cli6-0, W2-cli2-5, W2-cli14-1/2/3, cli-14, W2-cli12-3/-4, W1-api1-4 | Passes were reduced or argued inherent (D-1845..1848, D-2284), but the remaining pass is never timed. | Time the remaining pass in `crates/cli/benches/ratio.rs` (or api's) next to C-CLI-01..06, or remove it. |
| AC-whp-o1-1 | A fresh run streams (D-1844), but a resume still restores and decodes the whole checkpointed history before ranking. | Stream the restore through the same per-level retire path, and add a peak-memory test on resume. |
| o1api-33 | The memory multiple is now measured (12x/7x/17x), but by D-2290's own text it is not inherent. | Use a typed `serde` struct, or a streaming decode for vendor bodies. |
| ET-bars-candles-store-3 | The store enforces the grid and the month but not session hours, and relies on upstream callers to check them. | Move the session table into `core` (which both crates may name), and refuse an off-session stamp at the store's write boundary. |
| GAP16-26 | Accumulation is exact in i128 (outcome.rs:1873 `wide`), but four Edge money totals are still stored and served as f64, exact only below 2^53 paisa. | Keep money totals as i64/i128 paisa in Edge, and convert only ratios to f64. |
| rustonly-4 | The server still spawns `xdg-open` (a shell script on Linux) by default (server.rs:19717); BRUTEX_NO_OPEN is only an opt-out. | Print the URL by default, and open a browser only on an explicit opt-in flag. |

## Edge-case attacks run on FIXED items

- **TOTP (h-pull-2).** Probe `crates/pull/tests/zz_audit_rnew_1.rs`, deleted after the run. It compared 68,000 random strings of lengths 0..16 against a strict RFC 4648 reference and found 0 mismatches. The strings were mostly alphabet, with an A-heavy low-entropy set so that zero pad bits occur, and some non-alphabet bytes: '0', '1', '8', '9', tab, 0xC3, '+' and '/'. Lengths 1, 3, 6, 9, 11 and 14 are never accepted. `A` and `GEZDGNBVA` give TrailingBits. 128 characters give Ok(80 bytes); 129 give TooLong.
- **Lake (h-pull-1).** Probe `crates/lake/tests/zz_audit_rnew_2.rs`, deleted after the run. These are refused: TIMESTAMP_MILLIS on the legacy annotation with no logical type, NANOS, and a non-UTC timestamp. The parquet writer itself refuses to write a logical/converted pair that disagree. Stamps i64::MIN, i64::MIN+1, -1, 0, 1, i64::MAX-1 and i64::MAX pass through unchanged. A first run with i64::MIN in the volume and OI columns hit the existing null-sentinel refusal.
- **Ledger rollback (h-cli-1, h-cli-4).** Read only, because cli is not built. A write that fails half-way, or on a full disk (ENOSPC), is cut back to the recorded end. `inject_short_write` proves this per ledger, and shrinking with `set_len` needs no free space. A crash between the seek and the write leaves no bytes, so it is safe. A crash during the write leaves a sub-record tail. 14 other cli ledger files heal it under the lock with `heal_torn_tail`; the nine in rnew-1 refuse for good.
- **CI gates (v2-1, v2-2).** Every step-disabling spelling tried was refused by the step-runs scan or by Gate 0, and a lowered pin was refused. gate14 rc=0 on the real workflow.
- **api (h-api-1..3, probeapi-1, v3a-1, v3b-2).** 9 plus 15 lib tests passed, run from the prebuilt `t-audit` api test binary (api-8db105ca58f5b8f4) at this tree. Nothing was rebuilt.

## Scope and method

- **Tree and tooling.** Tree `/home/claude/wt-read` at 9b0614be. Cargo ran only with `CARGO_TARGET_DIR=/home/claude/t-audit CARGO_BUILD_JOBS=1`, on targeted tests in store, pull, lake, runner, engine, indicators and vocab. The cli crate was read, not run.
- **Probe files.** zz_audit_rnew_1 and zz_audit_rnew_2 were deleted, and `git status --porcelain` shows nothing of mine.
- **"(read)" in the evidence column.** It marks a verdict from source and decision text, with no run.
- **Tracker split.** The tracker's 82 rows give 68 distinct ids beyond the h-*/extras rows. `cli-14 (W2-cli11-0/-1` and `W2-cli12-3/-4)` were split across two TSV rows by a stray tab and are kept as two ids.
- **Outside this pass.** The c4a-1..9 and c4b-1..6 NEW findings (decisions D-1490..1502, D-1620..1642) were not assigned here. The three `origin/fix/cloud-*` branches D-1515 names (D-0963, D-0964, D-0968) belong to other fixers.

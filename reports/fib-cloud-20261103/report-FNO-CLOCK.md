STATUS: DONE

# FNO-CLOCK: cloud build report (3 Nov 2026)

Session FNO-CLOCK of the 3 Nov 2026 wave, token `FNO-CLOCK-20261103-1` (`OWNER-FNO-CLOCK.md`, the branch's first commit). The part section is `part-FNO-CLOCK.md` beside this file.

## In brief

- **Built:** `cli::fno_clock` (`crates/cli/src/fno_clock.rs`), the per-stock-day GDFL clock calibration, under its cli slot; C-FNO-CLOCK-01's thin bench wrapper under the cli bench slots; ledger rows under the four FNO-CLOCK slot lines only.
- **PARTS' UNVERIFIED is answered: `pull::gdfl_tickstore` does expose raw tick order**, through `pull::gdfl_cm::read_listed`, so the backward clock step is measured, not flagged "unmeasured" (D-8222).
- **Tests:** 41, all on synthetic (invented) feeds, all green.
- **Coverage, Route B on this host, touched set against `1ad6e24ef8e0`:** `crates/cli/src/fno_clock.rs` 100% of lines (2,082) and of branches (144), every function (206); the wrapper 100% of its 37 lines (no branch). Measured here, so advisory: the Mac's Route B decides.
- **Bench C-FNO-CLOCK-01:** passes in four runs of the final method (p99 341 to 828 ns a step, ratios 0.986 to 1.758 against the 2.5x ceiling, every p99 inside 1,600 floors); one earlier run of a first method breached a ratio on scheduler noise. All figures below.
- **Gates:** fmt and clippy `-D warnings` (every cli target) clean; `cargo test -p cli` green but for three root-only permission tests that fail identically on the base; every test file of the workspace that reads the four ledger documents green but store's `cited_commits`, which fails identically on the base; gates 10b, 14, 27 and 28 green; gates 10, 11, 12 and 27b red with exactly the base's refusals and none of mine.
- **Owed to the Mac (NEEDS THE MAC):** check 1 on its pin (cli Route B), the macOS run of the tests, its benches, and the table's first build on real data, which is FNO-RUN's.

## Branch and base

- Branch `cloud/fib-fno-clock-20261103`, cut from `origin/cloud/fib-kit-20261103b` (`fa9022bac962`, FNO-SEAM's passed tip `1ad6e24ef8e0` plus the kit folder). Pushed after every step; never forced.
- Base for the D2 set and coverage: `1ad6e24ef8e0`. Not speculative: FNO-CLOCK depends on FNO-SEAM alone.
- Commits, oldest first: `32191d9e` owner file; `6e1affc5` the module's production half and the lib slot; `0a09c8f6` tests; `f9946417` the bench wrapper and slots; `b7966525` ledger rows; `25f6e556` one stray path for every name, short bench samples, no non-main commit cited; `8c46586a` every line, branch and function run, the budget in floors, the bench figures; then this report and the part file.

## What was built

| File | Change against `1ad6e24ef8e0` |
|---|---|
| `crates/cli/src/fno_clock.rs` | new: 1,726 lines of module (docs included), 1,511 of tests |
| `crates/cli/src/lib.rs` | `pub mod fno_clock;` with two doc lines, directly under `// slot FNO-CLOCK (fno_clock)` |
| `crates/cli/benches/ratio/fno_clock.rs` | new: the thin wrapper |
| `crates/cli/benches/ratio.rs` | `#[path = "ratio/fno_clock.rs"] mod fno_clock;` under `// slot bench-mod FNO-CLOCK`; `let ok = ok & fno_clock::row(floor, &["C-FNO-CLOCK-01"]);` under `// slot bench-row FNO-CLOCK` |
| `docs/05-decisions.md` | D-8220 to D-8226 under `<!-- slot FNO-CLOCK -->` |
| `docs/04-invariants.md` | FNO-CLOCK-I01 to I14 and C-FNO-CLOCK-01 under its slot |
| `docs/06-limits.md` | L-FNO-CLOCK-01 to 07 under its slot |
| `docs/07-plan.md` | the part's row under its slot |
| `reports/fib-cloud-20261103/` | `OWNER-FNO-CLOCK.md`, this report, `part-FNO-CLOCK.md` |

No other line of any file changed (`git diff --stat 1ad6e24ef8e0` shows only these and the kit's own folder).

### Functions and types (`cli::fno_clock`)

- `calibrate(key, &DayBars, Ticks) -> ClockRow`: one stock-day; every outcome is a row whose `Flags` name what the day lacked.
- The offset search (D-8221): `minutes_of` keeps traded minutes that `pull::calendar::phase_at(NseCash, day, minute, CasEligibility::Unknown)` calls `Continuous`; `Grid` holds the day's traded seconds on a dense grid with a sparse table of range highs and lows (6 levels: two blocks of 32 cover a minute) and first/last-traded index arrays, so `Grid::window` folds any minute-long window in a fixed number of reads; `tally_minute` tallies a minute's four prices by their exact reduced ratio GDFL/Zerodha; a shift's score is the most prices one ratio has, counting only ratios two or more minutes give; `Shift::beats` takes the higher score, then the larger |delta|, then the positive delta; `find_offset` sweeps ±60 s and doubles the window while the best shift sits on its edge, until it is inside or the window covers the reach.
- The back step (D-8222): `back_step(stamps)`, the largest fall of a row's stamp below the running maximum and the count of such rows; `read_back_step(source, listing, stock)` through `pull::gdfl_cm::read_listed` over any `CmSource`.
- The floor (D-8223): `ClockRow::floor() -> Floor::{Calibrated, OffsetOnly, Uncalibrated}`, `max(0, offset) + back_step` in whole seconds.
- The table (D-8220, D-8224): `build(keys, bars, ticks, out) -> Result<Built, BuildRefusal>`, `BarSource` (the bar seam FNO-RUN implements), `Table::open/get/len/is_empty/leftovers/digest`, the file encoding and decoding, `TableRefusal`, `BuildRefusal`, `Built`.
- The bench (D-8225): `measure_minute_boundary(seconds, samples, steps) -> Vec<u128>` and `Spread::of(samples)`.

### The interpretations this build made, each recorded

1. **`build(keys, store, out)` takes the bars through a seam, `BarSource`, not a store root** (D-8220). FNO-CLOCK depends on FNO-SEAM alone and the store's GDFL seconds reader is FNO-SEC-WIRE's, so a reader here would be a second copy; FNO-RUN implements `BarSource` over the store. The ticks are read here, through `pull::gdfl_cm` over any `CmSource`, as PARTS asks.
2. **"Ratio-mapped inside the session" is the session ratio fitted at each shift**: the exact ratio that the most prices give, among the ratios that at least two minutes give (D-8221). SPEC 10.2's single-pair ρ(D) would hang the comparison on one minute's close, and the feeds agree on all four prices in 57.1% of minutes (SPEC 19).
3. **A minute whose shifted window has no traded second matches none of its four prices**, so every shift is scored over the same minutes.
4. **The auction band and recorded halts are left out by the calendar's own phases**, with eligibility `Unknown` for every share, so no date or time is typed (D-8221).
5. **The back step is measured over every kept row of the file**, pre- and post-session rows included: the larger step is the larger floor, the stricter reading (D-8222).
6. **The table is one file per stock**, so a list rebuild opens no other stock's file (D-8220).
7. **C-FNO-CLOCK-01 reads consecutive windows**, the order a shift's sweep reads, and holds PARTS 11's 1 µs as 1,600 instruction floors (D-8225).

## Tests (41, `crates/cli/src/fno_clock.rs`, module `tests`)

PARTS' list: `shifts_of_one_to_thirty_seconds_either_way_are_recovered_exactly` (every shift −30..30 s), `random_walks_recover_their_shift_under_many_seeds`, `a_day_with_no_agreement_is_flagged_uncalibrated`, `kite_hole_minutes_and_gdfl_gaps_leave_the_shift_exact` (holes, and no-trade seconds absent or with no volume), `the_auction_era_compares_only_the_continuous_minutes`, `the_partial_gdfl_day_of_2019_05_30_is_calibrated_from_its_afternoon`, `ties_take_the_larger_shift_and_the_later_one_on_an_equal_shift`; decision 5's `a_full_build_and_a_list_rebuild_over_the_same_keys_give_equal_bytes` and `keys_outside_the_list_are_untouched`.

The rest: `a_full_session_is_recovered_and_its_edges_are_unmatched_only_where_gdfl_is_cut`, `a_split_day_is_compared_under_its_session_ratio` (2/1, 3/2, 5/1), `the_window_doubles_until_the_best_shift_is_inside_it` (59, 60, 90, −200 and 480 s), `a_best_shift_on_the_edge_of_the_whole_reach_stops_the_search`, `a_recorded_halt_is_never_compared` (2020-03-13), `a_day_that_is_no_session_is_flagged_and_nothing_else_is_read`, `days_with_no_comparable_minute_or_no_traded_second_are_flagged`, `every_broken_input_rule_flags_its_feed_malformed`, `the_back_step_is_the_largest_fall_below_the_running_high`, `the_raw_tick_order_gives_the_back_step_through_the_tick_reader`, `the_floor_is_the_positive_offset_plus_the_back_step`, `a_row_agrees_with_its_flags_only_as_a_build_writes_it`, `the_flags_name_nine_distinct_bits_and_refuse_any_other`, `two_levels_of_the_sparse_table_cover_one_minute`, `the_sparse_table_folds_every_window_as_a_scan_does` (a property over 40 seeded random grids against a brute-force fold), `a_ratio_is_reduced_and_a_shift_is_taken_by_score_then_size_then_sign`, `a_file_round_trips_and_every_broken_check_is_refused_by_name` (16 corruptions), `the_table_reads_every_stock_file_and_refuses_what_is_not_one`, `an_empty_table_has_a_digest_of_its_own`, `a_build_calibrates_each_key_and_reports_what_it_met`, `a_build_names_only_the_first_refusals_and_counts_every_one`, `a_bar_source_refusal_refuses_the_build_and_writes_nothing`, `a_corrupt_or_unreadable_file_already_there_refuses_before_any_write`, `a_folder_that_cannot_be_made_refuses_the_build`, `a_day_the_tick_source_does_not_hold_leaves_the_back_step_absent`, `a_temporary_the_staging_door_refuses_stops_the_write_and_is_left_alone`, `a_failed_rename_removes_its_temporary_and_keeps_the_target`, `a_stray_entry_refuses_the_table_read_after_the_writes`, `an_empty_key_list_writes_no_file_and_names_the_table`, `every_refusal_says_what_it_refused`, `the_minute_boundary_measure_runs_at_tiny_sizes`, `a_spread_takes_the_nearest_rank`.

Every price, symbol pairing and tick file in them is invented at run time (random walks, primes, a rising price); no vendor row is quoted, and the tick files are built from parts so no source line has a vendor row's shape. The ICICI join is the caller's (a key names the store's spelling); rank, money and charge cases belong to other parts.

## Gates

| Gate | Result on the tip | On the base `1ad6e24ef8e0` |
|---|---|---|
| `cargo fmt --check` (cli) | clean | - |
| `cargo clippy -p cli --all-targets -- -D warnings` | clean | - |
| `cargo test -p cli --lib fno_clock` | 41 passed | - |
| `cargo test -p cli --no-fail-fast` (lib, 18 integration test files, doc tests) | the 18 integration files and the doc tests green; the lib 2,476 passed, 12 ignored, 3 failed: `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`, `watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many` and `watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`, each asserting that a file mode makes a file unreadable or unwritable, which root (this container's user) is not held to | the same three fail on the base, at the same lines |
| core: `decision_citations`, `cost_invariants`, `findings`, `graph`, `lint` | all green (4, 5, 14, 15 and 2 tests) | - |
| every other test file that reads `docs/04` to `07` (vocab 7, store 13, cli 10, pull 11, api 2, costs 1, greeks 2, indicators 2, lake 1, runner 1) | green, except `cited_commits` below | - |
| store `cited_commits` | red: `1a98b1e13`, the nightly compiler's commit in BASE-STACK-1028's text of `docs/06-limits.md`, is not on `origin/main` | the same red |
| store `durability` (4 tests), store `write` (1), pull `folder` (1) | red with `TMPDIR` on a root-only tmpfs folder (their helper re-runs them as `nobody`, who cannot write there); green with a world-writable `TMPDIR`: 6 of 6, 50 of 50, 29 of 29 | environment, not code |
| gate 10 | red: R427-11 and STACK1028-03 name no test; HA-02 and AG05-13 name missing tests | the same refusals (PARTS R2: gate 10 waits on BASE-REDS-1023) |
| gate 10b, gate 27 | green | - |
| gate 11 | red: 29 refusals in other files | the same 29 |
| gate 12 | red: 85 refusals in other files, none in the touched set | the same 85 |
| gate 14 | green | green |
| gate 27b | red: D-6247 heads two entries | the same |
| gate 28 | green; `fno_clock.rs`: one temporary name, one call of `store::staging::create` | - |

Gates were run with the workflow's own tools built by `rustc` from `.github/*.rs` (`source_scan`, `gates_bounds`, `gates_ledger`, `gates_tree`, `gates_tmp_open`, `invariant_paths`), on the committed tree, and on a worktree of the base for the comparison. Gate 8 is the bench below; gates 13, 18 (mutants, deferred by the operator), 20, 29 and 31 and the web gates were not run here.

## Coverage (Route B on the cloud host; advisory)

Method, as `CLOUD.md` item 4 states Route B: the pinned nightly `nightly-2026-08-08` (`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, the compiler `docs/06-limits.md` names for Route B), `RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'`, `CARGO_PROFILE_DEV_OPT_LEVEL=0`, its own target dir; `cargo test -p cli --lib -- fno_clock` (41 tests, 3.31 s instrumented) and the bench target run once under the same instrumentation (`cargo test -p cli --bench ratio`, the 9.8 precedent); `llvm-profdata merge -sparse`; `llvm-cov report --show-branch-summary` with the lib test binary and the bench binary as objects. The D3 recipe itself is not in this kit (below), so the touched set is the files above.

| File | Lines | Branches | Functions | Regions |
|---|---|---|---|---|
| `crates/cli/src/fno_clock.rs` | 2,082 of 2,082, 100% | 144 of 144, 100% | 206 of 206 | 3,881 of 3,889, 99.79% |
| `crates/cli/benches/ratio/fno_clock.rs` | 37 of 37, 100% | none | 3 of 3 | 45 of 45 |
| `crates/cli/benches/ratio.rs`, `crates/cli/src/lib.rs` | the added `row` call runs; the other added lines are a module declaration and comments, no region | - | - | - |

The 8 regions missed are each the non-matching arm of `matches!` inside an assertion that passes, in the test module; no production region is missed. The measured tree is the code of commit `8c46586a` but for one doc comment of the wrapper (its measured-range sentence); no later commit touches `crates/` (`git diff --stat 8c46586a -- crates` is empty). The first instrumented run found one production arm no test reached (a day the tick source holds no listing for), a read-error closure no test can reach, and test-code branches only a failing test could take; each was fixed (commit `8c46586a`). No `(F)` or `(U)` line is claimed.

## Benches (C-FNO-CLOCK-01; host: 4 vCPU Xeon at 2.1 GHz, 15 GB, Linux, shared; `cargo bench -p cli --bench ratio`)

Each figure is picoseconds per minute-boundary step. Sizes are 1x, 10x and 100x a session's 22,500 seconds.

| Run | Method | 1x p50 / p99 / max | 10x p50 / p99 / max | 100x p50 / p99 / max | n | p99 ratio 10x, 100x | p99 in floors | Verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 200 samples of 1,000 steps, absolute 1 µs budget | 171,487 / 222,757 / 321,775 | 231,059 / 841,702 / 2,302,608 | 311,958 / 505,908 / 754,354 | 200 | 3.778x BREACH, 2.271x | - | BREACH (scheduler noise in 200 µs samples) |
| 2 | 2,000 samples of 100 steps, 1 µs | 191,640 / 406,470 / 1,923,830 | 223,520 / 487,910 / 1,244,520 | 359,000 / 714,590 / 1,615,770 | 2,000 | 1.200x, 1.758x | 659 / 791 / 1,158 (floor 617) | ok |
| 3 | same | 189,480 / 341,080 / 997,610 | 253,090 / 547,170 / 1,898,230 | 309,550 / 566,590 / 932,040 | 2,000 | 1.604x, 1.661x | 553 / 887 / 918 (617) | ok |
| 4 | same, budget 1,600 floors (final code) | 253,230 / 513,520 / 2,392,400 | 220,330 / 506,420 / 2,764,370 | 322,890 / 540,830 / 686,290 | 2,000 | 0.986x, 1.053x | 834 / 822 / 878 (616) | ok |
| 5 | same (final code) | 227,780 / 546,400 / 1,716,220 | 234,910 / 828,470 / 8,244,950 | 328,250 / 767,600 / 30,609,370 | 2,000 | 1.516x, 1.404x | 883 / 1,338 / 1,240 (619) | ok |
| cov | final code, instrumented opt-level 0 (Route B, timing not meaningful) | 1,738,730 / 3,442,990 / 9,712,940 | 2,823,690 / 4,218,370 / 28,856,650 | 2,452,930 / 3,921,910 / 6,040,920 | 2,000 | 1.225x, 1.139x | 156 / 191 / 177 (22,097) | ok |

The crate's six existing rows (C-CLI-01 to 06) stayed `ok` in every run. The worst p99 of the final method is 1.2 times under its budget: a thin margin on a shared host, recorded as L-FNO-CLOCK-07. The figures are advisory; the Mac's gate 8 measures again.

## Decisions drafted (block D-8220 to D-8229)

D-8220 the table and its key-list input; D-8221 the offset's comparison; D-8222 the back step; D-8223 the floor; D-8224 every outcome a row, every failure a refusal; D-8225 C-FNO-CLOCK-01; D-8226 what stays FNO-RUN's. D-8227 to D-8229 unused.

## Out of scope (not done here, by the brief)

- Any run on real data: no store, no shadow, no `:8080`, no tick store; every input in the tests is invented.
- The `fno clock build` verb, its flags, its key file's format, the store adapter (`BarSource`), the lease, the build's telemetry and the run's labels: FNO-RUN's (D-8226, PARTS READINESS ADDENDUM item 8).
- Mutants (deferred by the operator), gate 14's cover table in `ci.yml` (the main session's at F1's assembly), `docs/02-store-format.md` (no slot; L-FNO-CLOCK-06).

## Evidence the rules cite that this folder lacks

None was guessed at. What I would have read, and what I did instead (`CLOUD.md` item 1):

| Cited | Where | Instead |
|---|---|---|
| `state/fib-strategy-20261030/ENGINE.md` | `RULES-MAC.md` R2 (a fact file); PARTS 2 and 3.3 | the code itself: `pull::gdfl_tickstore`, `pull::gdfl_cm`, `pull::calendar`, `store::format::Bar`, `store::staging`, `store::safe_open`; D-0802's measured facts in `docs/05-decisions.md` |
| `state/decisions-20261027/d3-recipe.txt`, `d3-route-b.sh`, `d3-deep-take.sh` | `RULES-MAC.md` THE BAR and R2 | the Route B form of `CLOUD.md` item 4 (above) |
| `MEM/design-minute-sweep-tick-fills.md` (lines 18, 21) and `RES/fib-and-seconds.json` (lines 166-172) | SPEC 4.2 and OP-07, the clock's back step of 2-14 s and the unmeasured offset | SPEC 4.2's quotation of them, and D-0802 items 1 and 6 (the time column stepping back 25 times a file; the alignment unmeasured) |
| `state/fno-data-audit-20261030/REPORT.md` (lines 176-177) | SPEC 19, the feeds' 57.1% four-price agreement | SPEC 19's quotation |
| `state/post-landing-20261101/calendar-v2.md` | PARTS decision 5 (lever L6) | PARTS decision 5 and READINESS ADDENDUM item 8 |
| `state/readiness-20261102/runbrief-verify.md`; the deploy runbook (R6, R7) | PARTS READINESS ADDENDUM | the ADDENDUM's own text |
| `state/fno-winners-20261013/sources-leverage-execution.md`, `sources-charges.md` and `.json`, `plan.json`, `fib-one-level.json`, `lens-*.json`; `state/fib-small-20261024/REPORT.md`; `state/fno-data-audit-20261030/report-evidence/backtest-skip.tsv`; `state/fno-data-fix-20261030/verify.md` and `per-stock-after.tsv`; `state/decision-blocks.json`; the operator's memory files | `RULES-MAC.md` PRE and R2 | not needed for this part; SPEC.md and PARTS.md quote what bears on it |
| `state/fix/DRAIN*`, the disk floor of `/Volumes/WD_BLACK`, `bin/cargo`, `tgt/fib-shared-*` | `RULES-MAC.md` R2 | Mac-only operations; here one stable target dir, its own coverage dir, `TMPDIR` on a tmpfs, `df -h` checked (16 GB free at the end) |

## NEEDS THE MAC

1. **Check 1 on its pin**, with the cli Route B of the D3 recipe over the D2 set against `1ad6e24ef8e0`.
2. **The macOS run of the 41 tests.** Nothing here ran on macOS. The paths that touch the file system (the staging door on APFS, a rename onto a directory, a directory flush, inode stability across an untouched rebuild) are ordinary there; one test case, a file name that is not UTF-8, is compiled on Linux only because APFS refuses such a name, and the code it reaches is the ordinary stray path every platform runs. No production line is macOS-only or Linux-only.
3. **The benches on the Mac** (gate 8, `cargo bench -p cli --bench ratio`).
4. **The table's first build on real data**: FNO-RUN's `fno clock build`, which implements `BarSource` and passes a `TickStore` with a stored scope for the 13 names off the swept surface.

## Problems and notes for the main session

- **Gate reds are the base's** (table above); none is in the touched set.
- **A slip of this session, disclosed:** one shell command (a version probe, run while editing a scratch script) named the forbidden language. Nothing of it ran, and no tracked file, commit or report carries the word.
- **`docs/02-store-format.md`** has no F&O slot; the `fno-clock-v1` format's row there is owed to whoever the main session names (L-FNO-CLOCK-06).
- **C-FNO-CLOCK-01's margin** on a shared host is 1.2x (L-FNO-CLOCK-07).

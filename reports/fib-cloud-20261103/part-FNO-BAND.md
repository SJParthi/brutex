# Part FNO-BAND

## Cloud build, Tue 3 Nov 2026

- **held:** false. **Status:** built and ready for its independent check;
  nothing counts until the Mac's check passes it.
- **Where:** a cloud session (`CLOUD.md`), branch `cloud/fib-fno-band-20261103`,
  pushed; cut from `origin/cloud/fib-kit-20261103b` (`fa9022ba` = FNO-SEAM's
  passed tip `1ad6e24ef8e0` plus the kit folder). Never merged, no pull
  request, no other branch pushed.
- **Base:** `1ad6e24ef8e0`, FNO-SEAM's passed tip. FNO-BAND depends on
  FNO-SEAM alone, so this is the final base, **not speculative**, and no
  catch-up is owed.
- **The part's commits** (`git rev-list --no-merges 1ad6e24ef8e0..HEAD`, less
  the kit commit `fa9022ba`, which holds only `reports/fib-cloud-20261103/`):
  `a5542839`, `f8f77d49`, `aff0594a`, `ab3393f8`, `46def571`, and the commits
  that add and complete this file and `report-FNO-BAND.md`. Each message ends
  with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Drain test and disk floor:** not applicable: they name the Mac's drive,
  and the cloud container has none (`CLOUD.md` adaptation 2). No store, no
  `runtime/`, no app and no `:8080` were read, written, started or needed.

### Files (the design's list, each new or at its own slot)

| File | Change |
|---|---|
| `crates/runner/src/fno_band.rs` | new: the module and its 25 tests |
| `crates/runner/src/lib.rs` | `pub mod fno_band;` under `// slot FNO-BAND (fno_band)` |
| `crates/runner/benches/ratio/fno_band.rs` | new: C-FNO-BAND-01's thin wrapper |
| `crates/runner/benches/ratio.rs` | the `#[path]` and `mod` lines under `// slot bench-mod FNO-BAND`; `fno_band::row(&["C-FNO-BAND-01"]),` under `// slot bench-row FNO-BAND` |
| `docs/05-decisions.md` | D-8210 to D-8212 under `<!-- slot FNO-BAND -->`; D-8213 to D-8219 unused |
| `docs/04-invariants.md` | FNO-BAND-I01 to FNO-BAND-I07 and C-FNO-BAND-01 under its slot |
| `docs/06-limits.md` | L-FNO-BAND-01 to L-FNO-BAND-06 under its slot |
| `docs/07-plan.md` | the plan row under its slot |

Every shared-file edit is one pure insertion under FNO-BAND's own slot line
(`git diff -U0 1ad6e24ef8e0`, no line removed). `git merge-tree --write-tree`
of this tip with `origin/cloud/fib-fno-law-20261103` and with
`origin/cloud/fib-fno-portfolio-20261103` exits 0.

### Functions and data structures

`DayBand::new(base, flex)` (the design's `DayBand::new(base_reading, base)`:
the reading travels inside `Base`, and the flex reading is the second
argument, D-8210 item 7), `DayBand::observe(sec)`, `DayBand::limits(ts)`,
`DayBand::near(sec, step)`, `DayBand::{base, flex}`;
`Base::close_times_ratio` (R-BAND-1), `Base::last_trade_before` (R-BAND-2),
`Base::{reading, paise, first_limits}`; `FlexReading { FivePerStep,
FiveThreeTwo }`, `BaseReading`, `ExDay`, `SessionRatio`, `Prints`, `Limits`
(`pair()` is the `(lower, upper)` FNO-RUN adapts to FNO-SEC-EXEC's `Bands`),
`BandSide`, `Flex`, `Observed`, `Refusal` (8 variants, `Display`, `Error`);
`LABEL` = "band computed", `BAND_RULE` = "fno-band-v1"; the bench's `Samples`,
`quantile`, `bench_day`, `measure_checks`. Details:
`report-FNO-BAND.md` section 2.

### Tests (25 of 25 green, stable and Route B)

The design's seven: `prints_exactly_at_the_band_are_at_it_and_flex_nothing`,
`a_flex_is_observed_from_a_print_past_the_band_and_holds_from_its_second`,
`a_sell_at_the_lower_band_and_a_buy_at_the_upper_band`,
`trading_resumes_off_the_band_and_the_band_never_narrows`,
`a_split_day_takes_the_adjusted_close_times_the_session_ratio_inside_one_session`,
`a_demerger_ex_day_is_skipped_under_both_readings`,
`a_first_print_of_the_day_already_at_the_band`. THE BAR's and the rest:
`a_no_trade_gap_then_a_print_past_several_flexes_takes_them_in_one_second`,
`a_second_past_both_limits_flexes_both_sides`,
`the_band_answers_an_earlier_second_as_it_stood_then`,
`a_print_one_paisa_inside_at_and_past_each_limit`,
`the_limits_round_inward_to_the_paisa`, `the_base_of_each_reading`,
`every_refusal_is_named`, `the_i128_guards_refuse_a_band_past_the_types`,
`near_is_within_one_step_of_a_limit`,
`the_label_the_rule_word_and_the_reading_names`,
`quantiles_take_the_nearest_rank`,
`the_bench_fixture_times_every_second_it_walks`. Seeded properties:
`no_later_second_moves_an_earlier_band`,
`the_band_holds_every_print_it_observed_and_never_narrows`,
`each_flex_is_the_fewest_steps_that_hold_the_print`,
`the_designs_band_is_never_wider_than_specs_while_it_holds`,
`asking_in_any_order_gives_the_same_answers`,
`a_trade_far_from_every_band_walks_one_lane`. What each proves:
`report-FNO-BAND.md` section 3.

### coverage

**Measured here by Route B, against the final base `1ad6e24ef8e0`, not
speculative.** Nightly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`
(`nightly-2026-08-08`), `RUSTFLAGS='-C instrument-coverage -Z
coverage-options=branch'`, `CARGO_PROFILE_DEV_OPT_LEVEL=0`, `cargo test -p
runner --locked --lib -- fno_band`, `llvm-profdata merge`, `llvm-cov report`:

`crates/runner/src/fno_band.rs` at `46def571`: **lines 1,069 of 1,069
(100.00%), branches 48 of 48 (100.00%)**, regions 1,806 of 1,806, functions 97
of 97, no zero-count marker. The bench wrapper, run once under the same
instrumentation: `report-FNO-BAND.md` section 8; its self-check failure arms
(lines 52-53 and 88 of `crates/runner/benches/ratio/fno_band.rs`) are excused
by name. `lib.rs`'s slot line has no region; the ledgers hold none. The D3
recipe file is on the Mac and was not read: this followed `CLOUD.md`'s
statement of Route B, and the Mac re-measures.

### gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy -p runner --all-targets --locked -- -D warnings`: clean.
- `cargo test -p runner --locked --no-fail-fast`: green (lib 770, every
  integration binary, doc-tests 6).
- Readers of the edited ledgers: `core` and `vocab` whole, green; the rest in
  `report-FNO-BAND.md` sections 4.1 and 8.
- Gates 27 and 14: OK. Gates 27b, 10 and 12: red on the base and here with
  the same refusals, none of them this part's. Gate 10's later stages check
  26 more named tests here than at the base, all resolving. As the main
  session's §7.5 (c) says, gate 10's crate-first half is re-run after
  BASE-REDS-1023 lands.
- Bench C-FNO-BAND-01 (advisory here): flat, check p99 73 to 85 ns, look-back
  p99 50 to 53 ns, inside the 100 ns budget under both flex readings.
- Mutation tests: deferred by the operator. `cargo deny check`: not installed
  here; no dependency, manifest or lock line changed.

### decisions drafted (own block only)

D-8210, D-8211, D-8212. D-8213 to D-8219 unused.

### problems

- Not this part's, reproduced on the base: gate 27b (D-6247 twice), gate 10
  (R427-11, STACK1028-03; HA-02 and AG05-13), gate 12's refusal list.
- C-R-04, a runner bench row this part does not touch, breached once on this
  host (5,700 floors against 5,000) with its numerator unchanged and its
  one-shot floor faster; `report-FNO-BAND.md` sections 6 and 8.
- A bench run in a base worktree sharing this checkout's `target/` left a stale
  binary that the next tip bench ran; that output is counted as a base sample
  only (`report-FNO-BAND.md` section 9 item 6).

### findings

- T06-03's flex is read two ways (SPEC 10.2: 5% a step; PARTS: 5%, then 3%,
  then 2%). Both are walked as lanes (D-8211); the main session should read
  `sources-leverage-execution.md` lines 21 and 189 and retire the refuted
  reading in a new entry.
- R-BAND-1's ratio reads the session's first traded minute (SPEC 10.2), which
  matters for FNO-DAILY's D+1 entries in that minute (L-FNO-BAND-01); FNO-RUN
  decides which lanes it walks there.
- R-BAND-2 is in the old basis across a split, bonus or rights issue until
  R4-26 is live (L-FNO-BAND-02); NSE's band rounding is unmeasured until
  FNO-BAND-FILES (L-FNO-BAND-03).

### refused_files

None.

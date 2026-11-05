STATUS: DONE

# report-PSRB-STV.md: the two-band Supertrend, session TOKEN PSRB-STV-20261103-1

Part PSRB-STV of the 3 Nov 2026 PSRB wave (PARTS.md section 5), built Thu 5 Nov 2026 in a Claude Code cloud
container (Linux, 4-core Intel Xeon at 2.10 GHz, 15 GiB). Branch `cloud/psrb-stv-20261103`, cut from
`origin/cloud/psrb-kit-20261103` (`56f3043dd4ca`). START = PSRB-SEAM `35234cead4009299a93fcd79a3d1fffd773dcc16`.
**Code tip (`git rev-list -1 HEAD -- . ':(exclude)reports'`): `c3c39a44bc23d6c08c806140f00d17444da9e490`.** Its Rust
is byte-identical to `0fecf16c610b` (`git diff --stat 0fecf16c c3c39a44 -- '*.rs'` is empty), where Route B and the
hand changes ran. What the Mac's check reads first: `part-PSRB-STV.md`. Result: `result-PSRB-STV.json.md`.

Commits (all pushed, none rewritten): `a6f2d565` OWNER file; `03f3005e` module, tests, bench wrapper, lib.rs and
ratio.rs lines; `6ce14e70` report in progress; `b9c40b61` the four ledger blocks; `0fecf16c` one condition per test
assertion (after Route B's first reading); `c3c39a44` L-PSRB-STV-04 states both bench runs; then this report.

## 1. Kit check

- `sha256sum MANIFEST.md` = `85b873ebc2a7cc86713701cbc7619bb3b14058b99225e4249fdbfb403c889ad8`, the hash the kit commit's
  message states. All twelve rows match, by the manifest's own extraction (`awk` between the five-backtick lines):
  CLOUD.md `19dff357...`, PARTS.md `1a96fc9b...`, d3-recipe.txt `9ed425e3...`, d3-route-b.sh `f3d70ac2...`,
  d3-route-b-cloud.sh `f032b525...`, d3-covset.mjs `ecef3bdc...`, sim.mjs `c78e72b8...`, fixtures.mjs `e4818033...`,
  fix2.mjs `d03d3ceb...`, rsi-run.mjs `a99a83ad...`, min-run.mjs `3c18cb3c...`, faithful-edge.mjs `2ed69004...`.
- `git diff --name-only START origin/cloud/psrb-kit-20261103` lists only the 13 files of `reports/psrb-cloud-20261103/`.
- The fixtures of PARTS.md 5.4 re-printed by `node fixtures.mjs` and `node fix2.mjs` (the kit's models, extracted to
  scratch, never into the tree) equal every value 5.4 states (5.4 prints the ATR column for ST-1 only; it agrees). The LITERAL-PINE figures of 5.3 and
  D-8602 (`Down 152 (152, 134)`, `Down 151 (151, 137)`, `Down 144 (144, 137)` at ST-9 candles 4 to 6, agreeing
  elsewhere) were re-derived by a small node model of the literal comparison on the kit's `Atr`, written for this step.
- Host baseline: `cargo test -p indicators --locked` on a worktree of START, pinned stable 1.97.1: **504 passed, 0
  failed**. No host red exists to carry.

## 2. What was built

`indicators::dual_band_supertrend`: the standard two-band Supertrend (both bands ratchet on every candle, the trend
turns when the close crosses the CURRENT candle's band, the previous TREND picks the band judged) as a second,
labelled reading beside the latch `indicators::trend::SuperTrend`, which is untouched. `trend::Atr` is reused
unchanged, the band arithmetic is the latch's in `i128`, the script's zero prior bands and first-candle `Down` are
kept, and `settled` marks the first ATR candle. Exactly the API of PARTS.md 5.2; the rules of 5.3; the refusals; the
23 tests of 5.5 by name with fixtures ST-1 to ST-9 asserting both forms on every candle; C-PSRB-STV-01 and -02 of 5.6
with the measuring functions in the module and a thin wrapper; the `pub mod` line; the four ledger blocks; D-8600 to
D-8605 (D-8606 to D-8609 untouched, spare).

One finding during the build, fixed before the first code commit: the first synthetic bench stream (independent
random opens and closes) never turned up, because noise alone never reaches `mid + 3 ATR`, so the bench would have
timed one branch path. `the_bench_measures_both_rows_at_tiny_sizes` caught it. The stream now carries a 400-candle
triangle drift under splitmix64 noise, and the test asserts turns both ways in 2,000 candles.

## 3. Files and functions (PARTS.md 5.1 and 2.2; nothing else outside `reports/`)

| file | change |
|---|---|
| `crates/indicators/src/dual_band_supertrend.rs` | new, 416 lines. Public: `MAX_FACTOR_MILLI`, `Refusal` (`PeriodZero`, `FactorOutOfRange { milli }`, `Corrupt(Corrupt)`), `Reading` (`trend`, `line`, `upper`, `lower`, `settled`), `DualBandSuperTrend` (`new`, `fold`, `reading`, `atr`), `StepCost`, `summarize`, `flat_within`, `measure_step`, `measure_period`. Private: `at_rank`, `timed`, `synthetic`, four constants, three `const` bound asserts and the size assert (`<= 256`). |
| `crates/indicators/src/dual_band_supertrend_tests.rs` | new, 1,109 lines: the 23 tests, the fixture checker (two-band form, latch and engine ATR side by side), a seeded xorshift walk, and `transcribe`, the from-scratch series transcription of 5.3. |
| `crates/indicators/benches/ratio/psrb_stv.rs` | new, 104 lines: `row`, `judge`, `report`; 7 rounds of 10,000 steps per size; budget 2,000 ns; ceiling `super::CEILING_PERMILLE`. |
| `crates/indicators/src/lib.rs` | +1 at :66, `pub mod dual_band_supertrend;` directly under `pub mod daily;`. |
| `crates/indicators/benches/ratio.rs` | +2 at :73-74 (`#[path = "ratio/psrb_stv.rs"]`, `mod psrb_stv;`) directly under `// slot bench-mod PSRB-STV`; +5 at :788-792 (the `verdicts` re-binding with `psrb_stv::row(floor, &["C-PSRB-STV-01", "C-PSRB-STV-02"])`) directly under `    // slot bench-row PSRB-STV`. Adds no `ratio(` call site: gate 14's pinned count for this file stays 6. |
| `docs/04-invariants.md` | +24 at :9062-9085, directly above `## /db reads only what its view shows (R3-12; D-7111)`. |
| `docs/05-decisions.md` | +140 at :86993-87132, directly above `### D-7111 — /db reads only what its view shows: ...`. |
| `docs/06-limits.md` | +15 at :22067-22081, directly above `## /db reads only what its view shows: what R3-12 leaves bounded, unmeasured or open (R3-12, D-7111)`. |
| `docs/07-plan.md` | +6 at :1110-1115, directly above `## GDFL import at one second — 2026-10-05 (D-2802 to D-2806)`. |

Every ledger insertion is pure (no deleted line), after the blank line that precedes its anchor, and ends with one
blank line. Signatures: `new`, `reading` and `atr` are plain `fn` as 5.2 shows them (`new` widens with `i128::from`).
`StepCost` also derives `Default` (an empty round reads zeros). Measured state size: **224 bytes** (the test pins it).

## 4. Tests by name (`crates/indicators/src/dual_band_supertrend_tests.rs`, 23)

`a_zero_period_and_a_factor_outside_its_range_are_refused`, `nothing_is_read_before_the_atr_has_its_period`,
`the_first_atr_candle_is_down_on_the_upper_band_and_unsettled`, `period_one_reads_the_published_zero_band_first`,
`both_bands_ratchet_on_every_candle_and_reset_as_published`, `a_turn_needs_a_close_beyond_the_current_band`,
`fixture_st_1_the_seed_differs`, `fixture_st_2_a_flip_down_keeps_the_ratcheted_upper_band`,
`fixture_st_3_a_flip_up_keeps_the_ratcheted_lower_band_and_turns_down_first`,
`fixture_st_4_a_flat_series_reads_up_in_the_latch_and_down_here`,
`fixture_st_5_a_wide_candle_turns_the_two_band_form_first`, `fixture_st_9_equal_bands_do_not_turn_an_up_candle`,
`a_gap_through_either_band_turns_on_the_gap_candle` (ST-7, ST-8), `an_all_up_and_an_all_down_series`,
`the_stream_equals_a_from_scratch_transcription_of_the_published_rules` (300 random series, periods 1 to 20, factors
500 to 5,000, compared at every candle; it asserts more than 10,000 readings, more than 500 turns and at least one
candle with equal bands), `the_atr_is_the_engine_atr`, `no_reading_depends_on_a_later_candle`,
`a_time_gap_reads_as_contiguous_candles`, `the_extremes_never_overflow` (high `i64::MAX`, low 1, factor 100,000,
periods 1 and 65,535), `a_corrupt_candle_is_refused_and_changes_nothing`, `the_state_is_a_fixed_size`,
`the_bench_measures_both_rows_at_tiny_sizes`, `a_cost_is_read_by_rank_and_judged_flat_or_not`.

Every fixture test asserts, on every candle, the engine ATR's `value()`, the latch's trend and stop, and the two-band
reading with `settled`, plus `atr()`. The latch's values are PARTS.md 5.4's for eight fixtures; 5.4 does not state
ST-9's, so those are `fix2.mjs`'s output. The ATR values are 5.4's for ST-1 and the kit models' for the other eight.

## 5. Gates (pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`, at the code tip's tree)

| command | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy -p indicators --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test -p indicators --locked` | exit 0: **527 passed**, 0 failed, 0 ignored (START's 504 plus 23) |
| `cargo test -p core --locked --test cost_invariants --test decision_citations` | exit 0: **9 passed** (5 + 4) |
| `git diff --exit-code START HEAD -- crates/indicators/src/trend.rs` | empty, and so are `evaluator.rs`, `vwap.rs`, `daily.rs`, `crates/indicators/tests`, `crates/vocab`, `Cargo.toml`, `Cargo.lock`, `CLAUDE.md`, `AGENTS.md`, `docs/00-charter.md` and `.github` |

**Gates 10, 11 and 12, run here as CI runs them** (the tools built from this tree's `.github/source_scan.rs`,
`gates_ledger.rs`, `gates_bounds.rs` and `invariant_paths.rs` with `rustc --edition=2024 -D warnings`, and the step
commands copied from `.github/workflows/ci.yml`): each exits 1 **both at START and at the tip, with the same
refusals**, all in other crates (`cli`, `store`, `api`, `pull`; for example gate 11's `crates/store/src/quarantine.rs`
and gate 12's `crates/api/src/autopilot.rs:2789`; gate 10's `STACK1028-03`). The diff of the two logs holds only
counts (gate 11 walks 525 production files against 524; gate 12 sees 798 claim blocks against 797) and gate 10's
`STACK1028-03` line number, moved by this part's 24 docs/04 lines. So this part adds no refusal to any of the three:
every docs/04 row's named tests resolve (gate 10), the production closure holds none of rules 1 to 7 (gate 11), and
the module's one cost-claim block names `crates/indicators/benches/ratio.rs`; gate 12 files it with the blocks that
say UNVERIFIED, because the same `//!` block says the definition is UNVERIFIED against `TradingView`'s own output, as
PARTS.md 2.4 asks. Gate 14 was not run: its cover-table row for the two new ids is the main session's (section 11).

## 6. Coverage: Route B on the pinned nightly

Pins (accepted only because `rustc -V` prints exactly them): `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`
(`nightly-2026-08-08`), `llvm-cov` and `llvm-profdata` `LLVM version 23.1.0-rust-1.99.0-nightly`. Command, in a
worktree at `0fecf16c610b` (Rust identical to the tip), no `RUSTC_BOOTSTRAP`:

    D3_NIGHTLY=<nightly> D3_LLVM_BIN=<nightly>/lib/rustlib/x86_64-unknown-linux-gnu/bin D3_CARGO=<nightly>/bin/cargo \
      d3-route-b-cloud.sh <tree> <deep-1> <new profile folder> <new log folder> <set file> indicators

It builds once with `RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'` and `CARGO_PROFILE_DEV_OPT_LEVEL=0`,
runs `cargo test -p indicators` (527 passed, 21 profiles), merges with `llvm-profdata merge -sparse` and reads each set
file with `d3-covset.mjs`. It exits 3 for one reason: `cargo test` never builds the bench target (`test = false`), so
`benches/ratio.rs` and `benches/ratio/psrb_stv.rs` are in no object of that call. **The bench target was then run once
under the same instrumentation** (same nightly, RUSTFLAGS, opt-level 0 and target dir: `cargo test -p indicators --bench
ratio --locked`, `main` run whole, every row `ok`), its profile merged with Route B's 21, and all five set files read
again with the same reader: `not_proved=0`.

The set (narrow reading: the tip holds no `Merge BASE-COV-LIBS`), written before measuring:

```
reading narrow
coverage crates/indicators/src/dual_band_supertrend.rs whole
coverage crates/indicators/src/dual_band_supertrend_tests.rs whole
coverage crates/indicators/benches/ratio/psrb_stv.rs whole
coverage crates/indicators/benches/ratio.rs main 758-821
coverage crates/indicators/benches/ratio.rs line 73
coverage crates/indicators/benches/ratio.rs line 74
coverage crates/indicators/src/lib.rs line 66
mutants crates/indicators/src/dual_band_supertrend.rs whole
mutants crates/indicators/src/dual_band_supertrend_tests.rs whole
mutants crates/indicators/benches/ratio/psrb_stv.rs whole
mutants crates/indicators/benches/ratio.rs main 758-821
mutants crates/indicators/benches/ratio.rs line 73
mutants crates/indicators/benches/ratio.rs line 74
mutants crates/indicators/src/lib.rs line 66
```

| file | item | lines | branches |
|---|---|---|---|
| `crates/indicators/src/dual_band_supertrend.rs` | whole | **148/148** | **28/28** |
| `crates/indicators/src/dual_band_supertrend_tests.rs` | whole | **775/775** | **54/54** |
| `crates/indicators/benches/ratio/psrb_stv.rs` | whole | 45/48 | 3/6 |
| `crates/indicators/benches/ratio.rs` | `main` 758-821 | 24/26 | 1/2 |
| `crates/indicators/benches/ratio.rs` | lines 73, 74 | 0/0 | 0/0 |
| `crates/indicators/src/lib.rs` | line 66 | 0/0 | 0/0 |

**Every uncovered line and side is an (F) item, a self-checking bench's own failing branch (recipe step 4); no GAP,
no (U).** Each is reached only when one of that file's own checks fails:

- (F) `crates/indicators/benches/ratio.rs:816` branch True, `if verdicts.contains(&false) {`; `:817`
  `println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");`; `:818` `std::process::exit(1);`: gate 8's
  failing exit (lines of START's `main`, in the set because this part adds a statement to `main`).
- (F) `crates/indicators/benches/ratio/psrb_stv.rs:39` branch False, `let [step_id, period_id] = ids else {`; `:40`
  `println!("  C-PSRB-STV REFUSED: the row judges two ids and was given {ids:?}");`; `:41` `return false;`: the row
  refusing an id list that is not exactly its two ids.
- (F) `psrb_stv.rs:68` `Err(why) => println!("{id} {axis} {size}: REFUSED {why:?}"),` and `:71` branch False,
  `let ok = kept.len() == HISTORIES.len() && flat_within(...)`: a refused measurement printing its reason and failing
  the row. The row passes only constants (periods 10, 100 and 1,000; factor 3,000) and synthetic candles, which are
  evaluable by construction (the lowest low is Rs 2,500 less 3.83 rupees of noise and wick).
- (F) `psrb_stv.rs:79` branch False, `if ok { "ok" } else { "BREACH" }`: the row's breach.

Route B's first reading, at `b9c40b61`, gave the tests file 762/764 lines and 75/92 branches, every miss the failing
side of an `a && b` or `a || b` inside an `assert!` or a message argument evaluated only on failure. Those were (F)
too; `0fecf16c` removed them instead (one condition per assertion, messages bound before it), with the conditions
unchanged, and the second reading above is clean. The module itself read 148/148 and 28/28 both times.

Route B's own lines for the tip (paths shortened to `$SP`, the scratch folder):

```
ROUTE-B START 2026-11-05 08:51:45 IST tree $SP/cov-tree tip 0fecf16c610bd3df1bb95a5abde09c8e1367870a packages indicators K $SP/deep-1 P $SP/prof-0fecf16c610b-1-B (entries before: 0)
versions: rustc 1.99.0-nightly (1a98b1e13 2026-08-07) | llvm-cov  LLVM version 23.1.0-rust-1.99.0-nightly | llvm-profdata  LLVM version 23.1.0-rust-1.99.0-nightly
OPS-10: every tracked file of $SP/cov-tree touched 2026-11-05 08:51:45 IST
== build START 2026-11-05 08:51:45 IST: cargo test -p indicators --locked --no-run --message-format=json (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', opt-level 0, CARGO_TARGET_DIR=$SP/deep-1/cov)
== build rc=0 END 2026-11-05 08:51:50 IST (0 min); 23 artifacts, 0 reused
objects: 21 ($SP/prof-0fecf16c610b-1-B/objects.txt, from this build's own report)
profile $SP/prof-0fecf16c610b-1-B/indicators entries before: 0
== test indicators START 2026-11-05 08:51:50 IST
== test indicators rc=0 END 2026-11-05 08:58:23 IST (6 min); tests passed 527 failed 0 ignored 0
profraw files: 21
merged $SP/prof-0fecf16c610b-1-B/indicators/merged.profdata
COVERAGE NOT READ: $SP/logs/cov-0fecf16c610b/show-indicators-crates_indicators_benches_ratio.rs.txt holds no source line (llvm-cov show failed or the file is in no object): NOT PROVED
ROUTE-B NOT PROVED: crates/indicators/benches/ratio.rs: not read ($SP/logs/cov-0fecf16c610b/show-indicators-crates_indicators_benches_ratio.rs.err)
COVERAGE NOT READ: $SP/logs/cov-0fecf16c610b/show-indicators-crates_indicators_benches_ratio_psrb_stv.rs.txt holds no source line (llvm-cov show failed or the file is in no object): NOT PROVED
ROUTE-B NOT PROVED: crates/indicators/benches/ratio/psrb_stv.rs: not read ($SP/logs/cov-0fecf16c610b/show-indicators-crates_indicators_benches_ratio_psrb_stv.rs.err)
COVERAGE crates/indicators/src/dual_band_supertrend.rs whole: lines 148/148, branches 28/28
COVERAGE crates/indicators/src/dual_band_supertrend.rs TOTAL: lines 148/148, branches 28/28
COVERAGE crates/indicators/src/dual_band_supertrend_tests.rs whole: lines 775/775, branches 54/54
COVERAGE crates/indicators/src/dual_band_supertrend_tests.rs TOTAL: lines 775/775, branches 54/54
COVERAGE crates/indicators/src/lib.rs line 66: lines 0/0, branches 0/0
COVERAGE crates/indicators/src/lib.rs TOTAL: lines 0/0, branches 0/0
ROUTE-B DONE 2026-11-05 08:58:24 IST WITH NOT PROVED lines above
ROUTE-B EXIT 3
BENCH-COV START 2026-11-05 08:58:24 IST tree $SP/cov-tree tip 0fecf16c610bd3df1bb95a5abde09c8e1367870a
bench objects: $SP/deep-1/cov/debug/build/indicators/bf80d91fcec4bfe4/out/ratio-bf80d91fcec4bfe4
bench run rc=0 (272 s); its printed lines are in $SP/logs/benchcov-0fecf16c610b/bench-run.log
bench profraw files: 1
COVERAGE crates/indicators/benches/ratio.rs main 758-821: lines 24/26, branches 1/2
  UNCOVERED LINE crates/indicators/benches/ratio.rs:817: println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");
  UNCOVERED LINE crates/indicators/benches/ratio.rs:818: std::process::exit(1);
  UNCOVERED BRANCH crates/indicators/benches/ratio.rs:816:8 True: if verdicts.contains(&false) {
COVERAGE crates/indicators/benches/ratio.rs line 73: lines 0/0, branches 0/0
COVERAGE crates/indicators/benches/ratio.rs line 74: lines 0/0, branches 0/0
COVERAGE crates/indicators/benches/ratio.rs TOTAL: lines 24/26, branches 1/2 (every uncovered line and side above is a GAP unless it is an (F) or (U) item of recipe step 4)
COVERAGE crates/indicators/benches/ratio/psrb_stv.rs whole: lines 45/48, branches 3/6
  UNCOVERED LINE crates/indicators/benches/ratio/psrb_stv.rs:40: println!("  C-PSRB-STV REFUSED: the row judges two ids and was given {ids:?}");
  UNCOVERED LINE crates/indicators/benches/ratio/psrb_stv.rs:41: return false;
  UNCOVERED LINE crates/indicators/benches/ratio/psrb_stv.rs:68: Err(why) => println!("{id} {axis} {size}: REFUSED {why:?}"),
  UNCOVERED BRANCH crates/indicators/benches/ratio/psrb_stv.rs:39:9 False: let [step_id, period_id] = ids else {
  UNCOVERED BRANCH crates/indicators/benches/ratio/psrb_stv.rs:71:14 False: let ok = kept.len() == HISTORIES.len() && flat_within(&kept, ceiling, BUDGET_NS);
  UNCOVERED BRANCH crates/indicators/benches/ratio/psrb_stv.rs:79:12 False: if ok { "ok" } else { "BREACH" }
COVERAGE crates/indicators/benches/ratio/psrb_stv.rs TOTAL: lines 45/48, branches 3/6 (every uncovered line and side above is a GAP unless it is an (F) or (U) item of recipe step 4)
COVERAGE crates/indicators/src/dual_band_supertrend.rs whole: lines 148/148, branches 28/28
COVERAGE crates/indicators/src/dual_band_supertrend.rs TOTAL: lines 148/148, branches 28/28
COVERAGE crates/indicators/src/dual_band_supertrend_tests.rs whole: lines 775/775, branches 54/54
COVERAGE crates/indicators/src/dual_band_supertrend_tests.rs TOTAL: lines 775/775, branches 54/54
COVERAGE crates/indicators/src/lib.rs line 66: lines 0/0, branches 0/0
COVERAGE crates/indicators/src/lib.rs TOTAL: lines 0/0, branches 0/0
BENCH-COV DONE 2026-11-05 09:02:57 IST not_proved=0
EXIT 0
```

## 7. Each new test red under a hand change

25 hand changes (M01 to M25), each applied alone to `dual_band_supertrend.rs` in a separate worktree at `0fecf16c`
by exact one-occurrence text replacement, then `cargo test -p indicators --locked --lib dual_band_supertrend`
(stable 1.97.1, opt-level 0), then restored (`git status` clean after every one). Unchanged: 23 passed. Every hand
change compiled and turned at least one test red; **every one of the 23 tests went red under at least one**, each
for the intended reason where checked (M19: size 256 against 224; M17: `attempt to multiply with overflow`; M12:
`ST-9 candle 4` reads `Down 152 (152, 134)`, LITERAL-PINE's figure; M14: ST-2 candle 10 reads `Down 140`, the latch's
re-seed; M07: ST-6's first upper band 120 against 0; M16: the whole state differs across a time gap). An earlier run at
`b9c40b61`, before the assertions were split, gave the same exit for every hand change.

| id | hand change |
|---|---|
| M01 | the period-0 refusal removed |
| M02 | the factor bound off by one (`> MAX_FACTOR_MILLI + 1`) |
| M03 | `fold` reads the ATR before it is warm (`.filter(warm)` removed) |
| M04 | `atr()` reads the ATR before it is warm |
| M05 | the first ATR candle reads `Up` |
| M06 | `settled` true from the first reading |
| M07 | a missing previous close compares true (`is_none_or`) |
| M08 | no zero prior bands (`i128::MAX` and `i128::MIN`) |
| M09 | no ratchet on the upper band (always the basic band) |
| M10 | the upper band resets on a close EQUAL to it (`>=`) |
| M11 | a close EQUAL to the upper band turns the trend up (`>=`) |
| M12 | LITERAL-PINE: the previous line compared with the previous upper band picks the band judged |
| M13 | the close judged against the PREVIOUS candle's bands (the latch's comparison) |
| M14 | the band re-seeded at the basic band on a flip (the latch's re-seed) |
| M15 | state shared outside the instance: a thread-local fold count reaches `settled` |
| M16 | the fold reads the timestamp (minute parity added to `mid`) |
| M17 | the band product in `i64` |
| M18 | the previous close written before the candle is validated |
| M19 | the state grows by a `[i128; 2]` field (256 bytes, still inside the `const` bound) |
| M20 | p99 read at rank 980 |
| M21 | a zero first p99 accepted (`base >= 0`) |
| M22 | `timed` drops one round |
| M23 | the synthetic stream without its drift |
| M24 | the budget doubled in `flat_within` |
| M25 | `measure_period` builds with `period.max(1)` |

| test | red under |
|---|---|
| `a_corrupt_candle_is_refused_and_changes_nothing` | M03 M18 |
| `a_cost_is_read_by_rank_and_judged_flat_or_not` | M20 M21 M24 |
| `a_gap_through_either_band_turns_on_the_gap_candle` | M03 M04 M05 M06 M09 M16 M18 |
| `a_time_gap_reads_as_contiguous_candles` | M16 |
| `a_turn_needs_a_close_beyond_the_current_band` | M05 M09 M11 M12 M13 M15 M16 M18 |
| `a_zero_period_and_a_factor_outside_its_range_are_refused` | M01 M02 |
| `an_all_up_and_an_all_down_series` | M03 M05 M09 M10 M11 M18 |
| `both_bands_ratchet_on_every_candle_and_reset_as_published` | M03 M09 M10 M14 M16 M18 |
| `fixture_st_1_the_seed_differs` | M03 M04 M05 M06 M09 M16 M18 |
| `fixture_st_2_a_flip_down_keeps_the_ratcheted_upper_band` | M03 M04 M05 M06 M09 M14 M16 M18 |
| `fixture_st_3_a_flip_up_keeps_the_ratcheted_lower_band_and_turns_down_first` | M03 M04 M05 M06 M09 M14 M16 M18 |
| `fixture_st_4_a_flat_series_reads_up_in_the_latch_and_down_here` | M03 M04 M05 M06 M11 M16 |
| `fixture_st_5_a_wide_candle_turns_the_two_band_form_first` | M03 M04 M05 M06 M09 M13 M16 M18 |
| `fixture_st_9_equal_bands_do_not_turn_an_up_candle` | M03 M04 M05 M06 M09 M10 M11 M12 M13 M16 M18 |
| `no_reading_depends_on_a_later_candle` | M15 |
| `nothing_is_read_before_the_atr_has_its_period` | M03 M04 M06 |
| `period_one_reads_the_published_zero_band_first` | M05 M06 M07 M08 M09 M13 M16 M18 |
| `the_atr_is_the_engine_atr` | M04 |
| `the_bench_measures_both_rows_at_tiny_sizes` | M01 M09 M18 M22 M23 M25 |
| `the_extremes_never_overflow` | M05 M08 M13 M15 M17 |
| `the_first_atr_candle_is_down_on_the_upper_band_and_unsettled` | M03 M04 M05 M06 M07 M08 M09 M15 M16 M18 |
| `the_state_is_a_fixed_size` | M19 |
| `the_stream_equals_a_from_scratch_transcription_of_the_published_rules` | M03 M04 M05 M06 M07 M08 M09 M10 M11 M12 M13 M14 M15 M16 M18 |

## 8. Benches (advisory here; the Mac's gate 8 run decides)

Two release runs, `cargo bench -p indicators --bench ratio --locked`, pinned stable, on the machine above, each the
only work running. The bench code is the same in both (`0fecf16c` changed only the tests file). Every row of both, the
crate's seven and this part's two, read `ok`; exit 0. Each timed step holds two `Instant::now()` reads.

Run 1, at `6ce14e70` (code `03f3005e`):

```
machine: 4 cores, Intel(R) Xeon(R) Processor @ 2.10GHz, 15 GiB
toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14)
tip: 6ce14e70dfb82cb3de3ce0af7684a1d0056f2ec2
command: cargo bench -p indicators --bench ratio --locked
Thu Nov  5 02:58:23 UTC 2026
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 573 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                           687573 ps = 1199.952 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in           673268 ps ->   673532 ps   ratio 1.000x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in          673268 ps ->   868197 ps   ratio 1.289x  ok
  C-I-02 step: a wandering candle -> a motionless one          862394 ps ->   571107 ps   ratio 0.662x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle   862394 ps ->   829090 ps   ratio 0.961x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  8287 ps over   2 iteration(s) = 4143 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          26700 ps over   5 iteration(s) = 5340 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             26709 ps over   5 iteration(s) = 5341 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         33287 ps over   6 iteration(s) = 5547 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover          850363 ps ->   722909 ps   ratio 0.850x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1085877 ps ->  1177292 ps   ratio 1.084x  ok
  the stable floor is 543 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1108688 ps = 2041.782 floors, budget 5200   ok
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 2000: best round p50 61 ns, p99 62 ns, max 3763 ns (n 10000); median round p99 64 ns; worst round p99 80 ns
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 20000: best round p50 61 ns, p99 64 ns, max 94 ns (n 10000); median round p99 66 ns; worst round p99 75 ns
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 200000: best round p50 61 ns, p99 64 ns, max 3386 ns (n 10000); median round p99 66 ns; worst round p99 67 ns
C-PSRB-STV-01 best-round p99 62 -> 64 -> 64 ns: within 3.000x of the first size and 2000 ns (machine floor 573 ps)   ok
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 10: best round p50 61 ns, p99 63 ns, max 13420 ns (n 10000); median round p99 65 ns; worst round p99 67 ns
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 100: best round p50 60 ns, p99 65 ns, max 10880 ns (n 10000); median round p99 69 ns; worst round p99 89 ns
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 1000: best round p50 61 ns, p99 65 ns, max 10263 ns (n 10000); median round p99 68 ns; worst round p99 78 ns
C-PSRB-STV-02 best-round p99 63 -> 65 -> 65 ns: within 3.000x of the first size and 2000 ns (machine floor 573 ps)   ok
all ratios within the ceiling
EXIT 0
Thu Nov  5 02:58:43 UTC 2026
```

Run 2, at `0fecf16c` (the code tip's Rust):

```
machine: 4 cores, Intel(R) Xeon(R) Processor @ 2.10GHz, 15 GiB
toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14)
tip: 0fecf16c610bd3df1bb95a5abde09c8e1367870a
command: cargo bench -p indicators --bench ratio --locked
Thu Nov  5 03:35:00 UTC 2026
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 451 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                           669083 ps = 1483.554 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in           699523 ps ->   817170 ps   ratio 1.168x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in          699523 ps ->   811900 ps   ratio 1.160x  ok
  C-I-02 step: a wandering candle -> a motionless one          839750 ps ->   561989 ps   ratio 0.669x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle   839750 ps ->   648119 ps   ratio 0.771x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  6509 ps over   2 iteration(s) = 3254 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          20956 ps over   5 iteration(s) = 4191 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             27692 ps over   5 iteration(s) = 5538 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         32098 ps over   6 iteration(s) = 5349 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover          777673 ps ->   686180 ps   ratio 0.882x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1379711 ps ->  1261133 ps   ratio 0.914x  ok
  the stable floor is 549 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1377228 ps = 2508.612 floors, budget 5200   ok
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 2000: best round p50 77 ns, p99 81 ns, max 16854 ns (n 10000); median round p99 85 ns; worst round p99 245 ns
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 20000: best round p50 78 ns, p99 82 ns, max 16717 ns (n 10000); median round p99 84 ns; worst round p99 91 ns
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 200000: best round p50 77 ns, p99 84 ns, max 19955 ns (n 10000); median round p99 86 ns; worst round p99 93 ns
C-PSRB-STV-01 best-round p99 81 -> 82 -> 84 ns: within 3.000x of the first size and 2000 ns (machine floor 451 ps)   ok
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 10: best round p50 77 ns, p99 82 ns, max 18593 ns (n 10000); median round p99 86 ns; worst round p99 88 ns
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 100: best round p50 77 ns, p99 81 ns, max 14807 ns (n 10000); median round p99 84 ns; worst round p99 89 ns
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 1000: best round p50 77 ns, p99 82 ns, max 145 ns (n 10000); median round p99 112 ns; worst round p99 175 ns
C-PSRB-STV-02 best-round p99 82 -> 81 -> 82 ns: within 3.000x of the first size and 2000 ns (machine floor 451 ps)   ok
all ratios within the ceiling
EXIT 0
Thu Nov  5 03:35:13 UTC 2026
```

The two coverage runs of section 6 also ran the bench, at opt-level 0 under instrumentation: their figures are
coverage evidence, not cost figures. At `0fecf16c`:

```
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 17948 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                         12484838 ps = 695.611 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in         12675047 ps -> 12705999 ps   ratio 1.002x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in        12675047 ps -> 12799255 ps   ratio 1.009x  ok
  C-I-02 step: a wandering candle -> a motionless one        12817503 ps ->  8550328 ps   ratio 0.667x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle 12817503 ps -> 13045617 ps   ratio 1.017x  ok 
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                 68721 ps over   2 iteration(s) = 34360 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)         125011 ps over   5 iteration(s) = 25002 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)            101198 ps over   5 iteration(s) = 20239 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)        115703 ps over   6 iteration(s) = 19283 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover        12424272 ps -> 10979949 ps   ratio 0.883x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars          28761901 ps -> 34006560 ps   ratio 1.182x  ok
  the stable floor is 17063 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask       30234094 ps = 1771.909 floors, budget 5200   ok
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 2000: best round p50 190 ns, p99 233 ns, max 11679 ns (n 10000); median round p99 241 ns; worst round p99 275 ns
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 20000: best round p50 192 ns, p99 273 ns, max 15116 ns (n 10000); median round p99 283 ns; worst round p99 290 ns
C-PSRB-STV-01 fold + reading(), period 10, factor 3,000, candles folded = 200000: best round p50 190 ns, p99 232 ns, max 11774 ns (n 10000); median round p99 312 ns; worst round p99 468 ns
C-PSRB-STV-01 best-round p99 233 -> 273 -> 232 ns: within 3.000x of the first size and 2000 ns (machine floor 17948 ps)   ok
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 10: best round p50 190 ns, p99 218 ns, max 13589 ns (n 10000); median round p99 231 ns; worst round p99 263 ns
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 100: best round p50 190 ns, p99 224 ns, max 11034 ns (n 10000); median round p99 275 ns; worst round p99 296 ns
C-PSRB-STV-02 fold + reading() after 20,000 candles, factor 3,000, ATR period = 1000: best round p50 190 ns, p99 233 ns, max 13465 ns (n 10000); median round p99 249 ns; worst round p99 283 ns
C-PSRB-STV-02 best-round p99 218 -> 224 -> 233 ns: within 3.000x of the first size and 2000 ns (machine floor 17948 ps)   ok
all ratios within the ceiling
```

## 9. Decisions drafted (this part's block only, docs/05 :86993-87132)

- D-8600: `indicators::dual_band_supertrend`, the standard two-band Supertrend, a second labelled reading beside the
  latch, which stays as it is (no vocabulary position; trend.rs, evaluator.rs and vocab untouched).
- D-8601: the ATR is `trend::Atr`, reused unchanged, and the band arithmetic is the latch's, so the readings differ in
  band logic alone; every band an `i128`, the bounds `const`-asserted.
- D-8602: the previous TREND picks the band judged, never a comparison of values; the script's zero prior bands and
  its first-candle direction kept; LITERAL-PINE named, not built.
- D-8603: the domain (period `u16` 1 to 65,535, factor 1 to 100,000 thousandths), refusals before anything moves,
  timestamps not read, no look-ahead.
- D-8604: `settled`, and that PSRB-STRAT is to decide only on settled readings.
- D-8605: C-PSRB-STV-01 and -02, the measuring functions in the module, the thin wrapper, the helpers copied per
  part.

docs/04 PSRB-STV-I01 to I06 and C-PSRB-STV-01, -02; docs/06 L-PSRB-STV-01 to 04; docs/07 one row. No other id used.
The bound in L-PSRB-STV-01 was re-derived for this step (each floor below real arithmetic by under 0.5, under `f`
and under 1 paisa respectively; the scaled ATR below the exact Wilder ATR by under `n` units of 10^-6 paisa by the
recurrence `e' = e (1 - 1/n) + d`), and its figures recomputed (3 x 10 / 10^6 = 0.00003 paisa; 100 x 65,535 / 10^6 =
6.55 paisa).

## 10. Not done, and facts to know

- Mutants: deferred (operator, 30 Oct 2026). The set file above carries the mutants lines for the background pass.
- TA-Lib's own Supertrend was not read, and no `TradingView` output was compared with these readings: parity here is
  parity with the published rules (L-PSRB-STV-03; the charter rows are PSRB-LAW's).
- Gate 14 was not run; gates 10, 11 and 12 are red on START itself (section 5), so their verdict on this part is the
  diff against START, which is empty of refusals.
- The fixtures' ATR columns, and ST-9's latch column, are the kit model's output, not PARTS.md's tables; the Rust
  agrees with every one of them.
- Nothing here read or wrote a store, an app, a shadow, vendor data, a credential or any private value. No scratch
  file entered the tree; the extracted kit scripts, logs, worktrees and target dirs are in the session's scratch folder.

## 11. NEEDS THE MAC (PARTS.md 5.7, 3.7)

1. Check 1 on the Mac's pin: Route B on the D2 set above against the seam commit `35234cead400`, and the bench wrapper
   and `fn main` by one instrumented bench run merged in (section 6 is this container's reading of the same).
2. Gate 8: `cargo bench -p indicators --bench ratio` with both C-PSRB-STV rows (the cloud figures are advisory).
3. Gates 10, 11, 12 and 14 on the Mac's base; gate 14's cover-table row for `C-PSRB-STV-01` and `C-PSRB-STV-02` is the
   main session's at the family assembly (`indicators crates/indicators/benches/ratio.rs 6 ... C-PSRB-STV-01
   C-PSRB-STV-02`; the measurement-point count stays 6, since the insertion adds no `ratio(` call).
4. macOS rows: none expected (no platform code). One to watch: `the_state_is_a_fixed_size` pins the measured 224
   bytes, which rests on `i128`'s 16-byte alignment; aarch64 is expected to agree, but that is not measured here, and
   a red there would be a macOS row.
5. The anchors on the then-current feat/db-fix (each insertion at its named place and nothing else in a shared file).
6. `git diff --exit-code <seam> <tip> -- crates/indicators/src/trend.rs crates/indicators/src/evaluator.rs
   crates/vocab`: empty here (section 5).

## 12. Appendix: the two scripts this step wrote and ran (from the scratch folder, not tracked elsewhere)

The bench coverage step of section 6 (`bench-cov.sh`), run after `d3-route-b-cloud.sh` with the same `D3_NIGHTLY`,
`D3_LLVM_BIN` and `D3_DIR` (the folder holding the kit's `d3-covset.mjs`). Its `rm -f` line targets build-time
profiles in its own new folder; this build runs no build script and no proc-macro (`bench-build.json` names none, and
neither `indicators` nor `vocab` has a `build.rs`), so it matched nothing, and the folder holds the run's one profile:

```
#!/bin/bash
# After d3-route-b-cloud.sh: runs the bench target `ratio` of indicators ONCE under the same
# instrumentation (RUSTFLAGS, opt-level 0, the same CARGO_TARGET_DIR and nightly), merges its profile
# with Route B's own indicators profiles, and reads every set file of the set with d3-covset.mjs.
# Usage: bench-cov.sh <S tree> <K> <P (Route B's profile folder)> <L new log folder> <set file>
set -uo pipefail
S=$1; K=$2; P=$3; L=$4; SET=$5
N=${D3_NIGHTLY:?}; T=${D3_LLVM_BIN:?}; D=${D3_DIR:?}
export RUSTC=$N/bin/rustc RUSTDOC=$N/bin/rustdoc PATH=$N/bin:$PATH
export CARGO_TARGET_DIR=$K/cov RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_PROFILE_DEV_OPT_LEVEL=0
mkdir -p "$L" "$P/bench" || exit 2
[ -z "$(ls -A "$P/bench")" ] || { echo "BENCH-COV REFUSED: $P/bench is not empty"; exit 2; }
cd "$S" || exit 2
ts() { TZ=Asia/Kolkata date '+%F %T IST'; }
echo "BENCH-COV START $(ts) tree $S tip $(git rev-parse HEAD)"
LLVM_PROFILE_FILE=$P/bench/build-%p-%m.profraw "$N/bin/cargo" test -p indicators --bench ratio --locked --no-run --message-format=json > "$L/bench-build.json" 2> "$L/bench-build.err" || { tail -5 "$L/bench-build.err"; echo "BENCH-COV NOT PROVED: build failed"; exit 3; }
rm -f "$P"/bench/build-*.profraw
jq -r 'select(.executable != null) | .executable' "$L/bench-build.json" | sort -u > "$P/bench-objects.txt"
echo "bench objects: $(cat "$P/bench-objects.txt")"
t0=$(date +%s)
LLVM_PROFILE_FILE=$P/bench/%p-%m.profraw "$N/bin/cargo" test -p indicators --bench ratio --locked > "$L/bench-run.log" 2>&1; rc=$?
echo "bench run rc=$rc ($(( $(date +%s) - t0 )) s); its printed lines are in $L/bench-run.log"
find "$P/bench" -name '*.profraw' > "$P/bench/profraw.list"; echo "bench profraw files: $(wc -l < "$P/bench/profraw.list")"
[ -s "$P/bench/profraw.list" ] || { echo "BENCH-COV NOT PROVED: the bench wrote no profile"; exit 3; }
cat "$P/indicators/profraw.list" "$P/bench/profraw.list" > "$P/all-profraw.list"
"$T/llvm-profdata" merge -sparse -f "$P/all-profraw.list" -o "$P/all.profdata" > "$L/merge-all.log" 2>&1 || { echo "BENCH-COV NOT PROVED: merge failed"; exit 3; }
cat "$P/objects.txt" "$P/bench-objects.txt" | sort -u > "$P/all-objects.txt"
OBJ=("$(head -1 "$P/all-objects.txt")"); while IFS= read -r o; do OBJ+=("--object=$o"); done < <(tail -n +2 "$P/all-objects.txt")
np=0
for f in $(awk '$1 == "coverage" {print $2}' "$SET" | sort -u); do
  b=$(printf '%s' "$f" | tr '/' '_')
  "$T/llvm-cov" show --show-branches=count --show-line-counts-or-regions --path-equivalence="$S/crates/indicators/src/..,$S/crates/indicators" --instr-profile="$P/all.profdata" "${OBJ[@]}" "$S/$f" > "$L/show-all-$b.txt" 2> "$L/show-all-$b.err"
  node "$D/d3-covset.mjs" "$SET" "$f" "$L/show-all-$b.txt" | tee "$L/figures-all-$b.txt"; [ "${PIPESTATUS[0]}" = 0 ] || { echo "NOT READ: $f"; np=1; }
done
echo "BENCH-COV DONE $(ts) not_proved=$np"
```

The local run of gates 10, 11 and 12 of section 5 (`gates.sh <tree> <tool dir>`), run on a worktree of START and on
the tip and the two logs diffed:

```
#!/bin/bash
# Runs CI gates 10, 11, 12 and 14 on one tree, with the step commands copied from
# .github/workflows/ci.yml; the tools are built from that tree's own .github/*.rs.
# Usage: gates.sh <tree> <tool dir>   (prints each gate's output and its exit status)
set -uo pipefail
TREE=$1; T=$2
mkdir -p "$T"
cd "$TREE" || exit 2
rustc --edition=2024 -D warnings .github/source_scan.rs -o "$T/source-scan" || exit 2
rustc --edition=2024 -D warnings .github/gates_ledger.rs -o "$T/gates-ledger" || exit 2
rustc --edition=2024 -D warnings .github/gates_bounds.rs -o "$T/gates-bounds" || exit 2
rustc --edition=2024 -D warnings .github/invariant_paths.rs -o "$T/invariant-paths" || exit 2
scan="$T/source-scan"; ledger="$T/gates-ledger"
export CARGO_BIN_EXE_source_scan="$scan"

echo "===== GATE 10"
( set -euo pipefail
  work=$(mktemp -d -p "$T")
  git ls-files -z -- 'crates/*.rs' '.github/*.rs' | xargs -0 -r "$scan" fns | "$ledger" path-declarations > "$work/path-fns"
  git ls-files -z > "$work/tracked.z"
  "$scan" modules "$work/tracked.z" | "$ledger" module-table > "$work/modules"
  git ls-files -z -- ':(glob)web/tests/*.test.js' | xargs -0 -r "$T/invariant-paths" --web-tests > "$work/web-tests"
  [ -s "$work/web-tests" ] || { echo "GATE 10 READ NO FRONT-END TEST."; exit 1; }
  "$T/invariant-paths" docs/04-invariants.md "$work/path-fns" "$work/modules" "$work/web-tests"
  git ls-files -z -- 'crates/*.rs' '.github/*.rs' | xargs -0 -r "$scan" inline-mods > "$work/inline"
  [ -s "$work/inline" ] || { echo "GATE 10 READ NO INLINE MODULE."; exit 1; }
  "$T/invariant-paths" --middles docs/04-invariants.md "$work/path-fns" "$work/modules" "$work/inline" > "$work/middles"
  "$ledger" invariant-tests "$work/path-fns" "$work/middles"
); echo "GATE 10 EXIT $?"

echo "===== GATE 11"
( set -euo pipefail
  work=$(mktemp -d -p "$T")
  git ls-files -z > "$work/tracked.z"
  st=0; "$scan" prod-files "$work/tracked.z" > "$work/prod-files" || st=$?
  "$ledger" banned-constructs "$work/prod-files" "$st"
); echo "GATE 11 EXIT $?"

echo "===== GATE 12"
( set -euo pipefail
  allow_claim='
          crates/cli/src/lib.rs base_win_rate_bp
          crates/costs/src/trip.rs charge_stack_legs
          '
  work=$(mktemp -d -p "$T")
  git ls-files -z -- 'crates/*.rs' > "$work/sources.z"
  xargs -0 -r "$scan" fns < "$work/sources.z" \
    | sed -E 's/^(.*):[0-9]+:([A-Za-z_][A-Za-z0-9_]*)$/\1\t\2/' > "$work/path-fns"
  git ls-files -z > "$work/tracked.z"
  "$scan" modules "$work/tracked.z" > "$work/modules"
  xargs -0 -r "$scan" inline-mods < "$work/sources.z" > "$work/inline"
  "$T/gates-bounds" gate12-tokens \
    | "$T/invariant-paths" --resolve "$work/path-fns" "$work/modules" "$work/inline" \
    > "$work/resolved"
  echo "  $(wc -l < "$work/resolved") (token, crate fn) resolution(s)"
  "$T/gates-bounds" gate12 "$allow_claim" "$work/resolved"
); echo "GATE 12 EXIT $?"
```

STATUS: DONE

# PSRB-RSI cloud report (token PSRB-RSI-20261103-1)

Branch `cloud/psrb-rsi-20261103`, cut from `origin/cloud/psrb-kit-20261103` (`56f3043dd4ca`).
START = `git rev-parse origin/cloud/psrb-kit-20261103~1` = `35234cead4009299a93fcd79a3d1fffd773dcc16` (PSRB-SEAM, parent
`1ad6e24ef8e0`). Code tip (`git rev-list -1 HEAD -- . ':(exclude)reports'`) = `947dbd3ad97854784ce3e895318b478a8a51354b`.
Only items for the Mac are left (NEEDS THE MAC, at the end). The short form for the Mac's check is `part-PSRB-RSI.md`;
the machine-read summary is `result-PSRB-RSI.json.md`.

Commits on the branch, oldest first: `23a378ec` OWNER file; `f6ff984a` this report IN PROGRESS with the kit check and
the START baseline; `c8021a70` the code (module, tests, bench wrapper, `lib.rs` line, two slot insertions); `f8096504`
the four ledger blocks and two test asserts split; `e47cf2df` the bench wrapper names a refusal on its printed line;
`947dbd3a` L-PSRB-RSI-02 cites both bench runs; then this report, the part file and the result file (`reports/` only).
Every push was preceded by `git fetch origin cloud/psrb-rsi-20261103`, whose OWNER file held this token each time; no
push was refused, nothing was forced, merged or rebased.

## Kit check

- `git diff --name-only $START origin/cloud/psrb-kit-20261103` lists only the 13 files under `reports/psrb-cloud-20261103/`.
- `sha256sum MANIFEST.md` = `85b873ebc2a7cc86713701cbc7619bb3b14058b99225e4249fdbfb403c889ad8`, the figure in the kit
  commit's subject.
- Every MANIFEST row matches: `CLOUD.md` `19dff357...`, `PARTS.md` `1a96fc9b...` (whole files); `d3-recipe.txt`
  `9ed425e3...`, `d3-route-b.sh` `f3d70ac2...`, `d3-route-b-cloud.sh` `f032b525...`, `d3-covset.mjs` `ecef3bdc...`,
  `sim.mjs` `c78e72b8...`, `fixtures.mjs` `e4818033...`, `fix2.mjs` `d03d3ceb...`, `rsi-run.mjs` `a99a83ad...`,
  `min-run.mjs` `3c18cb3c...`, `faithful-edge.mjs` `2ed69004...` (each extracted with MANIFEST.md's own `awk` line).
- `rsi-run.mjs`, `min-run.mjs` and `faithful-edge.mjs`, run with node from their extracted copies, print every figure
  PARTS.md 3.3 and 3.5 cite (RSI-1's five rows; the drift runs at periods 2, 5 and 14; the period-14 faithful edge at the
  21st and 22nd unchanged close); the Rust tests pin the same figures independently (below).

## START baseline (before the first code commit)

The code tree at the OWNER commit equals START outside `reports/` (`git diff --stat $START HEAD -- . ':(exclude)reports'`
printed nothing). On the pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo test -p indicators --locked`: 22 test
binaries, 504 passed, 0 failed, 0 ignored, exit 0 (02:30:54-02:31:09 UTC, 5 Nov 2026). This host has no indicators red.

## Toolchains and machine

- Stable: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)` (rust-toolchain.toml's pin).
- Coverage nightly: installed as `nightly-2026-08-08`; `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13
  2026-08-07)`; its own `llvm-cov --version` and `llvm-profdata --version` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
  No `RUSTC_BOOTSTRAP`.
- Machine: Linux 6.18.44 x86_64, KVM guest, 4 vCPU `Intel(R) Xeon(R) Processor @ 2.30GHz`, 15 GiB memory; TMPDIR on a
  tmpfs (`/dev/shm`); one CARGO_TARGET_DIR per toolchain (stable `target/`, nightly the scratch deep dir).

## What was built (PARTS.md 3.1-3.6, nothing else)

`indicators::rsi`: Wilder's RSI over closes as an exact-integer streaming reading with a runtime `u16` period and a warm
flag, in the published `TradingView` form (UNVERIFIED against their own output, as the module says). The first candle
records its close; each later one is one change; the reading appears after exactly `period` changes. Both averages are
held scaled by `trend::SCALE` in `i128`, seeded by the floored mean and stepped by the floored Wilder step, the two
accumulators holding the seed sums until the seed. The change counter is a `u32` saturating at `u32::MAX`. `ppm` is the
floor (1,000,000 when the average loss is 0, both averages zero included); `above` and `below` cross-multiply on the
state; `flat()` says both averages are 0; `faithful()` is `G + L >= 1,000 period`. Every bound of 3.3 is a
`const _: () = assert!(...)` taken with `checked_mul`. Refusals: `PeriodZero`, `LevelAboveMax { ppm }`, `Corrupt(why)`,
each before anything moves. The measuring half of the two bench rows lives in the module; the wrapper sorts, prints and
judges.

### Files and functions

| file | lines | what |
|---|---|---|
| `crates/indicators/src/rsi.rs` | 497 (new) | `MAX_LEVEL_PPM`, `FAITHFUL_FACTOR`; `Refusal`; `Level::{from_ppm, ppm}`; `Rsi::{new, period, fold, warm, reading}`; `Reading::{period, avg_gain_scaled, avg_loss_scaled, flat, faithful, ppm, above, below}`; `StepCost`, `summarize`, `flat_within`, `measure_step`, `measure_period`; private `PPM`, `MOVE_LIMIT`, `UNIT_LIMIT`, `PERIOD_LIMIT`, `GAIN_LIMIT`, `STEP_PERIOD`, `PERIOD_HISTORY`, `nearest_rank`, `measure`, `bench_candle`; seven const assertions |
| `crates/indicators/src/rsi_tests.rs` | 942 (new) | the 27 tests; helpers `Xorshift`, `candle`, `rsi`, `level`, `stream`, `last`, `averages`, `walk`, `recompute` (oracle 1), `exact_averages` (oracle 2), `Run`, `long_run`; constants `MINUTE`, `RSI_1`, `RSI_1_TABLE`, `DRIFT` |
| `crates/indicators/benches/ratio/psrb_rsi.rs` | 132 (new) | `pub(crate) fn row(floor, ids)`, `one_step_however_long_the_history` (C-PSRB-RSI-01), `one_step_whatever_the_period` (C-PSRB-RSI-02), `best_round`, `verdict`; `SAMPLES` 20,000, `ROUNDS` 9, `BUDGET_NS` 2,000, `HISTORIES`, `PERIODS` |
| `crates/indicators/src/lib.rs` | +1 | `pub mod rsi;` after `pub mod pattern;` |
| `crates/indicators/benches/ratio.rs` | +7 | `#[path = "ratio/psrb_rsi.rs"]` and `mod psrb_rsi;` under `// slot bench-mod PSRB-RSI`; the re-binding statement under `    // slot bench-row PSRB-RSI`, as rustfmt wraps it |
| `docs/04-invariants.md` | +13 | `## Wilder RSI for the PSRB family (PSRB-RSI; D-8580 to D-8584)`: PSRB-RSI-I01..I06, C-PSRB-RSI-01, -02 |
| `docs/05-decisions.md` | +170 | D-8580..D-8584 |
| `docs/06-limits.md` | +8 | `## Wilder RSI for the PSRB family: limits (PSRB-RSI; D-8580 to D-8584)`: L-PSRB-RSI-01..03 |
| `docs/07-plan.md` | +6 | `## PSRB-RSI: Wilder RSI — 2026-11-03 (D-8580 to D-8584)`, one row |

`git diff --stat $START 947dbd3a -- . ':(exclude)reports'`: 9 files, 1,776 insertions, 0 deletions. Each shared-file hunk
is an insertion only (`@@ -a,0 +b,n @@`) after exactly the anchor line PARTS.md 2.2 names: `lib.rs` after `pub mod
pattern;` (:70), `ratio.rs` after the two PSRB-RSI slots (:68, :781), docs/04 after :8950, docs/05 after :84675, docs/06
after :21954, docs/07 after :982. No `trend.rs`, `vwap.rs`, `evaluator.rs`, `daily.rs`, `tests/`, `Cargo.toml`,
`Cargo.lock`, `CLAUDE.md`, `AGENTS.md`, `docs/00-charter.md` or `.github/` line changed.

### Against PARTS.md 3.2 (signatures)

Every name, argument and return type is as shown, and every `const fn` shown is `const` (none dropped). Added beyond the
shown text: `StepCost` derives `Clone, Copy, Debug, Default, PartialEq, Eq` (3.2 shows only its fields); the private
items listed above. `size_of::<Rsi>()` is 64 bytes and `size_of::<Reading>()` 48 on x86_64 (a layout probe of the same
field types with rustc 1.97.1: `Rsi 64 bytes, Reading 48 bytes, i128 align 16`); the build asserts `Rsi` at most 96
bytes and `the_state_is_a_fixed_size_and_never_allocates` asserts it again with `Reading` at most 48.

## Tests by name (27, `crates/indicators/src/rsi_tests.rs`; every name of PARTS.md 3.5)

1. `a_zero_period_is_refused_and_every_other_period_is_accepted` (0; 1; 14; `u16::MAX`).
2. `a_level_above_one_hundred_is_refused_at_the_edge` (0 and 1,000,000 accepted and read back; 1,000,001 and `u32::MAX`
   refused naming the value).
3. `the_first_close_folds_no_change_and_reads_nothing` (the whole state compared).
4. `the_reading_appears_after_exactly_period_changes` (periods 1, 2, 3, 14, `u16::MAX`: none after `period` closes, one
   after `period + 1`, equal to the recomputation).
5. `the_largest_period_smooths_after_its_seed` (period 65,535, 65,539 closes of a walk that rises and falls; G and L after
   each of the three Wilder steps equal the from-scratch loop; white-box: `changes = u32::MAX - 1` at period 14 folds two
   closes exactly as its true count does and reads `u32::MAX` after each).
6. `the_seed_is_the_floored_mean_of_the_first_period_moves` (three hand cases with remainders; 20 random seeds at each
   period 1 to 30, the sums taken in paisa and scaled after).
7. `wilder_smoothing_matches_fixture_rsi_1` (the table, row for row: G, L, `ppm`).
8. `a_flat_series_reads_one_hundred_and_says_it_is_flat` (100 closes of 2,500,000 at periods 1, 2, 14).
9. `an_all_up_series_reads_one_hundred`; 10. `an_all_down_series_reads_zero` (`above` at 999,999 and 1,000,000, `below`
   at 0 and 1).
11. `equal_moves_at_the_seed_read_fifty` (period 2, closes 100, 101, 100: 500,000 exactly, neither above nor below it).
12. `a_period_of_one_reads_the_last_change_alone` (up, down, unchanged, up).
13. `a_tie_at_the_level_is_neither_above_nor_below` (RSI-1 at 107: G is exactly 3 L).
14. `a_floor_ppm_equal_to_the_level_is_not_a_tie` (RSI-1's first two readings, with the digits 75.000018750004687 and
    75.000014062506152 pinned as integers).
15. `ppm_is_the_floor_of_the_exact_ratio` (20,000 states built over the whole range plus streamed ones).
16. `a_price_gap_is_one_move_and_nothing_more` (a 10% jump and a 10% fall at periods 2 and 14: one Wilder step each, and
    the stream equals the recomputation throughout).
17. `a_time_gap_reads_as_contiguous_candles` (a minute apart, a day apart, random gaps, receding: equal states).
18. `no_reading_depends_on_a_later_candle` (every candle after t redrawn: the state at t unchanged).
19. `the_stream_equals_a_from_scratch_recomputation` (BRUTE FORCE: 1,000 random series at periods 1 to 30, every candle;
    more than 10,000 warm readings compared).
20. `the_fixed_point_stays_within_period_units_of_exact_wilder` (PARITY: the published formulas in exact rationals,
    `0 <= SCALE g - G < n` for both averages at every candle, cross-multiplied; more than 5,000 readings; RSI-1's `ppm`
    column is the floor of its exact RSI).
21. `a_long_unchanged_run_departs_only_where_the_ratio_bound_allows` (the ratio bound and the guard at every candle of four
    runs; every figure of 3.5 pinned: exact floors 428,572, 547,023, 593,601; unfaithful first after 11, 28, 70; departures
    after 20 (G 0, L 1, `ppm` 0), 58 (G 1, L 0, `ppm` 1,000,000), 159 (G 5, L 0, `ppm` 1,000,000), each `flat()` and
    `faithful()` false; both averages 0 first after 21, 59, 164; the period-2 run near 100: exact floor 999,900, departure
    after 19 (G 9,536, L 0, `ppm` 1,000,000, `faithful()` TRUE, `flat()` false), unfaithful first after 22, both zero
    after 33).
22. `the_faithful_edge_is_one_thousand_units_per_period` (white-box readings at periods 1, 14, 65,535: `1,000 n - 1` and
    `1,000 n` as all gain, all loss and split; through `fold` at 14: G 71,428, faithful through the 21st unchanged close
    and unfaithful from the 22nd).
23. `the_largest_moves_and_period_never_overflow` (closes alternating 1 and `i64::MAX - 1` at periods 1 and 65,535; the
    const bounds restated at runtime).
24. `a_corrupt_candle_is_refused_and_changes_nothing` (all five `Corrupt` kinds `check_evaluable` gives, at four states,
    whole state compared).
25. `the_state_is_a_fixed_size_and_never_allocates`.
26. `the_bench_measures_both_rows_at_tiny_sizes`; 27. `a_cost_is_read_by_rank_and_judged_flat_or_not` (including a zero
    first p99 failing, a zero later p99 failing and nothing measured failing).

## Gates (pinned stable 1.97.1, at the code tip `947dbd3a`, 03:20:51-03:21:05 UTC)

| command | result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy -p indicators --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test -p indicators --locked` | exit 0: 22 binaries, 531 passed, 0 failed, 0 ignored (START 504 + the 27 new) |
| `cargo test -p core --locked --test cost_invariants --test decision_citations` | exit 0: `cost_invariants` 5 passed, `decision_citations` 4 passed |

No host red: every command above is green here, so none needed the START or non-root proof.

## Each new test shown red under a hand change (CLOUD.md rule 6)

A node script applied 24 hand changes to `crates/indicators/src/rsi.rs` one at a time in a worktree of this session's own
at `f80965045f8b` (whose `crates/indicators/src` is byte-identical to the tip's), ran `cargo test -p indicators --locked
--lib rsi::` on the pinned stable for each, recorded the failed tests, and restored the file; it printed `RESTORED byte
for byte`, `git status --porcelain` of that worktree was empty afterwards, and `TESTS 27, shown red 27`.

| # | hand change of `rsi.rs` | tests red |
|---|---|---|
| 1 | `Rsi::new` accepts period 0 | `a_zero_period_is_refused_and_every_other_period_is_accepted`, `the_bench_measures_both_rows_at_tiny_sizes` |
| 2 | `Level::from_ppm` refuses 1,000,000 (`>=`) | `a_level_above_one_hundred_is_refused_at_the_edge`, `a_flat_series_reads_one_hundred_and_says_it_is_flat`, `an_all_up_series_reads_one_hundred`, `an_all_down_series_reads_zero` |
| 3 | the first close counts a zero change | 13, among them `the_first_close_folds_no_change_and_reads_nothing`, `the_reading_appears_after_exactly_period_changes`, `wilder_smoothing_matches_fixture_rsi_1` |
| 4 | a `u16`-like counter (saturates at 65,535) | `the_largest_period_smooths_after_its_seed` only |
| 5 | the seed rounds up | 6, among them `the_seed_is_the_floored_mean_of_the_first_period_moves`, `the_fixed_point_stays_within_period_units_of_exact_wilder` |
| 6 | the Wilder step truncates toward zero (`/`) | 7, among them `wilder_smoothing_matches_fixture_rsi_1`, `the_stream_equals_a_from_scratch_recomputation`, `a_long_unchanged_run_departs_only_where_the_ratio_bound_allows` |
| 7 | warm one change late (`>`) | 11, among them `the_reading_appears_after_exactly_period_changes`, `the_faithful_edge_is_one_thousand_units_per_period` |
| 8 | `flat()` reads either average zero (`||`) | `a_period_of_one_reads_the_last_change_alone`, `an_all_up_series_reads_one_hundred`, `an_all_down_series_reads_zero`, `a_long_unchanged_run_departs_only_where_the_ratio_bound_allows` |
| 9 | `faithful()` strict (`>`) | `the_faithful_edge_is_one_thousand_units_per_period` |
| 10 | `ppm` reads 999,999 when L is 0 | `a_flat_series_reads_one_hundred_and_says_it_is_flat`, `a_period_of_one_reads_the_last_change_alone`, `an_all_up_series_reads_one_hundred`, `ppm_is_the_floor_of_the_exact_ratio`, `a_long_unchanged_run_departs_only_where_the_ratio_bound_allows` |
| 11 | `ppm` rounds up | `ppm_is_the_floor_of_the_exact_ratio`, `a_floor_ppm_equal_to_the_level_is_not_a_tie`, `wilder_smoothing_matches_fixture_rsi_1` |
| 12 | both averages zero read above 1,000,000 | `a_flat_series_reads_one_hundred_and_says_it_is_flat` |
| 13 | `above` counts a tie (`>=`) | `a_tie_at_the_level_is_neither_above_nor_below`, `equal_moves_at_the_seed_read_fifty`, `an_all_up_series_reads_one_hundred` |
| 14 | `above` compares the floored `ppm` | `a_floor_ppm_equal_to_the_level_is_not_a_tie` |
| 15 | `below` counts a tie (`<=`) | `a_tie_at_the_level_is_neither_above_nor_below`, `a_flat_series_reads_one_hundred_and_says_it_is_flat`, `an_all_down_series_reads_zero`, `equal_moves_at_the_seed_read_fifty` |
| 16 | a refused candle still replaces the previous close | `a_corrupt_candle_is_refused_and_changes_nothing` |
| 17 | the gain reads the timestamp | 16, among them `a_time_gap_reads_as_contiguous_candles` |
| 18 | the fold reads a counter shared with later folds (a static; run with `--test-threads=1`) | 10, among them `no_reading_depends_on_a_later_candle`, `a_time_gap_reads_as_contiguous_candles` |
| 19 | a move is capped at 5,000 paisa | `a_price_gap_is_one_move_and_nothing_more`, `the_stream_equals_a_from_scratch_recomputation`, `a_long_unchanged_run_departs_only_where_the_ratio_bound_allows` |
| 20 | the scaled gain is held in an `i64` and wraps | `the_largest_moves_and_period_never_overflow` |
| 21 | `#[repr(align(128))]` on `Rsi`, its const bound relaxed with it | `the_state_is_a_fixed_size_and_never_allocates` |
| 22 | a round times one sample too many | `the_bench_measures_both_rows_at_tiny_sizes` |
| 23 | p99 read at rank 999 | `a_cost_is_read_by_rank_and_judged_flat_or_not` |
| 24 | a zero p99 passes the verdict | `a_cost_is_read_by_rank_and_judged_flat_or_not` |

Hand change 18 stands in for look-ahead: a streaming `fold` is never given a later candle, so the change that lets a
reading at t depend on anything after t is shared state that later folds update; the test turns red on it.

## Coverage: Route B on the pinned nightly

**Set** (recipe step 2 against START; reading `narrow`: START's first-parent history holds no "Merge BASE-COV-" commit;
sha256 `2fc49e30...`):

```
reading narrow
coverage crates/indicators/src/rsi.rs whole
coverage crates/indicators/src/rsi_tests.rs whole
coverage crates/indicators/benches/ratio/psrb_rsi.rs whole
coverage crates/indicators/src/lib.rs line 71
coverage crates/indicators/benches/ratio.rs line 69
coverage crates/indicators/benches/ratio.rs line 70
coverage crates/indicators/benches/ratio.rs main 758-821
mutants crates/indicators/src/rsi.rs whole
mutants crates/indicators/src/rsi_tests.rs whole
mutants crates/indicators/benches/ratio/psrb_rsi.rs whole
mutants crates/indicators/src/lib.rs line 71
mutants crates/indicators/benches/ratio.rs line 69
mutants crates/indicators/benches/ratio.rs line 70
mutants crates/indicators/benches/ratio.rs main 758-821
```

**Commands.** In a worktree of this session's own detached at `947dbd3a` (clean), deep dir and profile folder new:
`D3_W=<scratch> D3_DIR=<scratch>/kit D3_NIGHTLY=~/.rustup/toolchains/nightly-2026-08-08-x86_64-unknown-linux-gnu
D3_LLVM_BIN=$D3_NIGHTLY/lib/rustlib/x86_64-unknown-linux-gnu/bin D3_CARGO=$D3_NIGHTLY/bin/cargo bash d3-route-b-cloud.sh
<tree> <K> <P> <L> <set file> indicators` (the kit's script, unmodified), then `d3-bench-cov.sh <tree> <K> <P> <L2>
<set file> indicators ratio` (below), same pins and environment. Route B: pins as above; build 08:41:01-08:41:04 IST,
23 artifacts, 21 objects; `cargo test -p indicators` 08:41:04-08:46:39 IST, 531 passed, 0 failed, 21 profiles. Its
exit was 3 only because the two bench files are in no test object (`benches/ratio.rs` is `test = false`): "COVERAGE NOT
READ ... NOT PROVED" for those two, by design. The bench step built the bench target in the same test profile
(opt-level 0), ran it once 08:46:39-08:50:30 IST (exit 0), merged its profile with the 21 test profiles (22) and read
every set file again with the tests' objects and the bench executable: exit 0, every set file read.

**Figures** (d3-covset.mjs, the merged reading at `947dbd3a`):

| file | item | lines | branches |
|---|---|---|---|
| `crates/indicators/src/rsi.rs` | whole | 153/153 | 22/22 |
| `crates/indicators/src/rsi_tests.rs` | whole | 653/653 | 26/26 |
| `crates/indicators/benches/ratio/psrb_rsi.rs` | whole | 70/70 | 0/0 |
| `crates/indicators/src/lib.rs` | line 71 | 0/0 (no region) | 0/0 |
| `crates/indicators/benches/ratio.rs` | line 69, line 70 | 0/0, 0/0 (no region) | 0/0 |
| `crates/indicators/benches/ratio.rs` | `main` 758-821 | 24/26 | 1/2 |
| the set | | 900/902 | 49/50 |

The tests-only reading of Route B gives the same figures for `rsi.rs`, `rsi_tests.rs` and `lib.rs`.

**(F) items, the only uncovered lines and sides** (written per file, recipe step 4; no GAP, no (U)):

- `crates/indicators/benches/ratio.rs:816:8` branch True, `if verdicts.contains(&false) {`; `:817`
  `println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");`; `:818` `std::process::exit(1);` — `main`'s
  own failing arm, which a run reaches only when one of the file's own rows fails (CC-20's kind). These are base lines of
  `main`, in the set only because `main` holds this part's statement.

Two earlier readings, for the record: at `c8021a70` `rsi_tests.rs` read 651/651 lines and 30/34 branches, the four
missed sides the False arms of `&&` inside two `assert!`s; `f8096504` split each into two asserts (26/26). At
`f8096504` `psrb_rsi.rs` read 69/72, three lines of a refusal-printing closure no passing run reaches; `e47cf2df` prints
the refusal on every size's line instead (70/70).

The bench half of Route B, `d3-bench-cov.sh` (written in this session; not a kit file; sha256 `31ddc848...`):

```bash
#!/bin/bash
# d3-bench-cov.sh (PSRB-RSI cloud session, 5 Nov 2026; shell, jq and node only, no Py-language): the bench half of
# Route B for a part whose D2 set holds bench files (PARTS.md 2.4: "The bench wrapper and `main` are measured by running
# the bench target once under the same instrumentation and merging its profile"). Not a kit file; written beside the
# kit's d3-route-b-cloud.sh and run after it on the same tree, deep dir and profile folder:
#   1. the same pins (rustc -V, llvm-cov and llvm-profdata --version) and the same environment
#      (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', CARGO_PROFILE_DEV_OPT_LEVEL=0,
#      CARGO_TARGET_DIR=<K>/cov, RUSTC and RUSTDOC the nightly's);
#   2. `cargo test -p <pkg> --locked --bench <bench> --no-run --message-format=json` builds the bench target in the test
#      profile (opt-level 0, the profile the tests ran in); its executable is the one THIS build reports;
#   3. that executable runs ONCE with LLVM_PROFILE_FILE in a NEW folder <P>/bench-<bench>;
#   4. llvm-profdata merge -sparse of the tests' profiles (<P>/<pkg>/profraw.list) and the bench's;
#   5. llvm-cov show of every set file of the package with the tests' objects and the bench executable, read by
#      d3-covset.mjs.
# Deletes nothing. Exit 0: the bench ran (whatever its verdict) and every set file was read; 3: NOT PROVED.
# Usage: d3-bench-cov.sh <S> <K> <P of the Route B run> <L new log folder> <set file> <pkg> <bench>
set -o pipefail
N=${D3_NIGHTLY:?}; T=${D3_LLVM_BIN:?}; CARGO_Q=${D3_CARGO:?}; D=${D3_DIR:?}
PIN_RUSTC='rustc 1.99.0-nightly (1a98b1e13 2026-08-07)'; PIN_LLVM='LLVM version 23.1.0-rust-1.99.0-nightly'
S=$1; K=$2; P=$3; L=$4; SET=$5; C=$6; B=$7
[ -n "$B" ] || { echo "usage: d3-bench-cov.sh <S> <K> <P> <L> <set file> <pkg> <bench>"; exit 2; }
ts() { TZ=Asia/Kolkata date '+%F %T IST'; }
np=0; notp() { echo "BENCH-COV NOT PROVED: $*"; np=1; }
[ -s "$P/$C/profraw.list" ] || { echo "BENCH-COV REFUSED: $P/$C/profraw.list is missing (run d3-route-b-cloud.sh first)"; exit 2; }
[ -s "$P/objects.txt" ] || { echo "BENCH-COV REFUSED: $P/objects.txt is missing"; exit 2; }
PB=$P/bench-$B
if [ -e "$PB" ] && [ "$(ls -A "$PB" | wc -l | tr -d ' ')" != 0 ]; then echo "BENCH-COV REFUSED: $PB is not empty"; exit 2; fi
mkdir -p "$PB" "$L" || exit 2
echo "BENCH-COV START $(ts) tree $S tip $(git -C "$S" rev-parse HEAD) package $C bench $B"
rv=$("$N/bin/rustc" -V 2>&1); cv=$("$T/llvm-cov" --version 2>&1 | grep 'LLVM version'); pv=$("$T/llvm-profdata" --version 2>&1 | grep 'LLVM version')
echo "versions: $rv | llvm-cov ${cv# } | llvm-profdata ${pv# }"
[ "$rv" = "$PIN_RUSTC" ] || { echo "BENCH-COV ENVIRONMENT: rustc is '$rv', pin '$PIN_RUSTC'"; exit 3; }
case "$cv" in *"$PIN_LLVM"*) ;; *) echo "BENCH-COV ENVIRONMENT: llvm-cov '$cv'"; exit 3;; esac
case "$pv" in *"$PIN_LLVM"*) ;; *) echo "BENCH-COV ENVIRONMENT: llvm-profdata '$pv'"; exit 3;; esac
export PATH=$N/bin:$PATH RUSTC=$N/bin/rustc RUSTDOC=$N/bin/rustdoc
export CARGO_TARGET_DIR=$K/cov RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_PROFILE_DEV_OPT_LEVEL=0
cd "$S" || exit 2
t0=$(date +%s); echo "== bench build START $(ts): cargo test -p $C --locked --bench $B --no-run --message-format=json"
mkdir -p "$P/bench-build-$B" || exit 2
LLVM_PROFILE_FILE=$P/bench-build-$B/%p-%m.profraw "$CARGO_Q" test -p "$C" --locked --bench "$B" --no-run --message-format=json > "$L/bench-build.json" 2> "$L/bench-build.err"; rc=$?
echo "== bench build rc=$rc END $(ts) ($(( ($(date +%s) - t0) / 60 )) min)"
[ $rc = 0 ] || { tail -5 "$L/bench-build.err"; echo "BENCH-COV NOT PROVED: the instrumented bench build failed"; exit 3; }
EXE=$(jq -r --arg b "$B" 'select(.executable != null and .target.name == $b and (.target.kind | index("bench"))) | .executable' "$L/bench-build.json" | sort -u)
[ "$(printf '%s\n' "$EXE" | grep -c .)" = 1 ] || { echo "BENCH-COV NOT PROVED: the build reported '$EXE' as the bench executable"; exit 3; }
echo "bench executable: $EXE (from this build's own report)"
t0=$(date +%s); echo "== bench run START $(ts): LLVM_PROFILE_FILE=$PB/%p-%m.profraw $EXE"
LLVM_PROFILE_FILE=$PB/%p-%m.profraw "$EXE" > "$L/bench-run.log" 2>&1; brc=$?
echo "== bench run rc=$brc END $(ts) ($(( ($(date +%s) - t0) / 60 )) min) (rc 1 is a breach under instrumentation: a verdict, not a coverage matter)"
find "$PB" -name '*.profraw' > "$PB/profraw.list"; nb=$(wc -l < "$PB/profraw.list" | tr -d ' '); echo "bench profraw files: $nb"
[ "$nb" -gt 0 ] || { echo "BENCH-COV NOT PROVED: the bench wrote no profile"; exit 3; }
cat "$P/$C/profraw.list" "$PB/profraw.list" > "$PB/all.list"
"$T/llvm-profdata" merge -sparse -f "$PB/all.list" -o "$PB/merged.profdata" > "$L/merge-all.log" 2>&1 || { echo "BENCH-COV NOT PROVED: merge failed ($L/merge-all.log)"; exit 3; }
echo "merged $(wc -l < "$PB/all.list" | tr -d ' ') profiles (tests and bench) into $PB/merged.profdata"
OBJ=("$(head -1 "$P/objects.txt")"); while IFS= read -r o; do OBJ+=("--object=$o"); done < <(tail -n +2 "$P/objects.txt"); OBJ+=("--object=$EXE")
files=$(awk -v c="crates/$C/" '$1 == "coverage" && index($2, c) == 1 {print $2}' "$SET" | sort -u)
for f in $files; do
  b=$(printf '%s' "$f" | tr '/' '_')
  "$T/llvm-cov" show --show-branches=count --show-line-counts-or-regions --path-equivalence="$S/crates/$C/src/..,$S/crates/$C" --instr-profile="$PB/merged.profdata" "${OBJ[@]}" "$S/$f" > "$L/show-all-$b.txt" 2> "$L/show-all-$b.err"
  node "$D/d3-covset.mjs" "$SET" "$f" "$L/show-all-$b.txt" | tee "$L/figures-all-$b.txt"; [ "${PIPESTATUS[0]}" = 0 ] || notp "$f: not read ($L/show-all-$b.err)"
done
if [ $np = 0 ]; then echo "BENCH-COV DONE $(ts): every set file of $C read with the tests' and the bench's profiles"; exit 0; fi
echo "BENCH-COV DONE $(ts) WITH NOT PROVED lines above"; exit 3
```

## Benches (advisory; the Mac's gate 8 run decides)

Machine as above (KVM guest, 4 vCPU Xeon @ 2.30 GHz, rustc 1.97.1), command `cargo bench -p indicators --bench ratio
--locked` (the bench profile: release, `lto = "fat"`). Each PSRB-RSI sample holds two clock reads, so its figures are
upper bounds on the step. Both runs exit 0, every row of the file within its ceiling and budget.

Run 1, 02:58:44 UTC, HEAD `c8021a70` (the wrapper's earlier form, which printed a refusal in a closure instead of on the
line; the timed code is the same), load average 0.84:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 462 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                           671514 ps = 1453.493 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in           680964 ps ->   684693 ps   ratio 1.005x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in          680964 ps ->   667226 ps   ratio 0.979x  ok
  C-I-02 step: a wandering candle -> a motionless one          668244 ps ->   477534 ps   ratio 0.714x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle   668244 ps ->   654943 ps   ratio 0.980x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  6673 ps over   2 iteration(s) = 3336 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          21426 ps over   5 iteration(s) = 4285 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             21421 ps over   5 iteration(s) = 4284 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         26610 ps over   6 iteration(s) = 4435 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover          617735 ps ->   554285 ps   ratio 0.897x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1133957 ps ->  1212802 ps   ratio 1.069x  ok
  the stable floor is 462 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1139937 ps = 2467.396 floors, budget 5200   ok
  PSRB-RSI: 2 id(s) for its 2 rows; the machine's floor is 462 ps; budget 2000 ns a step
C-PSRB-RSI-01 history 2000, period 14: best of 9 rounds p50 39 ns, p99 40 ns, max 90 ns, n 20000; median round p99 41 ns; worst round p99 119 ns
C-PSRB-RSI-01 history 20000, period 14: best of 9 rounds p50 39 ns, p99 40 ns, max 33721 ns, n 20000; median round p99 40 ns; worst round p99 42 ns
C-PSRB-RSI-01 history 200000, period 14: best of 9 rounds p50 39 ns, p99 40 ns, max 14805 ns, n 20000; median round p99 40 ns; worst round p99 43 ns
C-PSRB-RSI-01: best-round p99 against the first size 1.000x -> 1.000x -> 1.000x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-RSI-02 period 14, history 20000: best of 9 rounds p50 40 ns, p99 40 ns, max 35271 ns, n 20000; median round p99 42 ns; worst round p99 43 ns
C-PSRB-RSI-02 period 140, history 20000: best of 9 rounds p50 40 ns, p99 41 ns, max 480 ns, n 20000; median round p99 42 ns; worst round p99 43 ns
C-PSRB-RSI-02 period 1400, history 20000: best of 9 rounds p50 41 ns, p99 42 ns, max 34242 ns, n 20000; median round p99 42 ns; worst round p99 44 ns
C-PSRB-RSI-02: best-round p99 against the first size 1.000x -> 1.025x -> 1.050x; ceiling 3.000x, budget 2000 ns   ok
all ratios within the ceiling
```

Run 2, 03:10:18 UTC, HEAD `e47cf2df` (the wrapper's final code; `947dbd3a` changes only docs/06), load average 0.42:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 513 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                           705906 ps = 1376.035 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in           692029 ps ->   707359 ps   ratio 1.022x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in          692029 ps ->   700677 ps   ratio 1.012x  ok
  C-I-02 step: a wandering candle -> a motionless one          697752 ps ->   472730 ps   ratio 0.677x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle   697752 ps ->   672179 ps   ratio 0.963x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  6779 ps over   2 iteration(s) = 3389 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          21433 ps over   5 iteration(s) = 4286 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             21430 ps over   5 iteration(s) = 4286 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         26038 ps over   6 iteration(s) = 4339 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover          667652 ps ->   555430 ps   ratio 0.831x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1129579 ps ->  1208545 ps   ratio 1.069x  ok
  the stable floor is 462 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1121057 ps = 2426.530 floors, budget 5200   ok
  PSRB-RSI: 2 id(s) for its 2 rows; the machine's floor is 513 ps; budget 2000 ns a step
C-PSRB-RSI-01 history 2000, period 14: best of 9 rounds p50 39 ns, p99 40 ns, max 13132 ns, n 20000; median round p99 41 ns; worst round p99 121 ns; refusal None
C-PSRB-RSI-01 history 20000, period 14: best of 9 rounds p50 39 ns, p99 40 ns, max 17408 ns, n 20000; median round p99 40 ns; worst round p99 41 ns; refusal None
C-PSRB-RSI-01 history 200000, period 14: best of 9 rounds p50 39 ns, p99 40 ns, max 145 ns, n 20000; median round p99 40 ns; worst round p99 42 ns; refusal None
C-PSRB-RSI-01: best-round p99 against the first size 1.000x -> 1.000x -> 1.000x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-RSI-02 period 14, history 20000: best of 9 rounds p50 39 ns, p99 40 ns, max 3459 ns, n 20000; median round p99 41 ns; worst round p99 41 ns; refusal None
C-PSRB-RSI-02 period 140, history 20000: best of 9 rounds p50 39 ns, p99 40 ns, max 3448 ns, n 20000; median round p99 40 ns; worst round p99 40 ns; refusal None
C-PSRB-RSI-02 period 1400, history 20000: best of 9 rounds p50 39 ns, p99 40 ns, max 17182 ns, n 20000; median round p99 41 ns; worst round p99 41 ns; refusal None
C-PSRB-RSI-02: best-round p99 against the first size 1.000x -> 1.000x -> 1.000x; ceiling 3.000x, budget 2000 ns   ok
all ratios within the ceiling
```

The coverage step's run of the same target at `947dbd3a` (instrumented, opt-level 0, 08:46:39-08:50:30 IST, exit 0) is
a coverage run, not a cost figure; its lines whole, for completeness:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 14388 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                         12370322 ps = 859.766 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in         12572979 ps -> 12567468 ps   ratio 0.999x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in        12572979 ps -> 12663284 ps   ratio 1.007x  ok
  C-I-02 step: a wandering candle -> a motionless one        12832710 ps ->  8581699 ps   ratio 0.668x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle 12832710 ps -> 13012672 ps   ratio 1.014x  ok 
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                 58336 ps over   2 iteration(s) = 29168 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)         102289 ps over   5 iteration(s) = 20457 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)            105944 ps over   5 iteration(s) = 21188 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)        123724 ps over   6 iteration(s) = 20620 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover        11662801 ps -> 10844628 ps   ratio 0.929x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars          28455383 ps -> 30511301 ps   ratio 1.072x  ok
  the stable floor is 14531 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask       30106642 ps = 2071.890 floors, budget 5200   ok
  PSRB-RSI: 2 id(s) for its 2 rows; the machine's floor is 14388 ps; budget 2000 ns a step
C-PSRB-RSI-01 history 2000, period 14: best of 9 rounds p50 138 ns, p99 147 ns, max 22286 ns, n 20000; median round p99 198 ns; worst round p99 266 ns; refusal None
C-PSRB-RSI-01 history 20000, period 14: best of 9 rounds p50 138 ns, p99 148 ns, max 18022 ns, n 20000; median round p99 193 ns; worst round p99 215 ns; refusal None
C-PSRB-RSI-01 history 200000, period 14: best of 9 rounds p50 138 ns, p99 175 ns, max 18933 ns, n 20000; median round p99 196 ns; worst round p99 206 ns; refusal None
C-PSRB-RSI-01: best-round p99 against the first size 1.000x -> 1.006x -> 1.190x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-RSI-02 period 14, history 20000: best of 9 rounds p50 135 ns, p99 143 ns, max 23830 ns, n 20000; median round p99 188 ns; worst round p99 249 ns; refusal None
C-PSRB-RSI-02 period 140, history 20000: best of 9 rounds p50 135 ns, p99 140 ns, max 33519 ns, n 20000; median round p99 145 ns; worst round p99 191 ns; refusal None
C-PSRB-RSI-02 period 1400, history 20000: best of 9 rounds p50 133 ns, p99 139 ns, max 16019 ns, n 20000; median round p99 141 ns; worst round p99 212 ns; refusal None
C-PSRB-RSI-02: best-round p99 against the first size 1.000x -> 0.979x -> 0.972x; ceiling 3.000x, budget 2000 ns   ok
all ratios within the ceiling
```

## Gates 10, 11, 12, 27 and 27b, run here as CI runs them (the Mac's items; context)

The executable lines of those steps of `.github/workflows/ci.yml`, with the tools built from the tree's own `.github/*.rs`
by rustc 1.97.1, were run on START and on `947dbd3a`. Gate 27 is green on both (3,479 and 3,487 row ids, all unique).
Gates 10, 11, 12 and 27b are red on both with the same refusals, none naming a PSRB-RSI file or id: gate 10 the rows
R427-11 and STACK1028-03 (line 9103 on START, 9116 on the tip, moved by this part's 13 lines above it); gate 11 29
refusals on START and 29 on the tip, in other packages' files (`rsi.rs` is in the production closure, 525 files against
524, and no rule refuses it); gate 12 85 and 85, the same files, with 797 claim blocks on both (no PSRB-RSI doc block is
a claim by the gate's own reading); gate 27b `D-6247` heads two entries on both. The only other differences are the
counts this part adds. So no gate red is this part's; the base reds are for the Mac to read on its own pin.

## Decisions drafted (this part's block only, docs/05)

- D-8580 — PSRB-RSI: Wilder's RSI is `indicators::rsi`, a reading with no vocabulary position, the first reading after
  `period` changes and a `u32` change counter — 2026-11-03.
- D-8581 — PSRB-RSI: averages scaled by `trend::SCALE` in `i128`, floored; the ratio bound and `faithful()`; exact
  comparisons; `ppm` the floor; bounds const-asserted — 2026-11-03.
- D-8582 — PSRB-RSI: both averages zero read 100 with `flat()` true; a long run of unchanged closes departs only where
  `faithful()` is false — 2026-11-03.
- D-8583 — PSRB-RSI: refusals first and the state unchanged; closes only, timestamps not read; no look-ahead — 2026-11-03.
- D-8584 — PSRB-RSI: C-PSRB-RSI-01 and -02, their sizes, ceiling and 2 µs budget; the bench helpers copied per part —
  2026-11-03.

Before writing, `grep -nE '^#{2,4} +D-85[89][0-9]|^#{2,4} +D-860[0-9]' docs/05-decisions.md` found no heading. D-8585 to
D-8589 are unused. Each entry is dated 2026-11-03 as PARTS.md 3.6 gives and says it was written on 5 Nov 2026.

## Not done here, and why

- Mutants: deferred (operator, 30 Oct). The set file above carries the mutants lines for the background pass.
- Gate 14: its cover-table row for C-PSRB-RSI-01 and -02 is the main session's at the family assembly (PARTS.md 3.7).
  This part adds no `ratio(` call site to `benches/ratio.rs`, so the file's measurement-point count is unchanged.
- The coverage, gate and bench figures above are this builder's; the Mac re-measures on its own pin.

## NEEDS THE MAC (PARTS.md 3.7)

1. Check 1 on the Mac's pin: Route B on the D2 set against START, the bench wrapper and `main` by the instrumented bench
   run (this report's set file, commands and figures are the builder's).
2. Its gate 8 run of `cargo bench -p indicators --bench ratio` with both C-PSRB-RSI rows.
3. Gates 10, 11, 12 and 14 (gate 14's cover-table row for the new ids is the main session's at the family assembly).
   Gates 10, 11, 12 and 27b were red here on START and on the tip with the same refusals (above).
4. macOS rows: none expected; no platform code (the only clock read is `Instant` in the measuring functions).
5. The anchors on the then-current `feat/db-fix`: `lib.rs` after `pub mod pattern;`, the two PSRB-RSI bench slots, and
   the four ledger headings of PARTS.md 2.2.

# Report: FNO-LEVERAGE, built in a cloud session (3 Nov 2026)

Branch `cloud/fib-fno-leverage-20261103`, cut from `origin/cloud/fib-kit-20261103b` (`fa9022ba`, which is `1ad6e24ef8e0` plus the kit folder). Coverage and every diff below are against base `1ad6e24ef8e0`. Nothing was merged, no pull request was opened, and no other branch was pushed.

## What was built

`pull::nse_margin` (new, `crates/pull/src/nse_margin.rs`): each stock's dated intraday leverage from NSE's daily margin files (SPEC 11.1 and 11.2; PARTS FNO-LEVERAGE; OP-16, OP-17). There is no network I/O: no socket, no clock, and only files already on disk are read.

| Design name (PARTS :522-528) | Built as |
|---|---|
| `parse_c_var1(bytes) -> Vec<MarginRow { symbol, series, var_ppm, elm_ppm, extra_ppm }>` | `parse_c_var1(&[u8]) -> Result<Vec<MarginRow>, ParseError>`, and `parse_counted` (the same rows plus the count of other-series lines). Only `EQ` rows are kept, from columns 7, 8 and 9, held exactly as parts per million. A malformed row is refused by line and column. |
| `build_from_dir(src, out) -> Report` | `build_from_dir(&Path, &Path) -> Result<Report, BuildError>`. It writes `<out>/nse-margin-v1/<YYYY-MM>.nsm` (fixed stride: 31 day slots × one 12-byte cell per symbol) and the symbol dictionary `symbols.nsd`. The `Report` names the other batches, the non-margin entries and the leftovers the staging door cleared, and gives the set's digest. `encode` is the pure core that the build and the bench share. |
| `MarginTable::open(dir)` | `MarginTable::open(&Path) -> Result<MarginTable, OpenError>`, and `MarginTable::from_bytes` (the same checks, without files). |
| `margin(symbol, day) -> Result<Margin, MarginUnknown>`, O(1) | `MarginTable::margin(&str, Day)`: one month subtraction, one bit of the day map, one hash probe and one cell. `MarginUnknown` has five named variants: `OutsideSpan`, `NoFile`, `NotASymbol`, `NeverListed`, `NotListed`. |
| `leverage(m, reading) -> Ratio = min(5, 100 / max(20, M))`, exact | `leverage(Margin, Reading) -> Ratio`, which is `1,000,000 / max(200,000, M)` reduced. `Ratio` has `numerator()`, `denominator()`, `Ord` by value and `Display`. |
| `Reading { Day, Today, WithExtra }` | As named. `MarginTable::readings(symbol, day, today_symbol, today) -> Readings { day, today, with_extra }`, and `Readings::smallest()`. |
| `file_digest_hex()` | `MarginTable::file_digest()` and `file_digest_hex()`: BLAKE3 over the dictionary's trailer and every month's trailer, in month order. This is the `leverage_digest` term of the run identity. |

Also new: `source_file(name)`, which reads `C_VAR1_DDMMYYYY_N.DAT` (day and batch), and `month_name(year, month)`.

One change from my own first draft: files were first written through a `.part` temporary with `fs::write`, which is the create-and-truncate open that gate 28 exists to refuse. That name happened to slip past the gate's `.tmp` count, and dodging the gate is no fix. Every file is now born through `store::staging::create`, synced, then renamed (commit `38add2af`).

## Files (the touched set)

| File | Change |
|---|---|
| `crates/pull/src/nse_margin.rs` | new: 2,629 lines. Lines 1-1,417 are the module (its documentation included); lines 1,418-2,629 are the in-file `#[cfg(test)] mod tests`. |
| `crates/pull/src/lib.rs` | `pub mod nse_margin;` with its doc comment, right after `pub mod nse;` (ADDENDUM 1 Nov, PARTS :1323) |
| `crates/pull/benches/ratio/nse_margin.rs` | new: the C-FNO-LEV-01 wrapper |
| `crates/pull/benches/ratio.rs` | `#[path = "ratio/nse_margin.rs"] mod nse_margin;` with its doc and a blank line, right after the blank line that follows `use std::time::Instant;`. The `println!` of C-FNO-LEV-01 and its `ok &=` line go right after `ok &= day_table::day_lookup_is_flat();` (PARTS :1324-1325). |
| `.github/gates_tree.rs` | `FNO_LEVERAGE_LITERAL` after the blank line that follows `LATE_LABEL`; `FNO_LEVERAGE_LITERAL,` after `NAV_WORD,`; the length line is now exactly `const DECLARED: &[&str] = &[` (READINESS ADDENDUM item 12) |
| `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md` | rows inserted directly under `<!-- slot FNO-LEVERAGE -->` only |

## Tests (32, all in `pull::nse_margin::tests`; fixture rows are constants in the test)

Parsing:
- `fixture_rows_parse_to_exact_parts_per_million`
- `every_percentage_form_is_held_exactly_or_refused`
- `malformed_rows_are_refused_by_line_and_name`
- `every_parse_refusal_says_what_and_where`
- `a_non_eq_series_is_ignored_and_counted`
- `a_margin_files_name_gives_its_day_and_batch`

Leverage:
- `twenty_five_percent_is_four_times`
- `twenty_two_point_one_one_percent_is_the_exact_ratio`
- `twelve_and_a_half_percent_caps_at_five`
- `the_floor_is_the_cap_at_and_around_twenty_percent`
- `the_column_nine_reading_adds_the_extra_margin`
- `ratios_order_by_value`

Table and lookups:
- `a_listed_day_answers_its_own_files_columns`
- `a_missing_day_is_margin_unknown_never_another_days_value`
- `a_month_with_no_file_inside_the_span_holds_no_day`
- `every_unknown_margin_says_why`
- `on_a_rename_day_only_the_dated_symbol_answers`
- `the_first_of_june_2020_method_change_reads_the_same_totals`
- `the_today_reading_reads_todays_file_and_refuses_without_it`
- `the_smallest_reading_is_valid_under_every_reading`
- `encode_refuses_no_day_and_days_out_of_order`
- `the_bytes_do_not_depend_on_the_order_rows_are_listed_in`
- `a_twenty_four_byte_symbol_round_trips`

Build and open:
- `only_batch_one_of_the_day_is_read_and_the_rest_is_named`
- `the_build_is_the_same_bytes_whatever_order_files_and_rows_come_in`
- `every_build_refusal_names_its_path_file_or_day`
- `a_dead_writers_leftover_is_cleared_and_named_never_published`
- `every_build_refusal_says_what`
- `open_refuses_a_missing_dictionary_or_month_by_path`

Corrupt bytes:
- `every_corrupt_dictionary_is_refused_by_name` (16 cases)
- `every_corrupt_month_is_refused_by_name` (9 cases)
- `a_month_built_against_another_dictionary_is_refused`

Every case the design names is covered:
- parsing fixture rows, and malformed rows refused by name;
- a missing day gives `MarginUnknown` and never another day's value;
- 25% gives 4x, 22.11% gives exactly 100/22.11, and 12.5% caps at 5x;
- the column-9 reading;
- batch 1 only;
- 1 Jun 2020: same totals;
- the dated symbol on a rename day;
- a non-`EQ` series is ignored;
- today's file.

Extremes and permutations tested beyond the design's list:
- 0%, exactly 20%, 20.0001%, 100%, 125%, and three columns of 429,496.7294% each;
- the absent mark and `u64` overflow at every step of the number parse;
- a leap day, day 31, a year boundary, and a month with no file inside the span;
- a 24-byte symbol;
- file order and row order (same bytes);
- an idempotent rebuild;
- a dead writer's leftover;
- every corrupt byte the reader checks;
- every I/O refusal, by path.

## Gates

| Gate | Command (from the repository root) | Result |
|---|---|---|
| fmt | `cargo fmt --check -p pull` | clean |
| clippy | `cargo clippy -p pull --all-targets --locked -- -D warnings` (pinned 1.97.1) | clean: lib, every test target, and the `ratio` bench |
| module tests | `cargo test -p pull --lib nse_margin` | 32 passed, 0 failed |
| pull's whole suite | `cargo test -p pull --locked --no-fail-fast` (pinned 1.97.1, run as root, final code) | 32 test binaries (the lib, 30 integration tests, doc tests): **2,734 passed, 9 failed, 54 ignored**, exit 101, 6 min 56 s. Every integration and doc-test binary passed. The 9 failures are all in the lib, and all are permission tests: read-only roots, an unwritable journal, an immutable month, refused opens. Uid 0 bypasses those denials. All 9 pass when the same binary is run as `nobody` (`su -s /bin/sh nobody -c '<binary> --exact <the 9 names>'`: 9 passed, 0 failed). None touches `nse_margin`. The 9: `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`, `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`, `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`, `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`, `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice`, `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`, `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`, `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`, `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named`. |
| gate 1d | `.github/source_scan.rs` and `.github/gates_tree.rs`, built with `rustc --edition=2024 -D warnings`; `gate-1d prepare` / `strings` / `verdict` | Red on the base `1ad6e24ef8e0` with its own 481 words (`git worktree` of the base, same tool). On this tree the refused list is identical, 481 words, so no word of this part's files stays on it. Over the two new files alone: "OK — every segment-shaped literal under crates/pull is declared" (5 distinct literals). The tool's own tests (`rustc --test`): 50 passed. |
| gate 28 | `.github/gates_tmp_open.rs check` | "ok crates/pull/src/nse_margin.rs: 1 temporary name(s), 1 call(s) of store::staging::create"; overall OK |
| gate 27 | `.github/gates_ledger.rs invariant-ids` | OK, 3,492 rows, all ids unique |
| gate 27b | `.github/gates_ledger.rs decision-numbers` | Red on D-6247 alone, which has two headings on the base itself (`git show 1ad6e24ef8e0:docs/05-decisions.md`: 2). D-8150 to D-8155 each head exactly one entry. |
| docs readers | `cargo test -p core --test cost_invariants --test decision_citations` | 5 and 4 passed: C-FNO-LEV-01 is declared and printed by `benches/ratio.rs`, and every D-815x cited under `crates/` heads an entry |
| cited commits | `cargo test -p store --test cited_commits` | 5 passed, 1 failed: `1a98b1e13`, the rustc nightly's own commit quoted at the base's `docs/06-limits.md`:22132, is not an ancestor of `origin/main`. This part adds no citation; its diff adds 0 occurrences. |

Gate 10's crate-first half does not run while the base row R427-11 is refused (PARTS R2 note (c)), so it was not run here; the Mac's check re-runs it after BASE-REDS-1023.

## Coverage (Route B, measured)

Pinned nightly: `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, installed as `nightly-2026-08-08`. This is the toolchain the Mac's Route B names at `docs/06-limits.md`:22131.

```
RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_PROFILE_DEV_OPT_LEVEL=0 \
  CARGO_TARGET_DIR=target/cov cargo +nightly-2026-08-08 test -p pull --lib --locked --no-run
LLVM_PROFILE_FILE=…/nsm-%p-%m.profraw target/cov/debug/build/pull/<hash>/out/pull-<hash> nse_margin
llvm-profdata merge -sparse … ; llvm-cov report <binary> -instr-profile=… --show-branch-summary crates/pull/src/nse_margin.rs
```

`crates/pull/src/nse_margin.rs`, whole file (production code and tests), at the final code (commit `38add2af`; the docs commits after it change no code):

| Measure | Covered | Missed | Cover |
|---|---|---|---|
| Lines | 1,669 | 0 | **100.00%** |
| Branches | 78 | 0 | **100.00%** |
| Regions | 2,852 | 0 | 100.00% |
| Functions | 156 | 0 | 100.00% |

The other touched lines:
- The `crates/pull/src/lib.rs` line is a module declaration: there is no instrumented region.
- The `gates_tree.rs` group is a `const` and a list entry, proved by gate 1d's run and the tool's own tests.
- The bench wrapper (`crates/pull/benches/ratio/nse_margin.rs`) and the harness lines it adds were measured by running the bench target once under the same instrumentation (CC-20's precedent).

How the bench run was made:
- **Flags:** `RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'`, the pinned nightly, `--profile dev`.
- **Opt-level:** the dev profile's own 3. An opt-level 0 run of the whole harness was started first. It reached and ran C-FNO-LEV-01, but its profile is written only when the whole harness exits, which takes far longer at opt-level 0, so the figures below are from the opt-level 3 run. A coverage counter counts executions whatever the optimisation. That run printed "all ratios within the ceiling" and exited 0 in 6 min.
- **Report:** `llvm-cov report` and an `llvm-cov export -format=lcov` of the merged profile.

Results, `crates/pull/benches/ratio/nse_margin.rs`:

| Measure | Covered | Missed | Cover |
|---|---|---|---|
| Lines, per source line (lcov `DA`) | 93 of 94 | 1 | 98.94% |
| Lines, `llvm-cov report` | 98 of 106 | 8 | 92.45% |
| Branches | 4 of 8 | 4 | 50.00% |

The two line figures differ because the report also counts, as separate functions, the seven `refuse(…)` closures that never ran, on lines whose other code did run.

Every missed line and branch is the self-checking bench's own failing-check arm, which R2 excuses, named for this file:
- line 118, `refuse("the invented table does not answer as built")`;
- branches 114:12, 115:16 and 116:16, the three `||` checks that guard it, never true;
- branch 100:12's false outcome, the `OVER BUDGET` arm. The opt-level 0 run did take this arm (its held-day p99 was 1,582 ns, as printed), but that profile was not merged;
- the seven `unwrap_or_else(|…| refuse(…))` closures (functions, not lines).

Harness lines in `crates/pull/benches/ratio.rs`:
- the `println!` at 1864 and the `ok &=` at 1867 each have count 1;
- the `mod` line at 58 has no region.

Only the module's tests were run under instrumentation (`… pull-<hash> nse_margin`, 2,226 other tests filtered out). No other module calls `nse_margin`, so the other tests cannot reach the file. The Mac's Route B runs the whole crate and re-measures before anything counts.

## Benches

C-FNO-LEV-01 (`crates/pull/benches/ratio.rs` → `nse_margin::margin_lookup_is_flat`) times `margin(symbol, day)` in three cases:
- on a held day;
- on a day inside the span with no file;
- on a day before the span.

It does so over in-memory tables of 1, 10 and 100 months (400 symbols, days 1-28 held except the 15th). It fails on a ratio above the harness's 3x ceiling, and on a p99 above 1 µs at 100x (PARTS section 11; an ESTIMATE until measured on the Mac).

Measured here with `cargo bench -p pull --bench ratio --profile dev --locked`. That is the dev profile (opt-level 3, no LTO), chosen because the bench profile's fat LTO over `pull` would not finish in this session's time on 4 cores. It ran on a shared 4-core container, so it is ADVISORY (L-FNO-LEV-6); the Mac's check re-measures. Lines as printed:

```
C-FNO-LEV-01 one margin lookup at 1x, 10x, 100x the months held (FNO-LEVERAGE, D-8154)
  C-FNO-LEV-01 margin(symbol, day), a held day, 10x     54099 ps ->     59001 ps   ratio 1.090x  ok
  C-FNO-LEV-01 margin(symbol, day), a held day, 100x     54099 ps ->     53291 ps   ratio 0.985x  ok
  C-FNO-LEV-01 margin(symbol, day), a held day, 100x p50       73 ns  p99      234 ns  max      26962 ns  n=20000  p99 budget 1000 ns  ok
  C-FNO-LEV-01 margin(symbol, day), a day with no file, 10x      5051 ps ->      5552 ps   ratio 1.099x  ok
  C-FNO-LEV-01 margin(symbol, day), a day with no file, 100x      5051 ps ->     10540 ps   ratio 2.086x  ok
  C-FNO-LEV-01 margin(symbol, day), a day with no file, 100x p50       50 ns  p99      143 ns  max      61316 ns  n=20000  p99 budget 1000 ns  ok
  C-FNO-LEV-01 margin(symbol, day), before the span, 10x      8316 ps ->      8445 ps   ratio 1.015x  ok
  C-FNO-LEV-01 margin(symbol, day), before the span, 100x      8316 ps ->      6847 ps   ratio 0.823x  ok
  C-FNO-LEV-01 margin(symbol, day), before the span, 100x p50       44 ns  p99      138 ns  max      25394 ns  n=20000  p99 budget 1000 ns  ok
```

The whole `pull` harness, every bench in `crates/pull/benches/ratio.rs` with this part's included, printed "all ratios within the ceiling" and exited 0 (1 min 44 s wall). All six of this part's ratios are within the 3x ceiling and all three p99 figures are within 1 µs. The largest ratio, 2.086x, is the no-file case at 100x, a path of about 5 ns a call, where one clock tick is a large share of the cost. It is under the ceiling, and it is not explained here. Each max (25 to 61 µs) is a single call that took the scheduler's slice, and is printed rather than gated, as the harness's own spread lines are.

## Decisions drafted (this part's block, D-8150 to D-8159; six used)

- **D-8150:** what is read (C_VAR1 batch 1; the `EQ` rows' columns 7, 8 and 9, exact in parts per million) and the leverage formula as a reduced ratio. No network I/O.
- **D-8151:** the `nse-margin-v1` format:
  - sealed month files and a sorted dictionary;
  - each month is bound to its dictionary;
  - files are born through the staging door, dictionary last;
  - no lock;
  - `file_digest`.
- **D-8152:** three readings. The caller names the trade day, today, and each dated symbol; there is no fallback to another day's file.
- **D-8153:** UNVERIFIED here: the symbol and series columns (2 and 3), a header record's shape, and the layout either side of 1 Jun 2020. A real-file check is an operations step.
- **D-8154:** the O(1) lookup and C-FNO-LEV-01.
- **D-8155:** gate 1d's three words, and `DECLARED` as a slice.

Invariants: FNO-LEV-I01 to FNO-LEV-I12, and the bench row C-FNO-LEV-01 (`docs/04-invariants.md`). Limits: L-FNO-LEV-1 to L-FNO-LEV-6 (`docs/06-limits.md`). Plan row: `docs/07-plan.md`.

## Not proven here, and out of scope (named)

- **Reading NSE's real margin files, and their download.** Both are Mac operations steps. The operator's yes is recorded (PARTS decision 9.3, 30 Oct 2026). No `C_VAR1` file exists in this session and none was fetched.
- **UNVERIFIED column positions** (D-8153, L-FNO-LEV-3):
  - The design gives columns 7, 8 and 9 and the `EQ` series. Its source, `sources-leverage-execution.md` section 1, is on the Mac only and was not read.
  - So the symbol column (2), the series column (3) and a header record's shape are this build's reading.
  - Before `cli fno margin build` runs, the Mac should read one real file from each side of 1 Jun 2020. A layout that differs is refused by line and column, never misread.
- **The build verb** `cli fno margin build --src DIR --out DIR` is FNO-RUN's (PARTS :969, :1471), and so is the wiring of `Readings` into FNO-NET.
- **Which store directory receives the table** is FNO-RUN's. The design says `reference/nse-margin-v1`, and the build writes `<out>/nse-margin-v1`.
- **No store and no `:8080`** in this session (CLOUD.md adaptation 2). Nothing here reads or writes a store.
- **Evidence the rules cite that this folder lacks** (Mac only, not read):
  - `sources-leverage-execution.md`;
  - the D3 recipe file `d3-recipe.txt` and `d3-route-b.sh` (Route B was run from CLOUD.md item 4's description);
  - `state/decision-blocks.json`.
- **`docs/02-store-format.md`** has no FNO slot (PARTS 4.1), so the format is recorded in D-8151 and the module doc. Copying it there is the main session's, at landing.
- **Mutation tests** are deferred by the operator (THE BAR).

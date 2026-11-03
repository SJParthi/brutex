# FNO-BAND: cloud build report (Tue 3 Nov 2026)

Every result and page of the F&O strategy path carries the label
**"current F&O members only (survivorship bias not removed)"**. This report is
not a backtest result.

## 1. Result

- **Status: built.** Every file of the design is written, every gate named in
  `CLOUD.md` adaptation 4 was run, and the module `crates/runner/src/fno_band.rs`
  is at **100% of lines and 100% of branches by Route B, measured here** with
  the Mac's own nightly. The bench wrapper's only unexecuted lines are its
  self-check failure arms, named in section 5. The Mac's independent check
  re-measures before anything counts.
- **Branch:** `cloud/fib-fno-band-20261103`, cut from
  `origin/cloud/fib-kit-20261103b` (`fa9022ba`, which is FNO-SEAM's passed tip
  `1ad6e24ef8e0` plus the kit folder). Pushed after every step. Never merged,
  no pull request, no other branch pushed.
- **Base for the D2 set and for coverage:** `1ad6e24ef8e0` (FNO-SEAM, passed).
  FNO-BAND depends on FNO-SEAM alone (PARTS FNO-BAND, "Depends on"), so this
  base is final, not speculative, and no catch-up is owed.
- **The part's own commits** (`git log --oneline 1ad6e24ef8e0..HEAD`, less the
  kit commit `fa9022ba`): `a5542839` (the module), `f8f77d49` (its tests),
  `aff0594a` (the bench row and every ledger row), `ab3393f8` (rustfmt's layout
  of one assertion), `46def571` (two cost sentences corrected, the bench
  verdict label), and the commit that adds this report and `part-FNO-BAND.md`.
  Each message ends with the `Co-Authored-By` line.
- **D2 set** (`git diff --name-only 1ad6e24ef8e0..HEAD -- crates/`):
  `crates/runner/src/fno_band.rs` (new), `crates/runner/benches/ratio/fno_band.rs`
  (new), `crates/runner/src/lib.rs` (one line under its slot),
  `crates/runner/benches/ratio.rs` (three lines under its two slots). The four
  ledger documents hold no instrumented region.

## 2. What was built, file by file

`git diff --stat 1ad6e24ef8e0..HEAD -- . ':(exclude)reports'`: 8 files, about
2,390 insertions, no deletion. Every edit of a shared file is one pure insertion
under FNO-BAND's own slot line (`git diff -U0`: one hunk in `lib.rs` and in each
of the four ledgers, two in `benches/ratio.rs`, no line removed anywhere).

### `crates/runner/src/fno_band.rs` (new; library lines 1-879, tests 880-2122)

The price band of one stock-day (SPEC 10.2, OP-12), computed for every date and
labelled "band computed". Pure: no I/O, no telemetry (gate 17), integers only
(FIB-I10).

| Item | What it is |
|---|---|
| `LABEL` | "band computed", word for word |
| `BAND_RULE` | "fno-band-v1", the word for the run identity's `band_rule` term (READINESS ADDENDUM item 2; FNO-SEC-WIRE hashes it) |
| `RANGE_PPM`, `SPEC_FLEX_PPM`, `DESIGN_WIDTHS_PPM` | 10%; SPEC's 5% a step; the design's 15%, 18%, 20% (parts per million of the base) |
| `FlexReading { FivePerStep, FiveThreeTwo }`, `ALL`, `id()` | the two readings of T06-03's flex (D-8211) |
| `BaseReading { CloseTimesRatio, LastTradeBefore }`, `ALL`, `id()` | R-BAND-1 and R-BAND-2 |
| `ExDay { Ordinary, Demerger }` | what the day mask knows of the day (M7, M11) |
| `SessionRatio { raw, adjusted }` | `rho(D) = G / Z` at the session's first traded minute (SPEC 6.6 R1) |
| `Base::close_times_ratio(day, prev_close, ratio)` | R-BAND-1: `C_z(D-1) x G / Z`, snapped to the paisa half-up once |
| `Base::last_trade_before(day, last_trade)` | R-BAND-2: GDFL's last trade of D-1 |
| `Base::{reading, paise, first_limits}` | the base, its reading and its band before any flex |
| `Prints { ts, high, low }` | one traded second, what the band observes |
| `Limits { lower, upper }`, `pair()` | plain limits; `pair()` is the `(lower, upper)` FNO-RUN adapts to FNO-SEC-EXEC's `Bands` |
| `BandSide`, `Flex { from_ppm, to_ppm }`, `Observed { limits, up, down, at_upper, at_lower }` | what one observed second did, for FNO-RUN's records |
| `Refusal` (8 variants) + `Display` + `Error` | every refusal, by name (D-8212) |
| `DayBand::new(base, flex)` | the design's `DayBand::new(base_reading, base)`: the reading travels inside `Base`, the flex reading is the second argument (D-8210 item 7) |
| `DayBand::observe(sec)` | widens a side by the fewest steps of its reading that hold a print past it, in force from that second |
| `DayBand::limits(ts)` | the band in force at `ts` from the prints observed at or before it |
| `DayBand::near(sec, step)` | the design's lane test: within one price step of a limit, or past one |
| `DayBand::{base, flex}` | accessors |
| `Samples`, `quantile`, `bench_day`, `measure_checks` | the bench's measuring function and fixture, in the library so tests cover them (PARTS section 5, "Bench shape") |

Private: `Edge` (one side: its first mark and its flexes in time order, `now`,
`at` by binary search, `push`), `Mark`, `positive`, `refuse_demerger`, `check`,
`xorshift`, `elapsed_ns`.

### `crates/runner/src/lib.rs`

`pub mod fno_band;` directly under `// slot FNO-BAND (fno_band)`.

### `crates/runner/benches/ratio/fno_band.rs` (new) and `crates/runner/benches/ratio.rs`

`#[path = "ratio/fno_band.rs"]` and `mod fno_band;` under
`// slot bench-mod FNO-BAND`; `fno_band::row(&["C-FNO-BAND-01"]),` under
`// slot bench-row FNO-BAND`. The wrapper's `row` calls
`one_band_check_is_flat`, which gathers `measure_checks` over 22,500 samples at
each size under both flex readings, prints p50, p99 and max, and passes only if
each p99 is flat within runner's `CEILING_PERMILLE` (2,500) at 10x and 100x and
at most 100 ns.

### Ledgers, each under `<!-- slot FNO-BAND -->`

- `docs/05-decisions.md`: D-8210, D-8211, D-8212; D-8213 to D-8219 unused.
- `docs/04-invariants.md`: FNO-BAND-I01 to FNO-BAND-I07 and C-FNO-BAND-01, each
  citing tests by the form gate 10 resolves (`runner::fno_band::tests::…`,
  `runner::bench::one_band_check_is_flat`).
- `docs/06-limits.md`: L-FNO-BAND-01 to L-FNO-BAND-06.
- `docs/07-plan.md`: the plan row.

## 3. Tests (25, all in `runner::fno_band::tests`)

**The design's seven named cases** (PARTS FNO-BAND, "Tests"):

| Design case | Test |
|---|---|
| prints exactly at the band | `prints_exactly_at_the_band_are_at_it_and_flex_nothing`: at each limit, at both, no flex; one paisa past is a flex (both readings) |
| a flex observed | `a_flex_is_observed_from_a_print_past_the_band_and_holds_from_its_second`: SPEC's 15, 20, held, 25; the design's 15, 18, 20, then `PastTheLastFlex` at 21%; each flex in force from its second and not before; a fall flexes the lower side alone |
| a sell at the lower band and a buy at the upper band | `a_sell_at_the_lower_band_and_a_buy_at_the_upper_band`: rule 4 read against the limits handed on, each side and a second spanning both |
| resuming off the band | `trading_resumes_off_the_band_and_the_band_never_narrows`: a lock of five seconds, one tick off resumes buys, a flex is never undone, the old limit holds nothing after it |
| a split day (the adjusted Zerodha close times rho, inside one session) | `a_split_day_takes_the_adjusted_close_times_the_session_ratio_inside_one_session`: a 1:2 split with rho = 1, and a later adjustment by 5 cancelled by rho = 5, give one base; R-BAND-2 in the old basis is refused by the design's reading and flexed to 50% by SPEC's |
| a demerger ex-day skipped | `a_demerger_ex_day_is_skipped_under_both_readings` |
| the first print of the day already at the band | `a_first_print_of_the_day_already_at_the_band`: a gap up to the upper limit and down to the lower, under both bases and both readings |

**THE BAR's cases that PARTS section 10 gives FNO-BAND:** gaps
(`a_no_trade_gap_then_a_print_past_several_flexes_takes_them_in_one_second`,
which also has an opening gap inside the range moving nothing); circuit limits
and rule 6 (the seven above); corporate actions, BAND's rho (the split-day
test); out of box: band at the open (the first-print test) and the `i128`
overflow guards (`the_i128_guards_refuse_a_band_past_the_types`: a half-width
past `u64` ppm, a limit past paise `i64`, both refused by name).

**Further cases:** `a_second_past_both_limits_flexes_both_sides` (and the
design's refusal on either side); `the_band_answers_an_earlier_second_as_it_stood_then`
(history, a second observed again, a wider range in the same second);
`a_print_one_paisa_inside_at_and_past_each_limit` (2 readings x 2 sides x 3
shifts); `the_limits_round_inward_to_the_paisa` (Rs 100.05, 19 paise, one
paisa, a flexed width); `the_base_of_each_reading` (half-up at exact halves,
the feeds' 5 bp gap); `every_refusal_is_named` (each of the 8 variants, its
`Display` words, a refused second changing nothing, a broken band refusing
every later second while its earlier answers stand);
`near_is_within_one_step_of_a_limit`;
`the_label_the_rule_word_and_the_reading_names`;
`quantiles_take_the_nearest_rank`;
`the_bench_fixture_times_every_second_it_walks` (and the refusal it passes on).

**Seeded xorshift properties** (no property crate; 100 to 150 seeds each,
sessions of 120 to 200 traded seconds with gaps, small moves, jumps of 5% to
34% and exact locks at the band):
`no_later_second_moves_an_earlier_band` (FIB-I1: the band at every second up
to a cut is unchanged whatever the seconds after it print);
`the_band_holds_every_print_it_observed_and_never_narrows` (and a broken band
refuses every later second the same way; asserts the design's reading is
refuted at least once); `each_flex_is_the_fewest_steps_that_hold_the_print`
(the rung below would not hold it; more than 100 flexes checked);
`the_designs_band_is_never_wider_than_specs_while_it_holds`;
`asking_in_any_order_gives_the_same_answers` (walk order against a shuffled
order after the whole day); `a_trade_far_from_every_band_walks_one_lane`
(four lanes; a second no standing lane calls near has one rule-4 outcome; more
than 1,000 far and 100 near seconds occur).

## 4. Gates

Toolchains: the pinned `1.97.1` (`rust-toolchain.toml`) for fmt, clippy, the
tests, the bench and the `.github` gate tools; `nightly-2026-08-08`,
`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, for Route B (installed here
with `rustup toolchain install nightly-2026-08-08 --profile minimal
--component llvm-tools-preview`).

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | clean (exit 0) |
| clippy | `cargo clippy -p runner --all-targets --locked -- -D warnings` | clean, the lib, the tests and the bench target |
| the touched crate's tests | `cargo test -p runner --locked --no-fail-fast` | green: lib 770 of 770 (this part's 25 among them), every integration binary (`end_to_end` 4, `d_0694_record` 3, which reads the ledgers, and the other nine), doc-tests 6 |
| the part's tests under Route B | section 5's command | 25 of 25 |
| `core`, which reads the ledgers | `cargo test -p core --locked --no-fail-fast` | green: unit 205, `decision_citations` 4 (D-8210 to D-8212 head entries), `cost_invariants` 5 (C-FNO-BAND-01 is printed by the bench and declared as a row), `graph` 15, `findings` 14, every other binary |
| `vocab`, which reads every tracked file and the ledgers | `cargo test -p vocab --locked --no-fail-fast` | green: `no_py_language` 9, `no_float` 3, `workspace_is_rust` 18, `stale_claims` 10, every other binary |
| other readers of the four ledgers | section 4.1 | see there |
| gate 27, invariant ids | `.github/gates_ledger.rs invariant-ids` | OK: 3,487 ids, all unique (base 3,479: these eight rows) |
| gate 27b, decision numbers | `.github/gates_ledger.rs decision-numbers` | rc 1 on the base and here, the same one refusal: D-6247 heads two entries (lines 81917 and 82161, not this part's); 1,781 headings here against 1,778, these three |
| gate 10 | the workflow's commands | rc 1 on the base and here at the first stage, the same two rows (R427-11, STACK1028-03). Its later stages, run here anyway: 4,070 named tests checked against 4,044 at the base, so all 26 of this part's citations resolve; the same 2 missing at both (HA-02, AG05-13) |
| gate 12 | `.github/gates_bounds.rs gate12` with the workflow's allowlist | rc 1 on the base and here, the same refusal list; this part adds two claim blocks, one proved by a named test and one by UNVERIFIED |
| gate 14 | `.github/gates_bounds.rs gate14` with the workflow's cover table | OK at both; `crates/runner` covered, 55 cost claims against 54 |
| merge with sibling parts | `git merge-tree --write-tree HEAD <branch>` | exit 0 with `origin/cloud/fib-fno-law-20261103` (`96f41086`) and with `origin/cloud/fib-fno-portfolio-20261103` (`49412ba6`) |

Each `.github` gate was run in a worktree of `1ad6e24ef8e0` and at the tip with
tools built by `rustc +1.97.1 --edition=2024 -D warnings`, and the logs were
compared; every difference is a count these rows add.

### 4.1 Other readers of the four ledgers

`grep` for a path literal to `docs/04` to `docs/07` under `crates/` lists them.
Run: `runner`'s `d_0694_record` and `core`'s and `vocab`'s readers (above);
`pull --test derive`; `store`'s `docs`, `libm_key`, `bisect_cost`,
`sealed_codec_alloc`, `tail_proof`, `cited_commits`; `cli`'s
`pool_and_escape_docs`, `ledger_scan_costs`, `c4_cli_02_limits`,
`limits_o1cli_2` to `_6`, `limits_doc_drift`, `ledger_append_lookup_costs`.
Results: section 8 (written when the runs end).

Not built here: the `cli` and `api` **unit-test** binaries holding eleven more
readers (`selection_v6_source.rs`, `boolean_statistics_tests.rs`, and in `api`
`server.rs`, `pullrun.rs`, `indexstoprankingjson.rs`, `cost_limits_tests.rs`,
`calendar_of.rs`, `booleanjson_tests.rs`, `bars_sealed_tests.rs`,
`audit_json.rs`, `census_request_tests.rs`, `operation_audit_tests.rs`). Each
reads only a section or rows it names (the D-1446 and D-1445 sections, rows
L-SC-24, L-SC-25 and P-01 to P-04, the W2-cli2-4 section), and a section reader
stops at the next `## ` heading, which at the base is already the F&O block's
heading, so rows under FNO-BAND's slot cannot reach any of them. That is read,
not run.

## 5. Coverage (Route B, measured here)

```
RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' \
CARGO_PROFILE_DEV_OPT_LEVEL=0 CARGO_TARGET_DIR=<scratch>/rb/target \
LLVM_PROFILE_FILE=<scratch>/rb/prof/runner-%p-%m.profraw \
cargo +nightly-2026-08-08 test -p runner --locked --lib -- fno_band
llvm-profdata merge -sparse <scratch>/rb/prof/*.profraw -o tests.profdata
llvm-cov report <runner unit-test binary> --instr-profile=tests.profdata \
  --show-branch-summary --show-region-summary crates/runner/src/fno_band.rs
```

The filter runs the 25 tests of this module; no other test of the crate calls
it, so the file's figures are the crate run's. The whole crate's 770 passed on
the stable toolchain (section 4).

| File (tip `46def571`) | Regions | Functions | Lines | Branches |
|---|---|---|---|---|
| `crates/runner/src/fno_band.rs` | 1,806 of 1,806 (100.00%) | 97 of 97 | **1,069 of 1,069 (100.00%)** | **48 of 48 (100.00%)** |

`llvm-cov show --show-regions --show-branches=count` prints no zero-count
marker for the file. The first measurement (`a5542839`..`f8f77d49`) found two
unrun regions and no unrun line or branch: the dead `_ => false` arms of two
`matches!` assertions, which became exact comparisons.

**The bench wrapper** (PARTS R2 COVERAGE: "run the bench target once under the
same coverage instrumentation and merge its profile"): `cargo +nightly-2026-08-08
bench -p runner --bench ratio --locked` with the same `RUSTFLAGS` and its own
profile directory, then `llvm-cov report <the bench binary>` over the wrapper
and `ratio.rs`. Figures: section 8. The wrapper's self-check failure arms, which
no correct fixture reaches, are excused by name: `crates/runner/benches/ratio/fno_band.rs`
lines 52-53 (a day refused or timed no flex) and 88 (`measure_checks` refused a
day). In `ratio.rs` this part's two code lines run; its other unexecuted lines
are the base's own failure arms, and its closing "ok" lines, which an
instrumented run skips because instrumentation slows a band check past the
100 ns budget (244 ns measured so), failing the row as it should.

Not measured by the D3 recipe file itself, which is on the Mac
(`decisions-20261027/d3-recipe.txt`): this follows `CLOUD.md`'s statement of
Route B. The Mac's Route B re-measures.

## 6. Benches (advisory here)

`cargo bench -p runner --bench ratio --locked` on this container (4 vCPU Intel
Xeon at 2.10 GHz, x86_64 Linux), release bench profile, each sample holding two
clock reads (C-FNO-BAND-01's row in `docs/04-invariants.md` carries these):

| Reading | Kind | p99 1x | p99 10x | p99 100x | ratio 10x / 100x | worst p99 vs 100 ns |
|---|---|---|---|---|---|---|
| SPEC's 5% a step | check | 85 ns | 73 ns | 74 ns | 0.858x / 0.870x | ok |
| SPEC's 5% a step | look-back | 51 ns | 52 ns | 50 ns | 1.019x / 0.980x | ok |
| design's 5, 3, 2 | check | 85 ns | 74 ns | 77 ns | 0.870x / 0.905x | ok |
| design's 5, 3, 2 | look-back | 51 ns | 51 ns | 53 ns | 1.000x / 1.039x | ok |

p50s were 35 to 53 ns. Maxima ranged from 183 ns to 67 µs: single interrupted
samples, printed, and not a quantile the row judges.

**C-R-04, a row this part does not touch,** breached on that first run here:
5,700 floors against its budget of 5,000. Its numerator, the column build, read
3.11 µs a bar, the same as on the base's run; its denominator, a five-pass floor
timed once, read 546 ps against the base run's 757 ps. That is the instability
D-0680 and D-0690 record for that floor. Section 8 holds the further runs.

## 7. Decisions drafted (own block only)

- **D-8210** — the computed band of every stock-day: both base readings walked
  as lanes, ten per cent either way rounded inward, flexes observed from prints
  and kept with their second, never undone, traded prints only, plain limits,
  the design's one change of shape, the bench row.
- **D-8211** — the flex rule's two readings (SPEC's 5% a step, the design's
  5%, then 3%, then 2%), both computed and walked as lanes, the design's refused
  by name past 20%, settled only by the source's own words in a later entry.
- **D-8212** — what the band refuses, the integers it keeps, the stated limits.
- D-8213 to D-8219 unused.

## 8. Results of the runs still going when this was first written

(Filled in as each run ends.)

## 9. Problems and findings

1. **T06-03 is read two ways and this part does not pick one by opinion.**
   SPEC 10.2 reads the flex as 5% a step; PARTS FNO-BAND reads "5%, then 3%,
   then 2%". FNO-LAW's charter row quotes both and hands the choice here. The
   source's own words are in `sources-leverage-execution.md` lines 21 and 189,
   on the Mac only. Both are built and walked as lanes (D-8211); **the main
   session should read those two lines** and have a new entry retire the
   reading they refute. Until then which one NSE applies is UNVERIFIED.
2. **R-BAND-1 reads the session's first traded minute** (SPEC 10.2's
   definition), so for a second inside that minute its base rests on that
   minute's close; only the feeds' gap carries into it (L-FNO-BAND-01).
   FNO-DAILY's D+1 entries start in that minute, so FNO-RUN may want R-BAND-2
   alone there; that is FNO-RUN's call, and it is stated, not decided, here.
3. **R-BAND-2 across a split, bonus or rights issue** is in the old basis
   until R4-26 is live. The design's reading refuses it past 20%; SPEC's flexes
   until it holds the day (the split-day test shows a 50% band). The headline
   takes the worse lane, so a wrong band can only lower it (L-FNO-BAND-02).
4. **NSE's tick rounding of its band** is unknown before 2024-02-05; a lock
   printed at an outward-rounded tick reads here as a flex (L-FNO-BAND-03).
   FNO-BAND-FILES will measure agreement with `PricRg`.
5. **The design's signature.** `DayBand::new(base_reading, base)` is
   `DayBand::new(base, flex)` (D-8210 item 7). FNO-SEC-EXEC's `Bands` trait is
   on no branch this session can read (no `fix/fib-*` branch is pushed), so the
   adapter's shape is taken from PARTS (`fn limits(&self, ts) -> (i64, i64)`),
   and `Limits::pair` returns `(lower, upper)` in that order.
6. **A shared target dir between two worktrees runs a stale bench.** A base
   worktree benched into this checkout's `target/` wrote the same artifact
   name, and the next tip bench (0.06 s "Finished") ran the base's binary: no
   C-FNO-BAND-01 line in its output. That run is counted as a second base
   sample, never as the tip's. Every tip figure above is from a run whose
   output holds the C-FNO-BAND-01 lines.
7. **Base reds, not this part's**, each reproduced on the base worktree with
   the same output: gate 27b (D-6247 twice), gate 10 (R427-11, STACK1028-03;
   HA-02 and AG05-13 at its later stage), gate 12 (the same refusal list).

## 10. Not done here, and why

- **Anything needing the store or NSE files:** none of this part reads one.
  NSE's security-file band (`PricRg`, from 2024-02-05) is FNO-BAND-FILES (F3);
  wiring the limits into FNO-SEC-EXEC's `Bands` and walking the lanes is
  FNO-RUN's (F2). The drain test and the disk floor name the Mac's drive, which
  this container does not have (`CLOUD.md` adaptation 2); no store, no
  `runtime/`, no app and no `:8080` were touched or needed.
- **`cargo test --workspace --locked` in full:** not run (4 cores; `cli` and
  `api`). Run instead: the touched crate whole and every reader of an edited
  file that section 4.1 lists, with the reason given for the unit-test readers
  not built.
- **`cargo deny check`:** not installed here; the part changes no manifest and
  no lock line (`git diff 1ad6e24ef8e0..HEAD -- Cargo.lock Cargo.toml
  crates/runner/Cargo.toml` is empty), so its answer is the base's.
- **Mutation tests:** deferred by the operator.
- **Evidence on the Mac only, not read here:** `sources-leverage-execution.md`
  (T06-03's own words, lines 21 and 189; T06-10), the D3 recipe, the operator's
  memory files, `ENGINE.md`. The code cites them by path and copies no value it
  did not see.

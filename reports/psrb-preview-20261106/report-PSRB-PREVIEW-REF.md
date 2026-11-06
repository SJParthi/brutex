STATUS: DONE

# report-PSRB-PREVIEW-REF.md: the independent reference model of the PSRB preview, session TOKEN PSRB-PREVIEW-REF-20261106-1

Built Fri 6 Nov 2026, 05:14 to 07:10 UTC, in a Claude Code cloud container (Linux 6.18, 4-core Intel Xeon at
2.10 GHz, 15 GiB). Branch `cloud/psrb-preview-ref-20261106`, cut from `origin/cloud/psrb-preview-kit-20261106`
(`a695f4c38c667fc2183452afb8536896306eb0b8` = `feat/db-fix` `f01d7a0e430e` + the kit commit). The OWNER file
`OWNER-PSRB-PREVIEW-REF.md` holds the token; it was compared on origin before every push.

## 0. At a glance

| item | figure |
|---|---|
| crate | `crates/psrb_preview_ref`: empty `[dependencies]`, 37 library files with code plus `lib.rs`, 31 test modules, one bench target |
| tests | 133 in the library test target plus the bench target's smoke pass, all passing (`cargo test -p psrb_preview_ref`; 26.5 s of tests on this host) |
| format, lints | `cargo fmt -p psrb_preview_ref --check` clean; `cargo clippy -p psrb_preview_ref --all-targets -- -D warnings` clean, on the pinned stable 1.97.1 |
| coverage | lines 4,093 / 4,093 (100.00%), branches 578 / 578 (100.00%), regions 6,712 / 6,714 (99.97%) of the 37 library files (cargo-llvm-cov 0.9.1, nightly-2026-08-08: a cloud figure, section 5) |
| bench | per candle and per second, p99 flat at 10x and 100x of history: worst ratio 1,066 permille against the 3,000 ceiling (section 6) |
| golden | 200 files, 20,696,101 bytes, in `reports/psrb-preview-20261106/golden/` (section 7) |
| code tip | `efe53cbc62c2af392c3e647dc0c430e7c9ab1827` (the last commit that touches anything outside `reports/`); the golden files are `5989f2c7` |
| outside the crate | three workspace gate tests now fail because the workspace has a 14th member (section 9); none was edited |

## 1. Kit check

- `MANIFEST.md`'s four rows equal the sha256 of `CLOUD.md` (`6321c814...`), `PREVIEW-SPEC.md` (`24162ba2...`, the
  08:54 IST version TEST-PLAN.md names), `OPERATOR-ADDENDUM-20261106.md` (`65acdb1d...`) and `TEST-PLAN.md`
  (`2e3550bb...`). The kit branch adds only those five files to `feat/db-fix` `f01d7a0e430e`.
- The kit arrived at 05:14 UTC (polled every 240 s from 03:13 UTC); the branch and the OWNER file were pushed at once.

## 2. What was built

A new workspace crate **`crates/psrb_preview_ref`** (one line added to the root `Cargo.toml` `members`; `Cargo.lock`
gains one package entry with no dependency). Its `[dependencies]` table is empty: it names no workspace crate (not
`indicators`, not `costs`, not `pull`) and no registry crate, so nothing it computes shares code with the tool under
test. Rust only, `#![deny(clippy::as_conversions)]`, the workspace lints (`float_arithmetic`, `indexing_slicing`,
`unwrap_used`, `panic`, ... deny), no floating-point type anywhere (gate test), every amount paisa in `i64` and every
product or sum `i128`, percent in basis points, ratios in permille.

The recipe, function by function as TEST-PLAN.md names them:

| TEST-PLAN | module::item | what |
|---|---|---|
| F01 | `universe::load`, `load_with` | names in byte order, ordinals, the ICICILIFE -> ICICIPRULI series, FNV-1a ids; every refusal listed (collision before duplicate) |
| F02 | `membership::probe` | the three probe answers + open -> Dated or today's list, every failing probe named |
| F03 | `membership::Membership::may_trade` | yes / no / unknown; the carry day (October 2026 asks 2026-09-30, modelled as `carry_from`); outside the window refused |
| F04, F05 | `span::trading_span`, `span::warmup_start` | the later first month, never before the window's first session; the contiguous Zerodha run (Q1 refused) |
| F06 | `daymask::admit` | M1..M5 in order, the outage day 2021-02-24 (skip row), the vendor-late day 2019-05-30 (fallback) |
| F07..F10 | `candles::fold_day`, `close_instant`, `auction_eligible`, `check_minutes` | k-minute buckets on the 09:15 grid, the 15:15 close of an auction-eligible day from 2026-08-03, padded minutes never folded, holes, the month's `FoldError` |
| F11 | `dayref::day_bar`, `PrevSession`, `AcceptedBar` | the previous full session's H, L, C; a refused or missing bar reads nothing |
| F12..F19 | `readings::Bank::step`, `indicators::{atr, supertrend, bollinger, rsi, willr, cci, ema}`, `readings::Taint` | one candle into all seven states or none; taint 10, 20, 14, 14, 20, 200 |
| F20..F27 | `cond::c1_narrow_cpr` .. `c8_ema_side` | the eight conditions in exact `i128` |
| (tables) | `ruleset::{VIDEO_ORIGINAL, AUTHOR_V2, VIDEO_SCREEN}`, `ruleset::check` | the three rule sets as data; a table naming 10/3 for AV2, an empty or repeated table is refused |
| F28, F29 | `signal::ready_gate`, `signal::evaluate` | span, hole, cold and tainted blockers in table order; the conjunction per side |
| F30, F34..F38 | `fills::{entry, StopScan, reaches, worst_since, minute_prints, second_prints}` | worst-print fills, the resting stop, the remainder rule, the minute fallback |
| F31, F32 | `pricemap::ratio_source`, `map_level` | Z and G of the same minute, the latest earlier minute with both; floor for a long, ceil for a short |
| F33 | `trail::step`, `across`, `tighter` | flip, move (tighter only), hold, no ratio |
| F39 | `trade::simulate` | the whole walk: entry, mapped stop, trailing, flip, 15:07:59 time order, 15:10:00 square-off, unclosable |
| F40 | `sizing::qty` | `50_000_000.div_euclid(entry)` |
| F41..F52 | `costs::{price_trade, trade_orders, gross_and_net, dated_rates, today_rates, state_stamp}` | every row and reading of `costs-table/2026-10-30/r1`, both clocks, the headline |
| F53, F54 | `book::merge`, `book::Slots` | the (day, T, symbol bytes) merge, the 20 slots, while open / unfillable / slots full |
| F55, F56 | `account::months`, `account::drawdown` | exit-day attribution, the compounding capital start, exact +2.5%; the deepest fall |
| F57, F61 | `report::row_stats`, `money::rupees` | every section 3.10 column; rupee text by integer division with Indian grouping |
| F58, F59 | `verdict::bar`, `verdict::rank` | PASS / MISS, gaps, floor; the closest-miss order with a final (rule set, timeframe, account) tie |
| F60 | `report::{head_compare, per, permille, PAIRS}` | AV2 over VO, VS over VO and VS over AV2 in permille for trades, net and both drawdown readings, a zero or negative base printed as such; the nearest met floor over every row |
| run | `run::run` | Phase A per stock and timeframe (portfolio-free candidates, priced), Phase B per row: 3 rule sets x 6 timeframes x 3 accounts = 54 rows |
| synthetic | `generator`, `catalogue` | the seeded step-path generator and the 200 scenarios |
| golden | `golden::{render, write_all}` | the plain-text golden files, `create_new` only |
| bench | `benchkit`, `benches/ratio.rs` | the ratio harness |

**Not built here:** F62..F73 and F75 (unit planning, worker pool, the results database, manifest, resume, snapshots,
dense index, audit, query, verify, progress, extremes, the command line) and F76 (`REPORT.md` and the `.tsv` files).
They are the Mac tool's infrastructure over the real store and its output files; an in-memory reference has none of
it, and its expected outputs are the golden files instead. F74's self-check fixtures are tests here (CN01 among the
24 cost vectors, the CPR tie CPR-1, the EMA fixture, 125 three-minute buckets per session).

## 3. What was read, and what was not

Read: `CLAUDE.md`; the kit (`CLOUD.md`, `PREVIEW-SPEC.md`, `OPERATOR-ADDENDUM-20261106.md`, `TEST-PLAN.md`,
`MANIFEST.md`). Because PREVIEW-SPEC defers the readings and the costs to documents it cites rather than restating
them, these **design texts** were read as the spec of record (none is code):

- `reports/psrb-cloud-20261103/PARTS.md` (on `cloud/psrb-kit-20261103`): RSI 3.2-3.5, Bollinger 4.2-4.5, the two-band
  Supertrend 5.2-5.4 with fixtures ST-1..ST-9.
- `reports/psrb-cloud-20261105/PARTS-WILLR-CCI.md` (on `cloud/psrb-kit-20261105`): narrow CPR and pivots 4.1-4.3 with
  CPR-1/2/2b and PIV-1/2, the EMA's form 4.1, Williams %R 5.2-5.5 (WR-1/2), CCI 6.2-6.5 (CCI-1/2/3).
- `reports/fib-cloud-20261103/COSTS.md` and `costs-table.json.md` (on `cloud/fib-kit-20261103b`): the dated table,
  the today clock, the ten readings and the 24 contract-note vectors (PREVIEW-SPEC's COSTS and CT).
- `docs/05-decisions.md` D-1542 (EMA and ATR seeded by the simple mean of the first period).
- The earlier kits' `CLOUD.md` and `d3-route-b-cloud.sh.md` (conventions only) and the file lists of the earlier part
  branches.

**Not read: any code of the Mac tool (it is not in the repository), of the PSRB part tips (`rsi.rs`, `bollinger.rs`,
`dual_band_supertrend.rs`, `williams_r.rs`, `cci.rs`), of `crates/indicators` (`trend.rs`, `anchored.rs`, `daily.rs`),
of `crates/costs` (`equity_clock.rs`, `stack.rs`), of `crates/pull` (`fold.rs`, `calendar.rs`) or of `crates/store`.**
Every function here was written from the texts above; the fixtures in those texts are this crate's tests. The only
other code read was the workspace's own gate tests (`crates/core/tests/graph.rs`, `crates/vocab/tests/*.rs`), run in
section 9, which compute nothing of the recipe.

## 4. Tests

`cargo test -p psrb_preview_ref`: **133 tests, all passing** (26.5 s on this host), plus the bench target's smoke
pass. Ids follow TEST-PLAN 0.1: `t_` normal and permutation, `x_` extreme-case tables (rows tagged `combo:` and `oob:`
in their comments), `p_` property tests against naive models, `t_g` the global gates.

- **Fixtures from the design texts, exact:** ST-1..ST-9 (every candle's trend, line, both bands and `settled`), RSI-1
  (every scaled G and L and ppm), BOLL-1 and BOLL-2 (sums, V, D, the bands at 1.999 / 2.000 / 2.001 and 1.647 / 1.648),
  WR-1 and WR-2, CCI-1, CCI-2 and its mirror (the +-200 ties), CCI-3 (the ceiling), the EMA 200 fixture
  (199 closes of 101 then 100: scaled 100,995,000, short true, long false), CPR-1/2/2b, PIV-1/2.
- **Costs: all 24 COSTS contract-note vectors** (CN01..CN24, every line of dated precise, dated worst, today precise and
  today worst, the headline and the net; multi-fill orders and the broker square-off of CN20..CN24 included) match to
  the paisa, plus every era boundary, the worst state by turnover, every refusal, both cases where the today clock is
  the dearer one (section 7), and invariants on 10,000 generated trades over the whole window.
- **Property tests (fixed seeds `0x5053_5242_0000_0001..=0004`):** universe vs a `BTreeSet` model; may_trade vs a map
  model with the carry; warm-up vs a walk-back model; the fold vs a naive bucket grouping; the bank (ATR, Supertrend,
  Bollinger, %R, CCI) vs full-history recomputation; RSI and EMA vs from-scratch recomputation; the taint vs a look-back
  model; C1/C2 vs rational models; ratio and mapping vs brute force and exact floor/ceil; **the trade walk vs an
  independent second-by-second replay written from SPEC 3.5 (10,000 stock-days)**; the quantity's exposure bound; the
  merge vs one sort; the slots vs an interval model; months vs naive sums and the drawdown vs an O(n^2) model; the rank
  vs a comparator sort; the rupee text vs a digit model; the head's permille vs its defining inequality.
- **Catalogue checks:** every planted exit kind and refusal occurs in its scenario; the 22-stock tie admits S01..S20 at
  the first instant; the exact months hold (250 bp pass, 249 bp fail with the exact gap 2,000,000); the run stops by
  name on the charge refusal and the duplicate key.
- **Gates:** no `f32`/`f64` token, the `as_conversions` deny, no `remove_file`/`remove_dir`/`fs::rename`/`File::create`
  or `truncate(true)` anywhere in the crate (every file it writes, tests included, is opened `create_new`; one test
  then appends to a file it created itself), only `.rs`/`.toml`/`.md` files, the rule sets named only in their table.
- Mutants: not run (the operator's standing deferral, TEST-PLAN section 6 item 1).

## 5. Coverage

`cargo +nightly-2026-08-08 llvm-cov --branch -p psrb_preview_ref`, then `cargo llvm-cov report --branch` (cargo-llvm-cov
0.9.1 on `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, the toolchain TEST-PLAN section 6 pins), at the workspace's
own test profile (`opt-level = 3`, overflow checks on), on the code tip `efe53cbc62c2`: all 133 tests passed under
instrumentation (387 s). A cloud figure, as measured, covered / total:

| file | lines | branches | regions |
|---|---|---|---|
| `account.rs` | 52/52 | 10/10 | 79/79 |
| `benchkit.rs` | 119/119 | 8/8 | 197/197 |
| `book.rs` | 63/63 | 8/8 | 91/91 |
| `candles.rs` | 101/101 | 32/32 | 151/151 |
| `catalogue.rs` | 967/967 | 64/64 | 1651/1651 |
| `cond.rs` | 87/87 | 12/12 | 119/119 |
| `costs.rs` | 255/255 | 40/40 | 428/428 |
| `daymask.rs` | 34/34 | 16/16 | 39/39 |
| `dayref.rs` | 36/36 | 2/2 | 54/54 |
| `fills.rs` | 65/65 | 10/10 | 103/103 |
| `generator.rs` | 185/185 | 10/10 | 272/272 |
| `golden.rs` | 430/430 | 32/32 | 743/745 |
| `indicators/atr.rs` | 43/43 | 6/6 | 67/67 |
| `indicators/bollinger.rs` | 49/49 | 8/8 | 82/82 |
| `indicators/cci.rs` | 42/42 | 8/8 | 70/70 |
| `indicators/ema.rs` | 25/25 | 4/4 | 28/28 |
| `indicators/mod.rs` | 15/15 | 20/20 | 21/21 |
| `indicators/rsi.rs` | 63/63 | 14/14 | 82/82 |
| `indicators/supertrend.rs` | 60/60 | 20/20 | 83/83 |
| `indicators/willr.rs` | 46/46 | 8/8 | 75/75 |
| `market.rs` | 46/46 | 8/8 | 53/53 |
| `membership.rs` | 59/59 | 20/20 | 68/68 |
| `money.rs` | 37/37 | 14/14 | 73/73 |
| `pricemap.rs` | 44/44 | 10/10 | 77/77 |
| `readings.rs` | 95/95 | 8/8 | 126/126 |
| `report.rs` | 92/92 | 6/6 | 207/207 |
| `rng.rs` | 30/30 | 6/6 | 55/55 |
| `ruleset.rs` | 53/53 | 8/8 | 71/71 |
| `run.rs` | 388/388 | 44/44 | 646/646 |
| `signal.rs` | 62/62 | 12/12 | 110/110 |
| `sizing.rs` | 5/5 | 2/2 | 6/6 |
| `span.rs` | 47/47 | 12/12 | 92/92 |
| `time.rs` | 132/132 | 22/22 | 251/251 |
| `trade.rs` | 133/133 | 32/32 | 194/194 |
| `trail.rs` | 30/30 | 6/6 | 34/34 |
| `universe.rs` | 65/65 | 30/30 | 128/128 |
| `verdict.rs` | 38/38 | 6/6 | 86/86 |
| **all 37 files** | **4,093 / 4,093 (100.00%)** | **578 / 578 (100.00%)** | 6,712 / 6,714 (99.97%) |

Functions 484 / 484. **Lines and branches are 100%.** The two regions not reached are the error returns of
`file.write_all(..)?` and `file.sync_all()?` in `golden::write_all` (`golden.rs:586-587`): a write or an fsync that
fails after `create_new` succeeded, which only a failing disk reaches; neither is a line or a branch of its own. No
`#[coverage(off)]`, no `cfg(not(coverage))`, no test skipped, no file excluded by this run's command. The previous run
(on `d56cfb48`) measured 4 lines and 5 branches short; each was closed by a test or by removing an unreachable arm
(section 0's commit `efe53cbc`), and the figures above are the re-measurement.

## 6. Bench

`cargo bench -p psrb_preview_ref --bench ratio` (full mode: the bench profile, `--bench` passed), one run on this host
on the final code, nanoseconds per step, 4,000 timed steps at each history size (2,000 / 20,000 / 200,000 steps of
history before the timed ones):

| bench | history | p50 | p99 | max | p99 ratio to 1x |
|---|---|---|---|---|---|
| B-candle: one candle (all seven reading states, the gate, the three rule sets' conjunctions) | 1x | 361 | 865 | 26,962 | |
| | 10x | 358 | 834 | 18,233 | 964 permille (Flat) |
| | 100x | 361 | 844 | 17,718 | 975 permille (Flat) |
| B-second: one second of the resting stop's scan | 1x | 29 | 30 | 38 | |
| | 10x | 29 | 32 | 116 | 1,066 permille (Flat) |
| | 100x | 29 | 31 | 39 | 1,033 permille (Flat) |

The gate is p99 at 10x and at 100x within 3,000 permille of p99 at 1x (TEST-PLAN section 3); both steps are flat, so
the per-candle and per-second work does not grow with history. `cargo test` runs the same file in SMOKE mode (sizes
20 / 200 / 2,000, 50 steps), which prints the same lines and never gates. These are one host's figures, not the Mac's.

## 7. The golden files

`reports/psrb-preview-20261106/golden/<scenario>.txt.md`, 200 files, 20,696,101 bytes in all, written by the test
`golden::tests::the_repository_golden_files_are_the_catalogue` with `create_new` (a file that exists is compared,
never overwritten; the test fails on any difference). Synthetic data only. Format `psrb-ref-golden/1`: a Markdown
header, then two fenced `text` blocks, **Inputs** and **Expected outputs**, one record per line, fields separated by
one space. Money is paisa; times are IST; days `YYYY-MM-DD`; months `YYYY-MM`; `-` is an empty list.

**Inputs**

| line | meaning |
|---|---|
| `window <first> <last>` | the judged months, inclusive |
| `calendar weekdays=full weekends=closed` | the default day kinds |
| `calendar-override <day> <full\|short\|muhurat\|closed>` | a day whose kind differs |
| `membership-probe opened=<yes\|no> reliance-2018-09-03=<a> reliance-2026-09-30=<a> syrma-2026-09-30=<a>` | the probe (`a` is `yes`, `no` or `unknown`); Dated iff opened, yes, yes, not yes |
| `membership-carry-from <day\|->` | later days ask this day |
| `membership-listed <names>` | the universe |
| `membership-answer <name> <day> <yes\|no\|unknown>` | a stored answer (every listed name-day without one is `yes`) |
| `stock <name>` ... `end-stock` | one stock |
| `zerodha-months`, `gdfl-months`, `gdfl-refused` `<months\|->` | the months each vendor holds; a refused GDFL month exists and refuses to open |
| `withheld <days\|->` | days the fold withholds |
| `day <day> minutes=<n> seconds=<n>` | one stored stock-day, then its records |
| `m <hh:mm> <open> <high> <low> <close> <volume> [x<count>]` | a Zerodha minute (adjusted); `x<count>`: that many consecutive minutes, 60 s apart, with this bar; an off-grid stamp prints `hh:mm:ss` |
| `s <hh:mm:ss> <open> <high> <low> <close> <volume> [x<count> every <stride>]` | a GDFL second (raw; high and low the traded extremes); the repeat steps `stride` seconds |

**Expected outputs**

| line | meaning |
|---|---|
| `stock <name> span=<first>..<last>\|none warmup=<month\|none> refused=<reason\|none> fold-errors=<months\|-> membership-unknown=<n>` | F04, F05, the month refusals, the unknown stock-days |
| `fold <name> k=<k> holes=<n> outside-session=<n>` | per timeframe |
| `candidates <VO\|AV2\|VS> <k> count=<n>` | the (rule set, timeframe)'s candidates, in (day, T, symbol bytes) order, ids from 0 |
| `c <id> <day> <T> <symbol> <LONG\|SHORT> <not-filled\|no-ratio\|zero-quantity>` | a candidate that cannot fill |
| `c <id> <day> <T> <symbol> <side> filled <entry-second> <entry-price> <qty> <exit-second> <exit-price> <stop\|flip\|time\|square-off\|unclosable> <fallback 0\|1> <late 0\|1> <first-stop> <last-stop> <trail-no-ratio> <gross> <headline> <dated\|today> <net>` | a filled candidate (stops are raw on a GDFL day) |
| `f <id> dated-worst <brk> <ex+ipft> <sebi> <stt> <stamp> <gst> <dp> <dp-gst> <sq> <sq-gst> <total> today-worst <same eleven> dated-precise <total> today-precise <total>` | the charges of every candidate some row admits |
| `row <set> <k> <LONG\|SHORT\|BOTH> verdict=<PASS\|MISS> trades= net= net-bp= dd-bp= dd= dd-peak=<day\|start> dd-low=<day\|none> months= months-pass= months-positive= worst= median= best= floor= win-rate= avg-win= avg-loss= pf= charges=` | section 3.10's columns (`n/a` where a column has no value) |
| `counts signals= while-open= cut= not-filled= zero-qty= no-ratio= late= stop= flip= time= square-off= unclosable= slots-full= fallback-days= no-ratio-trail= cold=ST:n,BB:n,RSI:n,WR:n,CCI:n,EMA:n taint=...` | the row's counters (cold and taint count decisions on clean trading candles blocked per reading; they are per rule set and timeframe) |
| `admitted <ids\|->` | the row's admitted candidates |
| `month <month> capital= net= trades= bp=<n\|n/a> pass=<yes\|no> gap=<250 x capital - 10,000 x net, or ->` | every judged month |
| `rank <place> <set> <k> <account>` | F59's order over the 54 rows |
| `head <k> <account> <AV2/VO\|VS/VO\|VS/AV2> trades=<p> net=<p> dd-bp=<p> dd=<p>` | F60 per timeframe and account: `p` is the permille, floored, or `base:<n>` when the base is 0 or negative |
| `nearest-met-floor <bp> <set> <k> <account>` | F60: the highest floor (worst month bp) of any row, even a negative one; ties keep the first row in (rule set, timeframe, account) order |
| `run-stopped charge-refusal <symbol> <day> <error>` / `run-stopped duplicate-candidate <symbol> <day> <T>` | a run that stopped by name |

**The corpus in numbers:** 35,086 candidates (34,850 filled, 195 not filled, 34 zero quantity, 7 no ratio); exits
19,372 stop, 13,676 time, 1,523 unclosable, 186 square-off, 93 flip; 17,236 fee lines; 10,692 rows (198 runs x 54),
of which 36 PASS (the long rows of `s055`); 10,692 head lines and 198 nearest-met-floor lines; 2 runs stop by name
(`s061`, `s062`).

**Every filled golden trade's headline is the dated clock's** (34,850 of 34,850), and so is every trade the rest of
the suite prices (the 10,000 generated trades of `p_costs_hold_their_invariants` included: before the two tests below
were added, the coverage run measured the today branch of the headline at zero). That is a finding, not a gap. The
dated clock's rates are at or above today's on every date but for two lines, SEBI in 2020-06-01..2021-03-31 (Rs 5
against Rs 10 a crore) and GST on the SEBI fee before 2022-07-18 (none against 18%); before 2024-10-01 the dated
exchange rate is the higher one, and from 2024-10-01 the two clocks' exchange + IPFT rates sum to the same Rs 307 a
crore. The unit tests pin the two shapes found where today's clock is the dearer one, each by one paisa: a Rs 5,065
trade on 2020-09-01 (both exchange lines round to 16 paisa, today's SEBI line rounds to 1), and Rs 50 lakh of
turnover on 2025-01-15 (today's split lines round up twice, 15,351 against 15,350). Section 8 A26 pins the tie.

**The 200 scenarios** (`s001`..`s083` hand-planted, `s084`..`s200` seeded random stories; each file's header repeats
its seed and what it plants):

| scenario | plants |
|---|---|
| `s001-long-gap-drift-time-exit` | a jump up then a 1-paisa drift: long signals on every timeframe, time exits at 15:07:59 |
| `s002-short-gap-drift-time-exit` | the mirror: a jump down then a 1-paisa slide, short signals, time exits |
| `s003-long-flat-after-gap-stop-at-level` | a jump then a flat tape: the ratcheted stop reaches the price and fills at it (low <= stop) |
| `s004-long-gap-through-stop` | after the long entry a minute far below the stop: the stop fills in full at that second's low |
| `s005-short-gap-through-stop` | after the short entry a spike far above the stop: the stop fills at that second's high |
| `s006-long-flip-close-below-band` | Zerodha closes far below the lower band at 11:00 while GDFL stays above the stop: a flip, filled at the next second |
| `s007-short-flip-close-above-band` | the mirror: Zerodha closes far above the upper band, GDFL stays below the stop: a short flip |
| `s008-stop-and-flip-in-one-second` | the flip's fill second also reaches the stop: one exit at that print, counted as a flip |
| `s009-stop-and-time-exit-in-one-second` | the 15:07:59 second's low reaches the stop: the resting stop is taken (Q3 pin) |
| `s010-square-off-no-second-in-window` | no GDFL second from 15:07:59 to 15:09:59: the broker squares off at 15:10:00's worst print (by_broker) |
| `s011-flip-filled-at-square-off` | a flip decided before 15:08 whose first second is at 15:10:00: the broker's square-off (Q4 pin) |
| `s012-halted-afternoon-long-unclosable` | no GDFL second and padded minutes from 11:00: the long ends unclosable at its lowest low since entry |
| `s013-halted-afternoon-short-unclosable` | the mirror: a short halted from 11:00 ends at its highest high since entry |
| `s014-entry-is-the-last-second` | the entry fills on the day's last traded second: unclosable at that second's worst print |
| `s015-late-fill-over-60-seconds` | no GDFL second for two minutes after the signal: the entry fills 121 s late (counted late) |
| `s016-entry-not-filled` | the GDFL tape stops at the signal close: entry not filled, the stock stays flat |
| `s017-no-ratio-minute` | no GDFL second before the signal on that day: the stop cannot be mapped (no ratio), not taken |
| `s018-zero-quantity-above-5-lakh` | a Rs 6 lakh share: 5,00,000 / price = 0 shares, zero quantity |
| `s019-close-at-2pow32-accepted-above-refused` | price edges: a close of 2^32 paisa folds; one of 2^32 + 1 is refused, a hole that taints every reading |
| `s020-one-rupee-share` | a Rs 1 share with 1-paisa moves: 5,00,000 shares, every charge at its smallest |
| `s021-corrupt-candle-hole` | a minute with high below low: the candle is refused, a hole, and the jump right after it decides nothing |
| `s022-missing-minute-taints` | a missing minute just before the jump: a hole, then every reading tainted for its own period |
| `s023-padded-minutes-are-not-holes` | zero-volume padded minutes before the jump: no hole, no taint, the signal stands |
| `s024-one-candle-day` | a day holding only its 09:15 minute: one clean 1-minute candle, every other bucket a hole |
| `s025-signal-on-the-last-entry-minute` | the jump in the 15:06 minute: the 1-minute signal at T 15:07:00 enters, then the 15:07:59 order |
| `s026-cut-by-1508` | the jump in the 15:07 minute: the 1-minute signal at T 15:08:00 is cut by 15:08 |
| `s027-reentry-after-a-stop` | a jump, a stop-out on a dip, a second jump later: the stock re-enters with no cap |
| `s028-zero-volume-seconds-used-as-stored` | GDFL seconds with volume 0 (a Rule T break) are used as stored for the entry and the exits |
| `s029-split-day-ratio-5` | Zerodha adjusted, GDFL raw at 5 times: the stop maps through the ratio 5 and the quantity sizes from the raw fill |
| `s030-gdfl-month-missing-fallback` | GDFL holds September but not October: every October fill on Zerodha minutes (high/low, slot second = start + 59) |
| `s031-gdfl-month-refused-fallback` | the GDFL month refuses to open: a fallback day by reason |
| `s032-day-without-seconds-fallback` | Zerodha minutes but no GDFL second that day: a fallback day |
| `s033-fallback-unclosable` | a fallback day whose minutes are padded from 11:00: unclosable at the worst minute since entry |
| `s034-fallback-entry-on-the-1507-minute` | fallback entry at T 15:07:00 on the 15:07 minute (slot 15:07:59), then the time rule on 15:08 |
| `s035-vendor-late-day-2019-05-30` | 2019-05-30: every stock fills on minutes though GDFL seconds exist |
| `s036-fallback-padded-entry-minute` | fallback: the minutes after the signal are padded, the entry takes the next traded minute |
| `s037-outage-day-2021-02-24` | 2021-02-24 never trades; its candles still fold (the jump decides nothing) |
| `s038-holiday-muhurat-short-session` | a closed weekday, a Muhurat day and a short session hold minutes: none folds or trades; the day after reads the last full session |
| `s039-membership-unknown-day` | the folder answers unknown on the signal day: not traded, counted per stock |
| `s040-membership-no-day` | not a member on the signal day: not traded |
| `s041-probe-fails-todays-list` | the probe fails (SYRMA answers Ok(true)): today's list on every day, the dated 'no' is ignored |
| `s042-membership-carry-from` | days after the folder's last day ask that day: its 'no' blocks the signal day |
| `s043-withheld-day` | the fold withholds the signal day: every scheduled bucket a hole, no bar folded |
| `s044-fold-error-month` | one minute stamped 09:25:30: the whole month's fold refuses, every bucket a hole |
| `s045-auction-eligible-days-2026-08` | minutes end at 15:14 from 2026-08-03: 15:15 continuous close, 360-minute schedule |
| `s046-auction-switch-eve` | 2026-07-31 a full session, 2026-08-03 auction-eligible: both session shapes in one run |
| `s047-trading-start-month-absent` | the trading start month has no Zerodha month: refused for the stock (Q1 pin) |
| `s048-late-gdfl-start` | GDFL starts a month after Zerodha: the trading span starts at October's first session |
| `s049-zerodha-gap-warmup-from-contiguous-run` | a foreign Zerodha month before a gap is never folded: warm-up from the contiguous run |
| `s050-no-warmup-cold-ema` | no warm-up month: VIDEO-ORIGINAL trades once its three readings warm; AUTHOR-V2 and VIDEO-SCREEN wait for EMA 200 |
| `s051-twenty-slots-full-22-ties` | 22 stocks jump at one instant: 20 admitted in symbol order, 2 skipped 'slots full' |
| `s052-slots-full-then-while-open` | 20 open, then later signals: their own stocks while open, two newcomers slots full |
| `s053-both-view-long-then-short` | a long open, then a short signal on the same stock: BOTH counts it while open, SHORT takes it |
| `s054-symbol-byte-order-ties` | ties at one instant ordered by bytes: AB&C < AB&CD < AB-C < ABC |
| `s055-month-exactly-2-5000-percent-passes` | one long trade per long row, every timeframe: GDFL flat at 112360 from the jump, the 15:07:59 second at 123698: a net of exactly 5000000 paisa on the Rs 20,00,000 start |
| `s056-month-2-4999-percent-fails` | one long trade per long row, every timeframe: GDFL flat at 121906 from the jump, the 15:07:59 second at 134184: a net of exactly 4999800 paisa on the Rs 20,00,000 start |
| `s057-no-trade-months` | three judged months, trades in October only: September and November are 0 bp and fail |
| `s058-loss-month-then-recovery` | a gap-through loss in September and a gain in October: a drawdown with its peak and low days |
| `s059-account-at-or-below-zero` | 20 longs crash to one paisa: September loses more than the account, October starts at or below zero |
| `s060-equal-drawdowns-first-dates-kept` | two equal falls from one peak: the first one's dates are kept (Q9 pin) |
| `s061-charge-refusal-outside-cost-window` | a trade on 2018-08-07, before the cost table's 2018-09-03: the run stops by name |
| `s062-two-stocks-one-spelling` | two stocks spelled S01 with signals at one instant: the merge refuses the duplicate key by name |
| `s063-era-2018-09-03` | window start: SEBI Rs 15/cr, NSE Rs 325/cr, the worst state's stamp: a long trade priced on both clocks |
| `s064-era-2019-03-29` | the last day of SEBI Rs 15/cr: a short trade priced on both clocks |
| `s065-era-2019-04-01` | the first day of SEBI Rs 10/cr: a long trade priced on both clocks |
| `s066-era-2020-05-29` | the last day before the SEBI 50% relief: a short trade priced on both clocks |
| `s067-era-2020-06-01` | the first day of SEBI Rs 5/cr: a long trade priced on both clocks |
| `s068-era-2020-06-30` | the last day of the state stamp on both sides: a short trade priced on both clocks |
| `s069-era-2020-07-01` | the first day of 0.003% stamp on the buy: a long trade priced on both clocks |
| `s070-era-2020-12-31` | the last day of NSE Rs 325/cr: a short trade priced on both clocks |
| `s071-era-2021-01-01` | the first day of NSE Rs 345/cr: a long trade priced on both clocks |
| `s072-era-2021-04-01` | the first day of SEBI back at Rs 10/cr: a short trade priced on both clocks |
| `s073-era-2022-07-15` | the last trading day with no GST on the SEBI fee: a long trade priced on both clocks |
| `s074-era-2022-07-18` | the first day of GST on the SEBI fee: a short trade priced on both clocks |
| `s075-era-2023-03-31` | the last day of NSE Rs 345 and IPFT Rs 0.01: a long trade priced on both clocks |
| `s076-era-2023-04-03` | the first trading day of NSE Rs 325 and IPFT Rs 10: a short trade priced on both clocks |
| `s077-era-2024-03-28` | the last trading day of NSE Rs 325 + 10: a long trade priced on both clocks |
| `s078-era-2024-04-01` | the first day of NSE Rs 322 + 10: a short trade priced on both clocks |
| `s079-era-2024-09-30` | the last day of the slab ladders: a long trade priced on both clocks |
| `s080-era-2024-10-01` | the first day of NSE flat Rs 297 + 10: a short trade priced on both clocks |
| `s081-era-2026-02-27` | the last day of NSE Rs 297 + 10: a long trade priced on both clocks |
| `s082-era-2026-03-02` | the first trading day of NSE Rs 306.99 + 0.01: a short trade priced on both clocks |
| `s083-era-2026-10-30` | the cost table's last priced day: a long trade priced on both clocks (the window's last month) |
| `s084-random-2022-01` | seeded random story 0: 1 stock(s), 1 judged month(s) from 2022-01 |
| `s085-random-2023-05` | seeded random story 1: 1 stock(s), 1 judged month(s) from 2023-05 |
| `s086-random-2019-02` | seeded random story 2: 3 stock(s), 1 judged month(s) from 2019-02 |
| `s087-random-2026-09` | seeded random story 3: 2 stock(s), 2 judged month(s) from 2026-09 |
| `s088-random-2019-02` | seeded random story 4: 1 stock(s), 1 judged month(s) from 2019-02 |
| `s089-random-2020-03` | seeded random story 5: 2 stock(s), 1 judged month(s) from 2020-03 |
| `s090-random-2023-05` | seeded random story 6: 3 stock(s), 1 judged month(s) from 2023-05 |
| `s091-random-2026-04` | seeded random story 7: 3 stock(s), 1 judged month(s) from 2026-04 |
| `s092-random-2025-01` | seeded random story 8: 3 stock(s), 1 judged month(s) from 2025-01 |
| `s093-random-2022-09` | seeded random story 9: 1 stock(s), 2 judged month(s) from 2022-09 |
| `s094-random-2022-09` | seeded random story 10: 1 stock(s), 1 judged month(s) from 2022-09 |
| `s095-random-2022-09` | seeded random story 11: 3 stock(s), 2 judged month(s) from 2022-09 |
| `s096-random-2019-11` | seeded random story 12: 2 stock(s), 1 judged month(s) from 2019-11 |
| `s097-random-2021-06` | seeded random story 13: 2 stock(s), 1 judged month(s) from 2021-06 |
| `s098-random-2024-06` | seeded random story 14: 3 stock(s), 2 judged month(s) from 2024-06 |
| `s099-random-2020-03` | seeded random story 15: 1 stock(s), 1 judged month(s) from 2020-03 |
| `s100-random-2022-01` | seeded random story 16: 1 stock(s), 1 judged month(s) from 2022-01 |
| `s101-random-2022-09` | seeded random story 17: 1 stock(s), 2 judged month(s) from 2022-09 |
| `s102-random-2019-02` | seeded random story 18: 3 stock(s), 1 judged month(s) from 2019-02 |
| `s103-random-2026-09` | seeded random story 19: 2 stock(s), 1 judged month(s) from 2026-09 |
| `s104-random-2023-11` | seeded random story 20: 1 stock(s), 2 judged month(s) from 2023-11 |
| `s105-random-2025-09` | seeded random story 21: 3 stock(s), 2 judged month(s) from 2025-09 |
| `s106-random-2020-03` | seeded random story 22: 3 stock(s), 1 judged month(s) from 2020-03 |
| `s107-random-2025-01` | seeded random story 23: 2 stock(s), 1 judged month(s) from 2025-01 |
| `s108-random-2020-03` | seeded random story 24: 1 stock(s), 1 judged month(s) from 2020-03 |
| `s109-random-2025-09` | seeded random story 25: 3 stock(s), 1 judged month(s) from 2025-09 |
| `s110-random-2024-06` | seeded random story 26: 3 stock(s), 1 judged month(s) from 2024-06 |
| `s111-random-2020-08` | seeded random story 27: 1 stock(s), 2 judged month(s) from 2020-08 |
| `s112-random-2024-06` | seeded random story 28: 3 stock(s), 2 judged month(s) from 2024-06 |
| `s113-random-2019-11` | seeded random story 29: 2 stock(s), 1 judged month(s) from 2019-11 |
| `s114-random-2019-11` | seeded random story 30: 2 stock(s), 1 judged month(s) from 2019-11 |
| `s115-random-2019-11` | seeded random story 31: 2 stock(s), 1 judged month(s) from 2019-11 |
| `s116-random-2023-11` | seeded random story 32: 2 stock(s), 1 judged month(s) from 2023-11 |
| `s117-random-2022-09` | seeded random story 33: 3 stock(s), 2 judged month(s) from 2022-09 |
| `s118-random-2021-06` | seeded random story 34: 1 stock(s), 2 judged month(s) from 2021-06 |
| `s119-random-2023-05` | seeded random story 35: 3 stock(s), 2 judged month(s) from 2023-05 |
| `s120-random-2023-11` | seeded random story 36: 22 stock(s), 2 judged month(s) from 2023-11 |
| `s121-random-2025-01` | seeded random story 37: 3 stock(s), 1 judged month(s) from 2025-01 |
| `s122-random-2020-03` | seeded random story 38: 2 stock(s), 2 judged month(s) from 2020-03 |
| `s123-random-2025-09` | seeded random story 39: 3 stock(s), 1 judged month(s) from 2025-09 |
| `s124-random-2019-11` | seeded random story 40: 2 stock(s), 1 judged month(s) from 2019-11 |
| `s125-random-2019-02` | seeded random story 41: 22 stock(s), 1 judged month(s) from 2019-02 |
| `s126-random-2023-11` | seeded random story 42: 3 stock(s), 1 judged month(s) from 2023-11 |
| `s127-random-2020-08` | seeded random story 43: 3 stock(s), 1 judged month(s) from 2020-08 |
| `s128-random-2026-04` | seeded random story 44: 3 stock(s), 1 judged month(s) from 2026-04 |
| `s129-random-2019-11` | seeded random story 45: 3 stock(s), 1 judged month(s) from 2019-11 |
| `s130-random-2024-06` | seeded random story 46: 1 stock(s), 1 judged month(s) from 2024-06 |
| `s131-random-2020-08` | seeded random story 47: 2 stock(s), 2 judged month(s) from 2020-08 |
| `s132-random-2020-03` | seeded random story 48: 3 stock(s), 1 judged month(s) from 2020-03 |
| `s133-random-2024-06` | seeded random story 49: 1 stock(s), 1 judged month(s) from 2024-06 |
| `s134-random-2023-05` | seeded random story 50: 3 stock(s), 1 judged month(s) from 2023-05 |
| `s135-random-2025-09` | seeded random story 51: 1 stock(s), 2 judged month(s) from 2025-09 |
| `s136-random-2025-01` | seeded random story 52: 3 stock(s), 1 judged month(s) from 2025-01 |
| `s137-random-2026-04` | seeded random story 53: 3 stock(s), 1 judged month(s) from 2026-04 |
| `s138-random-2021-06` | seeded random story 54: 3 stock(s), 1 judged month(s) from 2021-06 |
| `s139-random-2020-03` | seeded random story 55: 2 stock(s), 2 judged month(s) from 2020-03 |
| `s140-random-2022-09` | seeded random story 56: 22 stock(s), 1 judged month(s) from 2022-09 |
| `s141-random-2018-09` | seeded random story 57: 1 stock(s), 1 judged month(s) from 2018-09 |
| `s142-random-2021-06` | seeded random story 58: 2 stock(s), 1 judged month(s) from 2021-06 |
| `s143-random-2020-03` | seeded random story 59: 1 stock(s), 1 judged month(s) from 2020-03 |
| `s144-random-2021-06` | seeded random story 60: 2 stock(s), 1 judged month(s) from 2021-06 |
| `s145-random-2021-06` | seeded random story 61: 2 stock(s), 1 judged month(s) from 2021-06 |
| `s146-random-2025-09` | seeded random story 62: 2 stock(s), 1 judged month(s) from 2025-09 |
| `s147-random-2022-01` | seeded random story 63: 1 stock(s), 1 judged month(s) from 2022-01 |
| `s148-random-2019-02` | seeded random story 64: 1 stock(s), 1 judged month(s) from 2019-02 |
| `s149-random-2025-01` | seeded random story 65: 1 stock(s), 1 judged month(s) from 2025-01 |
| `s150-random-2021-06` | seeded random story 66: 2 stock(s), 1 judged month(s) from 2021-06 |
| `s151-random-2018-09` | seeded random story 67: 2 stock(s), 1 judged month(s) from 2018-09 |
| `s152-random-2024-06` | seeded random story 68: 2 stock(s), 1 judged month(s) from 2024-06 |
| `s153-random-2022-01` | seeded random story 69: 3 stock(s), 1 judged month(s) from 2022-01 |
| `s154-random-2022-09` | seeded random story 70: 2 stock(s), 2 judged month(s) from 2022-09 |
| `s155-random-2021-06` | seeded random story 71: 1 stock(s), 1 judged month(s) from 2021-06 |
| `s156-random-2026-09` | seeded random story 72: 2 stock(s), 2 judged month(s) from 2026-09 |
| `s157-random-2023-11` | seeded random story 73: 1 stock(s), 1 judged month(s) from 2023-11 |
| `s158-random-2026-04` | seeded random story 74: 2 stock(s), 1 judged month(s) from 2026-04 |
| `s159-random-2024-06` | seeded random story 75: 1 stock(s), 1 judged month(s) from 2024-06 |
| `s160-random-2025-01` | seeded random story 76: 2 stock(s), 1 judged month(s) from 2025-01 |
| `s161-random-2025-09` | seeded random story 77: 1 stock(s), 1 judged month(s) from 2025-09 |
| `s162-random-2022-09` | seeded random story 78: 2 stock(s), 1 judged month(s) from 2022-09 |
| `s163-random-2025-01` | seeded random story 79: 22 stock(s), 1 judged month(s) from 2025-01 |
| `s164-random-2023-11` | seeded random story 80: 2 stock(s), 2 judged month(s) from 2023-11 |
| `s165-random-2023-11` | seeded random story 81: 2 stock(s), 1 judged month(s) from 2023-11 |
| `s166-random-2020-08` | seeded random story 82: 1 stock(s), 1 judged month(s) from 2020-08 |
| `s167-random-2022-09` | seeded random story 83: 3 stock(s), 1 judged month(s) from 2022-09 |
| `s168-random-2019-02` | seeded random story 84: 3 stock(s), 2 judged month(s) from 2019-02 |
| `s169-random-2024-06` | seeded random story 85: 1 stock(s), 1 judged month(s) from 2024-06 |
| `s170-random-2021-06` | seeded random story 86: 1 stock(s), 1 judged month(s) from 2021-06 |
| `s171-random-2020-03` | seeded random story 87: 3 stock(s), 1 judged month(s) from 2020-03 |
| `s172-random-2021-06` | seeded random story 88: 3 stock(s), 1 judged month(s) from 2021-06 |
| `s173-random-2021-06` | seeded random story 89: 1 stock(s), 2 judged month(s) from 2021-06 |
| `s174-random-2023-11` | seeded random story 90: 3 stock(s), 1 judged month(s) from 2023-11 |
| `s175-random-2025-01` | seeded random story 91: 2 stock(s), 1 judged month(s) from 2025-01 |
| `s176-random-2023-05` | seeded random story 92: 3 stock(s), 1 judged month(s) from 2023-05 |
| `s177-random-2026-09` | seeded random story 93: 2 stock(s), 1 judged month(s) from 2026-09 |
| `s178-random-2024-06` | seeded random story 94: 2 stock(s), 2 judged month(s) from 2024-06 |
| `s179-random-2026-04` | seeded random story 95: 3 stock(s), 2 judged month(s) from 2026-04 |
| `s180-random-2018-09` | seeded random story 96: 1 stock(s), 1 judged month(s) from 2018-09 |
| `s181-random-2024-06` | seeded random story 97: 1 stock(s), 2 judged month(s) from 2024-06 |
| `s182-random-2022-01` | seeded random story 98: 3 stock(s), 2 judged month(s) from 2022-01 |
| `s183-random-2019-11` | seeded random story 99: 2 stock(s), 1 judged month(s) from 2019-11 |
| `s184-random-2026-09` | seeded random story 100: 3 stock(s), 1 judged month(s) from 2026-09 |
| `s185-random-2023-11` | seeded random story 101: 1 stock(s), 1 judged month(s) from 2023-11 |
| `s186-random-2022-09` | seeded random story 102: 1 stock(s), 1 judged month(s) from 2022-09 |
| `s187-random-2026-04` | seeded random story 103: 1 stock(s), 1 judged month(s) from 2026-04 |
| `s188-random-2026-04` | seeded random story 104: 3 stock(s), 1 judged month(s) from 2026-04 |
| `s189-random-2023-11` | seeded random story 105: 22 stock(s), 1 judged month(s) from 2023-11 |
| `s190-random-2022-01` | seeded random story 106: 1 stock(s), 1 judged month(s) from 2022-01 |
| `s191-random-2019-11` | seeded random story 107: 1 stock(s), 1 judged month(s) from 2019-11 |
| `s192-random-2023-05` | seeded random story 108: 2 stock(s), 2 judged month(s) from 2023-05 |
| `s193-random-2023-11` | seeded random story 109: 2 stock(s), 1 judged month(s) from 2023-11 |
| `s194-random-2026-09` | seeded random story 110: 3 stock(s), 1 judged month(s) from 2026-09 |
| `s195-random-2025-09` | seeded random story 111: 1 stock(s), 1 judged month(s) from 2025-09 |
| `s196-random-2021-06` | seeded random story 112: 3 stock(s), 1 judged month(s) from 2021-06 |
| `s197-random-2019-11` | seeded random story 113: 2 stock(s), 1 judged month(s) from 2019-11 |
| `s198-random-2019-02` | seeded random story 114: 1 stock(s), 1 judged month(s) from 2019-02 |
| `s199-random-2019-02` | seeded random story 115: 2 stock(s), 1 judged month(s) from 2019-02 |
| `s200-random-2020-03` | seeded random story 116: 1 stock(s), 1 judged month(s) from 2020-03 |

## 8. Every place the spec was ambiguous, and the reading taken

The Mac's tool pins TEST-PLAN's Q1..Q10 to its first run's code, which this session never saw; the pins below are
this model's own readings of the text, each stated so a difference in the comparison can be traced to it.

**Readings (the indicator states)**

- **A1. The readings' definitions** come from the cited design texts (section 3), because PREVIEW-SPEC cites the
  part tips' code lines for them. Every fixture in those texts holds exactly here.
- **A2. The ATR's state is scaled by 10^6** (like the EMA and the RSI averages): seed `floor(10^6 x sum / n)` of the
  first `n` true ranges (the first candle's own high-low span), then `atr += (tr x 10^6 - atr).div_euclid(n)`; the
  bands read it floored to whole paisa, `band = (atr x factor).div_euclid(1000)`. The texts say "Wilder seed = mean
  of the first n, then 1/n smoothing, floored, value() in whole paisa" without saying whether the state is scaled;
  ST-1..ST-9 hold under either reading, so they do not decide it. A whole-paisa state can differ by a paisa of band
  after a long run.
- **A3. The bar's mid** is `(high + low).div_euclid(2)` (prices are positive, so this is `i64::midpoint`).
- **A4. The candle check** (`check_evaluable`, not restated in the kit): every price above 0, `low <= high`, open and
  close inside the range, volume not negative. A refused candle, or a close above 2^32 paisa, folds into none of the
  seven states and is a hole.
- **A5. EMA 200**: the running mean of the scaled closes while fewer than 200 have folded, then
  `ema += ((close x 10^6 - ema) x 2).div_euclid(201)`; C8 compares `close x 10^6` with it.

**Candles, holes, days**

- **A6. A padded minute is any minute with volume 0** (flat or not): stored, so never missing, and never folded.
- **A7. The fold refuses a whole month** for a minute off the whole-minute grid or a stamp repeated or out of order
  (FoldError: every scheduled bucket of the month a hole). A minute outside 09:15..close is withheld and counted, not a
  refusal.
- **A8. Only full sessions are scheduled and folded.** Minutes stored on a Muhurat day, a short session or a closed day
  are never folded (withheld); the outage day 2021-02-24 is a full session that folds and never trades.
- **A9. A hole that holds a bar folds into every state** (its minutes are real trades) and still taints; a hole with
  no bar folds nothing; both restart every taint count; a reading is untainted again on its period-th clean candle.
- **A10. The trailing rule ignores taint**: only a hole candle moves nothing and fires no flip (the spec's trailing
  paragraph names holes only; taint gates entry decisions).
- **A11. The previous session** is the immediately preceding full session of the candle span; its day bar is the
  fold of that day's in-session traded minutes (09:15 to the close; a pre-open or after-close minute never enters it);
  no bar, a refused bar or a withheld day reads nothing (Q2 pinned: C1 and C2 false all day).
- **A12. The day mask asks membership only on full sessions inside the window**; days before the window (warm-up) are
  never asked. "Never before 2018-09-03" is read as "never before the window's first session", for any feed start,
  however early. The October carry is modelled as a `carry_from` day (October 2026 asks 2026-09-30).

**Fills and exits**

- **A13. Trailing candles** are those whose close instant is after the entry fill second and at or before 15:07:59; a
  candle that closed before the fill moves nothing. After the tape ends (a halt) the candles still trail the stop.
- **A14. Q3: the stop and the time order on one second: the stop** (the resting order was live). The same at the
  square-off second.
- **A15. Q4: a flip whose first traded unit is at or after 15:10:00 is the broker's square-off** (`by_broker`, Rs 20 +
  GST), by the time order's own rule that an order unfilled before 15:10:00 is squared off.
- **A16. Q5: the unclosable price includes the entry print**; **the unclosable exit second is the session's
  continuous close** (15:30:00, or 15:15:00 on an auction-eligible day), so the position holds its slot to the end
  of the day and is marked at the close.
- **A17. An entry at or after 15:07:59 gets no time order; its stop stays live** until the square-off.
- **A18. "Late"** compares the fill's recorded second (on a fallback day, the minute's start + 59) with T + 60 s.
- **A19. On a fallback day "at or after 15:07:59" is read on the minute's start**, so a 15:07 entry minute takes the
  time order on the 15:08 minute (X-F38-9).
- **A20. Phase A's order of refusals:** entry not filled, then no ratio (the entry stop cannot be mapped, or maps to a
  level at or below 0), then zero quantity. A later band that cannot be mapped leaves the stop and counts
  `no-ratio-trail`.
- **A21. The ratio's Z minute is an in-session minute** (start at or after 09:15) with volume above 0.
- **A22. Re-entry**: a stock may enter again whenever it is flat, any number of times a day.
- **A23. "Stop and target in one second"** (the brief's words): this recipe has no target; it is planted as the stop
  and the flip in one second (s008) and the stop and the time exit in one second (s009).
- **A33. Q7: a GDFL second with volume 0 is used as stored**: a print at its stored high and low for the entry, the
  stop and every exit, and a G for the ratio (s028). This model keeps no counter of such seconds; the golden inputs
  list every one, so a count is derivable.

**Book, months, costs**

- **A24. Q6: the slot counters' order:** signal while open first, then unfillable (holds no slot, counted by its kind),
  then slots full. `signals` counts every signal candle of the view, cut ones included; `late`, the exits by kind,
  `fallback-days` and `no-ratio-trail` count admitted trades only.
- **A25. Months at or below zero capital** have no bp (`n/a`), fail, and are left out of worst / median / best.
- **A26. Q8: equal clocks: the headline is the dated clock's.**
- **A27. Q9: equal deepest falls keep the first's dates;** the drawdown is read on the marks after each exit; the
  day-end marks repeat the day's last exit mark (positions never span a night) and change nothing. The peak day of the
  start is printed `start`.
- **A28. The state stamp before 2020-07-01** is the highest of every listed state on the day's contract-note value,
  Himachal's flat Rs 50 a day included; its worst reading is the larger of the two-decimal figure and its rupee
  rounding, without per-order rupee rounding (the reading that fits CN01..CN06 exactly).
- **A29. DP's worst reading** is Rs 12.50 + Rs 2.25 on every trade (each trade its own note, SPEC 4 item 13); GST on
  DP and on the square-off fee is 18% rounded once.
- **A30. Worst readings** taken as COSTS 6 lists them: brokerage max(per order, per fill); exchange + IPFT max(one
  line, split lines, per order); SEBI max(one line, per order); STT max(the VWAP basis, the sell value, per sell
  order), each to two decimals then to the rupee; stamp from 2020-07-01 max(the day, per buy order); GST max(18% once,
  9% + 9% each rounded) on the worst lines' base.
- **A31. The rank's win rate** of a row with no trade is 0; the floor is the worst month's bp.
- **A32. Rule sets** read their Supertrend by factor: VIDEO-ORIGINAL 10/3.000, AUTHOR-V2 and VIDEO-SCREEN 10/2.000;
  the gate checks warmth and taint, the conditions check settled (C3), faithful (C5) and flat (C6, C7).

**The head (F60)**

- **A34. Q10: VIDEO-SCREEN is printed over both**: over VIDEO-ORIGINAL (TEST-PLAN T-F60-1's reading) and over
  AUTHOR-V2 (the question's other reading), after AUTHOR-V2 over VIDEO-ORIGINAL (PREVIEW-SPEC 3.10). The head's
  side-by-side values (trades, net, max drawdown, months >= +2.5%, worst month) are each row's own columns in its
  `row` line and are not printed twice.
- **A35. "Max drawdown" in the head is compared in both of its readings** (section 3.9 prints it in bp and in rupees):
  `dd-bp` and `dd` (paisa). A drawdown of 0 is a zero base. A zero or negative base prints `base:<n>`, never divided.
- **A36. "The nearest met floor over all rows"** is the highest floor (worst month bp) of any row, negative or not
  (X-F60-8), printed with its row; ties keep the first row in (rule set, timeframe, account) order; a run whose rows
  have no floor prints `none`.

## 9. Outside the crate, and what is left to the Mac

**Three workspace gate tests fail on this branch, because the workspace now has a 14th member.** Measured here with
`cargo test -p core --tests` and `cargo test -p vocab --tests`. Nothing outside the crate, this report's folder, the
root `Cargo.toml` member line and `Cargo.lock` was edited, because each gate is a law of `CLAUDE.md` whose change
needs a `docs/05-decisions.md` entry, which is the Mac's to write:

| gate | message (left = this branch, right = the pin) | what closes it |
|---|---|---|
| `crates/core/tests/graph.rs` `the_manifest_list_is_the_whole_workspace` | members {13 crates + `psrb_preview_ref`} vs MANIFESTS {13 crates} | add the crate to MANIFESTS and to `docs/01-architecture.md` section 1 (it depends on nothing) |
| `crates/vocab/tests/workspace_is_rust.rs` `no_declared_native_dependency_is_compiled_any_more` | roots 14 vs 13 ("CLAUDE.md section 5 names thirteen") | the root count, and `CLAUDE.md` section 5's "Thirteen members" and graph block |
| `crates/vocab/tests/workspace_is_rust.rs` `the_dependency_set_has_not_moved_without_review` | 195 vs 194 packages | re-pin after the review the gate asks for (the new package is this crate itself and ships no other language) |

Every other test of `core` (205 unit tests, the 14 other tests of `graph.rs`, and 8 more integration files) and of
`vocab` (70 unit tests, the 16 other tests of `workspace_is_rust.rs`, and 10 more integration files, `no_py_language`,
`no_float` and `no_runtime_spawn` among them) passed; `no_py_language` was run again once the golden files and this
report were staged. The other eleven crates' suites were not run here: nothing in them depends on this crate, and a
search of their tests for `ls-files`, `members = [` and `Cargo.lock` found no other reader of the workspace's shape.

**Coverage at the Mac's bar (Route B) is the Mac's to take.** The figure in section 5 is cargo-llvm-cov at the
workspace's own test profile (`opt-level = 3`, overflow checks on), over the crate's 37 library files with code
(`lib.rs` holds only module lines and has no region); its report lists no `*_tests.rs` module and not
`benches/ratio.rs`. TEST-PLAN section 6 sets every `.rs` file, tests and bench included, at opt-level 0, with the
(F) exclusions for failing `assert!` arms; that run was not made here.

**Also not done:** `cargo deny check` and `cargo test --workspace --locked` (the `CLAUDE.md` section 9 items beyond
the crate's own) were not run; no `docs/04-invariants.md` or `docs/05-decisions.md` entry was written; mutants were
not run (deferred).

## 10. Honesty notes

- Early in the session one command printed the version of the interpreter the rules forbid, while listing the host's
  tools by mistake. Nothing ran with it, nothing was written by it, and no tracked file names or uses it.
- One in-place edit of a file this crate owns was made with a stock text tool (`perl -pi`) instead of the editor; the
  file is plain Rust and was re-checked by `rustfmt`, clippy and the tests like every other.
- The bench and coverage figures are this container's, measured on the final code; nothing in this report is
  extrapolated.
- No file was deleted that this session did not create. The golden files were regenerated several times while the
  catalogue was built, each time from this session's own untracked files; the last generation before the head lines
  was moved aside and compared with the final one, and only the head lines differ.

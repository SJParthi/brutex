# TEST-PLAN: the PSRB preview tool, hardened to the full standing standard (PLAN ONLY)

Written Fri 6 Nov 2026 ~09:1x IST by a workflow step (follow-up of wf_391708cd-84c, main session 641cf96a) for the
builder and the checker of the HARDENED preview tool. This step is text only: nothing was built, run, committed,
moved or deleted. The standard is the operator's standing block (memory `standing-principles-o1-extreme`, pasted
again Fri 6 Nov ~09:0x IST): 100% test coverage with every permutation, combination and extreme case (out of the box
too); O(1) even at p99 everywhere; everything parallel; everything saved in a database with auditing, logging,
tracking, debugging, searching and monitoring; one common, dynamic, incremental, scalable design.

**Read whole for this plan:** `PREVIEW-SPEC.md` (428 lines, sha256 `24162ba257ec125a1562a916d0d2e4ef8243ce3abb745b0d74492dfcd4723e30`,
the 08:54 IST version; cited below as SPEC §n), `OPERATOR-ADDENDUM-20261106.md` (the third rule set VIDEO-SCREEN; cited
ADD), `build-progress.md`, `W/state/decisions-20261027/d3-recipe.txt` (v4; cited D3 step n), the head of
`d3-route-b.sh`, `crates/indicators/benches/ratio.rs` and `crates/telemetry/benches/ratio.rs` (the project's ratio-bench
form), and at B = `c8f7608b5cbc` the public cost API (`TwoClock`, `NoteLines { brokerage, exchange, sebi, stt, stamp,
gst, dp, dp_gst, square_off, square_off_gst }`, `ChargeError`). W = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`,
PV = `W/state/psrb-preview-20261106`.

**If the spec changes again:** the builder re-hashes PREVIEW-SPEC.md and ADD before the first commit; on a new sha it
diffs the two versions and updates the affected F-sections' tests FIRST (a changed rule is a changed test before it is
changed code), and records the diff's line numbers in PV/BUILD-NOTES-HARDEN.md.

---

## 0. When, where, and how this never disturbs the first run

1. **Start gate (all must hold, checked by one shell line, never guessed):** `PV/PREVIEW-RESULTS.md` exists and is
   non-empty; `PV/BUILD-NOTES.md` names the first run's scratch tree and tool path; the first run's output folder
   (`PV/run/` per the workflow, `PV/out-1/` per SPEC §5; whichever BUILD-NOTES.md names) holds `identity.txt` and
   `REPORT.md`; and `pgrep -f psrb-preview-1` prints nothing (no build, run or checker process of the first run still
   works). The checker's `PV/check<k>.md` is not waited for; each of its findings, when it lands, becomes a named
   regression test in the F-section it concerns (id `X-Fnn-Rk`) BEFORE the fix.
2. **Tree:** the next free `n >= 2`: `git -C /Volumes/WD_BLACK/brutex/fresh-20260919/project worktree add --detach
   W/scratch/psrb-preview-<n> c8f7608b5cbc5383f294728e4eeb0d1208a31e97`; never an existing path. Then COPY (cp, never
   mv) the first run's `crates/psrb_preview/`, the five indicator modules and their `_tests.rs`, the five `pub mod`
   lines, the `scaled()` getter and the members line from the first run's tree; record each copied file's sha256 equal
   to the source. One local commit inside this detached tree is allowed (never a branch): its sha is "the tool's
   commit" in every audit line. Later hardening commits stack in the same detached tree.
3. **Target dirs:** `W/tgt/psrb-preview-<n>` (release build, benches, tests) and `W/tgt/psrb-preview-<n>-deep` as
   Route B's K (its build goes to `K/cov`; section 6). No other target dir; never `tgt/deep-*`, `tgt/pool-*`, `tgt/dbfix*`.
4. **Cargo only through the queue:** `PATH=W/bin:$PATH` (the `*psrb-preview*` priority line,
   `W/state/cargo-priority.txt`:37); the nightly only through D3 step 0's `nc()`.
5. **Every Bash call blocks at most 270 s:** builds, tests, coverage, benches and runs start as
   `nohup ( <cmd> ; echo "EXIT $?" ) > <log> 2>&1 &`, then waits `until grep -q '^EXIT ' <log>; do sleep 20; done`
   (timeout 270000), repeated with no status command or text between waits.
6. **Disk floor** before every build, run, bench and db write: `free KiB - 52,428,800 >= 1,074,218,750` (`df -k
   /Volumes/WD_BLACK`); the tool checks it itself too (F75).
7. **Read-only:** the real store and `runtime/` are opened read-only (`open_existing`, `load_span`); E-7 proves no
   byte, mtime or new file changes there. Nothing ever deletes a file: staging files, killed units' partial files and
   bench folders stay and are reported (F71 verify lists them).
8. **Rule sets are data** (build-progress.md, ADD): one `RuleSet` table each for VIDEO-ORIGINAL (VO), AUTHOR-V2 (AV2)
   and VIDEO-SCREEN (VS). VS = AV2 with the C6 levels mirrored (long above -15, short below -85). Rows = rule sets x 6
   timeframes x 3 accounts = 54 (36 if a run is limited to the spec's two). Every count below is per the three tables.

**First-run parity (the strongest single test, E-9):** the hardened tool's full run on the real store must reproduce
the first run's `rows.tsv`, `months.tsv`, `trades.tsv` and `counts.tsv` byte for byte. Every difference is either a
bug of the first run, fixed with a named regression test that shows it, or a bug of the hardened tool; never waved
through. The diff and each explanation go to PV/BUILD-NOTES-HARDEN.md.

## 0.1 Conventions used by every test below

- **IDs.** `Fnn` function; `T-Fnn-k` normal or permutation test; `X-Fnn-k` extreme case, tagged **(combo)** when it
  combines several extremes at once and **(oob)** when it is out of the box; `P-Fnn-k` property test against a naive
  brute-force model; `E-k` end-to-end; `B-k` bench; `T-G-k` a global gate. The snake_case word after the id is the
  Rust test function's name. Unit tests live in `crates/psrb_preview/src/<module>_tests.rs` (the project's form),
  integration tests in `crates/psrb_preview/tests/<area>.rs`, the naive models in `tests/model/<module>.rs`.
- **Exact integers only.** Money in paisa (`i64` stored, `i128` for products and sums), percent in basis points,
  ratios in permille; no `f32`/`f64` anywhere (T-G-01). Times are UTC micros in storage, compared against IST
  instants converted once (IST has no DST; X-F08-8).
- **Fixtures.** `day(y,m,d)`, `ist(hh,mm,ss)`; prices written as rupees in the text and paisa in code
  (Rs 1,250.00 = 125_000).
- **Generator.** SplitMix64 (the form of `crates/vocab/tests/mask.rs`:102), seeds S1..S4 =
  `0x5053_5242_0000_0001..=0x5053_5242_0000_0004`. A property runs 4 x 2,500 = **10,000 cases** unless its row says
  otherwise; a failure prints seed, case index and the input shrunk by a fixed rule (halve each field toward its
  minimum while it still fails, at most 64 steps). No randomness without a printed seed.
- **Injected seams** (production code, reachable by input, so covered and never (U)): a `Clock` trait (monotonic for
  rates, wall IST for stamps); a `FreeSpace` trait (the disk floor); a `Writer` trait that can fail at byte n; the
  fault hook `PSRB_PREVIEW_FAULT=<point>:<n>:<exit|pause>` (section 4).
- **Spec questions** the spec leaves open are listed in section 7 (Q1..Q8). Each is PINNED to what the first run's
  tool does (read from its code and output, recorded), tested as pinned, and flagged for the spec owner; never decided
  silently and never a skip rule.

---

## 1. FUNCTIONS: every unit of the recipe as a named pure function or streaming state

Kind: **pure** = a function of its arguments; **stream** = a fixed-size state with an O(1) `step`; **io** = bounded
reads or appends. "O(1)" is per call (per minute, candle, second, candidate, trade, mark, row or event as named), proved
at p99 by the bench in the last column (section 3); an honest non-O(1) bound is stated where it exists.

| F | name | kind | what it computes (SPEC source) | per-call bound | bench |
|---|---|---|---|---|---|
| F01 | `universe::load` | pure | the names of fno218.txt, byte order, ordinals, ICICILIFE -> ICICIPRULI series, `symbol_id` (§3.3) | O(names) once | B-27 |
| F02 | `membership::probe` | io | `Membership::open` + the three probe answers -> Dated(digest) or TodaysList (§3.3) | O(1) | B-27 |
| F03 | `membership::may_trade` | pure | `is_member(name, D)`: Yes / No / Unknown; October 2026 asks 2026-09-30 (§3.3) | O(1) | B-01 |
| F04 | `span::trading_span` | pure | later of first Zerodha and first GDFL month, never before 2018-09-03, to the window end (§3.3) | O(months) once | B-27 |
| F05 | `span::warmup_start` | pure | first month of the contiguous Zerodha run holding the trading start (§3.3) | O(months) once | B-27 |
| F06 | `daymask::admit` | pure | M1..M5, 2021-02-24 never trades (still folds), 2019-05-30 fallback day (§3.3) | O(1) | B-01 |
| F07 | `candles::fold_timeframe` | stream | one month of 1-minute bars -> k-minute candles via `derive_minutes`, `PADDED_FOLD = Absent` (§3.4) | O(1) per minute | B-02 |
| F08 | `candles::close_instant` | pure | T = bucket start + 60k s, clipped to the continuous close (§3.4) | O(1) | B-02 |
| F09 | `candles::auction_eligible` | pure | from 2026-08-03: eligible iff the stock-day's last stored minute <= 15:14 (§3.4) | O(1) | B-02 |
| F10 | `candles::HoleDetector` | stream | hole = a scheduled minute missing, a withheld day, or a refused candle; padded is not missing (§3.4) | O(1) per minute | B-03 |
| F11 | `dayref::PrevSession` | stream | the day bar at `Bucket::DAY`, `DailyReference::new`, the previous session's H/L/C for C1/C2 (§3.4) | O(1) per day | B-03 |
| F12 | `readings::Bank::step` | stream | one candle into all seven reading states or none (refusal = hole) (§3.1 Folding) | O(1) per candle | B-04 |
| F13 | `readings::SupertrendPair` | stream | `DualBandSuperTrend` 10/2_000 and 10/3_000: settled, trend, lower, upper | O(1) per candle | B-04 |
| F14 | `readings::Bollinger20` | stream | one `Bollinger::new(20)` window read at widths 2_000 and 3_000, population sigma | O(1) per candle | B-04 |
| F15 | `readings::Rsi14` | stream | `Rsi::new(14)`, faithful, levels in ppm | O(1) per candle | B-04 |
| F16 | `readings::WilliamsR14` | stream | `WilliamsR::new(14)`, flat, LEVEL comparisons | O(1) per candle | B-04 |
| F17 | `readings::Cci20` | stream | `Cci::new(20)`, flat, levels in milli | O(1) per candle | B-04 |
| F18 | `readings::Ema200` | stream | `trend::Ema::new(200)`, warm, `scaled()` (scratch insertion) | O(1) per candle | B-04 |
| F19 | `readings::Taint` | stream | per reading: tainted until its own period of clean candles after a hole (10, 20, 14, 14, 20, 200) (§3.4) | O(1) per candle | B-05 |
| F20 | `cond::c1_narrow_cpr` | pure | `10_000 x |2C-H-L| <= 20 x T3`, i128, ties narrow | O(1) | B-06 |
| F21 | `cond::c2_beyond_r1_s1` | pure | long `3c > 2T3 - 3L`, short `3c < 2T3 - 3H` | O(1) | B-06 |
| F22 | `cond::c3_supertrend_side` | pure | settled and trend Up (long) / Down (short); 10/3 for VO, 10/2 for AV2 and VS | O(1) | B-06 |
| F23 | `cond::c4_bollinger_beyond` | pure | above upper / below lower at the rule set's width | O(1) | B-06 |
| F24 | `cond::c5_rsi_beyond` | pure | faithful and above 700_000 ppm / below 300_000 ppm | O(1) | B-06 |
| F25 | `cond::c6_willr_level` | pure | not flat; AV2 long above -85 / short below -15; VS long above -15 / short below -85 | O(1) | B-06 |
| F26 | `cond::c7_cci_beyond` | pure | not flat; above +200_000 / below -200_000 milli | O(1) | B-06 |
| F27 | `cond::c8_ema_side` | pure | warm and `close x SCALE >` / `<` `ema.scaled()` | O(1) | B-06 |
| F28 | `signal::ready_gate` | pure | every reading the rule set uses is warm and untainted; the candle is in the trading span and not a hole | O(1) | B-06 |
| F29 | `signal::evaluate` | pure | the rule set's conjunction per side from its table; never both sides | O(conditions) = O(1), at most 8 | B-06 |
| F30 | `fills::entry` | io | T < 15:08:00 else cut; `first_at_or_after(T)` in the stock-day; worst print; not filled; late > 60 s (§3.5) | O(1), 2 reads | B-07 |
| F31 | `pricemap::ratio_source` | io | Z (last Zerodha minute, volume > 0, ending <= T) and G (last GDFL second inside it), else the latest earlier minute with both | O(1) amortised; p99 proved | B-08 |
| F32 | `pricemap::map_level` | pure | `L_g = L_z x G / Z` in i128, floor for a long's stop, ceil for a short's | O(1) | B-08 |
| F33 | `trail::Trailing` | stream | per closed candle <= 15:07:59: flip if close across its own band, else tighten only, never back; hole moves nothing | O(1) per candle | B-09 |
| F34 | `fills::StopScan` | stream | live from the first traded second after the entry second; long `low <= stop` fills at low, short `high >= stop` at high | O(1) per second | B-10 |
| F35 | `fills::flip` | io | first traded second >= T_t, worst print; the same second reaching the stop is one exit, a flip | O(1) | B-11 |
| F36 | `fills::time_exit` | io | order at 15:07:59, fills in [15:07:59, 15:10:00), worst print | O(1) | B-11 |
| F37 | `fills::square_off` | io | first second >= 15:10:00, worst print, `by_broker`; none: unclosable at the worst print since entry | O(1) | B-11 |
| F38 | `fills::MinuteFallback` | stream | a fallback day's fills on Zerodha minutes (volume > 0), slot second = minute start + 59 | O(1) per minute | B-10 |
| F39 | `trade::simulate` | stream | composes F30..F38 into one candidate `(T, symbol, side, entry, exit, kind, fallback)`, portfolio-free | O(1) per candle and per second | B-12 |
| F40 | `sizing::qty` | pure | `50_000_000.div_euclid(entry_fill)`; 0 = zero quantity (§3.6) | O(1) | B-13 |
| F41 | `costs::trade_fills` | pure | `TradeFills` (entry order, exit order, one `Fill` each, `by_broker` on square-off/unclosable), `TradeDay` (§3.7) | O(1) | B-13 |
| F42 | `costs::line_brokerage` | pure | BRK-1 min(Rs 20, 0.03%) per executed order, RD-BRK | O(1) | B-13 |
| F43 | `costs::line_square_off` | pure | BRK-SQ Rs 20 + GST 18%, RD-SQ, only on `by_broker` | O(1) | B-13 |
| F44 | `costs::line_dp` | pure | BRK-DP Rs 12.50 + GST, RD-DP, per trade | O(1) | B-13 |
| F45 | `costs::line_exchange_ipft` | pure | EX-2009..EX-2026 and IP-OLD..IP-2026 by date | O(1) | B-13 |
| F46 | `costs::line_sebi` | pure | SF-2017..SF-2021 by date | O(1) | B-13 |
| F47 | `costs::line_stt` | pure | STT 0.025% of the sell side, RD-STT-BASE | O(1) | B-13 |
| F48 | `costs::line_stamp` | pure | state levy both sides to 2020-06-30 (worst state, capped per trade), 0.003% of the buy from 2020-07-01 | O(1) | B-13 |
| F49 | `costs::line_gst` | pure | GST 18% on brokerage + exchange + IPFT; on SEBI from 2022-07-18 | O(1) | B-13 |
| F50 | `costs::rounding` | pure | RD-ROUND-TAX, RD-ROUND-EX, RD-GST at the worst level | O(1) | B-13 |
| F51 | `costs::headline` | pure | max(dated worst, today worst), `headline_from` | O(1) | B-13 |
| F52 | `costs::trade_net` | pure | gross - headline (`TwoClock::net`); a `ChargeError` stops the run by name | O(1) | B-13 |
| F53 | `book::merge_candidates` | stream | k-way merge of every stock's candidates by (T, symbol byte order); LONG, SHORT, BOTH views | O(log s), s = stocks (bounded) | B-14 |
| F54 | `book::SlotAllocator` | stream | 20 slots; ignore while open; skip when full; slot held T..exit second inclusive (§3.6) | O(1) per candidate | B-15 |
| F55 | `account::MonthAgg` | stream | exit-day attribution, compounding capital start, `month_bp` div_euclid, exact +2.5% test (§3.8) | O(1) per trade | B-16 |
| F56 | `account::DrawdownWalker` | stream | marks after each exit (second, symbol) and day end; peak incl. start; `max_dd_bp`, Rs, dates (§3.9) | O(1) per mark | B-17 |
| F57 | `report::row_stats` | stream | every §3.10 column (win rate, averages, profit factor, lower median, counters) | O(1) per trade; median O(m) once per row, m = months (stated) | B-16 |
| F58 | `verdict::bar` | pure | PASS iff every month >= +2.5%; else MISS with each month's gap and the floor = worst month | O(m) once per row (stated) | none (row-level) |
| F59 | `verdict::rank` | pure | closest-miss order of §3.10 plus a final deterministic tie (rule set, timeframe, account) | O(r log r) once, r = rows (stated) | none (run-level) |
| F60 | `report::head_compare` | pure | AV2 and VS over VO in permille (trades, net, drawdown); zero/negative base printed as such; nearest met floor | O(r) once (stated) | none (run-level) |
| F61 | `report::rupees` | pure | paisa -> rupees with two decimals by integer division, sign, Indian grouping as the first run prints | O(digits) <= 40 | B-26 |
| F62 | `run::plan_units` | pure | the unit list (stock x timeframe x rule set x month) and the row units, in one deterministic order | O(units) once | B-24 |
| F63 | `run::WorkerPool` | io | one stock per worker on every core; deterministic results for any worker count; a panic stops the run by name | O(1) per unit dispatch | B-25 |
| F64 | `db::commit_unit` | io | atomic, content-addressed write: create_new staging, fsync, hard_link into place, never overwrite | O(unit bytes) | B-18 |
| F65 | `db::Manifest` | io | append-only rows (key, path, bytes, sha256, row crc); a torn tail is closed and skipped, never rewritten | O(1) per row | B-19 |
| F66 | `db::resume` | io | the committed-unit bitmap from the manifest; skip committed units; recompute in-flight ones; commit each once | O(1) per unit check; O(rows) load once (stated) | B-20 |
| F67 | `db::Snapshot` | io | month-end state per (stock, timeframe) and per row; add-a-month resumes from it | O(1) per snapshot (fixed size) | B-21 |
| F68 | `db::DenseIndex` | io | mixed-radix slot of (run, rule set, timeframe, side/account, stock or ALL, month) -> fixed-width record | O(1) per lookup | B-22 |
| F69 | `db::Audit` | io | one hash-chained line per run event (inputs' sha256, tool commit, start, end, counts, refusals, fallback fills) | O(1) per event | B-23 |
| F70 | `cmd::query` | io | by run, stock, month, timeframe, rule set (any subset); a full key is one index read | O(1) per full key; O(matches) otherwise | B-22 |
| F71 | `cmd::verify` | io | re-hash every manifest file; MISMATCH / MISSING / ORPHAN / TORN; exit code | O(bytes) (inherent, stated); O(1) per file lookup | B-26 |
| F72 | `monitor::Progress` | stream | one line a minute: units done/total, rate, ETA (integers), busy workers, refusals, fallback fills | O(1) per event and per tick | B-23 |
| F73 | `monitor::Extremes` | stream | fixed-k running extremes: worst months, deepest drawdowns, most holes, slowest units (p50/p99) | O(log k), k fixed = O(1) | B-23 |
| F74 | `selfcheck::run` | io | CN01 costs (313.25 / 225.79), membership probe, CPR tie, EMA fixture, 125 buckets; stops by name | O(1) | none (once) |
| F75 | `cli::parse_and_guard` | io | arguments, refusals (out exists, store not readable, disk floor, any write outside --out/--db), identity.txt | O(args) once | none (once) |
| F76 | `report::write_outputs` | io | REPORT.md, rows.tsv, months.tsv, trades.tsv, counts.tsv, identity.txt: PREVIEW line + APPROXIMATE list, deterministic bytes | O(1) per line | B-26 |

**76 functions.** The indicator modules themselves (copied from the five part tips) keep their own parts' tests; F13..F18
test the tool's USE of them and re-verify each against an independent naive model (P rows), so a defect in a copied
module is caught here too and filed to its part.

---

## 2. TESTS per function

Every function has: its normal cases (T), every permutation and combination of its conditions that can differ (T rows
marked PERM, enumerated exhaustively inside the one named test, the case count given), at least 8 extreme cases (X),
2 or more of them **(combo)** and 1 or more **(oob)**, and at least one property test (P) against a naive brute-force
model on generated data with the fixed seeds of 0.1. Each row says what the test asserts.

### 2.0 Global gates (whole crate)

| id | test | asserts |
|---|---|---|
| T-G-1 no_float_token_anywhere | no `f32`, `f64`, `as f`, float literal or `float` lint allow in `crates/psrb_preview/**` (a grep test over the crate's files) and clippy `float_arithmetic`, `float_cmp` deny |
| T-G-2 no_as_casts_no_indexing | the workspace lints `as_conversions` and `indexing_slicing` are deny for the crate and no `allow` of either appears |
| T-G-3 no_delete_no_overwrite_api | no `remove_file`, `remove_dir`, `remove_dir_all`, `fs::rename`, `File::create` (truncating) or `OpenOptions::truncate(true)` token in the crate; every file is opened by `db::create_new` (O_EXCL) or `append(true)` |
| T-G-4 store_opened_read_only_only | every store access goes through `BarFile::open_existing` / `load_span`; no store path reaches `db::create_new` (a type test: `StoreRoot` has no write method; a grep test for `OpenOptions` near `StoreRoot`) |
| T-G-5 only_rust_toml_md_tsv_files | the crate holds only `.rs`, `.toml`, `.md`, `.tsv`, `.json` fixture files: glue is shell, jq and node outside the crate |
| T-G-6 one_common_path | no per-rule-set, per-timeframe or per-side copy of logic: the three rule sets, six timeframes and two sides run one code path driven by tables (a grep test: no `VIDEO_ORIGINAL`/`AUTHOR_V2`/`VIDEO_SCREEN` token outside `ruleset.rs` and tests) |
| T-G-7 every_counter_reaches_counts_tsv | every counter named in SPEC §3.10 "also" and every refusal kind has a row in counts.tsv (a table test over the counter enum: no counter without an output column) |
| T-G-8 deterministic_bytes_twice | two runs of the E-1 store produce byte-equal outputs and unit files (clock pinned) |

### F01 `universe::load`

| id | test | asserts |
|---|---|---|
| T-F01-1 loads_real_list_in_byte_order | the real fno218.txt (sha recorded) gives 218 names; ordinals 0..217 follow byte order; each `symbol_id` equals `cli::stored`'s own `fnv1a` id |
| T-F01-2 icicilife_reads_icicipruli_series | ICICILIFE is reported as ICICILIFE and its store series is ICICIPRULI; no other alias exists |
| T-F01-3 ordinal_is_byte_order_not_file_order | a shuffled copy of the list gives identical ordinals and identical run outputs |
| T-F01-4 count_is_runtime_not_fixed | a 3-name list loads 3 names and every row/units count follows from it (no constant 218 in code) |
| X-F01-1 duplicate_name_refused | a repeated name refuses by name, line numbers of both |
| X-F01-2 inner_empty_line_refused | an empty line inside the file refuses; exactly one final newline is accepted; none at the end is accepted |
| X-F01-3 lowercase_or_space_refused | `reliance`, ` RELIANCE`, `RELIANCE ` each refuse by line |
| X-F01-4 byte_order_with_ampersand_and_dash | `M&M` < `M&MFIN` < `MANAPPURAM` and `BAJAJ-AUTO` < `BAJAJFINSV` (0x26 < 0x2D < 0x41) |
| X-F01-5 empty_file_refused | zero names refuse ("empty universe"), never a run of nothing |
| X-F01-6 missing_or_directory_refused | a missing path and a directory path each refuse by name |
| X-F01-7 (combo) alias_and_target_both_listed | ICICILIFE and ICICIPRULI both listed refuse: two names on one series |
| X-F01-8 (combo) id_collision_with_duplicate_spelling | a forced 32-bit id collision (test hasher seam) between two distinct names AND a third duplicated line: both refusals are listed, collision first |
| X-F01-9 (oob) bom_crlf_and_non_utf8 | a UTF-8 BOM, CRLF endings and a 0xFF byte each refuse by name (no silent strip) |
| P-F01-1 load_vs_btreeset_model | 10,000 generated lists (A-Z, 0-9, `&`, `-`, lengths 1..20, with planted duplicates): ordinals equal a `BTreeSet` model; a refusal happens exactly when the model sees a duplicate |

### F02 `membership::probe`

| id | test | asserts |
|---|---|---|
| T-F02-1 real_folder_is_dated | the real pit-membership folder: RELIANCE 2018-09-03 `Ok(true)`, RELIANCE 2026-09-30 `Ok(true)`, SYRMA 2026-09-30 not `Ok(true)` -> Dated with its `file_digest_hex` |
| T-F02-2 PERM probe_answers_27 | every triple of answers from {Ok(true), Ok(false), Err} (27 cases): Dated only for (Ok(true), Ok(true), not Ok(true)); every other case TodaysList with each failing probe named |
| T-F02-3 open_refusal_falls_back | `Membership::open` refusing gives TodaysList and the label "survivorship: today's list" in every output |
| T-F02-4 survivorship_label_in_both_modes | the label "survivorship: only today's 218 names can trade ..." is printed in Dated and in TodaysList mode |
| X-F02-1 empty_folder | an empty folder falls back with its reason |
| X-F02-2 only_one_month_file | a folder with 2018-09 only: the 2026-09-30 probe refuses -> fallback |
| X-F02-3 syrma_err_keeps_dated | SYRMA answering Err counts as "not Ok(true)": Dated stands |
| X-F02-4 path_is_a_file | a file path falls back by name, no panic |
| X-F02-5 permission_denied | an unreadable folder (chmod 000 on a NEW test folder; never on the real one) falls back by name |
| X-F02-6 (combo) one_truncated_file_and_syrma_true | a truncated 2026-09 file and SYRMA Ok(true) at once: one fallback, both reasons in order |
| X-F02-7 (combo) all_three_wrong | all three probes wrong: one fallback, three reasons, the run continues on today's list |
| X-F02-8 (oob) symlink_loop_folder | a folder that is a symlink loop falls back by name in bounded time (deadline 5 s) |
| X-F02-9 digest_recorded_in_identity | identity.txt carries the digest in Dated mode and the words "today's list" in fallback |
| P-F02-1 probe_vs_truth_table | 10,000 generated answer triples and open outcomes: mode and reasons equal a truth-table model |

### F03 `membership::may_trade`

| id | test | asserts |
|---|---|---|
| T-F03-1 dated_yes_and_no | RELIANCE 2018-09-03 Yes; SYRMA 2026-09-30 No |
| T-F03-2 october_asks_sept_30 | every 2026-10 day asks 2026-09-30 and returns its answer |
| T-F03-3 todays_list_mode_yes_for_listed | in TodaysList mode every listed name is Yes on every window day |
| T-F03-4 refusal_is_unknown_and_counted | an `is_member` refusal returns Unknown, the stock-day does not trade, `membership unknown` counts +1 for that stock |
| X-F03-1 first_carry_day | 2026-10-01 asks 2026-09-30 |
| X-F03-2 last_window_day | 2026-10-30 asks 2026-09-30 |
| X-F03-3 day_outside_window_refused | 2026-11-02 and 2018-08-31 refuse by name (never asked of the folder) |
| X-F03-4 name_not_listed | a name outside the universe is No in both modes |
| X-F03-5 holiday_never_asked | the day mask asks membership only on M2 days: a holiday adds no Unknown |
| X-F03-6 (combo) carry_day_and_refusal | 2026-09-30 refusing for one stock makes every October day Unknown for that stock, counted per stock-day |
| X-F03-7 (combo) leap_day_and_month_boundary | 2020-02-29 and 2020-03-01 each read their own month file |
| X-F03-8 (oob) file_month_present_but_day_missing | a month file lacking one day answers Unknown for that day only |
| P-F03-1 may_trade_vs_map_model | 10,000 generated (name, day) pairs incl. October days against a `BTreeMap<(name, day), answer>` model with the carry rule |

### F04 `span::trading_span`

| id | test | asserts |
|---|---|---|
| T-F04-1 common_name | Zerodha 2018-01, GDFL 2018-09 -> 2018-09-03 .. 2026-10-30 |
| T-F04-2 irfc_and_tataconsum | IRFC (2018-03, 2021-01) starts at 2021-01's first session; TATACONSUM (2018-01, 2020-02) at 2020-02's |
| T-F04-3 PERM first_months_9 | Zerodha first month before / equal / after GDFL's x the later one before / at / after 2018-09 (9 cases): start = later month's first M2 session, never before 2018-09-03 |
| X-F04-1 no_gdfl_months | None, counted "no trading span" |
| X-F04-2 no_zerodha_months | None, counted |
| X-F04-3 gdfl_starts_after_window | None |
| X-F04-4 one_month_span | both first months = 2026-10: span 2026-10-01 .. 2026-10-30 (its M2 days) |
| X-F04-5 first_session_not_first_day | 2018-09 starts at 2018-09-03 (first M2 day), not 09-01 |
| X-F04-6 (combo) internal_gap_both_feeds | FORCEMOT's 2023-11..2024-01 gap does not change the span; those days fail M5 |
| X-F04-7 (combo) late_listing_and_early_end | ICICIPRULI's GDFL ending 2026-10-21 and no Zerodha 2026-10: span end stays 2026-10-30, the days after its data fail M5 |
| X-F04-8 (oob) unsorted_or_duplicate_months | a month list out of order or with one month twice refuses by name |
| P-F04-1 span_vs_month_set_model | 10,000 generated month sets: span equals a model that scans every window day |

### F05 `span::warmup_start`

| id | test | asserts |
|---|---|---|
| T-F05-1 jan_2018_names | 2018-01..2018-09 contiguous: warm-up from 2018-01 (167 sessions, 62,625 minutes for RELIANCE) |
| T-F05-2 irfc_foreign_minutes_skipped | IRFC's 2018-03 minutes before a gap of years are never folded |
| T-F05-3 kfintech_foreign_minutes_skipped | KFINTECH's 2018-05 minutes are never folded |
| X-F05-1 no_warmup_month | trading start month is the run's first month: no warm-up; entries only once every reading is warm |
| X-F05-2 one_month_gap_inside_run | a gap at 2018-05 starts the warm-up at 2018-06 |
| X-F05-3 gap_just_before_start | a gap at 2018-08 for a 2018-01 name: warm-up starts 2018-09 |
| X-F05-4 trading_start_month_absent | the trading start month has no Zerodha month: pinned (Q1), refuses by name for that stock, counted |
| X-F05-5 single_month_run | a run of one month |
| X-F05-6 (combo) many_runs_start_in_last | four runs separated by gaps: the last run holding the start is used |
| X-F05-7 (combo) gap_and_late_gdfl | a Zerodha gap plus GDFL starting later than Zerodha |
| X-F05-8 (oob) duplicate_generation_months | two served generations listed for one month refuse by name |
| P-F05-1 warmup_vs_walk_back_model | 10,000 generated month sets: equals a model that walks back month by month from the start |

### F06 `daymask::admit`

| id | test | asserts |
|---|---|---|
| T-F06-1 ordinary_day_trades | a regular full session with every check true -> Trade |
| T-F06-2 outage_day_never_trades_but_folds | 2021-02-24 -> NoTrade("skip row"), and its stored candles still feed every reading |
| T-F06-3 vendor_late_day_is_fallback | 2019-05-30 -> TradeFallback for every stock |
| T-F06-4 PERM m1_to_m5_x_day_kind | 2^5 masks of M1..M5 x {ordinary, 2021-02-24, 2019-05-30} (96 cases): Trade only for all-true on an ordinary day, TradeFallback only for all-true on 2019-05-30; every failing check listed in fixed order M1..M5 |
| X-F06-1 muhurat_day | `OpenLengthUnmeasured` -> NoTrade |
| X-F06-2 closed_day_with_stored_minutes | a closed day holding stored minutes -> NoTrade (counted as a contract note) |
| X-F06-3 first_and_last_window_days | 2018-09-03 and 2026-10-30 trade |
| X-F06-4 day_after_window | 2026-11-02 -> NoTrade(M4) |
| X-F06-5 auction_switch_day | 2026-08-03 trades with the 360-minute session |
| X-F06-6 (combo) fallback_day_and_unknown_member | 2019-05-30 with membership Unknown -> NoTrade (M3 wins over fallback) |
| X-F06-7 (combo) outage_day_outside_span | 2021-02-24 for a stock whose span starts 2021-03: both reasons listed |
| X-F06-8 (oob) skip_rows_found_by_content | the skip file with its rows reordered: the two market-wide rows are found by content; either missing refuses by name |
| X-F06-9 (oob) short_special_session | a day whose `kind_of` is open but not `Session::full()` -> NoTrade |
| P-F06-1 admit_vs_truth_table | 10,000 generated stock-days: verdict and reasons equal a truth-table model |

### F07 `candles::fold_timeframe`

| id | test | asserts |
|---|---|---|
| T-F07-1 reliance_3min_125_buckets | one RELIANCE month: 125 three-minute buckets per full session (the self-check) |
| T-F07-2 bucket_counts_every_k | full session 375/125/75/25/13/7 and auction-eligible 360/120/72/24/12/6 for k = 1/3/5/15/30/60 |
| T-F07-3 ohlcv_equals_derive_minutes | each candle equals `derive_minutes`'s bar (open first, high max, low min, close last, volume sum) |
| T-F07-4 PERM k_x_daykind_x_padded_x_stub | 6 k x {full, auction-eligible} x {padded minutes, none} x {stub bucket, none} (48 cases): counts and bars as the model says |
| X-F07-1 only_padded_bucket_has_no_bar | a bucket of only padded minutes yields no bar and is not a hole |
| X-F07-2 fold_error_month | a `FoldError` month gives no candles, counted, all its scheduled buckets holes |
| X-F07-3 one_minute_month | a month with one stored minute |
| X-F07-4 day_with_one_candle | one stored minute at 09:15: k=1 gives one clean candle; k=60 gives one candle that is a hole (missing minutes) |
| X-F07-5 stub_buckets | 15:15-15:30 stub for k=30 and k=60 folds, closes after 15:08, never enters |
| X-F07-6 (combo) eligible_day_padded_last_minute_60min | auction-eligible day + padded 15:14 minute + k=60 |
| X-F07-7 (combo) month_crossing_auction_switch | July 31 full and Aug 3 eligible in one fold call: both session shapes right |
| X-F07-8 (oob) off_grid_or_duplicate_stamp | a minute stamped 09:15:30 or twice: the fold's refusal makes the month holes, counted by name |
| X-F07-9 pre_open_minute | a 09:00 minute is withheld or refused exactly as `derive_minutes` does; never folded into 09:15 |
| P-F07-1 fold_vs_bucket_grouping_model | 10,000 generated month-days (gaps, padded minutes, both day kinds, every k): candles equal a naive grouping model |

### F08 `candles::close_instant`

| id | test | asserts |
|---|---|---|
| T-F08-1 ordinary | 09:15 k=3 -> 09:18:00 |
| T-F08-2 clipped_at_close | 15:00 k=30 -> 15:30; 15:15 stub k=30 -> 15:30 |
| T-F08-3 eligible_day_clip | auction-eligible 15:00 k=30 -> 15:15 |
| X-F08-1 sixty_minute_stub | 15:15 k=60 -> 15:30 |
| X-F08-2 bucket_at_close_refused | a bucket starting at the close refuses |
| X-F08-3 last_entry_minute | k=1 bucket 15:06 -> T 15:07:00 (< 15:08: may enter) |
| X-F08-4 first_cut_minute | k=1 bucket 15:07 -> T 15:08:00 (cut by 15:08) |
| X-F08-5 last_one_minute | k=1 bucket 15:29 -> 15:30 |
| X-F08-6 (combo) three_minute_cut | k=3 bucket 15:06 -> T 15:09 (cut) |
| X-F08-7 (combo) eligible_day_fifteen | auction-eligible k=15 bucket 15:00 -> 15:15 |
| X-F08-8 (oob) utc_ist_boundary | 09:15 IST = 03:45 UTC; an IST 05:30 (UTC midnight) stamp refuses as off-session; no DST anywhere |
| P-F08-1 close_vs_minute_walk_model | 10,000 generated buckets: equals a model that walks minute by minute to the close |

### F09 `candles::auction_eligible`

| id | test | asserts |
|---|---|---|
| T-F09-1 before_switch_not_required | days before 2026-08-03 -> None |
| T-F09-2 eligible | 2026-08-03 last stored minute 15:14 -> Some(true) |
| T-F09-3 not_eligible | last stored minute 15:29 -> Some(false) |
| X-F09-1 exactly_1514 | 15:14 -> true |
| X-F09-2 exactly_1515 | 15:15 -> false |
| X-F09-3 day_without_minutes | None, and M5 fails |
| X-F09-4 padded_last_minute_counts | a padded 15:29 minute is stored, so false |
| X-F09-5 switch_eve | 2026-07-31 -> None |
| X-F09-6 (combo) eligible_with_one_minute | 2026-08-03 with only 09:15: true; the 360-minute schedule makes the rest holes |
| X-F09-7 (combo) padded_1514_on_eligible_day | true, and the 15:14 padded bucket has no bar |
| X-F09-8 (oob) minutes_after_1530 | minutes stored after 15:30: false, counted as a contract break |
| P-F09-1 eligible_vs_model | 10,000 generated stock-days |

### F10 `candles::HoleDetector`

| id | test | asserts |
|---|---|---|
| T-F10-1 clean_bucket | no hole |
| T-F10-2 one_missing_minute | hole |
| T-F10-3 withheld_day | every bucket of that day a hole |
| T-F10-4 PERM missing_padded_withheld_refused | 2^4 = 16 cases: hole iff missing or withheld or refused; padded alone never |
| X-F10-1 no_bar_and_missing_minute | a scheduled bucket with no bar and a missing minute is a hole at its place in the stream |
| X-F10-2 all_padded_no_hole | an all-padded bucket: no bar, no hole |
| X-F10-3 days_before_candle_span | never scheduled, never holes |
| X-F10-4 holiday_not_scheduled | a non-M2 day schedules nothing |
| X-F10-5 outage_day_folds_clean | 2021-02-24's stored candles are not holes; the 15:45-16:59 window is not scheduled |
| X-F10-6 (combo) missing_and_refused_same_bucket | counted once, one hole |
| X-F10-7 (combo) whole_months_missing | FORCEMOT 2023-11..2024-01: every scheduled bucket a hole; every reading tainted afterwards |
| X-F10-8 (oob) duplicate_minute_record | the fold refuses, the day is withheld, all its buckets holes, counted by name |
| P-F10-1 holes_vs_set_difference_model | 10,000 generated stock-months: holes equal (scheduled minutes minus stored minutes) per bucket plus withheld and refused |

### F11 `dayref::PrevSession`

| id | test | asserts |
|---|---|---|
| T-F11-1 day_bar_hlc | the day bar's H, L, C are the session high, low and the last traded minute's close |
| T-F11-2 reads_previous_session | C1/C2 on D read D-1's accepted bar |
| T-F11-3 first_day_false | the first day of the candle span has no previous bar: C1 and C2 false all day |
| X-F11-1 refused_bar_no_earlier_read | a refused previous bar: false all day, and the bar two sessions back is NOT read |
| X-F11-2 padded_only_previous_day | the previous session has no day bar: pinned (Q2), false all day |
| X-F11-3 weekend_gap | Monday reads Friday |
| X-F11-4 muhurat_skipped | a Muhurat day is not M2: the next day reads the last M2 day |
| X-F11-5 outage_day_bar_used | 2021-02-25 reads 2021-02-24's bar built from its stored minutes |
| X-F11-6 (combo) split_day_adjusted_basis | across a 1:5 split the day bar and the candles share Zerodha's adjusted basis |
| X-F11-7 (combo) previous_is_fallback_day | a fallback (no GDFL) day still gives a valid Zerodha day bar |
| X-F11-8 (oob) high_below_low_bar | a stored bar with H < L or C outside [L, H] is refused by `DailyReference::new`: false all day, counted |
| P-F11-1 prev_session_vs_backward_scan | 10,000 generated day sequences: equals a model scanning back one M2 session |

### F12 `readings::Bank::step`

| id | test | asserts |
|---|---|---|
| T-F12-1 folds_into_all_seven | a valid candle advances ST 10/2, ST 10/3, BB20, RSI14, WR14, CCI20, EMA200: each state's count +1 |
| T-F12-2 shared_by_rule_sets | the three rule sets read the same states (one fold per stock and timeframe) |
| T-F12-3 PERM refusal_sources | {none, check_evaluable, Close::new} x {first candle, mid-stream} (6 cases): fold into all or none |
| X-F12-1 check_evaluable_refusal | high < low: folded into none, a hole |
| X-F12-2 close_above_2_pow_32 | `2^32 + 1` paisa: none, not even EMA |
| X-F12-3 close_at_2_pow_32 | `2^32` paisa is accepted |
| X-F12-4 i64_max_close | refused, a hole |
| X-F12-5 zero_or_negative_price | refused as `check_evaluable` does |
| X-F12-6 (combo) refusal_and_hole_same_candle | one hole counted |
| X-F12-7 (combo) refusal_on_first_candle | every reading stays cold; warm-up count unchanged |
| X-F12-8 (oob) atomic_no_partial_fold | a candle that only the 5th state would refuse: the bank pre-checks, so the first four never fold it |
| P-F12-1 bank_vs_naive_states | 10,000 generated streams with planted refusals: every state equals its naive model fed the same accepted candles |

### F13 `readings::SupertrendPair`

| id | test | asserts |
|---|---|---|
| T-F13-1 fixture_bands | a hand-written 30-candle fixture: lower, upper, trend and settled equal the hand values for 10/2 and 10/3 |
| T-F13-2 states_independent | the two multipliers never share state |
| T-F13-3 settled_after_atr_warm | settled exactly when the STV tip says (fixture) |
| X-F13-1 flat_series | ATR 0: bands at the close, no flip |
| X-F13-2 close_equal_band | equality does not flip (the tip's rule) |
| X-F13-3 gap_up_20_percent | flips Up on that candle |
| X-F13-4 alternating_spikes | 1,000 alternating candles: bounded state, no drift |
| X-F13-5 first_candle | not settled |
| X-F13-6 (combo) flip_and_settle_same_candle | both recorded |
| X-F13-7 (combo) one_flips_other_not | 10/2 flips while 10/3 does not |
| X-F13-8 (oob) thousand_flat_then_spike | 1,000 high = low = close candles then a spike: no overflow, trend right |
| P-F13-1 supertrend_vs_naive_full_history | 10,000 generated series: equals a naive model recomputing ATR (Wilder) from all history in exact i128 rationals |

### F14 `readings::Bollinger20`

| id | test | asserts |
|---|---|---|
| T-F14-1 fixture_window | 20-candle fixture: mean, population sigma, both bands as hand values |
| T-F14-2 two_widths_one_state | widths 2_000 and 3_000 read the same window |
| T-F14-3 warm_at_20 | warm only with 20 folded |
| X-F14-1 sigma_zero | flat window: above/below exactly as the BOLL tip defines |
| X-F14-2 close_on_band | on the band is not above (strict) |
| X-F14-3 nineteen_candles | not warm |
| X-F14-4 isqrt_edges | sigma at perfect squares and one below |
| X-F14-5 max_close_window | 20 closes of `2^32` paisa: i128 sums, no overflow |
| X-F14-6 (combo) above_2000_below_3000 | above the 2_000 band and inside the 3_000 band: VO true, AV2 and VS false |
| X-F14-7 (combo) max_close_and_sigma_zero | both at once |
| X-F14-8 (oob) alternating_1_and_2_pow_32 | alternating 1 paisa and `2^32` paisa: exact, no panic |
| P-F14-1 bollinger_vs_exact_square_model | 10,000 windows: above/below equal a model comparing `(20c - S)^2` against `w^2 x n x variance` in i128 with no square root |

### F15 `readings::Rsi14`

| id | test | asserts |
|---|---|---|
| T-F15-1 fixture_values | hand fixture RSI values |
| T-F15-2 faithful_rule | faithful exactly when the RSI tip says |
| T-F15-3 PERM levels | {faithful, not} x {above 70, at 70, between, at 30, below 30} (10 cases) |
| X-F15-1 all_gains | RSI 100: above 70 |
| X-F15-2 all_losses | RSI 0: below 30 |
| X-F15-3 flat | no gain or loss: as the tip defines (not faithful) |
| X-F15-4 exactly_70 | not above |
| X-F15-5 exactly_30 | not below |
| X-F15-6 (combo) faithful_and_crossing_same_candle | decided on that candle |
| X-F15-7 (combo) taint_after_hole_and_level | above 70 while tainted: blocked |
| X-F15-8 (oob) one_paisa_oscillation_100k | 100,000 candles of 1-paisa steps: no accumulator drift |
| P-F15-1 rsi_vs_wilder_rational_model | 10,000 series vs a naive Wilder model in exact rationals |

### F16 `readings::WilliamsR14`

| id | test | asserts |
|---|---|---|
| T-F16-1 fixture_values | hand fixture HH, LL, %R |
| T-F16-2 flat_false_both_sides | HH = LL: C6 false both sides |
| T-F16-3 PERM region_x_side_x_form | {flat, not} x 7 regions (C = HH, (-15, 0), -15, (-85, -15), -85, (-100, -85), C = LL) x 2 sides x {AV2, VS} (56 cases) |
| X-F16-1 close_at_hh | %R 0: AV2 long true, AV2 short false; VS long true, VS short false |
| X-F16-2 close_at_ll | %R -100: AV2 short true, AV2 long false; VS short true, VS long false |
| X-F16-3 boundary_minus_85 | `100(HH-C) = 85(HH-LL)`: not strictly above or below |
| X-F16-4 boundary_minus_15 | `100(HH-C) = 15(HH-LL)` |
| X-F16-5 one_paisa_range | HH - LL = 1 paisa |
| X-F16-6 (combo) flat_at_boundary | flat beats every level |
| X-F16-7 (combo) av2_vs_vs_disagree | %R = -50: AV2 long true, VS long false, same candle |
| X-F16-8 (oob) fourteen_identical_then_one_tick | exact |
| P-F16-1 willr_vs_scan_model | 10,000 windows vs a naive max/min scan over 14 |

### F17 `readings::Cci20`

| id | test | asserts |
|---|---|---|
| T-F17-1 fixture_values | hand fixture typical price, mean deviation, CCI |
| T-F17-2 flat_false | D = 0: C7 false both sides |
| T-F17-3 PERM region_x_side | {flat, not} x {> +200, = +200, between, = -200, < -200} x 2 sides (20 cases) |
| X-F17-1 exactly_plus_200 | `n x dev = 3 x D`: not above |
| X-F17-2 exactly_minus_200 | not below |
| X-F17-3 huge_outlier | one candle 1,000x: exact |
| X-F17-4 nineteen_candles | not warm |
| X-F17-5 tiny_deviation | D = 1 paisa |
| X-F17-6 (combo) flat_and_boundary | flat wins |
| X-F17-7 (combo) tiny_d_huge_dev | no overflow in i128 |
| X-F17-8 (oob) nineteen_equal_one_differs | exact sign |
| P-F17-1 cci_vs_mean_deviation_model | 10,000 windows vs an exact rational model |

### F18 `readings::Ema200`

| id | test | asserts |
|---|---|---|
| T-F18-1 ad_fixture | AD:572-578: 199 closes of 101 then 100: short true, long false |
| T-F18-2 warm_at_200 | warm exactly at 200 |
| T-F18-3 scaled_equals_state | `scaled()` returns the internal scaled value unchanged |
| X-F18-1 equal_scaled | `close x SCALE = scaled`: neither side |
| X-F18-2 large_close | i128 product, no overflow |
| X-F18-3 one_hundred_ninety_nine | not warm |
| X-F18-4 first_candle | cold |
| X-F18-5 constant_million | 10^6 identical closes: exact, no drift |
| X-F18-6 (combo) warm_on_signal_candle | warm and decided on the same candle |
| X-F18-7 (combo) taint_200_after_hole | blocked for 200 clean candles after a hole |
| X-F18-8 (oob) getter_only_in_scratch | the `scaled()` insertion is byte-identical to AD:197-211 and nothing else in trend.rs changed (diff test) |
| P-F18-1 ema_vs_direct_formula_model | 10,000 series vs an independent formula with the same rounding rule |

### F19 `readings::Taint`

| id | test | asserts |
|---|---|---|
| T-F19-1 periods | after a hole: ST 10, BB 20, RSI 14, WR 14, CCI 20, EMA 200 clean candles each |
| T-F19-2 rule_set_uses_its_own | VO gates on ST/BB/RSI only; AV2 and VS on all six |
| T-F19-3 counter_per_reading | "decisions blocked by taint" counts per reading |
| X-F19-1 second_hole_restarts | two holes 5 apart restart from the second |
| X-F19-2 hole_before_warm | cold and tainted at once |
| X-F19-3 refused_is_hole_not_clean | a refused candle is a hole and not a clean candle |
| X-F19-4 hole_with_bars_folded_not_clean | folded, still not clean |
| X-F19-5 exactly_period_clean | clean on the p-th candle exactly, not p-1 |
| X-F19-6 (combo) period_minus_one_then_hole | restarts |
| X-F19-7 (combo) ema_only_blocks | AV2 blocked, VO allowed, same candle |
| X-F19-8 (oob) every_other_candle_a_hole | a month never untaints; counters equal the model |
| P-F19-1 taint_vs_lookback_model | 10,000 streams vs a model looking back p candles for a hole |

### F20 `cond::c1_narrow_cpr` (AV2, VS)

| id | test | asserts |
|---|---|---|
| T-F20-1 centred_close_narrow | H 101.00, L 99.00, C 100.00: `|2C-H-L| = 0` -> true |
| T-F20-2 wide_day_not_narrow | H 110.00, L 90.00, C 108.00 -> false |
| T-F20-3 same_both_sides | the answer never depends on the side (PERM over both sides, 2 cases per fixture) |
| T-F20-4 self_check_tie | AD:536-543's tie fixture: narrow at its threshold, not one step inside it (the run's self-check) |
| X-F20-1 tie_exactly_20bp | `10_000 x |2C-H-L| = 20 x T3` -> true (ties narrow) |
| X-F20-2 one_unit_over | `10_000 x |2C-H-L| = 20 x T3 + 1` -> false |
| X-F20-3 flat_day | H = L = C -> true |
| X-F20-4 negative_inner_term | 2C - H - L < 0 uses the absolute value |
| X-F20-5 i64_edge_prices | H, L, C near `i64::MAX / 3`: i128, no overflow, exact |
| X-F20-6 one_paisa_day | H - L = 1 paisa |
| X-F20-7 (combo) tie_at_i64_scale | the tie at near-i64-max prices |
| X-F20-8 (combo) tie_with_negative_inner | the tie reached through a negative `2C - H - L` |
| X-F20-9 (oob) refused_bar_cannot_reach | `c1` takes only an `AcceptedBar` (type): a refused or zero bar cannot be passed (compile-fail doc test) |
| P-F20-1 c1_vs_rational_model | 10,000 generated (H, L, C) incl. planted ties: equals a checked-i128 rational model |

### F21 `cond::c2_beyond_r1_s1`

| id | test | asserts |
|---|---|---|
| T-F21-1 long_above_r1 | `3c > 2T3 - 3L` -> long true |
| T-F21-2 short_below_s1 | `3c < 2T3 - 3H` -> short true |
| T-F21-3 PERM side_x_region | 2 sides x {above R1, at R1, between, at S1, below S1} (10 cases) |
| X-F21-1 at_r1_exactly | equality: long false (strict) |
| X-F21-2 at_s1_exactly | equality: short false |
| X-F21-3 h_equal_l | R1 = S1: long and short still exclusive |
| X-F21-4 never_both_sides | proof test: for every generated (c, H, L, C) long and short are never both true (since `2T3 - 3L >= 2T3 - 3H`) |
| X-F21-5 i64_edge | exact in i128 |
| X-F21-6 close_far_below | very low close: short true, no underflow |
| X-F21-7 (combo) at_r1_with_cpr_tie | C2 false and C1 true on one candle |
| X-F21-8 (combo) h_equal_l_at_edge | H = L near the i64 edge |
| X-F21-9 (oob) no_previous_session | the type `Option<PrevSession> = None` -> false both sides, never a default reference |
| P-F21-1 c2_vs_rational_model | 10,000 cases vs exact rationals R1 = (2T3 - 3L)/3 |

### F22 `cond::c3_supertrend_side`

| id | test | asserts |
|---|---|---|
| T-F22-1 up_long | settled Up -> long true |
| T-F22-2 down_short | settled Down -> short true |
| T-F22-3 PERM settled_x_trend_x_side_x_set | {settled, not} x {Up, Down} x 2 sides x 3 rule sets (24 cases): true iff settled and trend matches; VO reads 10/3, AV2 and VS read 10/2 |
| X-F22-1 unsettled_correct_trend | false |
| X-F22-2 flip_candle | the trend on the flip candle is the new trend |
| X-F22-3 wiring_matters | a fixture where 10/2 and 10/3 differ: VO and AV2 answer differently |
| X-F22-4 no_candle_yet | a reading before any fold: false |
| X-F22-5 after_hole_untainted_only | tainted ST blocks through F28 even if settled |
| X-F22-6 (combo) unsettled_on_flip | not settled on the flip candle: false |
| X-F22-7 (combo) wiring_and_flip | VO's state flips while AV2's does not, same candle |
| X-F22-8 (oob) swapped_table_detected | a rule-set table naming 10/3 for AV2 is refused at load (tables checked against SPEC §3.1) |
| P-F22-1 c3_vs_model | 10,000 cases |

### F23 `cond::c4_bollinger_beyond`

| id | test | asserts |
|---|---|---|
| T-F23-1 above_upper_long | true |
| T-F23-2 below_lower_short | true |
| T-F23-3 PERM warm_x_region_x_width_x_side | {warm, not} x {above upper, on upper, inside, on lower, below lower} x {2_000, 3_000} x 2 sides (40 cases) |
| X-F23-1 on_band | false (strict) |
| X-F23-2 sigma_zero_close_above_mean | as the BOLL tip defines |
| X-F23-3 not_warm | false |
| X-F23-4 close_refused_by_close_new | a `2^32 + 1` close never reaches C4 (a hole) |
| X-F23-5 width_per_rule_set | VO uses 2_000; AV2 and VS 3_000 |
| X-F23-6 (combo) between_bands | above 2_000 but inside 3_000: VO true, AV2 false |
| X-F23-7 (combo) sigma_zero_and_on_band | false |
| X-F23-8 (oob) widths_built_once | the `Width` values are built once per rule set (a counter seam: one construction per run) |
| P-F23-1 c4_vs_model | 10,000 cases |

### F24 `cond::c5_rsi_beyond`

| id | test | asserts |
|---|---|---|
| T-F24-1 above_70_long | true |
| T-F24-2 below_30_short | true |
| T-F24-3 PERM faithful_x_level_x_side | {faithful, not} x 5 level regions x 2 sides (20 cases) |
| X-F24-1 exactly_700000 | false |
| X-F24-2 exactly_300000 | false |
| X-F24-3 not_faithful_extreme | RSI 100 not faithful: false |
| X-F24-4 rsi_100 | long true |
| X-F24-5 rsi_0 | short true |
| X-F24-6 (combo) not_faithful_at_boundary | false |
| X-F24-7 (combo) faithful_flip_and_level | the candle it turns faithful and crosses 70 |
| X-F24-8 (oob) levels_built_once | `Level::from_ppm` called once per rule set |
| P-F24-1 c5_vs_model | 10,000 cases |

### F25 `cond::c6_willr_level`

| id | test | asserts |
|---|---|---|
| T-F25-1 av2_long_above_minus_85 | `100(HH-C) < 85(HH-LL)` -> true |
| T-F25-2 av2_short_below_minus_15 | `100(HH-C) > 15(HH-LL)` -> true |
| T-F25-3 vs_long_above_minus_15 | `100(HH-C) < 15(HH-LL)` -> true (ADD) |
| T-F25-4 vs_short_below_minus_85 | `100(HH-C) > 85(HH-LL)` -> true (ADD) |
| T-F25-5 PERM flat_x_region_x_side_x_form | {flat, not} x 7 regions x 2 sides x {AV2 LEVEL, VS} (56 cases) |
| X-F25-1 boundary_85 | equality: false for both forms |
| X-F25-2 boundary_15 | equality: false |
| X-F25-3 flat | false both sides both forms |
| X-F25-4 not_warm | false |
| X-F25-5 tables_differ_only_in_c6 | VS's table equals AV2's except the two C6 levels (structural diff test) |
| X-F25-6 (combo) flat_on_boundary | false |
| X-F25-7 (combo) forms_disagree_same_candle | %R = -50: AV2 long true and VS long false; %R = -90: AV2 short true and VS short true |
| X-F25-8 (oob) vo_never_reads_c6 | VO's table has no C6: a cold or flat %R never blocks VO |
| P-F25-1 c6_vs_model | 10,000 cases, both forms |

### F26 `cond::c7_cci_beyond`

| id | test | asserts |
|---|---|---|
| T-F26-1 above_plus_200_long | true |
| T-F26-2 below_minus_200_short | true |
| T-F26-3 PERM flat_x_region_x_side | 20 cases |
| X-F26-1 exactly_plus_200 | false |
| X-F26-2 exactly_minus_200 | false |
| X-F26-3 flat | false |
| X-F26-4 not_warm | false |
| X-F26-5 huge_cci | an outlier gives CCI far above 200: true, no overflow |
| X-F26-6 (combo) flat_and_boundary | false |
| X-F26-7 (combo) boundary_while_tainted | false through F28 |
| X-F26-8 (oob) vo_never_reads_c7 | a cold CCI never blocks VO |
| P-F26-1 c7_vs_model | 10,000 cases |

### F27 `cond::c8_ema_side`

| id | test | asserts |
|---|---|---|
| T-F27-1 above_ema_long | true |
| T-F27-2 below_ema_short | true |
| T-F27-3 PERM warm_x_cmp_x_side | {warm, not} x {>, =, <} x 2 sides (12 cases) |
| X-F27-1 equal | false both sides |
| X-F27-2 not_warm | false |
| X-F27-3 large_close | i128 `close x SCALE`, exact |
| X-F27-4 one_unit_above | `close x SCALE = scaled + 1`: long true |
| X-F27-5 one_unit_below | short true |
| X-F27-6 (combo) warm_and_equal | false |
| X-F27-7 (combo) ad_fixture_both_sides | AD:572-578 fixture: short true, long false |
| X-F27-8 (oob) vo_never_reads_c8 | a cold EMA never blocks VO (60-minute VO entries begin before EMA 200 warms) |
| P-F27-1 c8_vs_model | 10,000 cases |

### F28 `signal::ready_gate`

| id | test | asserts |
|---|---|---|
| T-F28-1 all_ready | ready |
| T-F28-2 blocked_reason_first_in_order | the reason is the first blocking reading in table order; every blocking reading's counter +1 |
| T-F28-3 PERM vo_warm_taint_bits | VO's 3 readings x {warm, cold} x {clean, tainted} (2^6 = 64 cases): ready iff all warm and clean |
| T-F28-4 PERM av2_vs_warm_taint_bits | AV2 and VS: 6 readings, 2^12 = 4,096 cases each |
| T-F28-5 PERM span_hole_bits | {in trading span, not} x {hole, not} x all-ready (4 cases) |
| X-F28-1 warm_this_candle | ready on the warming candle |
| X-F28-2 taint_ends_this_candle | ready on the last clean candle of the period |
| X-F28-3 warm_but_tainted | blocked |
| X-F28-4 unused_reading_cold | VO ready with EMA cold |
| X-F28-5 warmup_candle_ready_but_outside_span | no decision before the trading start |
| X-F28-6 (combo) all_cold_all_tainted | blocked, 6 counters +1 |
| X-F28-7 (combo) only_ema_blocks | AV2 and VS blocked, VO ready, same candle |
| X-F28-8 (oob) first_candle_is_hole | the first candle of a stock is a hole: blocked, no counter underflow |
| P-F28-1 gate_vs_model | 10,000 generated gate states |

### F29 `signal::evaluate`

| id | test | asserts |
|---|---|---|
| T-F29-1 PERM vo_exhaustive | VO: 2^4 condition masks x 2 sides (32 cases): a signal iff all four true for that side and the gate ready |
| T-F29-2 PERM av2_exhaustive | AV2: 2^8 x 2 (512 cases) |
| T-F29-3 PERM vs_exhaustive | VS: 2^8 x 2 (512 cases) |
| T-F29-4 all_true | each rule set, each side: signal (6 cases) |
| T-F29-5 all_false | each rule set, each side: none (6 cases) |
| T-F29-6 one_off_each_condition | each rule set, each side, each condition false alone with all others true: none (VO 4 + AV2 8 + VS 8 = 20 x 2 sides = 40 cases, each named `<set>_<side>_without_<Ci>`) |
| T-F29-7 each_alone_true | only Ci true: none (40 cases) |
| T-F29-8 PERM gate_x_conditions | the gate's blocked/ready x every condition mask of each rule set (2 x (16 + 256 + 256) x 2 = 2,112 cases): blocked always gives none |
| X-F29-1 never_both_sides | on every generated candle of every rule set, never long and short at once (C2 and C3 exclusivity), checked on 10^5 candles |
| X-F29-2 hole_candle | all true on a hole: none |
| X-F29-3 cut_candle | all true at T = 15:08:00: a candidate is not emitted, "cut by 15:08" +1 |
| X-F29-4 last_entry_candle | all true at T = 15:07:00 (k=1): emitted |
| X-F29-5 reentry_needs_no_fresh_green | after an exit, a still-green candle signals again (AD:120-123) |
| X-F29-6 (combo) all_true_on_hole_at_cut | none, counted as hole only (cut not double-counted) |
| X-F29-7 (combo) different_sides_different_sets | %R = -10 on a candle: AV2 short false, VS long can be true: each rule set independent |
| X-F29-8 (oob) empty_or_duplicate_table | a rule set with zero conditions or a repeated condition is refused at load, never "always true" |
| X-F29-9 (oob) flip_waits_for_supertrend | after a flip exit, C3 itself keeps the same side false until the Supertrend turns back |
| P-F29-1 signals_vs_naive_pipeline | 10,000 generated stock-months: signals equal a model evaluating every condition with the naive reading models |

### F30 `fills::entry`

| id | test | asserts |
|---|---|---|
| T-F30-1 long_buys_high | `first_at_or_after(T)` = s: fill at s.high |
| T-F30-2 short_sells_low | fill at s.low |
| T-F30-3 PERM side_x_when_x_cut | 2 sides x {second at T, later in the minute, > 60 s later, none} x {T <= 15:07:00, T >= 15:08:00} (16 cases) |
| X-F30-1 not_filled | no traded second left that day: "entry not filled", stock stays flat |
| X-F30-2 never_crosses_day | the next traded second is tomorrow: not filled (bounded to the stock-day) |
| X-F30-3 late_fill_counted | fill 61 s after T: "late fill" +1, still filled; 60 s exactly is not late |
| X-F30-4 fill_at_150759 | goes straight to the square-off rule |
| X-F30-5 high_equal_low_second | fill at that price |
| X-F30-6 zero_volume_second | a stored 1-second record with volume 0 (a Rule T contract break): used as stored, `contract.zero_volume_seconds` +1 (Q7) |
| X-F30-7 (combo) late_and_after_150759 | late and at/after 15:07:59: both counted, square-off rule |
| X-F30-8 (combo) split_day_raw_fill | on a split day the fill is at the raw GDFL price, qty from raw |
| X-F30-9 (oob) gdfl_locked_mid_run | `open_existing` refusing (Locked) for that month: the stock-day becomes a fallback day, counted by reason |
| X-F30-10 (oob) tix_past_eof | a `.tix` entry past the file end: a refusal by name (never a wrong second) |
| P-F30-1 entry_vs_linear_scan | 10,000 generated second-streams vs a linear scan model |

### F31 `pricemap::ratio_source`

| id | test | asserts |
|---|---|---|
| T-F31-1 same_minute | Z = close of the last Zerodha minute (volume > 0) ending at or before T; G = close of the last GDFL second inside that minute |
| T-F31-2 earlier_minute | the minute has no GDFL trade: the latest earlier minute with both |
| T-F31-3 PERM z_x_g | {Z minute volume > 0, padded} x {G in minute, none} x {earlier minute with both, none} (8 cases) |
| X-F31-1 minute_ending_exactly_at_t | included |
| X-F31-2 padded_minute_skipped | volume 0 is never Z |
| X-F31-3 no_ratio | no minute of the session has both: "no ratio" +1, candidate not taken |
| X-F31-4 gdfl_second_at_minute_end | a second stamped at the minute end belongs to the next minute |
| X-F31-5 first_record_of_month | `first_at_or_after(minute end) - 1` at index 0: no underflow, earlier-minute rule |
| X-F31-6 (combo) first_minute_of_day | T 09:16 with GDFL's first trade 09:15:01 |
| X-F31-7 (combo) split_day_ratio | Z adjusted, G raw, ratio 5 for a 1:5 split |
| X-F31-8 (oob) every_gdfl_second_in_one_minute | a session whose GDFL trades sit in one minute: all candles map from it (bounded search) |
| P-F31-1 ratio_vs_brute_force | 10,000 generated sessions vs a brute-force search over every minute |

### F32 `pricemap::map_level`

| id | test | asserts |
|---|---|---|
| T-F32-1 long_floor | `L_z x G / Z` floored (`div_euclid`) |
| T-F32-2 short_ceil | ceiled |
| T-F32-3 PERM side_x_exactness | 2 sides x {exact division, remainder} (4 cases) |
| X-F32-1 exact_no_plus_one | ceil of an exact quotient is the quotient |
| X-F32-2 ratio_one | G = Z: unchanged |
| X-F32-3 i64_edge_product | `L_z x G` near `i64::MAX^2`: i128, exact |
| X-F32-4 z_zero_refused | Z = 0 refuses (never divides) |
| X-F32-5 one_paisa | L_z = 1 |
| X-F32-6 (combo) huge_ratio_at_edge | G/Z = 1,000 with a large level |
| X-F32-7 (combo) remainder_one_both_sides | floor and ceil differ by exactly one |
| X-F32-8 (oob) result_beyond_i64 | a mapped level beyond i64 refuses by name |
| X-F32-9 (oob) level_not_positive | a mapped level <= 0 refuses by name |
| P-F32-1 map_vs_rational | 10,000 cases vs exact rational floor/ceil |

### F33 `trail::Trailing`

| id | test | asserts |
|---|---|---|
| T-F33-1 tighten_long | a higher mapped lower band moves the stop up |
| T-F33-2 never_back | a lower band leaves the stop |
| T-F33-3 flip_long | close below its own lower band after folding: flip |
| T-F33-4 PERM side_x_across_x_band_x_hole_x_time | 2 sides x {across, not} x {tighter, equal, looser} x {hole, not} x {T_t <= 15:07:59, >= 15:08:00} (48 cases) |
| X-F33-1 equal_band | not tighter: no move |
| X-F33-2 flip_beats_move | across and tighter on one candle: flip |
| X-F33-3 first_candle_after_entry | applies |
| X-F33-4 candle_closing_150800 | no move, no flip |
| X-F33-5 no_ratio_on_move | the move's mapping fails: stop unchanged, "no ratio (trail)" +1 |
| X-F33-6 (combo) hole_and_across | nothing |
| X-F33-7 (combo) split_day_move | the move maps through the split-day ratio |
| X-F33-8 (oob) band_above_market | a long's new stop above the current price: fills at the first second at or after T_t (F34) |
| X-F33-9 (oob) stop_above_entry | a long's stop above its entry (profit locked) is valid |
| P-F33-1 trailing_vs_replay | 10,000 generated trades vs a model replaying every candle |

### F34 `fills::StopScan`

| id | test | asserts |
|---|---|---|
| T-F34-1 long_low_reaches | `low <= stop`: fill at that low |
| T-F34-2 short_high_reaches | `high >= stop`: fill at that high |
| T-F34-3 PERM side_x_reach_x_live | 2 sides x {reaches, not} x {entry second, next second, after a move} (12 cases) |
| X-F34-1 gap_through | low far below the stop: fill at that low, in full |
| X-F34-2 entry_second_not_live | the entry second reaching the stop does not fire |
| X-F34-3 stop_in_force_from_t | a second at T_t - 1 uses the old stop, T_t the new |
| X-F34-4 low_equal_stop | fills (`<=`) |
| X-F34-5 high_equal_stop | fills (`>=`) |
| X-F34-6 (combo) gap_and_flip_same_second | one exit, a flip (F35) |
| X-F34-7 (combo) stop_and_time_same_second | the 15:07:59 second reaches the stop: pinned (Q3) |
| X-F34-8 (oob) halted_stock | no seconds after 11:00: no stop fill; the trade ends unclosable (F37) |
| P-F34-1 stop_vs_scan_model | 10,000 generated second-streams vs a naive scan |

### F35 `fills::flip`

| id | test | asserts |
|---|---|---|
| T-F35-1 long_sells_low | first second at or after T_t, at its low |
| T-F35-2 short_buys_high | at its high |
| T-F35-3 PERM side_x_stop_same_second | 2 x 2 (4 cases): same second reaching the stop is one exit counted flip |
| X-F35-1 flip_at_150700 | decided at T_t <= 15:07:59: a flip |
| X-F35-2 no_second_after_flip | no second left that day: unclosable |
| X-F35-3 flip_fills_after_150759 | still a flip (decided before 15:08) |
| X-F35-4 close_on_band | not across: no flip |
| X-F35-5 flip_second_is_entry_second | impossible by time order (T_t > entry): asserted by type |
| X-F35-6 (combo) flip_fills_at_151000 | trading gap until 15:10:00: pinned (Q4) |
| X-F35-7 (combo) flip_and_gap_through | one exit at the print, a flip |
| X-F35-8 (oob) flip_on_fallback_day | on minutes (F38) |
| P-F35-1 flip_vs_model | 10,000 cases |

### F36 `fills::time_exit`

| id | test | asserts |
|---|---|---|
| T-F36-1 fills_at_150759 | worst print |
| T-F36-2 fills_later_in_window | first traded second before 15:10:00 |
| T-F36-3 PERM side_x_first_second | 2 sides x {15:07:59, 15:09:59, 15:10:00, none} (8 cases) |
| X-F36-1 last_second_150959 | time exit |
| X-F36-2 first_second_151000 | square-off instead |
| X-F36-3 entry_after_150759 | skips to square-off |
| X-F36-4 no_seconds_in_window | square-off rule |
| X-F36-5 auction_day_window | 15:07:59 to 15:10:00 on a 15:15-close day |
| X-F36-6 (combo) stop_reached_inside_window | pinned (Q3) |
| X-F36-7 (combo) fallback_day_1508_minute | on the 15:08 minute onward |
| X-F36-8 (oob) sub_second_stamp | a record not on a whole second refuses by name |
| P-F36-1 time_vs_model | 10,000 cases |

### F37 `fills::square_off`

| id | test | asserts |
|---|---|---|
| T-F37-1 square_off | first second at or after 15:10:00, worst print, `by_broker` |
| T-F37-2 unclosable | none to the session end: worst print since entry, `by_broker`, flagged |
| T-F37-3 PERM side_x_outcome | 2 sides x {square-off, unclosable} (4 cases) |
| X-F37-1 worst_since_entry_includes_entry_second | pinned (Q5) |
| X-F37-2 auction_day_ignores_auc | the `.auc` prints are never read; continuous close 15:15 |
| X-F37-3 halted_after_entry | unclosable at the lowest low since entry |
| X-F37-4 short_highest_high | unclosable short at the highest high |
| X-F37-5 square_off_151000_exactly | at 15:10:00 |
| X-F37-6 (combo) unclosable_on_split_day | raw prices |
| X-F37-7 (combo) unclosable_on_fallback_day | minutes' lowest low |
| X-F37-8 (oob) entry_is_last_second | nothing after the entry: unclosable at the entry second's worst |
| P-F37-1 square_off_vs_model | 10,000 cases |

### F38 `fills::MinuteFallback`

| id | test | asserts |
|---|---|---|
| T-F38-1 entry_on_minute | first minute starting at or after X with volume > 0: buy at high, sell at low |
| T-F38-2 stop_from_next_minute | live from the minute after the entry minute |
| T-F38-3 PERM reason_x_side_x_exit | {GDFL month missing, open refused, no traded second, 2019-05-30} x 2 sides x {stop, flip, time, square-off, unclosable} (40 cases): every fill on minutes, "minute fallback" mark |
| X-F38-1 vendor_late_day_every_stock | 2019-05-30 for all stocks |
| X-F38-2 missing_gdfl_month | ANANDRATHI 2026-08 |
| X-F38-3 padded_entry_minute | volume 0: the next minute |
| X-F38-4 time_on_1508_minute | onward |
| X-F38-5 square_off_on_1510_minute | onward |
| X-F38-6 slot_second_plus_59 | slot timing uses minute start + 59 s |
| X-F38-7 (combo) fallback_and_unclosable | worst minute since entry |
| X-F38-8 (combo) fallback_on_auction_day | 15:15 close |
| X-F38-9 (oob) entry_on_1507_minute | T 15:07:00: fills on the 15:07 minute, slot second 15:07:59, then the time rule |
| P-F38-1 fallback_vs_model | 10,000 generated fallback days |

### F39 `trade::simulate`

| id | test | asserts |
|---|---|---|
| T-F39-1 one_trade_per_exit_kind | fixtures ending by stop, flip, time, square-off, unclosable: every candidate field |
| T-F39-2 portfolio_free | the same stock's candidates are identical with 1 or 218 stocks in the run |
| T-F39-3 counters_by_kind | exits counted by kind |
| T-F39-4 PERM kind_x_side_x_fallback | 5 kinds x 2 sides x {GDFL, fallback} (20 cases) |
| X-F39-1 last_entry_candle | T 15:07:00: entry, then the time rule |
| X-F39-2 day_with_one_candle | one candle: at most one candidate |
| X-F39-3 halted_stock | unclosable |
| X-F39-4 stop_and_flip_same_second | one exit, flip |
| X-F39-5 gap_through_stop | taken in full |
| X-F39-6 overlapping_own_candidates | a second signal while its own would-be trade is open: both simulated; Phase B decides |
| X-F39-7 (combo) split_and_fallback | a split day that is also a fallback day |
| X-F39-8 (combo) late_fill_then_stop_next_second | both counted, exit right |
| X-F39-9 (oob) exit_before_entry_unrepresentable | an exit second before the entry second cannot be built (`ExitAfterEntry` newtype) |
| P-F39-1 simulate_vs_independent_replay | 10,000 generated stock-days vs a second, independent day-replay simulator written in the test from SPEC §3.5's text |

### F40 `sizing::qty`

| id | test | asserts |
|---|---|---|
| T-F40-1 cn01_size | Rs 1,250.00 -> 400 |
| T-F40-2 floor | Rs 1,234.56 -> 405 (`50_000_000.div_euclid(123_456)`) |
| T-F40-3 exposure_bound | `qty x price <= 50_000_000` and `(qty + 1) x price > 50_000_000` |
| X-F40-1 price_above_exposure | 50,000,001 paisa -> 0: "zero quantity", not taken, no `price_trade` call |
| X-F40-2 price_equals_exposure | 50,000,000 -> 1 |
| X-F40-3 one_paisa | -> 50,000,000 shares, fits i64 |
| X-F40-4 zero_price_refused | refuses (impossible fill) |
| X-F40-5 negative_price_refused | refuses |
| X-F40-6 exact_divisor | 125,000 -> 400, no remainder |
| X-F40-7 (combo) i64_max_price | 0, counted, no costs call, no slot held (Q6) |
| X-F40-8 (combo) split_day_raw_price | qty from the raw GDFL fill (Rs 500 -> 1,000), not the adjusted price |
| X-F40-9 (oob) never_compounds | after a +50% equity month the qty for the same price is unchanged |
| P-F40-1 qty_vs_model | 10,000 prices: equals the model and the exposure bound holds |

### F41 `costs::trade_fills`

| id | test | asserts |
|---|---|---|
| T-F41-1 long_buy_then_sell | entry Buy, exit Sell, one `Fill { second, quantity, price }` each |
| T-F41-2 short_sell_then_buy | entry Sell, exit Buy |
| T-F41-3 PERM side_x_kind | 2 sides x 5 exit kinds (10 cases): `by_broker` true only on square-off and unclosable |
| X-F41-1 trade_day_from_exit | `TradeDay` of the IST exit day |
| X-F41-2 fallback_seconds | minute start + 59 s seconds pass through |
| X-F41-3 first_priced_day | 2018-09-03 prices |
| X-F41-4 last_priced_day | 2026-10-30 prices |
| X-F41-5 outside_check_window | a day outside `check_window`: `ChargeError` stops the run by name (F52) |
| X-F41-6 (combo) unclosable_short | Buy exit `by_broker` true |
| X-F41-7 (combo) fallback_square_off | minute seconds and `by_broker` |
| X-F41-8 (oob) entry_and_exit_same_second | one-second trade: two orders, valid |
| P-F41-1 fills_vs_model | 10,000 candidates |

**F42..F50 share one oracle:** `tests/model/costs.rs` holds every rate and rounding rule as a literal typed from
COSTS.md:47-68 and :129-136 and CT's `rows`/`readings`, each with its source line, and computes each `NoteLines` field
by hand rules; each test drives the real `price_trade` with constructed `TradeFills` and asserts that field of
`dated_worst()` and of `today_worst()` (and `dated_official()` where a fixture states it) against the oracle. The
era rows are tested at each era's first day and the day before it, as CT dates them.

### F42 `costs::line_brokerage` (BRK-1, RD-BRK)

| id | test | asserts |
|---|---|---|
| T-F42-1 small_order_percent | 0.03% below Rs 20: brokerage = 0.03% rounded as RD-BRK |
| T-F42-2 large_order_cap | Rs 20 cap |
| T-F42-3 PERM per_order_x_side | {entry below cap, above} x {exit below, above} (4 cases): per executed order, summed |
| X-F42-1 at_cross_point | order value where 0.03% = Rs 20 exactly: Rs 20 |
| X-F42-2 one_paisa_below_cross | rounding at the cross |
| X-F42-3 one_share_one_paisa | minimum order |
| X-F42-4 fifty_million_shares | 50,000,000 shares at 1 paisa |
| X-F42-5 both_clocks | dated and today worst equal for BRK-1 |
| X-F42-6 (combo) orders_on_both_sides_of_cap | entry under, exit over |
| X-F42-7 (combo) square_off_order_also_brokered | `by_broker` exit pays BRK-1 plus BRK-SQ |
| X-F42-8 (oob) max_exposure_order | Rs 5,00,000 order: cap |
| P-F42-1 brokerage_vs_oracle | 10,000 generated trades |

### F43 `costs::line_square_off` (BRK-SQ, RD-SQ)

| id | test | asserts |
|---|---|---|
| T-F43-1 square_off_charged | Rs 20 + GST 18% on a square-off exit (`square_off`, `square_off_gst`) |
| T-F43-2 not_charged_otherwise | stop, flip, time: 0 |
| T-F43-3 PERM kind | 5 kinds (5 cases) |
| X-F43-1 unclosable_charged | `by_broker` |
| X-F43-2 gst_rounding | RD-SQ |
| X-F43-3 both_clocks | both |
| X-F43-4 short_square_off | charged |
| X-F43-5 fallback_square_off | charged |
| X-F43-6 (combo) unclosable_fallback_short | charged once |
| X-F43-7 (combo) flip_filling_at_1510 | per Q4's pin |
| X-F43-8 (oob) two_by_broker_orders | only the exit is `by_broker`; the entry never is |
| P-F43-1 square_off_vs_oracle | 10,000 trades |

### F44 `costs::line_dp` (BRK-DP, RD-DP)

| id | test | asserts |
|---|---|---|
| T-F44-1 dp_per_trade | Rs 12.50 + GST (`dp`, `dp_gst`) as `price_trade` applies it |
| T-F44-2 long_and_short | the rule for each side as COSTS states |
| T-F44-3 PERM side_x_kind | 10 cases |
| X-F44-1 gst_rounding | RD-DP |
| X-F44-2 both_clocks | both |
| X-F44-3 tiny_trade | 1 share |
| X-F44-4 max_trade | Rs 5,00,000 |
| X-F44-5 approx_13_per_trade | Rs 14.75 per trade (SPEC §4 item 13) |
| X-F44-6 (combo) dp_with_square_off | both lines |
| X-F44-7 (combo) dp_on_fallback_day | same |
| X-F44-8 (oob) two_trades_one_stock_day | each trade pays its own DP (no client-day aggregation, item 13) |
| P-F44-1 dp_vs_oracle | 10,000 trades |

### F45 `costs::line_exchange_ipft` (EX-2009..EX-2026, IP-OLD..IP-2026)

| id | test | asserts |
|---|---|---|
| T-F45-1 each_ex_era | one trade inside each of the six EX eras: Rs 325, 345, 325, 322, 297, 306.99 per crore |
| T-F45-2 each_ip_era | IP-OLD Rs 0.01, IP-2023 Rs 10, IP-2026 Rs 0.01 per crore |
| T-F45-3 PERM ex_x_ip_eras | every (EX era, IP era) pair that CT dates as co-existing |
| X-F45-1 ex2009_last_day | 2020-12-31 |
| X-F45-2 ex2021_first_day | 2021-01-01 |
| X-F45-3 every_era_boundary | each era's first day and the day before (CT's dates) |
| X-F45-4 today_clock_rates | the today clock always uses EX-2026 and IP-2026 |
| X-F45-5 rd_round_ex | the rounding of RD-ROUND-EX |
| X-F45-6 (combo) boundary_and_max_trade | a Rs 5,00,000 trade on a boundary day |
| X-F45-7 (combo) dated_vs_today_worst | the larger reading wins per line |
| X-F45-8 (oob) one_paisa_turnover | a 1-paisa order: rounding never gives a negative or skipped line |
| P-F45-1 exchange_vs_oracle | 10,000 trades across every day of the window |

### F46 `costs::line_sebi` (SF-2017..SF-2021)

| id | test | asserts |
|---|---|---|
| T-F46-1 each_era | Rs 15, 10, 5, 10 per crore |
| T-F46-2 both_sides | on the turnover as COSTS states |
| T-F46-3 PERM era_x_side | 4 eras x 2 sides (8 cases) |
| X-F46-1 every_boundary | each era's first day and the day before |
| X-F46-2 today_clock | the today rate |
| X-F46-3 rounding | RD-ROUND-TAX |
| X-F46-4 tiny_turnover | rounds as the rule says |
| X-F46-5 max_turnover | exact |
| X-F46-6 (combo) boundary_and_gst_sebi | before and after 2022-07-18 on an SF-2021 day |
| X-F46-7 (combo) min_and_max_same_day | two trades, each its own note |
| X-F46-8 (oob) leap_day | 2020-02-29 |
| P-F46-1 sebi_vs_oracle | 10,000 trades |

### F47 `costs::line_stt` (STT-ND, RD-STT-BASE)

| id | test | asserts |
|---|---|---|
| T-F47-1 long_sell_exit | 0.025% of the exit sell |
| T-F47-2 short_sell_entry | 0.025% of the entry sell |
| T-F47-3 PERM side | 2 cases |
| X-F47-1 rounding_base | RD-STT-BASE |
| X-F47-2 tiny_sell | 1 share 1 paisa |
| X-F47-3 max_sell | Rs 5,00,000 |
| X-F47-4 half_rupee_rounding | at the rounding edge |
| X-F47-5 both_clocks | both |
| X-F47-6 (combo) short_unclosable | sell value from the entry |
| X-F47-7 (combo) loss_trade | STT on the sell regardless of P&L |
| X-F47-8 (oob) buy_side_never | the buy side never pays STT |
| P-F47-1 stt_vs_oracle | 10,000 trades |

### F48 `costs::line_stamp` (SD-STATE, RD-STATE, SD-ND)

| id | test | asserts |
|---|---|---|
| T-F48-1 state_levy_before_july_2020 | both sides at the worst listed state, capped per trade |
| T-F48-2 national_from_july_2020 | 0.003% of the buy |
| T-F48-3 PERM era_x_side | 2 eras x 2 sides (4 cases) |
| X-F48-1 last_state_day | 2020-06-30 |
| X-F48-2 first_national_day | 2020-07-01 |
| X-F48-3 cap_reached | the per-trade cap |
| X-F48-4 f3_rounding | FNO-COSTS's open F3 (up to 50 paise an order before 2020-07-01) stands as `price_trade` gives it, labelled |
| X-F48-5 short_buy_exit | the buy is the exit |
| X-F48-6 (combo) cap_on_boundary | the cap on 2020-06-30 vs no cap on 2020-07-01 |
| X-F48-7 (combo) tiny_and_max | both ends |
| X-F48-8 (oob) sell_never_national | from 2020-07-01 the sell side pays no stamp |
| P-F48-1 stamp_vs_oracle | 10,000 trades |

### F49 `costs::line_gst` (GST-1, GST-SEBI)

| id | test | asserts |
|---|---|---|
| T-F49-1 gst_on_brokerage_exchange_ipft | 18% |
| T-F49-2 gst_on_sebi_from_2022_07_18 | added |
| T-F49-3 PERM sebi_era_x_kind | {before, from 2022-07-18} x {square-off, not} (4 cases) |
| X-F49-1 day_before | 2022-07-17: no GST on SEBI |
| X-F49-2 first_day | 2022-07-18: GST on SEBI |
| X-F49-3 rd_gst | rounding |
| X-F49-4 separate_dp_and_sq_gst | `dp_gst` and `square_off_gst` are separate lines, never double-counted in `gst` |
| X-F49-5 zero_brokerage | GST on the rest |
| X-F49-6 (combo) boundary_and_square_off | both |
| X-F49-7 (combo) max_trade_boundary | exact |
| X-F49-8 (oob) gst_never_on_stt_or_stamp | asserted |
| P-F49-1 gst_vs_oracle | 10,000 trades |

### F50 `costs::rounding` (RD-ROUND-TAX, RD-ROUND-EX, RD-GST)

| id | test | asserts |
|---|---|---|
| T-F50-1 each_rule | the three rules on hand cases |
| T-F50-2 worst_level | the worst level is chosen |
| T-F50-3 PERM rule_x_remainder | 3 rules x {0, half, just under half, just over} (12 cases) |
| X-F50-1 exact_half_paisa | the rule's direction |
| X-F50-2 zero | 0 stays 0 |
| X-F50-3 one_unit | the smallest non-zero |
| X-F50-4 large_value | exact in i128 |
| X-F50-5 both_clocks_same_rule | the same rule on both clocks |
| X-F50-6 (combo) half_on_every_line | one trade whose every line sits on a half |
| X-F50-7 (combo) worst_level_and_boundary_day | both |
| X-F50-8 (oob) sum_of_rounded_vs_rounded_sum | the note follows the table's order (per line, then sum), asserted against the oracle |
| P-F50-1 rounding_vs_oracle | 10,000 values |

### F51 `costs::headline`

| id | test | asserts |
|---|---|---|
| T-F51-1 max_of_clocks | headline = max(dated worst total, today worst total) |
| T-F51-2 headline_from | the label names the larger clock |
| T-F51-3 cn01_self_check | CN01 (400 at Rs 1,250, sold Rs 1,262.50, 2018-09-03): dated official 313.25, today official 225.79 |
| X-F51-1 equal_clocks | equal totals: pinned label (Q8) |
| X-F51-2 dated_larger | 2018 trade |
| X-F51-3 today_larger | a trade where today's rates are higher |
| X-F51-4 tiny_trade | exact |
| X-F51-5 max_trade | exact |
| X-F51-6 (combo) equal_and_square_off | label pinned |
| X-F51-7 (combo) boundary_day_switch | the larger clock switches across an era boundary |
| X-F51-8 (oob) official_vs_worst | the headline uses the worst readings, never the official ones |
| P-F51-1 headline_vs_oracle | 10,000 trades |

### F52 `costs::trade_net`

| id | test | asserts |
|---|---|---|
| T-F52-1 long_net | (sell - buy) value - headline |
| T-F52-2 short_net | (entry sell - exit buy) value - headline |
| T-F52-3 PERM side_x_sign | 2 sides x {gross > 0, = 0, < 0} (6 cases) |
| X-F52-1 zero_gross | net = -headline |
| X-F52-2 net_zero_exactly | gross = headline: net 0 (neither win nor loss) |
| X-F52-3 refusal_stops_run | a `ChargeError` stops the run with the unit key, the candidate and the error |
| X-F52-4 open_intraday_refused | mismatched quantities give `OpenIntradayPosition` (never reached by the tool: asserted by a constructed input) |
| X-F52-5 large_loss | i128 |
| X-F52-6 (combo) unclosable_loss_with_square_off | all lines |
| X-F52-7 (combo) fallback_and_split | raw/minute prices |
| X-F52-8 (oob) net_equals_twoclock_net | the tool never recomputes net itself; it reads `TwoClock::net` and asserts gross - headline equal |
| P-F52-1 net_vs_oracle | 10,000 trades |

### F53 `book::merge_candidates`

| id | test | asserts |
|---|---|---|
| T-F53-1 by_t_then_symbol | merged order (T, symbol byte order) |
| T-F53-2 views | LONG keeps long, SHORT short, BOTH all |
| T-F53-3 PERM view_x_tie | 3 views x {no tie, tie at T} (6 cases) |
| X-F53-1 all_stocks_same_t | 218 at one T: symbol order |
| X-F53-2 empty_stock | a stock with no candidates |
| X-F53-3 one_stock | merge is identity |
| X-F53-4 prefix_symbols | `M&M` before `M&MFIN` at the same T |
| X-F53-5 no_candidates_at_all | empty book |
| X-F53-6 (combo) ties_and_views | ties across sides in BOTH |
| X-F53-7 (combo) arrival_order_from_workers | candidate files committed in any order: identical merge |
| X-F53-8 (oob) same_t_same_symbol_twice | a duplicate key refuses by name (a bug upstream) |
| P-F53-1 merge_vs_sort_all | 10,000 generated books vs sorting everything |

### F54 `book::SlotAllocator`

| id | test | asserts |
|---|---|---|
| T-F54-1 admit_up_to_20 | 20 admitted, the 21st skipped "slots full" |
| T-F54-2 ignore_while_open | "signal while open" +1 |
| T-F54-3 separate_accounts | LONG, SHORT and BOTH books never share slots |
| T-F54-4 PERM open_x_full_x_exit_vs_t | {stock open, not} x {< 20 open, 20 open} x {an open exit second < T, = T, > T} (12 cases) |
| X-F54-1 twenty_full_three_ties | 19 open + 3 candidates at one T: first by symbol admitted, two "slots full" |
| X-F54-2 exit_second_equals_t | still held (inclusive): skipped |
| X-F54-3 exit_second_t_minus_one | freed: admitted |
| X-F54-4 own_exit_at_next_signal | the stock's exit second equals its next T: "signal while open" |
| X-F54-5 all_stocks_one_t | 218 signals at one T: 20 admitted in symbol order |
| X-F54-6 (combo) full_open_and_ties | 20 full + a while-open candidate + 3 ties: while-open counted first, then full |
| X-F54-7 (combo) fallback_and_gdfl_seconds | a fallback slot second (minute + 59) equal to another stock's GDFL exit second |
| X-F54-8 (oob) unfilled_candidate_never_holds | entry not filled, zero quantity and no ratio never hold a slot (Q6) |
| X-F54-9 (oob) both_sides_one_stock | BOTH: a long open on A and a short candidate on A: while open |
| P-F54-1 slots_vs_interval_model | 10,000 generated books vs a model counting the intervals containing each T |

### F55 `account::MonthAgg`

| id | test | asserts |
|---|---|---|
| T-F55-1 exit_day_attribution | a trade counts in its exit day's month |
| T-F55-2 compounding_capital_start | capital start = 200,000,000 + every earlier net |
| T-F55-3 month_bp_floor | `(10_000 x net).div_euclid(capital)` in i128 |
| T-F55-4 PERM net_sign_x_capital | {net > 0, = 0, < 0} x {capital > 0, = 0, < 0} (9 cases) |
| X-F55-1 no_trade_month | 0 bp, fails |
| X-F55-2 month_2_4999_percent | fails (bp 249) |
| X-F55-3 month_2_5000_percent | passes (bp 250, `10_000 x net = 250 x capital`) |
| X-F55-4 negative_floor | -0.01% -> -1 bp (div_euclid) |
| X-F55-5 capital_at_or_below_zero | "account at or below zero", fails |
| X-F55-6 million_trades_i128 | sums exact |
| X-F55-7 (combo) last_and_first_day | exits on the 31st and the 1st split across months |
| X-F55-8 (combo) negative_capital_positive_month | still fails |
| X-F55-9 (oob) window_months_runtime | the month count comes from the window (2 months in E-1), never a constant 98 |
| P-F55-1 months_vs_naive_sum | 10,000 generated trade lists vs a naive per-month sum with rational comparison |

### F56 `account::DrawdownWalker`

| id | test | asserts |
|---|---|---|
| T-F56-1 marks | after each exit (exit second, then symbol) and at each day end |
| T-F56-2 peak_includes_start | a first loss draws down from 200,000,000 |
| T-F56-3 dd_bp_rupees_dates | bp, rupees, peak date and low date |
| T-F56-4 PERM mark_moves | {up, equal, down} x {new peak, below peak} (6 valid cases) |
| X-F56-1 equal_to_previous_peak | no new drawdown, peak date unchanged |
| X-F56-2 monotonic_up | 0 bp |
| X-F56-3 first_trade_loss | drawdown from the start |
| X-F56-4 two_equal_drawdowns | the first one's dates kept (pinned) |
| X-F56-5 dd_beyond_100_percent | equity below zero: bp > 10,000, printed as is |
| X-F56-6 (combo) same_second_exits | two stocks' exits at one second: intermediate mark counts, symbol order |
| X-F56-7 (combo) peak_and_low_same_day | dates equal |
| X-F56-8 (oob) million_marks_fixed_state | 10^6 marks: state size constant (`size_of` assert) |
| P-F56-1 dd_vs_quadratic_model | 10,000 equity paths vs an O(n^2) max over all (peak, later low) pairs |

### F57 `report::row_stats`

| id | test | asserts |
|---|---|---|
| T-F57-1 every_column | each §3.10 column on a hand fixture |
| T-F57-2 lower_median | the 49th smallest of 98; of 2 months the 1st |
| T-F57-3 win_rate_permille_floor | floored |
| T-F57-4 PERM wins_losses_presence | {wins, none} x {losses, none} (4 cases): averages and profit factor or their words |
| X-F57-1 zero_trades | "no trades" words, no division |
| X-F57-2 no_wins | "no wins" |
| X-F57-3 no_losses | "no losses", profit factor "no loss" |
| X-F57-4 net_zero_trade | neither win nor loss |
| X-F57-5 all_months_equal | worst = median = best |
| X-F57-6 (combo) one_trade_one_month | every column |
| X-F57-7 (combo) huge_gross_loss | i128 profit factor |
| X-F57-8 (oob) months_from_window | 2-month window in E-1 |
| P-F57-1 stats_vs_naive | 10,000 generated rows |

### F58 `verdict::bar`

| id | test | asserts |
|---|---|---|
| T-F58-1 pass_all_months | PASS |
| T-F58-2 miss_lists_months | every month below with its gap |
| T-F58-3 floor_is_worst_month | the floor |
| T-F58-4 PERM month_states | {>= 250, < 250} per month on a 3-month window (8 cases) |
| X-F58-1 one_month_249 | MISS, gap 1 bp |
| X-F58-2 all_exactly_250 | PASS |
| X-F58-3 exact_gap_paisa | `250 x capital - 10_000 x net` shown in paisa x 10_000 / capital |
| X-F58-4 negative_month | gap above 250 |
| X-F58-5 single_month_window | works |
| X-F58-6 (combo) all_months_no_trades | MISS with every month gap 250 |
| X-F58-7 (combo) capital_below_zero_month | MISS labelled |
| X-F58-8 (oob) pass_needs_trades_every_month | a row with trades in all but one month misses |
| P-F58-1 bar_vs_model | 10,000 generated month vectors |

### F59 `verdict::rank`

| id | test | asserts |
|---|---|---|
| T-F59-1 passers_by_lowest_dd | then most net, then win rate |
| T-F59-2 misses_order | fewest months below, highest worst month, lowest dd, net |
| T-F59-3 passers_before_misses | always |
| T-F59-4 PERM each_criterion_decides | each of the 7 criteria decides alone between two rows (7 cases) |
| X-F59-1 all_pass | ordered |
| X-F59-2 none_pass | ordered |
| X-F59-3 full_tie | ordered by (rule set, timeframe, account) |
| X-F59-4 single_row | trivially |
| X-F59-5 negative_nets | ordered |
| X-F59-6 (combo) miss_with_huge_net | after every passer |
| X-F59-7 (combo) ties_on_first_three_criteria | the fourth decides |
| X-F59-8 (oob) input_order_free | every permutation of the input rows gives one output |
| P-F59-1 rank_vs_comparator_model | 10,000 generated row sets vs an explicit comparator sort |

### F60 `report::head_compare`

| id | test | asserts |
|---|---|---|
| T-F60-1 permille | AV2 over VO and VS over VO for trades, net, drawdown |
| T-F60-2 nearest_met_floor | over every row |
| T-F60-3 PERM base_sign | {base > 0, = 0, < 0} x 3 measures (9 cases) |
| X-F60-1 zero_base | printed "base 0" |
| X-F60-2 negative_base | printed as such |
| X-F60-3 equal | 1,000 permille |
| X-F60-4 huge_ratio | i128 |
| X-F60-5 no_trades_either | words, no division |
| X-F60-6 (combo) zero_base_negative_other | both words |
| X-F60-7 (combo) only_two_rule_sets | without VS the head still prints |
| X-F60-8 (oob) floor_all_negative | the nearest met floor is the highest worst month, even if negative |
| P-F60-1 head_vs_model | 10,000 generated pairs |

### F61 `report::rupees`

| id | test | asserts |
|---|---|---|
| T-F61-1 two_decimals | 123,456 -> "1,234.56" in the first run's grouping |
| T-F61-2 account_size | 200,000,000 -> "20,00,000.00" (Indian grouping, as SPEC writes it) |
| T-F61-3 PERM sign_x_size | {neg, 0, pos} x {< 1 rupee, < 1,000, lakh, crore} (12 cases) |
| X-F61-1 zero | "0.00" |
| X-F61-2 minus_one_paisa | "-0.01" |
| X-F61-3 five_paisa | "0.05" |
| X-F61-4 i64_min | exact, no overflow |
| X-F61-5 i128_large | exact |
| X-F61-6 (combo) negative_crore | "-1,00,00,000.00" |
| X-F61-7 (combo) negative_under_one_rupee | "-0.99" |
| X-F61-8 (oob) no_float_path | built by integer division only (T-G-1 plus a digit model) |
| P-F61-1 rupees_vs_digit_model | 10,000 values vs a naive digit-by-digit model |

### F62 `run::plan_units`

| id | test | asserts |
|---|---|---|
| T-F62-1 unit_count_formula | units = sum over stocks of 6 timeframes x rule sets x months in its span; row units = rule sets x 6 x 3 x months |
| T-F62-2 deterministic_order | (stock ordinal, timeframe, rule set, month) |
| T-F62-3 PERM sets_x_tfs_x_stocks | {1, 2, 3} rule sets x {1, 6} timeframes x {1, 3} stocks (12 cases): counts equal the formula |
| X-F62-1 spanless_stock | 0 units, listed |
| X-F62-2 one_month_span | one unit per timeframe and rule set |
| X-F62-3 one_name_universe | runs |
| X-F62-4 one_month_window | runs |
| X-F62-5 late_span_start | units only from its first month |
| X-F62-6 (combo) all_spanless | refuses "nothing to run" |
| X-F62-7 (combo) add_month_extends_only | the plan gains exactly the new month's units |
| X-F62-8 (oob) hash_seed_free | the plan file's sha256 is equal under different `RandomState` seeds (no hash-order dependence) |
| P-F62-1 plan_vs_nested_loops | 10,000 generated universes vs a nested-loop model |

### F63 `run::WorkerPool`

| id | test | asserts |
|---|---|---|
| T-F63-1 every_core | workers = `available_parallelism()` by default; one stock per worker at a time |
| T-F63-2 byte_equal_any_workers | outputs and unit files equal for 1, 2 and N workers |
| T-F63-3 PERM workers_x_stocks | {1, 2, N} x {1, 3, 22} stocks (9 cases): byte-equal |
| X-F63-1 more_workers_than_stocks | idle workers exit cleanly |
| X-F63-2 worker_panic_stops_by_name | the run stops with the unit key; no row written; committed units stand |
| X-F63-3 slow_stock_does_not_block | a 100x stock: the others finish first (work stealing) |
| X-F63-4 single_core | N = 1 works |
| X-F63-5 explicit_workers_bounds | `--workers 0` refuses; 1..=1024 accepted |
| X-F63-6 (combo) panic_then_resume | after the fix, resume completes byte-equal |
| X-F63-7 (combo) one_worker_many_stocks_memory | per-worker state fixed: peak RSS for 22 stocks within the bound measured for 3 |
| X-F63-8 (oob) results_independent_of_finish_order | randomized finish order (test seam delaying workers) gives equal bytes |
| P-F63-1 pool_vs_single_thread | 1,000 tiny generated stores x random worker counts: byte-equal to a single-thread model |

### F64 `db::commit_unit`

| id | test | asserts |
|---|---|---|
| T-F64-1 staged_then_linked | staging `create_new`, fsync, `hard_link` to `units/<key>-<sha12>.tsv`; returns path, sha256, bytes |
| T-F64-2 trailer_line | every unit file ends with a trailer holding the body sha256 and the unit's input digests |
| T-F64-3 PERM final_x_staging | {final absent, present same content, present different content} x {staging fresh, staging left by a killed attempt} (6 cases): link / adopt (no write) / NONDETERMINISM refusal; a left staging file stays, a new attempt name is used |
| X-F64-1 disk_full_at_byte_n | `Writer` seam fails at byte n: error by name, no final link, the partial staging file stays |
| X-F64-2 fsync_failure | refuses by name |
| X-F64-3 read_only_db | refuses |
| X-F64-4 empty_unit | a unit with no candidates is still a file (empty-body sha) |
| X-F64-5 large_unit | 100,000 candidates |
| X-F64-6 (combo) kill_before_link | resume re-evaluates once, links the same bytes |
| X-F64-7 (combo) kill_after_link_before_manifest | resume ADOPTS the linked file (trailer and sha verified) without re-evaluating, then appends the manifest row |
| X-F64-8 (oob) final_path_is_dir_or_symlink | refuses (O_EXCL + lstat) |
| P-F64-1 commits_vs_set_model | 10,000 random commit/kill sequences vs a model of the committed set |

### F65 `db::Manifest`

| id | test | asserts |
|---|---|---|
| T-F65-1 append_row | key, path, bytes, sha256, crc32 of the row; one `write` per row |
| T-F65-2 reader_checks_crc | bad rows skipped and counted |
| T-F65-3 PERM row_states | {complete, torn no newline, torn bad crc, duplicate same sha, duplicate different sha} (5 cases) |
| X-F65-1 torn_at_every_offset | the last row cut at each byte offset: the reader skips it; the writer first appends `\n`, then continues |
| X-F65-2 duplicate_different_sha | NONDETERMINISM refusal |
| X-F65-3 empty_manifest | valid |
| X-F65-4 million_rows | append cost flat (B-19) |
| X-F65-5 second_writer_refused | the db `LOCK` directory (mkdir) refuses a second run |
| X-F65-6 (combo) torn_then_reappended | one valid row for the key |
| X-F65-7 (combo) torn_and_disk_full_on_newline | refuses by name; the file is unchanged |
| X-F65-8 (oob) key_with_tab_or_newline | refused at key construction |
| P-F65-1 manifest_vs_vec_model | 10,000 random append/kill sequences vs a `Vec` model |

### F66 `db::resume`

| id | test | asserts |
|---|---|---|
| T-F66-1 skip_committed | committed units are not evaluated (evaluation counter 0) |
| T-F66-2 recompute_inflight | in-flight units are evaluated once and committed |
| T-F66-3 PERM unit_states | {committed, staged not linked, linked not in manifest, absent} (4 cases): skip / evaluate / adopt / evaluate |
| X-F66-1 changed_inputs_refused | a different spec, cost table, universe, membership digest or tool commit: refuses ("inputs changed: new run or add-month") |
| X-F66-2 complete_run | "already complete", no work |
| X-F66-3 committed_file_corrupt | a committed file failing its sha: refuses by name (never recomputed over it) |
| X-F66-4 bitmap_from_one_load | the manifest is read once; each check is one bit (B-20) |
| X-F66-5 resume_twice | idempotent |
| X-F66-6 (combo) kill_during_resume | a second resume completes byte-equal |
| X-F66-7 (combo) every_fault_point_in_sequence | killed at each of the 7 fault points in one schedule: byte-equal, one commit per unit |
| X-F66-8 (oob) foreign_run_in_same_db | another run id's rows are never mixed (run id in every key) |
| P-F66-1 resume_vs_uncut | 1,000 random kill schedules on the tiny store: byte-equal to the uncut run, one commit per unit |

### F67 `db::Snapshot`

| id | test | asserts |
|---|---|---|
| T-F67-1 stock_snapshot_content | all 7 reading states, taint counters, previous session, hole state, fold carry |
| T-F67-2 row_snapshot_content | capital, peak, drawdown and dates, counters, extremes |
| T-F67-3 PERM present_x_inputs | {present, absent} x {inputs same, one month's digest changed} (4 cases): continue / from start / recompute the chain from the changed month |
| X-F67-1 add_month_only_new_units | evaluation counter = stocks x 6 x rule sets x 1 |
| X-F67-2 revised_store_month | a changed served generation recomputes that stock's chain from that month and those rows' months from that month |
| X-F67-3 round_trip_bytes | serialize, read, serialize: equal bytes |
| X-F67-4 fixed_size | `const` size bound asserted at compile time |
| X-F67-5 no_open_position | a snapshot never holds an open trade (flat at night): asserted |
| X-F67-6 (combo) add_month_and_new_stock | the new stock's whole chain only |
| X-F67-7 (combo) month_ends_mid_taint | continues exactly (taint count carried) |
| X-F67-8 (oob) snapshot_claiming_open_trade | a hand-made snapshot with an open trade refuses by name |
| X-F67-9 (oob) other_commit_snapshot | a snapshot from another tool commit is never used |
| P-F67-1 split_runs_vs_uncut | 1,000 random month splits: month-by-month from snapshots equals one uncut run |

### F68 `db::DenseIndex`

| id | test | asserts |
|---|---|---|
| T-F68-1 slot_formula | mixed radix of (rule set, timeframe, side/account, stock or ALL, month); fixed-width record (offset, length, count, sha256 prefix) |
| T-F68-2 one_pread | a lookup is one `read_at` of the record plus one of the range |
| T-F68-3 PERM dims_first_mid_last | each of 5 dimensions at first/middle/last (3^5 = 243 cases): unique, in range, round-trip |
| X-F68-1 key_out_of_range | refuses |
| X-F68-2 all_stock_month_rows | stock = ALL holds the month row |
| X-F68-3 empty_slot | "no rows", not an error |
| X-F68-4 last_slot | the file's last record |
| X-F68-5 truncated_index | refuses by name |
| X-F68-6 (combo) max_slot | last set, timeframe, account, ALL, last month |
| X-F68-7 (combo) zero_trade_run | every slot empty but month rows present |
| X-F68-8 (oob) radix_change_versioned | a 4th rule set writes a new index version; an old file is never misread (the header carries the radix); checked multiplication |
| P-F68-1 index_vs_hashmap | 10,000 random keys vs a `HashMap` built from the same rows |

### F69 `db::Audit`

| id | test | asserts |
|---|---|---|
| T-F69-1 run_start_inputs | run.start holds sha256 of the spec, ADD, the cost table (and its version), the universe, the membership digest or "today's list", the skip file, the store-months digest list; the tool commit; the command line; workers |
| T-F69-2 run_end_counts | run.end holds every count, refusals, fallback fills, wall, per-unit p50/p99 |
| T-F69-3 PERM event_kinds | run.start, selfcheck, unit.done, unit.refused, fallback.day, progress, extreme, run.end (8 cases): written and parsed back |
| X-F69-1 chain_break_found | one changed byte in line k: verify names line k |
| X-F69-2 torn_last_line | skipped and closed with a newline |
| X-F69-3 wall_clock_back | sequence numbers stay monotonic; a "clock went back" event |
| X-F69-4 long_command_line | 64 KiB of arguments: one line |
| X-F69-5 sequence_gap_found | a missing line is found |
| X-F69-6 (combo) torn_then_resume | the chain continues from the last valid line |
| X-F69-7 (combo) two_runs_one_db | separate chains |
| X-F69-8 (oob) escaping_and_no_env_dump | tabs and newlines escaped; only named arguments are logged, never the environment |
| P-F69-1 audit_corruption_found | 10,000 random sequences with one random byte corrupted: verify names exactly that line |

### F70 `cmd::query`

| id | test | asserts |
|---|---|---|
| T-F70-1 full_key | one index read, the row |
| T-F70-2 each_single_filter | by run, stock, month, timeframe, rule set |
| T-F70-3 PERM filter_subsets | all 2^5 = 32 subsets on the E-1 db: equal to a scan model |
| X-F70-1 no_match | header only, exit 0 |
| X-F70-2 unknown_run | exit 2 by name |
| X-F70-3 lowercase_stock | refuses (exact spelling) |
| X-F70-4 month_outside_window | refuses |
| X-F70-5 whole_db_streamed | all trades streamed with constant memory |
| X-F70-6 (combo) each_alone_matches_together_none | header only |
| X-F70-7 (combo) query_during_run | reads committed rows only (manifest snapshot) |
| X-F70-8 (oob) query_on_failing_db | prints the verify warning first |
| P-F70-1 query_vs_scan | 10,000 random queries vs a full-scan model |

### F71 `cmd::verify`

| id | test | asserts |
|---|---|---|
| T-F71-1 clean_db | every file OK, exit 0 |
| T-F71-2 audit_chain_checked | the audit chain verified too |
| T-F71-3 PERM finding_kinds | MISMATCH, MISSING, ORPHAN, TORN each present/absent (16 cases): exit 1 iff MISMATCH or MISSING |
| X-F71-1 one_flipped_byte | that unit MISMATCH |
| X-F71-2 truncated_file | MISMATCH |
| X-F71-3 file_replaced_by_dir | MISMATCH |
| X-F71-4 orphans_expected | killed staging files listed as ORPHAN, exit 0 |
| X-F71-5 torn_rows | TORN listed, exit 0 |
| X-F71-6 (combo) all_kinds_at_once | each named, exit 1 |
| X-F71-7 (combo) verify_during_run | up to the manifest snapshot |
| X-F71-8 (oob) path_escape | a manifest row pointing outside the db (`../`) is MISMATCH-PATH |
| P-F71-1 verify_vs_model | 10,000 random corruption sets |

### F72 `monitor::Progress`

| id | test | asserts |
|---|---|---|
| T-F72-1 line_format | `PROGRESS <IST> units a/b (p.p%) rate r/min ETA hh:mm IST workers w/N refusals x fallback f`, integers |
| T-F72-2 every_minute | one line per minute of the `Clock` seam, to stdout and `db/runs/<run>/progress.log` |
| T-F72-3 PERM rate_x_done | {rate 0, > 0} x {done < total, = total} (4 cases): "ETA unknown" / time / "done" |
| X-F72-1 rate_zero | "ETA unknown" |
| X-F72-2 done | "done" |
| X-F72-3 eta_past_midnight | the date is shown |
| X-F72-4 clock_jump | a 10-minute jump prints one line saying "gap 10 min", not ten |
| X-F72-5 million_units | flat (B-23) |
| X-F72-6 (combo) rate_zero_with_refusals | both shown |
| X-F72-7 (combo) resume_counts | done includes earlier commits; the rate is this process's |
| X-F72-8 (oob) never_100_before_done | integer floor: 99.9% until the last unit |
| P-F72-1 progress_vs_recompute | 10,000 random timelines vs a brute-force recompute |

### F73 `monitor::Extremes`

| id | test | asserts |
|---|---|---|
| T-F73-1 topk_lists | k = 10 worst months, deepest drawdowns, most holes per stock, slowest units (p50/p99) to extremes.tsv and the db |
| T-F73-2 key_tiebreak | ties broken by key |
| T-F73-3 PERM count_x_tie | {< k, = k, > k events} x {tie at the k-th, none} (6 cases) |
| X-F73-1 all_equal | ordered by key |
| X-F73-2 zero_events | empty lists with headers |
| X-F73-3 negative_values | right order |
| X-F73-4 million_events | flat (B-23) |
| X-F73-5 kth_tie | deterministic |
| X-F73-6 (combo) ties_and_overflow_k | deterministic |
| X-F73-7 (combo) resumed_equals_uncut | equal lists (wall-time lists excluded: they are per process, labelled) |
| X-F73-8 (oob) zero_wall_time | clock granularity 0: ranked by key |
| P-F73-1 extremes_vs_sort | 10,000 random streams vs a full sort |

### F74 `selfcheck::run`

| id | test | asserts |
|---|---|---|
| T-F74-1 all_pass_real_inputs | five lines printed and audited; the run continues |
| T-F74-2 order_before_phase_a | no unit is evaluated before the checks pass |
| T-F74-3 PERM each_fails_alone | each of 5 checks failing alone (5 cases): stops by that name |
| X-F74-1 cn01_off_by_one_paisa | stops |
| X-F74-2 probe_fallback_not_a_stop | prints the mode; continues |
| X-F74-3 cpr_fixture_inverted | stops |
| X-F74-4 ema_fixture_wrong | stops |
| X-F74-5 reliance_month_missing | the 125-bucket check names the missing file and stops |
| X-F74-6 (combo) two_fail | both named, in order |
| X-F74-7 (combo) synthetic_store | the synthetic store's own RELIANCE fixture month passes |
| X-F74-8 (oob) audit_has_selfcheck_lines | each check's line is in the audit chain |
| P-F74-1 fixture_mutations_stop | 10,000 single-field mutations of the fixtures: every mutation that changes the expected answer stops the run |

### F75 `cli::parse_and_guard`

| id | test | asserts |
|---|---|---|
| T-F75-1 parse_all | --store --membership --universe --skip-file --out --db --workers --run-id |
| T-F75-2 identity_written | identity.txt: B, the five tips, the tool commit, cost table version, membership digest or "today's list", the command line |
| T-F75-3 PERM required_missing | each required argument missing alone (6 cases): usage refusal |
| X-F75-1 out_exists | refuses (a new folder only) |
| X-F75-2 out_inside_store_or_runtime | refuses |
| X-F75-3 disk_floor | `FreeSpace` seam 1 KiB under the floor: refuses with the numbers, nothing written |
| X-F75-4 store_unreadable | refuses |
| X-F75-5 unknown_or_repeated_flag | refuses |
| X-F75-6 (combo) out_exists_and_floor | both named |
| X-F75-7 (combo) relative_paths | canonical absolute paths in identity.txt |
| X-F75-8 (oob) symlink_into_store | `--out` symlinked into the store refuses after canonicalisation |
| X-F75-9 (oob) non_utf8_path | refuses by name |
| P-F75-1 args_vs_table | 10,000 random argument vectors vs a table model |

### F76 `report::write_outputs`

| id | test | asserts |
|---|---|---|
| T-F76-1 preview_line_first | line 1 of every file is the PREVIEW line verbatim, then the APPROXIMATE list |
| T-F76-2 columns | every §3.10 column; months.tsv rows x months; trades.tsv every admitted trade; counts.tsv every counter |
| T-F76-3 PERM file_x_book | 6 files x {trades, empty book} (12 cases) |
| X-F76-1 zero_trades_everywhere | valid files |
| X-F76-2 ampersand_symbols | `M&M` unescaped; names never hold a TAB |
| X-F76-3 negative_money | signs right |
| X-F76-4 rank_order | REPORT.md rows in F59 order |
| X-F76-5 lf_and_final_newline | always |
| X-F76-6 (combo) empty_book_all_miss | verdicts, gaps, floor |
| X-F76-7 (combo) two_vs_three_sets | the header lists the sets run |
| X-F76-8 (oob) byte_equal_any_path | equal across worker counts and resumes |
| P-F76-1 outputs_round_trip | 10,000 random row sets: write, parse back, equal |

### 2.E END-TO-END tests

| id | test | asserts |
|---|---|---|
| E-1 synthetic_store_hand_computed | **The SYN store** (built by `tests/fixtures/syn_store.rs` with the store crate's own writer, in a NEW folder `W/scratch/psrb-preview-<n>-fixtures/<ts>/`, never deleted): 24 symbols S01..S24 (plus a fixture RELIANCE for the self-check), Zerodha 1-minute months 2018-08 (warm-up), 2018-09 and 2018-10, GDFL 1-second months 2018-09 and 2018-10 with `.tix`, a synthetic membership folder and skip file. Price paths are built from segments a person can compute by hand (flat segments make sigma 0, monotone segments make RSI 100 or 0, one jump decides Bollinger and Supertrend). They plant: S01..S22 on one identical path, so 22 signals land at the same T and 20 are admitted, 2 skipped "slots full"; one trade per exit kind (stop with a gap through, flip, time, square-off, unclosable on a halted afternoon); a fallback day; a hole that taints and blocks; a membership-unknown day; a 1:5 split day; a signal on the last entry minute; a month with no trades. `tests/fixtures/e2e/expected-trades.tsv`, `expected-months.tsv`, `expected-rows.tsv` and `expected-counts.tsv` are written BY HAND, with `expected-derivation.md` deriving every trade (entry second and price, qty, exit second, price and kind, every `NoteLines` field from COSTS rates, gross, headline, net), every month's bp and pass, the drawdown with its dates, and each row's verdict. Asserts the tool's four files equal them byte for byte |
| E-2 kill_and_resume_byte_equal | E-1's store; for each fault point (after staging write, after link, mid manifest row, after manifest row, mid snapshot, mid audit line, mid output file) x n in {first, middle, last} (21 kills): `PSRB_PREVIEW_FAULT=<p>:<n>:pause`, SIGKILL when the child prints `FAULT <p> <n>`, then resume. Asserts outputs and unit files byte-equal to the uncut run, the manifest equal as a sorted set, `unit.done` exactly once per key, orphans listed by verify, nothing deleted |
| E-3 add_a_month | run on 2018-09 only, then add 2018-10 (a second NEW fixture folder) and run add-month: only 2018-10 units are evaluated (audit counter = stocks x 6 x rule sets) and outputs are byte-equal to E-1's uncut two-month run |
| E-4 worker_count_invariance | 1, 2 and N workers: byte-equal |
| E-5 query_verify_on_e2e_db | every hand trade found by its full key equals its expected row; verify exit 0; one flipped byte gives exit 1 naming that file |
| E-6 probe_fallback_run | a membership folder failing the probe: every output says "survivorship: today's list"; trades equal a second hand file |
| E-7 real_store_read_only | a 3-stock run (RELIANCE, M&M, ICICILIFE) on the REAL store: sha256 and mtime of every file it opened unchanged, `find STORE runtime -newer <stamp>` prints nothing, no `.lock` created |
| E-8 disk_floor_end_to_end | the `FreeSpace` seam under the floor: exit 2 before any write, `--out` not created |
| E-9 first_run_parity | the full real run: rows, months, trades and counts byte-equal to the first run's; every difference explained in BUILD-NOTES-HARDEN.md with its regression test |
| E-10 sets_independent | a run of VO + AV2 only gives the same VO and AV2 rows as the three-set run |
| E-11 smallest_store | 1 stock, 1 warm-up month, 1 trading day: completes; the month is 0 bp and fails; MISS; no panic |

---

## 3. O(1) AT p99: the benches

**Form (the project's ratio-bench form, `crates/telemetry/benches/ratio.rs` and `crates/indicators/benches/ratio.rs`):**
one file `crates/psrb_preview/benches/ratio.rs`, `[[bench]] name = "ratio", harness = false, test = true`, std only, no
framework. Its helpers live in `src/benchkit.rs` (so tests cover them with injected samples):
- `Dist::of(reps, op)` keeps EVERY per-call sample (`Instant` around one call), sorted; `p50 = at(500)`,
  `p99 = at(990)`, `max`, `n` (the telemetry form, because a min-of-means cannot see a tail).
- Each row measures the SAME per-call step at three sizes of its axis, **1x, 10x and 100x**, and prints
  `p50 / p99 / max / n` for each size, then `ratio(label, p99_1x, p99_10x)` and `ratio(label, p99_1x, p99_100x)`.
- **Ceiling: `CEILING_PERMILLE = 3_000` (3.000x)**, the shared-CI number of `docs/04-invariants.md` that every crate's
  ratio bench uses; integer permille, `permille = at x 1_000 / base`. A **zero 1x baseline FAILS** ("UNMEASURABLE":
  work the optimiser deleted has not been shown constant). Any BREACH prints the row and the bench exits 1.
- Content rows (where the work could depend on the data, not only its size) apply the SAME ceiling between the
  content variants at one size (the indicators bench's second axis).
- `cargo bench` passes `--bench`: full measurement. Without it (`cargo test`, Route B) the file runs in SMOKE mode (one
  short pass per row, every line printed, never gating), so its code is covered and its gating exit is the (F) item of
  section 6.
- Benches write only into a NEW folder `W/scratch/psrb-preview-<n>-bench-<ts>/` and never delete it (the telemetry
  bench's `remove_dir_all` is NOT copied). Run: `nohup ( PATH=W/bin:$PATH cargo bench --locked -p psrb_preview
  --target-dir W/tgt/psrb-preview-<n> ; echo "EXIT $?" ) > <log> 2>&1 &`, waits of at most 270 s.

| id | step (per call) | function | axis at 1x / 10x / 100x | content variants | reps per size | ceiling |
|---|---|---|---|---|---|---|
| B-1 | day mask + membership answer, per stock-day | F03, F06 | stock-days answered before: 10^3 / 10^4 / 10^5 | member yes / no / unknown; Oct carry | 4,000 | p99 3.000x |
| B-2 | timeframe fold, per input minute (each k = 1, 3, 5, 15, 30, 60) | F07, F08, F09 | minutes in the folded span: 375 / 3,750 / 37,500 | padded-heavy vs clean; auction-eligible vs full | 4,000 | p99 3.000x per k |
| B-3 | hole detector per minute + previous session per day | F10, F11 | stream depth: 2,000 / 20,000 / 200,000 | holes every 3 vs none | 4,000 | p99 3.000x |
| B-4 | reading bank per candle (all 7 states) | F12..F18 | depth: 2,000 / 20,000 / 200,000 | flat / trending / alternating | 4,000 | p99 3.000x |
| B-5 | taint per candle | F19 | depth: 2,000 / 20,000 / 200,000 | a hole every 3 candles vs none | 4,000 | p99 3.000x |
| B-6 | gate + conditions + signal per candle, each rule set | F20..F29 | depth: 2,000 / 20,000 / 200,000 | all-true / all-false / one-off | 4,000 | p99 3.000x |
| B-7 | entry fill (`first_at_or_after`) per call | F30 | records in the 1-second file: 5,000 / 50,000 / 500,000 | T at 09:16 vs 15:07 | 4,000 | p99 3.000x |
| B-8 | ratio source + level mapping per candle | F31, F32 | traded seconds per minute: 1 / 10 / 60 (dense); session position 09:16 vs 15:07 | minute with G vs a walk to an earlier minute | 4,000 | p99 3.000x |
| B-9 | trailing per candle | F33 | depth: 2,000 / 20,000 / 200,000 | tighten / hold / flip | 4,000 | p99 3.000x |
| B-10 | stop scan per second; minute fallback per minute | F34, F38 | seconds already scanned in the trade: 10^3 / 10^4 / 10^5 | stop hit vs never | 4,000 | p99 3.000x |
| B-11 | flip, time and square-off fill per call | F35..F37 | records in the file: 5,000 / 50,000 / 500,000 | each exit kind | 4,000 | p99 3.000x |
| B-12 | trade simulator per candle and per second | F39 | depth: 2,000 / 20,000 / 200,000 | in a trade vs flat | 4,000 | p99 3.000x |
| B-13 | qty + fills + `price_trade` + headline + net per trade | F40..F52 | trades priced before: 10^3 / 10^4 / 10^5 | 2018 vs 2026 day; square-off vs not; every era | 4,000 | p99 3.000x |
| B-14 | candidate merge per candidate | F53 | candidates merged before: 10^3 / 10^4 / 10^5 (s = stocks fixed at 218) | tie-heavy vs spread | 4,000 | p99 3.000x; the stock-count axis (2 / 22 / 218) is REPORTED as the honest O(log s) bound, not gated |
| B-15 | slot allocator per candidate | F54 | candidates processed: 10^3 / 10^4 / 10^5 | slots empty / full / 20 open with 3 ties | 4,000 | p99 3.000x |
| B-16 | month aggregate + row stats per trade | F55, F57 | trades before: 10^3 / 10^4 / 10^5 | wins vs losses | 4,000 | p99 3.000x |
| B-17 | drawdown walker per mark | F56 | marks before: 10^4 / 10^5 / 10^6 | new peak vs below peak | 4,000 | p99 3.000x |
| B-18 | unit commit per unit (fixed unit size) | F64 | units already in the db folder: 10^2 / 10^3 / 10^4 | new vs adopted | 1,000 | p99 3.000x |
| B-19 | manifest append per row | F65 | rows in the file: 10^4 / 10^5 / 10^6 | after a torn tail vs clean | 4,000 | p99 3.000x |
| B-20 | resume skip check per unit (after the one load) | F66 | manifest rows: 10^4 / 10^5 / 10^6 | committed vs absent | 4,000 | p99 3.000x; the load is O(rows) once (stated) |
| B-21 | snapshot write + read per snapshot | F67 | snapshots in the db: 10^2 / 10^3 / 10^4 | stock vs row snapshot | 1,000 | p99 3.000x |
| B-22 | index lookup + full-key query per lookup | F68, F70 | filled slots: 10^3 / 10^4 / 10^5; runs in the db: 1 / 10 / 100 | empty vs filled slot | 4,000 | p99 3.000x |
| B-23 | audit append + progress tick + extremes update per event | F69, F72, F73 | events before: 10^4 / 10^5 / 10^6 | each event kind | 4,000 | p99 3.000x |
| B-24 | unit planning per unit emitted | F62 | units planned: 10^3 / 10^4 / 10^5 | | 4,000 | p99 3.000x |
| B-25 | worker dispatch per unit | F63 | units queued: 10^2 / 10^3 / 10^4 | | 1,000 | p99 3.000x; speed-up at 1, N/2, N workers REPORTED with parallel efficiency in permille |
| B-26 | output line (rupees + one TSV line) per line; verify's per-file lookup | F61, F76, F71 | lines written: 10^3 / 10^4 / 10^5; files: 10^2 / 10^3 / 10^4 | negative vs positive money | 4,000 | p99 3.000x; re-hashing is O(bytes), stated |
| B-27 | once-per-stock steps (universe, probe, span, warm-up) per stock | F01, F02, F04, F05 | stocks processed before: 2 / 20 / 200 | | 1,000 | p99 3.000x |

**27 benches**, each with 2 size ratios (54 gated size rows) plus their content rows. The honest non-O(1) bounds are
labelled where they sit: F57's median O(m) once per row, F58 O(m) per row, F59 O(r log r) once, F60 O(r) once, F66's
manifest load O(rows) once, F71's re-hash O(bytes), F53's O(log s) with s bounded by the universe. None of them is per
minute, candle, second, candidate or trade.

---

## 4. PARALLEL AND INCREMENTAL

- **Units.** Phase A unit = (run, stock, timeframe, rule set, month): its candidates for both sides. Phase B unit =
  (run, rule set, timeframe, account, month): its admitted trades, month row and the row's month-end snapshot. One
  worker owns one stock at a time (F63) and folds its readings ONCE per timeframe, writing each rule set's unit; workers
  never share mutable state; one writer thread owns the append-only logs (manifest, audit, progress); unit files are
  written by the workers themselves (F64).
- **Deterministic bytes.** Every output is sorted by its key, never by completion order; no hash-order dependence
  (X-F62-8); results are byte-equal for any worker count (T-F63-2, E-4).
- **Atomic, never overwritten, never deleted.** A unit is COMMITTED when its manifest row is appended; its file is
  content-addressed and linked once (F64). A kill leaves at most staging files, which stay and are listed as ORPHAN.
- **Resume after a kill** loads the manifest once into a bitmap over the dense unit slots (F66) and skips every
  committed unit; a unit linked but not yet in the manifest is ADOPTED after its trailer and sha check (X-F64-7), so a
  finished evaluation is never repeated; only in-flight units are evaluated, once. Tests: T-F66-1..3, X-F66-1..8,
  P-F66-1, E-2 (21 kills, byte-equal to an uncut run).
- **Fault hook.** `PSRB_PREVIEW_FAULT=<point>:<n>:<exit|pause>` at the seven points of E-2: `exit` calls
  `std::process::exit(86)` after printing `FAULT <p> <n>` (its profile is written); `pause` prints and blocks on stdin,
  and the test either SIGKILLs it (a real kill) or closes stdin (so the pause branch is covered too).
- **Incremental by month.** Positions never span a night, so a month boundary is a clean cut: each stock-timeframe
  chain carries a fixed-size month-end snapshot (F67) and each row its own. Adding a later month (the store gains
  2026-11) evaluates only that month's units from the 2026-10 snapshots; a revised store month (a new served
  generation) changes that month's input digest and recomputes only that stock's chain from it and the rows' months
  from it. Tests: X-F67-1, X-F67-2, X-F67-6, P-F67-1, E-3.
- **Scalable.** Every state is fixed size (`const` size asserts) and every step O(1) at p99 (section 3); memory is
  bounded per worker (X-F63-7); adding stocks, timeframes, rule sets or months adds units, never per-unit cost.

## 5. RESULTS DATABASE AND AUDIT (`PV/db/`, append-only)

```
PV/db/
  LOCK/                                    mkdir lock: one writer at a time (X-F65-5)
  runs/<run-id>/
    run.tsv                                one row per run: inputs' sha256, tool commit, start, end, counts
    audit.log                              hash-chained events (F69)
    progress.log                           the minute lines (F72)
    manifest.tsv                           one row per committed file: key, path, bytes, sha256, row crc (F65)
    index.bin                              dense index, header with radix and version (F68)
    units/<key>-<sha12>.tsv                Phase A and Phase B unit files, content-addressed (F64)
    snapshots/<key>-<sha12>.bin            month-end states (F67)
    staging/                               attempt files; never deleted; ORPHAN in verify
    trades.tsv, months.tsv, rows.tsv, counts.tsv, extremes.tsv, REPORT.md, identity.txt   (F76, F73)
```

- **run id** = the first 16 hex of sha256 over (tool commit, spec sha, ADD sha, cost table version and sha, universe
  sha, membership digest or "today's list", skip file sha, rule-set tables' sha). A changed input is a new run (or an
  add-month, which keeps every other input equal).
- **Every trade row, every month row, every run row** is a row of these files; **every file** has a manifest row with
  its sha256.
- **O(1) lookup** by (run, rule set, timeframe, side/account, stock, month): one record read in `index.bin`, one range
  read in the file it points into (F68; B-22).
- **Audit lines** for every run event: run.start with the inputs' sha256 (spec, ADD, cost table, universe, membership,
  skip file, store-month digests) and the tool commit; selfcheck; unit.done and unit.refused (with the refusal);
  fallback.day; progress; extreme; run.end with counts, refusals, fallback fills, wall and per-unit p50/p99 (F69).
- **Subcommands:** `psrb_preview query --db PV/db [--run R] [--stock S] [--month YYYY-MM] [--timeframe K] [--rule-set
  NAME]` (F70) and `psrb_preview verify --db PV/db [--run R]` (F71); `psrb_preview extremes --db PV/db --run R` prints
  extremes.tsv.
- **Monitoring:** the PROGRESS line every minute (units done, rate, ETA; F72), and extremes.tsv (worst months, deepest
  drawdowns, most holes, slowest units with p50/p99; F73) for a dashboard to read.
- Tests for each: F64..F73 above, and E-2, E-3, E-5.

---

## 6. HOW COVERAGE IS PROVED (Route B, D3 recipe v4 step 4)

1. **Set file (SET OVERRIDE, D3 step 2b),** written before measuring to `PV/coverage/set-<tip12>.txt`:
   `reading whole:psrb_preview`, then `coverage <file> whole` for EVERY `.rs` file of `crates/psrb_preview/` (src/,
   tests/ incl. tests/model/ and tests/fixtures/, benches/), `coverage crates/indicators/src/trend.rs scaled <a>-<b>`
   (the scratch getter), the five `pub mod` lines as `coverage crates/indicators/src/lib.rs line <n>`, and `coverage
   crates/indicators/src/<m>.rs whole` plus `<m>_tests.rs whole` for the five copied modules; the same lines as
   `mutants ...` (mutants stay DEFERRED by the operator's 30 Oct decision, D3 step 5: no cargo mutants command runs
   here).
2. **Pins** (D3 step 0, recorded): `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; the nightly's own llvm-cov and
   llvm-profdata `LLVM version 23.1.0-rust-1.99.0-nightly`. A mismatch is an ENVIRONMENT item, never installed or
   upgraded.
3. **Run (one background command):** K = `W/tgt/psrb-preview-<n>-deep` (its lock `mkdir K.lock`, ours; the build goes
   to `K/cov`), P = a NEW empty `W/scratch/psrb-preview-<n>-prof/<tip12>-B`, L = `W/scratch/psrb-preview-<n>-logs/<tip12>-cov`:
   `nohup ( W/state/decisions-20261027/d3-route-b.sh W/scratch/psrb-preview-<n> K P L PV/coverage/set-<tip12>.txt
   psrb_preview indicators ; echo "EXIT $?" ) > L.log 2>&1 &`, then waits of at most 270 s. It builds once with
   `-C instrument-coverage -Z coverage-options=branch` at opt-level 0, runs each package's own tests (the bin and the
   SMOKE-mode bench included: the script reads every executable the build reports, so tests that spawn
   `CARGO_BIN_EXE_psrb_preview` add their profiles), merges, shows every set file with branches, and prints
   `COVERAGE <file> <item>: lines a/b, branches c/d` plus every UNCOVERED LINE / BRANCH with its text.
4. **The bar:** 100% of lines AND 100% of branches of EVERY set item, figures written as measured. No exclusion of any
   kind (no `#[coverage(off)]`, no `cfg(not(coverage))`, no test skipped after `--`) except the recipe's own:
   - **(F)** the failing arm of `assert!`, `assert_eq!`, `assert_ne!`, `panic!` or `unreachable!` inside a `#[test]`
     body or a `#[cfg(test)]` helper only tests call (every `tests/model/*.rs` oracle is such a helper), each listed by
     file:line with its text; and in `benches/ratio.rs`, the gating `std::process::exit(1)` branch reached only when one
     of that file's own ratios breaches (CC-20's kind), named per file.
   - **(U)** none expected: production code is written without `unreachable!` arms (types make impossible states
     unrepresentable, e.g. `ExitAfterEntry`, `AcceptedBar`); any (U) found is listed with its proof from the code, per
     D3 step 4 ("hard to reach" is never a proof).
   - Production refusals (every `Refusal` return, `?` on an error, let-else, the disk floor, the fault hook, the
     NONDETERMINISM refusal) are reachable by input and each has a test above; an uncovered one is a GAP.
   - `src/benchkit.rs`'s own branches (zero baseline, exactly 3.000x passes, 3.001x breaches, empty `Dist`) are
     covered by unit tests with injected samples, so they are not (F).
5. **The copied indicator modules** are measured too (their own `_tests.rs` plus this crate's P-F13..P-F18). They must
   stay byte-equal to their part tips, so a gap there is NOT patched in the scratch copy: it is filed to that part
   (PSRB-RSI, -BOLL, -STV, -WILLR, -CCI) and the preview's coverage line for that file reads NOT PROVED until the
   part's fixed tip is copied in again.
6. **Written:** the per-item and per-file figures, every GAP with file:line and the command that shows it, and the
   (F)/(U) list at `PV/coverage/fu-<tip12>.txt`; the summary in `PV/BUILD-NOTES-HARDEN.md`. Coverage is "proved" only
   when every set file was measured and read and gaps = 0.

---

## 7. SPEC QUESTIONS (pinned to the first run's behaviour, tested as pinned, flagged; never a skip rule)

| Q | the open point | tests that pin it |
|---|---|---|
| Q1 | the trading start month has no Zerodha month (a Zerodha gap exactly there) | X-F05-4 |
| Q2 | the previous M2 session has no day bar (only padded minutes): C1/C2 false all day, or the last earlier accepted bar | X-F11-2 |
| Q3 | the stop and the time exit reach on the same second (resting stop already live vs the 15:07:59 order) | X-F34-7, X-F36-6 |
| Q4 | a flip order whose first traded second is at or after 15:10:00: a flip, or the broker's square-off (`by_broker`) | X-F35-6, X-F43-7 |
| Q5 | "the worst print since the entry" for an unclosable exit includes the entry second itself | X-F37-1 |
| Q6 | candidates that cannot fill (entry not filled, zero quantity, no ratio) hold no slot, and the order in which the slot counters are taken | X-F40-7, X-F54-8 |
| Q7 | a stored GDFL 1-second record with volume 0 (a Rule T contract break): used as stored and counted | X-F30-6 |
| Q8 | `headline_from` when both clocks give the same total | X-F51-1 |
| Q9 | two equal maximum drawdowns: whose peak and low dates are printed | X-F56-4 |
| Q10 | VIDEO-SCREEN in the report head: over VIDEO-ORIGINAL (as AV2) or over AV2 | T-F60-1 |

---

## 8. BUILD ORDER (after the start gate of section 0)

1. Tree, copies, first local commit; record shas (section 0).
2. Write the naive models (`tests/model/`) and the fixtures FIRST, then the T/X/P tests per F-section, then the code
   changes they demand; the first run's code is the starting point, so most tests should pass at once and each that
   fails is a first-run bug with its regression test.
3. `cargo test` (release and debug) through the queue; then the benches (section 3); then Route B (section 6); then
   E-9's full real run into `PV/db/` and the parity diff.
4. An independent checker re-runs the tests, the benches and Route B on the same commit, re-derives five E-1 trades by
   hand, and re-computes sampled real trades as `check<k>.md` does; any miss of the standing principles is BLOCKING.
5. Everything is recorded in `PV/BUILD-NOTES-HARDEN.md`: tree, commits, commands, PIDs, logs, figures, wall times.

---

## 9. COUNTS (taken by grep over this file: `^### F`, `^| T-`, `^| X-`, `^| P-`, `^| E-`, `^| B-`)

| what | count |
|---|---|
| functions (F01..F76) | 76 |
| named tests, all kinds | 973 |
| - normal and permutation tests (T), incl. 8 global gates and 69 exhaustive PERM tests | 260 |
| - extreme-case tests (X) | 626 |
| - of which (combo) | 152 |
| - of which (oob) | 84 |
| - property tests against a naive model (P), 73 x 10,000 + 3 x 1,000 = 733,000 generated cases on fixed seeds | 76 |
| - end-to-end tests (E), incl. the hand-computed synthetic store, kill-and-resume (21 kills), add-a-month and first-run parity | 11 |
| benches (B), each gating p99 at 10x/1x and 100x/1x against 3.000x (54 gated size ratios plus content rows) | 27 |

Every one of the 76 functions has at least 1 T, at least 8 X (at least 2 combo, at least 1 oob) and at least 1 P
(checked by an awk pass over this file before it was written). Spec questions pinned: 10 (section 7).

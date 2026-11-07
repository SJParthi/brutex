STATUS: BLOCKED QUESTION

# FIB-PREVIEW-REF: an independent reference build of the Fibonacci preview's written rules

**PREVIEW.** This is a second build of the kit's rules, run on seeded synthetic data only. It is
not a result and says nothing about profit. Agreement between this build and another one is
evidence; a disagreement is a bug in one of them.

- Session FIB-PREVIEW-REF, token `FIB-PREVIEW-REF-20261107-1` (`OWNER-FIB-PREVIEW-REF.md`).
- Branch `cloud/fib-preview-ref-20261107`, cut from `cloud/fib-preview-kit-20261107` at `b6f7ac35`.
  Before every push the branch was fetched and compared: the tip was always this session's last
  push and the token was always this one. No force push, no merge, no pull request.
- Written from the kit's words only (`reports/fib-preview-kit-20261107/`: PREVIEW-SPEC.md,
  COSTS.md, SNIPER.md, REQUIREMENTS.md, COMBINATIONS.md, CLOUD.md). The Mac tool was not
  available and was not looked for. No vendor row, no futures data, no credential, no network.

## 0. Why the status is BLOCKED QUESTION

Everything the task names is built and verified (sections 2 and 3), except four rules whose
definitions are not written in the kit. Each needs the operator's answer:

| # | Rule | What the kit says | What is missing | What this build does now |
|---|---|---|---|---|
| Q1 | XF | "stop + trailing take-profit armed at a rung (the `grid::Ttp` semantics, SPEC:569)" | `grid::Ttp`'s rule: what arms it, the trail's unit, when the trail moves, the order type | a labelled reading: XE's trail armed at a rung price (W039) |
| Q2 | XI | "a Supertrend trail with the box stop as its floor", period and multiplier generated from learning months | the Supertrend formula itself: the ATR and its smoothing, the band carry rule, the flip rule, the integer rounding | not built |
| Q3 | XE stair-step | "step to below each new pause's low (`W/state/fno-winners-20261013/pauses.json`)" | what a pause is (it lives in a state file, not in the kit) | XE's other half, the trail by a distance (W010) |
| Q4 | MTM 80% | "80% of the position's margin, the worse of the two readings named (SPEC:611-613; OP-33)" | the two readings | not built |

One standard is also not met as written: "every function 8+ extreme cases, 2 combinations, 1 out
of the box" was not audited function by function (section 7 says what was measured instead).

## 1. What was built

`crates/fib_ref`, a Rust crate with no dependency, a library and one binary:

| Module | Rules (kit section) |
|---|---|
| `money` | exact paise; floor, ceil and half-up division; rupee rounding |
| `clock` | IST seconds, 09:15 open, λ = 14 s, 15:07:59 last entry, 15:08:00 cut, 15:10:00 square-off, 15:30 close, civil dates |
| `bars` | the 1-minute Zerodha candle, the GDFL second, hole minutes (3.4), the fold to 1, 2, 3, 5, 10, 15, 30, 60 minutes on either label grid (4, A04) |
| `ladder` | the running ladder A1 and the frozen ladder, direction, the twelve rungs in milli-paise, taint, candle relations (6) |
| `rank` | the move at a moment, competition ranks, the top-N clause both ways, stale and invalid stocks (5) |
| `model` | statuses and refusals by name, the 20 readings A01-A20, R1's level mapping (6.1, 7.3) |
| `e1` | E1 flag and box, E1C and E1S, holding rules S33, height, colour, S42 (7.1) |
| `e2` | E2 touch book: states, counts, bounce, return, E2C and E2S with every end the kit names, S29b's labelled reading, S42 (7.2) |
| `e3` | E3 opening range for the eight durations, E3C and E3S, the break ordinal ORBB, S42 (25.2.1, 25.9) |
| `gates` | S-STOP and S-RR on price (SNIPER.md 1.2) and S-RR on planned net (24.13), the "at least k" clause for the seeds 2, 3, 5 (24.1) |
| `walk` | exit lanes XA, XB, XC, XD2, XD3, XE (trail), XG, XH, XJ and the stop; latency, 10% of each second's volume, worst prints, protection limits, the 15:08 and 15:10 clocks (9, 9.2, 11.1) |
| `charges` | the official reading P on the dated and today's clocks, the worst reading W, the option reading, Dhan's estimator replica (COSTS.md and its addendum) |
| `account` | the exact account: ceilings as data, admission, size steps, dated leverage, margin and exposure checks, net, months, drawdown (10, 25.1) |
| `wf` | THE RANKING, the walk-forward windows and chain, the win-rate gate, the luck rule, the verdict (14, 24.R, 24.P) |
| `synth` | SplitMix64 and the seeded minute and second generator: the only source of every input |
| `golden` | the 284 golden scenarios, `INDEX.md`, `MANIFEST.md` and the writer |

**A workspace of its own, on purpose.** `crates/fib_ref/Cargo.toml` carries an empty
`[workspace]`, so the crate is not a member of the root workspace. Joining it would change the
root's member set, its documented crate graph and the tests that pin both, which is a change to
the repository's law this session had no mandate to make. `cargo build --workspace` at the root
never sees the crate. The crate does follow the repository's file rules (Rust only, a
`benches/ratio.rs`, bench labels `FIBREF-`), and the repository's own tree-walking gates were run
with it present (section 2).

Exact integers everywhere: prices and money in paise (`i64`, sums in `i128`), rung prices in
milli-paise, ratios compared by cross-multiplication, no float on any decision path (the bench
alone prints timings as floats).

## 2. Verification, as achieved

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` (clippy `all` deny, `pedantic` warn, so every pedantic lint is an error) | clean |
| `cargo test --locked` | 130 unit tests and 2 integration tests, all passing |
| Golden files | regenerated byte for byte by `tests/golden.rs`; `MANIFEST.md` checked by that test and by `sha256sum -c` |
| Route B coverage, the library | regions 9,882 of 9,882, functions 413 of 413, lines 7,497 of 7,497, branches 772 of 772: **100.00% each** |
| Route B coverage, the binary | regions 11 of 11, functions 1 of 1, lines 7 of 7 (no branch): **100.00%** |
| Hand mutations | 38 applied one at a time, 38 killed (section 7) |
| The repository's gates with this crate present | 15 test targets passed (below) |

**Route B, the method.** `rustc -C instrument-coverage -Z coverage-options=branch` on the pinned
1.97.1 toolchain (`RUSTC_BOOTSTRAP=1`, for this measurement only; nothing in the crate needs
it), merged and reported with that toolchain's own `llvm-profdata` and `llvm-cov`. Two reports,
each from one compiled copy of the code it covers, so no copy is counted twice: the library
(every `src/*.rs`) from the unit-test binary's run, and `src/bin/fib_ref_golden.rs` from one
run of the instrumented binary. Test code is excluded from the counts: unit tests live in
sibling files `src/<module>_tests.rs` and `tests/` is ignored.

**The repository's gates.** Run from the root with `crates/fib_ref` in the tree: `core`'s
`graph`, `decision_citations`, `cost_invariants`, `claim_reader`, `lint`, `banned_lists`;
`vocab`'s `no_float`, `no_py_language`, `no_runtime_spawn`, `workspace_is_rust`,
`stale_claims`; `pull`'s `token_identity`; `store`'s `libm_key`; `cli`'s `crate_graph_claims`;
`engine`'s `production_callers`. All passed.

## 3. The rules as built, with their sources

### Rank (5)
`m(s, T) = (P - C) / C` with P the close of the stock's last 1-minute candle closed by T and C
the previous session's close, compared exactly by cross-multiplication. Competition ranks: equal
moves share a rank and a tie at the edge of top N includes every tied stock. A long reads gainer
ranks, a short loser ranks. A stock with no valid previous close leaves the pool, as does a stale
price over the limit. A frame is a sort (named not O(1), B10); a lookup after it is O(1).

### Ladder (6)
A1: the session's highest high and lowest low so far; candle i is judged against the ladder of
candles 0..i-1 and then folds into it. Direction: Up when the high is the later extreme, Down when
the low is, Undetermined when one candle holds both (unused until a later one-sided extreme).
Twelve rungs 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1, 1.272, 1.618, 2, 2.618, 3, exact in
milli-paise. A folded hole taints the ladder for the session (the reading that admits less).
Frozen mode freezes the ladder at the box's first candle (E1) or at the first touch (E2).

### E1, the flag (7.1)
Push extreme: the candle of the last strictly new session high. A box of K complete candles after
it; holding rule S33 on rung L (all lows, all closes, no close below the first box low, or none),
optional height in ppm of R, optional colour. E1C: the first candle after the box closing above
the box high, T = its close. E1S: armed at each close where a box exists, the first traded GDFL
second from that close + 14 s to the next close whose high exceeds the R1-mapped box high, T =
that second + 1. Stop: box low less the offset; T1 the session high at T plus the offset; T2
1.272 and T3 1.618 of the target ladder. A decision after 15:07:59 is refused,
`after_last_entry` (S42, as E2 and E3 do; added this session after a review found E1 lacked it).

### E2, the second touch (7.2)
Relations at tolerance t: touch `low - t <= L <= high + t`, clear above, through below. States
Idle, At, Away, Ended. The first touch comes from the favourable side; the bounce is the best high
of clear candles since the touch, wiped by a through candle, passed at the m-th rung above L. The
return is the first candle of a later touch once the bounce passed: n advances, the gap is
counted. A hole or incomplete candle ends the count (`touch_count_unknown`); a signature change
restarts it; S29b's labelled reading ends it on a close beyond the level by more than t. E2C: the
touch candle's close. E2S: armed where the bounce passes; the first traded second from that close
+ 14 s whose low reaches the R1-mapped `L + t` decides at that second + 1. Its ends: a signature
change, the close of a hole minute, a through candle's close, the close of the touch candle it is
armed for, S42; at a tie the end wins. `no_fill_data_window` when the arming close + 14 s is
before the data start, `no_trigger_ratio` when R1's minute has no price, the resting reading
refused, `touch_order_unsourced`.

### E3, the opening range (25.2.1 with 25.9)
`H_d`, `L_d` over the first d minutes, d in 1, 2, 3, 5, 10, 15, 30, 60. A hole inside the range:
`orb_tainted`; `H_d = L_d`: `orb_no_width`. No record on or before the range's close (A08).
E3C: every fresh close above `H_d` is a break, numbered by its ordinal (ORBB clauses first,
second, second or later, any). E3S: armed at each close at or inside `H_d`, triggered by the first
traded second from that close + 14 s beyond the R1-mapped `H_d`. Stop `L_d` less the offset.

### Gates (24.1, 24.13, SNIPER.md 1.2)
From REF and the R1-mapped stop and first target, never kinder: `S-STOP = ceil(D_stop x 10^6 /
REF)` ppm; S-RR on price; S-RR on planned net `floor(1000 (D_tgt q - charges) / (D_stop q +
charges))` with `q = floor(amount / REF)` and the round trip's charges at REF; `D_tgt = 0` gives a
real negative value; `q = 0` is `sn_no_quantity`; XG, XH, XI are `s_rr_no_target`. The clause "S-RR
at least k" for the labelled seeds 2, 3, 5 prints `ok` or `rr_band` on both charge readings.

### Walk (9, 9.2, 11.1)
A decision at T starts its order at the first traded second at or after T + 14 s; a resting stop
or target acts from the second its trigger prints; an order move (XD, XE) is decided at the end of
the second that triggered it and takes effect 14 s later. At most 10% of each second's traded
volume, shared by entry and exit, every slice at that second's worst print (a buy at the high, a
sell at the low); a zero-volume second fills nothing. Protection limits at LTP x 1.03 for a buy
and x 0.97 for a sell (Rs 1.50 at or below Rs 50), a buy limit floored and a sell limit ceiled.
Whole shares at the last print before the start. A stop and a target in one second: the stop. The
entry's remainder is cancelled at 15:08:00; a strategy exit starting at or after 15:08:00 is
rejected; from 15:10:00 the broker squares off at the worst print from 15:10:00 to the session's
last second, or, with none, the worst since the entry (`unclosable_in_session`). A short walks as
the long on the price mirror; protection limits are computed on real prices first.

### Charges (COSTS.md, its 7 Nov addendum, REQ 25a)
The official reading P is the headline: brokerage Rs 20 or 0.03% per executed order, the NSE
charge with IPFT on turnover, the SEBI fee, STT on NSE's VWAP basis (0.025% x min(BQTY, SQTY) x
VWAP, the day's total to the rupee), stamp duty (state law before 2020-07-01, 0.003% of the buy
after it, to the rupee), GST 18% (on the SEBI fee from 2022-07-18 on the dated clock), the
square-off fee Rs 20 + GST per order. Priced on the trade date's rates and on today's; the
headline is the dearer. The worst reading W prices every open choice at its worse value and
decides nothing. All 24 COSTS.md contract notes match on both clocks and both readings.

**The two golden cases:**
- Adani, 547 x Rs 2,742.00 round trip: Dhan's screen **Rs 578.85** is reproduced to the paisa by
  the estimator replica (buy Rs 124.44 + sell Rs 454.41). The official reading is Rs 579.41 (the
  figure 25.1 gives for the calculator that passed); the worst reading Rs 594.17. C025.
- HDFCBANK 770 CE, 11,700 x Rs 26.95 round trip: Dhan's screen **Rs 794.67** is reproduced to
  the paisa by the replica (buy Rs 165.58 + sell Rs 629.09, the buy's stamp at Rs 9.46). The
  official option reading is Rs 794.33, within Rs 1. C026.

### Account (10, 25.1, 24.T and the day-cap ruling)
Ceilings are data `(day cap, month cap, amount, margin)`: main (5, 100, Rs 15 lakh, Rs 3 lakh) and
ceiling B (10, 100, Rs 10 lakh, Rs 2 lakh). Admission in start-second order, then decision, then
symbol, with no look-ahead: one position per stock (`while_open`), `day cap`, `month cap`, N slots
(`slots_full`). Full step `min(ceiling amount, Rs 1 crore / N)`, quantised down to the largest of
Rs 15, 10, 7.5, 5, 2.5 lakh whose margin at the stock-day's dated leverage fits the ceiling
(`margin_cap`); at the start, the margin check against closed equity and the exposure check
against Rs 1 crore may take it one step lower; below the half step `size_refused`. Net = money in
- money out - charges; a win is net above 0. The data charge, Rs 499 + GST = Rs 588.82, is due in
each calendar month with a trade. Max drawdown in tenths of a percent of Rs 20 lakh.

### Walk-forward, gate and verdict (14, 24.R, 24.P)
THE RANKING: passers first (every month has a trade and nets at least the fixed Rs 2,50,000),
then the lowest drawdown, the most net, the highest win rate, fewer trades, shorter holding,
higher net per trade, the id; misses by the least total shortfall, then the best worst month,
then the same keys. Seven anchored windows Y2020..Y2026, 82 unseen months. The win-rate gate
`W > L` and `(W - L)^2 x 1000 >= Zsq x (W + L)` exact in `u128`, seeds 2,706, 5,412, 9,548. The luck
rule `(1 + #{b : best_b >= O}) x 20 <= 1001`. The verdict's rules 1 to 7 in order, line 2 only.
Whole option lots round down (`lot_above_premium` when one lot costs more than the amount).

## 4. The readings: every ambiguity with both readings and a scenario for each

A scenario's head line `readings` is `headline` or the readings it changes, `Axx=name`.

| Id | Question the kit leaves open | Headline reading: scenarios | Other reading: scenarios |
|---|---|---|---|
| A01 | "top 10 gainers" when fewer than 10 are up | competition rank only: a falling stock can rank as a gainer | `signed_only`: a gainer must be up. R011 prints both columns for every stock (all R scenarios do) |
| A02 | an equal session extreme | strict: the extreme moves only on a strictly new high or low, L010 | `latest`: an equal extreme moves its index, L011 |
| A03 | which holes taint the ladder | any folded hole, for the session: L012, E1-022 | `extreme_candles`: only a hole in a candle that set an extreme: L013, E1-023 |
| A04 | the label grid of tf-minute candles | labels from 09:15: L017 | `clock`: labels on multiples of tf from midnight, 09:15-10:00 first on 60 minutes: L018 |
| A05 | E1 after a break | consumed until a new extreme: E1-024 | `next_box`: re-armed on the next box: E1-025 |
| A06 | which K candles form the box | sliding, the K right before the break candidate: E1-026 | `anchored`, the K right after the push: E1-027 |
| A07 | a re-touch before the bounce passes | joins the touch: E2-020 | `ignore`: E2-021 |
| A08 | E3's candle closing exactly at the range's close | writes no record: E3-016 | `arms` (E3S): E3-017 |
| A09 | an entry fill and an exit trigger in one second | entry first: W004, W006 | `exit_first`: W005, W007 |
| A10 | a resting stop printing after 15:08 | acts: W016 | `rejected`, the broker squares off: W017 |
| A11 | a zero-fill entry and the day count | released: A015 | `kept`: A016 |
| A12 | an amount below N = 1's half step | allowed: A012 | `refused`: A013 |
| A13 | drawdown tenths | floored: A017, K008 | `ceil`: A018, K009 |
| A14 | when the data charge enters the curve | at the month's first fill: A008, A017, K008 | `out_of_curve`: A009, A019, K011 |
| A15 | stamp duty before 2020-07-01 | the stated states (Telangana, Tamil Nadu): C029 | `literal`: every listed state, Himachal Pradesh's Rs 50 a day: C030 |
| A16 | option brokerage | a flat Rs 20 an order: C027 | `min20`: min(Rs 20, 0.03%): C028 |
| A17 | the ladder rung targets are read on | after folding the decision candle: E1-034 | `judging`, the ladder the candle was judged against: E1-028 (same inputs) |
| A18 | when a stock's lock and a slot end | inclusive of the exit's last fill second: A004 | `exclusive`: A005 |
| A19 | an interim report's line 2 | always PENDING: V012, V014 | `no_can_print`, a NO prints: V013 |
| A20 | verdict rules 5 and 6 when both could speak | rule 5 first, PENDING: V009 | `six_first`, NO: V015 |

## 5. Contradictions between documents, and readings outside the table

| Item | One reading | The other | This build | Scenarios |
|---|---|---|---|---|
| Adani official total | 25.1: Rs 579.41 (the calculator that passed) | COSTS.md addendum: Rs 579.43 (worst Rs 594.18) | Rs 579.41 and worst Rs 594.17, from the stated rules; no rule found that gives 579.43 or 594.18, so the other reading has no scenario | C025 prints every component |
| HDFCBANK buy-side stamp | the screen's line, Rs 9.00 | Rs 9.46, which the screen's own total Rs 794.67 needs | replica Rs 9.46 (0.003% to the paisa) reproduces Rs 794.67; the official reading rounds the stamp to the rupee, Rs 9.00 | C026 prints both |
| 24.13's worked charges | the vectors' charges (41,677; 15,167; 41,671 paise), which equal this build's worst reading W | 24.V: the official reading P is the headline (40,201; 13,690; 40,195) | every gate scenario prints S-RR on P, on W and on the given charges: the vectors' 161, -420, -151, 1,619, -120, 4,901 on W; 195, -403, -74, 1,631, -117, 4,905 on P | G007-G012 |
| E2's second reading at n = 1 | the kit: a record refused by name, `no_pattern_extreme` (S50 has no touch yet) | this build arms the second reading only once a bounce has passed, so the first touch gets no arming and no record | both enter nothing; this build never prints the name. The other reading needs an arming moment for a first touch that the kit does not give | E2-031 (this build's) |
| XF's semantics | the kit's words, a trailing take-profit armed at a rung | `grid::Ttp`'s own rule (Q1) | XE's trail armed at a rung price, labelled | W039 |
| The E2 bounce rung | the m-th rung above L ordered by price over all twelve rungs, extensions included | the retracement rungs only (0 to 1) | price order over all twelve; the other is not built (it differs only for a bounce past the high) | E2-007 (m = 2) |
| XD's entry price | the average fill price floored to the step for a long, ceiled for a short (never kinder) | the exact average, or rounded the other way | floored or ceiled against the position; the other readings are not built | W009 |
| The chain | each window's pick with its own unseen-month series | one account re-run over the chained picks' signals (caps and slots carried across a window boundary) | series level; the other is not built | K001-K006 |
| Refusal order | the order in FORMAT.md | any other order | when several clauses fail the first in FORMAT.md's order is printed; the entry decision is the same under every order | every refused record |
| Names the kit does not give | - | - | `slots_full` (all N slots taken) and `no_sizing_price` (no print before the start to size it) are this build's names | A004, W021 |
| `exposure_cap` | reachable | unreachable | N x the full step is at most Rs 1 crore by the amount formula, so the admission path cannot reach it; unit-tested on a constructed state; no input reaches it, so no golden scenario | - |
| Charge dates on a Saturday | - | - | C025-C028 use 2026-11-07, the day the screens were read: the date selects the rates in force, it is not a trade day | C025-C028 |

## 6. The golden scenarios

`reports/fib-preview-ref-20261107/golden/`: 284 scenarios and `FORMAT.md`, `INDEX.md`,
`MANIFEST.md` (287 files).

- **Format** (`FORMAT.md`): one `<ID>.md` per scenario; its records are the lines of its one
  fenced `text` block, split on single spaces: `fibref-golden 1`, `id`, `kind`, `seed`,
  `readings`, `title`, then `in` records (every input written out in full: nothing needs
  regenerating), `out` records (every one must match) and `end`. Prices in paise, rung prices in
  milli-paise, times `HH:MM:SS` IST.
- **Manifest**: `MANIFEST.md` holds one `sha256  name` line per file in the `sha256sum` form, so a
  separate program (or `sha256sum -c` over the fenced lines) can check every file. SHA-256 is
  implemented in the crate (FIPS 180-4) and tested against the standard vectors.
- **Pinned**: `tests/golden.rs` regenerates every file and fails on any byte of difference, on a
  missing file or on an extra one. To regenerate: `cargo run --bin fib_ref_golden -- <dir>`.
- **Seeds**: every input comes from `synth::SplitMix64` with the seed the file names. No vendor
  data anywhere.

| Kind | Count | Covers |
|---|---:|---|
| R | 15 | rank: ties, a twelve-way edge tie, a falling market, stale and invalid stocks |
| L | 18 | ladder on every timeframe, both extremes in one candle, equal extremes, holes and padded minutes, a Down leg, the A04 grids |
| E1 | 34 | the flag on K = 1..4 and five timeframes, both sides, every clause, holes, E1S, S42 |
| E2 | 31 | the touch book, clauses = 1, = 3, >= 2, >= 3, bounce m, tolerance, frozen, S29b, holes, re-touches, E2S and its ends, S42 |
| E3 | 22 | the eight durations, ORBB clauses, both refusals, E3S and its data window, S42 |
| G | 26 | SNIPER.md 1.2's vectors, 24.13's vectors, refusals, seeded cases, the at-least-k clause |
| W | 39 | latency, volume cap, worst prints, protection, the clocks, every lane, both sides, seeded walks |
| C | 38 | the 24 contract notes, the two screens, the option and stamp readings, multi-fill trades, a square-off |
| A | 27 | the account's caps, locks, slots, margin, leverage, zero fills, months and drawdown, seeded accounts |
| K | 14 | THE RANKING over seven windows, its keys, the gate's boundaries, the luck rule, whole lots |
| V | 15 | the verdict's rules in order and both interim readings |
| X | 5 | days end to end: E1C, R1-mapped levels, each lane walked, priced |

**The edge cases the task lists:**

| Edge case | Scenarios |
|---|---|
| long and short mirrors | E1-005..E1-007, E2-002, E2-026, E3-002, E3-004, W003, W008, G002, X002, X004 |
| ties | R001-R010, R012, R015, L010, L011 |
| hole | L012-L014, E1-021-E1-023, E2-017, E3-012, E3-019 |
| a candle with both extremes | L001-L009 |
| the 6th trade of a day | A001 |
| the 101st of a month | A003 |
| stop and target in one second | W002, W003 |
| stop through at the first fill | W004, W005 |
| a second with no trade | W001, L015, E1-031, E2-015, E2-028, X005 |
| the 15:08 clock | W013, W015, W017, W018, E1-033, E2-030, E3-022 |
| the 15:10 clock | W013, W014, W019, C038 |
| a tiny stop where charges decide S-RR | G005, G007 |
| whole lots rounding down | K014 |

**Every refusal by name.** Each name the crate can print appears in at least one scenario:
`incomplete_candle` E1-021, `tainted_ladder` E1-022, `ladder_undetermined` E1-016,
`holding` E1-001, `height` E1-015, `colour` E1-013, `not_in_top_n` E1-012, `touch_number` E2-001,
`touch_count_unknown` E2-017, `no_fill_data_window` E3-020, `no_trigger_ratio` E1-031,
`no_level_ratio` E2-015, `touch_order_unsourced` E2-013, `orb_tainted` E3-012, `orb_no_width`
E3-013, `no_ladder_target` E3-014, `sn_stop_through_at_decision` G003, `sn_no_quantity` G014,
`s_rr_no_target` G013, `rr_band` G010, `after_last_entry` E1-033, E2-030, E3-022, `day cap`
A001, `month cap` A003, `margin_cap` A006, `size_refused` A013, `while_open` A004, `slots_full`
A004, `price_above_position` W022, `no_sizing_price` W021, `lot_above_premium` K014,
`break_ordinal` E3-001, `unclosable_in_session` W014. Four names exist and are never printed:
`no_pattern_extreme` (section 5), `exposure_cap` (section 5), `sn_no_stop` (every lane here has a
stop) and `stop_band` (S-STOP has no seed clause; its bands are learning-month quantiles this
build does not generate).

## 7. Tests and their standard

130 unit tests (account 11, bars 7, charges 8, clock 6, e1 13, e2 12, e3 8, gates 5, golden 6,
ladder 7, model 5, money 8, rank 5, synth 4, walk 16, wf 9), most with many cases each, and the
two golden integration tests. They include all 24 contract notes on both clocks and both
readings, both screens to the paisa, SNIPER.md's six vectors and 24.13's six, the gate's
boundaries on the three seeds, and both sides of every boundary this session found (15:07:59,
the 14 s data window, the 15:10:00 print, a zero-net trade).

**Each test red under a hand change.** 38 hand mutations, each one production line changed, the
whole suite run with `--no-fail-fast`, the file restored:

| # | Change | Killed by |
|---|---|---|
| M01 | half-up rounding made half-down | the contract notes, CN01, the screens, `half_up` |
| M02 | ceil division made floor | leverage, R1 mapping, SNIPER.md, protection limits |
| M03 | latency 14 s made 13 s | the three data-window tests, the named instants, golden |
| M04 | last entry 15:07:59 made 15:08:00 | E1C, E1S and E2 S42 tests, golden |
| M05 | brokerage cap Rs 20 made Rs 21 | the contract notes, both screens |
| M06 | GST 18% made 18/101 | the contract notes, Adani |
| M07 | STT on min(BQTY, SQTY) made max | the unequal-quantity STT test |
| M08 | pre-2020 stamp rate changed | the state stamp readings, golden |
| M09 | S-STOP ceiled made floored | SNIPER.md's vectors, golden |
| M10 | S-RR's denominator sign flipped | 24.13's vectors, golden |
| M11 | strict new high made latest-wins | the equal-extremes test, golden |
| M12 | E1C's S42 bound a minute later | E1C S42, golden |
| M13 | E1S's S42 read on the trigger second | E1S S42 |
| M14 | E1S's height cap dropped | the E1S height test |
| M15 | E2's S42 bound a minute later | the E2 boundary test |
| M16 | E2S's signature end dropped | E2S's ends |
| M17 | E3's range-close candle allowed to break | E3S's blind seconds, golden |
| M18 | E3C's fresh-cross rule dropped | E3C's ordinals and clauses, golden |
| M19 | E3S's data window without latency | the E3S data-window test |
| M20 | volume cap 10% made 100% | five walk tests, golden |
| M21 | entry fills at the best print | four walk tests, golden |
| M22 | stop-through-at-first-fill made strict | golden |
| M23 | square-off window from 15:10:01 | the 15:10:00 print test |
| M24 | buy protection 3% made 4% | protection tests, golden |
| M25 | one position per stock dropped | the lock test, golden |
| M26 | 5 a day made 6 | the sixth-trade test, zero-fill release, golden |
| M27 | 100 a month made 101 | the 101st test, golden |
| M28 | passers no longer rank first | THE RANKING, windows, golden |
| M29 | less net ranks first | keys 4-8, THE RANKING, golden |
| M30 | the verdict needs two twins, not three | the verdict's rules, golden |
| M31 | a zero-net trade counted a win | the zero-net test |
| M32 | the loser competition rank off by one | rank tests, golden |
| M33 | net without charges | golden |
| M34 | E1S's data window without latency | the E1S data-window test |
| M35 | E2S's data window without latency | the E2S data-window test |
| M36 | the A04 clock grid made the session grid | the clock-grid test, golden |
| M37 | the A20 rule order inverted | the verdict's rules, golden |
| M38 | "at least k" made strictly above | the at-least-k test |

The first passes left six changes alive (M07, M15, M19, M23, M31 and M35 in their first form);
the tests that kill them were written then. One more change survived because it was equivalent: the rank
frame's tie-break by symbol inside the sort could not be observed (tied moves share one rank), so
that tie-break was removed rather than tested.

**Not verified as written:** "every function 8+ extreme cases, 2 combinations, 1 out of the
box". No function-by-function count was made. What was measured instead is the coverage above
(every region, line and branch of every function executed) and the mutation table.

## 8. Benches (p99, 10^3 to 10^7)

`cargo bench --bench ratio` on this cloud container (4 vCPU, shared): 2,000 samples a size, p50
and p99 in nanoseconds a step. Rows that time a whole sequence of n items per sample (B02, B03,
B09, B10) take 20,000,000 / n samples clamped to 5..2,000, so at 10^7 their p99 is the largest
of 5 samples: read those p99s as a maximum, not a percentile. A step prints FLAT when p99 at
the largest size is at most 1.5 x p99 at the smallest and p99 is at most 3 x p50 at every size;
the verdict is printed as measured, never adjusted. Final run (the run before it is in brackets):

| Bench | Step | p50 / p99 at the smallest n | p50 / p99 at the largest n | Verdict (earlier run) | Complexity |
|---|---|---|---|---|---|
| B01 | ladder fold, one candle | 4.0 / 4.1 ns (10^3) | 4.4 / 7.3 ns (10^7) | NOT FLAT 1.78 (FLAT) | O(1) |
| B02 | E1C box machine, per candle, K = 16 | 7.9 / 25.7 ns | 70.2 / 75.9 ns | NOT FLAT (NOT FLAT) | **not O(1)**: O(K) per candle, and the call copies the whole candle slice and builds one ladder a candle, O(n) memory; per-candle time rises once the working set leaves the caches |
| B03 | E2 touch book, per candle | 169.3 / 381.6 ns | 281.8 / 302.2 ns | FLAT (FLAT) | O(1) with a constant 12-rung sort |
| B04 | walk, one 5-minute round trip | 3,082 / 7,079 ns | 3,457 / 8,770 ns | FLAT (NOT FLAT) | **not O(1) in the day's seconds**: the start and each LTP are binary searches, O(log n); each walked second O(1) |
| B05 | account admission, one offer | 116.0 / 534.8 ns | 220.0 / 617.5 ns | NOT FLAT (NOT FLAT) | O(1) amortised; a heap of at most N + 1 events, O(log N) |
| B06 | charges, one round trip, both clocks | 711.3 / 1,264 ns | 369.5 / 758.4 ns | FLAT (FLAT) | O(fills) |
| B07 | S-STOP and S-RR, one record | 450.1 / 962.8 ns | 451.9 / 887.0 ns | FLAT (FLAT) | O(1) |
| B08 | THE RANKING, one comparison | 12.6 / 16.9 ns | 13.0 / 13.1 ns | FLAT (NOT FLAT) | O(1) |
| B09 | a series' key over its months | 5,527 / 10,960 ns (10^3) | 4.93 / 9.28 ms (10^6) | NOT FLAT | **not O(1)**: linear in the series' trades, once per series and window |
| B10 | rank frame of a pool of P | 5,774 / 11,547 ns (218) | 7.15 / 11.96 ms (10^5) | NOT FLAT | **not O(1)**: a sort, O(P log P) per frame; a lookup after it O(1) |

B01, B04 and B08 changed verdict between two runs with no code change in their path, and B05
failed the spread test (p99 above 3 x p50 at some size) in both runs while its p99 at the largest
size stayed within 1.5x of the smallest: on this shared container these p99s move by 2-3x run to
run, so those verdicts are noise around a flat p50, not evidence either way. Not benched and not O(1): building a golden file, the
account's `run` (a sort of its signals), and E2's bounce-rung sort (constant, twelve rungs).

## 9. Limits, and what is not built

- **Blocked on a definition** (section 0): XF beyond its labelled reading, XI, XE's stair-step,
  MTM 80%.
- **Seeds only.** The kit's generated dials take values from learning months, which a synthetic
  reference build does not have. Built at their seed or headline: S12 (in top N at T), S29 and
  S29a gaps (off), S29c (off), S29d (held at the arming), S65 (never), V-BOTH (not run; the engine
  direction rule is the headline), S-STOP bands (seed "any"), S-RR (seed "any" plus the labelled
  2, 3, 5). Not built: the generated bands themselves, the luck test's masks (only its clearing
  rule is built), a confidence range for the win rate, price bands and tick-size rounding beyond
  R1's step.
- **Synthetic only.** Every input is seeded. Nothing here was run on a real stock-day, and nothing
  here is a result or says anything about profit.
- **Measurement aids.** `RUSTC_BOOTSTRAP=1` was used only to measure branch coverage; the crate
  builds, tests and benches on stable 1.97.1 without it.

PREVIEW: an independent reference build on seeded synthetic data; not a result, says nothing about
profit.

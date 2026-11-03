# parts.json (copied as Markdown: the repository tracks .json only under .claude/)

```json
[
  {
    "name": "FNO-SEAM",
    "goal": "Wave 0, mechanical, comments and blank lines only: one slot line per part in every registry file more than one part edits, so no two FNO parts ever edit the same line (tested in a scratch git repo: insertions under different lines merge clean, end-of-file appends conflict).",
    "files": [
      "crates/runner/src/lib.rs (slots after line 80)",
      "crates/runner/benches/ratio.rs",
      "crates/indicators/src/lib.rs (slots after line 73)",
      "crates/indicators/benches/ratio.rs",
      "crates/cli/src/lib.rs (slots after line 400)",
      "crates/cli/benches/ratio.rs",
      "docs/04-invariants.md",
      "docs/05-decisions.md",
      "docs/06-limits.md",
      "docs/07-plan.md"
    ],
    "depends_on": [],
    "tests": "No new tests; cargo fmt --check, clippy -D warnings and cargo test --workspace --locked unchanged and green. The coverage set holds no instrumented region.",
    "benches": "none"
  },
  {
    "name": "FNO-LAW",
    "goal": "Bring CLAUDE.md, AGENTS.md, docs/00-charter.md and the ledgers to the operator's 30 Oct decisions: the 218-name F&O path as data, PIT membership, the survivorship label, the ICICI join, one GDFL-second fill model, two cost clocks (worse per trade), rules 2-7, no vocabulary widening, 1day decided on D and traded on D+1, the slot and family process, and OP-01..OP-46 (D-8100..D-8109).",
    "files": [
      "CLAUDE.md",
      "AGENTS.md",
      "docs/00-charter.md",
      "docs/05-decisions.md (slot)",
      "docs/04-invariants.md (slot)",
      "docs/07-plan.md (slot)",
      "crates/core/tests/fno_law.rs (new)"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "fno_law.rs: CLAUDE.md s1 holds the label verbatim and the universe-file rule; docs/05 heads D-8100..D-8108; the charter names all 25 cost-row ids and the C_VAR1 source. decision_citations, crate_graph_claims, banned_lists and graph stay green. No invariant row cites a test that does not exist.",
    "benches": "none"
  },
  {
    "name": "FNO-COSTS",
    "goal": "Make crates/costs match COSTS.md and costs-table.json exactly: the dated cash-equity intraday clock (25 rows, exact Ratio), the pre-2020 state stamp (worst state), every RD-* worst reading, both clocks per trade with headline = max(dated worst, today worst), the table version costs-table/2026-10-30/r1 for the identity, and the 24 contract notes as tests.",
    "files": [
      "crates/costs/src/equity_dated.rs (new)",
      "crates/costs/src/equity_readings.rs (new)",
      "crates/costs/src/equity_clock.rs (new)",
      "crates/costs/src/schedule.rs (dated_card Equity(Intraday) arm; DATED_RATES_VERSION r3)",
      "crates/costs/src/lib.rs",
      "crates/costs/tests/fno_contract_notes.rs (new)",
      "crates/costs/benches/ratio.rs",
      "docs ledgers at slot D-8110"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "CN01..CN24 under four columns, asserted to the paisa, plus the headline. Every rate boundary on the trading day before and after it (2019-04-01, 2020-06-01, 2020-07-01, 2021-01-01, 2021-04-01, 2022-07-18, 2023-04-01, 2024-04-01, 2024-10-01, 2026-03-01). Each reading alone and the interacting pairs (per-fill brokerage below Rs 66,666.67, STT on the sell value, the half-paisa VWAP). The square-off fee. The tie priced both ways. Golden V1-V5 unchanged. Delivery and out-of-span dates refused by name. Property: headline >= every single-clock total, and worst >= official.",
    "benches": "C-FNO-COSTS-01 rates_on p99 <= 200 ns flat; C-FNO-COSTS-02 price_trade per fill p99 <= 2 us flat over 1..512 fills; C-FNO-COSTS-03 worst over every reading p99 <= 30 us per trade (all ESTIMATE)"
  },
  {
    "name": "FNO-SEC-EXEC",
    "goal": "The one GDFL-seconds fill walk, operator rule 6 exactly. Pure (no I/O, no telemetry). Fills at that second's high (buy) or low (sell) via costs::fill::Anchor::PrintedExtreme. A volume-share cap per second with carry to later traded seconds; no fill in a zero-volume second. Latency lanes. Order kinds: entry, stop-entry, stop, target, time, condition, broker and MTM square-off. 3% market protection. Band refusal through a Bands trait. 15:08 rejection and 15:10 square-off. The remainder rule and unclosable flag. MTM 80%. One per-second budget shared by entry and exit. The 2019-05-30 start and the 2021-02-24 outage.",
    "files": [
      "crates/runner/src/fno_exec/ (mod.rs, walk.rs, order.rs, session.rs, tests.rs) (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_exec.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8120"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Every fill-walk case of SPEC 17: zero-volume second, minute gaps, caps at no cap/100/50/25/10%/minimum, floor(c*v)=0, entry and stop in one second, stop with Q_f=0, target filled below its level, band-locked buy and sell, 3% protection, halt, 2019-05-30, 2021-02-24, auction-era session ends and the dropped auction print, remainder, unclosable, exit start at 15:07:59 and 15:08:00, MTM 80%, LTP <= Rs 50, source error, i128 guards. Properties P1-P7: conservation, worst-price only, no zero-volume fill, cap bound, no fill before start + latency, cap monotonicity, no look-ahead.",
    "benches": "C-FNO-EXEC-01 one second with 20 positions + 20 pending + 6 lanes, p99 <= 1.5 us flat over 1x/10x/100x; C-FNO-EXEC-02 remainder and protection checks p99 <= 100 ns (ESTIMATE)"
  },
  {
    "name": "FNO-LADDER",
    "goal": "Fibonacci ladders on every anchor (day running and frozen, push, swing with k confirmation, ORB of any N, yesterday, last k sessions, gap, measured move). Extensions in the move's direction (CurDayFib bits 121-131 untouched). The continuous position, rung relations, streaks and packed flag. Operator rule 2 by type (judge with ladders whose known_at <= candle open, then fold, then set known_at to the close; usable_at(ts) for seconds; tainted ladders on incomplete candles). Rule 3: Leg::Undetermined by default, with the labelled V-SEC and V-BOTH variants. Tolerance passed in, never pinned.",
    "files": [
      "crates/indicators/src/fno_ladder/ (mod.rs, anchor.rs, rung.rs, relation.rs, swing.rs, tests.rs) (new)",
      "crates/indicators/src/lib.rs (slot)",
      "crates/indicators/benches/ratio/fno_ladder.rs (new)",
      "crates/indicators/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8130"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "On 1/2/3/5/10/15/30/60 min and 1day: a new-high candle judged against the old ladder; an engulfing candle judged, then Undetermined; the first candle alone; (true,true) then (false,false); a one-sided extreme used from the next candle; a frozen ladder never changes; ORB N first used at 09:15+N, including non-dividing timeframes; a swing used only after its confirmation; yesterday and k sessions from the first candle; the gap leg; the seconds walk never reads K > s; V-SEC (same-second extremes, feeds disagree, 1-min sub-variant); V-BOTH labels; a tainted ladder; extensions in direction; R=0 and a one-tick range; CurDayFib tests unchanged. Property: shuffling the future changes no judgement.",
    "benches": "C-FNO-LADDER-01 one candle with 16 ladders x 14 rungs, p99 <= 2 us, flat from 1k to 200k candles (ESTIMATE)"
  },
  {
    "name": "FNO-DAYMASK",
    "goal": "Which stock-days may trade (SPEC 3, M1-M11), carrying the 21:19 operator decision: the 218-name universe file with its digest; point-in-time membership within it (MembershipUnknown refused, never filled); the LABEL constant; the ICICIPRULI->ICICILIFE read-time join working before and after the D-7311 re-key; a normalised typed skip table built from the 79 audit rows (data file outside the repo, checked one-to-one); DayPlan session clocks, hole minutes and flags; per-reason counts and the PIT count outside the 218.",
    "files": [
      "crates/cli/src/fno_daymask/ (mod.rs, universe.rs, join.rs, skip.rs, plan.rs, tests.rs) (new)",
      "crates/cli/src/lib.rs (slot)",
      "crates/cli/benches/ratio/fno_daymask.rs (new)",
      "crates/cli/benches/ratio.rs (slots)",
      "state/fib-strategy-20261030/inputs/fno-skip-v1.tsv (data, outside repo)",
      "docs ledgers at slot D-8140"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Exactly 218 names; the 13 new names admitted; BAJAJHLDNG and DALBHARAT never in; member day traded, non-member refused, membership-unknown refused; the label byte for byte; the ICICI join in both store states, either spelling a member, Oct 2026 skipped; every normalised skip rule; BE/T2T from an input table, else the label 'T2T unchecked'; Muhurat and special sessions; 2019-05-30; 2021-02-24; the 17 market-wide Kite-hole days and per-stock hole minutes; demerger ex-days; ZOMATO/ETERNAL; the 12 no-Zerodha stocks counted under M5; a provisional last day; the 13 and 23 Mar 2020 halt flags. Properties: rule load order is irrelevant; counts sum to member-days.",
    "benches": "C-FNO-MASK-01 one stock-day verdict p99 <= 3 us, flat in the days held (ESTIMATE)"
  },
  {
    "name": "FNO-LEVERAGE",
    "goal": "Each stock's dated intraday leverage from NSE C_VAR1 batch-1 files: parse the EQ rows (columns 7, 8, 9), build a fixed-stride dated table reference/nse-margin-v1, O(1) lookup, leverage = min(5, 100/max(20, M)) exact, under three readings (that day, today, with the column-9 extra margin), with a file digest. No network code; the download is an operations step needing the operator's yes.",
    "files": [
      "crates/pull/src/nse_margin.rs (new)",
      "crates/pull/src/lib.rs",
      "crates/pull/benches/ratio/nse_margin.rs (new)",
      "crates/pull/benches/ratio.rs",
      ".github/gates_tree.rs (one literal group, READINESS ADDENDUM item 12)",
      "docs ledgers at slot D-8150"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Fixture rows parsed; malformed rows refused by name; a missing day gives MarginUnknown and never another day's value; M=25% gives 4x, 22.11% the exact ratio, 12.5% caps at 5x; the column-9 reading; batch 1; the 1 Jun 2020 method change; the dated symbol on a rename day; non-EQ series ignored; today's file.",
    "benches": "C-FNO-LEV-01 one lookup p99 <= 1 us, flat in the months held (ESTIMATE)"
  },
  {
    "name": "FNO-RANK",
    "goal": "The K16 strategy subset: the top-mover rank across the 218 every minute, known at decision time. m = (P-C)/C as an exact rational under both close bases; a fixed bitonic network over 256 padded slots (the same work every minute); competition ranks with ties; gainer and loser ranks; staleness; pool size. A per-stock-day rank series for 'entered', 'ordinal', 'minutes in top N' and 'best rank' at any N and W. Daily ranks for 1day.",
    "files": [
      "crates/runner/src/fno_cross_section/ (mod.rs, network.rs, tests.rs) (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_cross_section.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8160"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Ties at the top-N edge; a stale stock; no candle yet; the pool changing on a membership day; the 12 no-Zerodha stocks absent; a Kite-hole minute; a demerger ex-day excluded; 218 padded to 256 with an identical compare-exchange count; both bases; an all-stale minute; a pool of 1. Properties: the network order equals a reference sort; the rank at T is unchanged when later candles are altered.",
    "benches": "C-FNO-RANK-01 one minute with 218 live slots, p99 <= 15 us and p99/p50 <= 1.2 over 1x/10x/100x minutes (ESTIMATE)"
  },
  {
    "name": "FNO-PAUSE",
    "goal": "The K06 strategy subset: a push/box/break machine per (side, K) for the run's generated K set. Push measures; a box never including the push-extreme candle; the S33 holding rules against a level price passed in; break decided at the close; failed break; pause ordinal; staircase; partial U of 2 or 3 candles. Anchor events (push leg, measured move) with their known-at times for FNO-LADDER.",
    "files": [
      "crates/indicators/src/fno_pause/ (mod.rs, push.rs, boxm.rs, shape.rs, tests.rs) (new)",
      "crates/indicators/src/lib.rs (slot)",
      "crates/indicators/benches/ratio/fno_pause.rs (new)",
      "crates/indicators/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8170"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Every output true/false/unknown on hand-built bars; the push-extreme candle never in the box; a new high ends the box; an engulfing candle; K=1; each holding rule; a failed break inside its window; staircase intact and broken; the ordinal; partial U of 2 and 3; the short mirror; flat candles; an incomplete candle. Property: prefix (later bars removed changes nothing).",
    "benches": "C-FNO-PAUSE-01 one candle with 16 K x 2 sides, p99 <= 2 us, flat from 1k to 200k (ESTIMATE)"
  },
  {
    "name": "FNO-PORTFOLIO",
    "goal": "The money and order-rate rules with exact tie-breaks: 20 slots, a per-stock lock, 100 trades a day (entries counted at send), Rs 1 crore exposure, margin in use against carried capital (Rs 20 lakh plus net, worse clock), order-rate rings (10/s, 250/min, 1,000/h, 7,000/day), 25 modifications per order, exits before entries inside a second, and the tie-break order (a)-(e) with a seeded spread. (2 Nov 2026: PARTS.md, READINESS ADDENDUM at its end, binds this part: item 5: capital starts per window.)",
    "files": [
      "crates/runner/src/fno_portfolio.rs (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_portfolio.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8180"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "20th position admitted, 21st refused; 100th trade admitted, 101st refused, with cancelled entries counted; exposure exactly Rs 1 crore admitted, +1 paisa refused; margin exhausted by carried losses; one trade per stock across recipes; every tie-break order; the seeded spread reproducible; 20 simultaneous exits over 10/s; 25 modifications; slot taken at send and freed at the last fill or a zero-fill cancel; the daily reset. Property: after any random sequence, slots <= 20, exposure <= Rs 1 crore, trades <= 100, no double lock.",
    "benches": "C-FNO-PORT-01 admit or release with 20 slots full, p99 <= 300 ns flat (ESTIMATE)"
  },
  {
    "name": "FNO-WFT",
    "goal": "Walk-forward exactly per SPEC 13.4. Anchored and rolling schemes; generated learning and test lengths (doubling, midpoints where verdicts differ); forward windows tiling the span. MonthAgg prefix vectors (additive O(1), non-additive folds over at most 98 months). Settings regenerated from the learning window only (form G quantiles; excursion ladders fitted on learning data). The full Winner Gate, including leave-one-out, the five downside checks, generated NIFTY regimes and minimum returns on Rs 20 lakh. The pick order by share of profitable days. Recipe and procedure verdicts under every scheme; luck accounting via runner significance, pbo and bootstrap; 'nothing passed' with the closest misses. (2 Nov 2026: PARTS.md, READINESS ADDENDUM at its end, how FNO-RUN feeds this part's rows: items 5 and 11.)",
    "files": [
      "crates/runner/src/fno_wft/ (mod.rs, scheme.rs, agg.rs, gate.rs, generate.rs, tests.rs) (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_wft.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8190"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Windows tile the span (property over random spans and schemes); a span too short refused; learning outputs unchanged when forward months are altered (property); not-picked months count as not positive; every scheme; PF with zero losses; leave-one-out; pro-rated partial years; all trades cancelled; the nothing-passed report; the pick order.",
    "benches": "C-FNO-WFT-01 additive (candidate, window) total p99 <= 200 ns flat in total months; C-FNO-WFT-02 quantile ladder per window, a bounded batch (ESTIMATE)"
  },
  {
    "name": "FNO-DAILY",
    "goal": "The strategy-mvp FNO-1DAY on the new path: decide on D's daily candle and trade D+1 at the first traded second >= 09:15:00 + latency (or a swept later start). D+1 must be tradable. Daily skip rules. daily_invalid_mask() for time-of-day and VWAP bits by vocab name. The old path's 1day refusal is untouched.",
    "files": [
      "crates/runner/src/fno_daily.rs (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_daily.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8200"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Friday to Monday; a holiday gap; D+1 not tradable refused; the 2018-11-07 Muhurat daily bar never a signal; a provisional bar; the MAZDOCK, PATANJALI and RELIANCE 1day rules; the latency and later-start sweeps; the mask pinned by name. Property: a D signal reads nothing from D+1.",
    "benches": "C-FNO-DAILY-01 p99 <= 100 ns (ESTIMATE)"
  },
  {
    "name": "FNO-BAND",
    "goal": "The price band per stock-day, computed for every date: base +/-10%, flex observed in the data (5%, 3%, 2%). Two base readings, C_z(D-1) x the session ratio and GDFL's last trade of D-1; the headline takes the reading worse for net, as lanes. Labelled 'band computed'; the security-file check is FNO-BAND-FILES in F3.",
    "files": [
      "crates/runner/src/fno_band.rs (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_band.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8210"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Prints exactly at the band; a flex observed; a sell at the lower band and a buy at the upper band; resuming off the band; a split day inside one session; a demerger ex-day skipped; the first print of the day already at the band.",
    "benches": "C-FNO-BAND-01 one check p99 <= 100 ns (ESTIMATE)"
  },
  {
    "name": "FNO-CLOCK",
    "goal": "The per-stock-day GDFL clock calibration (D-0802 phase 2), so the headline uses only latencies at or above the calibrated floor. The offset comes from shifted minute folds of the seconds against Zerodha minutes (sparse-table range queries, ratio-mapped in session); the backward clock step comes from raw tick order if pull::gdfl_tickstore exposes it (UNVERIFIED), else the row is flagged. Output: a fixed-stride reference/fno-clock-v1 table with a digest; floor = max(0, offset) + back-step. (2 Nov 2026: PARTS.md, READINESS ADDENDUM at its end, binds this part: item 8: build takes the (stock, day) key list as its one input.)",
    "files": [
      "crates/cli/src/fno_clock.rs (new)",
      "crates/cli/src/lib.rs (slot)",
      "crates/cli/benches/ratio/fno_clock.rs (new)",
      "crates/cli/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8220"
    ],
    "depends_on": [
      "FNO-SEAM"
    ],
    "tests": "Synthetic feeds shifted by +/-1..30 s recovered exactly; a no-agreement day flagged uncalibrated; holes, the auction era and 2019-05-30; ties take the larger |offset|.",
    "benches": "C-FNO-CLOCK-01 per minute boundary flat; per stock-day cost linear in its seconds, a bounded batch recorded in docs/06-limits.md (ESTIMATE)"
  },
  {
    "name": "FNO-NET",
    "goal": "Quantity, leverage and charges per trade: the integer closest to Rs 1 lakh x min(leverage readings) / price, an exact tie taking the lower quantity, q=0 refused. Charges through costs::equity_clock::price_trade on both clocks with the worse headline. Net and 'signal only, before charges' (the same worst fills). The monthly Dhan data API line for the account replay.",
    "files": [
      "crates/runner/src/fno_net.rs (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_net.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8230"
    ],
    "depends_on": [
      "FNO-COSTS"
    ],
    "tests": "5,00,000/1,600 gives 312; q=0; M=25% gives Rs 4 lakh; the minimum of three readings; CN01..CN24 through trade() equal COSTS's headline; the square-off order count; signal-only equals gross with worst fills; a win only when net > 0.",
    "benches": "C-FNO-NET-01 charges per fill p99 <= 2 us flat (ESTIMATE)"
  },
  {
    "name": "FNO-EXITS",
    "goal": "The exit dimension on seconds (KIT X1-X3 subset): trailing stop (including the stair-step to each new pause low), trailing take-profit (grid::Ttp semantics), break-even move, time and condition exits. Every move is an order action with latency and counts toward the 25-modification cap. The S64 family enumeration always includes a stop; stop-less only as a labelled comparison. Families walked as fixed lanes in one pass.",
    "files": [
      "crates/runner/src/fno_exits.rs (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_exits.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8240"
    ],
    "depends_on": [
      "FNO-SEC-EXEC"
    ],
    "tests": "Each exit alone and in families; stop and target in one second (stop first); a trailing step with latency; the 26th modification triggers cancel and replace; a time exit at 15:07:59 and 15:08:00; a condition exit with its protective stop; break-even never; stop-less refused outside the labelled comparison. Property: no exit acts before its trigger.",
    "benches": "C-FNO-EXITS-01 per second per lane p99 <= 300 ns (ESTIMATE)"
  },
  {
    "name": "FNO-SEC-WIRE",
    "goal": "The GDFL seconds source over the store: SecondSource on BarFile::open_existing, honouring repaired-month pointers, one cached handle per stock-month, .tix first_at_or_after and read_record, never load_span on 1s; a locked month refused by name. The F&O run identity: a new versioned domain with a per-month GDFL binding (never hashing every second) and two levels (2 Nov 2026, PARTS.md, READINESS ADDENDUM item 2, over :845): FnoRunV1 for the whole run, with no recipe and no scheme (signal and fill feeds, lanes, mapping, cost-table version, leverage, universe, join, skip, t2t, membership, clock, band rule, pool digest, vocab version, commit, span first and last, sample, data digest over both feeds' month bindings, quote digest, calendar reading, proof scope, proof sha256) with fno_run_digest, and FnoCandidateV1 { run, wft_scheme, recipe_bytes } with fno_identity.",
    "files": [
      "crates/cli/src/fno_seconds.rs (new)",
      "crates/cli/src/lib.rs (slot)",
      "crates/runner/src/identity.rs (new domain functions; sole owner)",
      "crates/cli/benches/ratio/fno_seconds.rs (new)",
      "crates/cli/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8250"
    ],
    "depends_on": [
      "FNO-SEC-EXEC"
    ],
    "tests": "A temp store written by the store's writer; a repaired month served from its revision; a locked month refused; an empty month; None after the last traded second; the sizing price; the binding changes with each of its fields and not with another month; domain separation from data_digest_with_execution; existing Run ids unchanged; each run term moves the run digest; the candidate id moves with recipe and scheme alone; a sample and a full run never share a digest; the Zerodha binding changes with each of its fields.",
    "benches": "C-FNO-SECWIRE-01 warm second fetch p99 <= 3 us; cold first touch reported as device latency (ENGINE measured 118-124 us) (ESTIMATE)"
  },
  {
    "name": "FNO-STRAT",
    "goal": "The focused, parameterized strategy module on the runner side. Typed FeatureRow per candle from rank, ladders, pause, candle measures, causal slot norms and the measured price step, with the vocab mask kept for filters. Pass A: per-month histograms. Pass B: a run-local generated clause alphabet with posting lists. A level-wise search with one clause per dial and extinction at the requirement's >= 500 trades/yr pro-rated floor, so every combination is covered without a vocabulary change. Canonical recipe bytes; signals with stop and target under R1/R2 (R0 refused until R4-26); runtime FIB-I2 assertions; an incremental cache. (2 Nov 2026: PARTS.md, READINESS ADDENDUM at its end, binds this part: item 6: the cache root is an argument.)",
    "files": [
      "crates/runner/src/fno_strategy/ (mod.rs, features.rs, clauses.rs, recipe.rs, search.rs, distrib.rs, norms.rs, cache.rs, tests.rs) (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_strategy.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8260"
    ],
    "depends_on": [
      "FNO-LADDER",
      "FNO-PAUSE",
      "FNO-RANK",
      "FNO-DAILY",
      "FNO-WFT"
    ],
    "tests": "The operator's flag (push, box above 0.382, break) gives exactly one signal at the break candle's close, plus its short mirror; P0..P8; a new-extreme candle inside a box; V-SEC and V-BOTH at composition; nothing static (two distributions give two alphabets, no literal threshold); extinction exact against brute force on small alphabets (property); deterministic distributions; the incremental cache keeps earlier digests; no decision on an incomplete candle; R0 refused.",
    "benches": "C-FNO-STRAT-01 one candle over 2,048 clauses p99 <= 5 us flat in history; C-FNO-STRAT-02 posting-list intersection p99 <= 20 ns per element (ESTIMATE)"
  },
  {
    "name": "FNO-RECORDS",
    "goal": "Save every decision in the store's own records: a fixed-stride, blake3-sealed results/fno-v1 family (run header with every identity term and the label; mask verdicts; signals; orders; fills with second, quantity and price; trades; refusals and cap hits; MonthAgg; window and scheme verdicts; an append-only audit journal; search indexes). A lean policy (aggregates for every candidate; trades and fills only for picked recipes or on request). The live runtime/store is refused by default. (2 Nov 2026: PARTS.md, READINESS ADDENDUM at its end, binds this part: items 2, 7 and 10.)",
    "files": [
      "crates/cli/src/fno_records/ (mod.rs, rows.rs, index.rs, journal.rs, lean.rs, tests.rs) (new)",
      "crates/cli/src/lib.rs (slot)",
      "crates/cli/benches/ratio/fno_records.rs (new)",
      "crates/cli/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8270"
    ],
    "depends_on": [
      "FNO-SEC-EXEC",
      "FNO-NET",
      "FNO-WFT",
      "FNO-DAYMASK"
    ],
    "tests": "Every row kind round-trips; a corrupted seal is detected; a torn write recovers; a same-identity rerun writes nothing; the label is in every run header; the live store is refused; the lean policy holds.",
    "benches": "C-FNO-REC-01 append p99 <= 5 us (fsync batched per stock-month), read by index p99 <= 2 us (ESTIMATE)"
  },
  {
    "name": "FNO-FILTER",
    "goal": "The existing 370 conditions as extra filters, with no vocabulary change: the existing engine Apriori over the signal candles' masks of each surviving recipe, at the same extinction floor. Price-action rows only in the first run (indicator families excluded and labelled). daily_invalid_mask on 1day.",
    "files": [
      "crates/runner/src/fno_filter.rs (new)",
      "crates/runner/src/lib.rs (slot)",
      "crates/runner/benches/ratio/fno_filter.rs (new)",
      "crates/runner/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8280"
    ],
    "depends_on": [
      "FNO-STRAT",
      "FNO-DAILY"
    ],
    "tests": "A filter never adds a signal (subset property); support equals engine::support; the admitted families; the daily mask; the mask enters the recipe bytes and the identity.",
    "benches": "C-FNO-FILTER-01 per-signal mask gather p99 <= 50 ns (ESTIMATE); the engine's own benches stand"
  },
  {
    "name": "FNO-API",
    "goal": "Read-only JSON routes and one page for searching, monitoring and the dashboard: runs, run, windows, trades, fills, mask, refusals, events. Every response and the page carry the survivorship label and the data-limits line; 'nothing passed' is shown plainly; net beside signal-only and both cost clocks; fill-spread statistics; refusals; mask counts with the PIT count outside the 218. (2 Nov 2026: PARTS.md, READINESS ADDENDUM at its end, binds this part: items 2, 6 and 7.)",
    "files": [
      "crates/api/src/fnojson.rs (new)",
      "crates/api/src/lib.rs",
      "crates/api/src/server.rs (route lines)",
      "web/src/routes/fno/+page.svelte (new)",
      "web/src/lib/fno.js (new)",
      "web/src/lib/speed.js (page budget row)",
      "docs ledgers at slot D-8290"
    ],
    "depends_on": [
      "FNO-RECORDS"
    ],
    "tests": "Every route: label present, paging bounds, unknown run 404 with a reason, malformed query refused. Node tests for fno.js: the label always rendered and the nothing-passed render. The browser harness for the new page.",
    "benches": "C-FNO-API-01 one page of trades with an O(1) seek (ESTIMATE)"
  },
  {
    "name": "FNO-RUN",
    "goal": "The one driver of the first run, as a cli fno verb family (run, margin build, clock build, mask, report). Phases P0-P8: preflight (data proven by --proof FILE, whose scope and sha256 join the run digest; shadow only; execution lease and invocation audit by hand; input digests; every read month has its .qrg); day mask; load and fold; cross-section; features and distributions; per-window generation, search and filters; the seconds walk with exits, band, clock floor, portfolio and net; verdicts; records. Parallel by stock, day, partition and candidate; one process on one shadow is allowed, and a shard split is not owed (READINESS ADDENDUM item 4). Resumable and idempotent. A --sample measure-then-project step before the full run. Adapters between parts. NIFTY monthly regimes. Telemetry only at boundaries. (2 Nov 2026: PARTS.md, READINESS ADDENDUM at its end, binds this part: items 1-11, 14 and 15.)",
    "files": [
      "crates/cli/src/fno_run/ (mod.rs, phases.rs, inputs.rs, nifty_regime.rs, tests.rs) (new)",
      "crates/cli/src/lib.rs (slot + fno dispatch arms; sole owner)",
      "crates/cli/src/store_write.rs (the word fno: WRITERS, one writes_the_store arm in gate 30's shape, test rows)",
      "crates/cli/src/sweep_wiring_tests.rs (the word fno: non-sweep)",
      "crates/cli/benches/ratio/fno_run.rs (new)",
      "crates/cli/benches/ratio.rs (slots)",
      "docs ledgers at slot D-8300"
    ],
    "depends_on": [
      "FNO-STRAT",
      "FNO-RECORDS",
      "FNO-DAYMASK",
      "FNO-LEVERAGE",
      "FNO-CLOCK",
      "FNO-SEC-WIRE",
      "FNO-SEC-EXEC",
      "FNO-EXITS",
      "FNO-BAND",
      "FNO-PORTFOLIO",
      "FNO-NET",
      "FNO-WFT",
      "FNO-RANK",
      "FNO-LADDER",
      "FNO-PAUSE",
      "FNO-DAILY",
      "FNO-COSTS"
    ],
    "tests": "End to end on a temp store with synthetic Zerodha and GDFL months for 3-5 stocks: the flag example gives one trade with known fills, charges and record rows. Every refusal (missing input, locked month, live store, data not proven, span too short). Kill after any phase, then resume: byte-identical records, the journal left out (READINESS ADDENDUM item 10). An idempotent rerun. One integration test per THE BAR extreme case (PARTS.md section 10). The label on every output. The tests named in READINESS ADDENDUM items 1-3, 5-8, 10, 11 and 15, and in the P0 decision at PARTS.md :1424. Gate 30 and gate 11 on the post-data tree, rc 0.",
    "benches": "C-FNO-RUN-01 end-to-end cost per stock-minute flat over 1x/10x/100x synthetic months (ESTIMATE)"
  },
  {
    "name": "FNO-LEAN",
    "goal": "F3 follow-on, the strategy-mvp FNO-LEAN on the old range-all path: a lean run-mode knob entering Params.policy so Capture::begin stops writing millions of evidence files. Not needed for the first run, where the new path is lean by FNO-RECORDS.",
    "files": [
      "crates/cli/src/lib.rs (22819-22833 hook)",
      "crates/cli/src/candidate_trades.rs",
      "crates/cli/src/sweep_evidence.rs",
      "crates/cli/src/knobs.rs",
      "docs ledgers at slot D-8310"
    ],
    "depends_on": [
      "FNO-RUN",
      "FNO-API",
      "FNO-STRAT",
      "FNO-FILTER",
      "FNO-RECORDS"
    ],
    "tests": "The knob enters the identity; lean writes aggregates only; full mode unchanged; the evidence count bounded.",
    "benches": "the existing cli rows stay flat"
  },
  {
    "name": "FNO-SWEEP-WIRE",
    "goal": "F3 follow-on: point the existing range-all sweep's F&O pricing, day mask and 1day rule at the same code as the first run (fno_seconds, fno_daymask, fno_exec, fno_daily), so there is one fill implementation everywhere. Results v4 and partial-fill trade rows; API and backtest page fields.",
    "files": [
      "crates/cli/src/lib.rs (hooks 8256-8264, 8318, 8506, 10271, 10350, 14233, 14412, 22630, 20662, 23591, 2385, 10213, 18108, tests 28043-28071)",
      "crates/cli/src/results.rs",
      "crates/cli/src/trades.rs",
      "crates/api/src/backtest.rs",
      "crates/api/src/sweeprun.rs",
      "web/src/routes/backtest/+page.svelte",
      "web/src/lib/sweep.js",
      "web/src/lib/charge-scope.js",
      "docs ledgers at slot D-8320"
    ],
    "depends_on": [
      "FNO-RUN",
      "FNO-API",
      "FNO-STRAT",
      "FNO-FILTER",
      "FNO-RECORDS"
    ],
    "tests": "The old index path unchanged; F&O-path prices equal the new path's on the same trade; v3 rows still read; the 1day F&O path decides on D and trades on D+1 while the old refusal stands elsewhere.",
    "benches": "the existing cli rows stay flat; C-FNO-SEC-EXEC rows reused"
  },
  {
    "name": "FNO-BAND-FILES",
    "goal": "F3 follow-on: read NSE's daily security file (PricRg band and BidIntrvl tick, from 2024-02-05) to cross-check FNO-BAND's computed band and the measured price step, counted per stock-day. The download needs the operator's yes.",
    "files": [
      "crates/pull/src/nse_security.rs (new)",
      "crates/pull/src/lib.rs",
      "docs ledgers at slot D-8330"
    ],
    "depends_on": [
      "FNO-RUN",
      "FNO-API",
      "FNO-STRAT",
      "FNO-FILTER",
      "FNO-RECORDS"
    ],
    "tests": "Parsing fixtures; malformed rows refused; agreement or difference with the computed band counted per stock-day; dates before 2024-02-05 refused by name.",
    "benches": "one lookup O(1), flat (ESTIMATE)"
  }
]

```

# PARTS: every part the F&O top-mover Fibonacci backtest needs, the waves, the merge families and the schedule to Thu 12 Nov

Written 30 Oct 2026, about 23:xx IST, by the design step of workflow-fib-1b.js (run wf_cf9d340f-28c), side session 2da9f915.
Status: design only. Nothing was built, nothing was committed, no branch or worktree was made, nothing under `runtime/` was touched, no server was started.
Code references are at `feat/db-fix` `f01d7a0e4` (the DBFIX worktree HEAD, `git log --oneline -3`, run 30 Oct).

Every result and page this design produces carries this label, word for word:
**"current F&O members only (survivorship bias not removed)"**

Short names:
- SPEC = `state/fib-strategy-20261030/SPEC.md`; COSTS = `COSTS.md` and `costs-table.json` beside it; ENGINE = `ENGINE.md` beside it.
- MVP = `state/strategy-mvp-20261030/REPORT.md`; KIT = `state/fno-winners-20261013/condition-kit-design.json`.
- Code paths are relative to the worktree `wt/DBFIX`. ESTIMATE marks every number I did not measure.

---

## 0. The answer in brief

1. **Twenty-five parts**, in three merge families:
   - **F1 "Foundations"** (17 parts, built in waves 0, 1 and 2): the slot seam, the law, the cost table, the seconds fill walk, the ladders, the day mask, the margin files, the cross-section rank, the pause machine, money and charges, the portfolio rules, exits, bands, the clock calibration, walk-forward, the 1-day rule and the seconds source.
   - **F2 "Strategy and run"** (5 parts, wave 3): the strategy module, the 370-condition filters, the store records, the run driver and the read-only API with its page.
   - **F3 "Old path and follow-ons"** (3 parts): the lean mode and the seconds wiring of the existing `range-all` sweep, and the NSE security-file band check. The first run does not need F3. F3 lands while the first run is going.
2. **The first run needs F1 and F2 only.** It is one new, focused, parameterized strategy path. The strategy blocks are typed state machines, not vocabulary bits, so **no vocabulary widening**. The existing 370 conditions enter as extra filters through the existing Apriori `engine`. The 13 Oct condition kit (KIT-00..KIT-99) follows after; its K06/K07/K16 packages become thin adapters over the same state machines, so there is never a second copy (section 6).
3. **Wave 1 (build now): FNO-LAW, FNO-COSTS, FNO-SEC-EXEC, FNO-LADDER, FNO-DAYMASK, FNO-LEVERAGE.** Each lives in a different crate (or, for LAW, in documents only). None shares a code, test, bench or manifest file. They depend only on **FNO-SEAM**, a comments-only commit (wave 0, about 15-30 minutes, content fixed in section 4.2), which places one slot line per part in every shared registry file, including the four ledger documents. I tested in a scratch git repository that insertions under different lines merge cleanly and that two end-of-file appends conflict (section 4.1).
4. **The schedule (section 8).**
   - **Thu 5 - Fri 6 Nov does NOT fit** for the full minimal route. That date was estimated for the 8 strategy-mvp parts only; the operator's strategy blocks add 14 parts and one more serial wave.
   - **Likely: the backtest starts Sat 7 - Mon 9 Nov. Worst: Thu 12 Nov, with no slack left.** It fits Thu 12 only if every part passes within three check rounds, BASE-REDS-1023 lands by Tue 3, the data is proven by Sun 1, and each account-day supports about 25-35 agent-hours of builds (ESTIMATE).
   - One more failed round on the serial chain (STRAT, then RUN) moves the start past Thu 12. Section 8.3 gives the levers.
5. **Decisions needed** (section 9): register the D-number blocks; allow dependent parts to be cut from the seam or family branch rather than straight from `feat/db-fix`; the operator's yes to download NSE's public daily margin files (C_VAR1, about 2,010 small files); a source for trade-for-trade days; the client's state for stamp duty before 1 Jul 2020; and the order of COMMON-1 against F1.

---

## 1. What I read

- The law: `wt/DBFIX/CLAUDE.md` (484 lines), in full.
- The three reports of this round: SPEC (1,129 lines, every line), COSTS (sections 1-9 and the vector list), ENGINE (491 lines, every line).
- MVP REPORT.md (170 lines), and this folder's PROGRESS.md and RESUME.md.
- The operator's definitions in memory: `scope-fno-stocks-first.md` and `let-data-decide.md` in full; `backtest-acceptance-bar.md` lines 30-38. The other strategy memories were read through the line-by-line quotes in SPEC section 1.
- KIT: blocks K06, K07 and K16 in full; the package list with its dependencies and sizes; the mask plan.
- `sources-leverage-execution.md` section 1 (lines 1-70).
- The fno-data-audit report head and `backtest-skip.tsv` (80 lines including the header; free English in its "who" column, so not machine-readable as it stands); `fno218.txt` (218 lines).
- D3 recipe: `state/decisions-20261027/d3-recipe.txt`.
- The decision-block registers: `state/decisions-20261029/blocks.json` (free_from D-7720) and `state/decision-blocks.json` (free_from D-7730; last block D-7720..D-7729, owner COMMON-1).
- Code read at f01d7a0e4:
  - module lines in `crates/{runner,indicators,cli,api,costs,core,pull}/src/lib.rs`;
  - bench layouts (`crates/store/benches/ratio/`, `crates/pull/benches/ratio/` use a folder; runner, indicators, costs and cli use one `ratio.rs`; `fn main` of `crates/runner/benches/ratio.rs`);
  - `costs/src/schedule.rs` lines 585-608 (`RateSelector`) and 990-1035 (`dated_card` refuses equity), `RateCard` at 777;
  - `costs/src/ledger.rs` and `stack.rs` public items; `costs/src/fill.rs` 318-440 (`Anchor`, `fills_at`);
  - `store/src/file.rs` 2105, 3504, 3553; `store/src/format.rs` 234;
  - `pull/src/fno_membership.rs` 2126-2585; `pull/src/calendar.rs` 654, 3405; `pull/src/fold.rs` 163-1117;
  - `indicators/src/lib.rs` 235, 400, 484, 1637; `engine/src/lib.rs` public items;
  - `runner/src/{significance,pbo,bootstrap,split}.rs` public functions;
  - `crates/core/tests/decision_citations.rs` lines 1-68 (a D-number cited under `crates/` must head an entry of `docs/05-decisions.md`);
  - `crates/indicators/tests/module_doc_counts.rs` lines 1-40;
  - `.github/gates_bounds.rs` (gate 14 reads bench rows from `docs/04-invariants.md`, line 29);
  - `docs/10-shared-core.md` lines 1-40; `api/src/server.rs` 16950-16975.
- No property-test crate is in any manifest (`grep proptest|quickcheck crates/*/Cargo.toml`: none). So property tests are written by hand with seeded xorshift generators, and no dependency is added.

---

## 2. Facts the design rests on

From the reports, re-checked where marked:

- Today no backtest code reads a GDFL second, F&O membership, a charge or a quantity (ENGINE 0.1).
- `grid::level_fill` fills a target at its level or the bar's open (`runner/src/grid.rs` 5466-5487). That breaks the one fill model, so the minute grid is not reused for fills (SPEC 16).
- Reaching one GDFL second is two reads, measured flat from 437 to 327,545 seconds per month. Warm p50 is 1.2 µs. The cold p99 of 118-124 µs is the disk's first touch (ENGINE 3.3).
- `costs::schedule::dated_card` refuses equity on every date (`schedule.rs` 1013-1026, read). `RateSelector` is used only inside `crates/costs` (`grep -rln RateSelector crates`: only costs files). So making the dated clock exist for cash equity changes no other crate.
- The runner may not depend on `store` or `pull` (ENGINE 2.3). Every reader of seconds, membership, calendar and margin files is in `cli` or `pull`; every state machine is pure, in `runner` or `indicators`.
- Gate 17: `vocab`, `engine`, `indicators` and `runner` emit no telemetry. Only `cli` logs, at structural boundaries.
- Every D-number cited under `crates/` must head an entry of `docs/05-decisions.md` (`crates/core/tests/decision_citations.rs` 1-3). Bench rows must be rows of `docs/04-invariants.md` (gate 14, `.github/gates_bounds.rs` 29). So every part writes into the four ledger documents. That is why FNO-SEAM gives each part its own slot line in them.
- **ICICIPRULI to ICICILIFE.** The join ships in Saturday's build (D-7311). That build may re-key the store, so the join map must work in both store states (FNO-DAYMASK).
- **COMMON-1** (`state/decision-blocks.json`, block D-7720..D-7729) makes `vocab`, `indicators`, candle building and `costs` common for tickvault, as "the FIRST part after ingest + /db". If it lands before F1, FNO-COSTS, FNO-LADDER and FNO-PAUSE meet a refactor of the same crates. Decision 9.7.
- The skip table `backtest-skip.tsv` describes stocks in English ("171 stocks (all Zerodha keys listed by then except FEDERALBNK, HINDZINC, RELIANCE)"). A machine-readable normalised copy is a data step of FNO-DAYMASK (section 5, FNO-DAYMASK).

---

## 3. The first run: one common path, no vocabulary widening

### 3.1 The data flow

The run is one cli verb family, `cli fno ...` (owned by FNO-RUN). Every step reuses an existing authority where one exists:
- store reads: `BarFile::open_existing`, `first_at_or_after`, `read_record`;
- folding: `pull::fold`;
- calendar: `pull::calendar`;
- membership: `pull::fno_membership::Membership`;
- vocabulary masks: `indicators::column::Column::build`;
- the Apriori: `engine`;
- statistics: `runner::{significance, pbo, bootstrap}`;
- charges: `costs::{stack, ledger}` plus the new dated table;
- the fill price law: `costs::fill::Anchor::PrintedExtreme`.

```
inputs (data files, digests in the run identity):
  fno218.txt  join map  normalised skip table  T2T table  membership dir  C_VAR1 table  clock table  costs-table version
        |
P0 preflight ---- data proven? shadow store (never runtime/)? execution lease? every input present?
        |
P1 day mask (FNO-DAYMASK) --------------------------- per (stock, day): Tradable(DayPlan) or Refused(reason)
        |
P2 load + fold: Zerodha 1-min per stock-day -> 2..60-min rungs (pull::fold); 1day bars
        |
P3 cross-section, per minute (FNO-RANK) ------------- rank series per stock-day into the signal cache
        |
P4 per stock, per timeframe, one candle at a time (CLAUDE.md s3 r7 fold shape):
     FNO-LADDER judgements -> FNO-PAUSE machines -> vocab ConditionMask (Column::build)
     -> FNO-STRAT feature row -> (pass A) distributions per month
                              -> (pass B) clause bitsets and posting lists per month
        |
P5 per walk-forward scheme x learning window (FNO-WFT):
     generate settings from that window only -> level-wise search with extinction (FNO-STRAT)
     -> 370-condition filters (FNO-FILTER, engine Apriori on the signal candles)
        |
P6 walk on GDFL seconds, per candidate:
     FNO-SEC-WIRE source -> FNO-SEC-EXEC fill walk + FNO-EXITS lanes + FNO-BAND + FNO-CLOCK floor
     -> FNO-PORTFOLIO money rules -> FNO-NET quantity and charges (FNO-COSTS, two clocks, worse)
     -> per-month aggregates
        |
P7 verdicts: recipes and procedures under every scheme; luck accounting; "nothing passed" plainly
        |
P8 records (FNO-RECORDS, store's own results/fno-v1/), telemetry at boundaries, API + page (FNO-API)
```

### 3.2 How "every setting in every combination" stays honest and finite

The settings table (SPEC 14) has about 60 dials. Their full product is astronomically large, so a literal enumeration is not possible. This design reaches every combination by the law the engine already lives by: extinction, not a typed depth (CLAUDE.md section 6).

- **Entry clauses.** Every entry setting is a clause `(dial, value)` over the typed feature row. Thresholds are monotone where the dial is a threshold (rank <= N, |m| >= q, height <= h, and so on).
  - A recipe is a conjunction with at most one clause per dial ("never mix two settings of one dial", MEM/let-data-decide).
  - Direction and timeframe partition the search.
- **The extinction floor is a requirement number, not a static choice.** The Winner Gate demands at least 500 trades a year, pro-rated (SPEC 1.5). Signals are at least trades, so a clause set whose signals in a learning window fall below that floor cannot pass. Every superset has fewer signals.
  - So a combination that is never enumerated provably cannot pass. That is the same anti-monotone argument as CLAUDE.md section 6. This number is one of the operator's requirement thresholds, which FIB-I6 allows to be fixed.
- **Generated values.** Clause values come from the learning window (forms D and G of SPEC 14), refined where survivors crowd. No literal threshold sits on the path. The FNO-STRAT test "two different distributions give two different alphabets" proves it.
- **Exits are their own dimension** (MEM/let-data-decide). Only entry sets that survive the floor are walked. All exit families and fill settings of the run are walked together in one pass over each trade's seconds, as fixed lanes.
- **Run order.** Price action first. The indicator rows of the vocabulary are not admitted as filters in the first run (FNO-FILTER), as MEM/let-data-decide's run order says.
- **Honest limit.** The machine time of the full span is unknown until measured. FNO-RUN measures a sample first (`--sample`) and prints a projection of the full run's time and disk before the full run starts. Section 10.

### 3.3 Why a new path and not the `range-all` sweep

The `range-all` sweep prices on minutes (`grid::evaluate_over`, ENGINE 2.2 step 10), writes about 96 GB of evidence per stock-year (MVP line 91) and holds Apriori over one instrument at a time. The strategy needs a cross-section every minute and seconds fills. Wiring it into five regions of `cli/src/lib.rs` is the slow, conflict-heavy route (ENGINE 7).

The new path is **the** F&O path, not a second copy:
- the fill walk (FNO-SEC-EXEC) is the only seconds fill code;
- F3's FNO-SWEEP-WIRE later points the old sweep's F&O pricing at the same walker;
- the kit packages later wrap the same state machines.

### 3.4 Records, audit and "nothing static"

- **Pure parts return typed decision records and do no I/O.** For example: a `MaskVerdict` with its reason, a `Fill` with second, quantity and price, a `Refusal`, a `NetTrade` with every charge line under both clocks, a window `Verdict` with each failing clause.
- **FNO-RECORDS writes every record** into the store's own append-only, fixed-stride family `results/fno-v1/`, always on a shadow. FNO-RUN logs at boundaries. FNO-API serves the records read-only.
- **Incremental.** The signal cache (rank series, distributions, posting lists) is keyed by stock-month, data digest and feature-code version. Adding a month, a dial value or an exit family re-walks only what is new. Records are append-only. A rerun with the same identity writes nothing new.

---

## 4. File ownership, slots, branches and ledgers

### 4.1 The rule, and the test behind it

- In a scratch repo (`scratchpad/gitmerge-test`), branches that inserted a line after line b, after line c and after line d of one file merged with **no conflict**, even with one line between them. Two branches appending at the end of a file **conflicted**.
- So two parts may edit one registry file only by inserting under **different existing lines**.
- **FNO-SEAM** (wave 0) puts one slot line per part in every registry file more than one part edits:
  - `crates/runner/src/lib.rs`, `crates/runner/benches/ratio.rs`;
  - `crates/indicators/src/lib.rs`, `crates/indicators/benches/ratio.rs`;
  - `crates/cli/src/lib.rs`, `crates/cli/benches/ratio.rs`;
  - `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md`.
- Each part inserts its lines directly under its own slot line and edits no other line of those files.
- Every other file in this design has exactly one owning part.

### 4.2 FNO-SEAM's exact content (comments and blank lines only)

**`crates/runner/src/lib.rs`**, after line 80 (`pub mod validate;`).

The slots are separated by blank lines, so `rustfmt`'s module reordering never crosses a slot. They are in the alphabetical order of the module names, so a reorder inside a group could not move anything either.

```
// ---- F&O strategy path (state/fib-strategy-20261030/PARTS.md 4.2). One slot per
// ---- part; each part inserts its single `pub mod` line directly under its slot.
// slot FNO-BAND (fno_band)

// slot FNO-RANK (fno_cross_section)

// slot FNO-DAILY (fno_daily)

// slot FNO-SEC-EXEC (fno_exec)

// slot FNO-EXITS (fno_exits)

// slot FNO-FILTER (fno_filter)

// slot FNO-NET (fno_net)

// slot FNO-PORTFOLIO (fno_portfolio)

// slot FNO-STRAT (fno_strategy)

// slot FNO-WFT (fno_wft)
```

**`crates/runner/benches/ratio.rs`:**
- At the top level, one line per runner part: `// slot bench-mod FNO-<X>`. The part puts `#[path = "ratio/fno_<x>.rs"] mod fno_<x>;` under it.
- Inside the `rows` array of `fn main`, after `the_equity_column_build_stays_within_its_budget(),`, one line per part: `// slot bench-row FNO-<X>`. The part puts its row call under it.

**`crates/indicators/src/lib.rs`**, after `pub mod vwap;` (line 73): slots `FNO-LADDER (fno_ladder)` and `FNO-PAUSE (fno_pause)`, in the same blank-separated form. The same pair goes into `crates/indicators/benches/ratio.rs` (bench-mod and bench-row).

**`crates/cli/src/lib.rs`**, after `pub mod vix_reference;` (line 400): slots `FNO-CLOCK (fno_clock)`, `FNO-DAYMASK (fno_daymask)`, `FNO-RECORDS (fno_records)`, `FNO-RUN (fno_run)` and `FNO-SEC-WIRE (fno_seconds)`. The same five go into `crates/cli/benches/ratio.rs`.

**`docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md`:**
- At the end of each, one section `## F&O strategy path: part blocks (D-8100..D-8339)`.
- Under it, one line per part, each followed by a blank line: `<!-- slot FNO-<X> -->`, in the part order of section 7.
- Each part inserts its rows directly under its own slot line: decisions in 05, invariants and bench rows in 04, bounds and unmeasured items in 06, its plan row in 07.

Nothing else. Checking FNO-SEAM takes `cargo fmt --check`, clippy `-D warnings` and `cargo test --workspace --locked`, all unchanged. Its coverage set holds no instrumented region (comment lines only).

**Not slotted, because one part owns each:**
- `crates/costs/*` (FNO-COSTS);
- `crates/pull/src/lib.rs` and `crates/pull/benches/ratio.rs` (FNO-LEVERAGE in F1; FNO-BAND-FILES in F3 inserts after LEVERAGE's line, on a later family, so the two are sequential);
- `crates/api/*` and `web/*` (FNO-API in F2; FNO-SWEEP-WIRE in F3 edits other `api` files, sequential);
- `crates/runner/src/identity.rs` (FNO-SEC-WIRE);
- the cli dispatch arms in `crates/cli/src/lib.rs` (FNO-RUN; F3's old-path hooks sit at other lines, ENGINE 6);
- `CLAUDE.md`, `AGENTS.md`, `docs/00-charter.md` (FNO-LAW).

### 4.3 Where each branch is cut (decision 9.2)

The task's rule cuts every branch from `feat/db-fix`. That cannot work for a part that calls another part's unmerged code: nothing merges before BASE-REDS-1023 (D1). This design therefore cuts:

| Parts | Cut from |
|---|---|
| FNO-SEAM | `feat/db-fix` (`git -C wt/DBFIX worktree add wt/FIB-SEAM -b fix/fib-seam feat/db-fix`) |
| Wave 1 and every wave-2 part that needs only slots (RANK, PAUSE, PORTFOLIO, WFT, DAILY, BAND, CLOCK) | `fix/fib-seam` |
| NET | FNO-COSTS's passed tip |
| EXITS, SEC-WIRE | FNO-SEC-EXEC's passed tip |
| Wave 3 (F2) | `fam/fib-1`, the F1 family branch, once assembled. FILTER, RUN and API are cut from the F2 family branch after the parts they call are merged into it |
| F3 | `fam/fib-2` |

- The family branches `fam/fib-<k>` are the main session's merge vehicles: one merge each into `feat/db-fix`, after BASE-REDS-1023.
- They are not an integration branch for running the backtest. The operator's "wait for merges" decision forbids that, and this design keeps it: the full backtest runs only from merged `feat/db-fix`.
- Each family is cut from the one before it, so **each family merges cleanly onto the one before by construction**. Inside a family, parts touch disjoint files or different slot lines.
- Builders commit only on their own `fix/fib-<part>` branch, never merge and never push. The commit trailer is `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Target dirs:
  - the task names `tgt/fib-<part>`;
  - the 23:0x disk-floor edit in PROGRESS.md moved the build prompts to `tgt/fib-shared` and `tgt/fib-shared-deep`, because WD_BLACK is below the operator's 2 TB floor;
  - whichever the build step uses, it goes through `bin/cargo`.

### 4.4 Decision numbers and ids

- **D-number blocks.** D-8100..D-8339 is reserved here: 24 blocks of ten, one per part, in section 7 order (FNO-SEAM has none; FNO-LAW D-8100..D-8109, FNO-COSTS D-8110..D-8119, and so on, to FNO-BAND-FILES D-8330..D-8339). There is no spare block; the next free block after the range is the first spare.
- **Why that range.** A read-only grep of `state/` on 30 Oct found D-7990..D-7999, D-8004, D-8557 and D-8931 in use, and nothing in D-8100..D-8339. The registers list free_from D-7730.
- **Registration.** The main session must enter the range in `state/decision-blocks.json` (I write only in this folder; decision 9.1). If any number is taken by then, the whole range shifts to the next free block boundary.
- **Other ids.** Invariant ids are `FNO-<CODE>-Inn` and bench ids are `C-FNO-<CODE>-nn`, where `<CODE>` is the part's short name.

---

## 5. The parts

**Format.** Each part lists:
- its goal and whether the first run needs it;
- its files: N = new, E = edit at its own slot or as the sole owner;
- its functions and data structures;
- tests: the extreme cases of THE BAR it owns, plus property tests with seeded xorshift and many seeds, no new dependency;
- benches with their p99 targets (ESTIMATE until measured);
- coverage and dependencies;
- its size: agent-hours to write once (ESTIMATE), and S/M/L.

**Common to every part:**
- **Bar.** 100% line and branch coverage of its coverage set by D3 Route B (`d3-route-b.sh`). Comment-only slot lines have no regions. Every refusal is driven by a test; the (F)/(U) list follows the recipe's step 4. Cargo-mutants is deferred by the operator. `cargo fmt --check`, clippy `-D warnings` and the touched crates' tests must be green.
- **Gates.** A part's check also runs gates 12 and 14 (`.github/gates_bounds.rs`) and the decision-citation test on its branch, because its ledger rows are on it.
- **Bench shape.** The measuring function lives in the part's library module (`pub fn measure_*`, returning samples). A unit test runs it at tiny sizes, so it is covered. The bench file under `benches/ratio/` is a thin wrapper that prints p50, p99 and max at 1x, 10x and 100x, then asserts that the p99 ratio is flat within the crate's `CEILING_PERMILLE` and inside an absolute budget. How Route B treats the wrapper file is decision 9.8.
- **Telemetry.** A `runner` or `indicators` part emits none (gate 17). Its cli caller logs.
- **Integers only.** Prices are paisa `i64`, money is paisa `i64`, ratios are ppm or exact rationals, and comparisons cross-multiply in `i128` (FIB-I10).

### FNO-SEAM (wave 0, F1)
- **Goal:** create every slot of section 4.2, so no two FNO parts ever edit the same line. First run: yes, as everything is cut from it.
- **Files:** E `crates/runner/src/lib.rs`, `crates/runner/benches/ratio.rs`, `crates/indicators/src/lib.rs`, `crates/indicators/benches/ratio.rs`, `crates/cli/src/lib.rs`, `crates/cli/benches/ratio.rs`, `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md`. Comments and blank lines only.
- **Tests:** none new; the existing workspace stays green. **Benches:** none. **Coverage:** no instrumented region. **Depends on:** nothing.
- **Size:** about 0.25-0.5 h, S. It is mechanical: the launching session commits it before wave 1 starts.

### FNO-LAW (wave 1, F1)
- **Goal:** bring the law to the operator's 30 Oct decisions, so no part changes scope silently (CLAUDE.md section 3 rule 2). First run: yes.
- **Files:**
  - E `CLAUDE.md`:
    - section 1 gains a third surface: "the cash shares of the operator's current F&O list, a data file whose blake3 enters every run identity, each traded only on the days it was a point-in-time F&O member; ICICIPRULI joined to ICICILIFE at read time; signals on Zerodha candles; fills on GDFL 1-second bars under one model, the worst printed price in the second; charges on two clocks, the worse per trade; every result labelled ...";
    - the paragraph that bars equity results from authority "until a charter-sourced equity charge stack exists" is updated to name FNO-COSTS as that stack;
    - section 3 rule 3 gains one sentence naming the F&O path's identity domain (FNO-SEC-WIRE).
  - E `AGENTS.md` (the mirror).
  - E `docs/00-charter.md`: source rows for:
    - the 25 dated cost rows (ids EX-2009 .. GST-SEBI, URLs, read 2026-10-30, sha256 in COSTS's `costs-evidence/sha256-official.txt`);
    - NSE C_VAR1 margin files (T02-02);
    - Dhan's 15:10 square-off and Rs 20 + GST fee (T03-01, T03-05);
    - the 15:08 rejection window (T03-04);
    - the order-rate ceilings (T09-07);
    - the 3% market protection (T10-05);
    - the band rule (T06-03);
    - the closing-auction times (T11-09).
  - E `docs/05-decisions.md` at its slot, D-8100..D-8109:
    - D-8100: the F&O path and its universe. The 218 as data, leavers not added, point-in-time membership within, the ICICI join at read time, the label.
    - D-8101: one fill model on GDFL seconds and the live-like fill rules (operator rule 6).
    - D-8102: two cost clocks, the worse per trade (rule 7).
    - D-8103: the ladder rules 2 and 3 as law. Bits 121-131 untouched (OP-35); `TOL_FIB_MILLI` stays one point of the sweep (OP-36).
    - D-8104: no vocabulary widening for the first run. Typed feature rows and a run-local clause alphabet; the vocabulary only as filters (OP-37).
    - D-8105: walk-forward always and the one goal (rules 4 and 5); "nothing passed" reported plainly.
    - D-8106: 1 day on this path means decide on D, trade on D+1 (OP-44). The old path's refusal is unchanged.
    - D-8107: this file's slot and family process.
    - D-8108: the choices for SPEC's open points OP-01..OP-46, one line each.
    - D-8109: spare.
  - E `docs/07-plan.md` and `docs/04-invariants.md` at their slots.
  - N `crates/core/tests/fno_law.rs`.
- **Tests (`fno_law.rs`):**
  - CLAUDE.md section 1 holds the survivorship label verbatim and the universe-file rule;
  - `docs/05-decisions.md` heads D-8100..D-8108;
  - `docs/00-charter.md` names each of the 25 cost-row ids and the margin-file source.
  - The existing `decision_citations.rs`, `crate_graph_claims.rs`, `banned_lists.rs` and `graph.rs` stay green.
  - Invariant rows are written only for law statements this test proves. Each part writes its own invariant rows beside its own tests, so no row ever cites a test that does not exist.
- **Benches:** none. **Coverage:** `fno_law.rs` whole. **Depends on:** FNO-SEAM.
- **Size:** 2-4 h, S.

### FNO-COSTS (wave 1, F1)
- **Goal:** make `crates/costs` match COSTS.md and `costs-table.json` exactly, for cash-equity intraday from 3 Sep 2018 to today:
  - the dated clock;
  - every worst-case reading RD-* (COSTS 6);
  - both clocks priced per trade, the worse kept and both shown;
  - the table version in the identity;
  - the 24 contract notes as tests.

  First run: yes.
- **Files:**
  - N `crates/costs/src/equity_dated.rs`:
    - `pub struct DatedRow { id: &'static str, component: Component, rate: Ratio, base: Base, side: Side, from: TradeDay, to: Option<TradeDay>, source: &'static str, status: Status }`;
    - `pub const EQUITY_DATED_ROWS: [DatedRow; 25]`, transcribed from `costs-table.json` `rows`;
    - `pub const EQUITY_DATED_VERSION: &str = "costs-table/2026-10-30/r1"`;
    - `pub fn rates_on(day) -> EquityRates`. Each component has at most 6 rows, so this is a bounded walk: O(1). Rates are the exact `Ratio`, so Rs 306.99/cr and Rs 0.01/cr are representable (COSTS 7 item 2);
    - `pub struct StateStamp` (SD-STATE rows: rate, cap per order or per day, SECONDARY source);
    - `pub fn worst_state_stamp(day_turnover) -> StateStamp` (Telangana up to Rs 16.67 lakh a day, Tamil Nadu above it, COSTS 3).
  - N `crates/costs/src/equity_readings.rs`:
    - `pub enum Reading`, every RD-* row: BrkPerOrder/BrkPerFill, SttVwap/SttSellValue/SttPerSellOrder, RoundClientDay/RoundSecurityDay/RoundPerOrder, ExOnce/ExSplit/ExPerOrder, Gst18/GstCgstSgst, DpNone/DpIntraday, SqOnTop/SqInstead, StateStamp(State), SlabTop;
    - `pub fn note(input: &NoteInput, clock: Clock, readings: &ReadingSet) -> NoteLines`, every line in paise;
    - `pub fn worst(input, clock) -> (NoteLines, ReadingSet)`.
  - N `crates/costs/src/equity_clock.rs`:
    - `pub enum Clock { Dated, Today }`;
    - `pub struct TwoClock { dated_official, dated_worst, today_official, today_worst, headline }` with `headline = max(dated_worst, today_worst)`, both shown (COSTS 6 last line);
    - `pub fn price_trade(fills: &TradeFills, day: TradeDay) -> Result<TwoClock, ChargeError>`. `TradeFills` holds legs, each an order with its per-second fills `(ts, qty, price)`, plus the square-off orders;
    - the Dhan data API constant (Rs 499 + GST a month, RES/sources-charges.md line 37) with its source, for the account replay.
  - E `crates/costs/src/schedule.rs`: in `dated_card`, the `TradeKind::Equity(Intraday)` arm returns the dated equity card from `equity_dated`. Delivery and MTF stay refused, with the same message narrowed to name them. `DATED_RATES_VERSION` moves to r3, and its pin test moves with it.
  - E `crates/costs/src/lib.rs`: three `pub mod` lines (sole owner).
  - N `crates/costs/tests/fno_contract_notes.rs`.
  - E `crates/costs/benches/ratio.rs`: three rows (sole owner).
- **Data structures:** paise `i64`; `Ratio` from `schedule.rs`; nothing with a float.
- **Tests:**
  - The 24 vectors CN01..CN24 (COSTS 9; `costs-table.json` `test_vectors`), each under four columns (dated official, dated worst, today official, today worst), asserted to the paisa, plus the headline.
  - Each rate change has a test on the trading day before and the day after: SEBI 15->10 on 2019-04-01; relief 2020-06-01 and 2021-04-01; stamp 2020-07-01; NSE 325->345 on 2021-01-01; GST on the SEBI fee from 2022-07-18; 345->325+10 on 2023-04-01; 325->322 on 2024-04-01; slabs to flat on 2024-10-01; 297+10 to 306.99+0.01 on 2026-03-01.
  - Each reading alone and every pair that interacts: per-fill brokerage with fills under Rs 66,666.67 (CN24); STT on the sell value when SVAL > BVAL; the exact half-paisa VWAP (CN14/CN15).
  - The square-off fee on top (CN22/CN23).
  - The quantity tie priced both ways (CN22: 313, CN23: 312; the pin is FNO-NET's).
  - The crate's golden V1..V5 unchanged (`stack.rs` 1808-1861).
  - Delivery dated still refused by name.
  - Before-window and after-today dates refused by name.
  - Property: for random fill sets, `headline >= every single-clock total`, and `worst >= official` under each clock.
- **Benches:**
  - C-FNO-COSTS-01: `rates_on` flat at 1x/10x/100x days, p99 <= 200 ns.
  - C-FNO-COSTS-02: `price_trade` per fill flat for 1..512 fills, p99 <= 2 µs per fill.
  - C-FNO-COSTS-03: `worst` with every reading, a fixed count, p99 <= 30 µs per trade.
- **Coverage:** the three new files whole; `dated_card` whole; the test file.
- **Depends on:** FNO-SEAM (ledger slots only).
- **Size:** 6-10 h, L.

### FNO-SEC-EXEC (wave 1, F1)
- **Goal:** the one seconds fill walk, operator rule 6 exactly (SPEC 9 and 10). Pure: no I/O, no telemetry. First run: yes.
- **Files:**
  - N `crates/runner/src/fno_exec/` with `mod.rs`, `walk.rs`, `order.rs`, `session.rs`, `tests.rs`;
  - E `crates/runner/src/lib.rs` (`pub mod fno_exec;` at its slot);
  - N `crates/runner/benches/ratio/fno_exec.rs`;
  - E `crates/runner/benches/ratio.rs` (two slot lines).
- **Data structures and functions:**
  - `pub struct Second { ts: i64, high: i64, low: i64, last: i64, volume: i64 }`: raw paise. A second with no trade has no bar (rule T, ENGINE 3.1).
  - `pub trait SecondSource`:
    - `fn first_traded_at_or_after(&mut self, ts) -> Result<Option<Cursor>, SourceError>`;
    - `fn second(&mut self, c) -> Result<Second, SourceError>`;
    - `fn next(&mut self, c) -> Result<Option<Cursor>, SourceError>`;
    - `fn last_traded_at_or_before(&mut self, ts) -> ...` (the sizing price, OP-15).

    FNO-SEC-WIRE implements it over `BarFile`; tests use an in-memory vector.
  - `pub struct SessionClock { continuous_end, square_off /*15:10:00*/, self_exit_cutoff /*15:08:00*/, data_start: Option<i64> /*2019-05-30 11:57:47*/, outage: Option<(i64, i64)> /*2021-02-24*/, drop_auction_print: bool }`. Built by FNO-DAYMASK's `DayPlan`; the facts are sourced, not settings.
  - `pub trait Bands { fn limits(&self, ts) -> (i64, i64); }`. The "no band" form is for tests; FNO-BAND supplies the real one through FNO-RUN.
  - `pub enum OrderKind { Entry, StopEntry { trigger }, Stop { level }, Target { level }, TimeExit { at }, ConditionExit { at }, BrokerSquareOff, MtmSquareOff }` and `pub struct Order { side, qty, kind, start_not_before, protection: Option<i64> }`.
  - `pub struct Lane { share_ppm: Option<u32> /* None = no cap */, latency_s: u32 }`. One walk carries the run's fixed set of lanes, so the cost is O(lanes) per second, a constant per run.
  - `pub struct PositionWalk`: one stock's entry and exits, sharing one per-second volume budget `used(s)` (OP-11). `pub fn step(&mut self, sec: &Second) -> StepEvents` is O(1).
  - `pub enum Event`:
    - `Fill { ts, qty, price }`;
    - `Cancelled { reason }`;
    - `Refused { BandLocked | ProtectionExceeded | BeforeDataStart | AfterSelfExitCutoff | ZeroQty }`;
    - `Closed { Stop | Target | Time | Condition | BrokerSquareOff | MtmSquareOff | SessionRemainder | UnclosableInSession }`.
  - **Fill rule:** `q(s) = min(Q, floor(c * v(s)) - used(s))`. The price is that second's high for a buy and its low for a sell, through `costs::fill::Anchor::PrintedExtreme` (`fill.rs` 322-357, 421). Never a level, an open or a mid.
  - **Remainder rule:** keep a running worst price from 15:10:00. At the last continuous second, the remainder closes at that worst. With no traded second in that window, it closes at the worst since entry, flagged `UnclosableInSession`. Both are O(1) per second.
  - **Market protection:** a limit at LTP x 1.03 (or Rs 1.5 at LTP <= Rs 50), rounded against the order. A second whose worst print lies beyond it fills nothing (OP-13).
  - **MTM 80%:** of the position's margin (OP-33). The margin comes from FNO-PORTFOLIO as plain paise.
  - **15:08 and 15:10:** an exit order starting at or after 15:08:00 is rejected and left to the broker (OP-34). At 15:10:00 the broker square-off walks the remainder, with orders counted for the fee.
- **Tests:**
  - **Fill walk (SPEC 17):** a zero-volume second; a no-trade gap of minutes; caps at no cap, 100%, 50%, 25%, 10% and the minimum share; `floor(c*v) = 0`; entry and stop in one second; a stop while `Q_f = 0` (cancelled, counted); a target filled below its level; a buy at the upper band and a sell at the lower band, then resuming off the band; 3% protection exceeded; a halt (frozen price, zero volume).
  - **Special days:** 2019-05-30, where an order starting before 11:57:47 is refused, never delayed; 2021-02-24, decisions to 11:39 and the remainder closed at the worst of 15:45 to the end.
  - **Auction era:** session end at 15:14:59 for eligible stocks from 2026-08-03 and 15:29:59 for the 44 non-eligible stock-days; the served auction print dropped.
  - **Session end:** the session end with a remainder; no trade after 15:10 (`UnclosableInSession`); an exit order starting at 15:07:59 and at 15:08:00; the MTM 80% rule.
  - **Out of the box:** prices <= Rs 50; a quantity larger than every second's volume all day; a source error mid-walk (refused by name, nothing invented); `i64` products guarded with `i128`.
  - **Properties:**
    - P1, conservation: filled plus cancelled plus refused equals ordered;
    - P2: every buy fill equals that second's high and every sell fill its low;
    - P3: a zero-volume second never fills;
    - P4: per-second fills never exceed `floor(c*v) - used`;
    - P5: no fill before the start second plus latency;
    - P6: a lower cap never fills more in any prefix;
    - P7, no look-ahead: changing any second after s changes nothing at or before s.
- **Benches:**
  - C-FNO-EXEC-01: one second step with 20 open positions (an entry and two exits each), 20 pending entries and 6 lanes, over 1x/10x/100x seconds walked. p99 <= 1.5 µs per second, excluding the source's disk read, which FNO-SEC-WIRE's bench reports.
  - C-FNO-EXEC-02: the remainder and protection checks, p99 <= 100 ns.
- **Coverage:** `fno_exec/` whole; the slot line has no region.
- **Depends on:** FNO-SEAM.
- **Size:** 8-12 h, L.

### FNO-LADDER (wave 1, F1)
- **Goal:** Fibonacci ladders on every anchor, with the operator's rules 2 and 3 as type-level discipline. This is K07's strategy subset (SPEC 6). First run: yes.
- **Files:**
  - N `crates/indicators/src/fno_ladder/` with `mod.rs`, `anchor.rs`, `rung.rs`, `relation.rs`, `swing.rs`, `tests.rs`;
  - E `crates/indicators/src/lib.rs` at its slot;
  - N `crates/indicators/benches/ratio/fno_ladder.rs`;
  - E the indicators bench slots.
  - The module doc quotes no "**N vocabulary positions**": it emits none, so `module_doc_counts.rs` and `shared_core_doc.rs` stay as they are.
- **Data structures:**
  - `pub struct Ladder { anchor: AnchorId, lo: i64, hi: i64, leg: Leg /* indicators::Leg reused */, both_ways: bool, defining: (first_open, last_close), known_at: i64, mode: Running | Frozen, tainted: bool }`.
  - `pub enum AnchorId { DayRunning, DayFrozen { at }, Push(u16), Swing { k }, Orb { minutes }, Prev { sessions }, Gap, Measured(u16) }`.
  - Push and Measured come from outside, through `fn install(&mut self, Anchor { lo, hi, leg, defining, known_at })`; FNO-STRAT feeds them from FNO-PAUSE events.
  - Rungs in ppm: retracements 0, 236000, 312000, 382000, 500000, 618000, 650000, 786000, 1000000; extensions 1272000, 1618000, 2000000, 2618000, 3000000 **in the move's direction**: an up leg's extension x sits at lo + x·R, above the high (OP-35). The named rungs are the E domain; the continuous position p in ppm of R is the G feature (SPEC 6.2).
  - `pub struct Judgement { anchor, leg, p_ppm, relation: [Relation; 14], one_rung_inside: Option<u8>, same_rung_streak: [u16; 14], prev_rung: Option<u8>, packed: bool, dist_close_ppm: [i64; 14], dist_frac_r_ppm: [i64; 14] }`.
  - `Relation`: WickTouch, BodyThrough, BodyInside, CloseAbove, CloseBelow, Between, None, computed with a tolerance passed in (`Tolerance { FracR | Ppm | Steps, value }`), never a pinned one (OP-36).
- **Discipline (FIB-I2/I3):** `pub fn step(&mut self, c: &Candle, incomplete: bool, tol) -> ArrayVec<Judgement, MAX>` does three things in order:
  1. judge `c` against every ladder with `known_at <= c.open` and not tainted;
  2. fold `c` into the running anchors (the day range, the swing detector, the opening-range accumulators);
  3. set each changed ladder's `known_at` to `c`'s close.

  The same-candle rule:
  - **Engine reading (default):** a candle that makes both new extremes leaves the leg Undetermined, and no ladder is used until a later one-sided extreme; that ladder is first used on the next candle.
  - **V-SEC:** `fn resolve_order(&mut self, candle, first_max_ts, first_min_ts, agree: bool)`; FNO-STRAT supplies the seconds.
  - **V-BOTH:** up from the low and down from the high, labelled.

  `pub fn usable_at(&self, ts) -> bool` is the seconds-walk rule `K <= s`. An incomplete candle (a Kite-hole minute) taints every ladder that includes it for the rest of the session (OP-41).
- **Tests (SPEC 17, ladder list), on 1, 2, 3, 5, 10, 15, 30 and 60 min and 1 day:**
  - the candle that sets a new high is judged against the old ladder;
  - an engulfing candle is judged, then the leg becomes Undetermined;
  - the first candle alone gives no ladder;
  - (true, true) then (false, false) stays Undetermined; then a one-sided extreme, and the ladder is used from the NEXT candle;
  - a frozen ladder never changes after its known-at time;
  - an opening range of N is first used at 09:15+N on each timeframe, including 2 and 3 minutes, which do not divide N;
  - a swing is used only after its confirming candle (k = 1..4);
  - yesterday and the last k sessions are used from today's first candle;
  - the gap leg;
  - the seconds walk at second s never reads a ladder with K > s;
  - V-SEC: max and min in one second gives Undetermined; feeds that disagree fall back; the 1-minute sub-variant on 5 minutes;
  - V-BOTH labels;
  - a tainted ladder is unused;
  - `CurDayFib` (bits 121-131) is untouched: the existing tests stay green;
  - extensions in the move's direction for up and down legs;
  - R = 0 and a one-tick range;
  - `i128` products for R up to Rs 1 crore.
  - **Property:** for random sessions, shuffling every candle after i leaves every judgement of candles 0..i unchanged.
- **Benches:** C-FNO-LADDER-01: one candle step with 16 ladders (including both-ways) x 14 rungs, flat from 1k to 200k candles, p99 <= 2 µs.
- **Coverage:** `fno_ladder/` whole.
- **Depends on:** FNO-SEAM.
- **Size:** 6-10 h, L.

### FNO-DAYMASK (wave 1, F1)
- **Goal:** which stock-days a run may trade (SPEC 3), carrying the operator's 21:19 decision with its own tests. First run: yes.
- **Files:**
  - N `crates/cli/src/fno_daymask/` with `mod.rs`, `universe.rs`, `join.rs`, `skip.rs`, `plan.rs`, `tests.rs`;
  - E `crates/cli/src/lib.rs` at its slot;
  - N `crates/cli/benches/ratio/fno_daymask.rs`;
  - E the cli bench slots.
- **Data step (outside the repo).** The builder writes `state/fib-strategy-20261030/inputs/fno-skip-v1.tsv`. It is a normalised, typed copy of the 79 rows of `backtest-skip.tsv`, one rule per line:
  - scope: ALL, ZERODHA_KEYS_AT_DAY or a symbol list;
  - feed, timeframes, day_from, day_to, time_from, time_to;
  - action: skip_day | skip_window | end_at | refuse_before | drop_auction_print | no_1day_signal | outage_window;
  - reason_id, and a source pointing back to the audit row.

  The 44 auction-era non-eligible stock-days become end_at 15:29:59 rows. The part's checker confirms, with a jq script, that the normalised rows map one to one onto the audit rows. The repository holds no `.tsv` (CLAUDE.md section 2), so the tests embed a copy as Rust constants.
- **Functions:**
  - `Universe::load(path) -> (Universe, Digest)`, the 218 lines;
  - `JoinMap::load(path)`, one row today: ICICIPRULI to ICICILIFE, switch 2026-10-23;
  - `JoinMap::keys_for(symbol, day) -> [&str; 2]`, in order. It returns the store keys that may hold that day, so the reader takes the first key whose month exists. That works before and after the D-7311 re-key;
  - `SkipTable::load(path)` and `SkipTable::rules_for(sym, day) -> &[Rule]`, O(1) by a day index and a symbol set;
  - `DayMask::verdict(sym, day) -> MaskVerdict { Tradable(DayPlan) | Refused(Reason) }`. The reasons M1..M11 are checked in a fixed order, and the first failing one is the reason;
  - `DayPlan { session: SessionClock fields, hole_minutes: [u64; 6] /* 375-bit set */, flags }`;
  - `MaskCounts` per (stock, year, reason) and `pit_outside_218(day)` (`Membership::pool_size` less the 218 members);
  - `pub const LABEL: &str = "current F&O members only (survivorship bias not removed)"`.

  Membership comes through `pull::fno_membership::Membership::is_member`, the calendar through `pull::calendar::kind_of` and `tradable_day`.
- **Tests (the 21:19 decision):**
  - The universe: exactly 218 names; the 13 names outside today's code list (ANANDRATHI .. UJJIVANSFB) are admitted; BAJAJHLDNG and DALBHARAT are never in it.
  - Point-in-time membership within the 218: a member day is traded, a non-member day is refused (`not_member`), and a `MembershipUnknown` day is refused, never filled from today's list.
  - The label constant equals the operator's string byte for byte.
  - The join: ICICIPRULI history before 2026-10-23 and ICICILIFE from it; a member under either spelling is a member; both store states (re-keyed and not); Oct 2026 skipped.
- **Tests (the rest of SPEC 17's day-mask list):**
  - every normalised skip rule, each with one test;
  - BE/T2T (M8) from an input table, refused when the table says T2T; with no table, the run label says "T2T unchecked" (decision 9.4);
  - Muhurat and special sessions;
  - the 2019-05-30 data start and the 2021-02-24 outage plan;
  - the 17 market-wide Kite-hole days and per-stock hole minutes in `hole_minutes`;
  - demerger ex-days (M7, M11);
  - ZOMATO to ETERNAL;
  - the 12 stocks with no Zerodha data fail M5 and are counted;
  - a provisional last day (M4);
  - 13 and 23 Mar 2020 flagged as halt days.
- **Property:** shuffling the rule load order changes no verdict, and the counts sum to the member-days.
- **Benches:** C-FNO-MASK-01: one verdict, flat in the days held, p99 <= 3 µs.
- **Coverage:** `fno_daymask/` whole. **Its bench line (preflight F11): this file's end, READINESS ADDENDUM item 13.**
- **Depends on:** FNO-SEAM.
- **Size:** 4-6 h, M.

### FNO-LEVERAGE (wave 1, F1)
- **Goal:** each stock's dated intraday leverage from NSE's daily margin files, with three readings (SPEC 11.1-11.2; OP-16, OP-17). First run: yes.
- **Files:**
  - N `crates/pull/src/nse_margin.rs`;
  - E `crates/pull/src/lib.rs` (`pub mod nse_margin;` between `nse` and `nseindex`, sole FNO owner in F1);
  - N `crates/pull/benches/ratio/nse_margin.rs`;
  - E `crates/pull/benches/ratio.rs` (sole owner in F1). **Also E `.github/gates_tree.rs`, one literal group for gate 1d: this file's end, READINESS ADDENDUM item 12.**
- **Functions:**
  - `parse_c_var1(bytes) -> Vec<MarginRow { symbol, series, var_ppm, elm_ppm, extra_ppm }>`: the EQ rows, columns 7, 8 and 9 (`sources-leverage-execution.md` section 1, items 1-2);
  - `build_from_dir(src, out) -> Report`: writes `<out>/nse-margin-v1/<YYYY-MM>.nsm`, fixed stride, plus a symbol dictionary;
  - `MarginTable::open(dir)`;
  - `margin(symbol, day) -> Result<Margin, MarginUnknown>`, O(1): a symbol hash and a day offset;
  - `leverage(m, reading) -> Ratio = min(5, 100 / max(20, M))`, exact;
  - `Reading { Day, Today, WithExtra }`;
  - `file_digest_hex()`.

  It does no network I/O. The download is an operations step with the operator's yes (decision 9.3). The build verb is wired by FNO-RUN.
- **Tests:**
  - parsing fixture rows (constants in the test); malformed rows refused by name;
  - a missing day gives `MarginUnknown`, never another day's value;
  - M = 25% gives 4x; M = 22.11% gives the exact ratio 100/22.11; M = 12.5% caps at 5x;
  - the column-9 reading;
  - batch 1 of the trading day;
  - the 1 Jun 2020 change of method, same totals (T01-21);
  - the symbol on a rename day (the caller supplies the dated symbol);
  - a non-EQ series ignored;
  - today's file (L_today).
- **Benches:** C-FNO-LEV-01: one lookup, flat in the months held, p99 <= 1 µs.
- **Coverage:** `nse_margin.rs` whole.
- **Depends on:** FNO-SEAM.
- **Size:** 3-5 h, M.

### FNO-RANK (wave 2, F1): K16's strategy subset
- **Goal:** the top-mover rank across the 218 every minute, known at decision time (SPEC 5). First run: yes.
- **Files:**
  - N `crates/runner/src/fno_cross_section/` with `mod.rs`, `network.rs`, `tests.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_cross_section.rs`;
  - E the runner bench slots.
- **Data structures:**
  - `Pool<const SLOTS: usize = 256>`. Per slot: stock index, in-pool flag, the previous close under two bases (S10 a and b), the last 1-minute close and its close time.
  - `fn fold_minute(&mut self, t, updates: &[(slot, close, close_ts)]) -> RankFrame`.
  - The move is m = (P - C)/C, kept as `(num, den)`. Comparisons are cross-multiplied in `i128`.
  - A fixed bitonic network over 256 slots, the same 4,608 compare-exchanges every minute (KIT K16, `per_bar_work`). Out-of-pool slots are padded with sentinels. One linear pass gives competition ranks: ties share a rank, and a tie at the edge of top N includes every tied stock (OP-05).
  - `RankFrame { gainer_rank: [u16; 256], loser_rank: [u16; 256], m: [(i64, i64); 256], stale_s: [u32; 256], pool_size: u16 }`.
  - Each frame is appended to the per-stock-day rank series (375 u16 a day, about 0.75 KB), which FNO-STRAT reads for "entered", "ordinal", "minutes in top N" and "best rank" at any N and W with O(1) incremental counters.
  - `fn fold_day(...)` gives the daily ranks for 1 day.
- **Tests (SPEC 17 rank list):**
  - ties at the top-N edge;
  - a stale stock carried and flagged;
  - a stock with no candle yet (not in the pool);
  - the pool changing on a membership day;
  - the 12 no-Zerodha stocks absent;
  - a Kite-hole minute (stale carry);
  - a demerger ex-day excluded (M11);
  - 218 padded to 256 with an identical compare-exchange count every minute;
  - both bases;
  - an all-stale minute;
  - a pool of 1.
  - **Properties:** the network's order equals a reference sort over random inputs; the rank at T is unchanged when any candle after T is altered.
- **Benches:** C-FNO-RANK-01: one minute fold with 218 live slots, over 1x/10x/100x minutes, p99 <= 15 µs and p99/p50 <= 1.2 (fixed work).
- **Coverage:** whole.
- **Depends on:** FNO-SEAM.
- **Size:** 4-6 h, M.

### FNO-PAUSE (wave 2, F1): K06's strategy subset
- **Goal:** the push, box and break machine and the partial U (SPEC 7.1-7.3, P4 and P8). First run: yes.
- **Files:**
  - N `crates/indicators/src/fno_pause/` with `mod.rs`, `push.rs`, `boxm.rs`, `shape.rs`, `tests.rs`;
  - E indicators `lib.rs` slot;
  - N `crates/indicators/benches/ratio/fno_pause.rs`;
  - E the indicators bench slots.
- **Data structures:**
  - One machine per (side, K) for the run's K set (form D, so the set is fixed per run).
  - `Push { start, extreme, size_ppm, candles, volume, new_extremes, rungs_crossed /* level prices passed in */ }`.
  - `BoxState { first_open, hi, lo, height_ppm, holds: HoldFlags /* S33 readings vs a level price passed in */, first_low, broke, failed, ordinal, staircase_ok }`.
  - `PartialU { candles: 2 | 3, depth_ppm, symmetry_ppm, close_loc_ppm }`.
  - `fn step(&mut self, c: &Candle, levels: &LevelInputs) -> PauseOut`.
  - `PauseOut::anchor_events`: Push and Measured ladders with their known-at times, which FNO-STRAT installs into FNO-LADDER.
- **Rules:**
  - The box starts no earlier than the candle after the push extreme ("retracement never inside the high's own candle").
  - The push leg is known at the close of the first candle not making a new extreme.
  - The break is decided at the closing candle's close.
- **Tests:**
  - every output true, false and unknown on hand-built bars;
  - the push-extreme candle is never in the box;
  - a new high inside the box ends it and restarts the push;
  - an engulfing candle;
  - K = 1;
  - each holding rule;
  - a failed break inside the failure window;
  - staircase intact and broken;
  - the pause ordinal;
  - a partial U of 2 and of 3 candles;
  - the short mirror;
  - flat (colourless) candles;
  - an incomplete candle.
  - **Property:** the prefix property (outputs at bar N are identical when every later bar is removed).
- **Benches:** C-FNO-PAUSE-01: one candle step with 16 K values x 2 sides, flat from 1k to 200k, p99 <= 2 µs.
- **Coverage:** whole.
- **Depends on:** FNO-SEAM. It takes ladder levels as plain prices, so it does not depend on FNO-LADDER.
- **Size:** 6-9 h, L.

### FNO-NET (wave 2, F1)
- **Goal:** quantity, leverage reading and charges per trade, net and signal-only (SPEC 11.2, 12, 13.1). First run: yes.
- **Files:**
  - N `crates/runner/src/fno_net.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_net.rs`;
  - E the bench slots.
- **Functions:**
  - `quantity(target_value, price) -> Result<u64, Refusal>` gives the closest integer. An exact tie takes the lower quantity (OP-14, pinned in D-8108). q = 0 is refused as `price_above_position`.
  - `target_value(leverages: &[Ratio]) = Rs 1,00,000 x min(readings)`. The leverages arrive as plain rationals from the cli, because runner has no `pull` arrow.
  - `trade(fills, day) -> NetTrade { gross, two_clock: TwoClock, net = gross - headline, signal_only = gross, square_off_orders, lines }`, through `costs::equity_clock::price_trade`.
  - `monthly_fixed(month_has_trades)`: the Dhan data API line (OP-28).
- **Tests:**
  - 5,00,000 / 1,600 = 312.5 gives 312;
  - q = 0;
  - M = 25% gives Rs 4 lakh;
  - the minimum of the three readings;
  - CN01..CN24 through `trade()` equal COSTS's headline;
  - the square-off order count;
  - signal-only equals gross, still with worst fills;
  - a losing trade's net below gross;
  - a win counts only when net > 0.
- **Benches:** C-FNO-NET-01: per fill flat, p99 <= 2 µs.
- **Coverage:** whole.
- **Depends on:** FNO-COSTS (its types). Cut from FNO-COSTS's passed tip.
- **Size:** 3-5 h, M.

### FNO-PORTFOLIO (wave 2, F1)
- **Goal:** the money rules and the order-rate rules, with exact tie-breaks (SPEC 9.6-9.7, 11.3-11.6). First run: yes.
- **Files:**
  - N `crates/runner/src/fno_portfolio.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_portfolio.rs`;
  - E the bench slots.
- **Data structures:**
  - `Account`:
    - 20 slots in a fixed array;
    - a per-stock lock (`[Option<u8>; 256]`);
    - the trades today (100 at most);
    - exposure and margin-in-use as running sums;
    - capital: Rs 20 lakh plus the carried net under the worse clock (OP-21; the reset-daily reading is shown too) (2 Nov 2026: where capital starts in each window: this file's end, READINESS ADDENDUM item 5);
    - order-rate rings for 10 a second, 250 a minute, 1,000 an hour and 7,000 a day;
    - modification counters (25 per order);
    - the tie-break order (a)-(e), with the seed for (e).
  - `fn admit(&mut self, req) -> Admission { Admitted { slot } | Refused(PositionCap | TradeCap | ExposureCap | MarginCap | WhileOpen) | Queued(next_second) }`.
  - `fn release(...)`.
  - `fn schedule_second(&mut self, intents) -> Ordered`: exits before entries, then by tie-break.
- **Tests:**
  - the 20th position admitted and the 21st refused;
  - the 100th trade admitted and the 101st refused, with cancelled entries counted;
  - exposure at exactly Rs 1 crore admitted and +1 paisa refused;
  - margin exhausted by carried losses;
  - one trade per stock across recipes in procedure mode;
  - every tie-break order;
  - the seeded spread reproducible from its seed;
  - 20 simultaneous exits over the 10-a-second limit;
  - 25 modifications;
  - the slot taken at send and freed at the last fill or a zero-fill cancel;
  - the daily reset.
  - **Property:** after any random sequence, slots <= 20, exposure <= Rs 1 crore, trades <= 100 and no stock locked twice.
- **Benches:** C-FNO-PORT-01: admit and release with 20 slots full, p99 <= 300 ns, flat.
- **Coverage:** whole.
- **Depends on:** FNO-SEAM.
- **Size:** 4-6 h, M.

### FNO-EXITS (wave 2, F1): KIT X1-X3's strategy subset
- **Goal:** the exit dimension on seconds: trailing stop (including the stair-step to each new pause low), trailing take-profit (the `grid::Ttp` semantics, `grid.rs` 90-110), the break-even move, time exits, condition exits, and the S64 family enumeration. Every exit family includes a stop; a stop-less run is only a labelled comparison. First run: yes.
- **Files:**
  - N `crates/runner/src/fno_exits.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_exits.rs`;
  - E the bench slots.
- **Functions:**
  - `families(settings) -> impl Iterator<Item = ExitFamily>`;
  - `ExitLanes::on_second(&mut self, sec) -> Vec<OrderAction>`, O(lanes);
  - each level move is an order action that takes effect at the first traded second at or after its decision plus the latency, and counts toward the 25-modification limit.
- **Tests:**
  - each exit alone and in families ("whichever first");
  - stop and target in one second: the stop is taken;
  - a trailing step with latency;
  - the 26th modification triggers cancel and replace;
  - a time exit at 15:07:59 and at 15:08:00;
  - a condition exit with its protective stop live;
  - the break-even move "never";
  - a stop-less family refused outside the labelled comparison.
  - **Property:** no exit acts on a second before its trigger.
- **Benches:** C-FNO-EXITS-01: per second per lane, p99 <= 300 ns.
- **Coverage:** whole.
- **Depends on:** FNO-SEC-EXEC (its types). Cut from its passed tip.
- **Size:** 4-6 h, M.

### FNO-BAND (wave 2, F1)
- **Goal:** the price band per stock-day, computed for every date: the base price, ±10%, then the flex observed in the data, 5%, then 3%, then 2% (T06-03). It feeds rule 4 of the fill walk. First run: yes, labelled "band computed". The security-file check from 2024-02-05 is FNO-BAND-FILES in F3.
- **Files:**
  - N `crates/runner/src/fno_band.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_band.rs`;
  - E the bench slots.
- **Functions:** `DayBand::new(base_reading, base)`; `observe(sec)`, which widens on an observed print beyond the current band; `limits(ts)`.
- **Readings:**
  - R-BAND-1, SPEC's: C_z(D-1) x the session ratio ρ;
  - R-BAND-2: GDFL's last trade of D-1.

  Both are computed. Per trade, the headline uses the reading that gives the lower net, computed as lanes, and only trades that print within one step of a band need two lanes.
- **Tests:**
  - prints exactly at the band;
  - a flex observed;
  - a sell at the lower band and a buy at the upper band;
  - resuming off the band;
  - a split day (the adjusted Zerodha close times ρ, inside one session);
  - a demerger ex-day skipped;
  - a stock whose first print of the day is already at the band.
- **Benches:** C-FNO-BAND-01: p99 <= 100 ns.
- **Coverage:** whole.
- **Depends on:** FNO-SEAM. It hands plain limits to FNO-RUN, which adapts them to FNO-SEC-EXEC's `Bands` trait.
- **Size:** 2-3 h, S.

### FNO-CLOCK (wave 2, F1)
- **Goal:** the per-stock-day GDFL clock calibration, so the headline uses only latencies at or above the calibrated floor (OP-07; D-0802 phase 2: stock fills on seconds were refused until a per-instrument calibration existed). First run: yes.
- **Files:**
  - N `crates/cli/src/fno_clock.rs`;
  - E cli `lib.rs` slot;
  - N `crates/cli/benches/ratio/fno_clock.rs`;
  - E the cli bench slots.
- **Functions:**
  - `calibrate(stock, day) -> ClockRow { offset_ms, score, back_step_s: Option<u32>, flags }`.
  - The offset: for shifts δ over a generated window (start ±60 s, doubled until the best δ is interior), fold the GDFL seconds into shifted minutes and count disagreements with Zerodha's minute OHLC, ratio-mapped inside the session.
  - A per-day sparse table gives range maxima and minima, so each δ costs 375 O(1) queries.
  - The backward clock step comes from the raw tick order, read through `pull::gdfl_tickstore` if that module exposes order. That is UNVERIFIED here; the builder checks first. If it does not, the step is "unmeasured" and the row is flagged.
  - `build(range) -> table <shadow>/reference/fno-clock-v1/`, fixed stride, with a digest. **2 Nov 2026: it takes the (stock, day) key list as its one input; a bare range cannot know the mask: this file's end, READINESS ADDENDUM item 8.**
  - The floor is `floor(s, D) = max(0, offset) + back_step`.
- **Tests:**
  - synthetic feeds shifted by known δ (±1..±30 s) are recovered exactly;
  - a day with no agreement is flagged uncalibrated;
  - holes, the auction era and 2019-05-30;
  - ties take the larger |δ| (the worse reading).
- **Benches:** C-FNO-CLOCK-01: per minute boundary, flat. The per stock-day cost is linear in the day's seconds. It is a bounded batch step, recorded in `docs/06-limits.md`, not a hot path.
- **Coverage:** whole.
- **Depends on:** FNO-SEAM.
- **Size:** 4-6 h, M.

### FNO-WFT (wave 2, F1)
- **Goal:** walk-forward exactly as SPEC 13.4 defines it (operator rule 4), plus the Winner Gate (13.3) and luck accounting. First run: yes. (2 Nov 2026: how FNO-RUN feeds this part's rows, per window where capital binds, and its length check before `Unit::new`: this file's end, READINESS ADDENDUM items 5 and 11.)
- **Files:**
  - N `crates/runner/src/fno_wft/` with `mod.rs`, `scheme.rs`, `agg.rs`, `gate.rs`, `generate.rs`, `tests.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_wft.rs`;
  - E the bench slots.
- **Schemes:**
  - `Scheme { kind: Anchored | Rolling, learn: u16, test: u16 }`;
  - `windows(span, scheme)` tiles the forward windows with no gap and no overlap;
  - `lengths(span, min_trades_len) -> (L set, T set)` doubles; `refine(...)` adds midpoints where neighbouring verdicts differ.
- **Aggregates:**
  - `MonthAgg` per (candidate, month), per side: trades, wins, gross win and loss, net, largest win and loss, per-day and per-week nets, and an equity-path summary (start, min, max, end, max drawdown) for the drawdown and recovery checks.
  - Additive fields use prefix vectors, O(1) per (candidate, window).
  - Non-additive fields (drawdown, streaks, recovery) fold over the window's months. That is O(months in the window) <= 98, bounded and recorded in `docs/06-limits.md`.
- **Generation:** `generate::from_learning(window, merged_histograms) -> Settings` (form G quantile ladders, refined where survivors crowd). The excursion ladders are fitted by `runner::excursion::Ladder::from_excursions` (`excursion.rs` 213), on learning-window excursions only.
- **The gate:** `gate(agg, learned) -> Verdict { pass, failed: Vec<Clause> }`, every clause of SPEC 13.3:
  - win rate >= 51%; average win >= 2x average loss; profit factor >= 2; largest win >= 2x largest loss;
  - >= 500 trades a year, pro-rated; every year positive;
  - leave-one-out, exact, by re-reading the passing candidate's trades from FNO-RECORDS;
  - the five downside checks, unseen against learned;
  - positive in every NIFTY regime. The regimes are falling, flat and rising, with thresholds generated from the learning months' NIFTY monthly returns (MEM/backtest-acceptance-bar line 34); FNO-RUN supplies them;
  - the minimum returns on the Rs 20 lakh base (OP-22).
- **Picking and verdicts:**
  - Among passers, the pick orders by share of profitable days, then worst day, worst week, net per day (acceptance bar, 18 Oct 13:26).
  - Recipe and procedure verdicts are given under every scheme. A not-picked month counts as not positive (OP-25).
  - Luck accounting reuses `runner::significance::effective_trials` and `bonferroni`, `runner::pbo::probability_of_overfitting` and `runner::bootstrap::reality_check`.
  - The "nothing passed" report gives the closest misses by how much.
  - The newest forward windows are reported separately (OP-23).
- **Tests:**
  - the windows tile the span (property over random spans and schemes);
  - a span too short for two forward windows is refused with its reason;
  - regenerated settings depend only on learning months (property: altering any forward month changes no learning output);
  - not-picked months count as not positive;
  - every scheme;
  - profit factor with zero losses (exact infinity);
  - leave-one-out;
  - pro-rated partial years;
  - all trades cancelled;
  - the nothing-passed report.
- **Benches:**
  - C-FNO-WFT-01: one (candidate, window) additive total, flat in total months, p99 <= 200 ns.
  - C-FNO-WFT-02: a quantile ladder per window, a bounded batch.
- **Coverage:** whole.
- **Depends on:** FNO-SEAM.
- **Size:** 6-9 h, L.

### FNO-DAILY (wave 2, F1): the strategy-mvp "FNO-1DAY"
- **Goal:** decide on D's daily candle and trade on D+1 (SPEC 4.3; OP-40, OP-44). First run: yes.
- **Files:**
  - N `crates/runner/src/fno_daily.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_daily.rs`;
  - E the bench slots.
- **Functions:**
  - `entry_for(signal_day, next_session: Option<Day>, plan: &DayPlanView, latency, later_start) -> Result<StartTs, Refusal>`: the first traded second at or after 09:15:00 + λ on D+1, or the swept later start (S43);
  - `daily_invalid_mask() -> vocab::ConditionMask`: the time-of-day bits (44-47) and the VWAP bits, chosen by name from `vocab::table::TABLE` and pinned by a test, so the 1-day filter never uses an intraday-only condition.

  The old path's 1-day refusal (`cli/src/lib.rs` 10217-10225) is untouched.
- **Tests:**
  - a Friday signal trades on Monday;
  - a holiday gap;
  - D+1 not tradable: refused `next_day_not_tradable`;
  - the Muhurat 2018-11-07 daily bar is never a signal;
  - a provisional daily bar;
  - MAZDOCK, PATANJALI and RELIANCE 1-day rules from the skip table;
  - the latency and later-start sweeps;
  - **property:** a signal on D never reads anything from D+1.
- **Benches:** C-FNO-DAILY-01: O(1), p99 <= 100 ns.
- **Coverage:** whole.
- **Depends on:** FNO-SEAM.
- **Size:** 2-3 h, S.

### FNO-SEC-WIRE (wave 2, F1)
- **Goal:** the GDFL seconds source over the store, and the run identity of the F&O path. First run: yes. The old sweep's wiring is FNO-SWEEP-WIRE in F3.
- **Files:**
  - N `crates/cli/src/fno_seconds.rs`;
  - E cli `lib.rs` slot;
  - E `crates/runner/src/identity.rs` (sole owner);
  - N `crates/cli/benches/ratio/fno_seconds.rs`;
  - E the cli bench slots.
- **The source:** `GdflSeconds` implements `fno_exec::SecondSource` over `store::file::BarFile::open_existing` (`file.rs` 2105). That honours the repaired-month pointers: 315 symbols are served from `bar-revisions-v1` (ENGINE 3.1).
  - One open handle per stock-month is cached.
  - It uses `.tix` through `first_at_or_after` (`file.rs` 3553) and `read_record` (3504), and never `stored::load` or `load_span` on `1s`.
  - A month locked by a writer is refused by name (`StoreError::Locked`), never waited on.
- **The identity:** in `identity.rs`, a new versioned domain:
  - `gdfl_month_binding_v1(header generation, n_valid, first and last stamp, .crc digest)`, so a run never hashes every second (ENGINE 3.3, item 2);
  - `FnoRunV1 { signal_feed, fill_feed, lanes, mapping, cost_table_version, leverage_digest, universe_digest, join_digest, skip_digest, t2t_digest, membership_digest, clock_digest, band_rule, wft_scheme, recipe_bytes, pool_digest, vocab_version, commit }`. `t2t_digest` (added 2 Nov 2026) is the digest `T2tTable::load` returns (FNO-RUN's brief, :968), with a distinct none term when the run has no table, so a resume with another table, or with none, has another identity and is refused; **2 Nov 2026: this list is overtaken. It splits into two levels, with eight more terms, and that binds FNO-SEC-WIRE: this file's end, READINESS ADDENDUM item 2.**
  - `fn fno_identity(&FnoRunV1) -> RunId`, with a new append-only tag. Existing Run ids are unchanged; the existing tests prove it. (2 Nov 2026: it takes `FnoCandidateV1`, and the new `fno_run_digest` takes `FnoRunV1`: item 2 of that section.)
- **Tests:**
  - a temporary store written with the store's own writer;
  - a repaired month served from its revision;
  - a locked month refused;
  - an empty month;
  - the last traded second followed by None;
  - the sizing price;
  - the binding changes when any of its fields changes and not when another month changes;
  - domain separation from `data_digest_with_execution` (`identity.rs` 397).
- **Benches:** C-FNO-SECWIRE-01: a warm second fetch through the cached handle, p99 <= 3 µs. The cold first touch is reported as device latency; ENGINE measured 118-124 µs.
- **Coverage:** `fno_seconds.rs` whole and the new `identity.rs` functions whole.
- **Depends on:** FNO-SEC-EXEC (the trait). Cut from its passed tip.
- **Size:** 3-5 h, M.

### FNO-STRAT (wave 3, F2)
- **Goal:** the focused, parameterized strategy module: feature rows, the generated clause alphabet, the level-wise search with extinction, recipes and signals (SPEC 7-8, 14; section 3.2 here). First run: yes. (2 Nov 2026: the cache takes its root as an argument, and a sample searches on N stocks over a pool of 218: this file's end, READINESS ADDENDUM item 6.)
- **Files:**
  - N `crates/runner/src/fno_strategy/` with `mod.rs`, `features.rs`, `clauses.rs`, `recipe.rs`, `search.rs`, `distrib.rs`, `norms.rs`, `cache.rs`, `tests.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_strategy.rs`;
  - E the bench slots.
- **Data structures:**
  - `FeatureRow` (per stock, timeframe and candle, about 1-2 KB, never stored for every candle). It holds:
    - from FNO-RANK at T: the rank and |m|;
    - from FNO-LADDER, per anchor: leg, p, relations, streaks, packed;
    - from FNO-PAUSE: push, box, holds, break, failed, ordinal, staircase, U;
    - candle measures: colour (flat is no colour), body and range in ppm;
    - slot norms: the stock's usual move, volume and range at this time of day, causal over all history before the decision (KIT K12's subset);
    - the measured raw price step, generated (K19's subset; OP-39);
    - the vocab `ConditionMask`, for FNO-FILTER.
  - `Dial`: the SPEC 14 ids S10..S65. `Clause { dial, value }`. `Alphabet`: the union of every window's generated values, run-local `u16` ids.
- **Functions:**
  - `distrib::Pass` (A): per (month, timeframe, side, feature), integer histograms at the feature's resolution, with log-spaced exact edges for ppm features;
  - `clauses::Pass` (B): per candle a run-local bitset, then posting lists per (clause, month) in the cache;
  - `search::level_wise(window, floor, alphabet) -> Frontier`: one clause per dial; direction and timeframe as partitions; galloping intersection; extinction below the floor; rayon over partitions;
  - `recipe::canonical_bytes()` and `RecipeId`;
  - `signals(recipe) -> Signal { stock, day, tf, T, side, entry trigger S40, stop and target in Zerodha prices plus the mapping reading R1 or R2 (R0 refused by name until R4-26 is live), ladders used }`.
  - The composition asserts FIB-I2 at runtime: every ladder used for candle i has K <= open(i), and every level handed to the seconds walk has K <= s.
- **Tests:**
  - the operator's flag on synthetic sessions (push, box holding above 0.382, break) produces exactly one signal at the break candle's close, plus its short mirror;
  - P0..P8, each;
  - a candle that sets a new extreme inside a box;
  - V-SEC and V-BOTH at composition level;
  - **nothing static:** two different distributions give two different alphabets, and no literal threshold appears;
  - extinction is exact (property: brute force over small alphabets equals the level-wise result);
  - the distribution pass is deterministic;
  - the incremental cache: adding a month leaves earlier months' digests equal;
  - an incomplete candle gives no decision;
  - R0 is refused.
- **Benches:**
  - C-FNO-STRAT-01: per candle evaluation over a fixed 2,048-clause alphabet, flat in history, p99 <= 5 µs;
  - C-FNO-STRAT-02: posting-list intersection per element, p99 <= 20 ns.
- **Coverage:** whole.
- **Depends on:** FNO-LADDER, FNO-PAUSE, FNO-RANK, FNO-DAILY and FNO-WFT (generation interface). Cut from `fam/fib-1`.
- **Size:** 8-12 h, L.

### FNO-FILTER (wave 3, F2)
- **Goal:** the existing 370 conditions as extra filters, with no vocabulary change. First run: yes.
- **Files:**
  - N `crates/runner/src/fno_filter.rs`;
  - E runner `lib.rs` slot;
  - N `crates/runner/benches/ratio/fno_filter.rs`;
  - E the bench slots.
- **Functions:**
  - `admitted_mask(run_order) -> ConditionMask`: price-action rows only in the first run. The indicator families (VWAP, pivots, CPR, EMA, SuperTrend) are excluded and labelled (MEM/let-data-decide run order).
  - `filter(recipe_signals, masks, floor) -> Vec<(RecipeId, ConditionMask)>`: the existing `engine` Apriori over the signal candles' masks, with the same extinction floor, AND-combinations only. The 1-day timeframe applies `fno_daily::daily_invalid_mask`.
- **Tests:**
  - a filter never adds a signal (subset property);
  - support equals `engine::support`;
  - the admitted families;
  - the daily mask;
  - the mask enters the recipe bytes and so the identity.
- **Benches:** C-FNO-FILTER-01: the per-signal mask gather, p99 <= 50 ns. The engine's own benches stand.
- **Coverage:** whole.
- **Depends on:** FNO-STRAT and FNO-DAILY. Cut from the F2 family branch after STRAT is merged into it.
- **Size:** 3-5 h, M.

### FNO-RECORDS (wave 3, F2)
- **Goal:** save every decision into the store's own records, searchable and auditable (THE BAR's "saving into db ... auditing ... searching"). First run: yes.
- **Files:**
  - N `crates/cli/src/fno_records/` with `mod.rs`, `rows.rs`, `index.rs`, `journal.rs`, `lean.rs`, `tests.rs`;
  - E cli `lib.rs` slot;
  - N `crates/cli/benches/ratio/fno_records.rs`;
  - E the cli bench slots.
- **Format:** `<store>/results/fno-v1/` holds fixed-stride, blake3-sealed rows, versioned like `results.rs` v3:
  - the run header (identity, every term, the label, the data-limits text id, the trial count);
  - mask verdicts;
  - signals;
  - orders;
  - fills (second, quantity, price);
  - trades (every field of SPEC 13.2);
  - refusals and cap hits;
  - `MonthAgg`;
  - window and scheme verdicts;
  - an append-only journal of every phase and stock-month (time, run, phase, counts). (2 Nov 2026: with its lifecycle rows, `run started` and the rest: this file's end, READINESS ADDENDUM item 7.)

  Per-run index files are searchable by stock, recipe and day.
- **Lean policy** (the role of the strategy-mvp FNO-LEAN on this path): aggregates for every candidate; trades and fills only for picked recipes and on request (`--keep-trades`). The projected bytes are printed before writing.
- **Safety:** the writer refuses a store root that resolves to the live `runtime/store` unless the operator passes an explicit flag. The default refuses.
- **Tests:**
  - every row kind round-trips;
  - a corrupted seal is detected;
  - a torn write is recovered (temporary file plus rename);
  - a rerun with the same identity writes nothing;
  - the label is in every run header;
  - the live store is refused;
  - the lean policy.
- **Benches:** C-FNO-REC-01: append p99 <= 5 µs (fsync batched per stock-month); read by index p99 <= 2 µs.
- **Coverage:** whole.
- **Depends on:** F1 types (`Fill`, `NetTrade`, `MonthAgg`, `Verdict`, `MaskVerdict`). Cut from `fam/fib-1`.
- **Size:** 4-6 h, M.

### FNO-RUN (wave 3, F2)
- **Goal:** the one driver of section 3.1, running in parallel, resumable, measure-then-project. First run: yes.
- **Files:**
  - N `crates/cli/src/fno_run/` with `mod.rs`, `phases.rs`, `inputs.rs`, `nifty_regime.rs`, `tests.rs`;
  - E cli `lib.rs` slot;
  - E the dispatch arms in `crates/cli/src/lib.rs` (sole owner of the `fno` verbs); **2 Nov 2026: also E `crates/cli/src/store_write.rs` and `crates/cli/src/sweep_wiring_tests.rs`: the word `fno` is classified there and stays out of `is_sweep_command`: this file's end, READINESS ADDENDUM item 1.**
  - N `crates/cli/benches/ratio/fno_run.rs`;
  - E the cli bench slots.
- **Verbs:**
  - `cli fno run <shadow-store> --from --to --universe --join --skip --t2t --membership --margin --clock [--sample N:M] [--phase p] [--keep-trades id]`. **The T2T input (added 2 Nov 2026; deploy runbook R7, R9, R11 and section 9 item 5).** For the builder: `--t2t` is this brief's name, in the form of the inputs beside it, and not yet a built flag; if your usage spells it otherwise, say so in `part-FNO-RUN.md`, and the main session corrects those three lines. The run takes the file `state/post-landing-20261101/t2t-20261102/t2t.tsv` (the runbook's `$FI/t2t.tsv`), loads it through `T2tTable::load`, which calls `T2tTable::parse` and returns its digest, which joins the run identity as `t2t_digest` (:845), gives it to `DayMask::new` as `t2t: Some(_)` so M8 refuses those stock-days, and prints `T2tTable::len` and `T2tTable::outside` once, at P0. A row `parse` refuses stops the verb by name. With no file the run behaves as decision 9.4 says: `t2t: None` and the label "T2T unchecked". Tests: `the_run_loads_the_t2t_table_and_prints_its_len`, `a_t2t_stock_day_is_refused_in_the_run`, `a_row_the_t2t_parse_refuses_stops_the_verb`, `with_no_t2t_table_every_output_says_t2t_unchecked`, `a_resume_with_another_t2t_table_refuses`; **2 Nov 2026: `--sample N:M` is defined in this file's end, READINESS ADDENDUM item 6; `--phase` is optional (item 4); the resume refusals read item 2.**
  - `cli fno margin build` (FNO-LEVERAGE's `build_from_dir`);
  - `cli fno clock build` (FNO-CLOCK); its inputs: this file's end, READINESS ADDENDUM item 8;
  - `cli fno mask` (the counts report; it takes `--t2t` as `fno run` does, and prints `T2tTable::len`);
  - `cli fno report`. FNO-RUN's builder designs it: this file's end, READINESS ADDENDUM item 9.
- **Phases P0..P8:** as section 3.1, each resumable and recorded.
  - P0 refuses when the data is not proven or the store is `runtime/`, and takes `execution_lease`. "Data proven" means what the section "MAIN-SESSION DECISION, 1 Nov 2026 ~17:3x IST: P0's "data proven"" says: its default, the whole AUDIT-3 pass. L7's yes in "OPERATOR DECISIONS, Sun 1 Nov 2026 ~19:2x IST" is withdrawn (this file's end, 2 Nov 2026) (CR6, 2 Nov 2026). **The lease and the invocation audit, by hand: this file's end, READINESS ADDENDUM item 1. P0's refusal of a held month with no `.qrg`: item 3.**
  - Parallel: rayon over stocks for P2 and P4, per day for P3, per (window, partition) for P5, and per candidate for P6.
  - Years are sharded across several shadows, because there is one lease per store (MVP correction 5). What the built parts offer for this today, and what FNO-RUN's builder must decide, is in this file's section "APPENDIX, read Mon 2 Nov 2026 ~15:5x IST". **2 Nov 2026: that reason does not hold on this path, and one process is allowed: this file's end, READINESS ADDENDUM item 4.**
  - The adapters between parts live here: BAND's limits into SEC-EXEC's `Bands`, DAYMASK's `DayPlan` into `SessionClock`, LEVERAGE's readings into NET. **Also QRANGE-1's `QuoteMonth` into `Second::with_quote`, quote-only seconds included: this file's end, READINESS ADDENDUM item 3.**
  - `nifty_regime.rs` computes the NIFTY monthly returns for FNO-WFT's regime clause.
  - Telemetry at run, phase, stock-month and stride boundaries only. **2 Nov 2026: stdout reaches a log only at exit, so the run and phase events are all a live run shows; what they hold: this file's end, READINESS ADDENDUM item 15.**
- **Tests:** an end-to-end run on a temporary store with synthetic Zerodha and GDFL months for 3-5 stocks over a few months:
  - the flag example gives one trade with known fills, charges and record rows;
  - each refusal (a missing input, a locked month, the live store, data not proven, a span shorter than two forward windows);
  - kill after any phase and resume: byte-identical records (2 Nov 2026: the journal is left out: this file's end, READINESS ADDENDUM item 10);
  - idempotent rerun;
  - an integration test for each THE BAR extreme case (section 10);
  - the label on every output.
- **Benches:** C-FNO-RUN-01: the end-to-end cost per stock-minute, flat over 1x/10x/100x months (synthetic).
- **Coverage:** whole, plus the dispatch arms.
- **Depends on:** every F1 part, FNO-STRAT, FNO-FILTER and FNO-RECORDS. Cut from the F2 family branch after those merge into it. (2 Nov 2026: FNO-FILTER moved to F3 (8.3 lever 1, :1267), so FNO-RUN does not wait for it, and `parts.json` no longer names it.)
- **Size:** 6-9 h, L.

### FNO-API (wave 3, F2)
- **Goal:** the records easy to reach: read-only JSON routes and one page for monitoring, searching and visualising. First run: yes for the routes; the page can be minimal.
- **Files:**
  - N `crates/api/src/fnojson.rs`;
  - E `crates/api/src/lib.rs` (one `pub mod` line, sole FNO owner in F2);
  - E `crates/api/src/server.rs` (route lines in the `.route` chain, sole FNO owner in F2);
  - N `web/src/routes/fno/+page.svelte`;
  - N `web/src/lib/fno.js`;
  - E `web/src/lib/speed.js`, one page-budget row (the R3-15 budget table, `speed.js` 703).
- **Routes (GET):** `/fno/runs.json`, `/fno/run.json?id=`, `/fno/windows.json?run=&recipe=`, `/fno/trades.json?run=&recipe=&page=`, `/fno/fills.json?trade=`, `/fno/mask.json?run=&stock=&year=`, `/fno/refusals.json?run=` and `/fno/events.json?run=`. (2 Nov 2026: `run=` is the run digest, `/fno/runs.json` gives each run a `state`, and a sample's rows carry its "no verdict" label: this file's end, READINESS ADDENDUM items 2, 6 and 7.)
  - Every response carries the label and the data-limits line (SPEC 19).
  - Pages hold at most 1,000 rows, as `/bars/window.json` does.
- **The page shows:**
  - the "nothing passed" banner in plain words when nothing passed;
  - recipe and procedure verdicts;
  - the forward windows;
  - net beside signal-only, and both cost clocks;
  - the fill-spread statistics;
  - refusals by reason;
  - mask counts per year with the point-in-time count outside the 218;
  - a search by stock, recipe or day.
- **Tests:**
  - route tests for every route: the label is present, paging bounds hold, an unknown run gives 404 with a reason, a malformed query is refused;
  - node tests for `fno.js`: the label is always rendered, and "nothing passed" renders;
  - the browser harness run for the new page.
- **Benches:** C-FNO-API-01: one page of trades, O(page) with an O(1) seek.
- **Coverage:** `fnojson.rs` whole and the route lines.
- **Depends on:** FNO-RECORDS.
- **Size:** 4-6 h, M.

### FNO-LEAN (F3, follow-on): the strategy-mvp FNO-LEAN on the old path
- **Goal:** the `range-all` sweep's lean run mode, so the old path stops writing about 3.4 M evidence files per stock-year (MVP line 91). The first run does not need it: the new path is lean by FNO-RECORDS.
- **Files:**
  - E `crates/cli/src/lib.rs` at 22819-22833 (`Capture::begin`);
  - E `crates/cli/src/candidate_trades.rs`, `crates/cli/src/sweep_evidence.rs` and `crates/cli/src/knobs.rs` (a knob entering `Params.policy`).
- **Tests:** the knob enters the identity; lean writes aggregates only; full mode is unchanged; the evidence count is bounded.
- **Benches:** the existing cli rows.
- **Depends on:** F2 landed (cut from `fam/fib-2`).
- **Size:** 2-4 h, S.

### FNO-SWEEP-WIRE (F3, follow-on): the old path's F&O pricing on the one walker
- **Goal:** point the existing sweep's F&O pricing, its day mask and its 1-day rule at the same code the first run uses, so there is one fill implementation everywhere.
- **Files:** `crates/cli/src/lib.rs` hooks:
  - `:8256-8264`: the execution series becomes GDFL seconds through `fno_seconds`;
  - `:8318`: the cut list is united with `fno_daymask`;
  - `:8506`: the identity terms;
  - `:10271`, `:10350`, `:14233` and `:14412`: pricing through `fno_exec` instead of `grid::evaluate_over` on the F&O path;
  - `:22630`;
  - `:20662` and `:23591`: the record;
  - `:2385`, `:10213` and `:18108`, plus the tests at `:28043-28071`: 1 day as D to D+1 on the F&O path.

  Also:
  - `crates/cli/src/results.rs` (v4 record) and `crates/cli/src/trades.rs` (partial-fill rows);
  - `crates/api/src/backtest.rs` and `crates/api/src/sweeprun.rs`;
  - `web/src/routes/backtest/+page.svelte`, `web/src/lib/sweep.js` and `web/src/lib/charge-scope.js`.
- **Tests:** the old index path is unchanged; the F&O path's prices equal the new path's on the same trade; v3 rows still read.
- **Depends on:** F2 landed.
- **Size:** 8-12 h, L.

### FNO-BAND-FILES (F3, follow-on)
- **Goal:** read NSE's daily security file (PricRg and BidIntrvl, from 2024-02-05; T06-10, T07) to check the computed band and the measured price step.
- **Files:** N `crates/pull/src/nse_security.rs`; E `crates/pull/src/lib.rs` (after LEVERAGE's line, on a later family).
- **Tests:** parsing; the band agrees with or differs from the computed band, counted per stock-day.
- **Depends on:** F2 landed. The download needs the operator's yes.
- **Size:** 2-4 h, S.

---

## 6. The 13 Oct condition kit: what the first run takes and what follows

| KIT block (positions) | First run | Where | What follows in the kit |
|---|---|---|---|
| K16 Cross-section (280) | the %-move ranks, entered, ordinal, minutes in top N, best rank, staleness, pool size, daily ranks | FNO-RANK | the other yardsticks (move from open, volume in play, ATR units, excess), breadth, dispersion, peers and sectors, anchored VWAP from top-10 entry, multi-day rank flags |
| K07 Ratio ladder (528) | every anchor A1-A9, rules 2 and 3 with both variants, extensions in the move's direction, the continuous position, relations, P1-P3 and P5-P7 | FNO-LADDER (+ FNO-STRAT) | the day-low leg, ORB extensions, measured moves as vocabulary positions |
| K06 Pause machine (401) | push, box, holding rules, break, failed break, ordinal, staircase, partial U, pause/push volume | FNO-PAUSE | the tighter second pause, the 20-pause habit ring, quiet/fake/real labels as positions |
| K12 Slot norms (54) | the usual move, volume and range per time slot, causal | FNO-STRAT `norms.rs` | the full block |
| K19 Tape quality (27) | the measured raw price step | FNO-STRAT | the rest |
| X1 / X2 / X3 / X4 | condition, level, time exits; trade rules | FNO-EXITS, FNO-SEC-EXEC, FNO-PORTFOLIO | ExitPolicyV2 as a kit seam |
| K18 Seconds summary (29), order flow delta (OF-2) | refused by name (SPEC 7.4) | - | later wave |
| K01-K05, K08-K11, K13-K15, K17 | not used; the 370 conditions stand in as filters | FNO-FILTER | the full kit |

- **Sized honestly:** the kit's three strategy blocks are about 1,209 positions (K06 401 + K07 528 + K16 280; KIT `mask.positions_by_block`). They need the mask widened to vocabulary v4 through KIT-00 -> KIT-01 -> KIT-02/KIT-03 -> KIT-04 -> KIT-116. That is five serial waves of mostly L packages. It cannot meet Thu 12, which is why the first run takes the typed route.
- **After the first run:** the kit's KIT-106, KIT-107 and KIT-116 become adapters that read `fno_pause`, `fno_ladder` and `fno_cross_section` state and emit vocabulary positions. They do not re-implement them, so the live path and the kit share one implementation.

---

## 7. Waves and merge families

| Order | Part | Crate(s) | Wave | Family | First run | Depends on (unbuilt today) | D-block | Size (h, ESTIMATE) |
|---|---|---|---|---|---|---|---|---|
| 0 | FNO-SEAM | runner, indicators, cli, docs | 0 | F1 | yes | - | - | 0.25-0.5 |
| 1 | FNO-LAW | docs, core tests | 1 | F1 | yes | SEAM | D-8100 | 2-4 |
| 2 | FNO-COSTS | costs | 1 | F1 | yes | SEAM | D-8110 | 6-10 |
| 3 | FNO-SEC-EXEC | runner | 1 | F1 | yes | SEAM | D-8120 | 8-12 |
| 4 | FNO-LADDER | indicators | 1 | F1 | yes | SEAM | D-8130 | 6-10 |
| 5 | FNO-DAYMASK | cli | 1 | F1 | yes | SEAM | D-8140 | 4-6 |
| 6 | FNO-LEVERAGE | pull | 1 | F1 | yes | SEAM | D-8150 | 3-5 |
| 7 | FNO-RANK | runner | 2 | F1 | yes | SEAM | D-8160 | 4-6 |
| 8 | FNO-PAUSE | indicators | 2 | F1 | yes | SEAM | D-8170 | 6-9 |
| 9 | FNO-PORTFOLIO | runner | 2 | F1 | yes | SEAM | D-8180 | 4-6 |
| 10 | FNO-WFT | runner | 2 | F1 | yes | SEAM | D-8190 | 6-9 |
| 11 | FNO-DAILY | runner | 2 | F1 | yes | SEAM | D-8200 | 2-3 |
| 12 | FNO-BAND | runner | 2 | F1 | yes | SEAM | D-8210 | 2-3 |
| 13 | FNO-CLOCK | cli | 2 | F1 | yes | SEAM | D-8220 | 4-6 |
| 14 | FNO-NET | runner | 2 | F1 | yes | COSTS | D-8230 | 3-5 |
| 15 | FNO-EXITS | runner | 2 | F1 | yes | SEC-EXEC | D-8240 | 4-6 |
| 16 | FNO-SEC-WIRE | cli, runner identity | 2 | F1 | yes | SEC-EXEC | D-8250 | 3-5 |
| 17 | FNO-STRAT | runner | 3 | F2 | yes | LADDER, PAUSE, RANK, DAILY, WFT | D-8260 | 8-12 |
| 18 | FNO-RECORDS | cli | 3 | F2 | yes | F1 types | D-8270 | 4-6 |
| 19 | FNO-FILTER | runner | 3 | F2 | yes | STRAT | D-8280 | 3-5 |
| 20 | FNO-API | api, web | 3 | F2 | yes | RECORDS | D-8290 | 4-6 |
| 21 | FNO-RUN | cli | 3 | F2 | yes | all of F1, STRAT, FILTER, RECORDS | D-8300 | 6-9 |
| 22 | FNO-LEAN | cli | F3 | F3 | no | F2 landed | D-8310 | 2-4 |
| 23 | FNO-SWEEP-WIRE | cli, api, web | F3 | F3 | no | F2 landed | D-8320 | 8-12 |
| 24 | FNO-BAND-FILES | pull | F3 | F3 | no | F2 landed | D-8330 | 2-4 |

Sums (ESTIMATE, writing once): F1 + F2 about 93-144 agent-hours; F3 about 12-20.

**WAVE 1 (build now, 6 parts):** FNO-LAW, FNO-COSTS, FNO-SEC-EXEC, FNO-LADDER, FNO-DAYMASK, FNO-LEVERAGE.
- Each depends only on FNO-SEAM, which is built first and takes minutes.
- No two of them share a code, test, bench or manifest file: they sit in docs/core tests, costs, runner, indicators, cli and pull.
- The four ledger documents are each edited at a different slot line (section 4.1).

**Wave 2 (10 parts, two rounds of 6 and 4).** The seven parts that depend only on SEAM (RANK, PAUSE, PORTFOLIO, WFT, DAILY, BAND, CLOCK) may start as soon as a builder frees up, even before wave 1 passes. NET, EXITS and SEC-WIRE start when COSTS or SEC-EXEC passes.

**Families, in landing order (each one merge into `feat/db-fix`, after BASE-REDS-1023):**
1. **F1 "Foundations"** = FNO-SEAM plus parts 1-16. It is assembled on `fam/fib-1` from `fix/fib-seam`. It holds every crate change below the strategy, and the law.
2. **F2 "Strategy and run"** = parts 17-21, on `fam/fib-2` cut from `fam/fib-1`. With F2 merged, the first run starts.
3. **F3 "Old path and follow-ons"** = parts 22-24, on `fam/fib-3` cut from `fam/fib-2`. It lands while the first run is going.

Each family is cut from the family before it, so it merges onto that family with no conflict by construction. Its one expected conflict with other in-flight work is the ledger end-of-file sections (FNO-SEAM's appended section against BASE-REDS-1023's appends). The main session resolves that once, at F1's landing, by its usual "merge `feat/db-fix`" step.

---

## 8. Schedule to Thu 12 Nov (one account a day, none on Wednesdays)

### 8.1 Day by day

| Day | Builds and checks (fib session) | Landing and data (main session) | Checkpoint |
|---|---|---|---|
| Fri 30 night | This design. FNO-SEAM committed. Wave 1 launched if the 5-hour window allows (the data run has priority, PROGRESS 22:3x) | data fix running | SEAM exists; wave 1 started or queued |
| Sat 31 | Wave 1 built, round-1 checks, repairs begin. Wave 2a (RANK, PAUSE, PORTFOLIO, WFT, DAILY, BAND) launched as builders free | decisions 9.1-9.7 asked; C_VAR1 download if yes | 6 wave-1 branches checked once |
| Sun 1 | Wave 1 repairs, target pass. Wave 2a checks. Wave 2b (NET, EXITS, SEC-WIRE, CLOCK) from passed tips | F&O data proven (target evening); shadow store cloned from it | wave 1 passed |
| Mon 2 | Wave 2 repairs, target pass. `fam/fib-1` assembled. Wave 3 starts: STRAT, RECORDS, API | BASE-REDS-1023 lands (likely Sun 1 - Tue 3, MVP section 4) | F1 complete |
| Tue 3 | F1 landing. Wave 3 round-1 checks. FILTER from STRAT; RUN from STRAT + FILTER + RECORDS | F1 merges into `feat/db-fix` (one 4-5 h slot) | F1 merged |
| Wed 4 | no account | - | - |
| Thu 5 | Wave 3 repairs. RUN round 2. A small end-to-end check of RUN on a shadow sample (3-5 stocks x 2 months; 2 Nov 2026: at least 3 months, as two cannot hold two forward windows, READINESS ADDENDUM item 6) as part of RUN's check, not the backtest | - | STRAT, RECORDS, API passed |
| Fri 6 | Wave 3 target pass; `fam/fib-2` assembled | F2 landing (4-5 h) | F2 merged (likely Fri 6 - Sat 7) |
| Sat 7 | Deploy the merged build on a shadow; `fno margin build`, `fno clock build` (hours); `fno run --sample`, then the projection; **the full run starts (LIKELY Sat 7 - Sun 8)**. F3 builds start in parallel | - | run started |
| Sun 8 - Tue 10 | Buffer: a third repair round on any wave-3 part, or a landing redo. F3 checks | F3 landing when passed | - |
| Thu 12 | **Latest start (WORST)** | - | run running |

### 8.2 Does it fit?

- **Thu 5 - Fri 6 Nov does not fit.** The PROGRESS fast-path estimate (start Thu 5 - Fri 6) covered the 8 strategy-mvp parts. The operator's own blocks (rank, ladders, pause, walk-forward, live-like fills, records) add 14 parts and a third serial build wave (STRAT, then RUN).
- **Likely start: Sat 7 - Mon 9 Nov.**
- **Thu 12 fits, with no slack**, only if all of these hold:
  - (a) every part passes within three check rounds;
  - (b) BASE-REDS-1023 lands by Tue 3;
  - (c) the data is proven by Sun 1;
  - (d) each account-day supports about 25-35 agent-hours of builds and repairs. This is an ESTIMATE: writing F1 + F2 once is about 93-144 agent-hours; repairs and checks roughly double that, to about 190-290 agent-hours over the 8 account-days from Sat 31 to Sat 7. MVP measured 18-34 agent-hours for 8 smaller parts as "1 to 1.5 account-days".
- If each account-day supports only about 20 agent-hours (the MVP rate), the work needs about 10-15 account-days, and **the start moves past Thu 12 (about Fri 13 - Tue 17 Nov)**.

### 8.3 Levers to protect Thu 12 (each needs the operator's yes; the bar is never lowered)

1. **Move FNO-FILTER and the FNO-API page into F3**, keeping the JSON routes in F2. The first run then runs with no vocabulary filters, labelled "price-action blocks only, filters follow", and adds them later. The cache and records are incremental, so only the filter dimension is walked again. Saves about 6-9 agent-hours and one serial step before RUN.
2. **Split F2's landing:** STRAT, RECORDS and API first, then RUN. This shortens the last landing's risk, at the cost of a fourth landing.
3. **Six builders at all times, and the checker of each part started the moment its builder ends.** This was already chosen by the operator ("Yes, use all three").
4. **FNO-CLOCK's raw-tick back-step.** If `pull::gdfl_tickstore` does not expose order, take the offset-only floor in the first run, labelled, and add the back-step later. This avoids a tick-store reader on the critical path.

---

## 9. Decisions needed

1. **Main session:** enter D-8100..D-8339 in `state/decision-blocks.json` for the FNO parts, or name the range to shift to. Register FNO-SEAM's branch.
2. **Main session or operator:** branch bases. Dependent parts are cut from `fix/fib-seam`, from a dependency's passed tip, or from the family branch `fam/fib-k`, not straight from `feat/db-fix` (section 4.3). The family branches are only merge vehicles, and the backtest still runs only from merged `feat/db-fix`.
3. **Operator:** a yes to download NSE's public daily margin files, `C_VAR1_DDMMYYYY_1.DAT` for every session from 3 Sep 2018 to today plus today's (about 2,010 small files, from nsearchives.nseindia.com; T02-02), into a shadow reference folder. Without them the leverage clock of rule 6 and rule 7 is UNVERIFIED and every result says so. Also, for F3 only: the NSE security files from 2024-02-05.
4. **Operator:** a source for trade-for-trade (BE/BZ) stock-days (M8). For example NSE's daily bhavcopy SERIES column, which would need a download yes. Until then FNO-DAYMASK runs M8 from an empty table and every result says "T2T unchecked". (2 Nov 2026: the table is built from NSE's bhavcopies; :968 gives it to the run.)
5. **Operator:** the client's state for stamp duty from 3 Sep 2018 to 30 Jun 2020 (COSTS 8 item 1). Until named, the dated clock uses the worst listed state, which adds about Rs 85 a trade.
6. **Recorded, no action unless the operator disagrees:** the quantity tie at exactly n + 0.5 takes the lower quantity (SPEC OP-14), and COSTS's vectors price both readings.
7. **Main session:** COMMON-1 changes `costs` and `indicators` for tickvault. Please order it after F2, or have it rebase onto F1 and F2. Otherwise FNO-COSTS, FNO-LADDER and FNO-PAUSE meet a refactor of the same files.
8. **Main session:** D3 Route B builds and runs tests only; bench targets are `test = false`. Please confirm how a part's new `benches/ratio/<x>.rs` wrapper file is treated. This design keeps every measuring function in `src/`, where tests cover it, so only the thin wrapper is at issue.

---

## 10. THE BAR's extreme cases: which part proves each

| Case | Unit test owner | Integration (FNO-RUN) |
|---|---|---|
| no-trade seconds | SEC-EXEC (zero volume, no bar), CLOCK, RANK (stale) | yes |
| gaps (no-trade minutes, opening gap) | SEC-EXEC, LADDER (A8), BAND | yes |
| halts (frozen price, zero volume; 13 and 23 Mar 2020 flagged) | SEC-EXEC, DAYMASK | yes |
| circuit limits | BAND, SEC-EXEC rule 4 | yes |
| partial and Muhurat days | DAYMASK (M9), DAILY (2018-11-07 1day), SEC-EXEC session end | yes |
| 2019-05-30 partial GDFL day | DAYMASK (data start), SEC-EXEC (refused before 11:57:47), CLOCK | yes |
| auction-era 15:15-15:29 and the dropped auction print | DAYMASK (per-stock session end, 44 rows), SEC-EXEC | yes |
| BE/T2T days | DAYMASK (M8) | yes (labelled until decision 9.4) |
| renames (ICICIPRULI/ICICILIFE, ZOMATO/ETERNAL) | DAYMASK (join, both store states), LEVERAGE (dated symbol), RANK (pool identity) | yes |
| corporate actions (split, bonus, rights, demerger) | STRAT (R1/R2 inside one session; R0 refused), BAND (ρ), DAYMASK (demerger ex-days) | yes |
| market-wide Kite-hole minutes | DAYMASK (hole set), LADDER (tainted), RANK (stale), STRAT (no decision) | yes |
| leverage regime changes (1 Jun 2020 method, column 9, day vs today) | LEVERAGE, NET | yes |
| charge regime changes (every boundary CN02-CN21) | COSTS | yes (CN vectors through NET) |
| quantity rounding ties (n + 0.5) | NET (lower), COSTS (CN22/CN23 both) | yes |
| 20-position and 100-trade caps hit exactly | PORTFOLIO | yes |
| exposure cap (Rs 1 crore exactly, then +1 paisa) | PORTFOLIO | yes |
| session end (15:08 rejection, 15:10 square-off and fee, remainder, unclosable) | SEC-EXEC, EXITS, PORTFOLIO (order-rate queue) | yes |
| out of box: price > Rs 5 lakh (q = 0); LTP <= Rs 50 (Rs 1.5 protection); band at the open; zero traded seconds all day (M10); an all-stale minute; a locked month; a repaired month; clock back-steps; `i128` overflow guards; membership unknown; a span shorter than two forward windows; PF with zero losses; all entries cancelled | NET, SEC-EXEC, BAND, DAYMASK, RANK, SEC-WIRE, CLOCK, WFT | yes |
| rule 2 (ladder never judges its own candles), every anchor and timeframe, frozen variant, new-extreme candle, engulfing candle | LADDER, STRAT (runtime assertion), PAUSE (box never includes the push extreme) | yes |
| rule 3 (same-candle extremes), engine reading and both labelled variants | LADDER, STRAT | yes |
| rule 4 (walk-forward always) | WFT | yes |
| rule 6 (live-like fills) | SEC-EXEC, EXITS, BAND, CLOCK, LEVERAGE, PORTFOLIO | yes |
| rule 7 (two clocks, every open reading, square-off fee) | COSTS, NET | yes |
| the 21:19 decision (218, PIT, label, ICICI join) | DAYMASK, RECORDS (label in every header), API (label on every route and the page) | yes |

---

## 11. Benches (p50 and p99 printed at 1x, 10x and 100x; the gate is the flat ratio plus an absolute budget; every target is an ESTIMATE until measured)

| Id | Step | p99 target |
|---|---|---|
| C-FNO-COSTS-01 / 02 / 03 | dated rate lookup / two-clock pricing per fill / worst over every reading per trade | 200 ns / 2 µs / 30 µs |
| C-FNO-EXEC-01 / 02 | one second, 20 positions + 20 pending, 6 lanes / remainder and protection checks | 1.5 µs / 100 ns |
| C-FNO-LADDER-01 | one candle, 16 ladders x 14 rungs | 2 µs |
| C-FNO-MASK-01 | one stock-day verdict | 3 µs |
| C-FNO-LEV-01 | one margin lookup | 1 µs |
| C-FNO-RANK-01 | one minute, 256 slots (fixed work, p99/p50 <= 1.2) | 15 µs |
| C-FNO-PAUSE-01 | one candle, 32 machines | 2 µs |
| C-FNO-NET-01 | charges per fill | 2 µs |
| C-FNO-PORT-01 | admit or release | 300 ns |
| C-FNO-EXITS-01 | one second per lane | 300 ns |
| C-FNO-BAND-01 | one band check | 100 ns |
| C-FNO-CLOCK-01 | one minute boundary (batch, linear per day, bounded) | 1 µs |
| C-FNO-WFT-01 / 02 | additive window total / quantile ladder (batch) | 200 ns / bounded |
| C-FNO-DAILY-01 | next-session start | 100 ns |
| C-FNO-SECWIRE-01 | warm second fetch (cold first touch reported as device latency) | 3 µs |
| C-FNO-STRAT-01 / 02 | one candle over 2,048 clauses / posting-list element | 5 µs / 20 ns |
| C-FNO-FILTER-01 | per-signal mask gather | 50 ns |
| C-FNO-REC-01 | record append / read by index | 5 µs / 2 µs |
| C-FNO-RUN-01 | end to end per stock-minute (synthetic) | flat |
| C-FNO-API-01 | one page seek | O(1) seek |

**Not O(1), bounded and written in `docs/06-limits.md` by the owning part:**
- the clock calibration per stock-day (FNO-CLOCK);
- walk-forward non-additive folds over at most 98 months (FNO-WFT);
- quantile generation per learning window (FNO-WFT);
- a fill that spans many seconds (FNO-SEC-EXEC): one O(1) step per traded second. ENGINE 3.3 measured p99 spans of 9-503 traded seconds at 100% of volume, and up to 1,305 at 10%;
- the search's frontier size (FNO-STRAT), measured by the sample run.

---

## 12. Before the first full run (operations, not parts)

1. The data proven by the main session (target Sun 1 evening; the audit verdict on 30 Oct was NOT COMPLETE).
2. A fresh APFS shadow of the proven store under `work-20260925/shadow/` (`state/fix/shadow.sh` style). Never `runtime/`.
3. The membership built into the shadow with the existing verb (`cli fno-membership build`, about 13.7 s, MVP).
4. The C_VAR1 files downloaded (decision 9.3); then `cli fno margin build`.
5. `cli fno clock build` over the span (hours, ESTIMATE).
6. The normalised skip table, the universe file and the join file under `state/fib-strategy-20261030/inputs/`, each with its digest.
7. `cli fno run --sample` (10 stocks x 12 months), then the projection of time and disk, reported to the operator before the full run. (2 Nov 2026: which stocks and months, the pool and the sample's floor: this file's end, READINESS ADDENDUM item 6.)
8. **Disk:** WD_BLACK is below the operator's 2 TB floor (PROGRESS 23:0x). The run's records are lean, but the projection must show they fit before the full run starts.

---

## 13. Not proven here

- Every size, hour and p99 target above is an ESTIMATE. Nothing was built or measured in this step, except the git-merge behaviour (scratch repo) and the code lines cited.
- Whether `pull::gdfl_tickstore` exposes raw tick order (FNO-CLOCK's backward clock step).
- How long the full first run takes. FNO-RUN's sample measures it, and the search's frontier size is the unknown.
- Whether every part passes within three rounds. The past rate is 22 of 57 merged parts passing their first check (MVP section 4).
- That COMMON-1 will not land between now and F2.
- The GDFL limits stand on every report: trades, not the order book; 98.2% of seconds are single-price (SPEC 19).

## OPERATOR DECISIONS, 30 Oct 2026 ~23:4x IST (binding; added by the fib session after this design)
- 9.3 and 9.4: yes, download the NSE C_VAR1 margin files and the daily bhavcopy (trade-for-trade days) into a reference folder, not the live store.
- 9.5: stamp duty before 1 Jul 2020 uses the worst listed state.
- 8.3 lever 1: yes. FNO-FILTER and the FNO-API page move to F3; the JSON routes stay in F2; the first run is labelled "price-action blocks only, filters follow".
- Disk: the operator chose to free space (2 TB floor); builds wait until free space is above 2 TB plus 50 GiB headroom.

## OPERATOR DECISION, 31 Oct 2026 ~08:4x IST
- 0.31 (0.312) and 0.65 were typed by mistake: they are not ladder rungs. FNO-LADDER uses 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1 and the extensions only.
- The hunt order is unchanged: top movers first, then Fibonacci on those stocks.

## OPERATOR DECISION, 31 Oct 2026 ~12:4x IST: the closest recipes when nothing passes
- If no recipe passes the Winner Gate on walk-forward, the run still reports the CLOSEST recipes. They are labelled "did not pass: closest to the requirement" and never called winners.
- Ranking (fixed in advance, never tuned on results): first the number of Winner Gate checks passed (out of 28); then the total relative shortfall on the failed checks; then the walk-forward net after charges.
- Each one shows which checks failed and by how much, both scores (net, and signal only), every walk-forward window, and its trade count. A minimum trade count applies (the 25 Oct check showed 1-2-trade "passes" are luck), and the luck accounting counts every recipe tried.
- The closest recipes become the next ideas to test. None is traded with real money until it passes.
- Owner: FNO-RECORDS and FNO-RUN (F2); the next design round sets the exact report. (2 Nov 2026: no such round exists; FNO-RUN's builder designs it: this file's end, READINESS ADDENDUM item 9.)

## OPERATOR NOTE, 31 Oct 2026 ~12:5x IST: any pattern, any level
- The winning combo can be anything: known patterns (the 62 candle patterns, rest shapes, Elliott waves) or unknown ones, which the search finds as mixes of measured features. It can occur at any level or at none: the level is a measured feature, never a precondition (16 Oct rule). The goal stays the common combo that meets the requirement.
- New idea for the queue: an Elliott-wave block built from objective swings (zigzag legs on a swept swing size, wave counts and wave ratios measured against the Fibonacci rungs). It is not in the first run; it is added after it, like the 13 Oct kit.

## OPERATOR CORRECTION, 31 Oct 2026 ~13:0x IST (supersedes the Elliott-wave idea just above)
- "don't focus on some known names ... it can be anything ... candle color, body, wick, touches, above, near, below, crosses, retraces, fallback, pullback, closer, U or V or W ... one and only pure completely common dynamic scalable incremental approach".
- So there are NO named-pattern blocks (no Elliott-wave block, no special case per pattern name). There is ONE common primitive path:
  - every candle on every timeframe is measured by the same generic dials: colour, body, upper and lower wick, range, close location, against the previous candle (higher, lower, equal), and runs of N;
  - every level (every rung of every ladder, and the day, period and opening levels) is measured by the same relation dials: above, near, below, touch, cross, retrace depth, pullback and fall-back size, and distance in R and in percent;
  - shapes such as U, V and W are only measured turn sequences over N candles, not names.
- Every dial value is swept or generated from the data. Mixes of any size are found by the incremental search, so a "pattern" is simply a combo the data finds.
- The goal: the common combo that meets the requirement. The FNO-STRAT and FNO-PAUSE designs follow this one path.

## OPERATOR DECISION, 31 Oct 2026 ~14:5x IST: STEP 2 = context filters, after the first run (binding)
Asked by the main session (560b525a) whether the top-movers strategy needs index values, INDIA VIX and each mover's index membership, the operator answered: "ok add this as the second step dude so that we can have precise predictions of probabilities".
- The FIRST run stays price action only (decision of 30 Oct ~23:4x IST; NIFTY regimes stay in the walk-forward acceptance, SPEC line 109; INDIA VIX stays a stamp on every trade).
- STEP 2, right after the first run, adds context filters, each tested on the data like every other dial (nothing static, walk-forward, luck accounting): NIFTY and BANKNIFTY state at decision time; INDIA VIX level and change; the mover's sector index move; the mover's move relative to its sector index and to NIFTY; dated index membership (NIFTY 50/100/200/500, sector indices).
- Step 2 needs: (a) a law change, approved by these words: INDIA VIX may enter the condition vocabulary as a tested filter (only the value known at the decision second; never ranking, never run identity), with a new decision entry citing this approval; (b) a NEW DATA PART: dated (point-in-time) sector/industry classification and dated index constituents from NSE files since 2018 (today's lists in core::universe are never used on old dates); (c) the index data proven to the 218-stock standard (heart-piece audit v2 task 'indices', from 31 Oct).
- Results are reported per context bucket with trade counts, so every probability shown names how many trades it rests on.

## OPERATOR DECISION, 31 Oct 2026 ~17:4x IST: N4, the GDFL quote range in worst-case fills (binding for FNO-SEC-EXEC, FNO-EXITS and every part that reads a second's price)
The heart-piece audit v2 found that NSE's day highs and lows often sit on GDFL rows with LTQ 0 that D-2802 drops (the stored 1s high equals NSE's on 17,272 of 32,904 stock-days measured). The operator approved the recommendation ("Fix resolve implement everything"): the 1s bars stay trades only (LTQ > 0), and the LTQ-0 rows whose price moved become a separate, labelled per-second QUOTE RANGE (data part QRANGE-1, never a trade, never volume). Rule for this strategy: adverse triggers (stops and every exit that fires against the position) and fill prices use the WORSE of the second's traded range and its quote range; volume caps still use traded volume only. Until QRANGE-1 lands, the quote range is absent and the walk records "quote range not loaded" on every trade; build the walk to take an optional per-second quote range now, with tests for present and absent.

**MAIN-SESSION DECISION, 1 Nov 2026 ~00:3x IST (operator aim: backtest ready Wed 11 Nov even in the worst case; "whatever is needed go ahead").** Round 3 (`state/fib-strategy-20261030/workflow-fib-3.js`, run wf_fe1198f2-624) builds F1 and F2 in one run: each F2 part starts the moment the parts whose code it calls have passed, cut from their passed tips (one merged `base/<part>` from exactly those tips when it needs several), instead of waiting for `fam/fib-1` to be assembled (this replaces "Cut from fam/fib-1" at :900 and :956 and "cut from the F2 family branch" at :921 and :989 for the build only). FNO-FILTER is in F3 (:1267), so FNO-RUN does not wait for it. At most `maxActive` fib agents run at once, parts with dependents first. `fam/fib-1` and `fam/fib-2` are still the merge vehicles: the main session assembles each from the passed tips at landing time with its family landing tool (`state/landing-20261101/`, being built and checked), and F2's coverage is proved again on the assembled family before it lands.

## ADDENDUM 1 Nov 2026: cross-family anchors with the data build (conflicts.md section 6; main session 8957d0bd, levers task A)

Binding over sections 4.1-4.3 and the Files lines of FNO-RUN, FNO-LEVERAGE and FNO-API. Backup: `PARTS.md.bak-20261101-levers`. The data build now gives every data part its own anchor (`state/data-build-20261031/PARTS.md`, "MAIN-SESSION DECISION 1 Nov 2026: ONE ANCHOR PER PART"). The anchors below are the fib parts' own. Each sits at least 2 unchanged base lines (`bcac15e2c` = `f01d7a0e4` for these files) from every data part's and BASE-REDS-1023's edits. They were validated with the data anchors in `state/post-landing-20261101/anchors-validation.md` section 4. F1 and F2 land after the data family, and under D1 a code conflict there aborts the landing.

**FNO-RUN (F2), `crates/cli/src/lib.rs`:**
- `pub mod fno_run;` directly under FNO-SEAM's `// slot FNO-RUN (fno_run)` (rule unchanged).
- `"fno",` in `COMMANDS` right after `"feed-reconcile",` (:3191). The list must stay sorted, because `every_command_is_listed_in_both_places` asserts `COMMANDS.windows(2)`. The data verbs sit after :3196 (AUCRS-1), :3229 (REFOLD) and :3230 (REPLACE).
- Dispatch arms right after the `["fno-membership", ..]` arm (:3090-3092). The data arms are after :3071 (REFOLD), :3074 (REPLACE) and :3098 (AUCRS-1).
- Usage lines right after the fno-membership usage block (:540-559, before `cli fold-audit`). The data usage lines are after :676 (AUCRS-1), :778 (REFOLD) and :790 (REPLACE).
- **The `COMMANDS` length line (:3167) has no shared conflict-free text.**
  - The data family writes 65 (REFOLD), then 66 (REPLACE and AUCRS-1), and its seal sets the joined count.
  - The slice type `&[&str]` is not available: `crates/cli/src/store_write.rs:309-317` and `crates/cli/src/sweep_wiring_tests.rs:147-160` need `&str` items.
  - **Rule:** FNO-RUN's tree must contain the LANDED data family before its passing check. Either `base/FNO-RUN` (the F2 family branch) has the post-data-landing `feat/db-fix` as an ancestor, or FNO-RUN merges that `feat/db-fix` as its next step before its passing check. Its count is then the landed count plus its own verbs.
  - Synthetic check: FNO-RUN written on a pre-data tree conflicts with REPLACE and AUCRS-1 on that one line (1 hunk each). Against REFOLD it silently matches 65, which is a wrong count. On the post-data tree the line is clean by construction.
  - Any other fib part that adds a verb follows the same rule.

**FNO-LEVERAGE (F1):**
- `crates/pull/src/lib.rs`: `pub mod nse_margin;` right after `pub mod nse;` (:236, sorted). AUCRS-1 adds its line after :195.
- `crates/pull/benches/ratio.rs`: `#[path = "ratio/nse_margin.rs"] mod nse_margin;`, with its doc and a blank line, goes right after the blank line :53 that follows `use std::time::Instant;` (before C-TSW-01's doc).
- Its `println!` of C-FNO-LEV-01 and its `ok &=` line go right after `ok &= day_table::day_lookup_is_flat();` (:1857).
- Never "after the last mod" or "before `if ok {`". Those places now hold:
  - REFOLD :91 / :1888;
  - CAL-2027 :79 / :1880;
  - REPLACE :70 / :1870;
  - CAL-1126 :64 / :1864;
  - REPLACE-2 :59 / :1860;
  - BASE-REDS-1023 after :88.
- FNO-BAND-FILES (F3) still inserts after LEVERAGE's lines, on a later family.

**FNO-API (F2):**
- `crates/api/src/lib.rs`: `pub mod fnojson;` at its sorted place, right after `pub mod fault_log;` (:75) and before `pub mod folder;`. PCT-1 adds after :148.
- `crates/api/src/server.rs`: its `.route("/fno/...")` lines right after `.route("/indexmap.json", axum::routing::get(indexmap_json))` (:16717). The other edits in this file:
  - REPLACE's `/pull/replace` goes after :16851;
  - BASE-REDS-1023's hunks are at :17252-17463;
  - PCT-1's are at :1690-1822;
  - CAL-2027 edits no router line.
- `.github/gates_routes.rs`, only if its routes need a `BENCHES_OWED` entry: ONE new `Owed { owner: "FNO-API", .. }` right after the closing `},` of "no package (the engine's read surface)" (:187). The other edits in this file:
  - REPLACE adds after :128;
  - BASE-REDS-1023 adds after :126 and :205;
  - QRANGE-1 adds a new entry after :153.
- `web/src/lib/speed.js`: its one page-budget row. No built data tip edits `speed.js` (checked 10:5x IST).

**Every fib part:**
- The slot rule of 4.1-4.2 is unchanged.
- A fib part that must edit a file that a data part or BASE-REDS-1023 edits, and that is not named here or under its slot lines, stops and asks the main session for an anchor.
- Only FNO-RUN's count line needs the post-data merge. Every other anchor here merges clean without it. **(2 Nov 2026: wrong by two lines. FNO-RUN has three count lines on the post-data tree, and the word `fno` is classified in `store_write.rs` and `sweep_wiring_tests.rs`: this file's end, READINESS ADDENDUM item 1. FNO-LEVERAGE's `gates_tree.rs` anchors are its item 12.)**

## MAIN-SESSION DECISIONS, 1 Nov 2026 ~12:0x IST (binding; found by the independent accuracy check of the blueprint page, state/blueprint-20261101/check-r1 in the workflow journal wf_86b67449-203; they keep the operator's bar: nothing that could pass may be skipped, and the Winner Gate is exactly the operator's words)
1. **The extinction floor is PAIR-aware (fixes 3.2's per-partition floor).** The Winner Gate counts >= 500 trades a year across the long + short PAIR (MEM/backtest-acceptance-bar.md, 16 Oct 17:11 FINAL; SPEC 1.5 and 13.3), while direction partitions the search. A floor of 500 per side would prune a 300/yr long mix whose pair with a 300/yr short mix passes. So a one-side mix is pruned only when its pro-rated yearly trade count in the learning window PLUS the largest pro-rated yearly count reached by ANY mix of the other side in the same learning window is below 500 (per side: floor = 500 minus the other side's maximum; recomputed per learning window, never typed). This stays anti-monotone, so the skipped mixes still provably cannot pass. Owners: FNO-STRAT (the search) and FNO-RUN (the driver; it must hold both sides' maxima before pruning either). Test obligation: a 300/yr long + 300/yr short pair is not pruned and passes the quantity check; a mix whose count plus the other side's maximum is 499 is pruned.
2. **The five downside checks run PER DIRECTION AND FOR THE PAIR** (operator 18 Oct 2026 ~13:07 IST, verbatim: "add all five downside checks (worst day, worst week, max drawdown, losing streaks, recovery time) to the Winner Gate, per direction and for the pair, net of all charges"; SPEC 1.5 line 107 and 13.3 dropped "per direction"). The Winner Gate is therefore 38 checks: per side 5 quality + 5 downside (x2 = 20); per pair 500 a year, every year positive, leave-one-out x5, downside x5, NIFTY regimes (13); minimum returns x5 (5). The closest-recipes ranking (decision of 31 Oct ~12:4x) counts checks passed out of 38. Owners: FNO-WFT (the per-direction and pair downside references, learned vs unseen), FNO-RECORDS and FNO-RUN (reports).
3. **Ladder anchors in the first run are A1-A9** (FNO-LADDER builds A9, PARTS :427); SPEC 7.6's "A1-A8" predates that part.
4. **Named patterns:** SPEC 14's S30 (P0-P8) and S36 (2 or 3 candles) are superseded by the 31 Oct ~13:0x correction (one common primitive path; a "pattern" is a mix of the common candle and level dials found by the search). They are not separate switches in the first run.
5. **FNO-CLOCK takes a stock-day (or key) list (lever L6 of state/post-landing-20261101/calendar-v2.md; main session 1 Nov ~15:0x IST).** The fib prep (margin, membership, clock, inputs) runs on a frozen pre-proof clone in parallel with the data run. Margin reads only NSE's C_VAR1 files and membership only the tick store, so no data-run row changes them. The clock reads GDFL seconds and Zerodha 1-minute bars, of which about 14 k stock-days (~3%) change in the data run (S-53/55, S-52, S-62, S-32, S-68/69, S-57, 7.6, S-61, S-43, S-64). So `fno clock build` must accept a list of (stock, day) keys and rebuild ONLY those, byte-identical to a full build for every key (test: a full build vs a list rebuild over the same keys give equal bytes; keys outside the list are untouched). Owner: FNO-CLOCK.

**REPAIR 1 of the 1 Nov 2026 anchors addendum (levers repair 1, ~15:xx IST; `state/post-landing-20261101/check-levers-r1.md` B1 and B4; data PARTS.md decisions C, I and J; backup `PARTS.md.bak-20261101-lc`). Binding over the FNO-RUN `COMMANDS` bullets above where they differ.**
- **The data count is written by its chain, not a seal:** REFOLD 65, REPLACE 66, AUCRS-1 67 (AUCRS-1's merge base also merges REPLACE's passed tip; data decision I). The landed data family holds 67, and FNO-RUN writes 67 plus its own verbs.
- **How FNO-RUN takes the landed data family (the Y rule, as Y0 for the data parts):** a part must never merge a moving tip and then assemble over a base that lacks it (data check-levers-r1 B1: no base the landing tool can use assembles it). So either `base/FNO-RUN` itself is made with the post-data-landing `feat/db-fix` as an ancestor (preferred), or FNO-RUN merges ONE commit `Y_RUN` = the merge of its args base `base/FNO-RUN` and that `feat/db-fix` (made once with `git merge-tree --write-tree` + `commit-tree`, pinned `base/FNO-RUN-Y`), never `feat/db-fix` itself, and the F2 family args give `--bases {"FNO-RUN": "<Y_RUN>"}` (`famland-args.mjs` fib2 already reads FNO-RUN's base from `--bases`). Its passing check covers that merge.
- **FNO-SEAM and every F1 part:** if one ever takes BASE-REDS-1023's change by a merge, it merges the data family's Y0 (`base/Y0-base-reds-1023`, data REMERGE.md section 9), never REDS's tip, and F1 lands with `--stack <Y0>` only if every F1 part cut from `bcac15e2c` merged that same Y0 (famtool-items.md T1 otherwise).

## MAIN-SESSION DECISION, 1 Nov 2026 ~17:1x IST: L12, the speculative cut, is in force for every run with args.speculative (binding; lever L12 of state/post-landing-20261101/calendar-v2.md; state/fib-strategy-20261030/L12.md; backup PARTS.md.bak-20261101-l12)
A dependent part no longer waits for its dependency's PASSED tip before it starts. It starts on the BUILT tip and it is still checked only against the passed one: the bar does not move. This overrides "cut from X's passed tip" in section 4.3 and in R2's BRANCH BASES, and "starts the moment the parts have passed" in the F2 rules and the decision of ~00:3x above, for a run launched with `args.speculative = true`. A run without that arg (the live run wf_b15cfc81-7c3 included) behaves as before; so does a part whose dependencies have all passed when its slot is granted.

**For a builder:**
- Your task names your base and says whether it is speculative, with each dependency's tip and whether it is built or passed. A dependency that has not passed may still change.
- Code to the dependency's specified interface: its section here and its JSON in parts.json, not an accident of its present code.
- Never edit a dependency's own code on your branch, even to fix a bug you find in it. Name the bug in `problems`.
- Measure coverage against the speculative base for now, and say that it is speculative in `coverage` and in part-<X>.md.
- Do not merge the dependency's later tips yourself. If your branch already exists from an earlier attempt and does not hold the base your task names, merge that base first, as a merge commit: that one merge is allowed.

**The catch-up (a new step, `catchup-<X>`, phase Catchup@xhigh):** when every dependency has passed, your FINAL base is exactly what the old flow would have cut from: one dependency's passed tip, or `base/<X>` merged from the passed tips by the merge-base step. If your branch already holds it, no agent runs. Otherwise the catch-up merges it into `fix/fib-<x>` as a merge commit carrying the Co-Authored-By line, and only that step merges.
- Conflicts in the five append-only ledgers (docs/00, 04, 05, 06, 07) go by the append rule: both sides whole, in id order, each part's rows under its own slot line.
- A conflict in code is resolved only when it is the part's own lines at the part's own anchors. Anything else: `git merge --abort`, HELD with the files, and the main session decides.
- It fixes in the part's own code whatever the merge broke, re-proves 100% line and branch coverage of the D2 set against the FINAL base by Route B and the gates, and writes a "Catch-up" section in part-<X>.md.

**For a checker:**
- You run only after every dependency has passed and the catch-up is done. A dependency's failure never reaches you: the dependent part ends HELD, with its tip and speculative base in the run's return value, never as a failed round, and the next relaunch names it in `args.built` with no rebuild.
- First prove, at the tip: the final base and each dependency's passed tip are ancestors, and `git rev-list --no-merges <final base>..<tip>` lists only this part's own commits. Any of these failing fails the check.
- Coverage and the diff are measured against the FINAL base only. A speculative base (`base/<X>-spec-<short tip>-<digest>`) is never the measure, except when it holds exactly the passed tips and the script names it as the final base.

**Unchanged:** the drain test and the disk floor in every step, the lanes, the effort ladder (builds xhigh, checks 1-2 xhigh, 3+ max, repairs max, merge bases high), `maxActive` (a part that waits holds no slot), and the merge order. The family landing takes the same args base as before (`state/post-landing-20261101/famtool-items.md` T3). FNO-RUN's Y rule above is unchanged: give its base through `args.bases`, and the catch-up merges it.

## MAIN-SESSION DECISION, 1 Nov 2026 ~17:3x IST: P0's "data proven" (binding on FNO-RUN's first build; `state/post-landing-20261101/decisions-sun1.md`; deploy runbook H4 and open item 2; lever L7 of `state/post-landing-20261101/calendar-v2.md`; backup `PARTS.md.bak-20261101-cal3`)

The FNO-RUN section says "P0 refuses when the data is not proven" and no more. It means this:

**What counts as proven.**
- **By default: the whole AUDIT-3 pass:** every data-run row 1-24 live and verified and AUDIT-3 passed with zero errors (`state/data-build-20261031/spec-AUDIT-3.md` 2.7; data-run runbook 4.21). (2 Nov 2026: the bar is that spec's ADDENDUM 2 Nov 2026: a defect in the data is not a pass, no `proof-v1.json` is written, its door runs with rehearse, attack, live and verify, and AUDIT-3 runs again on what it touched. Only list (c)'s labels stay as named rows.)
- **If and only if the operator consents to L7:** the bar part of AUDIT-3, passed after rows 1-19 and ZBASIS-1 Stage A (row 19). P8, S-50, the GET verifies and the rest of AUDIT-3 then run while the full run runs. **WITHDRAWN 2 Nov 2026 (the operator, ~16:5x IST: "Yes, all before start").** The operator gave this consent on 1 Nov and withdrew it on 2 Nov: nobody passes the argument below. (2 Nov 2026, later the same day: the argument is not built either; this file's end.)

**WITHDRAWN 2 Nov 2026 (main session c0907298; this file's end): `cli fno run` gains one flag, `--proof FILE`, and no new verb; FNO-RUN's first build does not write `--proof-scope`.** As it stood: **The first build supports both by one argument, and the strict one is the default.** `cli fno run` gains two flags and no new verb:
- `--proof FILE`: required; no file means "data not proven", refused.
- **WITHDRAWN 2 Nov 2026, not to be built, kept as the record:** `--proof-scope whole|bars`: the one argument; default `whole`. (2 Nov 2026: it stays in the first build as briefed, off and not used: no run passes it, and no `proof-v1.json` says `bars`, so its refusals and tests stand.)

**What P0 reads: FILE only** (it does not call `:8080`, which the run never touches). The main session writes it with `jq` when AUDIT-3 (or its bar part: withdrawn 2 Nov 2026) passes, as `state/fno-data-audit-<date>/proof-v1.json`; no part builds a writer:
```json
{"proof_version": 1, "scope": "whole", "verdict": "pass", "end": "<END: YYYY-MM-DD>",
 "report": "<path of REPORT.md>", "report_sha256": "<64 hex>",
 "census_dir": "<AUDIT-3's census folder>", "census_tag": "audit3-end", "census_sha256": "<64 hex over its four files and its lengths file>",
 "operator_consent": null}
```
**WITHDRAWN 2 Nov 2026, kept as the record:** `scope` is `"bars"` for the bar part, and only then `operator_consent` is `{"at_ist": "<when>", "words": "<the operator's words>"}`. (The file says `"scope": "whole"` and `"operator_consent": null`. P0 reads no consent.)

**P0's refusals, each by name, nothing run:**
- no file, unreadable, or `proof_version` other than 1;
- `verdict` other than `"pass"`;
- `--to` later than `end`;
- a file whose `scope` is anything but `"whole"`, refused by name, the refusal naming the scope it read (2 Nov 2026; was: `--proof-scope whole` (or none) with a file that says `scope: "bars"`).
- **WITHDRAWN 2 Nov 2026, not to be built, kept as the record:** `--proof-scope bars` with a file whose `operator_consent` is empty.

(**WITHDRAWN 2 Nov 2026, not to be built:** `--proof-scope bars` also accepts a `whole` file.) Only `fno run` reads the proof: `fno margin build`, `fno clock build` and `fno mask` do not, so lever L6's prep on a pre-proof clone is not refused.

**What it leaves on the run:**
- The proof's scope and the sha256 of FILE join the run identity (FNO-SEC-WIRE's identity terms), so a resume with another proof or scope refuses.
- **WITHDRAWN 2 Nov 2026, not to be built, kept as the record:** under `bars`, every output and record carries the label "data proof: bar part only; rows 20-24 still open". Under `whole` there is no such label. (No output carries it: there is only `whole`.)
- **WITHDRAWN 2 Nov 2026, kept as the record (AUDIT-3 is one pass, before the start):** a later pass of the rest of AUDIT-3 changes no record: the main session reports it beside the run. If it finds a bar defect, the run restarts from a new cut, which is the condition the operator consents to.
- R0 stays refused (FNO-STRAT's first-build test, :895). (2 Nov 2026; was: under both scopes: P8's tables are absent from a `bars` cut.)
- P0 does not compare FILE with the shadow. That tie is deploy runbook R2's census check against `census_dir`.

**Tests (added to FNO-RUN's "each refusal" list):** `p0_refuses_no_proof_file`, `p0_refuses_an_unknown_proof_version`, `p0_refuses_a_failed_verdict`, `p0_refuses_a_span_past_the_proven_end`, `p0_refuses_a_bars_proof_by_default`, `a_resume_with_another_proof_or_scope_refuses`, `the_margin_clock_and_mask_verbs_read_no_proof`. **Dropped 2 Nov 2026, not to be written, named here as the record:** `p0_refuses_bars_without_the_operators_consent`, `p0_accepts_bars_only_with_the_argument_and_consent`, `every_output_under_a_bars_proof_carries_the_label`, `a_whole_proof_carries_no_such_label`.

**One count to correct in the REPAIR 1 note above.** "The landed data family holds 67, and FNO-RUN writes 67 plus its own verbs": QDATE-1 now adds the verb `session-master-install` (`state/data-build-20261031/spec-QDATE-1.md`, ADDENDUM 1 Nov), so the landed data value is **68** once FAM-DATA-B has landed. FNO-RUN writes the landed value it finds plus its own verbs, never a typed number. The new flags above add no verb.

**MAIN-SESSION DECISION 1 Nov 2026 ~18:0x IST (post-landing-20261101/check-r2.md RB2, deploy-firstrun-runbook.md R2): FNO-RUN READS NOTHING UNDER bars/gdfl/NSE/FNO.** The first run reads no option bars (SPEC.md:86, PARTS.md:839), so the fib cut keeps the FNO folder as a link and does not clone it (about 11.3 M files, 23 min+ under the lock). FNO-RUN's brief: no read under that folder, with a test that proves it. Only R3's 14-second names cross-check reads there, and that is the runbook's step, not the run's.

**MAIN-SESSION RULE 1 Nov 2026 ~18:5x IST, every step of every part: THE WAIT RULE.** While a build, test or coverage run works, wait with one blocking call of at most 270 s (Bash timeout 270000), repeat it with no status command and no text between calls, and run no foreground command longer than 270 s. Your prompt cache lives 5 minutes: a wake after it died costs 8.5 times a wake before it dies, and those rewrites were 29% of all allowance (state/preflight-20261101/measure). It changes nothing you check. A builder does not return ready without a coverage figure for that exact tip (15 of 18 coverage failures reached their check without one).

**OPERATOR DECISIONS, Sun 1 Nov 2026 ~19:2x IST (main session 76894513); item (1) WITHDRAWN 2 Nov 2026 (the operator, ~16:5x IST: "Yes, all before start"; this file's end); items (2), (3), (4) in force:** (1) **L7 YES:** the full run starts once the bars are final (rows 1-19 and ZBASIS Stage A) and the bar part of AUDIT-3 passes; P8, S-50, the GET verifies and the rest of AUDIT-3 run in parallel; the operator knows that a bar defect found later restarts the run from a new cut. FNO-RUN launches with `--proof-scope bars` and P0 reads the bar part's proof file. (2) **L8b YES:** an 08:30 IST Kite login on every day of the data run, on top of the weekday pull logins; the main session reminds the operator the evening before. (3) **Full width on every account** (operator ~19:0x). (4) **One new account a day through Fri 20 Nov** (operator ~19:1x; never two in a day).

**WITHDRAWN 2 Nov 2026 (the operator, ~16:5x IST: "Yes, all before start").** **OPERATOR DECISION, Sun 1 Nov 2026 ~22:0x IST (main session 76894513; "Yes, move them"; written ~22:2x IST, backup `PARTS.md.bak-20261101-mv4`): BASE-BENCH-1028 AND THE DATA PARTS PCT-1, SYMCAL-1, SIDX-1 JOIN AFTER THE FULL RUN STARTS (deploy runbook R12), under the same bar.** FNOLIST-1 stays in the data family and FNO-API stays in F2. Nothing in this file's parts, anchors or tests changes but what follows.

- **"The full backtest runs only from merged `feat/db-fix`" (:236, SPEC.md:94) now reads, by the operator's word:** merged with every family before the start: BASE-REDS-1023, BASE-STACK-1028, FAM-DATA-A0, -A1, -B, FAM-FIB-1, FAM-FIB-2 (deploy runbook R0, G1). The run reads nothing the four change (data-run 3A.7).
- **FNO-RUN's rule (:1318) "the LANDED data family":** A0, A1 and B. Its COMMANDS line is the landed value it finds (68 once FAM-DATA-B has landed) plus its own verbs; the later family adds no verb.
- **The identity (:845) holds `commit`,** so a later landing changes no result of the started run: its binary is M's `tgt/rel-$S8`, and its shards read BASE. Never rebuild or remove that binary while the run lives (deploy runbook R13). A run started again from a new M has a new id and commit term.
- **FAM-FIB-1's args need `--later`** (`famland-args.mjs fib1 --later BASE-BENCH-1028,PCT-1,SYMCAL-1,SIDX-1`), or the family waits for BENCH (`state/post-landing-20261101/famtool-items.md` T6).
- **FNO-API's bench, C-FNO-API-01 (:1017, :1231).** `cargo bench -p api --bench ratio` exits 1 at `reads` until BENCH lands (`spec-PCT-1.md`:805-806; its `main` exits before the later modules), and FNO-API has no anchor in `crates/api/benches` (:1335-1350), which BENCH owns. **So FNO-API stops and asks the main session for that anchor (the rule at the end of its anchor list).** The precedent is `spec-PCT-1.md` B5: if the bench cannot print on this base, the claim is proved by a `#[test]` bench and the gate-8 figure is re-run after BENCH lands. No other fib part benches `-p api` there.
- **No fib part may read PCT-1's columns, SYMCAL-1's `not_owed`, or `.srt`:** as before (:841, :895, :1421), now also because they are not on M.

**MAIN-SESSION DECISION 1 Nov 2026 ~22:3x IST, FNO-API's bench while BASE-BENCH-1028 lands after the start (replaces 'stops and asks'):** do not stop. C-FNO-API-01 is measured by a `#[test]` bench, as spec-PCT-1.md B5 does, because `cargo bench -p api --bench ratio` cannot print past `reads` until BENCH lands. Re-run the gate-8 figure after BENCH lands (deploy runbook R13; 2 Nov 2026: R13 is withdrawn and BENCH lands right after STACK, so this is due on the first tree that holds both). FAM-FIB-2 is on the critical path and must not wait for this.

## APPENDIX, read Mon 2 Nov 2026 ~15:5x IST: what the parts offer `cli fno` and the shards today (facts only, not a decision section)

Deploy runbook section 9, open items 3 and 4, say that the shard interface and the flags of `fno margin build`, `fno clock build`, `fno mask` and `fno report` are in no section of this file. This appendix writes down what the branches really hold on 2 Nov, and what FNO-RUN's builder must decide. **It decides nothing and changes no rule above.** Everything here was read with `git show` on the shas below. A later repair can change any line, so read the branch again before leaning on a signature. Where this differs from a part's section, the section stands and the difference goes to the main session.

### What was read

| part | branch | sha read | state |
|---|---|---|---|
| FNO-SEAM | `fix/fib-fno-seam` | `1ad6e24ef8e0` | passed |
| FNO-COSTS | `fix/fib-fno-costs` | `feecbd417214` | ready build, check 1 running (`wf_a89fd63f-417`) |
| FNO-LADDER | `fix/fib-fno-ladder` | `aa7381851d97` | ready build, check owed (`relaunch-args-20261102.json`) |
| FNO-SEC-EXEC | `fix/fib-fno-sec-exec` | `489e256c9861` | build running; the branch moved during this read |
| FNO-DAILY | `fix/fib-fno-daily` | `d1cb2179b658` | not returned, not checked |
| FNO-DAYMASK | `fix/fib-fno-daymask` | `8d8080ddadba` | not returned, not checked |
| FNO-PAUSE | `fix/fib-fno-pause` | `f85301027d33` | not returned, not checked |
| FNO-RANK | `fix/fib-fno-rank` | `3a9c823d97ff` | not returned, not checked |
| FNO-WFT | `fix/fib-fno-wft` | `8e7f56efb257` | not returned, not checked |

**No branch exists for FNO-LEVERAGE or FNO-CLOCK.** `crates/pull/src/nse_margin.rs` and `crates/cli/src/fno_clock.rs` are on no `fix/fib-*` ref, and `part-FNO-LEVERAGE.md` is still round 1's "NOT STARTED" note. For those two parts this appendix can only quote the design. FNO-LAW, FNO-PORTFOLIO, FNO-BAND, FNO-NET, FNO-EXITS, FNO-SEC-WIRE, FNO-STRAT, FNO-RECORDS, FNO-API and FNO-RUN have no branch either.

### The verbs

**No `fno` verb exists anywhere.** `"fno"` does not occur in `crates/cli/src/lib.rs` on any of the nine branches; the only such dispatch word there is `fno-membership`. FNO-RUN is the sole owner of the `fno` verbs (:964). The words "shard", "cli fno", "fno mask", "fno run", "fno report", "fno margin" and "fno clock" occur in no line any branch adds. So every flag below is still open, and the runbook's names are assumptions.

| verb | what exists today | what this file says | what the runbook assumes |
|---|---|---|---|
| `fno margin build` | Nothing. | `build_from_dir(src, out) -> Report`, writes `<out>/nse-margin-v1/<YYYY-MM>.nsm` and a symbol dictionary (:523). No proof read (:1415). | R5: `--src DIR --out DIR`, with `--out` the parent of `nse-margin-v1`. |
| `fno clock build` | Nothing. | `build(range) -> table <shadow>/reference/fno-clock-v1/`, with a digest (:746). It must also take a list of (stock, day) keys and rebuild only those (:1358). No proof read (:1415). | R6: `--from --to`, and `BRUTEX_GDFL_TICKSTORE` in the environment. No key-list flag. |
| `fno mask` | The library, on `fix/fib-fno-daymask`, listed below. No verb and no printing. | "the counts report" (:971). No proof read (:1415). | R7: `<shadow> --from --to --universe --join --skip --t2t --membership`, output on stdout. `--t2t` is the brief's name (:971), not yet a built flag. |
| `fno report` | Nothing on the verb. `runner::fno_wft::report(recipes, min_trades, luck) -> Report` is pure, on `fix/fib-fno-wft`. | The name only (:972). "the next design round sets the exact report" (:1279). | Nothing on its flags. No step runs it; the runbook's table of owners names `cli fno report` at R12. |

**What `fno mask` can call today (`crates/cli/src/fno_daymask/`, `8d8080ddadba`).**
- Loaders, each returning its digest: `Universe::load(path)`, `JoinMap::load(path)`, `SkipTable::load(path, &universe, &join)`, `T2tTable::load(path, &universe, &join)`.
- `DayMask::new(Inputs { universe, join, skip, t2t: Option<_>, membership, facts, span, reading })`. `span` is `Span { first, last_proven, last_held }`. `reading` is `CalendarReading::NoSpecialSessions` (the headline) or `HaltAndOutagePlans` (a labelled variant).
- `membership` is any `PitMembership`; `pull`'s `Membership` already implements it (`mod.rs`:458).
- `facts` is any `DayFacts`. Outside the tests only `FactTable` implements it today; `mod.rs`:32 says FNO-RUN's store reader will.
- `DayMask::verdict(symbol, day)`, `rank_pool_member(symbol, day)`, `pit_outside(day)`, `labels()`.
- `MaskCounts`: `record`, `rows(&skip)`, `member_days(stock, year)`.
- The T2T file's header line (`state/post-landing-20261101/t2t-20261102/t2t.tsv`) equals `T2T_HEADER` byte for byte. Only the header was compared: `T2tTable::parse` has not run on that file.

**What `fno report` could read today.** `Report { label, nothing_passed, passers, refused_by_luck, closest, trials, newest }` with `plain_answer()`, and the constants `SURVIVORSHIP_LABEL`, `CLOSEST_LABEL`, `NOTHING_PASSED`, `WALK_FORWARD_PASSED` and `NEWEST_LABEL`. FNO-RECORDS, which would hold the rows, has no branch.

### The shards

**No shard code exists.** What the parts' interfaces show:

- **Why there are shards.** `cli::execution_lease` holds one non-blocking lock per store (`.sweep-execution-v1.lock`; `Lease::acquire(root)`, `probe(root)`). This is at the seam's sha. This file (:976) says years are sharded across shadows, and `parts.json` says the same. R11 assumes one shadow per shard, each an APFS clone of the frozen BASE, split by "year range or `--phase`".
- **Months are numbered from the span, not from a shard.** `Calendar::new(first_year, lead, span, sessions, days)` is the whole calendar. `windows(span, scheme)`, `lengths(span, ..)`, `family(span, ..)`, `Frame::new` and `Exam::new` all work from it. A shard that built its own shorter calendar would get other month numbers and other windows.
- **What already joins by month.** `Unit::new(months)` takes rows already folded, one `MonthAgg` per span month, in span order, for one (candidate, unit). `Histogram::merged(&[Histogram])` merges months, and `FeatureInput.months` is one histogram per span month. Neither reads a file: `fno_wft/mod.rs`:20 says nothing there reads a file, and FNO-RECORDS and FNO-RUN do the saving.
- **What does not join by concatenation.** `Unit::from_rows` refuses rows out of (month, day, second) order (`RowOutOfOrder`). `from_learning(window, features, excursions, nifty)` takes a window's months from every feature's whole-span list, and excursion lists are per month, so settings for a window need the months before it, whichever shard made them.
- **What crosses a year boundary.** FNO-DAYMASK's M11 reads the previous session (`prev_session_traded`). FNO-RANK's `DayEntry` needs the previous session's closes. FNO-DAILY decides on D and trades on D+1 (`signal_candle`, then `entry_for`). FNO-LADDER's anchors A6-A8 (`AnchorId::Prev`, `AnchorId::Gap`) read earlier sessions.
- **What a split by stock would touch.** `Pool::begin_day(open_ts, entries)` and `fold_minute` rank one pool a day, and a stock the plan leaves out is simply absent (`fno_cross_section/mod.rs`). FNO-PORTFOLIO's money rules are across stocks; it has no branch.
- **Design lines that bear on it.** Parallel inside a run: rayon over stocks for P2 and P4, per day for P3, per (window, partition) for P5, per candidate for P6 (:975). The identity `FnoRunV1` (:845) has no shard term. The proof's scope and sha join the identity (P0's decision). The sample's records may sit in shard 0 (R11).

### What FNO-RUN's builder must decide

Each is open. None is answered here, and each answer goes into `part-FNO-RUN.md` and the usage text, so the main session can correct R5-R7 and R11.

**The verbs.**
1. `fno margin build`: its flag names, whether it takes the execution lease (R6 says R5 and R6 overlap only if neither does), and what it prints as its report and digest.
2. `fno clock build`: its range flags, the flag and file format of the (stock, day) key list (:1358), how the shadow is named, whether it takes the lease, and whether it resumes.
3. `fno mask`: where `Span.last_proven` comes from, since this verb reads no proof (:1415); where `last_held` comes from; whether the `CalendarReading` is a flag (the T2T file is, since 2 Nov 2026: :971); and the output's form.
4. `fno report`: everything: its flags, what it reads, and how a report is named. FNO-RECORDS and FNO-RUN own it (:1279).
5. `fno run`: how the `CalendarReading` variant is chosen. (The T2T file: :968 names it since 2 Nov 2026, as `L-FNO-MASK-06` on the DayMask branch says FNO-RUN is given that table. The builder confirms the flag's spelling.)
6. The shadow argument: `fno run` and R7 take it as a positional and R5-R6 do not; `BRUTEX_STORE` is also set in each runbook command. Which one is the authority.

**The shards.**
7. What a shard is: a range of traded days, a `--phase`, or both, and the flag names. R11 waits on this.
8. Which of P1-P8 run per shard and which run once over the whole span, and where P5-P7 run.
9. How the once-only phases get every shard's month rows and histograms: where they are kept, in what form, and which store or folder that is, given that each shard writes only its own shadow.
10. What refuses a join: a missing shard, a month held twice, shards from another run, cut or binary.
11. What a shard reads outside its own days (the previous session, D+1, yesterday's ladder), and how `--from` and `--to` read then.
12. Whether every shard uses the whole-span `Calendar`.
13. Whether a split by stock is ever allowed, given the rank pool and the money rules.
14. Identity: one run id across shards or one each, and whether the shard count is a term. What a resume with another shard count does.
15. Kill and resume (:983) across a join: what byte-identical means for shards and for the joined records.
16. The lease during a join, and whether shard 0 may be `$SH` with the sample's records in it (R11).
17. The projection (R9, R10): how `--sample`'s figure scales to N shards.

## OPERATOR DECISION, Mon 2 Nov 2026 ~16:5x IST: "Yes, all before start" (main session c0907298; written by workflow wf_5b10c1d8-2f6; backup `PARTS.md.bak-20261102-abs`)

After "without having the error free bug free complete precise heart piece data we should never ever move ahead", the operator withdrew two yeses of 1 Nov. Both stay above as the record, marked WITHDRAWN. This section binds over them.

- **P0: the default applies.** "Data proven" is the whole AUDIT-3 pass, every data-run row 1-24 live and verified. The file `proof-v1.json` says `"scope": "whole"` and `"operator_consent": null`. No run passes `--proof-scope bars`. FNO-RUN's first build does not write --proof-scope: P0 accepts only scope whole and refuses any other scope by name. No output carries the "data proof: bar part only" label.
- **"The full backtest runs only from merged `feat/db-fix`" (:236, SPEC.md:94) reads as first written:** every family is on it, and the three base parts: BASE-REDS-1023, BASE-STACK-1028, BASE-BENCH-1028, FAM-DATA-A0, -A1, -B, FAM-FIB-1, FAM-FIB-2, with R4-20 and the nine parts of deploy section 1 item 6 (deploy runbook R0, G1).
- **FNO-RUN's rule (:1318) "the LANDED data family":** A0, A1 and B with all 17 parts. Its COMMANDS line is the landed value it finds plus its own verbs.
- **FAM-FIB-1's args take no `--later`:** `famland-args.mjs fib1 --passed <fib's>`. BENCH lands right after STACK, so FAM-FIB-1 finds it landed.
- **The identity (:845) holds `commit`,** as before: the run's binary is M's `tgt/rel-$S8`, and nothing is rebuilt or removed while the run lives (deploy runbook R12).
- **FNO-API's bench:** do not stop, as the 1 Nov ~22:3x IST line says. Its gate-8 figure is due on the first tree that holds both FNO-API and BENCH, before the start.
- **No fib part reads PCT-1's columns, SYMCAL-1's `not_owed`, or `.srt`:** as before (:841, :895, :1421). R0 stays refused in FNO-STRAT's first build (:895), whatever P8's tables hold.

## MAIN-SESSION DECISIONS, Mon 2 Nov 2026 ~21:3x IST: READINESS ADDENDUM, FNO-RUN's brief and the parts it calls (binding; main session c0907298, workflow readiness-fix, group runbrief; findings `state/readiness-20261102/runbrief-verify.md`; backup `PARTS.md.bak-20261102-rf`)

Binding on every builder, checker and repair of the parts it names, F1 included. The old text above stays as written; where it differs, this section stands, and each part's section carries a one-line pointer here. Line numbers (:NNN) are this file's; code lines are at the shas named. No date moves. No check, floor, lock or stop rule is weakened. FNO-SEC-WIRE, FNO-CLOCK, FNO-LEVERAGE and FNO-RUN have no branch today, so none of this costs a re-landing.

**1. The word `fno` (FNO-RUN; RB-02, NF-2).** Code lines at `fix/fib-fno-seam` 1ad6e24ef8e0. FNO-RUN's tree holds the landed data family when it writes these (:1318 and REPAIR 1), so each count below is the landed value it finds plus one, never a typed number.
- **Three count lines, not one (:1351 is corrected):** `COMMANDS`' length (:3180 of `lib.rs`; it is :3167 at bcac15e2c, the line the anchor tables use), `WRITERS: [&str; N]` (`store_write.rs`:102) and the test's `writing: [(&str, &[&str]); N]` (`store_write.rs`:491). `sweep_wiring_tests.rs` has no count. The slice type is not available for any of them: `crates/cli/src/store_write.rs:309-317` and `crates/cli/src/sweep_wiring_tests.rs:147-160` need `&str` items (:1317).
- **`store_write.rs`, FNO-RUN's anchors:**
  - `"fno",` in `WRITERS` right after `"feed-reconcile",` (:107), before `"fno-membership",`. The array stays sorted.
  - One arm in `writes_the_store`, right after `"fno-membership" => arguments.contains(&"build"),` (:149), in exactly this shape:
    `"fno" => matches!(arguments.first(), Some(&("clock" | "margin" | "run"))),`
    It splits on the sub-verb: `fno run`, `fno margin build` and `fno clock build` write; `fno mask` and `fno report` read. Gate 30's `form_of` (`.github/gates_store_steps.rs`:2362) reads three arm shapes and refuses any other by name (:2391-2393). A `.contains` also costs a line of gate 11's budget, which caps this file at 7 (`.github/gates_ledger.rs`:2037).
  - In the test beside it: `("fno", &["run"], true)`, `("fno", &["margin", "build"], true)`, `("fno", &["clock", "build"], true)`, `("fno", &["mask"], false)` and `("fno", &["report"], false)` right before `("feed-reconcile", &["2025-03", "--out", "audit"], true),` (:352); `("fno", &["run"]),` in `writing`, right after `("feed-reconcile", &["2025-03", "--out", "audit"]),` (:496); `("fno", &["mask"][..]),` and `("fno", &["report"][..]),` right before `("top", &[][..]),`.
- **`sweep_wiring_tests.rs`, FNO-RUN's anchor:** `| "fno"` in `only_actual_computation_verbs_can_establish_sweep_status`, right after `| "fno-membership"`, with a comment that says why: `fno run` computes a strategy, and its P0 takes the lease and the audit itself, because the lease goes by the first word.
- **The lease path, as :974 already says.** `fno` stays OUT of `is_sweep_command` (`lib.rs`:3250). `run_durable` (:2826-2834) takes the lease by the first word, before dispatch, so with `fno` in there all five sub-verbs would take it, R5 and R6 could not overlap, `fno mask` and `fno report` would be refused while a run lives, and P0's own `Lease::acquire` would meet the lease its own process holds and answer `Busy` (`execution_lease.rs`:179).
  - `fno run`'s P0 does by hand what `run_durable` does for a sweep: `execution_lease::Lease::acquire(&root)`, `operation_audit::begin(&root, operation_audit::Origin::Cli, "fno")`, the run inside `audit.enter(..)`, then `audit.finish(audit_phase(code), 0)`. The lease is held through that terminal audit, and a refused finish is a failed run, as at `lib.rs`:2846-2854. This keeps the doc line at :3247 true in substance: a strategy run has its lease and its invocation audit.
  - `fno margin build`, `fno clock build`, `fno mask` and `fno report` take no lease, as `fno-membership` takes none. So R5 and R6 overlap, and `fno mask` and `fno report` run beside a live run.
- **Tests:** `fno_run_holds_the_lease_through_its_terminal_audit`, `a_second_fno_run_on_one_store_is_busy`, `fno_mask_and_fno_report_run_beside_a_live_run`, `fno_run_writes_one_invocation_audit`.
- **FNO-RUN's check runs gate 30 and gate 11 on the post-data tree, rc 0 required.** Gate 30 is rc=1 on fib's base and rc=0 on `BASE-REDS-1023-r6-full` (`ci-gates-logs/FNO-SEC-EXEC-489e256c9861-lp/summary.txt`), so on the fib base a bad arm reads as "the base's set" and first shows at FAM-FIB-2's landing.

**2. The identity has two levels and eight more terms (FNO-SEC-WIRE builds it, sole owner of `identity.rs`; FNO-RECORDS, FNO-API and FNO-RUN use it; RB-03, RB-04, NF-1).** :845 and :846 are overtaken by this item.
- **`FnoRunV1` is the run, with no recipe and no scheme:** `FnoRunV1 { signal_feed, fill_feed, lanes, mapping, cost_table_version, leverage_digest, universe_digest, join_digest, skip_digest, t2t_digest, membership_digest, clock_digest, band_rule, pool_digest, vocab_version, commit, span_first, span_last, sample, data_digest, quote_digest, calendar_reading, proof_scope, proof_sha256 }`, and `fn fno_run_digest(&FnoRunV1) -> [u8; 32]`, with a new append-only tag. This digest is the id of a whole run: the `run=` of every `/fno/*` route, the identity in the run header (:932), and what a resume compares.
- **`FnoCandidateV1 { run, wft_scheme, recipe_bytes }` is one candidate,** `run` being that digest, and `fn fno_identity(&FnoCandidateV1) -> RunId`, with its own append-only tag. A shard, if there ever is one, has no term: one run digest across shards.
- **The eight terms:**
  - `span_first`, `span_last`: the run's `--from` and `--to`.
  - `sample`: a distinct none term for a full run; for `--sample N:M`, N, M and the digest of the stock list item 6 picks. A sample and a full run never share a digest.
  - `data_digest`: the term of CLAUDE.md's rule (SPEC:902). One blake3, under its own domain tag, over every month binding the run opens, in (feed, stock key, rung, month) order: GDFL through `gdfl_month_binding_v1` (:844), and Zerodha through a new `zerodha_month_binding_v1` with the same inputs, read from the file the store serves, so a repaired month's revision counts. A new cut with one changed month has another digest. No bar is hashed.
  - `quote_digest`: the same blake3 over one `.qrg` binding per GDFL stock-month the run reads: the record count, the first and last stamp and a digest of the file's block checksums. `gdfl_month_binding_v1` covers the bar file, not `.qrg`, and a month's `.qrg` can be rebuilt whole (`spec-QRANGE-1.md` rule 4). The term is 32 bytes with a distinct none term in `identity.rs`. FNO-RUN computes it on the post-data tree, because QRANGE-1's reader is not in FNO-SEC-WIRE's base. The none term is for tests: P0 refuses a real run with a missing month (item 3).
  - `calendar_reading`: `NoSpecialSessions` or `HaltAndOutagePlans`, as a word. Without it the labelled variant and the headline hold one id, and the rerun writes nothing (:950).
  - `proof_scope`, `proof_sha256`: the terms :1418 promises: the word the file says (`whole`) and the sha256 of FILE.
- **`pool_digest` (SPEC:910 gives it no content; this is it):** the same blake3 over the Zerodha 1-minute month bindings of every stock in the rank pool over the span, warm-up months included. The rank pool stays the 218 in a sample (item 6), so a sample's `pool_digest` covers the 218's months for its span.
- **Resume (:968, :1424).** P0 reads the run headers in `results/fno-v1/`. "Terminal" means the run's journal ends in `run finished` or `run refused` (item 7). `run stopped` is not terminal: a stopped run resumes, as a killed run does, so a stop loses no refusal.
  - The same `fno_run_digest` with a terminal row: writes nothing (:950). A `run refused` prints its refusal again.
  - The same digest with no terminal row, stopped or killed: resumes.
  - Another digest beside a run with no terminal row, stopped or killed: P0 stops by name and prints each term that differs. That is the refusal the tests at :968 and :1424 name.
  - Another digest, with every run in the store terminal: no refusal. So a finished sample stays beside the full run in `$SH` under its own digest, and a refused sample does not block one with a larger M (item 6).
- **`--keep-trades id` (:944, :968) is no term of the digest.** It is read when a run starts, and the run header holds the list. A resume with another list refuses by name, because item 10's byte test needs one list. On a digest with a terminal row it writes nothing, as any rerun does, and says so by name, so a late request is not silently lost: no trades are added to a run that has ended. The picked recipes' trades are kept in any case (:944).
- **Migration:** adding terms is a migration (SPEC:902), so FNO-SEC-WIRE records its decision entry. Its tags are new and append-only, and existing Run ids stay unchanged.
- **Tests, FNO-SEC-WIRE:** `each_run_term_moves_the_run_digest`, `the_candidate_id_moves_with_recipe_and_scheme_alone`, `a_sample_and_a_full_run_never_share_a_digest`, `the_zerodha_binding_changes_with_each_of_its_fields`. **FNO-RUN:** `a_new_cut_under_one_commit_has_another_run_digest`, `a_rebuilt_quote_month_changes_the_run_digest`, `the_halt_plan_reading_is_its_own_run`, `an_unfinished_run_under_another_digest_refuses_by_term`, `a_finished_sample_does_not_stop_the_full_run`, `a_stopped_run_resumes_under_its_digest`, `another_digest_beside_a_stopped_run_refuses_by_term`, `a_refused_run_writes_nothing_again`, `a_larger_sample_starts_beside_a_refused_one`, `keep_trades_on_a_finished_run_writes_nothing_and_says_so`, `a_resume_with_another_keep_list_refuses_by_name`.

**3. Who reads `.qrg`: FNO-RUN (RB-05; N4, the decision of 31 Oct ~17:4x IST).** `spec-QRANGE-1.md` rule 5 names FNO-SEC-WIRE and FNO-SEC-EXEC, and `workflow-fib-3.js`:68 names FNO-RUN. FNO-SEC-WIRE cannot call a store type that lands after it, so the reader is FNO-RUN's (`spec-QRANGE-1.md`, ADDENDUM 2 Nov 2026). It is written after FNO-RUN's post-data merge, with the count lines of item 1.
- **The adapter (:977), in `fno_run/inputs.rs`:** one `fno_exec::SecondSource` over FNO-SEC-WIRE's `GdflSeconds` and QRANGE-1's `store::quote_range::QuoteMonth`.
  - `second(at)`: a traded second with a quote row goes through `Second::with_quote`.
  - A traded second with no quote row, in a month whose `.qrg` is present, goes through `with_quote(QuoteRange::new(sec.high(), sec.low())?)`. This is the ordinary case: `.qrg` holds a record only for a second with a moved LTQ-0 row (`spec-QRANGE-1.md` rule 2, 0.4-1.2% of in-session price changes). It widens nothing (`high.max`, `low.min`), so no price moves, and it says loaded. Left as `quote: None`, it would count in `quote_absent` (`walk.rs`:542 at 489e256c9861), so `quote_range_not_loaded()` (:184) would be true on nearly every trade of a fully loaded month, and `Event::Fill`'s `quote` (:841) false. `QuoteRange::new` cannot refuse here: `Second::new` has already refused a low under one tick or above the high.
  - `next(at)`: the earlier of the next traded second and the next quote row. So the walk meets the quote-only seconds in which `Second::adverse_for` (`fno_exec/mod.rs`:302) fires a stop. A source over bar files alone never yields one.
  - A quote-only second is `Second::new(ts, quote.high, quote.low, quote.high, 0)` and then `with_quote`. No bar supplies its `last`; the value only has to lie inside the range, because the walk reads `last` on traded seconds alone (`walk.rs`:1167-1174) and volume 0 keeps it from every fill and volume cap.
  - `first_traded_at_or_after` and `last_traded_at_or_before` stay on traded seconds (SPEC OP-15).
- **P0 refuses a held month with no `.qrg`:** every GDFL CASH 1s month the run reads whose `.qrg` answers `absent` (`spec-QRANGE-1.md` rule 5) is named, stock and month, and nothing runs. Otherwise a partial gap runs labelled month by month, and the operator's rule of 2 Nov allows no start on a known gap (deploy runbook section 9 item 6). The label "quote range not loaded" stays in the walk, where FNO-SEC-EXEC tests it, and no run prints it.
- **Tests, on the post-data tree, inside FNO-RUN's passing check:** `a_stop_fires_in_a_quote_only_second`, `a_fill_takes_the_worse_of_the_traded_and_the_quote_range`, `a_held_month_with_no_quote_file_refuses_by_name`, `a_traded_second_with_no_quote_row_in_a_loaded_month_is_not_quote_absent` (a month mixing seconds with a row and seconds without: `quote_absent` 0, no label, and the fill prices of the seconds with no row equal to the traded range's). A fixture with `None` proves nothing about this adapter.

**4. One process is allowed (FNO-RUN; RB-06). A design line, not operator law.** :976's reason does not hold on this path. MVP correction 5 (`state/strategy-mvp-20261030/REPORT.md`:146) is about several old-path sweeps at once. This path is one process with rayon inside (:975), so one lease does not limit its cores: this Mac has 14 cores and 48 GiB, all reachable by one process. P4 reads all earlier history (SPEC:802), P5 needs every learning month, and P6 restarts capital at each window, so it could split by window but not by year (item 5). None of that adds a core on one Mac.
- **N = 1, one `fno run` on `$SH` with no clone, is allowed, and R9, R10 and R11 plan it first** (deploy runbook R11).
- **FNO-RUN's first build need not hold `--phase`, a shard flag or a join.** If it leaves them out, its usage says so. The appendix's questions 7, 8, 9, 10, 12 and 13 then fall away, and 11, 14, 15, 16 and 17 keep their one-shadow form.
- **A split comes back only on a measure:** R9 prints peak memory (item 6), and R12's tick reads the live process's. If one process cannot hold the run, the main session reopens this item, and the appendix's questions stand as written.

**5. Capital and the month rows (FNO-RUN, FNO-PORTFOLIO; FNO-WFT's rows follow; RB-07).**
- **Where capital starts.** OP-21 reads "carried from the window start" (SPEC:1084) and SPEC:671 names the forward window. A learning window is read the same way: every window's walk starts at Rs 20 lakh on that window's first session, and carries its own net from there.
- **P6's rows are per (candidate, scheme, window).** Where capital binds, one month of one candidate differs by the window it sits in, and `Unit::new` (`agg.rs`:457) holds one row per span month for one (candidate, unit). Reusing one row across schemes gives wrong refusal counts.
- **A row is shared only when it is proven equal.** Quantity never reads capital (SPEC 11.1-11.2), so a candidate whose capital never binds trades the same under every window start. That is proven by: its peak margin in use + the largest peak-to-trough fall of its carried net over the span (worse clock) <= Rs 20 lakh. Such a candidate may be walked once and the bound checked after the walk, and SPEC 13.4's "differences of per-month prefix vectors" then holds for it. The bound holds only if that walk had no `margin_cap` refusal (SPEC 11.3), or ran with the capital check off. A refused entry is the failure itself: margin in use never saw it, so the figures can pass, while a window that starts after the fall has more capital and takes that entry. With no refusal, margin in use at any moment is at most Rs 20 lakh less the largest fall, which is at most the capital of every window start, so no window refuses either. If the bound fails, or the walk has one such refusal, that walk is thrown away and the candidate is walked per window, as is any other candidate. A procedure (SPEC 11.6) is always walked per window.
- **The cost is unknown until measured:** per-window walks multiply P6, and no estimate holds that, so item 6 prints it apart.
- **Tests:** `each_window_starts_at_twenty_lakh`, `a_candidate_under_the_capital_bound_has_equal_rows_under_two_schemes`, `a_candidate_over_the_capital_bound_is_walked_per_window`, `a_walk_refused_for_capital_never_shares_its_rows`.

**6. `--sample N:M`, the projection and the signal cache (FNO-RUN, FNO-STRAT; RB-08, RB-16, NF-4, NF-5).** Measurement only. It changes no verdict.
- **It is never a verdict.** Its run digest holds `sample` (item 2), every output adds the label "sample: a measurement, no verdict", and `fno report` prints no pass for it. The run header holds the label, so every `/fno/*` response for that run carries it too (FNO-API). The full run's floors do not move.
- **The pool stays the 218.** P1-P3 run on all 218 over the sample's months, so rank clauses select as they will in the full run. P4-P7 run on the N stocks. In a pool of 10 every stock is in the top 10 all day.
- **The N stocks:** positions floor((i + 1/2) x 218 / N), i = 0..N-1, of `fno218.txt` in file order. Fixed in advance, never chosen on results.
- **The M months:** the last M whole months of `--from..--to`. Reads outside them follow the full run (the appendix's question 11). `M >= 3`: `lengths` and `windows` (`scheme.rs`) need one learning month and two forward windows, so a 2-month span is the refusal at :982. This also holds FNO-RUN's end-to-end check (:1138).
- **The sample's floor, in a sample only:** the pair-aware floor of decision 1 (:1354) x N / 218, rounded up, and `shortest_learning`'s floor the same way. The absolute 500 a year would kill the search early on about 5% of the signals (ESTIMATE, by count of stocks), and the frontier, the declared unknown (:1238, :1259), would measure far too small, with no warning.
- **A sample that stops at `TooShort` or `NoMinimum` fails R9.** It writes `run refused` (item 7), so the larger sample starts beside it. Raise M and run again. Nothing lowers a floor further.
- **The projection is one line (G-L5).** The last line of a sample's stdout, one JSON object after the word `projection`. Stdout reaches `$FR/sample.log` only at exit (item 15), so R10 reads it after R9's process has ended:
  `projection {"version":1,"sample":"10:12","fold_seconds":N,"search_seconds":N,"walk_seconds":N,"records_kib":N,"signal_cache_kib":N,"peak_rss_kib":N}`
  - `fold_seconds` (P1-P4) scales by stock-months. `search_seconds` (P5) and `walk_seconds` (P6) are counted apart from it, because they follow the frontier and the walks of item 5, not the stock-months. The line above it says in words how each was scaled. All are ESTIMATES.
  - `records_kib` is the full run's projected records, `signal_cache_kib` the cache below, scaled from the sample's measured bytes.
  - `peak_rss_kib` is the sample process's own peak memory, measured and not scaled. P1-P3 held all 218 over M months, and P4-P7 only N stocks, so it is a floor for the full run, not a projection. A figure near the Mac's 48 GiB says one process will not hold; a small one proves nothing alone. Item 4's unknown is settled by R12's tick, which reads the live process through the full run.
- **The signal cache's folder (:151, :880):** `<shadow>/fno-cache-v1/`. FNO-STRAT's cache takes its root as an argument, and FNO-RUN passes this one. It holds rank series, distributions and posting lists, each keyed as :151 says. It is not in `results/fno-v1/`, and it is per shadow.
- **Tests:** `a_sample_keeps_the_pool_of_218`, `a_sample_picks_the_same_stocks_each_time`, `a_two_month_sample_is_refused_by_name`, `a_sample_prints_one_projection_line`, `a_sample_is_no_verdict`.

**7. "Running", and the journal's lifecycle rows (FNO-RECORDS, FNO-RUN, FNO-API; RB-09).**
- **FNO-RECORDS:** the journal (:941) holds these rows beside its phase and stock-month rows:
  - `run started`: one for each start and each resume (item 10), after P0's refusals have passed and the run header is in place. A P0 refusal writes no row, so nothing is left to resume;
  - `run stopped`, with the phase it stopped after: not terminal (item 2);
  - `run finished`: terminal;
  - `run refused`, with the refusal in words: `fno_wft::Refused::TooShort` or `NoMinimum`, met after `run started`. Terminal: the same digest would meet the same refusal. Any other failure after `run started` writes no row, so the run reads `killed`, and the same digest resumes once the cause is mended.
- **How a stop is asked for (FNO-RUN).** As `cli gdfl-stop` asks a GDFL run: one file, never a signal and never the run's lock (`crates/cli/src/gdfl_stop_command.rs`). The operator makes the empty file `<shadow>/fno-run.stop` (deploy runbook R12).
  - The run looks for it at each phase boundary, once the phase is journalled. If it is there, the run writes `run stopped`, removes the file, and ends through its terminal audit with a failed code, so nothing reads a stop as done. A phase is never cut, so a stop waits for the phase to end. A request met after the last phase is removed with `run finished`: the run has ended anyway.
  - P0 refuses by name when the file is already there, because a stale request would stop the next run at its first boundary.
  - The file sits outside `results/fno-v1/`, so item 10's compare never meets it. The name is this brief's: a usage that spells it otherwise says so in `part-FNO-RUN.md`.
  - If the first build keeps `--phase p` (item 4), a run that ends at p writes `run stopped` too.
- **FNO-API:** each row of `/fno/runs.json` has `state`, from the run's last lifecycle row: `finished`, `refused` or `stopped` from those rows. With `run started` last, or a header with no row yet, it is `running` while `cli::execution_lease::probe(root)` answers `Busy`, and `killed` when the lease is free. `probe` creates no file and changes no result. A route sees only the store it serves (deploy runbook R8, R12).
- **Tests:** `a_run_with_no_terminal_row_and_a_held_lease_is_running`, `a_run_with_no_terminal_row_and_a_free_lease_is_killed`, `a_refused_run_reads_refused`, `a_stopped_run_reads_stopped_and_running_again_once_resumed`, `a_stop_file_ends_the_run_at_the_next_phase_boundary`, `a_stop_file_left_from_before_refuses_at_p0`.

**8. `fno clock build` takes the key list as its one input (FNO-CLOCK, FNO-RUN; RB-11, decision 5 at :1358).** FNO-CLOCK depends on FNO-SEAM alone (:755), so `build(range)` (:746) cannot know the mask, the universe or the join: it could calibrate every key in CASH, or none of the renames.
- **FNO-CLOCK:** `build(keys, store, out)`, keys being (stock, day) pairs, and nothing else chooses a day. It takes no mask type. Decision 5's test stands: a full build and a list rebuild over the same keys give equal bytes, and keys outside the list are untouched.
- **The verb, FNO-RUN's:** `cli fno clock build --from --to --universe --join --skip --t2t --membership [--keys FILE]`, as R7's inputs. With no `--keys` it runs the day mask over the span and passes every `Tradable` stock-day, under the key or keys `JoinMap::keys_for` gives for that day. With `--keys FILE` it takes those keys (the deploy runbook's L6 block). It reads no proof (:1415), and the mask reads no clock, so nothing circles. The flag spellings and FILE's format are the builder's, in usage.
- **The key list's reading:** the day mask for the list always runs under `CalendarReading::HaltAndOutagePlans`, whichever reading a run will use, so the verb has no reading flag. Its `Tradable` days are the headline's plus the two March 2020 halt days and 2021-02-24 (`classify`, `fno_daymask/plan.rs`, at fad867d2fc2b). One table then serves both readings, a run reads only its own keys, and `clock_digest` does not split them. R6's "one row per tradable stock-day" counts those days. Test: `the_key_list_holds_the_variants_days`. `fno run` and `fno mask` take the headline; a flag for the variant is the builder's, in usage.

**9. `fno report` (FNO-RUN; RB-12, :1279).** No later design round exists: FNO-RUN's builder designs it, flags included, from these contents:
- the closest recipes, labelled "did not pass: closest to the requirement", ranked by checks passed of 38, then shortfall, then net (:1276; decision 2 at :1355);
- every failed check by how much, both scores, every window and the trade count (:1277);
- "nothing passed" plainly (SPEC:821), and the newest windows apart (SPEC:823);
- the run digest, the trial count and every label.

It reads FNO-RECORDS's rows, and it takes `runner::fno_wft::report`'s `Report`. It writes nothing to the store: stdout only. The deploy runbook's R14 runs it when the run ends.

**10. "Byte-identical records" (FNO-RECORDS, FNO-RUN; RB-13).** At :983 "records" means every sealed row family under `results/fno-v1/` and its indexes. The journal is left out: each row has a `time` (:941), and a resume adds rows. The test compares every other file byte for byte, and asserts that the resumed journal holds the killed journal as its prefix. :983's test kills after a phase; a power cut does not wait for one (deploy runbook R14 resumes a `killed` run first), so a second test kills inside a phase, between two stock-months: `a_kill_inside_a_phase_resumes_to_byte_identical_records`.

**11. A month row missing from a join or a resume (FNO-RUN; NF-7).** `Unit::new` is infallible and unchecked (`agg.rs`:457), and `Prefix::total` reads a month past the rows held as a month with no trades. So rows lost by a join or a resume would read as quiet months, never as an error. Before every `Unit::new` FNO-RUN checks that the row list's length equals `calendar.span()`, and refuses by name with both counts. Test: `a_month_list_shorter_than_the_span_is_refused`.

**12. FNO-LEVERAGE's anchors for gate 1d (NF-3).** Gate 1d says every segment-shaped literal under `crates/pull` is declared, and `crates/pull/src/nse_margin.rs` is new (`nse-margin-v1`, `nsm` and the like). Code lines at `fix/fib-fno-seam` 1ad6e24ef8e0. Both anchors below are read against the data build's table B1 (`state/data-build-20261031/PARTS.md`, every data part's anchors in this file) and BASE-REDS-1023's hunks, by the same rule: at least 2 unchanged base lines from any other part's edit. `PIT_1_LITERAL`'s own neighbour is not free: SYMCAL-1 inserts after :2991, and REPLACE-2 after :3088.
- The group's comment, saying where each word was read, as PIT-1's says, `const FNO_LEVERAGE_LITERAL: &str = "...";` and one blank line, right after the blank line that follows `LATE_LABEL` (`.github/gates_tree.rs`: it ends :2335; insert after :2336).
- `    FNO_LEVERAGE_LITERAL,` right after `    NAV_WORD,` (:3047).
- The length line (:3018) changes to exactly `const DECLARED: &[&str] = &[`, byte for byte the line AUC-1, ICI-1, REP-OI, KASK-1, REFOLD, CAL-2027 and CAL-1126 each wrote, so equal edits merge clean. Never `[&str; 80]`, and the length comment above it stays untouched.
- Gate 1d is red on fib's base, on the base's own 481 words at 489e256c9861 (`state/fix/ci-gates-logs/FNO-SEC-EXEC-489e256c9861-lp/logs/language-purity/009-1d.log`; the 455 at `gates_tree.rs`:2401 is a 12 Oct figure), and rc=0 on `BASE-REDS-1023-r6-full`. So FNO-LEVERAGE's check compares the gate's word list with the base's: no word of `nse_margin.rs` may stay on it.
- **Preflight:** `CROSS['FNO-LEVERAGE']` in `state/preflight-20261101/preflight-core.mjs` holds :2336, :3018 and :3047, and data `PARTS.md` section E names the slot, so `args.preflight` prints no ANCHOR-NONE for these edits. Lines at bcac15e2c: the file is equal at `fix/fib-fno-seam`.

**13. FNO-DAYMASK's bench line (preflight F11).** Its edit at `crates/cli/benches/ratio.rs`:543 stays. `fno_daymask::row` adds a budget row, so "the two budget rows" would be false, and the count-free text spares FNO-CLOCK, FNO-SEC-WIRE, FNO-RECORDS and FNO-RUN this edit. The line goes in `part-FNO-DAYMASK.md`:
`PREFLIGHT-WAIVER ANCHOR-SLOT crates/cli/benches/ratio.rs:543: the line loses its count because C-FNO-MASK-01 adds a budget row; restoring it would be false (readiness preflight-verify F11)`

**14. What is still the builder's.** The flag spellings, in usage; question 2's resume (the key list can serve it); question 3's `last_held` reader and print forms; question 6's authority (one source, or a mismatch refused by name). Each answer goes into `part-FNO-RUN.md` and usage, so the main session can correct R5-R7, R11 and R14. Item 7's file name and item 15's event names are this brief's and R12 reads them, so a change to either goes the same way. Answered above: question 1's and 2's lease (item 1), 2's keys (item 8), 4 (item 9), 5's identity (item 2), 14 (item 2), 15 (item 10), 16's records (item 2), 17 (item 6).

**15. What a live run shows (FNO-RUN, deploy runbook R9, R12; `state/readiness-20261102/fix/runbrief-check1.md` B3).** A verb writes into `out: &mut String`, and `main` prints it once, after the verb returns (`crates/cli/src/main.rs`, `cli::deliver`). So `$FR/sample.log` and `$FR/shard-<k>.log` are whole only at exit, and a multi-day run's log holds nothing while the run lives. FNO-RUN does not print around that: gate 23's clause A holds the print surface exact (`.github/gates_runtime.rs`:1534), and its list is in `.github/workflows/ci.yml`, where a fib part has no anchor. The live reading is telemetry (:979): `telemetry::emit` writes each line at once, with nothing buffered in user space, to `<store>/logs/cli/events.ndjson`, rolled to `events.<n>.ndjson` (`crates/telemetry/src/sink.rs`:562-570). Code lines at `fix/fib-fno-seam` 1ad6e24ef8e0.
- **The events, target `cli.fno_run`, level info. Every one holds `run_digest` (hex).** Every run on one store writes to one log folder, and on the planned route that is R9's sample beside the full run, so nothing else tells two runs apart (3 Nov 2026; `state/readiness-20261102/fix/runbrief-check2.md` C1):
  - `run started`, once for each start and each resume, right after the journal's row of that name. Fields: `run_digest`, `resumed` (bool) and `labels`.
  - `phase started` and `phase finished`, one pair a phase, each with `run_digest` and `phase` (`P1`..`P8`). `phase finished` also has `labels`.
  - `run finished`, `run refused` (with `why`) and `run stopped` (with `phase`), each with `run_digest`, each beside the journal's row of that name (item 7).
- **`labels` is one text:** every label the run holds so far, in the output's own words, joined by ` | `: the survivorship label, "price-action blocks only, filters follow", the calendar reading's label, "T2T unchecked" when the run has no table, "sample: a measurement, no verdict" on a sample, and "quote range not loaded" from the first phase in which a walk's `quote_range_not_loaded()` is true. One text value holds 512 bytes, past which the line says `"cut":true` (`crates/telemetry/src/event.rs`:88), so a test holds every label at once and asserts no cut.
- **No event, no pass.** A `cli` whose log folder another `cli` holds runs with no log, on a warning alone (D-1537). Then the run writes no event of its own, and the newest `run started` in the folder can be R9's sample's. So R12 stops on no `run started` event, on a newest one that holds "sample: a measurement, no verdict", and on the warning itself: `announce_unlogged` (`crates/cli/src/lib.rs`:4187) writes `events are NOT being recorded` to stderr at once, for every verb, and R11's `2>&1` puts it in the shard log. It is the one line a live run's log holds.
- **Tests:** `the_run_started_event_holds_the_digest_and_every_label`, `a_resume_writes_its_own_run_started_event`, `each_phase_has_its_two_events`, `every_event_of_a_run_holds_its_digest`, `a_walk_with_no_quote_range_puts_the_label_on_its_phase_event`.

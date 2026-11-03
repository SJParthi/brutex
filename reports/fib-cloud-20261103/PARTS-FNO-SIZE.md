# PARTS-FNO-SIZE: the sizing and admission policies of the F&O engine (part FNO-SIZE)

STATUS: FINAL for this step (Tue 3 Nov 2026 ~17:5x IST, spec workflow wf_7aec42d4-f08). A later check of this workflow may amend it;
its dated backups go to `test-work/`. Repair 1 (~18:3x IST) answers check1.md: repair1.md (same folder) lists each finding and its fix.
Repair 2 (~19:3x IST) answers check2.md: repair2.md (same folder). In this file: the risk book takes releases in any order (4.4),
the view's negative figures are refused by name, the edge value floors a negative net, the bench shapes of -01, -04 and -05,
the row call's rustfmt layout, a RANKED table per window, and the queued entries cancelled at 15:10.

Written for the main session. Nothing was built, pushed, launched or fetched in the project, and no ref or file of it changed; no
cargo, rustc or clippy ran; every code line cited was read with `git show <ref>:<path>` in `/Volumes/WD_BLACK/brutex/fresh-20260919/project`.
The scripts of this folder were tried only on copies in `test-work/` (a scratch repository there borrows the project's objects
through an alternates link and holds its own refs).
Short names: P = `/Volumes/WD_BLACK/brutex/fresh-20260919/project`, that repository; W = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`; FP = `W/state/fib-strategy-20261030/PARTS.md`;
SPEC = `W/state/fib-strategy-20261030/SPEC.md`; F3 = `W/state/fib-strategy-20261030/workflow-fib-3.js`; OI =
`W/state/strategy-psrb-20261103/operator-inputs-20261103.md`; BAR = the memory file `backtest-acceptance-bar.md`;
SEAM = `1ad6e24ef8e05e8836e511873e8352071dd98fa0` (FNO-SEAM's passed tip); KIT1 = `fa9022bac962620528735ab97a61d6ab6d3dedbb`.
Branch tips read: `fix/fib-fno-portfolio` 4034f5d1e343, `fix/fib-fno-net` 74324377b9d8, `fix/fib-fno-leverage` 78582689a079,
`fix/fib-fno-wft` a67a84918c37 (`git for-each-ref`, 3 Nov ~17:1x IST). A later repair can move a line: read again before leaning on it.
Read again at repair 1 (~18:2x IST): PORTFOLIO, NET and LEVERAGE unchanged; FNO-WFT moved to d3962603ea20 (18:01 IST, its repair), with
`generate.rs` byte-identical, `gate.rs`'s `Exam::new` now at :477-480 and `returns_required` at :495, and `mod.rs`'s `Trials` at :683-706
and `report` at :948. Every FNO-WFT line cited below is a67a84918c37's.

Every result this part's numbers reach carries, beside FNO-RUN's labels, the policy label and the limits label of section 6.

---

## 0. The answer in brief

1. **One new pure module, `runner::fno_size`, owned by FNO-SIZE alone**, on SEAM (through a kit commit on KIT1). It takes
   every other part's figures as plain integers (as FNO-NET takes leverages as plain rationals, FP:626, and as the F2 rule
   "define a plain row or value yourself", F3:66, asks), so it calls no code of FNO-PORTFOLIO, FNO-NET, FNO-LEVERAGE or
   FNO-WFT, none of which has passed (`W/state/resume-kit/NEXT-PROMPT.md`:1426: "LAW, PORTFOLIO, BAND, LEVERAGE check 1 next;
   LADDER and WFT repair first"; FNO-NET was being built in the live run, `build-FNO-NET` in its journal).
   So it can be built now, in the cloud, with no speculative cut, no catch-up and no re-check against a moved base.
2. **It edits no other part's file.** Its four shared-registry edits sit at its own anchors (section 2), at least two
   unchanged base lines from every other part's lines.
3. **The four policies (OI:5-8), exactly (section 4):** SIZE-FIXED-20-SEEDED is today's sizing and order (FNO-NET's quantity,
   FNO-PORTFOLIO's 20 slots and its order S81), with only the account readings every policy shares (the margin limit below, and
   the queued-entry readings OP-56 and OP-59 of SPEC section 20); SIZE-FIXED-20-RANKED orders the same candidates by a learned
   edge; SIZE-SPLIT-ALL splits the free margin of one second equally; SIZE-RISK sizes by risk with an open-risk cap.
   - **The operator's hard limits** (OI:13: margin in use <= Rs 20,00,000 and exposure <= Rs 1,00,00,000, "at every instant")
     are checked at every entry for all four, on positions marked at their last traded price (SPEC 11.3). The margin limit is
     `min(capital, Rs 20 lakh)`: carried losses still lower it (OP-21), carried profit never lifts it (`Limits::margin_cap`;
     FNO-PORTFOLIO's margin check reads the same limit once AMENDMENTS P1 lands, fno_portfolio.rs:1281-1285 reads the capital
     alone today). SPLIT-ALL's and RISK's shares are cut from that limit and from the exposure room, so no second's rows together
     pass either.
   - **Between entries a mark can carry either figure past its limit,** and SPEC 11.3 forces no reduction for it (SPEC:674; the
     one exception, Dhan's MTM rule of SPEC 9.8, acts on a position's loss, not on these limits; fno_portfolio.rs:909-927, whose
     test asserts "a mark may pass the cap" at :2424). So FNO-RUN reports each policy's peak marked margin in use and peak
     marked exposure and the seconds above each limit, and whether "at every instant" asks for forced cuts is put to the
     operator (section 12; SPEC section 20 OP-58).
4. **FNO-PORTFOLIO stays the one owner of the five caps and of the order S81.** FNO-SIZE returns a plan (who is offered,
   with what quantity, in what order); FNO-PORTFOLIO admits or refuses each entry as today. Only RANKED is offered in the
   plan's order; the other three go through `Account::schedule_second` (fix/fib-fno-portfolio:crates/runner/src/fno_portfolio.rs:992).
5. **Two things outside this part are required before SPLIT-ALL and RISK can run** (AMENDMENTS.md): FNO-PORTFOLIO's
   20-slot table becomes a per-account slot cap (`slots: [Slot; POSITION_SLOTS]`, fno_portfolio.rs:737, with
   `POSITION_SLOTS = 20`, :73), and FNO-RUN walks every policy. The same FNO-PORTFOLIO repair (AMENDMENTS P1) makes its margin
   check read the Rs 20 lakh limit of item 3, which the FIXED pair needs, and adds a read of a stock's lock, from which FNO-RUN
   builds `busy` (the lock table is private today, fno_portfolio.rs:1164-1180). None of this touches this part's files.
6. **The Winner Gate does not move.** Its minimum returns stay on the Rs 20 lakh base for every policy (SPEC:781, OP-22 at
   SPEC:1085; `CAPITAL_PAISA` in fix/fib-fno-wft:crates/runner/src/fno_wft/gate.rs:47 and :455). The capital actually needed
   (section 5) is shown beside it, as OP-22 already says of the 19 Oct base; it can never loosen a check, because it is at
   most Rs 20 lakh under the margin check (5.3).
7. **Decision ids:** D-8630..D-8639 (`W/state/decision-blocks.json`, block "D-8630..D-8639", owner FNO-SIZE; free_from D-8640).
   Ids: invariants `FNO-SIZE-Inn`, benches `C-FNO-SIZE-nn`, limits `L-FNO-SIZE-nn` (the FP:249 form).

---

## 1. Goal and place in the run

- **Goal:** answer the operator's question of 3 Nov ~16:5x IST (OI:3, "if 50 or 100 trades can happen at the same time, how
  do we choose 20? ... or ... no slot limit and split the capital") with data: the sizing and admission policy becomes a
  named run dimension, setting **S82** (form V, SPEC:835: "All are run, never one silently"), and every policy is walked,
  judged by the Winner Gate on its own and reported side by side (AMENDMENTS.md, FNO-RUN and SPEC).
- **First run:** yes, as a dimension of FNO-RUN's P6 (FP:114-117). A (recipe, policy) pair is one candidate of the
  walk-forward family; the luck accounting counts every policy (AMENDMENTS.md, FNO-WFT).
- **Family and wave:** F2, before FNO-RUN (WIRING.md: `F2 = [..., 'FNO-API', 'FNO-SIZE', 'FNO-RUN']`). Only FNO-RUN calls it.
- **Not this part's:** the fills (FNO-SEC-EXEC), the FIXED quantity (FNO-NET's `size`, fix/fib-fno-net:crates/runner/src/fno_net.rs:369),
  the caps and S81 (FNO-PORTFOLIO), the leverage readings (FNO-LEVERAGE's `leverage`, fix/fib-fno-leverage:crates/pull/src/nse_margin.rs:1192),
  form-G generation and the gate (FNO-WFT), the walk and every adapter (FNO-RUN, FP:977). FNO-SIZE emits no telemetry (gate 17, FP:268)
  and writes no run output, record or route: the F2 RULES' label clause and N4 clause (F3:67-68) bind FNO-RUN, which carries
  FNO-SIZE's labels beside its own.

---

## 2. Files and anchors (exact base lines at SEAM; one owner per file)

| File | N/E | What |
|---|---|---|
| `crates/runner/src/fno_size/mod.rs` | N | `Policy`, `PolicyInput`, `Side`, `StockId`, `StockSet`, `Fraction`, `Candidate`, `AccountView`, `Limits`, `Plan`, `PlanRow`, `Outcome`, `Sized`, `Bound`, `SizeRefusal`, `Invalid`, `plan`, the labels, `VERSION` |
| `crates/runner/src/fno_size/share.rs` | N | SPLIT-ALL's and RISK's share arithmetic, exact |
| `crates/runner/src/fno_size/order.rs` | N | RANKED's comparator, the seeded key, the fixed network |
| `crates/runner/src/fno_size/edge.rs` | N | `EdgeTable`, `EdgeTrade`, `Edge`, `Level` |
| `crates/runner/src/fno_size/risk.rs` | N | `RiskBudget`, `RiskBook` |
| `crates/runner/src/fno_size/capital.rs` | N | `CapitalNeeded` |
| `crates/runner/src/fno_size/measure.rs` | N | `pub fn measure_*` and the bench verdict (FP:267: the measuring function lives in the library) |
| `crates/runner/src/fno_size/tests.rs` | N | unit and property tests (`#[cfg(test)] mod tests;` in `mod.rs`) |
| `crates/runner/benches/ratio/fno_size.rs` | N | the thin wrapper: `pub(crate) fn row(ids: &[&str]) -> bool` (the form of SEAM:crates/runner/benches/ratio.rs:599-601) |
| `crates/runner/src/lib.rs` | E | one line, at its anchor below |
| `crates/runner/benches/ratio.rs` | E | two insertions, at its anchors below |
| `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md` | E | one block each, at its anchor below |

FNO-SIZE has no seam slot: FNO-SEAM's slot lists end at FNO-WFT in the runner files (SEAM:crates/runner/src/lib.rs:84-102,
SEAM:crates/runner/benches/ratio.rs:64-82 and :602-611) and at FNO-BAND-FILES in the ledgers (SEAM:docs/05-decisions.md:87649).
So it takes these anchors (FP:1350: "A fib part that must edit a file ... not named here or under its slot lines, stops and asks
the main session for an anchor"; this section is that answer once appended to FP). Each anchor line is unique in its file at SEAM.

| File | Insert directly after this exact line (SEAM line no.) | Inserted text | Nearest other edit (read on every fib branch, `git diff -U0 SEAM <tip>`) |
|---|---|---|---|
| `crates/runner/src/lib.rs` | `pub mod family_allocation_v1;` (:62) | `pub mod fno_size;` | FNO-BAND after :84 (fix/fib-fno-band); 22 unchanged lines between (:63-:84). Alphabetical inside its group (`family_allocation_v1` < `fno_size` < `grid`, :63), so rustfmt's module order holds |
| `crates/runner/benches/ratio.rs` | `use vocab::ConditionMask;` (:59) | a blank line, `// FNO-SIZE (state/fno-size-20261103/PARTS-FNO-SIZE.md): its bench module.`, `#[path = "ratio/fno_size.rs"]`, `mod fno_size;` | FNO-BAND after :64; lines :60-:64 unchanged between |
| `crates/runner/benches/ratio.rs` | `        the_equity_column_build_stays_within_its_budget(),` (:589, eight spaces) | the row call `fno_size::row(&["C-FNO-SIZE-01", "C-FNO-SIZE-02", "C-FNO-SIZE-03", "C-FNO-SIZE-04", "C-FNO-SIZE-05", "C-FNO-SIZE-06", "C-FNO-SIZE-07"]),` as rustfmt lays it out: 144 characters on one line pass the default width of 100 (SEAM has no rustfmt.toml), so `cargo fmt` writes it as nine lines, `fno_size::row(&[`, one id a line, `]),`; the ids and their order are what count (cost_invariants.rs reads string literals, SEAM:crates/core/tests/cost_invariants.rs:242-250) | FNO-BAND after :602; the seam's comment block :590-:601 unchanged between |
| `docs/04-invariants.md` | the F&O section's comment line (:9111, starts `<!-- One slot line per F&O strategy part`) | a blank line, `<!-- slot FNO-SIZE (D-8630..D-8639; state/fno-size-20261103/PARTS-FNO-SIZE.md) -->`, its table | FNO-LAW after :9114 (fix/fib-fno-law); :9112-:9114 unchanged between |
| `docs/05-decisions.md` | the F&O section's comment line (:87601) | a blank line, the same slot comment, its entries D-8630.. | FNO-LAW after :87604; :87602-:87604 unchanged between |
| `docs/06-limits.md` | the F&O section's comment line (:22147) | a blank line, the slot comment, its table | FNO-COSTS after :22152 (fix/fib-fno-costs; FNO-LAW writes no docs/06 row); :22148-:22152 unchanged |
| `docs/07-plan.md` | the F&O section's comment line (:1125) | a blank line, the slot comment, its plan table | FNO-LAW after :1128; :1126-:1128 unchanged |

Why these lines:
- **Never adjacent to another part.** In a scratch repository, insertions under different lines merged clean even one line
  apart, and two appends at the end of one file conflicted (FP:159). The 1 Nov rule for cross-family anchors asks for at least
  two unchanged base lines (FP:1308); every anchor above has two or more.
- **No base part edits them.** `git diff --stat f01d7a0e4 <tip>` on `fix/db-BASE-REDS-1023` f424b6ee285e, `-STACK-1028`
  2f136e0c2bed, `-BENCH-1028` 37d7f98661ed and `-COV-LIBS` bd04fb299d23 (and its cloud shard `origin/cloud/cov-libs-ls2-20261103`
  074584f3e538) lists no change to `crates/runner/src/lib.rs` or `crates/runner/benches/ratio.rs`; their ledger rows were
  written on bases without the seam's F&O section, so none can anchor inside it, and PARTS 7 already settles their end-of-file
  conflict at F1's landing (FP:1122). The data parts were cut from `bcac15e2c` (FP:1308), also without that section.
- **Checked against the PSRB parts,** designed at the same time with "per-part distinct anchors" (`W/state/strategy-psrb-20261103/PARTS.md`,
  885 lines at ~17:3x IST, its section 2.2): PSRB-RSI, -BOLL and -STV anchor in `crates/indicators/src/lib.rs` (:63, :65, :70), in
  `crates/indicators/benches/ratio.rs` (a PSRB-SEAM commit) and in the ledgers after the blank line above their own headings
  (docs/04 :8950, :9015, :9061; docs/05 :84675, :86591, :86992; docs/06 :21954, :22023, :22066; docs/07 :982, :1057, :1109). No line
  is shared; the nearest is 16 lines away (docs/07 :1109 against :1125). PSRB-RSI's docs/05 anchor was :85248 until PSRB's repair 1
  moved it above D-7272 (:84676), read again at repair 2 (~19:2x IST, its PARTS.md:138, :158, :163, 1,072 lines then); either line
  is far above FNO-SIZE's :87601. PSRB-STRAT and PSRB-RUN have no anchors yet: START.md step 1 repeats the check when they get them.
- **The module line is outside the seam's F&O block on purpose.** Inside the block every free line is at most one line from a
  slot (SEAM:crates/runner/src/lib.rs:82-102: comment, slot, blank, slot, ...). The main list is sorted, and no branch that lands
  with FNO-SIZE edits it: `git diff -U0 SEAM <tip> -- crates/runner/src/lib.rs crates/runner/benches/ratio.rs` on every
  `fix/fib-*` tip and every `origin/cloud/fib-fno-*` tip (repair 1, ~18:2x IST) shows hunks only after slot lines (lib.rs
  :84-:102, ratio.rs :64-:82 and :602-:611); no base or data branch edits either file (above). `git log --all f01d7a0e4.. -- crates/runner/src/lib.rs`
  also lists older unmerged branches (59 commits, August to October, an 18 Oct FIB-1 WIP among them), none of which lands.
- **Decision citations.** Code under `crates/` may cite only D-numbers that head an entry of `docs/05-decisions.md` on the same
  tree (`crates/core/tests/decision_citations.rs`:1-3 at SEAM). FNO-PORTFOLIO's D-8180..D-8183 and FNO-NET's D-8230.. are not on
  SEAM, so FNO-SIZE's code and its ledger rows cite only D-8630..D-8639, and name other parts and files in words.

---

## 3. Public API (all integers; paise `i64`/`i128`, ppm `u32`, exact ratios compared by cross-multiplying in `u128`/`i128`)

```rust
pub const VERSION: &str = "fno-size/v1";            // joins the candidate identity through Policy::canonical_bytes
pub const UNIVERSE: usize = 256;                    // the 218 padded, as fno_portfolio::STOCKS (fno_portfolio.rs:85)
pub const SESSION_MINUTES: u16 = 375;               // minute-of-session 0 (09:15) ..= 374 (15:29)
pub const MARGIN_FLOOR_PPM: u32 = 200_000;          // SPEC 11.3 max(0.20, M/100); the caller already applied it

pub enum Policy { FixedSeeded, FixedRanked, SplitAll, Risk }
impl Policy {
    pub const ALL: [Self; 4];
    pub const fn name(self) -> &'static str;        // "SIZE-FIXED-20-SEEDED" | "SIZE-FIXED-20-RANKED" | "SIZE-SPLIT-ALL" | "SIZE-RISK"
    pub const fn label(self) -> &'static str;       // section 6, byte for byte
    pub const fn offer(self) -> Offer;              // AccountOrder for all but FixedRanked: PlanOrder
    pub const fn twenty_slots(self) -> bool;        // true for the two FIXED policies
    pub const fn reads_capital(self) -> bool;       // true for SplitAll and Risk (section 5)
    pub fn canonical_bytes(self) -> Vec<u8>;        // VERSION, 0x00, name: the candidate identity term (AMENDMENTS W1)
}
pub enum Offer { AccountOrder, PlanOrder }
pub enum Side { Long, Short }
pub struct StockId(u8);                             // position in fno218.txt, the meaning of fno_portfolio::StockId (:118-125)
pub struct StockSet([u64; 4]);                      // 256 bits
pub struct Fraction { num: u64, den: u64 }          // den > 0; Ord by value

pub struct Candidate {
    pub stock: StockId,
    pub recipe: u32,          // 0 alone, the pick position in a procedure (fno_portfolio.rs:275-277's meaning)
    pub side: Side,
    pub start: i64,           // the order's start second; equal for every candidate of one batch
    pub minute: u16,          // minute of session of `start`, 0..=374
    pub price: i64,           // P_s in paise, raw (SPEC 11.2, OP-15 at SPEC:1078)
    pub rate_ppm: u32,        // max(200_000, the largest reading's M ppm) = 10^6 / the smallest leverage (nse_margin.rs:1189-1202)
    pub fixed_qty: u64,       // FNO-NET's quantity for N = Rs 1 lakh x L (FIXED policies; read by no other policy)
    pub stop: Option<i64>,    // the protective stop's level in raw paise after the mapping reading (FIB-I8); None on a stop-less lane
    pub stop_in_r: Fraction,  // the stop's distance as a fraction of R, the ladder's range (fno_portfolio.rs:228-230's meaning)
    pub liquidity: u128,      // session-to-date traded value in paise to the decision candle's close (D-8635)
}
pub struct AccountView {
    pub capital: i128,            // paise: Rs 20 lakh + the settled carried net (Account::capital, fno_portfolio.rs:1026)
    pub margin_in_use: i128,      // millionths of a paisa (Account::margin_in_use().micro_paisa(), :1045, :459-463)
    pub exposure: i128,           // paise (Account::exposure, :1039)
    pub reserved_margin: i128,    // millionths of a paisa: entries the order-rate rings queued, not yet sent (FNO-RUN's)
    pub reserved_exposure: i128,  // paise: the same entries
    pub trades_today: u16,        // Account::trades_today (:1070)
    pub queued: u16,              // entries queued, not yet sent; each will take a trade when sent (SPEC 11.4, OP-18)
    pub open_risk: i128,          // paise: RiskBook::open_risk plus the queued entries' planned risk (SIZE-RISK only)
    pub busy: StockSet,           // stocks with a position in force or a queued entry (SPEC 9.6)
}
pub struct Limits { pub margin_cap: i128, pub exposure_cap: i128, pub trades_per_day: u16 }
// paise, paise, entries. FNO-RUN passes the operator's Rs 20,00,000 (fno_portfolio's MARGIN_CAP_PAISA once AMENDMENTS P1 lands;
// the number of CAPITAL_PAISA, :66), EXPOSURE_CAP_PAISA (:70) and TRADES_PER_DAY (:76)
pub enum PolicyInput<'e> {
    FixedSeeded,
    FixedRanked { edge: &'e EdgeTable, seed: u64 },
    SplitAll,
    Risk { budget: Option<RiskBudget> },            // None: the learning window had no trade (section 4.4)
}

pub fn plan(input: &PolicyInput<'_>, view: &AccountView, limits: &Limits,
            batch: &[Candidate], out: &mut Plan) -> Result<(), Invalid>;

pub struct Plan {
    pub policy: Policy, pub offer: Offer,
    pub rows: Vec<PlanRow>,       // one per candidate; PlanOrder: in offering order; AccountOrder: batch order
    pub eligible: u16,            // distinct non-busy stocks
    pub k: u16,                   // the share divisor (SPLIT-ALL, RISK), 0 otherwise
    pub share_margin: i128,       // paise, floor(F / k)
    pub share_exposure: i128,     // paise, floor(X / k)
    pub risk_each: i128,          // paise, b (RISK)
    pub seed_reached: u32,        // RANKED comparisons that fell through to the seeded key (section 4.2)
}
pub struct PlanRow { pub index: u16, pub outcome: Outcome }   // index into `batch`
pub enum Outcome { Sized(Sized), Refused(SizeRefusal) }
pub struct Sized { pub qty: u64, pub value: i128, pub margin: i128, pub risk: Option<i128>, pub bound: Bound }
pub enum Bound { FixedTarget, MarginShare, ExposureShare, Risk }
pub enum SizeRefusal { WhileOpen, NoFreeMargin, NoExposureRoom, BelowOneShare, NoLearnedRisk, NoRiskRoom, NoStop, ZeroStop, StopThroughPrice }
pub enum Invalid { BatchTooLarge(usize), MixedStart { first: i64, other: i64 }, Duplicate { stock: u8, recipe: u32 },
                   NonPositivePrice(i64), RateBelowFloor(u32), MinuteOutOfSession(u16), ZeroFixedQuantity { stock: u8 },
                   ZeroDenominator, Overflow(&'static str), NegativeView(&'static str), BadRungs, MonthOutsideLearning { month: u16 },
                   MonthsOutOfOrder, NonPositiveValue, StockInForce(u8), NotInForce(u8), TimeBackwards { at: i64, clock: i64 },
                   ReleaseBeforeAdmit { after: i64, admitted: i64 }, NonPositiveBudget }
```

- `plan` does a fixed amount of work per candidate: O(n) for n <= 256, plus RANKED's network (L-FNO-SIZE-01) and the duplicate check's comparisons among candidates of one stock (section 4, L-FNO-SIZE-03; at most 32,640 a batch).
  It allocates nothing when `out.rows` already holds `UNIVERSE` capacity (FNO-RUN keeps one `Plan` per walk).
- Every refusal word is `SizeRefusal::word()`: `while_open` (SPEC 9.6's own name), `size_no_free_margin`,
  `size_no_exposure_room`, `size_below_one_share`, `size_no_learned_risk`, `size_no_risk_room`, `size_no_stop`,
  `size_zero_stop`, `size_stop_through_price`. None collides with FNO-PORTFOLIO's five (`position_cap`, `trade_cap`,
  `exposure_cap`, `margin_cap`, `while_open`: fno_portfolio.rs:341-345), and `while_open` means the same thing in both.
- `Invalid` is a caller defect, refused whole, changing nothing (`out` untouched), as FNO-PORTFOLIO's `Invalid` (fno_portfolio.rs:405-451).

**How FNO-RUN calls it, per second `s` of one walk (the adapter is FNO-RUN's, AMENDMENTS R2):** settle the account and the risk
book at `s`; mark the held positions at their last traded price (SPEC 11.3); build `AccountView` from the account's accessors plus
its own queue (`busy` from the account's own lock table through the reader AMENDMENTS P1 adds, plus the stocks of its kept queued
entries); build one `Candidate` per entry intent whose start is `s`; `plan`; then for `AccountOrder` pass the second's exits
and the plan's `Sized` rows to `schedule_second`, and for `PlanOrder` pass the exits to `schedule_second` and then each `Sized`
row to `admit` in row order. A `Refused` row is counted under its word and never offered. After each admission:
`CapitalNeeded::observe`, and under RISK `RiskBook::admit`. Then FNO-RUN keeps a queued entry's reservation only within the
day's trade room and only one a stock (R2, SPEC section 20 OP-59), so `room`, `F`, `X` and `Rr` count no entry that cannot go, and
it drops every kept entry still unsent at the 15:10:00 square-off (SPEC 9.8: "Pending and partial orders are cancelled"), counted,
so no reservation passes into the next session.

---

## 4. The rules, exactly

Common to all four, for one batch (the entry intents of one second):

- **The decision instant is one order start second `s`** (D-8631): the candidates FNO-PORTFOLIO would sort into one batch
  (SPEC 11.5 rule 1 at SPEC:685, "Entries are queued by the second they may start"; `schedule_second` takes "every order that
  wants to go in one second", fno_portfolio.rs:25-31). A batch whose candidates differ in `start` is `Invalid::MixedStart`.
  Consequence, stated in every report: two signals of one candle whose stocks first trade in different seconds belong to two
  instants, the earlier first, exactly as SPEC 11.5 already orders them.
- **Busy stocks:** a candidate whose stock is in `view.busy` (a position in force through its last fill second, or an entry
  queued by the order-rate rings: SPEC 9.6 locks "from the entry order's start") is `Refused(WhileOpen)`, counted under SPEC
  9.6's name. Two candidates on one stock in one batch (two recipes of a procedure, SPEC 11.6) are both sized, the stock
  counted once in `eligible`; FNO-PORTFOLIO admits the first in its order and refuses the other `while_open`. When the rings
  queue the first, FNO-PORTFOLIO queues the other too (it locks a stock only at admission, fno_portfolio.rs:1233, :1270), and
  FNO-RUN then refuses the other `while_open` at once (SPEC 9.6, OP-56, OP-59; AMENDMENTS R2): one stock never holds two
  reservations.
- **Validation:** at most `UNIVERSE` candidates; distinct (stock, recipe); `price > 0`; `rate_ppm >= MARGIN_FLOOR_PPM`;
  `minute < 375`; under a FIXED policy `fixed_qty >= 1` (FNO-NET already refused `q = 0` as `price_above_position`,
  fno_net.rs:286-287 and :428, so a zero here is a caller defect); every product checked in `i128`/`u128`, an overflow refused by name;
  the view's `margin_in_use`, `exposure`, `reserved_margin`, `reserved_exposure` and `open_risk` each at least 0, else
  `NegativeView` naming the field (each is a sum of positive amounts, so a negative one is a caller defect, and it would lift `F`,
  `X` or `Rr` past the limits); `capital` may be 0 or negative (carried losses past it: every eligible row `size_no_free_margin`,
  4.3 step 3).
- **The duplicate check, without a sort or a map** (gate 11): a per-stock chain over the batch, two fixed arrays
  `first: [u16; UNIVERSE]` (by stock) and `next: [u16; UNIVERSE]` (by candidate), each candidate compared only with the earlier
  candidates of its own stock. That is `c (c - 1) / 2` recipe comparisons for `c` candidates on one stock: in a valid batch a
  stock's candidates are its recipes, one each, so `c` is at most the procedure's picks; at the universe's worst (256 candidates
  on one stock) `256 x 255 / 2 = 32,640` a batch. Bounded by the universe, measured by C-FNO-SIZE-07, recorded in L-FNO-SIZE-03.

### 4.1 SIZE-FIXED-20-SEEDED (today's sizing and order)

- Quantity: `fixed_qty`, FNO-NET's integer closest to `Rs 1,00,000 x L / P_s` with the tie down (SPEC 11.2 at SPEC:645-660;
  `quantity`, fno_net.rs:313; `size`, :369). FNO-SIZE re-derives nothing: one implementation of SPEC 11.2.
- `Sized { qty: fixed_qty, value: qty x P_s, margin: qty x P_s x rate_ppm, risk: None, bound: FixedTarget }` for every
  non-busy candidate. `Offer::AccountOrder`: FNO-PORTFOLIO's 20 slots, 100 trades, Rs 1 crore, margin and its S81 order with
  the seeded (e) and k seeds (SPEC 11.4-11.5; fno_portfolio.rs:1383-1431), as today but for the account readings every policy
  shares: the margin limit of section 0 item 3 (AMENDMENTS P1) and the queued-entry readings OP-56 and OP-59 (AMENDMENTS R2;
  today FNO-PORTFOLIO locks a stock only at admission, fno_portfolio.rs:1233).

### 4.2 SIZE-FIXED-20-RANKED

- Quantity as 4.1. The rows are ordered, `Offer::PlanOrder`, by this strict total order (D-8634):
  1. **edge**, higher first: `edge.score(stock, side, minute)` (4.5). A score with no trade behind it ranks after every
     score with one; two such tie here.
  2. **stop in R**, smaller first (the key of SPEC 11.5 (b), SPEC:688).
  3. **liquidity**, larger first.
  4. **the seeded key**, smaller first: `seeded(seed, start, stock, recipe)`, the formula of FNO-PORTFOLIO's tie-break (e),
     restated byte for byte (fno_portfolio.rs:1413-1431: SplitMix64's step and finaliser over the seed, then `^ start`, `^ stock`,
     `^ recipe`, each followed by the mix), so one seed draws one permutation in every policy. `plan.seed_reached` counts the
     comparisons that reached this level.
  5. **stock** position, smaller first (symbol order, SPEC 11.5 (d)).
  6. **recipe**, smaller first.
  Distinct (stock, recipe) make it total. The edge only orders; it refuses nothing (a negative edge is still offered: a filter
  would be another variant, and the operator named four).
- **The order is a fixed network, not a sort** (gate 11 rule 4 bans `.sort*(` on production lines,
  SEAM:.github/gates_ledger.rs:2157-2178, unless a file is allowlisted there, and FNO-SIZE edits no `.github` file): Batcher's
  bitonic network over the next power of two `P >= rows` (at most 256, padded with sentinels that compare last), the same
  compare-exchanges for every batch of that `P` (at most `128 x 8 x 9 / 2 = 4,608` at 256), the shape FNO-RANK already uses
  over 256 slots (FP:557). Bounded by the universe, measured by C-FNO-SIZE-03, recorded as L-FNO-SIZE-01.
- FNO-PORTFOLIO's 20 slots, 100 trades, exposure and margin then apply in this order: with 20 in force the 21st by rank is
  refused `position_cap`, and once the margin left no longer covers a row, that row is refused `margin_cap`, in rank order; a
  later, smaller row can still fit and be admitted (fno_portfolio.rs:1245, :1281-1285).
- **The account's tie-break (D-8634):** RANKED's account is `Account::new(capital, TieBreak::Symbol)` (AMENDMENTS R3). Its
  entries never reach the account's sort, because they go to `admit` one by one in the plan's order; its exits do
  (`schedule_second` sorts every intent by the account's tie-break, fno_portfolio.rs:1002-1004, :1358-1365, :1383-1396), and
  that order decides which exits the 10-a-second ring sends first and which it queues (:1010, :1304-1310, `ORDERS_PER_SECOND`
  :95). Symbol order, SPEC 11.5 (d), draws nothing from the seed, so the seed reaches a RANKED walk only through the plan's
  fourth key. Not chosen: `TieBreak::Seeded(seed)`, which would make every RANKED walk depend on the seed through its exits
  alone and always cost k walks.
- **Seeds and luck:** RANKED is walked under the run's k seeds (the seeded key is its fourth level). A network's
  compare-exchanges are fixed, and the account orders RANKED's exits by symbol, so a walk whose `seed_reached` stayed 0 at every
  second made every decision without the seed and is identical under every seed (by induction over its seconds: the same
  account state, the same batch, the same plan, the same outcomes). FNO-RUN may then walk it once and report its spread as 0,
  "no tie reached the seeded level" (AMENDMENTS R5). This is exact, not an estimate; FNO-RUN's check proves it on the real
  account with more than ten exits in one second (AMENDMENTS R7).

### 4.3 SIZE-SPLIT-ALL (D-8632)

For the batch at second `s`:
1. `e` = distinct non-busy stocks of the batch. If `e = 0`, every row is `WhileOpen` and nothing else happens.
2. `room = trades_per_day - trades_today - queued`, saturating at 0. `k = min(e, room)` when `room > 0`, else `k = e`: every
   row is then refused `trade_cap`, by FNO-PORTFOLIO once the day's 100 entries are sent or by FNO-RUN when the rings queue it
   while the kept queued entries hold the rest of the day (SPEC section 20 OP-59), unless a queued entry is refused when offered
   again and frees its trade; `k = e` keeps the quantities defined either way.
3. Free margin `F = floor((min(capital, margin_cap) x 10^6 - margin_in_use - reserved_margin) / 10^6)` paise: the operator's
   Rs 20 lakh limit, or the capital when carried losses leave less (section 0 item 3). `F <= 0`: every eligible row
   `Refused(NoFreeMargin)`.
4. Exposure room `X = exposure_cap - exposure - reserved_exposure` paise. `X <= 0`: every eligible row `Refused(NoExposureRoom)`.
5. Equal shares: `m = floor(F / k)`, `x = floor(X / k)`. The remainders `F - k m` and `X - k x` stay free.
6. Per eligible row: `q_m = floor(m x 10^6 / (rate_ppm x price))` (the stock's leverage on its margin share, so leverage below
   1x, `rate_ppm > 10^6`, gives less value than margin), `q_x = floor(x / price)`, `q = min(q_m, q_x)`, `bound = MarginShare`
   when `q_m <= q_x`, else `ExposureShare`. **Floor, not SPEC 11.2's nearest integer:** the share is a budget, and a rounded-up
   quantity could pass the free margin or the exposure room and get the last row refused by an artefact of rounding.
7. `q = 0`: `Refused(BelowOneShare)`, and its share stays free for later seconds (**no re-split**: a re-split would change
   every other row's share after the fact and need more than one pass).

**The minimum position SPLIT-ALL allows is one whole share,** and nothing higher. Why: one share is the exchange's minimum
order and SPEC 11.2's own refusal (`price_above_position`, SPEC:660); a typed floor in rupees would be a static number the
operator's rule 1 forbids (F3:41, "NOTHING STATIC"). The cost of small positions is not hidden: below Rs 66,667 brokerage is
0.03% a leg and above it a flat Rs 20 (OI:7), and every flat charge (the per-order rupee roundings, the square-off fee, any flat
DP reading: SPEC 12 at SPEC:709-720) is priced per trade by FNO-NET under the worse clock, so a position too small to pay its
charges shows as a loss. FNO-RUN's report gives charges per trade and as a share of value per policy (AMENDMENTS R6).

**Later signals once margin is in use:** a later second's batch splits only what is free at that second: `F` already holds
every position in force at its last mark, the carried net (the limit is `min(Rs 20 lakh plus the settled net, Rs 20 lakh)`:
OP-21 lowers it after losses and the operator's item 5 caps it, fno_portfolio.rs:1022-1029) and every kept queued entry's
reserved share (no more of them than the day's trades left, one a stock, OP-59). Margin comes back only when a position settles
after its last fill second (FNO-PORTFOLIO's reading, fno_portfolio.rs:40-48). So an early burst can hold every rupee and later
signals are refused `size_no_free_margin`
until exits free some: that is the policy as the operator worded it ("capital split equally across all concurrent signals",
OI:7), and its refusals are counted, never hidden. A reserve for later signals would be a fifth policy (and a trial), not
built.

**Hard limits, proved at every entry:** every row sized here has `margin <= m x 10^6` and `value <= x`, so any `j <= k` of
them admitted together hold at most `F x 10^6` of margin and `X` of exposure, and margin in use, reservations included, stays at
most `min(capital, margin_cap) x 10^6`. FNO-PORTFOLIO's margin check (`margin + extra > capital x PPM`,
fno_portfolio.rs:1281-1285, reading `min(capital, Rs 20 lakh)` once AMENDMENTS P1 lands) and exposure check (:1242-1244)
therefore never refuse a SPLIT-ALL row of one second, and the operator's limits (OI:13) hold at each of its entries; between
entries a mark can carry either figure past its limit (section 0 item 3). When `e > room`, the rows take the room in S81 order
(admitted, or queued and kept by FNO-RUN) and every later row is refused `trade_cap` (by FNO-PORTFOLIO, or by FNO-RUN when the
rings queued it, OP-59), each row with the share computed for `k = room`.

**Which share binds:** with the operator's limits (`exposure_cap = 5 x margin_cap`) and every rate at or above 20%, margin in
use and reserved margin are each at least 20% of their exposure, so `5 F <= X`, `5 m <= x` and `q_m <= q_x`: the bound is
always `MarginShare`, and `size_no_exposure_room` never fires (`X >= 5 F > 0` whenever step 3 passes). `ExposureShare` and that
refusal are reached only under `Limits` whose exposure cap is below five margin caps, or a view whose margin is under 20% of its
exposure (the tests use both, so every branch is covered); they stay because `Limits` and the view are inputs and the exposure
room is the second limit's own proof.

`Offer::AccountOrder`, under an account whose slot cap is the day's 100 entries (AMENDMENTS P1): with one trade at a time per stock
(SPEC 9.6, SPEC:642), at most 100 entries sent a day and everything closed by the session end, no more than 100 positions are ever
open at once (SPEC 11.4, OP-18).

### 4.4 SIZE-RISK (D-8633)

Steps 1-5 of 4.3 (busy rows, `room`, `k`, `F`, `X`, `m`, `x`), then:
6. `budget = None` (the learning window's SEEDED walk had no trade with a stop at a positive distance): every eligible row
   `Refused(NoLearnedRisk)`.
7. Risk room `Rr = budget.open_cap - open_risk`. `Rr <= 0`: every eligible row `Refused(NoRiskRoom)`.
8. Risk each `b = min(budget.per_trade, floor(Rr / k))`: equal risk per trade, and when the open-risk cap binds, the
   second's rows share what is left equally (no order is needed, so FNO-PORTFOLIO stays the only owner of S81).
9. Per eligible row: `stop = None`: `Refused(NoStop)` (a stop-less lane is only a labelled comparison, FP:683, SPEC:880);
   distance `d = price - stop` (long) or `stop - price` (short); `d = 0`: `Refused(ZeroStop)`; `d < 0`:
   `Refused(StopThroughPrice)`. Else `q_r = floor(b / d)`, `q = min(q_r, q_m, q_x)` with `q_m`, `q_x` as 4.3 step 6
   ("capped by leverage", OI:8: the stock's leverage on its equal share of the free margin), `bound = Risk` when `q_r` is the
   smallest (ties go to `Risk`), else as 4.3; `q = 0`: `Refused(BelowOneShare)`; `risk = q x d <= b`.
10. So the second's admitted risk is at most `k x b <= Rr`: open risk never passes `open_cap`.

**The two budgets, generated from the learning window only** (nothing typed; D-8633): from the same candidate's
SIZE-FIXED-20-SEEDED walk over that window's learning months, under tie-break (e) with the run's first seed (the walk RANKED's
edge table also reads, 4.5):
- only its trades with a stop at a positive distance count: `d > 0` at `P_s`, with `d` as step 9 measures it. A stop-less
  trade, or one whose stop sat at or through `P_s`, has no initial risk and is left out of both budgets;
- `per_trade` = the median initial risk `q x d` of those trades, the lower one of an even count (FNO-WFT's median rule,
  fix/fib-fno-wft:crates/runner/src/fno_wft/generate.rs:260-272); FNO-RUN computes it with `generate::Histogram`
  (generate.rs:21-89), because a median needs an order and FNO-SIZE sorts nothing;
- `open_cap` = the peak open risk that walk held: `RiskBook::peak` over the walk, the book admitting those trades only;
- no such trade: `Risk { budget: None }` (step 6).
So SIZE-RISK takes today's typical risk per trade and today's peak total risk, and only redistributes them by stop distance.
With at least one such trade `per_trade >= 1` paisa (`q >= 1`, `d >= 1`) and `open_cap >= per_trade`, so `RiskBudget::new`'s
refusal of a budget at or below 0 (`NonPositiveBudget`) stays a caller defect that a correct FNO-RUN never meets.

**Open risk** is the sum of the initial risks of positions in force, each held until FNO-PORTFOLIO would free its slot: through
its last fill second, settled at the first call of a later second (fno_portfolio.rs:40-48). `RiskBook` (risk.rs) keeps it, one
row per stock (`UNIVERSE` rows, as FNO-PORTFOLIO's lock table) and a list of the released rows ordered by `after`:
- `admit(at, stock, risk)`: `StockInForce` for a stock held, or released and not yet settled; `TimeBackwards` before the clock.
- `release(stock, after)`: `NotInForce` for a stock not held; `ReleaseBeforeAdmit` for an `after` before its admission second
  (FNO-PORTFOLIO's `ReleaseBeforeSend`, fno_portfolio.rs:964-968). It takes releases **in any order and with a future `after`**,
  exactly as `Account::release` does (fno_portfolio.rs:953-980; that file's random walk releases at `at + 0..4` in any order,
  :2981-2984), so FNO-RUN owes the book no order: the row joins the list at its place by `after`, found by walking back from the
  tail. A release in last-fill order is one step, O(1); any other walks back over at most the 255 other released rows (bounded by
  the universe, L-FNO-SIZE-07, measured by C-FNO-SIZE-04).
- `settle(at)`: frees from the list's head every row whose `after` is before `at`, each row once, so amortized O(1);
  `TimeBackwards` before the clock. Then `open_risk()` and `peak()`.
A stop moved to break-even frees nothing until the position closes (the reading that admits less). FNO-RUN calls the book beside
every account call (AMENDMENTS R2), and its check proves the two agree at every second, releases out of last-fill order included (R7).

`Offer::AccountOrder`, slot cap 100 as 4.3.

### 4.5 The edge table (RANKED's first key; D-8634)

- **Source:** the candidate's trades in the window's learning months from the walk 4.4 names (SEEDED, tie-break (e), first
  seed): a recipe's own trades, or, for a procedure, its picked recipes' trades pooled in one table (an `EdgeTrade` carries no
  recipe), so a procedure's recipes share its cells. Under (e) the admitted trades at a binding second are a seeded draw, so
  they are not chosen by any key that could correlate with their outcome. A trade whose entry filled nothing is not a trade (FNO-NET refuses an order that filled
  nothing, `fix/fib-fno-net` commit 4d87105d0) and is not in the table.
- **Value:** net per rupee of entry value, `floor(net x 10^6 / value)` ppm, computed as `(net x 10^6).div_euclid(value)` in `i128`:
  a floor for a negative net too (Rust's `/` rounds toward zero, so `-10^6 / 3` gives -333,333 where the floor is -333,334). `net`
  is FNO-NET's headline net under the worse clock (`NetTrade::net`, fno_net.rs:571) and `value > 0` the exact sum over the entry's
  fills of quantity x fill price in paise (never a rounded volume-weighted price times the quantity), so a 4x and a 5x stock
  compare. The score of a cell is its mean, `sum / count`, compared exactly.
- **Cells, three levels:** (stock, side, time-of-day bucket), then (stock, side), then (side). `score` returns the finest
  level holding at least one trade (`Level::Cell`, `StockSide`, `Side`), else `Level::None` with count 0. Dense arrays:
  `256 x 2 x B`, `256 x 2`, `2`, so a lookup is three array reads, O(1).
- **Time-of-day buckets, generated:** the rungs are FNO-WFT's form-G quantile ladder of the learning trades' minute of
  session (`quantile_ladder`, generate.rs:153; `Domain::new(0, 374, None, 1)`, :126), computed by FNO-RUN and handed in;
  `EdgeTable::build` refuses rungs that are not strictly ascending or pass 374 (`BadRungs`) and builds a 375-entry minute map.
  No rung is typed here. The rungs are fitted once per window over the entry minutes of all its learning trades, those of any
  learning month later judged included: entry times only, never an outcome, as every form-G setting is fitted over the whole
  learning window (generate.rs:3-5, `from_learning`). Leave one month out (below) takes a judged month's trades out of the
  scores, not out of the bucket edges.
- **Learning months only, never the month it judges:** `build(rungs, learn_from, test_from, trades)` refuses a trade outside
  `[learn_from, test_from)` (`MonthOutsideLearning`) and trades out of month order (`MonthsOutOfOrder`). A forward month is
  judged with the whole learning table. When RANKED walks a learning month `j` (for the pick and the learned references),
  `judging(j)` takes month `j`'s own trades out of all three levels first and puts the previous month's back (leave one month
  out), so no month is ever scored by its own trades. Cost: O(trades of the two months) once per month boundary, amortized
  O(1) per trade (L-FNO-SIZE-02, measured by C-FNO-SIZE-06).
- **One table per window:** FNO-RUN builds one table per (candidate, scheme, window) from that window's learning months
  (AMENDMENTS R4), and a RANKED walk reads only its own window's table, `judging` included. So a RANKED walk is never shared
  between windows or schemes, even when its capital never binds: READINESS item 5's shared rows (FP:1593) rest on fixed
  quantities, and RANKED's order also reads the window. A month that is one window's forward month and another window's learning
  month is ordered by the first window's whole table in the first, and by the second's table without that month in the second;
  a walk shared between them would let the month's own trades rank its own signals (AMENDMENTS R8).

### 4.6 What FNO-SIZE never does

It never re-sizes a position after its start (SPEC 9.2), never forces a reduction (SPEC:674), never reads a forward month for
a setting, never changes a Winner Gate number, and never re-implements FNO-NET's SPEC 11.2 rounding, FNO-PORTFOLIO's checks or
FNO-WFT's form-G generation.

---

## 5. The capital actually needed, per policy (for the gate's return checks; D-8636)

### 5.1 The rule

| Policy | Capital actually needed K | Why |
|---|---|---|
| SIZE-FIXED-20-SEEDED, SIZE-FIXED-20-RANKED | `K = max over every admission of (ceil(margin in use right after it / 10^6) - carried net settled then)`, over every window walk the gate reads (learning and unseen months; each starts at Rs 20 lakh, FP:1591) | quantities never read capital (SPEC 11.1-11.2; FP:1593), so K is the smallest starting capital with which every admitted entry still passes SPEC 11.3's margin check and nothing else in the walk changes: the 19 Oct "peak concurrent positions x Rs 1 lakh" (BAR:38), made exact for carried losses and for margins that are not exactly Rs 1 lakh |
| SIZE-SPLIT-ALL, SIZE-RISK | `K = Rs 20,00,000`, the window's starting capital | their quantities are shares of the free margin (4.3 step 6, 4.4 step 9), so any smaller capital gives another walk; the capital they size from is the capital they need |

The peak of `ceil(margin in use) - carried net` is reported for every policy as "capital in use at peak", beside K.
`CapitalNeeded` (capital.rs): `observe(margin_after, carried_net)` after each admission, `value(reads_capital, window_capital)
-> Option<i128>` (None when nothing was admitted: "no admission", and no return on K is printed).

### 5.2 How the gate uses it (no number moves)

- The five minimum returns (0.3%/day, 1.5%/week, 6%/month, 18%/quarter, 75%/year; BAR:37; gate.rs:95-130) stay judged on Rs 20
  lakh for every policy: SPEC:781 and OP-22 (SPEC:1085) chose it as "the stricter one", and `Exam::new` uses `CAPITAL_PAISA`
  (gate.rs:452-455).
- Beside it the report shows the same five on K, through `returns_required(frame, K)` (gate.rs:470, whose doc says the base
  is an argument "so a report can show the 19 Oct base beside the headline (OP-22)"), labelled as section 6 says. No FNO-WFT
  code changes (AMENDMENTS, FNO-WFT).

### 5.3 Why K can never loosen a check

FNO-PORTFOLIO admits only while `margin in use <= (Rs 20 lakh + carried net) x 10^6` (fno_portfolio.rs:1245-1247 and :1281-1285, `Carried`),
and once AMENDMENTS P1 lands only while it is also at most Rs 20 lakh `x 10^6` (the limit of section 0 item 3; the second term
does not depend on the starting capital, so K's definition in 5.1 is unchanged).
So at every admission `ceil(margin / 10^6) - carried net <= Rs 20 lakh`, hence `K <= Rs 20 lakh` for every policy, and a net that
meets a minimum on Rs 20 lakh meets it on K. Judging on Rs 20 lakh is therefore never weaker than judging on K: the headline
stays the operator's strict reading, and K is information (the operator's capital-efficiency question).

---

## 6. Labels (constants in `mod.rs`; tests compare them byte for byte; FNO-RUN puts them on every output, record header and route)

- `SIZE-FIXED-20-SEEDED: Rs 1 lakh margin x the stock's leverage a position, at most 20 open, simultaneous signals in the cap tie-break order S81 with its seeded spread (today's rule)`
- `SIZE-FIXED-20-RANKED: Rs 1 lakh margin x the stock's leverage a position, at most 20 open, simultaneous signals by the learned edge (net per rupee of the candidate's learning trades for that stock, side and time of day, a procedure's recipes pooled; a learning month is scored without its own trades, its entry times still shaping the time-of-day buckets), then the smaller stop in R, then liquidity, then a seeded order`
- `SIZE-SPLIT-ALL: no 20-slot cap; the free margin of each second split equally across its signals, whole shares, at least one each`
- `SIZE-RISK: equal risk per trade (risk budget / stop distance), capped by the stock's leverage on an equal share of the free margin, total open risk capped; both budgets from learning months only`
- `LIMITS_LABEL`: `account limits: margin in use at most Rs 20,00,000 (or the capital, when carried losses leave less) and open exposure at most Rs 1,00,00,000, checked at every entry on positions marked at their last traded price; a later mark can carry either past its limit and forces no reduction (SPEC 11.3; Dhan's MTM rule, 9.8, acts on losses only), so the peaks are reported (operator, 3 Nov 2026)`
- `CAPITAL_FIXED_LABEL`: `capital actually needed: the largest margin in use after an admission less the carried net then (quantities never read capital)`
- `CAPITAL_SHARE_LABEL`: `capital actually needed: the whole Rs 20 lakh (quantities are shares of the free margin)`
- `RETURNS_LABEL`: `minimum returns judged on Rs 20 lakh (OP-22, the stricter); on the capital actually needed shown beside`

---

## 7. Tests (`fno_size/tests.rs`; property tests by hand with seeded xorshift generators and many seeds, no new dependency, FP:57)

Batch sizes and caps:
- `an_empty_batch_plans_nothing_and_divides_by_nothing` (every policy)
- `one_candidate_takes_the_whole_free_margin_and_at_five_x_the_whole_exposure_room` (SPLIT-ALL: `F = Rs 20 lakh` at 5x, so
  `q_m = q_x = floor(Rs 1 crore / P)`, a tie labelled `MarginShare`)
- `two_hundred_eighteen_candidates_split_over_the_trade_room` (with the real `Limits` of 100 trades: `k = 100`, the rest FNO-PORTFOLIO's `trade_cap`; with a test `Limits` of 256 trades: `k = 218`)
- `a_batch_of_two_hundred_fifty_six_is_planned_and_two_hundred_fifty_seven_refused`
- `trade_room_zero_keeps_quantities_defined` and `trade_room_one_with_many_candidates`
- `two_recipes_on_one_stock_count_the_stock_once`
- `a_duplicate_stock_and_recipe_is_refused_wherever_it_sits` (adjacent, and at positions 0 and 255, under every policy)
- `two_hundred_fifty_six_candidates_on_one_stock_pass_the_duplicate_check_at_its_bound` (32,640 comparisons, counted by the
  test's own model of the chain)

RANKED ties at every level:
- `a_higher_edge_goes_first`, `equal_means_from_different_counts_tie` (2/4 and 1/2), `no_trade_ranks_after_every_score`
- `an_edge_tie_goes_to_the_smaller_stop_in_r`, `equal_fractions_tie` (1/2 and 2/4)
- `a_stop_tie_goes_to_the_larger_liquidity`
- `a_liquidity_tie_goes_to_the_seeded_key` (and `seed_reached` counts it)
- `a_seeded_tie_falls_to_the_stock_then_the_recipe` (forged equal keys through the comparator's own key form)
- `the_network_order_equals_a_reference_sort` (property: random batches 0..=256, the reference a `sort_by` inside the test module)
- `one_seed_one_order_whatever_the_arrangement`, `another_seed_another_order`
- `the_seeded_key_matches_tie_break_e` (vectors computed from the restated formula of fno_portfolio.rs:1413-1431; FNO-RUN's
  check proves equality against the real function, AMENDMENTS R7)
- `a_negative_edge_is_offered_not_refused`

Margin, exposure and risk at the edge:
- `free_margin_exactly_k_shares_sizes_each_and_leaves_no_remainder`, `one_paisa_less_leaves_the_remainder_free`
- `carried_losses_past_the_capital_refuse_every_row_size_no_free_margin` (margin exhausted before the batch)
- `margin_exhausted_mid_batch_under_ranked_is_portfolios_margin_cap_in_rank_order` (an in-test model of SPEC 11.3's check over
  the plan's order, a later smaller row that still fits admitted; the real account's agreement is FNO-RUN's integration test,
  AMENDMENTS R7)
- `carried_profit_never_lifts_the_free_margin_past_the_margin_cap` (capital Rs 25 lakh, one stock at 300,000 ppm: `F` is
  Rs 20 lakh, not Rs 25 lakh, and the row's margin is at most Rs 20 lakh; check 1's B1 case)
- `carried_losses_lower_the_free_margin_below_the_margin_cap`
- `under_the_operators_limits_the_margin_share_always_binds` (property: views whose margin, reserved included, is at least 20%
  of their exposure, rates at or above 20%: never `ExposureShare`, never `size_no_exposure_room`)
- `exposure_room_binds_under_limits_whose_exposure_cap_is_below_five_margin_caps` (`bound = ExposureShare`)
- `no_exposure_room_refuses_every_row` (a test view or test `Limits`, as the property above shows)
- `a_share_that_buys_no_share_is_refused_and_stays_free` (a Rs 1.3 lakh share price against a Rs 9,174 share)
- `split_all_never_passes_the_free_margin_or_the_exposure_room` (property: every subset of at most `k` rows)
- `risk_never_passes_the_open_risk_cap` (property), `the_risk_room_shrinks_equally_when_the_cap_binds`
- `no_risk_room_refuses_every_row`, `no_learned_risk_refuses_every_row`, `a_budget_at_or_below_zero_is_refused`
- `a_stop_of_zero_distance_is_refused`, `a_stop_through_the_price_is_refused` (long and short), `a_stop_less_lane_is_refused`
- `a_tie_between_risk_and_share_is_labelled_risk`

Leverage, prices, overflow:
- `leverage_below_one_x_gives_less_value_than_margin` (`rate_ppm = 1,250,000`: 0.8x), `a_huge_margin_rate_buys_no_share` (`u32::MAX`)
- `a_rate_below_the_twenty_percent_floor_is_a_caller_defect`
- `a_one_paisa_price_and_an_i64_max_price`
- `every_product_is_checked_and_an_overflow_is_refused_by_name` (capital near `i128::MAX / 10^6`, `u64` quantity overflow,
  a value past `i64::MAX` paise, which FNO-PORTFOLIO would refuse as `ValueOverflow`, fno_portfolio.rs:720-722)

Edge table:
- `an_empty_learning_window_scores_every_cell_none`, `one_trade_fills_all_three_levels`
- `judging_a_learning_month_leaves_its_own_trades_out` (property: altering any trade of month j changes no score while judging j)
- `a_forward_month_reads_the_whole_learning_table`, `a_trade_outside_the_learning_window_is_refused`, `months_out_of_order_are_refused`
- `rungs_empty_one_and_three_hundred_seventy_five`, `unsorted_duplicate_or_late_rungs_are_refused`
- `the_fallback_goes_cell_then_stock_side_then_side`, `a_zero_value_trade_is_refused`
- `a_negative_net_floors_toward_minus_infinity` (net -1 paisa on a value of 3 paise: -333,334 ppm, not -333,333)

Risk book and capital:
- `open_risk_holds_through_the_last_fill_second_and_settles_after` (FNO-PORTFOLIO's in-force reading)
- `a_stock_in_force_cannot_be_admitted_again`, `a_release_before_its_admission_and_time_backwards_are_refused`
- `releases_in_any_order_settle_each_row_after_its_own_last_fill` (property: random admit, release and settle sequences whose
  releases carry future last fills in any order, as fno_portfolio.rs:2952-2984's walk makes them; at every second the book's
  open risk equals a reference sum over the rows in force, and a release in last-fill order takes one step)
- `the_peak_open_risk_is_kept`
- `capital_needed_for_fixed_is_the_peak_margin_less_the_carried_net`, `capital_needed_for_share_policies_is_the_capital`,
  `no_admission_means_no_capital_needed`, `capital_needed_never_passes_the_capital_under_the_margin_check` (property)

Whole plan:
- `the_plan_does_not_depend_on_the_batch_arrangement` (property, every policy: rows keyed by candidate, equal as sets; RANKED's order equal)
- `seeded_passes_fixed_quantities_through_untouched`, `busy_stocks_are_while_open_under_every_policy`
- `every_label_is_the_operators_text_byte_for_byte`, `canonical_bytes_differ_per_policy_and_carry_the_version`
- `every_caller_defect_is_refused_by_name_and_changes_nothing`
- `a_negative_view_figure_is_refused_by_name` (each of `margin_in_use`, `exposure`, `reserved_margin`, `reserved_exposure`,
  `open_risk` at -1, under every policy: `NegativeView` naming it, `out` untouched; a negative `capital` is no defect)
- `the_measurements_run_at_tiny_sizes` (all seven bench functions covered, FP:267)

---

## 8. Benches (p50 and p99 printed at 1x, 10x, 100x; flat within the runner crate's `CEILING_PERMILLE = 2_500`, SEAM:crates/runner/benches/ratio.rs:92, and inside an absolute budget; every budget an ESTIMATE until measured on the Mac)

The harness's ratio is two-sided: a row whose per-unit cost falls 2.5x at a larger size breaches as surely as one that rises
2.5x (`up.max(down) <= CEILING_PERMILLE`, SEAM:crates/runner/benches/ratio.rs:176-190; the part wrappers call it, `use
super::ratio;`, as fix/fib-fno-portfolio's benches/ratio/fno_portfolio.rs:21 does). So no row divides fixed work by a count
that its sizes vary: fixed work is timed on its own, in the p99/p50 form, and per-unit work at sizes that keep the same mix.

| Id | Step | Sizes | p99 budget (ESTIMATE) |
|---|---|---|---|
| C-FNO-SIZE-01 | `plan` per candidate, SPLIT-ALL, RISK and SEEDED, on one batch of 218 candidates on distinct stocks (the universe's size) | timed after 10, 100 and 1,000 batches already planned through the same `Plan` (1x, 10x, 100x of history: the C-FNO-PORT-01 form, the batch fixed and the history behind it grown), so the batch's fixed work (the `first` reset, `F`, `X`, `k`, `m`, `x`) sits in every sample alike and cannot move the ratio; the per-candidate p99 flat, which is what O(1) per decision means here | 300 ns per candidate |
| C-FNO-SIZE-02 | `EdgeTable::score` | tables of 1k, 10k, 100k learning trades | 50 ns |
| C-FNO-SIZE-03 | RANKED's network, 256 rows (fixed work; p99/p50 <= 1.2, the FNO-RANK form, FP:574) | 1x/10x/100x batches | 80 us per batch |
| C-FNO-SIZE-04 | `RiskBook` admit, release and settle, in two parts timed apart | (a) one cycle, its release in last-fill order, with 255 others in force, at 1x/10x/100x cycles of history: the cycle's p99 flat; (b) the worst release, joining the list at its head behind 255 released rows (fixed work at 1x/10x/100x repetitions; p99/p50 <= 1.2, the C-FNO-SIZE-03 form; L-FNO-SIZE-07) | (a) 100 ns a cycle; (b) 2 us a release |
| C-FNO-SIZE-05 | `EdgeTable::build` in its two parts, timed apart (L-FNO-SIZE-02's O(trades + cells)), so the cheap clearing of a cell and the dearer insertion of a trade are never summed into one unit whose mix moves with the size | (a) the per-trade insertion into a table already reset, 1k, 10k and 100k learning trades on one 16-rung ladder: the per-trade p99 flat; (b) the reset of the cells and the 375-entry minute map alone, on a 375-rung ladder (the bound: 256 x 2 x 376 cells; fixed work at 1x/10x/100x repetitions, p99/p50 <= 1.2) | (a) 20 ns a trade; (b) 2 ms a reset |
| C-FNO-SIZE-06 | `EdgeTable::judging` at a month boundary, per trade moved (the judged month's out, the previous month's back) | months of 100, 1k and 10k trades | 50 ns per trade |
| C-FNO-SIZE-07 | `plan` on its duplicate check's worst batch: 256 candidates on one stock, distinct recipes, 32,640 comparisons (fixed work; p99/p50 <= 1.2, the C-FNO-SIZE-03 form) | 1x/10x/100x batches | 150 us per batch |

Benches measured in the cloud are advisory (cloud-wave PLAN section 2 item 9): every run's p50, p99, max, n and method, never a
best-of-N pick.

---

## 9. Ledger rows (drafts; each written only beside the test that proves it, FP:312)

`docs/04-invariants.md` (table form of fix/fib-fno-portfolio's rows; `| Id | Invariant | Proof | |`):
- FNO-SIZE-I01 SPLIT-ALL and RISK plans keep the hard limits at every entry: any rows of one second admitted together hold at
  most the free margin, cut from `min(capital, margin_cap)`, and the exposure room. FNO-SIZE-I02 No zero quantity is ever planned: `q >= 1` or a named refusal. FNO-SIZE-I03 SPLIT-ALL's
  shares, floors, one-share minimum and no re-split; under the operator's limits the margin share binds. FNO-SIZE-I04 RISK's quantity, its equal risk, the open-risk cap, and stops
  of zero, negative or no distance refused. FNO-SIZE-I05 RANKED's strict total order, equal to a reference sort. FNO-SIZE-I06
  The edge table reads learning months only and never the month it judges. FNO-SIZE-I07 One seed, one order; the seeded key is
  tie-break (e)'s. FNO-SIZE-I08 A plan does not depend on the batch's arrangement. FNO-SIZE-I09 Open risk is held through each
  row's own last fill second, releases taken in any order as the account takes them; settle is amortized O(1), and a release out
  of last-fill order walks at most 255 rows. FNO-SIZE-I10 The capital actually needed per policy, at most the capital.
  FNO-SIZE-I11 Every caller defect is refused by name and changes nothing (a negative view figure included); a duplicate (stock,
  recipe) is found wherever it sits; an edge value floors a negative net.
  FNO-SIZE-I12 Every label is the operator's text.
- C-FNO-SIZE-01..07 rows, each naming `runner::bench::fno_size::row` and the library's `measure_*` (the C-FNO-PORT-01 form).

`docs/05-decisions.md` (`### D-86nn — FNO-SIZE: <title> — 2026-11-03`, then Context, Decision, Why; the fix/fib-fno-portfolio form):
- D-8630 the sizing and admission policy is S82, a tested run dimension: four labelled policies, each judged on its own; the
  operator's hard limits checked at every entry for all four, the margin limit `min(capital, Rs 20 lakh)`; a mark forces no
  reduction and the peaks are reported, the forced-cut question put to the operator; FNO-PORTFOLIO stays the owner of the caps
  and S81.
- D-8631 the decision instant is one order start second.
- D-8632 SPLIT-ALL: equal margin and exposure shares over the distinct eligible stocks capped by the trade room, the free margin
  cut from `min(capital, margin_cap)`; floor quantities; one-share minimum and why; no re-split; later signals see only what is
  free; under the operator's limits the margin share binds.
- D-8633 RISK: quantity, equal risk, the open-risk cap; both budgets from the learning window's SEEDED walk, its trades with a
  stop at a positive distance only; stops refused.
- D-8634 RANKED: the edge table (source, a procedure's recipes pooled, value, three levels, buckets generated over all learning
  entry times, leave one month out for the scores, one table per (candidate, scheme, window), so a RANKED walk is never shared
  between windows) and the order (edge, stop in R, liquidity, seeded, stock, recipe); the network instead of a sort; the
  account's exits by symbol, so the seed enters only through the fourth key.
- D-8635 the liquidity key: session-to-date traded value from the Zerodha 1-minute candles up to the decision candle (causal),
  computed by FNO-RUN.
- D-8636 the capital actually needed per policy; the gate stays on Rs 20 lakh (OP-22); K shown beside; K at most Rs 20 lakh.
- D-8637 the labels and refusal words, counted per policy.
- D-8638 FNO-SIZE's own anchors (section 2) and why not inside the seam's blocks; that they stand for its slot lines wherever
  the F2 RULES say "under its own slot line" (F3:69); and that its docs/05 entries sit under the seam's heading "F&O strategy
  path: part blocks (D-8100..D-8339)" (SEAM:docs/05-decisions.md:87599) though D-8630..D-8639 is outside that range: the heading
  is the seam's, and FNO-SIZE edits none of its lines.
- D-8639 spare.

`docs/06-limits.md` (`| ID | Limit | Why it is not O(1) / not closed | Bound | Package |`):
- L-FNO-SIZE-01 RANKED's network: at most 4,608 compare-exchanges a batch, bounded by the universe (256), not O(1).
- L-FNO-SIZE-02 `EdgeTable::build` O(trades + cells), cells at most `256 x 2 x 376`; `judging` O(trades of two months) a month
  boundary. Measured by C-FNO-SIZE-05 (per trade inserted, and the reset of the cells and the minute map timed apart at the
  bound) and C-FNO-SIZE-06 (per trade moved).
- L-FNO-SIZE-03 `plan` is O(n) a second, n at most 256, plus the duplicate check's `c (c - 1) / 2` comparisons for `c`
  candidates of one stock: at most 32,640 a batch at the universe's worst, measured by C-FNO-SIZE-07.
- L-FNO-SIZE-04 the bench figures are one cloud machine's; NOT MEASURED on the Mac or CI.
- L-FNO-SIZE-05 SPLIT-ALL and RISK cannot run until FNO-PORTFOLIO's slot cap lands (fno_portfolio.rs:73, :737; AMENDMENTS P1),
  and the FIXED pair keeps the Rs 20 lakh margin limit only once the same repair lands (fno_portfolio.rs:1281-1285).
- L-FNO-SIZE-06 the liquidity key, the edge table's source walk and the risk budgets are FNO-RUN's adapters; FNO-SIZE takes them as inputs.
- L-FNO-SIZE-07 `RiskBook::release` out of last-fill order walks back over the released rows to its place: at most 255 steps,
  bounded by the universe, not O(1) (in last-fill order one step; `settle` amortized O(1)). Measured by C-FNO-SIZE-04 (b).

`docs/07-plan.md`: one table (the fix/fib-fno-portfolio form): built in the cloud on `cloud/fib-fno-size-20261103`; next the Mac's
check; then FNO-PORTFOLIO's slot cap and FNO-RUN's wiring (AMENDMENTS).

---

## 10. Coverage, gates, base, size

- **Coverage:** 100% lines and branches of `crates/runner/src/fno_size/` whole by Route B against its base; the bench wrapper by
  CC-20's precedent (F3:57); the `lib.rs` line holds no region. Mutants deferred.
- **Gates:** fmt, clippy `-D warnings` on `runner`, the runner tests, gates 11, 12 and 14 and the decision-citation test on its
  branch (FP:266). Gate 11 on its files: no `.sort*(`, `binary_search`, `.iter().find(`, `.contains(&`, float, unsized map,
  panic, local lint allow or runtime assert on a production line (SEAM:.github/gates_ledger.rs:2122-2224); `get` and named
  refusals instead of indexing. Gate 12: the module doc's cost claim names its proof, the bench `crates/runner/benches/ratio.rs`
  and the test that runs the measurements (the form FNO-NET took for gate 12: `fix/fib-fno-net` commit ef5af65ad). Gate 14's cover
  rows in `ci.yml` for C-FNO-SIZE-01..07 are the main session's at F2's assembly, never this part's (F3:52, :70).
- **Base: SEAM** (`1ad6e24ef8e0`), reached through the kit branch `cloud/fib-kit-size-20261103` (KIT1 plus this part's kit
  folder; KIT1 changes nothing outside `reports/`: `git diff --name-status SEAM KIT1` lists 12 added `.md` files). The Mac
  integrates the cloud branch without the kit commits (`replay-minus-reports.sh` with START SEAM, FROM the kit-size commit) and
  checks it against SEAM. **Depends on: FNO-SEAM only**, passed: no speculative cut and no catch-up. A seeded `built` row goes
  straight to its check (F3:346-348 in `runF2`, F3:465-470 in `runSpec`); with one dependency the final base is that
  dependency's passed tip (F3:333, F3:575), the seam's, which the row's base already names, so no catch-up runs (F3:591).
  - Not chosen: FNO-PORTFOLIO's built tip under L12 (to reuse `StockId`, `TieKeys`, `Fraction`). It would make FNO-SIZE wait for
    PORTFOLIO's check, a catch-up and PORTFOLIO's slot-cap repair (AMENDMENTS P1), for three small types that FNO-RUN maps in
    three lines.
- **Size:** 6-9 agent-hours (ESTIMATE), M. The arithmetic is small; the edge table, the network and the property tests are most of it.

---

## 11. NEEDS THE MAC

1. Check 1 by Route B on the Mac's pin (`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, cloud-wave PLAN section 2 item 6), with
   macOS rows; its benches on the Mac (cloud figures are advisory).
2. Gate 14's `ci.yml` cover rows for C-FNO-SIZE-01..07, at F2's family assembly (main session).
3. FNO-PORTFOLIO's slot cap, margin limit and lock reader (AMENDMENTS P1), a Mac repair round of that part, before SPLIT-ALL or RISK can be
   walked and before the FIXED pair keeps the Rs 20 lakh limit.
4. FNO-RUN's integration tests that tie FNO-SIZE to the real account, FNO-NET and FNO-WFT (AMENDMENTS R7).
5. The anchor cross-check against PSRB-STRAT and PSRB-RUN once they have anchors (START.md step 1; PSRB-RSI, -BOLL, -STV checked, section 2).

## 12. Not proven here (UNVERIFIED or ESTIMATE), and the operator's question

- **OPERATOR QUESTION (SPEC section 20 OP-58; from check 1's B2).** The 3 Nov limits say "at every instant" (OI:13). SPEC 11.3
  checks margin in use and exposure at every entry, on positions marked at their last traded price, and marks never force a
  reduction except under Dhan's MTM loss rule (SPEC:674, 9.8). So a price move after an entry can carry margin in use above Rs 20,00,000 or exposure
  above Rs 1,00,00,000 until positions close; no entry is admitted while either stands above its limit. The choice in this
  design: SPEC 11.3 as written, with each policy's peaks and seconds above each limit reported. If "at every instant" means
  positions are cut when a mark passes a limit, that is a new SPEC rule (which position, at what fill, under which clock) for
  FNO-PORTFOLIO and FNO-RUN, and the operator's to set. FNO-SIZE plans entries only: such a rule would change `LIMITS_LABEL`'s
  words, nothing else of this part.
- Every bench budget and the size (ESTIMATE). Nothing was built or measured.
- That PSRB-STRAT and PSRB-RUN, not designed yet, will take none of section 2's anchor lines (UNVERIFIED; START.md step 1 repeats the check).
- That gate 11 rule 5c counts `#[expect(clippy::...)]` the way it counts a local allow (UNVERIFIED: FNO-SIZE avoids both, so it
  does not matter for this part).
- That the 100-entry slot table keeps FNO-PORTFOLIO's admit and release inside its 300 ns budget (ESTIMATE; AMENDMENTS P1 asks
  for a new bench row).
- How often a RANKED tie reaches the seeded level, and so whether RANKED's k seeds cost k walks (measured by `seed_reached`).

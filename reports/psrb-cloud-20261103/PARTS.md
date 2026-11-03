# PSRB PARTS: the three indicator parts built now on cloud lanes, their slot lines, and the parts that follow

Written Tue 3 Nov 2026, 16:3x-17:xx IST, by a workflow step of main session 70a1a8d5, after the operator's "Yes, build
them now" to building the PSRB indicator parts on cloud lanes. Strategy family PSRB = Pivot R1 + Supertrend + RSI +
Bollinger, tested only on NSE F&O stocks, cash, intraday (`psrb-strategy-v1.html` sections 1, 3, 9). This file is the
design; it is also committed, byte for byte, as `reports/psrb-cloud-20261103/PARTS.md` in the kit (MANIFEST.md).

Repaired Tue 3 Nov 2026 ~18:3x IST by repair 1, from the independent check `check1.md` (3 blocking, 10 not blocking;
the table of what changed is `repair1.md`; the pre-repair file is `work/PARTS.md.bak-20261103-repair1`). Repaired again
~19:2x IST by repair 2, from the independent check `check2.md` (1 blocking: RSI's change counter is a `u32`, with the
test `the_largest_period_smooths_after_its_seed`; 6 not blocking; the table is `repair2.md`; the pre-repair file is
`work/PARTS.md.bak-20261103-repair2`).

Nothing was built, run, committed, pushed or started in this step. No cargo, rustc or clippy ran. Every size, hour and
p99 target is an ESTIMATE until measured. Short names: P = the repository `/Volumes/WD_BLACK/brutex/fresh-20260919/project`
(read with `git show`/`grep` only); W = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`; FP = `W/state/fib-strategy-20261030/PARTS.md`;
WU = `psrb-strategy-v1.html` (this folder); RA and RB = report A and report B of `research-results.jsonl` (this folder,
line 2 and line 1); CW = `W/state/cloud-wave-20261103/PLAN.md`. Repository line numbers are at the base
`1ad6e24ef8e05e8836e511873e8352071dd98fa0` unless a line says `f01d7a0e4`.

---

## 0. The answer in brief

1. **Three parts now, three cloud sessions started in the same operator-typed turn, none waiting on another:**
   PSRB-RSI (`indicators::rsi`, Wilder RSI), PSRB-BOLL (`indicators::bollinger`, SMA plus or minus z sigma) and PSRB-STV
   (`indicators::dual_band_supertrend`, the standard two-band Supertrend). All three live in `crates/indicators` and
   touch no other crate. The existing latch `indicators::trend::SuperTrend` (trend.rs:375) stays byte for byte as it is,
   because the vocabulary bits read it (RB `indicators_present`: evaluator.rs:707, table.rs:356-357, 1029-1030, 1104-1106).
2. **Base: `1ad6e24ef8e0` (FNO-SEAM's passed tip), not `f01d7a0e4`** (section 2.1). The deciding fact: the only
   conflict-free place for a chained bench row in `crates/indicators/benches/ratio.rs` is directly under the verdicts
   array's `];`, and on `f01d7a0e4` that line is FNO-SEAM's own anchor (its hunk is "insert after f01d7a0e4:759"), so
   PSRB lines there would conflict with FNO-SEAM at landing, and D1 aborts a landing on a code conflict
   (`W/state/data-build-20261031/PARTS.md:634`). PSRB lands after the Fibonacci families anyway (WU section 10).
3. **Slot lines: per-part distinct anchors everywhere they fit, and ONE comments-only PSRB-SEAM commit for the one file
   where they cannot fit** (section 2.3). In `crates/indicators/benches/ratio.rs`, `fn main` has exactly one clean
   statement position after the verdicts array (proof in 2.3), and three parts need three. PSRB-SEAM is not a session:
   the main session's `kit-commit.sh` makes it in the same step as the kit commit, as the launching session made FNO-SEAM
   (FP:275, "the launching session commits it before wave 1 starts"). So every session still starts at the same moment.
   Every other shared line (the `pub mod` lines of `lib.rs`, the four ledgers) is folded into each part at its own anchor.
4. **Integration:** the Mac takes each branch without the kit commit and without `reports/` by
   `W/state/cloud-wave-20261103/replay-minus-reports.sh <P> <SEAM> <KIT> <TIP> refs/heads/fix/psrb-<part>`, START = the
   PSRB-SEAM commit, FROM = the kit commit (section 7); then its own check 1 on its pin; then the PSRB family lands after
   FAM-FIB-2 under D1 (base green first).
5. **Later parts, not built now:** PSRB-LAW, PSRB-STRAT, PSRB-RUN (section 6). Their inputs (FNO-LAW, FNO-WFT,
   FNO-SEC-EXEC, FNO-EXITS, FNO-RUN, FNO-RECORDS) have not passed, so no session for them can start today.
6. **Decision ids** are registered (`W/state/decision-blocks.json`: PSRB-LAW D-8570..D-8579, PSRB-RSI D-8580..D-8589,
   PSRB-BOLL D-8590..D-8599, PSRB-STV D-8600..D-8609, PSRB-STRAT D-8610..D-8619, PSRB-RUN D-8620..D-8629; read with `jq`
   in this step). PSRB-SEAM takes none, as FNO-SEAM took none (FP:246).

---

## 1. What I read (each claim below cites file:line)

- WU sections 4 (definitions) and 9 (engine); RA `definitions`, `ambiguities_and_resolution`, `seeds_and_search`,
  `unverified`; RB `engine_reuse`, `indicators_present`, `new_parts`, `decision_ids`, `operator_decisions`.
- The verifier's result (`.../subagents/workflows/wf_f3a867e8-5ef/journal.jsonl` line 7, `verify:both`, arrived while
  this was written; read ~17:2x IST). It confirms the TradingView formulas the three parts use (Supertrend bands and
  direction, RSI `rma` 14, Bollinger 20/2 on the close) and that the latch is looser after a flip and differs on the
  close test only on bars wider than 2 m ATR. Applied here: (a) RSI's "100 when the average loss is 0" is NOT on
  TradingView's help page; it is the built-in RSI script's guard (`down == 0 ? 100 : up == 0 ? 0 : ...`, secondary
  sources), so 3 and D-8582 cite it so, keep 100 as the VIDEO-ORIGINAL reading and give "undefined" as a labelled
  variant (`flat()`), as the verifier recommends; (b) STV's tests cover the wide bar and TradingView's warm-up against
  the latch's seed (ST-5, ST-1, as designed); (c) its strategy-level corrections (the exit rule, smaller stops, the hole
  rule for carried readings, re-folding per dial value, the tie-break without the rank, the build bases) go into the
  later parts' outlines (section 6). Its other corrections concern reports A and B's strategy text, not these parts.
- FP 4.1-4.4 (:155-251, the slot rule and FNO-SEAM's text), section 5's part format and "Common to every part"
  (:253-269), section 11 (:1208-1240, benches), the 1 Nov anchors addendum (:1306-1351).
- `W/state/data-build-20261031/PARTS.md:630-700` (ONE ANCHOR PER PART: rule A.1-A.7; "D1 aborts a landing on a conflict
  in any file but the five ledgers", :634; "at least 2 unchanged base lines", rule A.6).
- `git show f01d7a0e4:CLAUDE.md` (484 lines): section 2 (:50-112, Rust only, `.json` only under `.claude/` at :54-55,
  the Py-language rule at :80-87), section 3 rules 4, 5, 7 (:138-140, :176, :179), section 7 (:388-395, paisa `i64`,
  no float), section 9 (:426-440, definition of done).
- `crates/indicators` at `1ad6e24ef`: lib.rs:63-73 (module list), :75-79 (FNO-SEAM slots), :253-254 (`Corrupt`),
  :1643 (`Candle`), :1710 (`check`), :1778 (`check_evaluable`); trend.rs:90 (`SCALE`), :265-358 (`Atr`), :362 (`Trend`),
  :375-466 (`SuperTrend`); vwap.rs:170 (`isqrt_i128`), :187, :212, :225; benches/ratio.rs (795 lines; :63, :767-789);
  daily.rs:181, :203, :206. `crates/indicators/Cargo.toml` (one dependency, `vocab`); workspace lints
  (`f01d7a0e4:Cargo.toml:72-90`: `indexing_slicing`, `float_arithmetic`, `cast_possible_truncation`, `cast_sign_loss`,
  `unwrap_used`, `expect_used`, `panic` denied; pedantic warn, so an error under `-D warnings`).
- Gates read for what the parts must satisfy: `.github/gates_ledger.rs` gate 11 (rules 1-7 over the production closure,
  `#[cfg(test)]` modules stripped: :699-900, :2040-2215), `.github/gates_bounds.rs` gate 12 (every doc-block cost claim
  names its proof, :1-3, :238-298), `crates/core/tests/cost_invariants.rs` (bench ids printed as the first word of a
  string literal in a top-level `benches/*.rs`, paired with a `| C-... |` row of docs/04), `crates/core/tests/decision_citations.rs`
  (every `D-nnnn` cited under `crates/` heads a docs/05 entry), `.github/invariant_paths.rs` (gate 10: a row's named
  tests exist in its named file), `crates/indicators/tests/module_doc_counts.rs` (only eight listed modules are read).
- The cloud forms: `git show fa9022bac:reports/fib-cloud-20261103/CLOUD.md` and `RULES-MAC.md`; `W/state/cloud-wave-20261103/kit-base/CLOUD.md`;
  CW section 2 (:57-70) and sections 8-11; `briefs/TEMPLATE-continue.md`; `kit-base-commit.sh`; `replay-minus-reports.sh`.
- The edits other work makes to the shared files, from `git diff -U0 f01d7a0e4 <ref>` over 66 refs committed since
  28 Oct (fix/*, base/*, fam/*, origin/cloud/*): `work/hunks-20261103.txt` in this folder. In
  `crates/indicators/src/lib.rs` every fib branch inserts after f01d7a0e4:74 and BASE-COV-LIBS-LS3 edits :883 and :2016
  (tests only, ee9016850); in `benches/ratio.rs` every fib branch inserts after :64, edits :723, inserts after :759.

---

## 2. Rules shared by the three parts

### 2.1 The base, and why not f01d7a0e4

**Decision: every PSRB branch descends from `1ad6e24ef8e05e8836e511873e8352071dd98fa0`** through the PSRB-SEAM commit
(2.3) and the kit commit.

- **The bench rows decide it.** `crates/indicators/benches/ratio.rs` judges its rows in one array typed `[bool; ROWS]`
  (:759-767), so a row inside the array would make every part edit the one `ROWS` line (:745), as the FNO-SEAM comment
  itself says (:768-770). A part's row must be a statement after `];` that re-binds `verdicts` (FNO-LADDER's form,
  `fix/fib-fno-ladder`, benches/ratio.rs +791-798). On `f01d7a0e4` the only statement position between `];` (:759)
  and `if verdicts.contains(&false)` (:760) is `];` itself, and FNO-SEAM inserts there (its hunk `@@ -759,0 ...`;
  `work/hunks-20261103.txt`, every fib ref). A PSRB insertion at the same place conflicts with FNO-SEAM when PSRB
  lands, and D1 aborts a landing on any code conflict (`data-build PARTS.md:634`). On `1ad6e24ef` FNO-SEAM's lines
  are already in the base, so PSRB-SEAM can sit directly under `];` (:767), 20 lines above FNO-LADDER's and
  FNO-PAUSE's insertion points (:788, :789): no conflict by construction.
- **It costs no landing time.** PSRB lands after the Fibonacci families by the operator's order ("data, audit and base
  first; then the Fibonacci strategy; then PSRB", WU section 10), and PSRB-STRAT and PSRB-RUN need F1/F2 code anyway
  (RB `new_parts` 6-7). `1ad6e24ef` reaches feat/db-fix with F1 (FNO-SEAM) and with BASE-STACK-1028 (merged into the
  seam by e9423b8b4), both of which land before PSRB.
- **It is a proven cloud base.** KIT1 (`fa9022bac`) is `1ad6e24ef` plus a Markdown folder, and four fib parts were built
  on it in the cloud on 3 Nov (CW:38-46). Its history is already public (KIT1 is on origin), so pushing the PSRB kit
  publishes only PSRB-SEAM and the kit folder.
- **What f01d7a0e4 would have bought:** landing the three parts before the fib families. Nothing needs that: no PSRB
  run can start before PSRB-STRAT and PSRB-RUN, which need fib code (RB `where_in_plan`).
- `1ad6e24ef` and `f01d7a0e4` hold the same text in every region this file anchors in, except benches/ratio.rs
  (FNO-SEAM's 30 lines) and the ledgers' FNO sections at their ends (`git diff -U0 f01d7a0e4 1ad6e24ef` over the six
  shared files, read here). So the ledger and `lib.rs` line numbers below hold on both.

### 2.2 Files, ownership and anchors

Every file below has ONE owner. New files are the owning part's alone. A shared file is edited only by inserting at the
owner's own anchor, never by changing an existing line; each anchor sits at least 2 unchanged lines from every other
part's insertion and from every edit the 66 refs make (rule A.6 of the ONE ANCHOR decision). No part edits
`crates/indicators/src/trend.rs`, `vwap.rs`, `evaluator.rs`, `daily.rs`, any `tests/*.rs`, `Cargo.toml`, `Cargo.lock`,
`CLAUDE.md`, `AGENTS.md`, `docs/00-charter.md` or `.github/`. A part that finds it must edit any other file stops with
`STATUS: BLOCKED OUTSIDE` and the exact change (CLOUD.md item 6).

| file | PSRB-RSI | PSRB-BOLL | PSRB-STV | others' edits nearby (f01d7a0e4 lines) |
|---|---|---|---|---|
| `crates/indicators/src/lib.rs` | `pub mod rsi;` after `pub mod pattern;` (:70) | `pub mod bollinger;` after `pub mod anchored;` (:63) | `pub mod dual_band_supertrend;` after `pub mod daily;` (:65) | FNO block after :74 (in the base), FNO-LADDER/-PAUSE under :77/:79; LS3 at :883, :2016 (base :889, :2022) |
| `crates/indicators/benches/ratio.rs` | under `// slot bench-mod PSRB-RSI` and `    // slot bench-row PSRB-RSI` | the two PSRB-BOLL slots | the two PSRB-STV slots | FNO-LADDER/-PAUSE under :68/:70 and :788/:789 (base coordinates) |
| `docs/04-invariants.md` | above `## The quarantine (R4-18, D-7290)` (:8951) | above `## GDFL readers: series codes, stem hand-overs, header shapes, named bad rows (R4-22; D-7310 to D-7314)` (:9016) | above `## /db reads only what its view shows (R3-12; D-7111)` (:9062) | IA-3b, PREC-9 after :8981; ICI-1 edits :9030; everyone else after :9095 (EOF) |
| `docs/05-decisions.md` | above `### D-7272 — AUT-10 decision 2 (DB-B44): the staleness bound stays 14 days; ...` (:84676; repair 1, was D-7300 :85249) | above `### D-7310 — G3: ...` (:86592) | above `### D-7111 — /db reads only ...` (:86993) | BASE-REDS-1023 after :83974; IA-3b after :85624; PREC-9 after :86535; AG-06b after :87044; EOF :87513; the 24 Oct cloud refs CC-5, IA-10a, R3-20 above :85249 (check1 N2) |
| `docs/06-limits.md` | above `## The quarantine (R4-18, D-7290)` (:21955) | above `## One not-owed authority (R4-19; D-7315)` (:22024) | above `## /db reads only what its view shows: what R3-12 leaves bounded, unmeasured or open (R3-12, D-7111)` (:22067) | after :21977; ICI-1 edits :22016; EOF :22127 |
| `docs/07-plan.md` | above `## Historical sweep completion follow-through — 2026-09-06 (D-0524)` (:983) | above `## Grammar search limits recorded but not changed — OPEN (D-0751 to D-0754)` (:1058) | above `## GDFL import at one second — 2026-10-05 (D-2802 to D-2806)` (:1110) | :500; EOF :1121 |
| new files | `src/rsi.rs`, `src/rsi_tests.rs`, `benches/ratio/psrb_rsi.rs` | `src/bollinger.rs`, `src/bollinger_tests.rs`, `benches/ratio/psrb_boll.rs` | `src/dual_band_supertrend.rs`, `src/dual_band_supertrend_tests.rs`, `benches/ratio/psrb_stv.rs` | none |

**How to insert, exactly:**
- `lib.rs`: ONE line, the `pub mod` line, directly under the named `pub mod` line. The three names sort into place
  (`anchored < bollinger < column`, `daily < dual_band_supertrend < evaluator`, `pattern < rsi < session`), so rustfmt's
  module ordering leaves them where they are, and no blank or comment line is added. Distances in unchanged lines: BOLL to STV 2
  (:64-:65), STV to RSI 5 (:66-:70), RSI to FNO-LADDER's insertion point 7 (:71-:77 in the base). `work/anchors-check.sh`
  merges the three lines with each other, FNO-LADDER, FNO-PAUSE and BASE-COV-LIBS-LS3: clean, and the module list stays
  sorted.
- `benches/ratio.rs`: directly under your own two slot lines (2.3), exactly as FNO-LADDER does under its slots
  (`#[path = "ratio/psrb_<x>.rs"]` and `mod psrb_<x>;` under the bench-mod slot; one re-binding statement under the
  bench-row slot). Nothing else in the file.
- Ledgers: your block goes directly ABOVE the named heading: after the blank line that precedes it (each of the twelve
  headings has a blank line before it and a unique text; checked here), and your block ends with one blank line, so the
  file keeps one blank line between sections. In docs/04, 06 and 07 the block is one `## ...` section; in docs/05 it is
  only your `### D-85xx` entries (no `##` heading, so the file's later entries do not fall under it). FNO-SEAM's own
  note says other work goes above its section (base docs/04:9111), and these anchors are far above it. In "insert
  after" terms the anchor line is that blank line: docs/04 :8950 (RSI), :9015 (BOLL), :9061 (STV); docs/05 :84675,
  :86591, :86992; docs/06 :21954, :22023, :22066; docs/07 :982, :1057, :1109; the heading under it is the unique text
  that locates it. `work/anchors-check.sh` merges the three blocks with each other and with FNO-SEAM, IA-3b, PREC-9,
  ICI-1, AG-06b, SURV-1, AUC-1 and BASE-REDS-1023's ledger edits: every merge clean (run 3 Nov ~17:2x IST, with RSI's
  docs/05 block then at :85248).
- **Repair 1 moved PSRB-RSI's docs/05 anchor** from above D-7300 (:85249) to above D-7272 (:84676): three unlanded cloud
  refs of 24 Oct (`origin/cloud/db-CC-5-20261024`, `db-IA-10a-20261024`, `db-R3-20-20261024`) insert their D-7040,
  D-4896 and D-7093 entries directly above D-7300 (check1 N2). Evidence in `repair1-work/`: over all 775 refs under
  `refs/heads` and `refs/remotes/origin`, every ref that holds the D-7272 heading (104) has the base's exact 4-line
  window above and below it (`win-cand2.out`; the same check flags those three refs at D-7300, `win-D7300.out`); a
  `diff3` merge of docs/05 with the three blocks at the new anchors is clean against those three refs and against
  FNO-SEAM, IA-3b, PREC-9, AG-06b, BASE-REDS-1023 and AUC-1, where the old anchor conflicts with all three refs
  (`ledger05-diff3.out`; its one other conflict, `fix/db-R3-13`, is R3-13's own branch of 24 Oct, older than f01d7a0e4,
  whose D-7300 entry and repair-2 text f01d7a0e4 already holds (`git grep`), so it has landed and the diff3 model over
  f01d7a0e4, which is not its merge base, is the wrong one for it; the unchanged BOLL anchor alone conflicts with it the
  same way under that model; its ancestry is UNVERIFIED, since `git show`/`git grep` cannot show it).
  `work/anchors-check.sh` now names the new line and the three refs; its run after repair 1 (with the reworded seam
  comment of 2.3) is clean in all 40 merges (`repair1-work/anchors-check.out`).
- Column of ids: invariants `PSRB-<CODE>-Inn`, benches `C-PSRB-<CODE>-nn`, limits `L-PSRB-<CODE>-nn`, with `<CODE>` =
  `RSI`, `BOLL`, `STV` (upper-case letters only, so `cost_invariants.rs` `is_cost_id` reads `C-PSRB-RSI-01`).

### 2.3 PSRB-SEAM (wave 0; comments only; made by the main session's `kit-commit.sh`, never a session)

**Why it is needed (the proof that per-part anchors cannot do it).** Three parts each need one statement after the
verdicts array and before `if verdicts.contains(&false)` (2.1). On the base those lines are: `    ];` (:767), then
FNO-SEAM's 20 comment lines (:768-:787), FNO-LADDER's slot (:788), FNO-PAUSE's slot (:789), then the check (:790).
A statement after any of :768-:786 would split FNO-SEAM's sentences; after :787 it is 1 line from FNO-LADDER's
insertion point; after :788 or :789 it IS FNO-LADDER's or FNO-PAUSE's point. That leaves `];` alone: one position for
three parts. Before the array, a row cannot fold into the check without editing `ROWS` or the check line; after the
check (:793) a row would run only when every earlier row passed and would need its own exit. So a seam is needed, and
because the hard constraint forbids a session that the others wait on, the main session writes it in the kit step.

**Its exact content: one file, two insertions, 19 added lines, no deleted line.**

Insertion A, directly after base line 63 (`use indicators::{Candle, OI_NULL};`), before the blank line 64:

```
                                                                    <- one blank line
// ---- PSRB indicator parts (state/strategy-psrb-20261103/PARTS.md 2.3). One slot per
// ---- part; each part inserts its bench module (the path attribute naming
// ---- benches/ratio/psrb_<x>.rs and its mod line) directly under its slot.
// slot bench-mod PSRB-RSI
                                                                    <- one blank line
// slot bench-mod PSRB-BOLL
                                                                    <- one blank line
// slot bench-mod PSRB-STV
```

Insertion B, directly after base line 767 (`    ];`, the line after `        a_known_step_stays_within_its_budget(),`):

```
    // PSRB rows (state/strategy-psrb-20261103/PARTS.md 2.3): each part puts one statement
    // directly under its own slot, in the form the F&O rows below take, naming its own
    // module and ids: `.chain([psrb_<x>::row(floor, &["C-PSRB-<CODE>-01", ...])])`.
    // They run before the F&O rows, fold into the same check, and are not counted in ROWS.
    // slot bench-row PSRB-RSI
                                                                    <- one blank line
    // slot bench-row PSRB-BOLL
                                                                    <- one blank line
    // slot bench-row PSRB-STV
                                                                    <- one blank line
```

(The arrows are not part of the text; `kit-commit.sh` holds the bytes and proves them.) After the seam, the slots sit at
seam lines 68, 70, 72 (bench-mod) and 781, 783, 785 (bench-row). Each part's insertion is 2 unchanged lines from its
neighbour's (a blank line and the next slot). The seam's comments hold no `ratio(` (gate 14's measurement counter) and
no cost id: `"C-PSRB-<CODE>-01"` is no id for `is_cost_id` (the `<` stops it), the same as FNO-SEAM's `"C-FNO-<CODE>-01"`.
Gate 12 reads `///` and `//!` blocks only (gates_bounds.rs:238-298), and the seam has neither. Repair 1 (check1 N10)
reworded block B's fourth line to say the PSRB rows "are not counted in ROWS", because the `ROWS` doc comment (base
benches/ratio.rs:730-731) names only the F&O rows as chained; `kit-commit.sh --print-seam` on the base bench (blob
4932b53f) still adds 19 lines and deletes none, every added line a comment or blank, slots at 68, 70, 72, 781, 783, 785,
quote marks 108 to 110, `ratio(` 9 to 9, longest line 91 characters (`repair1-work/seam-check.out`).

**Checking it.** Comments only: `cargo fmt --all --check`, clippy `-D warnings` and the tests are unchanged by it, and
its coverage set holds no instrumented region (FP:271-275 for FNO-SEAM). It is checked as part of every PSRB part's
check (each part's tree contains it) and by the family check; it needs no check round of its own. It is integrated as
`refs/heads/fix/psrb-seam` = the seam commit, written by `kit-commit.sh`, never pushed. It lands as the first commit of
the PSRB family.

### 2.4 Common to every part (the bar, the gates, the style)

- **Bar.** 100% line and branch coverage of the part's D2 set by Route B (`d3-recipe.txt` step 4; the set is the new
  files whole, `fn main` of `benches/ratio.rs` whole because the part adds a statement in it, and the added `lib.rs`
  line, which has no region). The bench wrapper and `main` are measured by running the bench target once under the same
  instrumentation and merging its profile (RULES-MAC R2 "COVERAGE", CC-20's precedent); only a self-checking bench's own
  failing arm is (F). `cargo fmt --all --check`, `cargo clippy -p indicators --all-targets --locked -- -D warnings`,
  `cargo test -p indicators --locked`, and `cargo test -p core --locked --test cost_invariants --test decision_citations`
  (they read docs/04, docs/05 and every bench file). Mutants are deferred (operator, 30 Oct). Each new test is shown red
  under a hand change of the code it guards, then the change is undone (kit-base CLOUD.md item 7).
- **Integers only** (CLAUDE.md section 7, :388-395; FP:269). Prices are paisa `i64`; averages are held scaled by
  `trend::SCALE` (trend.rs:90) in `i128`; every comparison cross-multiplies or compares squares in `i128`; no `as` cast
  (`cast_possible_truncation`, `cast_sign_loss` denied), no float (`float_arithmetic` denied), no indexing
  (`indexing_slicing` denied: `get`, `get_mut`, iterators). Every bound that rules out an `i128` overflow is a
  `const _: () = assert!(...)` beside the arithmetic, computed with `checked_mul` chains in the const.
- **Narrowing an `i128` result** (`ppm() -> u32`; `basis_floor`, `upper_floor`, `lower_ceil() -> i64`; `z_milli`,
  `atr() -> Option<i64>`), one way in all three parts (repair 1, check1 N7): `T::try_from(x).ok()` where the API returns
  an `Option` (the house form, trend.rs:241, :349; vwap.rs:298, :382), else `T::try_from(x).unwrap_or(B)` with `B` the
  bound the const asserts or the formula guarantees (`MAX_LEVEL_PPM` for `ppm`; the form of daily.rs:815). Never `as`
  (`cast_possible_truncation` and `cast_sign_loss` are denied, f01d7a0e4:Cargo.toml:86-87) and never a `match` or `if`
  whose arm cannot be reached (an uncoverable branch under the 100% bar). A doc comment says why the value always fits.
- **Signatures.** The API blocks below fix names, argument and return types. `const fn` is asked only where it needs no
  conversion: `From` cannot be called in a `const fn`, and `as` casts trip the pedantic cast lints, which are errors
  under `-D warnings`; a builder may drop `const` from a shown signature if its body would need either, and says so in
  the part's report. Every public item has a doc comment (`#![deny(missing_docs)]`, lib.rs:61), with `# Errors` on each
  `Result` and `#[must_use]` on each getter (pedantic).
- **A refusal changes nothing.** Every `fold` validates first (`Candle::check_evaluable`, lib.rs:1778, then the part's
  own domain), and only then mutates. A test for each refusal compares the whole state before and after (`PartialEq`).
- **Streaming, no look-ahead** (CLAUDE.md section 3 rule 7, :179). `fold` takes one candle; a reading after candle t
  depends on candles 0..=t only. The current candle IS in its own reading (TradingView's convention, RA D5), and a caller
  that wants the engine's "anchor excludes the candle" convention keeps the previous reading (every reading is `Copy`).
- **Timestamps are not read.** The three folds read OHLC only; ordering and holes are the caller's (PSRB-STRAT taints a
  hole candle, WU section 3 "no decision on a candle with a missing minute"). A test proves a time gap changes nothing.
- **No vocabulary position.** The modules emit no condition bit. Their module docs must not use the bold
  `**N vocabulary positions**` form (`module_doc_counts.rs` reads only its eight listed modules, :52-85, and the
  lib.rs table, lib.rs:9-27, lists position sources only: neither changes). They are readings for the F&O strategy path.
- **Gate 11** scans the production closure with `#[cfg(test)]` modules stripped (gates_ledger.rs:699-900): no `.sort`,
  `binary_search`, `.iter().find(`, `.contains(&`, `assert!` in fn scope, `panic!`, `.unwrap()`, `.expect(` in
  production code. Tests live in `src/<module>_tests.rs`, declared at the end of the module as
  `#[cfg(test)] #[path = "<module>_tests.rs"] mod tests;`: the `#[cfg(test)] #[path]` form LS3 uses (trend.rs at
  ee9016850, last three lines), where LS3 names its module `trend_cover_tests`; the name `tests` is this file's choice, and
  gate 11's closure follows `#[path]` and skips `#[cfg(test)]` whatever the name (gates_ledger.rs:855-865; repair 1,
  check1 N6).
- **Gate 12**: every `///` or `//!` block that makes a cost claim (constant, O(1), flat, bounded, per candle) names its
  proof, the bench id `C-PSRB-<CODE>-0n` or a test fn (gates_bounds.rs:1-3), as trend.rs:148-150 names `C-I-01`.
- **Gate 10: every docs/04 row names its proofs in the path-qualified form gate 10 reads** (repair 1, check1 N8): the
  test fns in backticks, joined only by `,`, `;`, `&`, `and` or `or`, then ` in ` and the file in backticks, e.g.
  `` `wilder_smoothing_matches_fixture_rsi_1` and `the_seed_is_the_floored_mean_of_the_first_period_moves` in
  `crates/indicators/src/rsi_tests.rs` `` (invariant_paths.rs `references`: a `|` line holding `` in `crates/ `` is
  read, each name must be snake case, and gate 10 then checks the file declares each name). Two files in one cell: two
  such groups, `;` between them. Not FNO-LADDER's `` `path`: `name`, ... `` form (fix/fib-fno-ladder
  docs/04-invariants.md:9125-9126), which `references` does not read. A C-PSRB row names its bench test the same way.
- **Gate 17**: an `indicators` module emits no telemetry and prints nothing (FP:268).
- **Bench shape (FP:267).** The measuring functions live in the module (`pub fn measure_step`, `pub fn measure_period`,
  each returning `rounds` vectors of `samples` unsorted nanosecond timings, plus `StepCost`, `summarize`, `flat_within`
  in FNO-LADDER's form, `fix/fib-fno-ladder:crates/indicators/src/fno_ladder/mod.rs:1253-1291`). A unit test runs them at
  tiny sizes. The wrapper `benches/ratio/psrb_<x>.rs` sorts (gate 11 keeps the sort out of the crate), prints for each
  size the best round's p50, p99 and max with n, plus the median and worst rounds' p99, and judges the best round (the
  crate's minimum-over-trials rule, benches/ratio.rs:87-100, judged as FNO-LADDER's rows judge it on
  `fix/fib-fno-ladder`, whose decision is not in docs/05 at START: cite none for it, 2.5): p99 flat within `CEILING_PERMILLE`
  (3.0x, :75) of the first size, and inside the row's budget. `pub(crate) fn row(floor: u128, ids: &[&str]) -> bool`
  prints each id as the first word of a line. Each part keeps its own copy of the three helpers: a shared helper would
  make one part wait on another (accepted duplication, recorded in each part's decision on benches).
- **Commits** end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (RULES-MAC PRE "RULES").
- **Facts only.** No sentence in code, docs or a report claims a measurement the session did not take (CLAUDE.md
  section 3 rule 6). TradingView's definitions are as RA read them on 3 Nov 2026 (RA `definitions` D2-D4, `sources`);
  their charter rows are PSRB-LAW's (section 6.1). Until then each module says "the published definition, UNVERIFIED
  against TradingView's own output".

### 2.5 Decision ids and row ids

| part | decisions (docs/05) | invariants (docs/04) | benches (docs/04) | limits (docs/06) | plan (docs/07) |
|---|---|---|---|---|---|
| PSRB-RSI | D-8580..D-8584 (D-8585..D-8589 spare, for repairs) | PSRB-RSI-I01..I06 | C-PSRB-RSI-01, -02 | L-PSRB-RSI-01..03 | one row |
| PSRB-BOLL | D-8590..D-8595 (D-8596..D-8599 spare) | PSRB-BOLL-I01..I06 | C-PSRB-BOLL-01, -02 | L-PSRB-BOLL-01..04 | one row |
| PSRB-STV | D-8600..D-8605 (D-8606..D-8609 spare) | PSRB-STV-I01..I06 | C-PSRB-STV-01, -02 | L-PSRB-STV-01..04 | one row |

A part takes ids only from its own block, in order. Before writing, a session greps docs/05 for `^#{2,4} +D-85[89][0-9]`
and `^#{2,4} +D-860[0-9]`: only an entry heading counts as a clash (TEMPLATE-continue.md note (c)); a hit is
`STATUS: BLOCKED QUESTION`.

**Cite only ids that head an entry (repair 2, check2 N6).** Every `D-nnnn` in a `.rs`, `.toml` or `.md` file under
`crates/` must head a docs/05 entry (`decision_citations`, crates/core/tests/decision_citations.rs:1-3 and :41): one at
START or one your own block writes. FNO-LADDER's D-8138 heads an entry only on `fix/fib-fno-ladder` (its docs/05:87767),
none at START, and the 24 Oct ids of 2.2 (D-4896, D-7040, D-7093) head none at START either, so no PSRB file cites them.

---

## 3. PSRB-RSI (wave 1; D-8580..D-8589)

**Goal.** Wilder's RSI over closes as an exact-integer streaming reading with a runtime period and a warm flag, in the
TradingView form (RA D3): `change = C_t - C_(t-1)`; `avgGain = RMA(max(change, 0), n)`, `avgLoss = RMA(max(-change, 0), n)`,
the RMA seeded by the simple mean of the first n values; `RSI = 100 - 100/(1 + avgGain/avgLoss)`; and 100 when
`avgLoss = 0`, the guard of TradingView's built-in RSI script (not its help page: verifier, section 1). First run: the
PSRB first run needs it (C3, WU section 3). Size: 2-3 h to write once, S (WU section 9, ESTIMATE).

### 3.1 Files
- N `crates/indicators/src/rsi.rs`, N `crates/indicators/src/rsi_tests.rs`, N `crates/indicators/benches/ratio/psrb_rsi.rs`.
- E `crates/indicators/src/lib.rs`: `pub mod rsi;` after `pub mod pattern;` (:70).
- E `crates/indicators/benches/ratio.rs`: under `// slot bench-mod PSRB-RSI`:
  `#[path = "ratio/psrb_rsi.rs"]` / `mod psrb_rsi;`; under `    // slot bench-row PSRB-RSI`:
  `let verdicts: Vec<bool> = verdicts.iter().copied().chain([psrb_rsi::row(floor, &["C-PSRB-RSI-01", "C-PSRB-RSI-02"])]).collect();`
  (rustfmt wraps it as it wraps FNO-LADDER's).
- E the four ledgers at the anchors of 2.2 (section titles in 3.6).

### 3.2 Public API (`indicators::rsi`)

```rust
pub const MAX_LEVEL_PPM: u32 = 1_000_000;          // RSI 100, in parts per million of the 0..100 scale
pub const FAITHFUL_FACTOR: i128 = 1_000;           // faithful(): G + L >= FAITHFUL_FACTOR * period (3.3; repair 1)
pub enum Refusal { PeriodZero, LevelAboveMax { ppm: u32 }, Corrupt(crate::Corrupt) }   // Clone, Copy, Debug, PartialEq, Eq
pub struct Level(u32);                              // Copy; 0..=MAX_LEVEL_PPM
impl Level { pub const fn from_ppm(ppm: u32) -> Result<Self, Refusal>; pub const fn ppm(self) -> u32; }
pub struct Rsi { /* period: u16, previous close, changes: u32 (saturating, 3.3), two scaled i128 accumulators */ }  // Clone, Copy, Debug, PartialEq, Eq
impl Rsi {
    pub fn new(period: u16) -> Result<Self, Refusal>;           // PeriodZero for 0; 1..=u16::MAX accepted
    pub const fn period(&self) -> u16;
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;
    pub fn warm(&self) -> bool;                                  // at least `period` changes folded
    pub fn reading(&self) -> Option<Reading>;                    // None until warm
}
pub struct Reading { /* period: u16; avg_gain, avg_loss: i128, scaled by trend::SCALE */ } // Clone, Copy, Debug, PartialEq, Eq
impl Reading {
    pub const fn period(&self) -> u16;                           // the Rsi's period: a kept copy judges itself
    pub const fn avg_gain_scaled(&self) -> i128;
    pub const fn avg_loss_scaled(&self) -> i128;
    pub const fn flat(&self) -> bool;                            // both averages exactly 0
    pub fn faithful(&self) -> bool;                              // G + L >= FAITHFUL_FACTOR * period (3.3)
    pub fn ppm(&self) -> u32;                                    // floor(10^6 * G / (G + L)); 1_000_000 when L == 0
    pub fn above(&self, level: Level) -> bool;                   // RSI > level, exact
    pub fn below(&self, level: Level) -> bool;                   // RSI < level, exact
}
pub struct StepCost { pub p50_ns: u64, pub p99_ns: u64, pub max_ns: u64, pub samples: usize }
pub fn summarize(sorted: &[u64]) -> StepCost;
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool;
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>;   // period 14
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>; // history 20_000
```

### 3.3 Exact formulas and warm-up
- `fold(c)`: `c.check_evaluable()` refused first (so every close is a positive `i64`). The first candle only records its
  close. Each later candle gives `change = close - previous` in `i128`; `gain = max(change, 0) * SCALE`,
  `loss = max(-change, 0) * SCALE`; `changes = changes.saturating_add(1)`, with `changes` a `u32` (repair 2, check2
  B1): it saturates at `u32::MAX`, far above any `u16` period, so once `changes > n` holds it holds for good. Not a
  `u16`: at period `u16::MAX` a `u16` counter saturates at 65,535 = n, meets `changes == n` again on the next change
  and re-seeds instead of taking a Wilder step, and every later reading is wrong, yet no other test of 3.5 checks a
  Wilder step at that period and 100% coverage holds (check2 B1; `repair2-work/counter-r2.out` runs the two that reach
  it, and both pass with a `u16`); `the_largest_period_smooths_after_its_seed` pins it.
- Seed and Wilder, with `changes` already counted for this candle: `changes < n` adds the scaled gain and loss to the
  two sums; `changes == n` adds them and seeds, `G = gain_sum.div_euclid(n)` and `L = loss_sum.div_euclid(n)` (the
  simple mean, floored); `changes > n` takes the Wilder step `G += (gain - G).div_euclid(n)`, same for L (Wilder,
  alpha = 1/n, floored, the form `trend::Atr` uses, trend.rs:329-338). The first reading is at candle n (0-based), after
  n changes, exactly where TradingView's `ta.rma` seeded by `ta.sma` gives its first value (RA D3; the na at bar 0 makes
  the first SMA cover changes 1..n).
- `ppm`: `L == 0` gives 1_000_000 (this covers the flat case, both zero: the built-in script's `down == 0 ? 100`); else
  `floor(1_000_000 * G / (G + L))`, which is `100 - 100/(1 + G/L)` in ppm (algebra: `100 G/(G + L)`).
- `above(level)`: `L == 0` and `G == 0` reads as RSI 100 (`level < 1_000_000`); otherwise `1_000_000 * G > level * (G + L)`.
  `below(level)`: `1_000_000 * G < level * (G + L)` (false for the flat case). A tie is neither above nor below.
- Bounds (const-asserted): `|change| < 2^63`, `gain, loss < 2^83` (SCALE < 2^20), seed sums `< 2^99` (n < 2^16),
  `1_000_000 * G < 2^103`, `level * (G + L) < 2^105`, `FAITHFUL_FACTOR * period < 2^26` (repair 1): all inside `i128`
  (2^127), so no checked or saturating arithmetic is needed and none hides a wrong value.
- **Truncation, stated (L-PSRB-RSI-01):** each floored step leaves each average within `n` units of `10^-6` paisa of the
  exact real Wilder average (error recurrence `e' = e(1 - 1/n) + d`, `0 <= d < 1`), and never above it: with `g`, `l`
  the exact averages in the same units, `g = G + a` and `l = L + b` with `0 <= a, b < n`. The comparisons are exact on
  the integer state; a state within that bound of a level can compare differently from real arithmetic.
- **The RSI is a ratio, so its bound is relative (L-PSRB-RSI-01; repair 1, check1 B1).** For `G + L > 0`:
  `G/(G + L) - g/(g + l) = (bG - aL) / ((G + L)(g + l))`, and `|bG - aL| < n max(G, L) <= n (G + L)`, so
  `|G/(G + L) - g/(g + l)| < n/(g + l) <= n/(G + L)`: the reading is within `100 n/(G + L)` RSI points
  (`10^6 n/(G + L)` ppm) of exact Wilder. The per-average bound holds at every step, yet this one has no limit as `G + L`
  shrinks.
- **A long run of unchanged closes.** In exact arithmetic an unchanged close scales both averages by `(n - 1)/n`, so the
  RSI does not move. The floored state takes `ceil(X/n)` off each average (`X' = floor((n - 1) X/n)`), so each reaches
  exactly 0 after a finite run, the smaller first. Until the other also does, the reading is exactly 0 (`G == 0`) or
  exactly 100 (`L == 0`, `flat()` false), which looks the same as a real all-down or all-up RSI; then both are 0 and the
  reading is 100 by the built-in script's convention, `flat()` true. Measured on this file's integer rule against exact
  rationals (`repair1-work/rsi-run.out`, written apart from check1's model, which agrees: `check1-work/rsi-decay.out`):
  after the closes 100, 103, 101, ..., 112, 110 (3.5's long-run test), period 2 reads 0 after 20 unchanged closes
  (G 0, L 1) where the exact RSI is 42.857; period 5 reads 100 after 58 (G 1, L 0; exact 54.702); period 14 reads 100
  after 159 (G 5, L 0; exact 59.360). On three walks at Rs 1,000 (100,000 paisa) with 5-paisa moves the departures came
  after 20-21 (period 2), 60-61 (5) and 167-173 (14) unchanged closes.
- **The guard: `Reading::faithful()` = `G + L >= FAITHFUL_FACTOR * period`** (`FAITHFUL_FACTOR` = 1_000, compared in
  `i128`; `faithful` is not `const` because it widens `period` with `i128::from`, 2.4). By the ratio bound a faithful
  reading is within 0.1 RSI point (1_000 ppm) of exact Wilder (its floored `ppm` within 1_000 of the exact floor). Flat
  reads unfaithful (`G + L = 0`), and so does every reading more than 0.1 point from exact Wilder; so every 0/100
  departure from an exact RSI of at least 0.1 and at most 99.9 points reads unfaithful (a faithful 0 has an exact RSI
  below 0.1, a faithful 100 one above 99.9). A departure from an exact RSI within 0.1 point of 0 or 100 can read
  faithful, and is then within the guard's 0.1 point (repair 2, check2 N1: period 2, closes 100, 10100, 10099, then
  unchanged: after 19 unchanged closes G 9_536, L 0, `ppm` 1_000_000 against an exact floor of 999_900, `faithful()`
  true, pinned by 3.5's long-run test; `repair2-work/departure-r2.out`, where over 191,045 random readings no faithful
  departure had an exact RSI in 0.1..=99.9). In the runs above `faithful()` turns false after 11, 28 and 70 unchanged
  closes, long before the departures; over 32,690 random readings with unchanged runs mixed in, no faithful reading was
  more than 1_000 ppm off. **PSRB-STRAT decides only on faithful readings**, as it decides only on warm and settled
  ones (6.2). `ppm`, `above` and `below` stay exact on the state whatever `faithful()` says.
- **Why VIDEO-ORIGINAL's own 14/20 was safe anyway, and searched seeds are not.** A close that moves by `m >= 1` paisa
  leaves `G + L >= floor(10^6/n)` (the moved average becomes `floor(((n - 1) X + m 10^6)/n)`; a seed with any move in
  it is at least as large), which is at least `1_000 n` for every `n <= 31`, so such a candle's reading is faithful;
  each unchanged close then keeps `floor((n - 1) X/n)` of each average. At period 14 an unfaithful reading therefore needs at least 22 unchanged closes in a row
  (`repair1-work/min-run.out`), so the video's 20-close Bollinger window is one repeated close, sigma 0, and C4 (close
  above the upper band) is false (4.3): there a departure could not have made VIDEO-ORIGINAL's signal. That argument is
  for the video's values only. RSI period and band length are searched per stock and side (WU :367), and at period 2 a
  reading can be unfaithful after only 8 unchanged closes while a longer band still has spread; above period 31 one
  1-paisa move after a flat spell is not enough to make a reading faithful. Hence the guard, which holds at every
  period.

### 3.4 Refusals
`PeriodZero` (`new(0)`); `LevelAboveMax { ppm }` (`Level::from_ppm` above 1_000_000); `Corrupt(why)` (the candle's own
`check_evaluable` refusal, lib.rs:1778: inverted, out-of-range, overflowing range, negative volume, nonpositive price).
Each leaves the state unchanged.

### 3.5 Tests (`src/rsi_tests.rs`; deterministic xorshift, many seeds, no new dependency)

Fixture RSI-1 (period 3, closes 100, 103, 101, 104, 104, 98, 101, 107 paisa), derived by `work/fixtures.mjs` and by hand
in exact rationals (G, L as fractions in the last column; every value below is exact for this fixture):

| close | warm | G (scaled) | L (scaled) | ppm | exact |
|---|---|---|---|---|---|
| 100, 103, 101 | no | - | - | - | - |
| 104 | yes | 2_000_000 | 666_666 | 750_000 | G 6/3, L 2/3: RSI 75 |
| 104 | yes | 1_333_333 | 444_444 | 750_000 | 12/9, 4/9: 75 |
| 98 | yes | 888_888 | 2_296_296 | 279_069 | 24/27, 62/27: 27.906976...|
| 101 | yes | 1_592_592 | 1_530_864 | 509_881 | 129/81, 124/81: 50.988142... |
| 107 | yes | 3_061_728 | 1_020_576 | 750_000 | 744/243, 248/243: 75 |

- `a_zero_period_is_refused_and_every_other_period_is_accepted` (0; 1; 14; `u16::MAX`).
- `a_level_above_one_hundred_is_refused_at_the_edge` (repair 1, check1 B3: `Level::from_ppm` accepts 0 and 1_000_000
  and `ppm()` reads each back; refuses 1_000_001 and `u32::MAX` with `LevelAboveMax { ppm }` naming that value).
- `the_first_close_folds_no_change_and_reads_nothing`.
- `the_reading_appears_after_exactly_period_changes` (periods 1, 2, 3, 14, `u16::MAX`: none after `period` closes, one
  after `period + 1`).
- `the_largest_period_smooths_after_its_seed` (repair 2, check2 B1: period `u16::MAX`, `u16::MAX + 4` closes of a
  deterministic random walk that both rises and falls, so the seed is at close `u16::MAX` and the last three closes are
  the three Wilder steps after it. After each of those three closes the whole state, G and L, not only `ppm`, equals
  the independent loop of `the_stream_equals_a_from_scratch_recomputation`: three recomputations of about 65,540 steps
  each. Then white-box, in the child test module: a warm state at period 14 whose `changes` is set to `u32::MAX - 1`
  folds two more closes exactly as a copy with its true count does (G and L equal), and `changes` reads `u32::MAX`
  after both. A `u16` counter fails the first part at the first close after the seed (`repair2-work/counter-r2.out`:
  with the sums and the averages in the two shared fields `ppm` goes to 642_857 against the loop's 498_060; with the
  sums kept in fields of their own the state differs while `ppm` moves by at most 1, which is why the state is
  compared).
- `the_seed_is_the_floored_mean_of_the_first_period_moves` and `wilder_smoothing_matches_fixture_rsi_1` (the table).
- `a_flat_series_reads_one_hundred_and_says_it_is_flat` (100 closes of 2_500_000: `ppm` 1_000_000,
  `flat()` true, `above(Level 999_999)` true, `below(any)` false).
- `an_all_up_series_reads_one_hundred` and `an_all_down_series_reads_zero` (strictly rising and falling; `above` at
  999_999 and 1_000_000, `below` at 0 and 1).
- `equal_moves_at_the_seed_read_fifty` (period 2, closes 100, 101, 100: 500_000 exactly).
- `a_period_of_one_reads_the_last_change_alone` (up 1_000_000, down 0, unchanged flat).
- `a_tie_at_the_level_is_neither_above_nor_below` (fixture RSI-1 at its last close, 107, where the integer state is
  exactly 75: G 3_061_728 = 3 x L 1_020_576; `Level 750_000` neither; 749_999 above; 750_001 below).
- `a_floor_ppm_equal_to_the_level_is_not_a_tie` (RSI-1's first two readings: `ppm` 750_000, yet `above(Level 750_000)`
  is true, because the floored L leaves the state at 75.0000187... and 75.0000140... (100 G/(G + L) of the table's
  rows: 75.000018750004... and 75.000014062506..., `repair1-work/rsi-run.out`; check1 N1); the comparison reads the
  state, `ppm` is for records and seeds).
- `ppm_is_the_floor_of_the_exact_ratio` (random states with `G + L > 0`: `ppm * (G + L) <= 10^6 G < (ppm + 1)(G + L)`;
  repair 2, check2 N6: in the flat state the right side would read `0 < 0`, and that state is
  `a_flat_series_reads_one_hundred_and_says_it_is_flat`'s).
- `a_price_gap_is_one_move_and_nothing_more` (a 10% overnight jump equals the brute force with that move).
- `a_time_gap_reads_as_contiguous_candles` (stamps a day apart or a minute apart give equal states).
- `no_reading_depends_on_a_later_candle` (redraw every candle after t: the reading at t is unchanged).
- `the_stream_equals_a_from_scratch_recomputation` (BRUTE FORCE: at every candle recompute from the whole close history
  by an independent loop of the same integer rule, which counts changes by the history's index, never by a saturating
  counter; 1,000 random series, periods 1..=30).
- `the_fixed_point_stays_within_period_units_of_exact_wilder` (PARITY with the published definition: an exact
  rational transcription of RA D3's formulas, `num/den` in `i128`, on series short enough that `den = n^k` fits; checks
  `|G - SCALE * exact G| < n` and `|L - SCALE * exact L| < n` at every candle, both as exact cross-multiplications, and
  that fixture RSI-1's `ppm` column equals the floor of its exact RSI).
- `a_long_unchanged_run_departs_only_where_the_ratio_bound_allows` (repair 1, check1 B1; periods 2, 5 and 14: the
  closes 100, 103, 101, 104, 102, 105, 103, 106, 104, 107, 105, 108, 106, 109, 107, 110, 108, 111, 109, 112, 110, then
  `20 n + 60` closes of 110. The exact RSI does not move during the run, so the test computes it once, at the run's
  first close, in exact rationals `num/den` in `i128` (below 2^76 even unreduced, `repair1-work/ratio-bits.out`). At
  every candle of the run, while `G + L > 0`, it asserts the ratio bound cross-multiplied,
  `|G * den - num * (G + L)| < n * den`, and that `faithful()` implies `|ppm - floor(10^6 num/den)| <= 1_000`. It pins
  the integer rule's points (`repair1-work/rsi-run.out`): exact floors 428_572, 547_023 and 593_601 ppm; `faithful()`
  first false after 11, 28 and 70 unchanged closes; the first 0/100 departure after 20 (G 0, L 1, `ppm` 0), 58 (G 1,
  L 0, `ppm` 1_000_000) and 159 (G 5, L 0, `ppm` 1_000_000), each with `flat()` false and `faithful()` false; `flat()`
  first true after 21, 59 and 164. Repair 2 (check2 N1) adds a fourth run in the same form: period 2, the closes 100,
  10_100, 10_099, then 40 closes of 10_099 (exact RSI 10_000/10_001, floor 999_900 ppm; at period 2 an unchanged close
  floors each average to half, so each figure checks by hand; `repair2-work/departure-pins.out`): the first 0/100
  reading after 19 unchanged closes, G 9_536, L 0, `ppm` 1_000_000, with `faithful()` TRUE and `flat()` false, a
  departure from an exact RSI above 99.9 that reads faithful, 100 ppm from the exact floor; `faithful()` first false
  after 22; `flat()` first true after 33).
- `the_faithful_edge_is_one_thousand_units_per_period` (repair 1: white-box `Reading`s, built in the child test module,
  at periods 1, 14 and `u16::MAX`: `G + L = 1_000 n - 1` unfaithful and `1_000 n` faithful, each as all gain, all loss
  and split in two; flat unfaithful at every period. Through `fold` at period 14: 15 closes of 100 (flat, unfaithful),
  then 101 (G 71_428, L 0: faithful), then closes of 101: faithful through the 21st and unfaithful from the 22nd, the
  `min-run.out` figure behind 3.3's 14/20 argument).
- `the_largest_moves_and_period_never_overflow` (closes alternating 1 and `i64::MAX - 1` with periods 1 and `u16::MAX`;
  `ppm` stays in 0..=1_000_000; the const bounds are restated as runtime checks at the extremes).
- `a_corrupt_candle_is_refused_and_changes_nothing` (each `Corrupt` kind; whole state compared).
- `the_state_is_a_fixed_size_and_never_allocates` (`size_of::<Rsi>()` asserted, at most 96 bytes, ESTIMATE).
- `the_bench_measures_both_rows_at_tiny_sizes` and `a_cost_is_read_by_rank_and_judged_flat_or_not` (cover
  `measure_step`, `measure_period`, `summarize`, `flat_within`, including a zero first p99 failing).

### 3.6 Benches, rows and ledger blocks
- **C-PSRB-RSI-01**: one `fold` plus `reading()` and both comparisons, after 2_000, 20_000 and 200_000 candles, period 14.
  p99 flat within 3.0x of the first size, inside a 2 µs budget (PARTS target 300 ns, ESTIMATE).
- **C-PSRB-RSI-02**: the same step at periods 14, 140 and 1_400 after 20_000 candles: the period never enters the
  per-candle cost. Same ceiling and budget.
- docs/04 section `## Wilder RSI for the PSRB family (PSRB-RSI; D-8580 to D-8584)` with `| Id | Invariant | Proof | |`:
  - PSRB-RSI-I01: the first reading after exactly `period` changes, seeded by the floored mean, Wilder after, at every
    period up to `u16::MAX` (fixture RSI-1 tests, warm-up tests, `the_largest_period_smooths_after_its_seed`).
  - PSRB-RSI-I02: comparisons exact on the state; a tie is neither; `ppm` is the floor (tie and floor tests).
  - PSRB-RSI-I03: flat reads 100 with `flat()`; all up 100; all down 0; period 1 the last change (four tests).
  - PSRB-RSI-I04: the stream equals the from-scratch recomputation and stays within `period` units of exact Wilder; the
    RSI stays within `10^6 n/(G + L)` ppm of exact Wilder, within 1_000 ppm whenever `faithful()`, and a long unchanged
    run departs to 0 or 100 only as 3.3 states (brute-force, parity, long-run and faithful-edge tests).
  - PSRB-RSI-I05: every refusal changes nothing, `Level`'s at its edge; no overflow at any positive `i64` close and any
    period, and the `u32` change counter never re-seeds, saturated or not (refusal, level-edge and extremes tests,
    `the_largest_period_smooths_after_its_seed`).
  - PSRB-RSI-I06: no reading depends on a later candle or on a timestamp (look-ahead and time-gap tests).
  - C-PSRB-RSI-01, C-PSRB-RSI-02 rows naming the two bench tests.
- docs/06 section `## Wilder RSI for the PSRB family: limits (PSRB-RSI; D-8580 to D-8584)`, FNO-LADDER's columns
  `| ID | Limit | Why it is not O(1) / not closed | Bound | Package |`: L-PSRB-RSI-01 the fixed-point bounds above (each
  average within `n` units, the RSI within `10^6 n/(G + L)` ppm, so a long run of unchanged closes departs to 0 or 100,
  and every reading that may be more than 1_000 ppm off reads `faithful()` false);
  L-PSRB-RSI-02 what was measured (the cloud figure is advisory; the Mac's figure is owed until its check; never a
  number not measured); L-PSRB-RSI-03 no real candle has passed through it until PSRB-STRAT.
- docs/07 section `## PSRB-RSI: Wilder RSI — 2026-11-03 (D-8580 to D-8584)` with `| Part | What | State |` and one row
  ("Built on `cloud/psrb-rsi-20261103`; not merged (D1). PSRB-STRAT is its first caller").
- docs/05 entries (`### D-858n — PSRB-RSI: ... — 2026-11-03`):
  - D-8580: Wilder RSI as `indicators::rsi`, no vocabulary position; runtime period `u16` 1..=65,535; the TradingView
    form (RA D3) with the first reading after `period` changes; the change counter a `u32`, saturating far above every
    period, so a Wilder step never re-seeds (3.3); carried across sessions, a daily reset is a new state.
  - D-8581: units: averages scaled by `trend::SCALE` in `i128`, floored seed and steps, the truncation bound and the
    RSI's ratio bound with its one-line proof (L-PSRB-RSI-01, 3.3); `faithful()` and `FAITHFUL_FACTOR` (within 0.1 RSI
    point when true); exact comparisons by cross-multiplication; `ppm` the floor; bounds const-asserted.
  - D-8582: both averages zero read 100, as TradingView's built-in RSI script does (`down == 0 ? 100`; secondary sources,
    UNVERIFIED on the help page, which leaves the case open: verifier), the VIDEO-ORIGINAL reading, with `flat()` so a
    caller can treat it as undefined, a labelled variant. In a long run of unchanged closes one average reaches 0 before
    the other, and the reading is exactly 0 or 100 with `flat()` false where the exact RSI has not moved; every reading
    more than 0.1 point from exact Wilder reads `faithful()` false, so such a reading is unfaithful whenever the exact
    RSI is at least 0.1 and at most 99.9 points, and one that reads faithful is within 0.1 point of exact (3.3; repair
    2, check2 N1); PSRB-STRAT decides only on faithful readings. The argument that such a reading cannot make
    VIDEO-ORIGINAL's signal holds only at the video's 14/20 (3.3), never for searched periods.
  - D-8583: refusals and validation order; timestamps not read; no look-ahead.
  - D-8584: C-PSRB-RSI-01 and -02, their sizes, ceiling and 2 µs budget; the helpers copied per part (2.4).

### 3.7 NEEDS THE MAC
Check 1 on the Mac's pin (Route B on the D2 set against the seam commit; the bench wrapper and `main` by the
instrumented bench run), its gate 8 run of `cargo bench -p indicators --bench ratio` with both C-PSRB-RSI rows,
gates 10, 11, 12 and 14 (gate 14's cover-table row for the new ids is the main session's at the family assembly, as for
F1: RULES-MAC R2 "(b)"), macOS rows (none expected: no platform code), and the anchors on the then-current feat/db-fix.

---

## 4. PSRB-BOLL (wave 1; D-8590..D-8599)

**Goal.** Bollinger Bands over closes in the TradingView form (RA D4): `basis = SMA(close, n)`, `upper = basis + z sigma`,
`lower = basis - z sigma`, sigma the population standard deviation of the same n closes, the current candle included,
with exact band comparisons, a runtime `n` in a ring allocated once, O(1) per candle. Sample sigma is a labelled reading
(RA A9). First run: needed (C4). Size: 3-5 h, M (WU section 9, ESTIMATE).

### 4.1 Files
- N `crates/indicators/src/bollinger.rs`, N `crates/indicators/src/bollinger_tests.rs`, N `crates/indicators/benches/ratio/psrb_boll.rs`.
- E `crates/indicators/src/lib.rs`: `pub mod bollinger;` after `pub mod anchored;` (:63).
- E `crates/indicators/benches/ratio.rs`: under `// slot bench-mod PSRB-BOLL` and `    // slot bench-row PSRB-BOLL`
  (ids `"C-PSRB-BOLL-01", "C-PSRB-BOLL-02"`), the 3.1 form.
- E the four ledgers at the anchors of 2.2.

### 4.2 Public API (`indicators::bollinger`)

```rust
pub const MAX_PERIOD: u16 = 4_096;          // closes in the window
pub const MAX_CLOSE: i64 = 1 << 32;         // 4_294_967_296 paisa = Rs 4.29 crore a share
pub const MAX_WIDTH_MILLI: u32 = 10_000;    // z at most 10.000
pub const WIDTH_DEN: i128 = 1_000;          // z is in thousandths
pub enum Refusal { PeriodZero, PeriodAboveMax { period: u16 }, CloseOutOfRange { close: i64 },
                   WidthAboveMax { milli: u32 }, RingOutOfStep, Corrupt(crate::Corrupt) }
pub struct Close(i64);                       // Copy; 1..=MAX_CLOSE
impl Close { pub const fn new(paisa: i64) -> Result<Self, Refusal>; pub const fn paisa(self) -> i64; }
pub struct Width(u32);                       // Copy; z in thousandths, 0..=MAX_WIDTH_MILLI
impl Width { pub const fn from_milli(milli: u32) -> Result<Self, Refusal>; pub const fn milli(self) -> u32; }
pub struct Bollinger { /* ring: Vec<i64> (len = period, zeros), next: usize, filled: u16, sum: i128, sum_sq: i128,
                          last: Close (stored by fold, 4.3) */ }
impl Bollinger {
    pub fn new(period: u16) -> Result<Self, Refusal>;           // allocates the ring once
    pub const fn period(&self) -> u16;
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;
    pub const fn warm(&self) -> bool;                            // `period` closes held
    pub fn window(&self) -> Option<Window>;                      // None until warm
}
pub struct Window { /* period: u16, sum: i128, sum_sq: i128, last: Close */ }   // Clone, Copy, Debug, PartialEq, Eq
impl Window {
    pub const fn period(&self) -> u16;  pub const fn last(&self) -> Close;   // not `len`: clippy len_without_is_empty
    pub const fn sum(&self) -> i128;  pub const fn sum_sq(&self) -> i128;
    pub fn variance_numerator(&self) -> i128;                    // V = n*Q - S^2 >= 0; population sigma^2 = V / n^2
    pub fn basis_floor(&self) -> i64;                            // floor(S / n)
    pub fn above_upper(&self, close: Close, width: Width) -> bool;           // close > basis + z*sigma, exact
    pub fn below_lower(&self, close: Close, width: Width) -> bool;           // close < basis - z*sigma, exact
    pub fn upper_floor(&self, width: Width) -> i64;              // floor of the real upper band
    pub fn lower_ceil(&self, width: Width) -> i64;               // ceiling of the real lower band
    pub fn z_milli(&self, close: Close) -> Option<i64>;          // floor(1000 * (close - basis) / sigma); None when V = 0
    pub fn above_upper_sample(&self, close: Close, width: Width) -> Option<bool>;   // None when n = 1
    pub fn below_lower_sample(&self, close: Close, width: Width) -> Option<bool>;
}
// StepCost, summarize, flat_within, measure_step (n = 20), measure_period (history 20_000): the 3.2 forms.
```

### 4.3 Exact formulas
- `fold(c)`: `check_evaluable`, then `Close::new(c.close)` (refused above `MAX_CLOSE`), then ONE `get_mut(next)` whose
  `None` arm returns `RingOutOfStep` (unreachable through the API; a white-box test builds the state), then
  `old = replace(slot, close)`, `sum += c - old`, `sum_sq += c^2 - old^2` (the ring starts at zeros, so the filling phase
  needs no branch), `filled = min(filled + 1, period)`, `next = next + 1` or 0 at the end, `last = close` (repair 2,
  check2 N5: `window()` copies `last` and never reads the ring, because a second ring access would bring a `None` arm
  no test can reach; `new` starts `last` at `Close(1)`, which no reading shows, since `window()` is `None` until warm).
- With `S = sum`, `Q = sum_sq`, `n`, `V = nQ - S^2` (`= n^2 sigma^2`, so `V >= 0`), and `D = n * close - S`:
  - `above_upper`: `close > S/n + z sqrt(V)/n` iff `D * 1000 > z_milli * sqrt(V)` iff `D > 0 && (D * 1000)^2 > z_milli^2 * V`.
  - `below_lower`: the same with `-D`.
  - sample sigma `sqrt(V / (n (n - 1)))`: `D > 0 && (D * 1000)^2 * (n - 1) > z_milli^2 * n * V`; `None` for `n = 1`.
  - `upper_floor = (S * 1000 + isqrt(z_milli^2 * V)).div_euclid(n * 1000)`, using `crate::vwap::isqrt_i128`
    (vwap.rs:170): exact, because `floor((a + y)/m) = floor((a + floor(y))/m)` for integers `a`, `m > 0` and real
    `y >= 0`, and `floor(z sqrt(V)) = isqrt(z^2 V)`. `lower_ceil = -((isqrt(z_milli^2 * V) - S * 1000).div_euclid(n * 1000))`.
  - For every integer close, `above_upper(close) == (close > upper_floor)` and `below_lower(close) == (close < lower_ceil)`:
    the squares test and the floor band are two exact readings of one fact, and a test proves they agree.
  - `z_milli`: `V = 0` gives `None`; `D >= 0` gives `isqrt((10^6 * D^2).div_euclid(V))`; `D < 0` gives the negated
    ceiling (`k = isqrt(...)`, ceiling `k` when `k^2 V == 10^6 D^2`, else `k + 1`).
- Bounds (const-asserted): with `n <= 2^12`, `close <= 2^32`, `z_milli < 2^14`: `|D| <= 2^44`, `V <= n^2 close^2 / 4 <= 2^86`,
  `(D * 1000)^2 * n <= 2^120`, `z_milli^2 * n * V <= 2^125`, `10^6 * D^2 <= 2^108`: inside `i128`. A bigger price, window
  or width is refused, never wrapped (L-PSRB-BOLL-01).
- Warm-up: no window until `period` closes; the window includes the current candle (TradingView's default, offset 0).
- Cost: `fold` and every comparison are a fixed handful of `i128` operations; only the band values and `z_milli` call
  `isqrt_i128`, which is bounded by `ITERATION_CEILING` = 16 (vwap.rs:212-225, C-I-03), not flat (docs/06 section 51);
  decisions never need them (L-PSRB-BOLL-02). The ring is the only memory that grows with `period`: 8 bytes a close,
  32 KiB at `MAX_PERIOD`, allocated once in `new` (L-PSRB-BOLL-03).

### 4.4 Refusals
`PeriodZero`, `PeriodAboveMax { period }` (above 4,096), `CloseOutOfRange { close }` (a folded close above `MAX_CLOSE`;
`Close::new` also refuses 0 and below), `WidthAboveMax { milli }`, `RingOutOfStep` (a state no API call can make),
`Corrupt(why)`. Each leaves the state unchanged.

### 4.5 Tests (`src/bollinger_tests.rs`)

Fixture BOLL-1 (n = 8, closes 200, 400, 400, 400, 500, 500, 700, 900; the textbook population example, mean 500,
sigma 200): S 4_000, Q 2_320_000, V 2_560_000, D 3_200. z 2.000: the close 900 is ON the upper band (not above),
`upper_floor` 900, `lower_ceil` 100; z 1.999: above, `upper_floor` 899, `lower_ceil` 101; z 2.001: not above,
`upper_floor` 900, `lower_ceil` 100. Fixture BOLL-2 (n = 20, closes 100..=119, a steady trend): S 2_190, Q 240_470,
V 13_300, D 190; `z^2 = D^2/V = 57/21 = 3(n - 1)/(n + 1)`, z = 1.6475...: above at z 1.647 (`upper_floor` 118), not
above at 1.648 (`upper_floor` 119); `z_milli` 1_647. Its mirror (119 down to 100): below at 1.647 (`lower_ceil` 101),
not at 1.648 (`lower_ceil` 100). BOLL-2 is WU section 6's "about 1.65 standard deviations", now exact. (All values from
`work/fixtures.mjs`, and by hand.)

- `a_zero_period_and_one_past_the_ceiling_are_refused` (0 and 4,097 refused, and `u16::MAX` with
  `PeriodAboveMax { period: 65_535 }`; 1 and 4,096 accepted, ring length equal).
- `a_close_outside_its_range_is_refused_and_changes_nothing` (`MAX_CLOSE` folds; `MAX_CLOSE + 1` and `i64::MAX` refuse,
  the state unchanged. `Close::new`, public, at both edges (repair 1, check1 B3): 1 and `MAX_CLOSE` accepted and read
  back by `paisa()`; 0, -1, `i64::MIN`, `MAX_CLOSE + 1` and `i64::MAX` refused with `CloseOutOfRange { close }` naming
  the value. Through `fold` a close of 0 or below is the candle's own `Corrupt` refusal first, `check_evaluable`).
- `a_width_past_its_ceiling_is_refused` (10,000 accepted, 10,001 and `u32::MAX` refused with `WidthAboveMax { milli }`,
  0 accepted).
- `a_zero_width_band_is_the_basis` (repair 1, check1 B3: on BOLL-1, V 2_560_000 > 0, at width 0: `above_upper` iff
  `D > 0` (501 above, 500 neither, the basis exactly), `below_lower` iff `D < 0` (499 below); `upper_floor` =
  `basis_floor` = 500 = `lower_ceil`; on a window whose mean is not whole (BOLL-1 with its last close 901: S 4_001,
  basis 500.125), `upper_floor` 500 and `lower_ceil` 501, and 500 below, 501 above; the sample readings agree at width 0).
- `a_corrupt_candle_is_refused_and_changes_nothing`.
- `no_window_before_period_closes_and_one_at_the_period_th`.
- `the_basis_and_sigma_match_fixture_boll_1` and `a_close_exactly_on_the_band_is_not_beyond_it` (BOLL-1 at z 1.999,
  2.000, 2.001; the lower side by the mirrored fixture).
- `a_steady_trend_sits_one_point_six_four_seven_sigma_above_its_mean` (BOLL-2 and its mirror; `z_milli` 1_647 and -1_648
  for the mirror's floor).
- `a_flat_window_has_zero_sigma_and_the_close_on_both_bands` (V 0; neither above nor below at any width; `upper_floor`
  = `lower_ceil` = basis; `z_milli` None).
- `a_window_of_one_close_has_zero_sigma_and_no_sample_sigma` (n = 1).
- `sample_sigma_is_wider_by_the_root_of_n_over_n_minus_one` (BOLL-1: sample sigma 200 sqrt(8/7) = 213.8...; z values on
  both sides of the population and sample bands).
- `an_all_up_and_an_all_down_series` (BOLL-2 at every n in 2..=60: the last close is above the band iff
  `z^2 < 3(n - 1)/(n + 1)`).
- `a_gap_enters_the_window_and_leaves_it_after_period_closes` (sum and sum of squares equal the brute force at every
  step, the gap close gone exactly `period` closes later).
- `the_square_test_and_the_floor_band_agree_on_every_integer_close` (random windows; every close in
  `[upper_floor - 3, upper_floor + 3]` and the lower mirror).
- `the_ring_equals_a_brute_force_window` (BRUTE FORCE: random series, periods 1..=64, sums and every comparison against a
  direct recomputation of the last n closes at every candle).
- `parity_with_the_published_definition` (RA D4 transcribed in exact rationals: basis `S/n`, variance `V/n^2`, the band
  test decided by squares in a slow path written apart from the module; random windows and the two fixtures).
- `the_extremes_never_overflow` (n 4,096 of alternating 1 and `MAX_CLOSE`, width 10,000, both dispersions, `z_milli`,
  both band values; the const bounds restated at runtime at the extremes).
- `a_ring_out_of_step_is_refused_by_name_and_changes_nothing` (white-box: `next == period`).
- `the_previous_window_reads_the_current_candle_excluded` (RA A7: a copied window judges the next close).
- `the_ring_never_grows_after_new` (length and capacity unchanged after 100,000 folds).
- the two bench tests of 3.5's form.

### 4.6 Benches, rows and ledger blocks
- **C-PSRB-BOLL-01**: one `fold`, `window()`, `above_upper` and `below_lower` (population, z 2.000), after 2_000, 20_000
  and 200_000 candles, n 20; flat within 3.0x, inside 2 µs (PARTS target 400 ns, ESTIMATE).
- **C-PSRB-BOLL-02**: the same step at n 20, 200 and 2_000 after 20_000 candles: the ring's length never enters the
  per-candle cost.
- docs/04 `## Bollinger Bands for the PSRB family (PSRB-BOLL; D-8590 to D-8595)`: PSRB-BOLL-I01 the window is the last
  n closes including the current one, none before n; I02 the squares test equals `close > upper_floor` (and the lower
  mirror) for every integer close, a close on the band is not beyond it, and at width 0 the band is the basis; I03
  population and sample sigma match the fixtures, flat sigma 0, n = 1, the steady-trend z; I04 the ring equals the brute
  force; I05 every product fits `i128` (const-asserted) and every input outside the domain is refused at both of its
  edges with nothing changed; I06 no reading depends on a later
  candle or a timestamp; C-PSRB-BOLL-01, -02.
- docs/06 `## Bollinger Bands for the PSRB family: limits (PSRB-BOLL; D-8590 to D-8595)`: L-PSRB-BOLL-01 the domain
  (4,096 closes, 2^32 paisa, z 10.000), refused above; L-PSRB-BOLL-02 the band values and `z_milli` are bounded by
  `isqrt_i128`'s 16 iterations, not flat, and no decision uses them; L-PSRB-BOLL-03 the ring is O(n) memory allocated
  once, 32 KiB at most; L-PSRB-BOLL-04 what was measured (as L-PSRB-RSI-02).
- docs/07 `## PSRB-BOLL: Bollinger Bands — 2026-11-03 (D-8590 to D-8595)`, one row.
- docs/05: D-8590 the module, the TradingView form, the current candle included, no vocabulary position, runtime n
  1..=4,096 in a ring allocated once; D-8591 exact comparisons by squares, the domain bounds and why each (every product
  inside `i128`, const-asserted; Rs 4.29 crore a share is far above any NSE price); D-8592 population sigma is the
  reading (TradingView's default, RA D4, "secondary sources only" per RA `unverified`), sample sigma a labelled reading,
  `None` at n = 1; D-8593 band values and `z_milli` exact via `isqrt_i128` for records and seeds, never for a decision;
  D-8594 `RingOutOfStep` named and unreachable; D-8595 the two benches, their budget and the helpers copied per part.

### 4.7 NEEDS THE MAC
As 3.7, with the C-PSRB-BOLL rows.

---

## 5. PSRB-STV (wave 1; D-8600..D-8609)

**Goal.** The standard two-band Supertrend, as TradingView documents it and TA-Lib's standard form describes it (RA D2;
RB `evidence`, the ta-lib.org page), as a second, labelled reading beside the latch `trend::SuperTrend`, which is left
untouched. Both bands ratchet on every candle whether active or not; the trend turns when the close crosses the CURRENT
candle's band. First run: needed for VIDEO-ORIGINAL (WU section 8), with the latch kept as the labelled engine reading
(WU section 11 decision 3). Size: 2-3 h, S (WU section 9, ESTIMATE).

### 5.1 Files
- N `crates/indicators/src/dual_band_supertrend.rs`, N `crates/indicators/src/dual_band_supertrend_tests.rs`,
  N `crates/indicators/benches/ratio/psrb_stv.rs`.
- E `crates/indicators/src/lib.rs`: `pub mod dual_band_supertrend;` after `pub mod daily;` (:65).
- E `crates/indicators/benches/ratio.rs`: under the two PSRB-STV slots (ids `"C-PSRB-STV-01", "C-PSRB-STV-02"`).
- E the four ledgers at the anchors of 2.2. NOT `trend.rs` (the check proves `git diff` of it is empty).

### 5.2 Public API (`indicators::dual_band_supertrend`)

```rust
pub const MAX_FACTOR_MILLI: u32 = 100_000;     // 100x ATR
pub enum Refusal { PeriodZero, FactorOutOfRange { milli: u32 }, Corrupt(crate::Corrupt) }
pub struct DualBandSuperTrend { /* atr: crate::trend::Atr, factor_milli: i128, previous_close: Option<i64>,
                                   upper: i128, lower: i128, trend: crate::trend::Trend, seen_atr: bool,
                                   reading: Option<Reading> */ }   // Clone, Copy, Debug, PartialEq, Eq
impl DualBandSuperTrend {
    pub fn new(atr_period: u16, factor_milli: u32) -> Result<Self, Refusal>;   // period 1..; factor 1..=100_000
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;
    pub fn reading(&self) -> Option<Reading>;                    // None until the ATR has folded its period
    pub fn atr(&self) -> Option<i64>;                            // the reused ATR, once warm
}
pub struct Reading { pub trend: crate::trend::Trend, pub line: i128, pub upper: i128, pub lower: i128, pub settled: bool }
// StepCost, summarize, flat_within, measure_step (period 10, factor 3_000), measure_period (history 20_000).
```

### 5.3 Exact rules (the published per-candle rules, in the latch's own arithmetic)
- ATR: `crate::trend::Atr` reused unchanged (trend.rs:265-358: true range with the first candle's own high-low span,
  Wilder seed = mean of the first n, then 1/n smoothing, floored, `value()` in whole paisa). That is TradingView's
  `ta.atr` (RMA seeded by an SMA, RA D2), so the two Supertrend readings differ in band logic alone (D-8601).
- Band arithmetic exactly as the latch's (trend.rs:422-427): `mid = high.midpoint(low)` in `i128`,
  `band = (atr * factor_milli).div_euclid(1000)`, `basic_upper = mid + band`, `basic_lower = mid - band`. Bands are held in
  `i128` (`|band| < 2^80`), so no band is ever "absent" as the latch's `i64` stop can be (trend.rs:462-465).
- `fold(c)`: `check_evaluable`; fold a copy of the ATR; if it is not warm, keep it, record the close, and stop (no reading:
  TradingView plots nothing before its ATR exists). Otherwise, with `pc` the previous close (`None` at the very first
  candle) and `pu`, `pl` the previous bands (both 0 before the first ATR candle, which is what TradingView's `nz()` of
  its missing values gives):
  - `upper = basic_upper` if `basic_upper < pu` or `pc > pu`, else `pu`;
  - `lower = basic_lower` if `basic_lower > pl` or `pc < pl`, else `pl` (a `None` close compares false);
  - trend: at the first ATR candle, `Down` (TradingView's direction when the previous ATR is missing); after it, if the
    previous trend was `Down`: `Up` when `close > upper`, else `Down`; if it was `Up`: `Down` when `close < lower`, else `Up`;
  - `line = lower` when `Up`, `upper` when `Down`; `settled` = false at the first ATR candle, true after.
- **"The previous line was the upper band" is the previous trend, not a value comparison (D-8602).** TradingView's
  script decides it by `previous line == previous upper band`, which in floating point equals the previous trend except
  where the two bands are exactly equal. Whole paisa make equal bands far more common than floats do; fixture ST-9
  shows an up-candle that TradingView's literal comparison, run on integers, would turn down. The trend reading keeps the
  published meaning. Named for later parity runs (repair 1, check1 N5), not built here: LITERAL-PINE, the literal
  comparison on these integer bands, which on ST-9 reads `Down 152 (152, 134)`, `Down 151 (151, 137)` and
  `Down 144 (144, 137)` at candles 4-6 and agrees elsewhere (`repair1-work/st9.out`; check1's model agrees). A parity run
  against TradingView's own output reads such equal-band candles with this decision; TradingView's floats rarely make
  equal bands, so on them the previous-trend form is the closer reading.
- The first ATR candle's reading is TradingView's, degenerate cases included: with `atr_period = 1` the first candle
  has `upper = 0` (fixture ST-6), because the previous band is 0 and there is no previous close. PSRB-STRAT reads only
  settled readings (D-8604).

### 5.4 Fixtures where the two forms differ (prices in paisa; H, L, C; latch = trend.rs's `SuperTrend` with the same
period and factor; derived by `work/fixtures.mjs` and `work/fix2.mjs`, in the kit as `fixtures.mjs.md` and `fix2.mjs.md`,
each value checked against the rules above)

ST-1, the seed (period 2, factor 1_000): the latch seeds on the first candle's side; the two-band form starts down.

| i | H, L, C | ATR | latch | two-band trend line (upper, lower) |
|---|---|---|---|---|
| 0 | 110, 90, 100 | 20 | Up 80 | - |
| 1 | 112, 100, 110 | 16 | Up 90 | Down 122 (122, 90) first |
| 2 | 115, 105, 114 | 13 | Up 97 | Down 122 (122, 97) |
| 3 | 116, 108, 115 | 10 | Up 102 | Down 122 (122, 102) |
| 4 | 116, 109, 111 | 8 | Up 104 | Down 120 (120, 104) |
| 5 | 112, 96, 98 | 12 | Down 116 | Down 116 (116, 104) |
| 6 | 100, 90, 95 | 11 | Down 106 | Down 106 (106, 84) |

ST-2, a tighter line at a flip down (period 2, factor 1_500): both up on candles 6-9; at 10 both turn down, the latch
re-seeds its stop at the fresh basic band 140, the two-band form keeps its ratcheted upper band 138.

| i | H, L, C | latch | two-band |
|---|---|---|---|
| 0 | 126, 104, 106 | Down 148 | - |
| 1 | 118, 104, 115 | Down 138 | Down 138 (138, 84) first |
| 2 | 120, 109, 110 | Down 135 | Down 135 (135, 93) |
| 3 | 115, 108, 113 | Down 126 | Down 126 (126, 96) |
| 4 | 118, 101, 101 | Down 126 | Down 126 (126, 96) |
| 5 | 123, 100, 119 | Down 126 | Down 126 (126, 96) |
| 6 | 135, 118, 130 | Up 101 | Up 101 (126, 101) |
| 7 | 139, 126, 136 | Up 110 | Up 110 (154, 110) |
| 8 | 136, 119, 123 | Up 110 | Up 110 (151, 110) |
| 9 | 124, 116, 121 | Up 110 | Up 110 (138, 110) |
| 10 | 126, 104, 108 | Down 140 | Down 138 (138, 110) |

ST-3, a tighter line at a flip up, then an earlier flip down (period 2, factor 1_000): at 3 the latch's stop is the
fresh basic band 118, the two-band line is the ratcheted 124; at 6 the close 119 is below 124 but not below 118, so the
two-band form turns down one candle before the latch.

| i | H, L, C | latch | two-band |
|---|---|---|---|
| 0 | 143, 138, 142 | Up 135 | - |
| 1 | 142, 126, 126 | Down 144 | Down 144 (144, 124) first |
| 2 | 128, 119, 124 | Down 132 | Down 132 (132, 124) |
| 3 | 144, 123, 140 | Up 118 | Up 124 (132, 124) |
| 4 | 142, 119, 124 | Up 118 | Up 124 (149, 124) |
| 5 | 137, 123, 137 | Up 118 | Up 124 (146, 124) |
| 6 | 141, 119, 119 | Up 118 | Down 146 (146, 124) |
| 7 | 123, 114, 116 | Down 132 | Down 132 (132, 104) |

ST-4, a flat series (period 3, factor 3_000; five candles with H = L = C = 100): latch `Up 100` on every candle; two-band none,
none, then `Down 100 (100, 100)` first, then `Down 100 (100, 100)` twice. A flat stock reads up in the latch and down here.

ST-5, equal histories and a wide candle (period 2, factor 500): at 4 both are up on the same line 106; candle 5's basic
lower band (113) lies above its close (111), so the two-band form turns down at once while the latch, which compares the
close with its PREVIOUS stop (106), stays up with a stop above the close and turns at 6 (RA D2: "differs only on a bar
wider than 2 m ATR").

| i | H, L, C | latch | two-band |
|---|---|---|---|
| 0 | 121, 101, 118 | Up 101 | - |
| 1 | 118, 102, 105 | Up 101 | Down 119 (119, 101) first |
| 2 | 106, 101, 103 | Up 101 | Down 108 (108, 101) |
| 3 | 107, 101, 106 | Up 101 | Down 108 (108, 101) |
| 4 | 120, 104, 118 | Up 106 | Up 106 (108, 106) |
| 5 | 138, 109, 111 | Up 113 | Down 133 (133, 113) |
| 6 | 113, 104, 104 | Down 115 | Down 115 (115, 101) |

ST-6, period 1 (factor 1_000): candle 0 (110, 90, 100): latch `Up 80`; two-band `Down 0 (0, 80) first`. Candle 1
(106, 100, 105): latch `Up 97`; two-band `Down 109 (109, 97)`. Candle 2 (109, 104, 108): latch `Up 101`; two-band
`Down 109 (109, 101)`.

ST-7 and ST-8, gaps (period 2, factor 1_000; 101/99/100, 101/99/100, 102/100/101, then a gap candle 121/118/120 and
122/119/121, then for ST-8 a gap down 96/93/94): two-band none, `Down 102 (102, 98) first`, `Down 102 (102, 99)`, then
`Up 108 (102, 108)` on the gap candle (the close 120 crossed the upper band 102; the bands cross, which the rules allow),
`Up 113 (127, 113)`, and for ST-8 `Down 111 (111, 113)` on the gap down. The latch reads `Up 98, Up 98, Up 99, Up 108,
Up 113`, then `Down 111`.

ST-9, equal bands while up (period 2, factor 500): candles 119/108/108, 119/105/119, 129/116/128, 139/128/136,
156/130/151, 151/138/138, 142/137/141, 171/124/155. Two-band: none; `Down 118 (118, 106) first`; `Down 128 (128, 116)`;
`Up 128 (128, 128)`; `Up 134 (152, 134)`; `Up 137 (151, 137)`; `Up 137 (144, 137)`; `Up 137 (144, 137)`. At candle 3
the two bands are both 128; at candle 4 the close 151 is below the upper band 152, so TradingView's literal value
comparison would read `Down`; the published meaning (the previous trend was up) reads `Up` (D-8602).

### 5.5 Tests (`src/dual_band_supertrend_tests.rs`)
- `a_zero_period_and_a_factor_outside_its_range_are_refused` (period 0; factor 0, 100_001 and `u32::MAX` refused with
  `FactorOutOfRange { milli }` naming the value; 1 and 100_000 fold).
- `nothing_is_read_before_the_atr_has_its_period` (periods 1, 2, 10, `u16::MAX`).
- `the_first_atr_candle_is_down_on_the_upper_band_and_unsettled` and `period_one_reads_the_published_zero_band_first` (ST-6,
  both forms on every candle).
- `both_bands_ratchet_on_every_candle_and_reset_as_published` (random series: upper never rises while the previous
  close stayed at or below it, lower never falls while it stayed at or above it, and each reset follows a cross).
- `a_turn_needs_a_close_beyond_the_current_band` (up only when `close > upper`, down only when `close < lower`; a close
  equal to the band does not turn).
- Six fixture tests, each asserting both forms on every candle (the latch folded beside it in the test), named one by
  one (repair 2, check2 N4: START.md step 9.4 checks every named test by its name): `fixture_st_1_the_seed_differs`,
  `fixture_st_2_a_flip_down_keeps_the_ratcheted_upper_band`,
  `fixture_st_3_a_flip_up_keeps_the_ratcheted_lower_band_and_turns_down_first`,
  `fixture_st_4_a_flat_series_reads_up_in_the_latch_and_down_here`,
  `fixture_st_5_a_wide_candle_turns_the_two_band_form_first` and `fixture_st_9_equal_bands_do_not_turn_an_up_candle`.
- `a_gap_through_either_band_turns_on_the_gap_candle` (ST-7, ST-8, both forms on every candle).
- `an_all_up_and_an_all_down_series` (up after the first close above the upper band and up from then on, the line
  rising; down throughout for a falling series).
- `the_stream_equals_a_from_scratch_transcription_of_the_published_rules` (BRUTE FORCE: at every candle recompute
  from candle 0 with a separately written loop over the rules of 5.3, `Option` for TradingView's missing values; random
  series, periods 1..=20, factors 500..=5_000).
- `the_atr_is_the_engine_atr` (a `trend::Atr` folded alongside: at every candle `atr()` is `None` while that `Atr` is
  not warm and equals its `value()` once it is; repair 2, check2 N6: `atr()` reads only once warm, 5.2, while
  `Atr::value()` reads the running mean from the first candle, trend.rs:343-350, so the values are compared once warm).
- `no_reading_depends_on_a_later_candle` and `a_time_gap_reads_as_contiguous_candles`.
- `the_extremes_never_overflow` (prices near `i64::MAX` with the widest true ranges, factor 100_000, periods 1 and
  `u16::MAX`).
- `a_corrupt_candle_is_refused_and_changes_nothing`; `the_state_is_a_fixed_size` (`size_of::<DualBandSuperTrend>()`
  at most 256 bytes, ESTIMATE; repair 1, check1 N3: 160 could not hold, since the fields of 5.2 sum to 210 bytes,
  `trend::Atr` alone 80 (three `i128`, an `Option<i64>`, a `u64` and a `bool`, trend.rs:265-275), so about 224 with
  `i128`'s 16-byte alignment, UNVERIFIED until built; the report states the measured size); the two bench tests.

### 5.6 Benches, rows and ledger blocks
- **C-PSRB-STV-01**: one `fold` plus `reading()` after 2_000, 20_000 and 200_000 candles, period 10, factor 3_000; flat
  within 3.0x, inside 2 µs (PARTS target 400 ns, ESTIMATE).
- **C-PSRB-STV-02**: the same step at ATR periods 10, 100 and 1_000 after 20_000 candles.
- docs/04 `## The two-band Supertrend for the PSRB family (PSRB-STV; D-8600 to D-8605)`: PSRB-STV-I01 nothing before the
  ATR's period; the first ATR candle is down on the upper band from TradingView's zero prior bands, unsettled; I02 both
  bands ratchet every candle and reset as published, a turn needs a close beyond the current band; I03 the nine fixtures
  (seed, tighter line at a flip down, tighter line at a flip up and an earlier flip down, flat, wide candle, period 1,
  two gaps, equal bands); I04 the stream equals the from-scratch transcription, and the ATR is `trend::Atr`'s; I05 every
  refusal changes nothing, no overflow at the extremes; I06 no reading depends on a later candle or a timestamp;
  C-PSRB-STV-01, -02.
- docs/06 `## The two-band Supertrend for the PSRB family: limits (PSRB-STV; D-8600 to D-8605)`: L-PSRB-STV-01 the band
  arithmetic is the latch's (ATR floored to whole paisa, `mid` floored, `band` floored). With `f = factor_milli / 1000`
  and `n` the ATR period, each basic band is within `f + 1.5` paisa of real arithmetic on the exact value of
  `trend::Atr`'s scaled state, and within `f (1 + n / 10^6) + 1.5` paisa of real arithmetic on the exact real Wilder
  ATR, the one TradingView computes in floating point: the state's own floored steps (trend.rs:333) leave it below
  that ATR by less than `n` units of 10^-6 paisa (3.3's recurrence) before `value()` floors it to paisa (trend.rs:349).
  The extra
  `f n / 10^6` is 0.00003 paisa at 10/3 and 6.55 paisa at period 65,535 and factor 100 (repair 2, check2 N2;
  `repair2-work/atr-drift.out`). So a close within that of a band can compare differently from TradingView's floating
  point; L-PSRB-STV-02 the first ATR candle is TradingView's degenerate reading (ST-6), unsettled;
  L-PSRB-STV-03 TA-Lib's own seeding and first candles were not read in this step (UNVERIFIED; RA read TradingView's
  page, RB the ta-lib.org page); L-PSRB-STV-04 what was measured.
- docs/07 `## PSRB-STV: the two-band Supertrend — 2026-11-03 (D-8600 to D-8605)`, one row.
- docs/05: D-8600 the module and its name (two bands carried, beside the active-band latch), the latch and its
  vocabulary bits untouched; D-8601 the ATR reused and the latch's band arithmetic, so the readings differ in band logic
  only, bands held in `i128`; D-8602 the previous trend decides which band is judged (5.3, ST-9), TradingView's zero
  prior bands and its first-candle direction kept; D-8603 the domain (period `u16` 1.., factor 1..=100,000 thousandths)
  and refusals; D-8604 `settled`, and that PSRB-STRAT reads settled readings only; D-8605 the two benches.

### 5.7 NEEDS THE MAC
As 3.7, with the C-PSRB-STV rows, plus `git diff --exit-code <seam> <tip> -- crates/indicators/src/trend.rs
crates/indicators/src/evaluator.rs crates/vocab` (empty: the latch and the vocabulary untouched).

---

## 6. The later parts (outlined only; not built now; no session can start for them today)

Each is cut, when its time comes, from the passed tips of the parts whose code it calls (one merged `base/<part>`; the
1 Nov build rule the verifier cites, FP:1304), never from a family branch. The PSRB first run follows the fib first run
and the fib step-2 context filters, which are queued first (memory "Fib step 2: context filters"; FP:1294-1299), unless
the operator reorders.

### 6.1 PSRB-LAW (D-8570..D-8579; S, 1-2 h, ESTIMATE)
- **What:** CLAUDE.md and AGENTS.md: more than one strategy family runs on the F&O path; indicator readings enter it as
  run-local features with searched dials, never as vocabulary bits (D-8104's rule, FP:299); the survivorship label is
  unchanged. docs/00-charter.md: source rows for the TradingView help pages of RSI, Bollinger Bands, Supertrend and
  Pivot Points Standard (RA `sources`, read 3 Nov 2026), the built-in RSI script's zero-loss guard (a secondary source:
  the verifier), and ta-lib.org's Supertrend page (RB `evidence`), so the parts' "published definition" sentences cite a
  charter row (CLAUDE.md section 3 rule 1, :118-120).
- **Needs:** FNO-LAW's passed tip (it is the sole owner of CLAUDE.md, AGENTS.md and the charter, FP:220), so it is
  sequential after FNO-LAW and cut from that tip. Not before.

### 6.2 PSRB-STRAT (D-8610..D-8619; L, 6-9 h, ESTIMATE)
- **What (RB `new_parts` 6, as the verifier corrected it):** a `runner` module.
  - Features per candle: the exact R1 test `3 close_t > 2 (H + L + C)_(D-1) - 3 L_(D-1)` (which avoids the floored P of
    daily.rs:203-206) and the floored-P reading as a labelled twin, S1 mirrored, the distance beyond the level; weekly
    pivots (new code), the TradingView-Auto mapping (daily up to 15 min, weekly above, monthly on the 1-day timeframe) and
    the other pivot types as labelled variants; both Supertrend readings (settled only) and the stop distance in ATR
    units; RSI `ppm`, `flat()` and `faithful()` (decisions only on faithful readings, 3.3; repair 1, check1 B1);
    Bollinger: a decision reads `above_upper` / `below_lower` at the searched width (exact, no square root, timed by
    C-PSRB-BOLL-01), and `z_milli` is recorded per candle for records and seeds only, never decided on (D-8593,
    L-PSRB-BOLL-02; repair 1, check1 N4): a z threshold `w` is the width `w`, so nothing is lost.
  - Every value of a state-changing dial (RSI, Bollinger and ATR periods, the Supertrend factor) is its own reading,
    folded from the Jan-Aug 2018 warm-up (WU section 3), so refining one re-runs the feature pass for that value: sized
    by the sample run.
  - A hole: a carried reading is tainted until it has folded its own period of clean candles after the hole (RSI,
    Bollinger: `period`; two-band: the ATR period), counted by this part, or it is refolded from a fresh state; this part
    decides which, by a test. No indicator API is needed for either.
  - Variants: VIDEO-ORIGINAL + house stop (the video names no resting stop; the stop is the house rule, SPEC.md:554),
    the stop-less literal reading as a labelled comparison, VIDEO-ORIGINAL-DAILY (WU section 8); seeds and coarse-to-fine
    dials (RA G1-G3); long and short separate searches; the pair-aware extinction floor (FP:1354).
  - Entries: market at the next print, a stop-entry, and retest limits at R1 or the band (the precise-entry family).
    Exits: the flip on a CLOSE with a protective stop held at the ENTRY line (the original); the line trailed and touched
    intrabar is a labelled variant (it would pre-empt almost every close-based flip, the verifier); smaller generated
    stops (the signal candle's low, R1 minus a buffer, the band's basis); the FNO-EXITS families. `ConditionExit` and
    `MoveStop` already exist on `fix/fib-fno-sec-exec` (order.rs:37, walk.rs:80-98); the flip trigger is PSRB's own.
  - Refusals: no signal before every reading is settled and warm, and none on an RSI reading that is not faithful; no
    decision on a hole candle; the day mask M1-M11.
  - The operator's inputs of 3 Nov ~16:5x-17:0x IST, as the main session wrote them into this folder
    (`operator-inputs-20261103.md`, read here as input for this later part): sizing and admission as named variants
    (SIZE-FIXED-20-SEEDED, SIZE-FIXED-20-RANKED, SIZE-SPLIT-ALL, SIZE-RISK); VWAP side and distance (session VWAP exists,
    vwap.rs) and the other precision criteria as searched, ablated features; and the operator's hard account limits at
    every instant (the margin and exposure caps of that file's item 5, kept on the Mac and not repeated in this public
    kit: repair 1, check1 N9; each stock's leverage from FNO-LEVERAGE), every result net of every charge. None of it
    changes the three indicator parts built now.
- **Needs:** the three parts of this file passed; FNO-WFT, FNO-SEC-EXEC, FNO-EXITS passed (FNO-DAILY for the 1-day
  variant). Its runner anchor is decided then (the runner's FNO slots belong to the FNO parts).

### 6.3 PSRB-RUN (D-8620..D-8629; M, 4-6 h, ESTIMATE)
- **What (RB `new_parts` 7):** a family switch in `cli fno run` (P3 skipped, P4 = PSRB features), a family and version
  term in the run identity (FnoRunV1 has none, FP:1555-1560), family-tagged records, resume refused across families, and
  a cap tie-break set that works without the rank (SPEC 11.5's rank terms need FNO-RANK, which this run skips).
- **Needs:** FNO-RUN, FNO-RECORDS, FNO-SEC-WIRE passed, then PSRB-STRAT. Sequential after FNO-RUN.

---

## 7. Integration, check and landing (START.md has the commands)

1. **Kit commit:** `kit-commit.sh <P>` writes `refs/heads/fix/psrb-seam` (the seam commit on `1ad6e24ef`) and
   `refs/heads/cloud/psrb-kit-20261103` (the kit folder on the seam), refusing any existing ref and proving each change.
2. **Push** the kit branch only, after the private scan (CW section 9's pattern) over the kit folder and the seam diff.
3. **Start** the three sessions in one operator-typed turn (CW section 11 item 5).
4. **Intake** of a DONE branch, after its tip is the same at two ticks 30 or more minutes apart (CW 3.1):
   `replay-minus-reports.sh <P> <seam sha> <kit sha> <tip> refs/heads/fix/psrb-<part>` (START = the seam commit, FROM =
   the kit commit; P0-P4 prove the code tree equals the cloud's outside `reports/`, `reports/` equals the seam's, no merge,
   the kit commit absent from the new history).
5. **Check 1 on the Mac**, a fresh reader: the part's section here, the cloud report, Route B on the Mac's pin, gates,
   the bench run, the anchors (each insertion at its named place and nothing else in a shared file), and 5.7's diff.
   A failed check is repaired by a continuation session on the same branch (`W/state/cloud-wave-20261103/briefs/TEMPLATE-continue.md`) or, for a
   macOS-only item, on the Mac; never two owners.
6. **Landing (D1, base green first):** nothing merges before BASE-REDS-1023; the order is the base parts, the data
   family, the audit, FAM-FIB-1, FAM-FIB-2, then the PSRB family. `fam/psrb-1` is cut from feat/db-fix after FAM-FIB-2
   lands and only once `git merge-base --is-ancestor 1ad6e24ef8e0 feat/db-fix` succeeds (START.md 11.2; repair 2,
   check2 N3: every PSRB branch holds the base, so the base, with FNO-SEAM and the BASE-STACK-1028 it merged, must be in
   feat/db-fix first, or a PSRB merge would carry them in; a failure stops the landing); the main session merges
   `fix/psrb-rsi`, `fix/psrb-boll` and `fix/psrb-stv` into it (each holds `fix/psrb-seam`; their files and anchors are
   disjoint, so the merges are clean by construction), adds the six
   C-PSRB ids to gate 14's cover table (the main session's edit at assembly, as for F1), runs the family check
   (fmt, clippy `-D warnings` workspace, `cargo test --workspace --locked`, gates, `cargo bench -p indicators --bench
   ratio`), and lands it with one merge. If BASE-COV-LIBS has landed by then, the coverage reading for `indicators` is
   whole-crate (`d3-recipe.txt` step 2, "THE END OF THE NARROW READING"), and the family check measures it whole.

---

## 8. UNVERIFIED, open, not proven here

- The verifier's result arrived during this step and is applied (section 1); RSI's flat-window 100 rests on secondary
  sources for the built-in script (UNVERIFIED on TradingView's help page).
- TradingView's defaults and smoothing are from RA, which marks several as secondary sources (RA `unverified`: ATR 10/3,
  RMA seeding, the population divisor). No TradingView output was compared with these definitions; parity here means
  parity with the published formulas.
- TA-Lib's Supertrend was not read in this step; RB cites its page (RB `evidence`). L-PSRB-STV-03.
- `i128::midpoint` rounding toward zero for signed integers is the Rust documentation as I recall it (UNVERIFIED here);
  for positive prices it equals the floor, and STV inherits the latch's use of it (trend.rs:422) either way.
- The anchors were checked against 66 refs' current diffs (`work/hunks-20261103.txt`); refs created later, BASE-COV-LIBS
  LSA (which may edit `crates/*/src/lib.rs`, kit-base CLOUD.md item 6) and the data parts' future edits were not seen.
  The landing's anchor check (7.5, 7.6) covers them. Repair 1 checked the moved docs/05 anchor against all 775 current
  refs (2.2).
- Every size, hour, budget and p99 target is an ESTIMATE; no bench ran.
- The fixtures were computed by node BigInt models written in this step (`work/sim.mjs`, `fixtures.mjs`, `fix2.mjs`),
  not by the Rust code; the sessions' brute-force tests are what prove the Rust.
- Repair 1's figures (the RSI departure points and guard counts of 3.3 and 3.5, ST-9's LITERAL-PINE reading, the seam
  re-print, the docs/05 anchor checks) come from node BigInt models and shell checks in `repair1-work/`, read with
  `git show`/`git grep` only; the Rust is proved by the sessions' tests. LITERAL-PINE is named, not built. The STV state
  size (about 224 bytes) is from Rust's layout rules, UNVERIFIED until built.
- Repair 2's figures (the `u16` counter's wrong readings at period `u16::MAX`, the faithful 0/100 departures near 0 or
  100, the ATR state's drift and the 1.0000245-paisa gap between the exact Wilder ATR and `value()`) come from node
  BigInt models in `repair2-work/` (`counter-r2`, `departure-r2`, `atr-drift`), not from Rust; the new and amended
  tests are what prove the Rust.

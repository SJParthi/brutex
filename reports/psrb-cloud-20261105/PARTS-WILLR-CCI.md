# PSRB PARTS, second wave: PSRB-WILLR and PSRB-CCI, their seam, and the narrow CPR

Written Thu 5 Nov 2026, ~09:0x-10:xx IST, by a workflow step of the main session, after the operator confirmed the
PSRB rules and said "Yes, use defaults" (`operator-inputs-20261103.md` item 7; `psrb-rules-check-20261105.html`, the
page "PSRB Rules Check"). The confirmed rule set (AUTHOR-V2) adds two indicators the engine does not have: Williams %R
and CCI. This file designs them as two indicator parts in the exact shape of `PARTS.md` section 3 (PSRB-RSI), decides
the base and the anchors that let both land beside PSRB-RSI, PSRB-BOLL and PSRB-STV with no code conflict, and says
where the narrow-CPR check goes. The strategy rules themselves are in `PARTS-STRAT-ADDENDUM-20261105.md`.

Repaired Thu 5 Nov 2026 ~10:3x IST by repair 1, from the independent check `specs-1105-check1.md` (3 blocking, 14 not
blocking; the table of what changed is `specs-1105-repair1.md`; the pre-repair file is
`PARTS-WILLR-CCI.md.bak-20261105-repair1`). In this file: the Williams %R pop test counts back-pops by a stated
white-box method that matches its pinned figures (B1, 5.3, 5.5, 5.6); the floored-level gaps (N1, 4.1, 4.3), the `u128`
conversions (N2), the CCI ceiling's equality clause (N3), the CCI cost, now decided (N4), the crossing's period check
(N9), the CCI level domain (N10), the memory ESTIMATEs (N11) and two refs' ancestry (N13, 3.5). Repair 1's figures are
`specs-1105-repair1-work/repair1.mjs` and `repair1.out` (node BigInt; the model goes in the kit as `repair1.mjs.md`,
3.6).

Repair 2, from the independent check `specs-1105-check2.md`, stopped mid-run (11:35 IST, ENOTFOUND) and changed
nothing in this file. Repaired Thu 5 Nov 2026 ~12:2x IST by repair 3, from the independent checks
`specs-1105-check2.md` and `specs-1105-check3.md` (the table of what changed is `specs-1105-repair3.md`; the pre-repair
file is `PARTS-WILLR-CCI.md.bak-20261105-repair3`). In this file: the pop staircase's exact generator (check 3 N1a,
check 2 N1: 5.3, 5.5); the kit names of the models (N1b: 3.6, 4.3, 5.3, 5.5, 6.5, 8); the CCI ceiling at `u16::MAX`
pinned (N1c, check 2 N5: 6.5); gate 12 and the word "flat" (N1d, check 2 N7: 2); the source bar of N5-1 and N5-2
(N1e, check 2 N12: 4.3); a clone's capacity (N3: 2, 5.3, 6.3); C2's type and bounds and `k`'s domain (N4, check 2
N4: 4.3); `free_from` and the ref count of the moment (N9: 0.4, 3.5, 8). Repair 3's figures are
`specs-1105-repair3-work/repair3.mjs` and `repair3.out` (node BigInt; in the kit as `repair3.mjs.md`, 3.6).

Repaired Thu 5 Nov 2026 ~16:2x IST by repair 4, from the independent check `specs-1105-check4.md` (3 blocking, 10 not
blocking; the table of what changed is `specs-1105-repair4.md`; the pre-repair file is
`PARTS-WILLR-CCI.md.bak-repair4`). In this file: the gate 12 rule names only the proofs gate 12 reads (a `::` path to
a test or bench, a tracked test or bench file, a docs/04 row id whose row proves, or UNVERIFIED), and every example
in it was run through gate 12's own readers (B1: 2, with `PARTS.md` 2.4 and both briefs); each part gains a test that
tells the exact `above` from one decided on the floored `ppm` or `milli`, named in its I02 row (B3: 5.5, 5.6, 6.5,
6.6); repair 4's evidence (8).
Repair 4 ran `rustc` 1.97.1 in a scratch directory only, on gate 12's own source at the base with a test appended and
on the engine's ATR and PSRB-STV code read by `git show` (`specs-1105-repair4-work/run.sh`); nothing in P was built
or changed.

Nothing was built, run, committed, pushed or started in this step. No cargo, rustc or clippy ran. Every size, hour and
p99 target is an ESTIMATE until measured. Short names: P = the repository `/Volumes/WD_BLACK/brutex/fresh-20260919/project`
(read with `git show`, `git grep`, `git log`, `git merge-base` and `git merge-file -p` on plain files only);
W = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`; PARTS = `PARTS.md` in this folder (the first wave's design);
FP = `W/state/fib-strategy-20261030/PARTS.md`; WU = `psrb-strategy-v1.html`; RC = `psrb-rules-check-20261105.html`;
OI = `operator-inputs-20261103.md`; AD = `PARTS-STRAT-ADDENDUM-20261105.md`; XW = `work-willr-cci/` in this folder
(this step's scripts and their outputs). Repository line numbers are at PSRB-SEAM `35234cead400` for
`crates/indicators/benches/ratio.rs` and at the base `1ad6e24ef8e0` for every other file (the seam changed only
`ratio.rs`; `git show --stat 35234cead`).

---

## 0. The answer in brief

1. **Two parts, two cloud sessions, started in one operator-typed turn, neither waiting on the other or on the first
   wave:** PSRB-WILLR (`indicators::williams_r`, Williams %R over the last n highs and lows) and PSRB-CCI
   (`indicators::cci`, the Commodity Channel Index on the typical price). Both live in `crates/indicators` and touch no
   other crate. Both are exact rational functions of integer prices, so neither needs a fixed-point scale or a
   truncation bound: every comparison is exact on the published definition.
2. **Base and seam (section 3).** One more comments-only commit is needed: **PSRB-SEAM-2**, on PSRB-SEAM `35234cead400`
   (never on the kit commit), 18 added lines in two files (`benches/ratio.rs` +12, `src/lib.rs` +6), no deleted line.
   The proof: at PSRB-SEAM, `fn main` of `ratio.rs` has exactly one clean statement position left for a bench row
   (after `];`, line 776), and two parts need two. Since the seam is needed for that, it also gives each part a
   bench-module slot and a `lib.rs` slot, in the first seam's form; `cci` could not sort into the module list anyway,
   because it falls at the very point where PSRB-BOLL inserts `pub mod bollinger;` (after `lib.rs:63`). The new kit
   branch is `cloud/psrb-kit-20261105` (the kit folder `reports/psrb-cloud-20261105/` on PSRB-SEAM-2). A textual
   3-way merge of all five parts (the first three from their real cloud tips) is clean in both orders, and no ref of
   the 95 that hold `f01d7a0e4` gains a conflict from the new lines (`XW/anchors2-check.out`).
3. **EMA and CPR exist (section 4).** `trend::Ema` (trend.rs:167-261) is the EMA 200; `daily::DailyLevels`
   (daily.rs:152-382) holds the pivot, BC, TC, R1 and S1, fed by `anchored::DailyReference::new` (anchored.rs:111-126).
   The existing narrow-CPR test (`cpr_class`, daily.rs:343-365; vocabulary position 63, vocab table.rs:354) measures the
   width against the previous session's RANGE, not against price, so it is not the operator's rule. The width check
   belongs in PSRB-STRAT, in exact integers: `10_000 x |2C - H - L| <= k x (H + L + C)` with `k` in basis points.
4. **Decision ids are registered** (`W/state/decision-blocks.json`, written in this step by `jq` and `mv` after a check;
   backup `decision-blocks.json.bak-20261105-psrb`): PSRB-WILLR D-8640..D-8649, PSRB-CCI D-8650..D-8659; `free_from`
   was D-8640 and became D-8660 in this step. Since 11:48 IST BASE-COV-API holds D-8660..D-8669 and `free_from` is
   D-8670 (no overlap; check 3 N9). PSRB-SEAM-2 takes none, as PSRB-SEAM took none.
5. **One house bar is not met, and it is named.** CCI's mean absolute deviation is taken around the CURRENT mean, so
   every reading needs one pass over its window: `fold` is O(1), but `reading()` is O(period) (L-PSRB-CCI-02). At the
   confirmed period 20 that is 20 integer steps. An order-statistic tree would make it O(log period). **Decided
   (repair 1, check N4; D-8653): O(period) is accepted and no tree is built**, the default the CCI brief already
   carries. The decision goes to the operator in the evening report, and the operator may reverse it (section 8).
6. **Sizes:** PSRB-WILLR S, 2-3 h; PSRB-CCI S, 2-3 h (ESTIMATE, as PSRB-RSI's).

---

## 1. What I read (each claim below cites file:line)

- OI item 7 (the confirmed AUTHOR-V2 rules and defaults, 5 Nov 2026 ~08:0x-09:3x IST) and RC whole (its rule sheets,
  "What changed from the first version", "What I noticed", the eight default questions).
- PARTS sections 0, 1, 2 (2.1-2.5), 3 (PSRB-RSI, the model for this file), 4.2-4.5 (the ring), 6.2 and 7, whole.
- The three first-wave branches as they are now (read, not fetched; `git log`/`git show` only): `origin/cloud/psrb-rsi-20261103`
  at `bd7560c699fa` (STATUS: DONE), `origin/cloud/psrb-boll-20261103` at `9b60567e163a` and
  `origin/cloud/psrb-stv-20261103` at `3d1cb6aa72ee` (both report line 1 `STATUS: DONE` at those tips). Their insertions in the shared files:
  `ratio.rs` after seam lines 68, 70, 72 (bench-mod) and 781, 783, 785 (bench-row); `lib.rs` after 70 (`rsi`), 63
  (`bollinger`), 65 (`dual_band_supertrend`); docs/04 after 8950, 9015, 9061; docs/05 after 84675, 86591, 86992;
  docs/06 after 21954, 22023, 22066; docs/07 after 982, 1057, 1109 (`git diff -U0 56f3043dd <tip>`). The built RSI's
  public items match PARTS 3.2 (`origin/cloud/psrb-rsi-20261103:crates/indicators/src/rsi.rs:77-446`).
- PSRB-SEAM `35234cead400`: `crates/indicators/benches/ratio.rs` (814 lines; 63-80 and 755-814 read whole) and the kit
  commit `56f3043dd4ca` (13 Markdown files under `reports/psrb-cloud-20261103/`).
- `crates/indicators` at `1ad6e24ef8e0`: lib.rs:60-81 (module list and FNO-SEAM's slots), :1643 (`Candle`), :1710
  (`check`), :1778 (`check_evaluable`); trend.rs:23-37 (the EMA's integer form), :90 (`SCALE`), :167-261 (`Ema`),
  :949-958 (the EMA 200 emission), :1041-1050 (`side`, a strict comparison); daily.rs:9-40 (the ladder and its
  rounding), :88-134 (`CprWidth`), :152-253 (`DailyLevels::from_previous_session`), :255-262 (`from_daily_bar`),
  :307-341 (`cpr_width`), :343-365 (`cpr_class`), :367-381 (`cpr_span`), :596-607 (bits 63, 274, 275);
  anchored.rs:30-36 and :101-135 (`DailyReference::new`); vocab table.rs:261-264 and :354; runner resample.rs:1-40 and
  :276 (`resample`).
- FP:381 and :395 (`SessionClock`: the 15:08:00 self-exit cut-off and the 15:10:00 square-off), FP:770 (the equity
  path and its max drawdown), FP:1355 (the Winner Gate's 38 checks); WU lines on VIDEO-ORIGINAL's entry, stop and exit
  (its "Entry", "Stop", "Exit" and "Day end" rows, and the VIDEO-ORIGINAL variant row).
- The rustfmt evidence for a module line in its own slot group: `fix/fib-fno-ladder:crates/indicators/src/lib.rs:77-78`
  puts `pub mod fno_ladder;` after `pub mod vwap;` in a group of its own, and that tip's gates passed with "fmt 0"
  (`W/state/fib-strategy-20261030/progress-check-FNO-LADDER-r3.md:12`).
- `W/state/decision-blocks.json` (`free_from` D-8640 before this step); every docs/05 at the base and on the three
  first-wave tips holds no heading `^#{2,4} +D-86[1-5][0-9]` (`git grep -c`, 0 each); the only other mention of
  D-864x in `W/state` is FNO-SIZE's "free_from D-8640" (`fno-size-20261103/PARTS-FNO-SIZE.md:64`, `START.md:144`).

---

## 2. Rules shared by the two parts

PARTS 2.4 (the bar, the gates, the style) binds both parts word for word, with `<CODE>` = `WILLR` or `CCI`, the module
names `williams_r` and `cci`, the bench wrappers `benches/ratio/psrb_willr.rs` and `benches/ratio/psrb_cci.rs`, and the
anchors of section 3.4. In short: 100% line and branch coverage of the D2 set by Route B on the pinned nightly; exact
integers, no `as`, no float, no indexing, every `i128` bound a `const _: () = assert!(...)` with `checked_mul` chains;
a refusal changes nothing (validate first, then mutate, and a test compares the whole state); streaming, no
look-ahead, the current candle in its own reading; timestamps not read; no vocabulary position; gates 10, 11, 12, 14,
17; tests in `src/<module>_tests.rs` declared `#[cfg(test)] #[path = "<module>_tests.rs"] mod tests;`; every new test
red under a hand change; facts only. Three points differ from PSRB-RSI and are stated here once:

- **No fixed-point scale.** Williams %R and CCI hold no average that a recursion smooths. Every quantity is an exact
  integer (a price, a difference, a sum), and every comparison cross-multiplies integers. So neither part has a
  truncation bound, a `faithful()` guard or a `SCALE`.
- **A window, held in a `VecDeque` allocated once.** Both parts keep `VecDeque`s built by `VecDeque::with_capacity(period)`
  in `new`. A deque never holds more than `period` entries (section 5.3 and 6.3 prove it), so a deque built by `new`
  never reallocates; a test pins the capacity after 100,000 folds. `VecDeque` is `std`; no new dependency. **A clone
  reserves only its length** (repair 3, check 3 N3): the pinned 1.97.1 std builds a cloned `VecDeque` with
  `with_capacity_in(self.len(), ...)` (alloc/src/collections/vec_deque/mod.rs:119-124), so a cloned `WilliamsR`, or a
  cloned `Cci` not yet warm, reallocates as it fills. The measured path never clones, and both module docs say so:
  "a clone reserves only its length; the measured path never clones". The tests that compare a state with a copy
  are unaffected.
- **A cost claim in a doc block names a proof gate 12 reads** (repair 4, check 4 B1; repair 3, check 3 N1d; check 2
  N7). Gate 12 reads every `///` and `//!` block under `crates/`, the bench wrapper's included. A block is a cost
  claim when, after gate 12's scrub, it holds `O(1)`, `constant time`, `never scan` or the whole word `flat`
  (gates_bounds.rs:400-417 at the base). The scrub (:299-398) removes `::flat` after a name (so [`Reading::flat`]
  claims nothing), `flat(` and `flat (` (so `flat()` claims nothing), "flat" before one of 17 nouns (bar, price,
  close, session, day and the rest), "flat JSON object", and "flat" after a few lead words such as "A" and "exactly".
  Every other "flat" is a claim: "flat window", "flat state", "false when flat", "p99 flat within 3.0x". A claim
  passes only if the same block holds one of four proofs (`prove`, :670-698, and the UNVERIFIED pass, :756):
  1. a `::` path that resolves to a test or bench fn. Name a test crate first, by its file, in backticks:
     `indicators::williams_r_tests::a_flat_window_has_no_value_and_compares_false` (for CCI,
     `indicators::cci_tests::a_flat_window_has_no_value_and_compares_false`); `--resolve` maps it to the fn of that
     name in `crates/indicators/src/williams_r_tests.rs`, which answers to `williams_r_tests` by its own path
     (invariant_paths.rs:225-248, :298-391). Never as a link: the test module exists only under `cfg(test)`.
  2. a tracked `crates/*/tests/*.rs` or `crates/*/benches/*.rs` path (`harness_paths`, :544-569):
     `crates/indicators/benches/ratio.rs`. The wrapper `crates/indicators/benches/ratio/psrb_willr.rs` is one
     directory too deep and `crates/indicators/src/williams_r_tests.rs` is not under `tests/`, so neither is read.
  3. an id of the form `[A-Z]+(-[A-Z]+)?-nn` whose docs/04 row holds a ticked `::` path that proves (`row_ids`,
     :571-595), as `C-I-01` does (docs/04:1972). No id of this wave is one: gate 12 reads `C-PSRB-WILLR-01` and
     `L-PSRB-WILLR-01` as `PSRB-WILLR-01`, which heads no docs/04 row, and `PSRB-WILLR-I02` not at all.
  4. the word UNVERIFIED anywhere in the block.

  A bare test name, a bench id alone and a link set beside the words prove nothing. So a block that would say "flat
  window" or "flat state" says "a window of one price" (for CCI, "a window of one typical price") or links
  [`Reading::flat`] in place of those words, or keeps them and names its test as in 1. A block that claims `O(1)`,
  "constant time" or "flat within" names `crates/indicators/benches/ratio.rs` beside its `C-PSRB-<CODE>-0n` id, as
  the first wave's module docs do (BOLL tip bollinger.rs:41-45). The API comments of 5.2 and 6.2 ("None when flat",
  "false when flat", "O(1)") are notes for the builder; a doc comment made from one follows the same rule, e.g.
  "false on a window of one price ([`Reading::flat`])" (for CCI, "of one typical price"). Every form in this
  bullet, and every refused one, was run through gate 12's own readers at the base
  (`specs-1105-repair4-work/run.sh`, output `gate12.out`).
- **An `Option` the code cannot meet is read with `map_or` or `unwrap_or`, never matched.** A deque's `front()` or
  `pop_front()` returns an `Option` whose `None` cannot occur where the part calls it (proved beside each call). A
  `match` or `if let` there would be a branch no test can take, and the 100% bar would fail. `map_or(d, f)` and
  `unwrap_or(d)` are calls, not branches in the part's own code; a doc comment says why the default never applies
  (the form daily.rs:608-613 explains for `unwrap_or(mask)`). `.unwrap()` and `.expect(` stay forbidden (gate 11).

### 2.1 Decision ids and row ids

| part | decisions (docs/05) | invariants (docs/04) | benches (docs/04) | limits (docs/06) | plan (docs/07) |
|---|---|---|---|---|---|
| PSRB-WILLR | D-8640..D-8644 (D-8645..D-8649 spare, for repairs) | PSRB-WILLR-I01..I06 | C-PSRB-WILLR-01, -02 | L-PSRB-WILLR-01..04 | one row |
| PSRB-CCI | D-8650..D-8655 (D-8656..D-8659 spare) | PSRB-CCI-I01..I06 | C-PSRB-CCI-01, -02 | L-PSRB-CCI-01..04 | one row |

A part takes ids only from its own block, in order. Before writing, a session greps docs/05 for `^#{2,4} +D-864[0-9]`
or `^#{2,4} +D-865[0-9]`: only an entry heading counts as a clash; a hit is `STATUS: BLOCKED QUESTION`. Every `D-nnnn`
a part cites under `crates/` must head a docs/05 entry at START or in its own block (PARTS 2.5, `decision_citations`).
`WILLR` and `CCI` are upper-case letters only, so `cost_invariants.rs` reads `C-PSRB-WILLR-01` and `C-PSRB-CCI-01`.

---

## 3. Base, anchors and seam

### 3.1 The decision

**Every second-wave branch descends from PSRB-SEAM `35234cead4009299a93fcd79a3d1fffd773dcc16` through one more
comments-only commit, PSRB-SEAM-2, and then the second kit commit.** PSRB-SEAM-2's parent is PSRB-SEAM itself, not the
first kit commit `56f3043dd4ca`, so no kit folder ever enters the code history. Nothing waits on the first wave: the
new parts never need RSI's, BOLL's or STV's code, and their anchors are disjoint from those parts' insertions by
construction (3.2, 3.5). Waiting for the first wave to land and cutting from its tip was the alternative; it would
make the second wave wait on three intakes and a family check, for no gain.

### 3.2 Why per-part anchors cannot do it (proved from the files at PSRB-SEAM)

Rule A.6 of the ONE ANCHOR decision (`W/state/data-build-20261031/PARTS.md:630-700`) asks every insertion to sit at
least 2 unchanged lines from every other insertion, and D1 aborts a landing on a code conflict (`:634`). The first
wave's real insertions (section 1) and FNO-LADDER's and FNO-PAUSE's fix the occupied points.

- **`benches/ratio.rs`, the bench rows (seam lines 776-809): this is what decides it.** A row is a statement after the
  verdicts array's `];` (776) and before `if verdicts.contains(&false)` (809), for the reason PARTS 2.1 gives (before
  the array a row cannot fold into the check without editing `ROWS` or the check line; after the check it would run
  only when every earlier row passed). 777-780 are PSRB-SEAM's row comment; 781, 783, 785 the RSI, BOLL, STV slots
  with blanks between; 786 blank; 787-806 FNO-SEAM's comment; 807 and 808 the FNO slots. Parts insert after 781, 783,
  785, 807 and 808. After 777-779 splits PSRB-SEAM's sentences; after 780 is 1 line from RSI's point; after 782, 784,
  786 is 1 line from a neighbour; after 787-805 splits FNO-SEAM's comment; after 806 is 1 line from FNO-LADDER's.
  Only after 776 is clean (5 lines from RSI's point). **One position, two parts.**
- **`benches/ratio.rs`, the bench modules (seam lines 56-80), not decisive alone.** 56-63 are the `use` block; 64
  blank; 65-67 PSRB-SEAM's comment; 68 the RSI slot; 69 blank; 70 the BOLL slot; 71 blank; 72 the STV slot; 73 blank;
  74-76 FNO-SEAM's comment; 77 the FNO-LADDER slot; 78 blank; 79 the FNO-PAUSE slot; 80 blank. In that region only
  63-64 is clean (one position). A `mod` item may also stand between two later top-level items (after
  `const BASE` at :93, say), so the modules alone could find two anchors without a seam. Since the rows need the seam
  anyway, the seam carries the module slots too, beside the PSRB block, in the first seam's form: one form for every
  PSRB part, and no part's lines among the file's helper functions.
- **`src/lib.rs` (base lines 63-79).** The module list is sorted (rustfmt's module ordering). `cci` sorts between
  `bollinger` and `column`, and PSRB-BOLL inserts `pub mod bollinger;` directly after `pub mod anchored;` (:63), so a
  sorted `pub mod cci;` must go at the same point: a conflict with BOLL by construction. An unsorted line in the main
  group fails `cargo fmt --all --check`. `pub mod cci;` could stand in a group of its own somewhere among the file's
  later items, but the file's form, and FNO-SEAM's, is a slot group under the module list. `williams_r` alone could go
  directly after `pub mod vwap;` (:73), 3 lines from RSI's point and 4 from FNO-LADDER's. The seam gives both new lines
  a slot group under the module list instead, in FNO-SEAM's form (lib.rs:75-79). A module line alone in its own group
  after `vwap` is the form `fix/fib-fno-ladder` uses, and its gates passed `fmt` (section 1).

So a seam is needed (the rows decide it), and since no session may make the others wait, the main session writes it
in the kit step, as it wrote PSRB-SEAM.

### 3.3 PSRB-SEAM-2: its exact content (comments and blank lines only; made by the main session's kit step)

Three insertions in two files; 18 added lines; no deleted line; no quote mark, no `ratio(`, no cost id, no `///` or
`//!` (so gates 12 and 14 and `cost_invariants.rs` read nothing new); longest added line 86 characters. The bytes are
`XW/seam2/A2.txt`, `B2.txt` and `C2.txt`; the resulting files are `XW/seam2/ratio.rs.seam2` and `XW/seam2/lib.rs.seam2`,
and `XW/anchors2-check.sh` rebuilds and checks both from PSRB-SEAM's blobs.

Insertion A2, `crates/indicators/benches/ratio.rs`, directly after seam line 64 (the blank line under
`use indicators::{Candle, OI_NULL};`), before line 65 (`// ---- PSRB indicator parts (state/strategy-psrb-20261103/PARTS.md 2.3). One slot per`):

```
// ---- PSRB second wave (state/strategy-psrb-20261103/PARTS-WILLR-CCI.md 3). One slot
// ---- per part, in the form of the PSRB slots below.
// slot bench-mod PSRB-WILLR
                                                                    <- one blank line
// slot bench-mod PSRB-CCI
                                                                    <- one blank line
```

Insertion B2, the same file, directly after seam line 776 (`    ];`), before line 777
(`    // PSRB rows (state/strategy-psrb-20261103/PARTS.md 2.3): each part puts one statement`):

```
    // PSRB second-wave rows (state/strategy-psrb-20261103/PARTS-WILLR-CCI.md 3): one
    // statement per part directly under its slot, in the form of the PSRB rows below.
    // slot bench-row PSRB-WILLR
                                                                    <- one blank line
    // slot bench-row PSRB-CCI
                                                                    <- one blank line
```

Insertion C2, `crates/indicators/src/lib.rs`, directly after base line 74 (the blank line under `pub mod vwap;`), before
line 75 (`// ---- F&O strategy path (state/fib-strategy-20261030/PARTS.md 4.2). One slot per`):

```
// ---- PSRB second wave (state/strategy-psrb-20261103/PARTS-WILLR-CCI.md 3). One slot
// ---- per part; each part inserts its single `pub mod` line directly under its slot.
// slot PSRB-WILLR (williams_r)
                                                                    <- one blank line
// slot PSRB-CCI (cci)
                                                                    <- one blank line
```

(The arrows are not part of the text.) After PSRB-SEAM-2, the slots sit at `ratio.rs` lines 67 and 69 (bench-mod) and
785 and 787 (bench-row), and at `lib.rs` lines 77 and 79. Distances, in unchanged lines:

| insertion | its neighbours | unchanged lines between |
|---|---|---|
| A2 (after seam 64) | RSI's bench module (after seam 68) | 4 (65-68) |
| B2 (after seam 776) | RSI's bench row (after seam 781) | 5 (777-781) |
| C2 (after base 74) | RSI's `pub mod` (after 70); FNO-LADDER's (after 77) | 4 (71-74); 3 (75-77) |
| WILLR's and CCI's own lines, in seam-2 lines | each other, in all three places | 2 (a blank and the next slot) |
| CCI's bench module (after seam-2 69) | RSI's (after seam-2 74) | 5 |
| CCI's bench row (after seam-2 787) | RSI's (after seam-2 793) | 6 |
| CCI's `pub mod` (after seam-2 79) | FNO-LADDER's (after seam-2 83) | 4 |

**Checking it.** Comments only: `cargo fmt --all --check`, clippy `-D warnings` and the tests are unchanged by it, and
its coverage set holds no instrumented region (FP:271-275 for FNO-SEAM). It is checked as part of each second-wave
part's check (each part's tree contains it) and by the family check; it needs no check round of its own. It is
integrated as `refs/heads/fix/psrb-seam-2` = the seam-2 commit, written by the kit step, never pushed on its own. It
lands with the first second-wave part merged into the PSRB family (3.6).

### 3.4 Files, ownership and anchors of the two parts

Every file below has ONE owner. New files are the owning part's alone. A shared file is edited only by inserting at
the owner's own anchor, never by changing an existing line. No part edits `crates/indicators/src/trend.rs`, `vwap.rs`,
`daily.rs`, `evaluator.rs`, any first-wave file (`rsi.rs`, `bollinger.rs`, `dual_band_supertrend.rs`, their tests and
wrappers), any `tests/*.rs`, `Cargo.toml`, `Cargo.lock`, `CLAUDE.md`, `AGENTS.md`, `docs/00-charter.md` or `.github/`.
A part that finds it must edit any other file stops with `STATUS: BLOCKED OUTSIDE` and the exact change.

| file | PSRB-WILLR | PSRB-CCI | others' edits nearby |
|---|---|---|---|
| `crates/indicators/src/lib.rs` | `pub mod williams_r;` under `// slot PSRB-WILLR (williams_r)` (seam-2 :77) | `pub mod cci;` under `// slot PSRB-CCI (cci)` (seam-2 :79) | RSI after :70; FNO-LADDER under seam-2 :83; LS3 at base :883, :2016 |
| `crates/indicators/benches/ratio.rs` | `#[path = "ratio/psrb_willr.rs"]` / `mod psrb_willr;` under `// slot bench-mod PSRB-WILLR`; one statement under `    // slot bench-row PSRB-WILLR` | the same under the two PSRB-CCI slots | RSI, BOLL, STV under their slots; FNO-LADDER and FNO-PAUSE under theirs |
| `docs/04-invariants.md` | above `## One not-owed authority: NSE's withdrawal, before listing, after delisting, unread (R4-19; D-7315)` (:9038) | above `## Corporate actions, basis runs and the day universe (R4-26, D-6892)` (:8671) | ICI-1 edits :9030; BOLL after :9015; STV after :9061; BASE-REDS-1023 edits :8633 |
| `docs/05-decisions.md` | above `### D-7315 — R4-19: one not-owed authority — ...` (:86873) | above `### D-7196 — R4-26 repair 2: a basis step is crossed back by the ratio's LEVELS, ...` (:87133) | PREC-9 after :86535; BOLL after :86591; STV after :86992; AG-06b after :87044 |
| `docs/06-limits.md` | above `## GDFL readers: series codes, stem hand-overs, header shapes, named bad rows (R4-22; D-7310 to D-7314)` (:22007) | above `## Corporate actions, basis runs and the day universe (R4-26, D-6892)` (:21624) | IA-3b, PREC-9 after :21977; ICI-1 edits :22016; BOLL after :22023; SURV-1 after :21734 |
| `docs/07-plan.md` | above `## GDFL one-second fills: the capital-market reader and its archive check — 2026-10-01 (D-0808, D-0812)` (:1084) | above `## Cash-stock research checkpoint — 2026-09-05` (:929) | BOLL after :1057; STV after :1109; RSI after :982; R4-20 at :500 |
| new files | `src/williams_r.rs`, `src/williams_r_tests.rs`, `benches/ratio/psrb_willr.rs` | `src/cci.rs`, `src/cci_tests.rs`, `benches/ratio/psrb_cci.rs` | none |

**How to insert, exactly:**
- `lib.rs`: ONE line, the `pub mod` line, directly under your slot line. No blank or comment line is added. Each
  line sits alone in its own group, so rustfmt has nothing to reorder (the FNO-LADDER form, section 1).
- `benches/ratio.rs`: directly under your own two slot lines, exactly as PSRB-RSI does under its slots
  (`origin/cloud/psrb-rsi-20261103`, the hunks `@@ -68,2 +68,4 @@` and `@@ -781,2 +783,7 @@`): the path attribute and
  the `mod` line under the bench-mod slot; under the bench-row slot,
  `let verdicts: Vec<bool> = verdicts.iter().copied().chain([psrb_willr::row(floor, &["C-PSRB-WILLR-01", "C-PSRB-WILLR-02"])]).collect();`
  (or `psrb_cci` and the two `C-PSRB-CCI` ids), which rustfmt wraps into five lines, the longest 81 characters.
  Nothing else in the file.
- Ledgers: your block goes directly ABOVE the named heading, after the blank line that precedes it, and ends with one
  blank line (PARTS 2.2). In docs/04, 06 and 07 the block is one `## ...` section; in docs/05 it is only your
  `### D-86xx` entries. **One exception, docs/07 for PSRB-WILLR:** the line above `## GDFL one-second fills: ...` is
  not blank (base :1083 is the last line of a list item), so this one block STARTS with a blank line and ends with
  one; the heading then has a blank line above it, which it lacks today. In "insert after" terms the anchor lines are:
  docs/04 :9037 (WILLR), :8670 (CCI); docs/05 :86872, :87132; docs/06 :22006, :21623; docs/07 :1083 (WILLR, the
  block's first line blank), :928 (CCI). The heading under each is the unique text that locates it.
- Ids: invariants `PSRB-<CODE>-Inn`, benches `C-PSRB-<CODE>-nn`, limits `L-PSRB-<CODE>-nn`.

### 3.5 The proof runs (shell, awk and git only; read-only on P)

- **`XW/anchors2-check.sh` -> `XW/anchors2-check.out` (run Thu 5 Nov ~09:3x IST, exit 0, "ANCHORS-2 OK").** It checks the
  base text at every anchor line, builds PSRB-SEAM-2 from PSRB-SEAM's blobs (+12/-0 and +6/-0, every added line a
  comment or blank, three hunks), builds the two new parts' code insertions on it, takes the first wave's `ratio.rs`
  and `lib.rs` from their real tips (RSI `bd7560c699fa`, BOLL `9b60567e163a`, STV `3d1cb6aa72ee`), and merges with
  `git merge-file -p`: RSI x BOLL x STV on PSRB-SEAM, then PSRB-SEAM-2 (base PSRB-SEAM), then WILLR and CCI (base
  PSRB-SEAM-2): all clean; the other order (WILLR x CCI first, then the first wave) is clean too and gives the same
  file byte for byte. Every `pub mod` group of the merged `lib.rs` is sorted (the main group, then `williams_r` alone,
  then `cci` alone). Then every ref that holds `f01d7a0e4` (95 refs under `refs/heads` and `refs/remotes/origin`,
  among them FNO-LADDER, FNO-PAUSE, the fib cloud parts, the data parts, BASE-COV-LIBS LS1-LS4 and the three 24 Oct
  cloud refs CC-5, IA-10a, R3-20) is merged into the first-wave family with and without the second wave, each with its
  own merge base's text as the base: in `ratio.rs` (6 distinct texts), `lib.rs` (7), docs/04 (58), docs/05 (60),
  docs/06 (57) and docs/07 (21), **no ref gains a conflict**. The ledger half uses one synthetic block per new part at
  the named headings and the first wave's real ledger files.
- **`XW/win-check.sh` and `XW/classify.sh` -> `XW/win-chosen.out`.** For each chosen heading, every unique blob of the
  file over all 789 refs: the heading is absent, or present once with the base's exact 4-line window above and below
  it. The only refs with another window are two that wrote or predate the section themselves (the heading is absent
  at their merge base with the base): `fix/db-R4-22` (26 Oct) at docs/06 :22007 and `feat/gdfl-cm-reader` (4 Oct) at
  docs/07 :1084. Neither edits next to the anchor. Neither is an ancestor of the base, and their sections are already
  in the base by another route, so neither is a merge candidate (check 1, N13). The candidates rejected on this
  evidence: docs/04 :8709, :8728, :8749, :8780, :8819, :8859, :8897 and :8982, docs/05 :84468, :85249, :85625,
  :86033, :86536 and :87045 (each has a ref that really edits next to it), and docs/07 :1084's first reading, which
  had no blank line above it (kept, with the leading blank line of 3.4). In docs/06 the rejected ones are :21502,
  :21530, :21564, :21642, :21652, :21780, :21791, :21833, :21846, :21855 and :21978 (:21517, :21574 and :21817 were
  clean too; :21624 and :22007 were taken).
- **Refs of the moment** (repair 3, check 3 N9): 789 refs when this ran; 790 at check 3 (~11:5x IST). The new one,
  `refs/heads/fix/fib-fno-size`, edits no anchor region (runner files; its ledger inserts sit after docs/04 :9112,
  docs/05 :87602, docs/06 :22148 and docs/07 :1126), and check 3's ref and window runs over all 790 found no new
  conflict (`specs-1105-check3-work/refmerge.out`, `win.out`). The intake re-run (7.3) covers every later ref.
- The first-wave tips used are the ones present when the check ran (all three DONE; no `refs/heads/fix/psrb-<part>` exists yet, only `fix/psrb-seam`). A later
  commit on any of them is possible until intake, so the Mac re-runs `anchors2-check.sh` at each intake (section 7, item 3).

### 3.6 The second kit, intake and landing

- **Kit step (main session, after the operator's start):** write `refs/heads/fix/psrb-seam-2` = PSRB-SEAM-2 on
  `fix/psrb-seam` (`35234cead400`), proving it adds exactly `XW/seam2/A2.txt`, `B2.txt` and `C2.txt` at the three
  lines of 3.3; then `refs/heads/cloud/psrb-kit-20261105` = one commit on it adding `reports/psrb-cloud-20261105/`:
  `CLOUD.md` (the first kit's, with this wave's names), `MANIFEST.md`, `PARTS.md` (byte copy; its 2.4 binds),
  `PARTS-WILLR-CCI.md` (byte copy of this file), the coverage recipe and Route B files the first kit carried, and
  `XW/model/wr-cci.mjs`, `pops.mjs` and `cpr-wr6.mjs` fenced as `<name>.mjs.md` (the fixture models), plus repair
  1's `specs-1105-repair1-work/repair1.mjs` fenced as `repair1.mjs.md` (the white-box pop counts and the CCI
  equality clause behind 5.5 and 6.5) and repair 3's `specs-1105-repair3-work/repair3.mjs` fenced as
  `repair3.mjs.md` (the staircase at both generator readings and the CCI ceiling at `u16::MAX`), in the first kit's
  `kit-build.sh` form with MANIFEST sha256 rows. The kit carries the models only, as `<name>.mjs.md`, never an
  `.out` file: this file states every figure a session needs, and the models show how each was computed (repair 3,
  check 3 N1b). Refuse any existing ref. Push the kit branch
  only, after the private scan over the kit folder and the seam-2 diff (the repository is public: every push
  publishes). START for both sessions = `git rev-parse origin/cloud/psrb-kit-20261105~1` = PSRB-SEAM-2.
- **Intake** of a DONE branch, after its tip is the same at two ticks 30 or more minutes apart:
  `W/state/cloud-wave-20261103/replay-minus-reports.sh <P> <PSRB-SEAM-2 sha> <kit-2 sha> <tip> refs/heads/fix/psrb-<part>`
  (START = PSRB-SEAM-2, FROM = the second kit commit), as PARTS 7 item 4.
- **Landing (D1, base green first), as PARTS 7 item 6, with two more merges:** after `fix/psrb-rsi`, `fix/psrb-boll` and
  `fix/psrb-stv`, the main session merges `fix/psrb-willr` (its merge base with the family is PSRB-SEAM; it carries
  PSRB-SEAM-2 in), then `fix/psrb-cci` (merge base PSRB-SEAM-2). Both orders are proved clean (3.5). Gate 14's cover
  table then gains ten C-PSRB ids, not six (the main session's edit at assembly).

---

## 4. Narrow CPR and the pivot levels

### 4.1 What exists (cite, do not re-spec)

- **The previous session's high, low and close** reach the indicators as a caller-certified sealed daily bar:
  `anchored::DailyReference::new` (anchored.rs:111-135) validates it with `check_evaluable` and builds the levels with
  `DailyLevels::from_daily_bar` (anchored.rs:126; daily.rs:255-262).
- **The ladder:** `DailyLevels::from_previous_session(high, low, close)` (daily.rs:181-253) computes, in `i128` from the
  three prices, `p = (H + L + C).div_euclid(3)` (:203), `b = (H + L).div_euclid(2)` (:204), `tc = 2p - b` (:229),
  `r1 = 2p - L` (:206), `s1 = 2p - H` (:211), and narrows once (`fit`, :226-241, :411). Getters: `pivot()` (:276),
  `bc()` (:283), `tc()` (:288), `resistance(1)` (:293), `support(1)` (:298). These are the classic levels the operator
  named ("the CPR indicator's own classic levels, R1 = 2P - L, S1 = 2P - H").
- **The CPR width:** `cpr_width()` (daily.rs:307-341) = `|tc - bc|` of the FLOORED levels, documented as
  `|2c - h - l| / 3` in exact arithmetic and, once floored, "up to one paisa above `range / 3`" (:325-336). That doc
  is loose (repair 1, check N1): with `p = T div 3` and `b = (H + L) div 2`, the floored `tc - bc` is the exact
  `x/3` plus `(H + L) mod 2 - 2 (T mod 3)/3`, so the floored width is within 4/3 paisa of the exact width, above or
  below it, and can be 4/3 paisa above `range / 3` (H 10, L 8, C 8: p 8, b 9, tc 7, width 2, where the exact width
  and `range / 3` are both 2/3). Every `1 <= L <= C <= H <= 60` gives at most 4/3
  (`specs-1105-repair1-work/repair1.out`). `daily.rs` is engine code outside every PSRB part; the main session tells
  its owner.
- **The existing narrow test does not fit the rule.** `cpr_class(CprWidth)` (daily.rs:343-365) compares the width with
  the previous session's RANGE in thousandths (`width x 1000 <= range x narrow`, :356-359), cut at 80 thousandths
  (`CprWidth::CLASSICAL`, :115-121, UNVERIFIED by its own doc, :91), and feeds vocabulary position 63
  `narrow_cpr_day` (daily.rs:602-607; vocab table.rs:354). The operator's rule is a fraction of PRICE (0.10%, 0.20%,
  0.30%) and "narrowest of the last 5 sessions". Neither is in the engine.
- **The EMA 200:** `trend::Ema` (trend.rs:167-261): `Ema::new(200)` (const, :184), `fold(close)` (:207), seeded by the
  simple mean of the first `period` prices (D-1542, :194-206), then `ema += (price x SCALE - ema) x 2 / (n + 1)` floored
  (`div_euclid`, :227-232), held scaled by `SCALE` = 10^6 (:90); `value()` = the average floored to paisa (:237-242);
  `warm()` = `period` folds (:258-260). The engine's own use compares the close with `value()` strictly (`side`,
  trend.rs:1041-1050; :954-958), which is exact for "above" only. PSRB-STRAT's C8 compares on the scaled state
  instead, through one read-only getter it inserts in `trend.rs` (AD 2.3; repair 1, check B3).

### 4.2 Where the width check goes: PSRB-STRAT (recommended and taken)

The check is a feature of one strategy, not an indicator: it reads one daily bar per session and decides one boolean
per session. `daily.rs` is outside every PSRB part's files (PARTS 2.2), and a new `DailyLevels` method would make a
PSRB part edit a module the vocabulary reads (position 63). So it lives in PSRB-STRAT's runner module, beside the
exact R1/S1 test PARTS 6.2 already gives it, and AD section 2 binds its form. No indicator part changes for it.

### 4.3 The exact integer forms (derived here, each re-checked by `XW/model/cpr-wr6.mjs`, kit `cpr-wr6.mjs.md`)

With `H`, `L`, `C` the previous session's high, low and close (paisa, `L <= C <= H`, all positive), let
`x = 2C - H - L` and `T = H + L + C`. Then `P = T/3`, `BC = (H + L)/2`, `TC = 2P - BC`, and in exact arithmetic:

- **Width:** `TC - BC = 2(P - BC) = 2(2T - 3(H + L))/6 = (2C - H - L)/3 = x/3`, so `width = |x|/3`. `|x| <= H - L`
  (equality with the close on an edge), the identity daily.rs:313-318 states.
- **Width as a share of P:** `width / P = (|x|/3) / (T/3) = |x| / T`. The rule "width <= k% of P" with `k` in basis
  points (1 bp = 0.01%, so 0.10% = 10 bp, 0.20% = 20 bp, 0.30% = 30 bp) is
  `|x| / T <= k / 10_000`, i.e. **`10_000 x |2C - H - L| <= k x (H + L + C)`**. Both sides are integers. The scale is
  basis points because the smallest default, 0.10%, is then the integer 10. Bounds: `|x| < 2^63`, so the left side
  `< 2^77`; `T < 3 x 2^63 < 2^65` and `k <= 10_000 < 2^14`, so the right side `< 2^79`: inside `i128`, const-asserted.
  `k` is a `u16` in `1..=10_000`, built once per variant and refused outside by name (`BasisPointsOutOfRange { bp }`,
  AD 2.1). The refusal is what keeps the bound: at `k = u16::MAX` and the largest `T` the right side passes `2^79`,
  though it stays below `2^81` (repair 3, check 3 N4; `repair3.out`).
- **Narrowest of the last 5 sessions:** today's CPR (from the previous session's bar, `x_0`, `T_0`) is the narrowest
  of the last 5 when its share of price is at most each of the 4 CPRs before it (each from its own previous session,
  `x_j`, `T_j`, j = 1..4): `|x_0| / T_0 <= |x_j| / T_j` for every j, i.e. **`|x_0| x T_j <= |x_j| x T_0`**. Ties
  count as narrowest. Both factors are nonnegative and the product is below `(2^63) x (3 x 2^63) < 2^128`, so it is
  computed in `u128`: `x.unsigned_abs()` and `t.unsigned_abs()` on the `i128` values (`T > 0`, so the second is `T`
  itself). There is no `From<i128>` or `From<i64>` for `u128`, so `u128::from` cannot take them (repair 1, check N2).
  The bound,
  `(2^63 - 2) x 3 (2^63 - 1) < 2^128`, is const-asserted with `checked_mul`; it is above `2^127`, so an `i128` product
  can overflow at `i64` extremes (`cpr-wr6.mjs` prints 128 bits; `repair1.out` the bound). The share of price, not
  the paisa width, is compared, so a 5-day price move does not decide it; the paisa form is a labelled twin (AD
  D-8611). The five bars are the last five eligible sealed bars, as `Prev5` counts them (anchored.rs:34): an
  excluded record (a Muhurat or disaster-recovery session) is skipped, never counted (AD 1). With fewer than 5
  eligible bars behind a day, or a refused bar among them, the feature has no reading and the condition is false
  (repair 3, check 3 B3).
- **R1 and S1 on the unrounded pivot (PARTS 6.2):** close above R1 iff `C_t > 2T/3 - L` iff **`3 C_t > 2T - 3L`**;
  close below S1 iff **`3 C_t < 2T - 3H`**. `R1 - S1 = H - L >= 0`, so no close is both above R1 and below S1. The
  floored ladder's `resistance(1)` and `support(1)` give a labelled twin. With `p = T div 3`, the exact R1 minus the
  floored `2p - L` is `2 (T mod 3)/3`, so the floored level sits 0, 2/3 or 4/3 paisa below the exact one, and S1 the
  same (repair 1, check N1). So at most one close decides differently at R1 and at most two at S1; PIV-2
  (`T mod 3 = 2`) shows the 4/3 gap, one R1 close and two S1 closes. **The test is computed in `i128`** (repair 3,
  check 3 N4): at the extremes `3 C_t` and `2T` leave `i64`. With every price in `1..=i64::MAX` and `L <= C <= H`:
  `|3 C_t| < 2^65`, `2T < 2^66`, `2T - 3L = 2H + 2C - L` lies in `(0, 2^65)` and `2T - 3H = 2L + 2C - H` in
  `(-2^63, 2^65)`, each bound const-asserted beside the arithmetic (AD 2.2; `repair3.out`).

**Fixtures (each value by hand, re-checked by `cpr-wr6.mjs`, in the kit as `cpr-wr6.mjs.md`):**

| id | H, L, C (paisa) | x | T | result |
|---|---|---|---|---|
| CPR-1 | 100_100, 99_800, 100_100 | 300 | 300_000 | width 100 paisa = exactly 0.10% of P = 100_000: `3_000_000 <= 3_000_000`, narrow at 10 bp (a tie counts), not at 9 bp |
| CPR-2 | 101_000, 99_000, 100_150 | 300 | 300_150 | narrow at 10 bp (`3_000_000 <= 3_001_500`) |
| CPR-2b | 101_000, 99_000, 100_151 | 302 | 300_151 | not narrow at 10 bp (`3_020_000 > 3_001_510`); narrow at 20 bp |
| PIV-1 | 100_100, 99_800, 100_100 | | 300_000 | P = 100_000 exactly: R1 = 100_200, S1 = 99_900 in both forms; close 100_201 above R1, 100_200 not (a tie); 99_899 below S1, 99_900 not |
| PIV-2 | 10_007, 10_000, 10_001 | | 30_008 | R1 = 30_016/3 = 10_005.33 exact, 10_004 floored: close 10_005 is above the floored R1 only; S1 = 29_995/3 = 9_998.33 exact, 9_997 floored: closes 9_997 and 9_998 are below the exact S1 only |
| N5-1 | days 1-4: (100_300, 99_500, 100_200), (100_250, 99_600, 100_150), (100_400, 99_300, 100_300), (100_200, 99_700, 100_100); the bar today's CPR comes from, the previous session's: (100_200, 99_700, 100_100) | 600, 450, 900, 300; 300 | 300_000 each | shares 20, 15, 30, 10 bp; today 10 bp ties day 4: narrowest (ties count) |
| N5-2 | N5-1 with the bar today's CPR comes from (the previous session's) as (100_200, 99_699, 100_101) | 303 | 300_000 | 10.1 bp: not narrowest |

---

## 5. PSRB-WILLR (wave 2; D-8640..D-8649)

**Goal.** Williams %R as an exact-integer streaming reading with a runtime period and a warm flag, in the form
TradingView's built-in "Williams %R" script computes (`100 x (close - highest) / (highest - lowest)`, the highest high
and lowest low of the last `n` candles, the current one included; UNVERIFIED in this step against TradingView's own
page or output: section 8). In integers: `%R = -100 x (HH - C) / (HH - LL)`, so `%R` lies in `-100..=0`. A window
whose highest high equals its lowest low (every candle in it has `H = L` at one price) has no value: a named flat
state, never a division. Plus the two crossing readings the operator's "crosses above -85" variant needs. First run:
needed (AUTHOR-V2, AD). Size: 2-3 h to write once, S (ESTIMATE).

### 5.1 Files
- N `crates/indicators/src/williams_r.rs`, N `crates/indicators/src/williams_r_tests.rs`, N `crates/indicators/benches/ratio/psrb_willr.rs`.
- E `crates/indicators/src/lib.rs`: `pub mod williams_r;` under `// slot PSRB-WILLR (williams_r)` (seam-2 :77).
- E `crates/indicators/benches/ratio.rs`: under `// slot bench-mod PSRB-WILLR`:
  `#[path = "ratio/psrb_willr.rs"]` / `mod psrb_willr;`; under `    // slot bench-row PSRB-WILLR`:
  `let verdicts: Vec<bool> = verdicts.iter().copied().chain([psrb_willr::row(floor, &["C-PSRB-WILLR-01", "C-PSRB-WILLR-02"])]).collect();`
  (rustfmt wraps it as it wraps PSRB-RSI's).
- E the four ledgers at the anchors of 3.4 (section titles in 5.6).

### 5.2 Public API (`indicators::williams_r`)

```rust
pub const MIN_LEVEL_PPM: i32 = -1_000_000;        // %R -100 (close at the window's low), in ppm of the 0..-100 scale
pub const MAX_LEVEL_PPM: i32 = 0;                 // %R 0 (close at the window's high); -85 is -850_000
pub enum Refusal { PeriodZero, LevelOutOfRange { ppm: i32 }, Corrupt(crate::Corrupt) }  // Clone, Copy, Debug, PartialEq, Eq
pub struct Level(i32);                             // Copy; MIN_LEVEL_PPM..=MAX_LEVEL_PPM
impl Level { pub const fn from_ppm(ppm: i32) -> Result<Self, Refusal>; pub const fn ppm(self) -> i32; }
pub struct WilliamsR { /* period: u16; highs, lows: VecDeque<(u64, i64)> (candle number, price), capacity `period`,
                          allocated in `new`; next: u64 (wrapping candle number); seen: u32 (saturating); close: i64 */ }
                                                   // Clone, Debug, PartialEq, Eq (not Copy: it owns two deques)
impl WilliamsR {
    pub fn new(period: u16) -> Result<Self, Refusal>;           // PeriodZero for 0; 1..=u16::MAX accepted
    pub const fn period(&self) -> u16;
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;
    pub fn warm(&self) -> bool;                                  // at least `period` candles folded
    pub fn reading(&self) -> Option<Reading>;                    // None until warm
}
pub struct Reading { /* period: u16; highest, lowest, close: i64 */ }  // Clone, Copy, Debug, PartialEq, Eq
impl Reading {
    pub const fn period(&self) -> u16;                           // a kept copy judges itself
    pub const fn highest(&self) -> i64;
    pub const fn lowest(&self) -> i64;
    pub const fn close(&self) -> i64;
    pub const fn flat(&self) -> bool;                            // highest == lowest: no %R value
    pub fn ppm(&self) -> Option<i32>;                            // floor(-10^6 (HH - C) / (HH - LL)); None when flat
    pub fn above(&self, level: Level) -> bool;                   // %R > level, exact; false when flat
    pub fn below(&self, level: Level) -> bool;                   // %R < level, exact; false when flat
    pub fn crossed_above(&self, previous: &Reading, level: Level) -> bool;  // previous not above, self above (5.3)
    pub fn crossed_below(&self, previous: &Reading, level: Level) -> bool;  // previous not below, self below
}
pub struct StepCost { pub p50_ns: u64, pub p99_ns: u64, pub max_ns: u64, pub samples: usize }
pub fn summarize(sorted: &[u64]) -> StepCost;
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool;
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>;   // period 14
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>; // history 20_000
```

`Level::from_ppm` and `Level::ppm` are `const fn` (comparisons only). `warm` is not `const` (it widens `period` with
`u32::from`); `ppm`, `above`, `below` and the crossings widen to `i128` with `i128::from` (PARTS 2.4).

### 5.3 Exact formulas and warm-up

- **`fold(c)`**: `c.check_evaluable()` refused first (so every price is a positive `i64` and `L <= C <= H`). Then, with
  `s = next` (this candle's number) and `n = u64::from(period)`:
  1. **Expire.** If the front of `highs` has `s.wrapping_sub(t) >= n`, pop it; the same for `lows`. At most one entry
     per deque can expire per candle: numbers are distinct and rise by one per fold, the front is the oldest entry,
     and after the previous fold every entry had age at most `n - 1` from that candle, so only the entry numbered
     exactly `s - n` can now be out. One `if`, not a loop; both arms are reached by tests.
  2. **Push.** `while highs.back().is_some_and(|&(_, h)| h <= c.high) { highs.pop_back(); }`, then
     `highs.push_back((s, c.high))`; mirrored for `lows` with `>=` and `c.low`. Expiry runs first, so before the push a
     deque holds at most `n - 1` entries and after it at most `n`: the capacity reserved in `new` is never exceeded and
     a deque built by `new` never reallocates (a clone reserves only its length, section 2).
  3. **Count.** `next = s.wrapping_add(1)`; `seen = seen.saturating_add(1)`; `close = c.close`.
- **The counters.** `next` is a `u64` that wraps: an age is the wrapping difference `s.wrapping_sub(t)`, exact while no
  entry is `2^64` candles old, and no entry outlives `period <= 65,535` candles. `seen` is a `u32` that saturates at
  `u32::MAX`, far above every period, so `warm()` (`seen >= u32::from(period)`) never turns false again. Neither needs
  a refusal; both are pinned by white-box tests.
- **The reading.** `warm()` false gives `None`. Else `HH = highs.front()` and `LL = lows.front()`, read with
  `map_or(close, ...)`: the default never applies, because the fold that made the state warm pushed onto both deques
  and a push is never undone without another push (a doc comment says so). With `U = HH - C` and `R = HH - LL`, both
  widened to `i128`: `L_t >= LL` and `H_t <= HH` with `L_t <= C <= H_t` give `0 <= U <= R`.
- **The first reading is at candle `n - 1` (0-based), after `n` candles,** where `ta.highest(n)` and `ta.lowest(n)` first
  have a value.
- **`flat()`**: `HH == LL` (then `U = R = 0`). Every comparison and crossing is false on it, and `ppm()` is `None`, as
  TradingView's division by zero gives `na` (UNVERIFIED, section 8).
- **`above(level)`**, with `l = level.ppm()` (so `-l >= 0`): `R > 0 && 10^6 x U < (-l) x R`. Derivation: for `R > 0`,
  `%R > l / 10^4` (points) iff `-100 U / R > l / 10^4` iff `-10^6 U > l R` iff `10^6 U < -l R`. At `l = -850_000` this is
  `100 U < 85 R`, i.e. `100 x (HH - C) < 85 x (HH - LL)`, the form RC's rule "%R > -85" reads.
  **`below(level)`**: `R > 0 && 10^6 x U > (-l) x R`; at `l = -150_000`, `100 x (HH - C) > 15 x (HH - LL)`. A tie is
  neither above nor below.
- **`ppm()`**: `R == 0` gives `None`; else `(-10^6 x U).div_euclid(R)`, the floor of the exact ppm value, in
  `-1_000_000..=0`, narrowed by `i32::try_from(q).ok()` (PARTS 2.4; it always fits, by the range). `ppm` is for records
  and seeds; decisions read `above` and `below`, which are exact on the state.
- **Crossings (TradingView's `ta.crossover` / `ta.crossunder` form: the current value strictly beyond, the previous one
  at or on the other side).** `crossed_above(previous, level)` = `previous.period == self.period && !previous.flat() &&
  !previous.above(level) && self.above(level)`; `crossed_below` the mirror with `below`. False when either reading is
  flat (TradingView's `na` on either side gives false) or the two readings have different periods. The period check
  catches a reading of another period only; two streams of the same period are the caller's to keep apart (repair 1,
  check N9). The caller keeps the previous candle's reading (it is `Copy`); the module stores none.
- **The two default levels overlap.** Every `%R` strictly between -85 and -15 is both "above -85" and "below -15". The
  module does not hide this; the strategy's side comes from its other conditions (R1/S1 are exclusive, 4.3).
- **Bounds (const-asserted):** `0 <= U <= R < 2^63`; `10^6 x U < 2^20 x 2^63 = 2^83`; `(-l) x R <= 10^6 x 2^63 < 2^83`; the
  floor's dividend `< 2^83`: all inside `i128` (2^127), so no checked or saturating arithmetic is needed.
- **Exactness.** There is no smoothing and no truncation. Every comparison is exact on the published definition; only
  `ppm` floors.
- **Cost.** Per candle and per deque: one expiry check (at most one expiry), one push, and the `while` loop's
  `pop_back`s, called **back-pops** below. Each entry is pushed once and removed at most once, by an expiry or a
  back-pop, so over any run the removals are at most the candles folded: amortized O(1). **At p99 the count is
  bounded by a constant whatever the period:** if more than 1% of `N` candles each removed more than 100 entries, the
  removals would exceed `N`, which they cannot; so at least 99% of candles remove at most 100 per deque. The worst
  single candle back-pops `period - 1` entries (a strictly falling staircase of highs, then a jump) and, with its one
  expiry, removes `period` in all; no candle removes more, because after the expiry a deque holds at most
  `period - 1`. Measured on the model over 20,000 candles of that staircase. **The staircase, exactly** (repair 3,
  check 3 N1a): with `n = period` and `k = 0..=19_999` the index of the fold from the first candle,
  `h_k = 1_000_000` when `k mod n = n - 1`, else `h_k = 900_000 - (k mod n)`; `l_k = h_k - 10`; the open and the close
  `l_k` (any price in `[l_k, h_k]` gives the same deques); volume 0. So the highs fall one paisa a candle and jump at
  the last candle of every block of `n`. The pins need the jump there: with the jump at `k mod n = 0` instead, period
  2,000 gives 17,991 highs back-pops and 9 candles above 100 (`repair3.out`). The model is `XW/model/pops.mjs` (in the
  kit as `pops.mjs.md`), which counts back-pops only. At period 2,000 the highs deque back-pops 19,990 in total, p99 0,
  max 1,999, and 10 candles (0.05%) back-pop more than 100; the lows deque 19,989, p99 1, max 1; at period 14 the
  highs deque 18,564, p99 13, max 13, the lows deque 18,571, p99 1, max 1 (p99 is the value at sorted index
  `floor(0.99 x 20,000)`, from 0). **The pinned figures are back-pops** (repair 1, check B1). Counted with the
  expiries (9 per deque at period 2,000, 1,427 at period 14), the removals are 19,999 (highs, max 2,000) and 19,998
  (lows, max 2) at period 2,000, and 19,991 (max 14) and 19,998 (max 2) at period 14, the same with `next` starting
  at `u64::MAX - 5` (`specs-1105-repair1-work/repair1.out` and `specs-1105-repair3-work/repair3.out` on the Mac; the
  kit carries their models, `repair1.mjs.md` and `repair3.mjs.md`). No deque ever holds more than `period` entries.
  Memory: two deques of 16-byte entries, `32 x period` bytes, about 2 MiB at `u16::MAX` (ESTIMATE: `VecDeque` may
  reserve more than asked), allocated once in `new` (L-PSRB-WILLR-02).
- **Across sessions.** The stream carries across session boundaries (TradingView's intraday charts do; WU "Indicators
  run continuously across sessions"). A hole is PSRB-STRAT's (PARTS 6.2): the module reads OHLC only.

### 5.4 Refusals
`PeriodZero` (`new(0)`); `LevelOutOfRange { ppm }` (`Level::from_ppm` above 0 or below -1_000_000, naming the value);
`Corrupt(why)` (the candle's own `check_evaluable` refusal, lib.rs:1778: inverted, overflowing range, price outside
the range, negative volume, nonpositive price). Each leaves the state unchanged.

### 5.5 Tests (`src/williams_r_tests.rs`; deterministic xorshift, many seeds, no new dependency)

Fixture WR-1 (period 3; H, L, C in paisa), by hand in exact rationals and re-checked by `XW/model/wr-cci.mjs`
(`willr` and `willrBrute`) and `cpr-wr6.mjs` (in the kit as `wr-cci.mjs.md` and `cpr-wr6.mjs.md`):

| candle | H, L, C | window | HH | LL | U = HH - C | R = HH - LL | ppm | exact %R | > -85 | < -15 | crossed above -85 | crossed below -15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 110, 100, 105 | - | - | - | - | - | - | - | - | - | - | - |
| 1 | 115, 104, 114 | - | - | - | - | - | - | - | - | - | - | - |
| 2 | 113, 101, 102 | 0..2 | 115 | 100 | 13 | 15 | -866_667 | -260/3 = -86.666... | no | yes | - (no previous) | - |
| 3 | 120, 108, 119 | 1..3 | 120 | 101 | 1 | 19 | -52_632 | -100/19 = -5.263... | yes | no | yes | no |
| 4 | 118, 112, 112 | 2..4 | 120 | 101 | 8 | 19 | -421_053 | -800/19 = -42.105... | yes | yes | no | yes |
| 5 | 110, 100, 103 | 3..5 | 120 | 100 | 17 | 20 | -850_000 | -85 exactly | no (tie) | yes | no | no |
| 6 | 121, 105, 120 | 4..6 | 121 | 100 | 1 | 21 | -47_620 | -100/21 = -4.761... | yes | no | yes (previous ON the level) | no |

Fixture WR-2 (period 3): three candles (500, 500, 500), then (501, 500, 501), then (501, 499, 499). After the third:
`flat()` true, `ppm()` None, every comparison false. After the fourth: HH 501, LL 500, U 0, R 1, `ppm` 0, `above(-850_000)`
true, `crossed_above(previous = the flat reading, -850_000)` false. After the fifth: HH 501, LL 499, U 2, R 2, `ppm`
-1_000_000, `below(-150_000)` true, `crossed_below(previous, -150_000)` true.

- `a_zero_period_is_refused_and_every_other_period_is_accepted` (0 refused with `PeriodZero`; 1, 14 and `u16::MAX`
  accepted, each deque's capacity at least `period`).
- `a_level_outside_minus_one_hundred_to_zero_is_refused_at_both_edges` (`Level::from_ppm` accepts -1_000_000 and 0 and
  `ppm()` reads each back; refuses 1, `i32::MAX`, -1_000_001 and `i32::MIN` with `LevelOutOfRange { ppm }` naming the
  value).
- `the_first_reading_appears_after_exactly_period_candles` (periods 1, 2, 3, 14, `u16::MAX`: none after `period - 1`
  candles, one after `period`).
- `williams_r_matches_fixture_wr_1` (the table, every column, at every candle).
- `a_tie_at_the_level_is_neither_above_nor_below` (WR-1 at candle 5, exactly -85: `Level -850_000` neither;
  `-850_001` above; `-849_999` below).
- `a_floored_ppm_equal_to_the_level_is_not_a_tie` (repair 4, check 4 B3; the model is PSRB-RSI's
  `a_floor_ppm_equal_to_the_level_is_not_a_tie`, PARTS 3.5. Period 1, the candle (2_000_001, 1_000_000, 1_150_001):
  HH 2_000_001, LL 1_000_000, U 850_000, R 1_000_001, `ppm()` -850_000, the level itself, yet the exact %R is
  -85_000_000/1_000_001 = -84.999915...: `above(Level -850_000)` true (`10^6 x 850_000` = 850_000_000_000 <
  `850_000 x 1_000_001` = 850_000_850_000) and `below` false, where a decision on `ppm() > level` would read false.
  With `a_tie_at_the_level_is_neither_above_nor_below`, where `ppm() >= level` would read true, it tells the exact
  comparison from both floored forms. `below` needs no such case: for an integer level `l`, `floor(x) < l` exactly
  when `x < l`. Figures: `specs-1105-repair4-work/fixtures.out`.)
- `a_flat_window_has_no_value_and_compares_false` (WR-2: at the flat candle `ppm` None and `above`/`below` false at
  -1_000_000, -850_000, -150_000 and 0; the next candle reads 0).
- `a_period_of_one_reads_the_candle_alone` (period 1: (110, 90, 100) -> -500_000; (100, 100, 100) flat; (120, 100, 120)
  -> 0; (120, 100, 100) -> -1_000_000).
- `the_crossings_follow_tradingviews_crossover` (WR-1: `crossed_above(-85)` true at candles 3 and 6 only, candle 6
  after a previous reading exactly on the level; `crossed_below(-15)` true at candle 4 only; WR-2: false from a flat
  previous reading, true at its fifth candle; a previous reading of another period: false both ways).
- `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen` (WR-1 candle 4, -42.105: both
  `above(-850_000)` and `below(-150_000)` true; pins that the module does not make the two sides exclusive).
- `every_reading_lies_between_minus_one_hundred_and_zero` (random series: `ppm` in -1_000_000..=0; a close on the
  window's high reads 0, on its low -1_000_000).
- `ppm_is_the_floor_of_the_exact_ratio` (random non-flat readings: `ppm x R <= -10^6 x U < (ppm + 1) x R`).
- `the_deques_equal_a_brute_force_window` (BRUTE FORCE: at every candle the highest high and lowest low of the last
  `period` candles by a direct scan, and every comparison at -850_000, -150_000 and -500_000 against it; 1,000 random
  series with flat candles, repeated highs and lows mixed in, periods 1..=40).
- `parity_with_the_published_definition` (an exact rational transcription, `100 x (C - HH) / (HH - LL)` as `num/den`
  in `i128`, written apart from the module: each comparison against the rational and `ppm` as its floor, on random
  series and both fixtures; the model's run of the same check, 8,631 warm readings, found 0 disagreements).
- `the_pops_stay_bounded_at_the_ninety_ninth_percentile` (white-box, in the child test module; repair 1, check B1: it
  counts BACK-POPS, the `while` loop's `pop_back`s, per deque and per fold, as
  `len_before + 1 - len_after - expired`, where `expired` is 1 when, before the fold, the deque is non-empty and its
  front's age `next.wrapping_sub(front.0)` is at least `period`, else 0. The staircase of 5.3, exactly as stated
  there (`h_k = 1_000_000` when `k mod n = n - 1`, else `900_000 - (k mod n)`; `l_k = h_k - 10`; open and close
  `l_k`; volume 0; `k = 0..=19_999`; repair 3, check 3 N1a), 20,000 candles, at
  periods 14 and 2_000, once with `next` starting at 0 and once at `u64::MAX - 5`: per deque the total back-pops are
  at most the candles folded, and fewer than 1% of candles back-pop more than 100. It pins the model's back-pop
  figures, the same at both starts: period 2_000, highs total 19_990, max 1_999, 10 candles above 100; lows total
  19_989, max 1; period 14, highs total 18_564, max 13; lows total 18_571, max 1. It also asserts that no fold
  removes more than `period` entries from a deque in all (`len_before + 1 - len_after`, the back-pops plus the
  expiry), and that the highs deque removes exactly `period` at every jump after the first).
- `at_most_one_entry_expires_per_candle` (white-box: after every fold, each deque's front has age below `period`, the
  numbers in each deque rise strictly, and the prices in `highs` fall strictly and in `lows` rise strictly front to
  back).
- `the_deques_never_grow_past_their_period_or_reallocate` (length at most `period` and capacity unchanged after
  100,000 folds at periods 1, 14 and 1_000).
- `the_candle_counter_wraps_without_changing_a_reading` (white-box: a fresh state with `next = u64::MAX - 5` folds the
  same 40 candles as a fresh state with `next = 0`; every reading is equal).
- `the_warm_counter_saturates_and_stays_warm` (white-box: a warm state with `seen = u32::MAX - 1` folds two candles,
  reads `seen == u32::MAX` and stays warm, every reading equal to a copy with its true count).
- `the_largest_prices_and_period_never_overflow` (H `i64::MAX`, L 1, with C 1, `i64::MAX` and `i64::MAX - 1`, at periods
  1 and `u16::MAX`: `ppm` -1_000_000, 0 and -1; the const bounds restated as runtime checks at the extremes).
- `a_price_gap_enters_the_window_and_leaves_it_after_period_candles` (a 10% jump in the high is HH for exactly
  `period` candles, then leaves, equal to the brute force throughout).
- `a_time_gap_reads_as_contiguous_candles` (stamps a day apart or a minute apart give equal states).
- `no_reading_depends_on_a_later_candle` (redraw every candle after t: the reading at t is unchanged).
- `a_corrupt_candle_is_refused_and_changes_nothing` (each `Corrupt` kind, `PriceNotPositive` included; whole state
  compared, deques and counters).
- `the_state_is_a_fixed_size_and_allocates_only_in_new` (`size_of::<WilliamsR>()` at most 128 bytes, ESTIMATE; both
  capacities unchanged by folding).
- `the_previous_reading_judges_with_the_current_candle_excluded` (a copied reading keeps its window after the next
  fold, so a caller can use it in the engine's "anchor excludes the candle" convention, PARTS 2.4).
- `the_bench_measures_both_rows_at_tiny_sizes` and `a_cost_is_read_by_rank_and_judged_flat_or_not` (cover
  `measure_step`, `measure_period`, `summarize`, `flat_within`, a zero first p99 failing).

### 5.6 Benches, rows and ledger blocks
- **C-PSRB-WILLR-01**: one `fold` plus `reading()`, both comparisons and `crossed_above` against the previous reading,
  after 2_000, 20_000 and 200_000 candles of the deterministic walk, period 14. p99 flat within 3.0x of the first size,
  inside a 2 µs budget (target 300 ns, ESTIMATE; PSRB-RSI's cloud run read 40-42 ns for its step, advisory).
- **C-PSRB-WILLR-02**: the same step at periods 14, 140 and 1_400 after 20_000 candles: the period never enters the
  per-candle p99 (the counting argument of 5.3). Same ceiling and budget.
- docs/04 section `## Williams %R for the PSRB family (PSRB-WILLR; D-8640 to D-8644)` with `| Id | Invariant | Proof | |`:
  - PSRB-WILLR-I01: the window is the last `period` candles' highs and lows, the current one included; the first
    reading after exactly `period` candles at every period up to `u16::MAX` (warm-up, fixture WR-1 and period-one tests).
  - PSRB-WILLR-I02: comparisons exact on the published definition, `10^6 x (HH - C)` against `|level| x (HH - LL)`; a tie
    is neither; `ppm` is the floor, and a floored `ppm` equal to the level is not a tie; every reading lies in -100..=0
    (tie, floor and range tests: `a_tie_at_the_level_is_neither_above_nor_below`,
    `a_floored_ppm_equal_to_the_level_is_not_a_tie`, `ppm_is_the_floor_of_the_exact_ratio` and
    `every_reading_lies_between_minus_one_hundred_and_zero`; repair 4, check 4 B3).
  - PSRB-WILLR-I03: a flat window has no value and compares false; the crossings follow `ta.crossover` and are false
    next to a flat reading or across periods; the two default levels overlap (flat, crossing and overlap tests).
  - PSRB-WILLR-I04: the deques equal the brute-force window, and the readings equal the published definition in exact
    rationals (brute-force and parity tests).
  - PSRB-WILLR-I05: at most one expiry per candle; back-pops at most the candles folded and fewer than 1% of candles
    above 100; at most `period` removals from a deque on any candle, the expiry included; the deques never pass
    `period`, and a state built by `new` never reallocates; the counters wrap and saturate without changing a
    reading; no overflow at any positive `i64` price and any period; every refusal changes nothing (deque, wrap,
    saturation, extremes, refusal and level-edge tests).
  - PSRB-WILLR-I06: no reading depends on a later candle or on a timestamp (look-ahead and time-gap tests).
  - C-PSRB-WILLR-01, C-PSRB-WILLR-02 rows naming the two bench tests. Every proof cell in the path-qualified form gate
    10 reads (PARTS 2.4), e.g. `` `williams_r_matches_fixture_wr_1` and `the_first_reading_appears_after_exactly_period_candles` in `crates/indicators/src/williams_r_tests.rs` ``.
- docs/06 section `## Williams %R for the PSRB family: limits (PSRB-WILLR; D-8640 to D-8644)`, FNO-LADDER's columns
  `| ID | Limit | Why it is not O(1) / not closed | Bound | Package |`: L-PSRB-WILLR-01 the worst single candle
  back-pops `period - 1` entries from a deque (a staircase) and, with its one expiry, removes `period` in all, so the
  per-candle cost is O(1) amortized and at p99 (at most 100 removals per deque at p99, by counting), not in the worst
  case; the staircase figures the pop test pins are back-pops (repair 1, check B1); L-PSRB-WILLR-02 memory
  O(period), 32 bytes a candle, about 2 MiB at `u16::MAX` (ESTIMATE: `VecDeque` may reserve more than asked),
  allocated once; L-PSRB-WILLR-03 what was measured (the cloud figure is advisory; the Mac's
  figure is owed until its check; never a number not measured); L-PSRB-WILLR-04 the definition is the published one,
  UNVERIFIED against TradingView's own output, and no real candle has passed through it until PSRB-STRAT.
- docs/07 section `## PSRB-WILLR: Williams %R — 2026-11-05 (D-8640 to D-8644)` with `| Part | What | State |` and one row
  ("Built on `cloud/psrb-willr-20261105`; not merged (D1). PSRB-STRAT is its first caller").
- docs/05 entries (`### D-864n — PSRB-WILLR: ... — 2026-11-05`):
  - D-8640: Williams %R as `indicators::williams_r`, no vocabulary position; the published form (the highest high and
    lowest low of the last `period` candles, the current one included; `-100 x (HH - C) / (HH - LL)`); runtime period
    `u16` 1..=65,535; the first reading after `period` candles; carried across sessions, a daily reset is a new state.
  - D-8641: units and comparisons: levels in ppm of the 0..-100 scale, `Level` refused outside -1_000_000..=0; exact
    comparisons `10^6 U` against `|l| R` in `i128`, a tie neither; `ppm` the floor, for records only; bounds
    const-asserted; the flat window (`HH == LL`) a named state with no value, every comparison false (TradingView's
    `na`, UNVERIFIED); the two default levels overlap, and the module says so.
  - D-8642: the crossings in TradingView's `ta.crossover` / `ta.crossunder` form for the operator's "crosses above -85"
    variant; false next to a flat reading or across periods; the previous reading is the caller's copy.
  - D-8643: the two monotonic deques: expiry first, then back-pops and one push, so a deque never passes `period`
    entries and, built by `new`, never reallocates (a clone reserves only its length; the measured path never
    clones); at most one expiry per candle; removals amortized O(1), at most 100 per deque at
    p99 by counting; the worst candle back-pops `period - 1` and removes `period` with its expiry (L-PSRB-WILLR-01);
    the pop test counts back-pops white-box, `len_before + 1 - len_after - expired` (5.5); the wrapping `u64` candle
    number and the saturating `u32` warm count; `map_or` where a deque's `Option` cannot be `None`; refusals and validation order; timestamps
    not read; no look-ahead.
  - D-8644: C-PSRB-WILLR-01 and -02, their sizes, ceiling and 2 µs budget; the helpers copied per part (PARTS 2.4).

### 5.7 NEEDS THE MAC
Check 1 on the Mac's pin (Route B on the D2 set against START = PSRB-SEAM-2; the bench wrapper and `main` by the
instrumented bench run), its gate 8 run of `cargo bench -p indicators --bench ratio` with both C-PSRB-WILLR rows,
gates 10, 11, 12 and 14 (gate 14's cover-table row for the new ids is the main session's at the family assembly),
macOS rows (none expected: no platform code), `XW/anchors2-check.sh` re-run against the then-current first-wave tips,
and the anchors on the then-current feat/db-fix.

---

## 6. PSRB-CCI (wave 2; D-8650..D-8659)

**Goal.** The Commodity Channel Index on the typical price, in the form TradingView's built-in "CCI" script computes
(`ta.cci(hlc3, n)` = `(src - ta.sma(src, n)) / (0.015 x ta.dev(src, n))`, `ta.dev` the mean absolute deviation from the
same mean; UNVERIFIED in this step against TradingView's own page or output: section 8), as an exact-integer streaming
reading with a runtime period, a ring allocated once, and a named flat state when the mean deviation is 0. **In
thirds and n-multiples every quantity is an integer:** with `T = H + L + C` (= 3 TP), `S` = the sum of the last `n`
values of `T` (= 3 n SMA), `dev = n T_t - S` (= 3 n (TP - SMA)) and `D = sum |n T_i - S|` over the window
(= 3 n^2 MD), **`CCI = 200 x n x dev / (3 x D)`**. First run: needed (AUTHOR-V2, AD). Size: 2-3 h to write once, S
(ESTIMATE).

### 6.1 Files
- N `crates/indicators/src/cci.rs`, N `crates/indicators/src/cci_tests.rs`, N `crates/indicators/benches/ratio/psrb_cci.rs`.
- E `crates/indicators/src/lib.rs`: `pub mod cci;` under `// slot PSRB-CCI (cci)` (seam-2 :79).
- E `crates/indicators/benches/ratio.rs`: under `// slot bench-mod PSRB-CCI` and `    // slot bench-row PSRB-CCI`
  (ids `"C-PSRB-CCI-01", "C-PSRB-CCI-02"`), the 5.1 form.
- E the four ledgers at the anchors of 3.4.

### 6.2 Public API (`indicators::cci`)

```rust
pub const MAX_LEVEL_MILLI: i32 = 10_000_000;      // CCI +/-10,000 points, in thousandths of a point (+200 is 200_000)
pub enum Refusal { PeriodZero, LevelOutOfRange { milli: i32 }, Corrupt(crate::Corrupt) }  // Clone, Copy, Debug, PartialEq, Eq
pub struct Level(i32);                             // Copy; -MAX_LEVEL_MILLI..=MAX_LEVEL_MILLI
impl Level { pub const fn from_milli(milli: i32) -> Result<Self, Refusal>; pub const fn milli(self) -> i32; }
pub struct Cci { /* period: u16; ring: VecDeque<i128> of T = H + L + C, capacity `period`, allocated in `new`;
                    sum: i128 (S); last: i128 (the current candle's T, stored by fold) */ }  // Clone, Debug, PartialEq, Eq
impl Cci {
    pub fn new(period: u16) -> Result<Self, Refusal>;           // PeriodZero for 0; 1..=u16::MAX accepted
    pub const fn period(&self) -> u16;
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;  // O(1)
    pub fn warm(&self) -> bool;                                  // `period` candles held
    pub fn reading(&self) -> Option<Reading>;                    // None until warm; O(period): one pass for D (6.3)
}
pub struct Reading { /* period: u16; deviation: i128 (n T - S); total_deviation: i128 (D) */ }  // Clone, Copy, Debug, PartialEq, Eq
impl Reading {
    pub const fn period(&self) -> u16;
    pub const fn deviation(&self) -> i128;                       // n T_t - S = 3 n (TP - SMA)
    pub const fn total_deviation(&self) -> i128;                 // D = sum |n T_i - S| = 3 n^2 MD
    pub const fn flat(&self) -> bool;                            // D == 0: every typical price in the window equal
    pub fn milli(&self) -> Option<i64>;                          // floor(1000 CCI) = floor(200_000 n dev / (3 D)); None when flat
    pub fn above(&self, level: Level) -> bool;                   // CCI > level, exact; false when flat
    pub fn below(&self, level: Level) -> bool;                   // CCI < level, exact; false when flat
}
// StepCost, summarize, flat_within, measure_step (period 20), measure_period (history 20_000): the 5.2 forms;
// measure_period times `fold` alone (C-PSRB-CCI-02), and a third, measure_reading_period, times the step with
// `reading()` for the wrapper's unjudged line (6.6).
pub fn measure_reading_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>;
```

### 6.3 Exact formulas and warm-up

- **`fold(c)`**: `c.check_evaluable()` refused first. Then `t = i128::from(c.high) + i128::from(c.low) + i128::from(c.close)`
  (widened before the sum, as daily.rs:196-199 does); if the ring holds `period` values,
  `sum -= ring.pop_front().unwrap_or(0)` (the default never applies: the arm runs only when the ring is full, and
  `period >= 1`; a doc comment says so); then `ring.push_back(t)`, `sum += t`, `last = t`. The pop comes first, so the
  ring never holds more than `period` values, and a ring built by `new` never reallocates (a clone of a `Cci` not yet
  warm reserves only its length, section 2). Two `i128` additions, one pop, one push: O(1).
- **`warm()`**: `ring.len() == usize::from(period)`. The first reading is at candle `n - 1` (0-based), after `n` candles,
  where `ta.sma(hlc3, n)` and `ta.dev(hlc3, n)` first have a value.
- **`reading()`**: `None` until warm; else with `n = i128::from(period)`: `dev = n x last - sum`, and
  `D = ring.iter().map(|&t| (n x t - sum).abs()).sum()`, one pass over the ring. `Reading` copies `period`, `dev` and
  `D`; it never reads the ring again.
- **The identity, derived.** `TP = T/3`, `SMA = S/(3n)`, so `TP_t - SMA = (n T_t - S)/(3n) = dev/(3n)`. `MD =
  (1/n) sum |TP_i - SMA| = (1/n) sum |n T_i - S|/(3n) = D/(3n^2)`. `CCI = (TP - SMA)/(0.015 MD) = (dev/(3n)) x
  (200/3) x (3n^2/D) = 200 n dev / (3 D)` (0.015 = 3/200). **The confirmed level: `CCI > +200` iff `200 n dev > 600 D`
  iff `n x dev > 3 x D` iff `(TP - SMA) > 3 x MD`** (multiply both sides of `TP - SMA > 3 MD` by `3n^2`: `n dev > 3 D`);
  `CCI < -200` iff `n x dev < -3 x D`.
- **`above(level)`**, with `m = level.milli()`: `D > 0 && 200_000 x n x dev > 3 x m x D` (from `1000 x CCI > m`,
  multiplied by `3 D > 0`). **`below(level)`**: `D > 0 && 200_000 x n x dev < 3 x m x D`. A tie is neither.
- **`milli()`**: `D == 0` gives `None`; else `(200_000 x n x dev).div_euclid(3 x D)`, narrowed by `i64::try_from(q).ok()`.
  It always fits: by the ceiling below, `|q| <= 100_000 n / 3 <= 2_184_500_000 < 2^32`. For records and seeds only.
- **`flat()`**: `D == 0`, which holds exactly when every `n T_i` equals `S`, i.e. every typical price in the window is
  equal. Different candles can share a typical price ((101, 99, 100) and (102, 98, 100) both have T = 300), so a flat
  window need not be a flat market. `dev` is then 0 too: TradingView's `0/0` is `na` (UNVERIFIED). Every comparison is
  false on it and `milli()` is `None`.
- **The ceiling.** The deviations `n T_i - S` sum to 0, so `D` is twice the sum of the positive ones, and
  `D >= 2 |dev|`. Hence **`|CCI| <= 200 n |dev| / (3 x 2 |dev|) = 100 n / 3`**, reached exactly when `dev != 0` and
  the current candle is the only one strictly on its side of the mean (every other deviation has the other sign or is
  0). At `dev == 0` the reading is 0, and `2 |dev| = D` holds only in the flat window, which has no reading (repair 1,
  check N3). Consequences the strategy must know: a period of 6 or less can never read
  above +200 (at 6 the ceiling is exactly +200); at the default 20 the ceiling is 666.67; a period of 1 is always flat
  (`D = |T - T| = 0`); a period of 2 reads exactly +66.666... or -66.666... unless flat.
- **Bounds (const-asserted):** `3 <= T <= 3(2^63 - 1) < 2^65`; `0 < S < 2^16 x 2^65 = 2^81`; `|n T - S| < 2^81`;
  `D < 2^16 x 2^81 = 2^97`; `200_000 x n x |dev| < 2^18 x 2^16 x 2^81 = 2^115`; `3 x |m| x D <= 3 x 10^7 x 2^97 < 2^123`
  (`10^7 < 2^24`): all inside `i128`. `MAX_LEVEL_MILLI` (10,000 points) is a chosen domain, not forced by this bound:
  `3 |m| D < 2^127` with `D < 2^97` would allow `|m|` up to about 357,913,941 thousandths (about 357,914 points). The
  chosen domain is 50 times the confirmed level and keeps the product below `2^123` (repair 1, check N10). A level
  beyond +/-10,000 points is refused, never wrapped (L-PSRB-CCI-01). Any positive `i64` price is accepted: no price
  ceiling is needed.
- **Exactness.** No smoothing, no truncation: every comparison is exact on the published definition; only `milli`
  floors.
- **Cost (the one non-O(1) reading in this file; L-PSRB-CCI-02).** `fold` is O(1) in every case. `reading()` is
  O(period): the mean deviation is taken around the CURRENT mean, which moves with every candle, so every term of `D`
  changes and one pass is needed. At the confirmed period 20 that is 20 multiply-subtract-abs-add steps in `i128`. An
  order-statistic tree with subtree sums (the count and sum of window values above the mean give `D = 2 x (n x
  sum_above - S x count_above)`) would make it O(log period); no O(1) method is known to this file. **Decided (repair
  1, check N4; D-8653): O(period) is accepted and no tree is built**, the default the CCI brief carries; the decision
  goes to the operator in the evening report, and the operator may reverse it (section 8). C-PSRB-CCI-02 judges `fold`
  alone, so no judged row measures `reading()` across periods; the wrapper's unjudged line prints that slope (6.6).
  Memory: 16 bytes a candle, about 1 MiB at `u16::MAX` (ESTIMATE: `VecDeque` may reserve more than asked), allocated
  once (L-PSRB-CCI-03).
- **Across sessions** and **holes**: as 5.3.

### 6.4 Refusals
`PeriodZero` (`new(0)`); `LevelOutOfRange { milli }` (`Level::from_milli` beyond +/-10_000_000, naming the value);
`Corrupt(why)` (the candle's own `check_evaluable` refusal). Each leaves the state unchanged.

### 6.5 Tests (`src/cci_tests.rs`; deterministic xorshift, many seeds, no new dependency)

Fixture CCI-1 (period 4; H, L, C in paisa), by hand and re-checked by `XW/model/wr-cci.mjs` (in the kit as
`wr-cci.mjs.md`; `cci` and the independent `cciPublished`, which computes TP, SMA, MD and `(TP - SMA)/(0.015 MD)` in
exact rationals):

| candle | H, L, C | T | window T | S | deviations n T_i - S | dev | D | milli | exact CCI |
|---|---|---|---|---|---|---|---|---|---|
| 0 | 110, 100, 105 | 315 | - | - | - | - | - | - | - |
| 1 | 112, 104, 110 | 326 | - | - | - | - | - | - | - |
| 2 | 108, 98, 100 | 306 | - | - | - | - | - | - | - |
| 3 | 120, 106, 118 | 344 | 315, 326, 306, 344 | 1_291 | -31, 13, -67, 85 | 85 | 196 | 115_646 | 17_000/147 = 115.646258... |
| 4 | 125, 115, 124 | 364 | 326, 306, 344, 364 | 1_340 | -36, -116, 36, 116 | 116 | 304 | 101_754 | 5_800/57 = 101.754385... |
| 5 | 126, 120, 121 | 367 | 306, 344, 364, 367 | 1_381 | -157, -5, 75, 87 | 87 | 324 | 71_604 | 5_800/81 = 71.604938... |
| 6 | 118, 100, 101 | 319 | 344, 364, 367, 319 | 1_394 | -18, 62, 74, -118 | -118 | 272 | -115_687 | -5_900/51 = -115.686274... |

At `Level 100_000`: above at candles 3 and 4, not at 5 or 6; at `Level -100_000`: below at candle 6 only.

Fixture CCI-2 (period 7, the +200 tie): five candles (101, 99, 100) (T 300), then (105, 103, 104) (T 312), then
(113, 111, 113) (T 337). S 2_149, deviations -49 (x5), 35, 210; dev 210, D 490; CCI = 200 x 7 x 210 / (3 x 490) =
**exactly +200**, `milli` 200_000: `above(200_000)` false, `below(200_000)` false, `above(199_999)` true,
`below(200_001)` true. Its mirror CCI-2m: five (101, 99, 100), then (97, 95, 96) (T 288), then (89, 87, 87) (T 263):
S 2_051, dev -210, D 490, **exactly -200**: `below(-200_000)` false, `below(-199_999)` true, `above(-200_001)` true.
(Built from `dev = 6 x` the other positive deviation, which makes `n dev = 3 D` at n = 7.)

Fixture CCI-3 (period 7, the ceiling): six (101, 99, 100), then (101, 99, 101) (T 301): S 2_101, dev 6, D 12; CCI =
700/3 = 233.333... = 100 n / 3, `milli` 233_333, `above(200_000)` true. At period 6 the same shape (five, then the 301)
gives S 1_801, dev 5, D 10, CCI exactly +200: `above(200_000)` false.

- `a_zero_period_is_refused_and_every_other_period_is_accepted` (0; 1; 20; `u16::MAX`; ring capacity at least `period`).
- `a_level_past_ten_thousand_points_is_refused_at_both_edges` (`Level::from_milli` accepts 10_000_000, -10_000_000 and
  0, read back by `milli()`; refuses 10_000_001, -10_000_001, `i32::MAX` and `i32::MIN` with `LevelOutOfRange { milli }`
  naming the value).
- `the_first_reading_appears_after_exactly_period_candles` (periods 1, 2, 3, 20, `u16::MAX`).
- `cci_matches_fixture_cci_1` (the table: T, S, dev, D and `milli` at every candle, and the four level results).
- `a_tie_at_plus_two_hundred_is_neither_above_nor_below` (CCI-2 and CCI-2m, every comparison listed above).
- `a_floored_milli_equal_to_the_level_is_not_a_tie` (repair 4, check 4 B3; as 5.5's. Period 7: five candles
  (1_000_000, 1_000_000, 1_000_000) (T 3_000_000), one (1_040_000, 1_040_000, 1_040_000) (T 3_120_000), then
  (1_123_334, 1_123_333, 1_123_334) (T 3_370_001): S 21_490_001, deviations -490_001 (x5), 349_999 and 2_100_006;
  dev 2_100_006, D 4_900_010, `milli()` 200_000, the level itself, yet CCI = 2_940_008_400/14_700_030 =
  200.000163...: `above(Level 200_000)` true (`200_000 x 7 x 2_100_006` = 2_940_008_400_000 >
  `3 x 200_000 x 4_900_010` = 2_940_006_000_000) and `below` false, where a decision on `milli() > level` would read
  false. With CCI-2's tie, where `milli() >= level` would read true, it tells the exact comparison from both floored
  forms; `below` needs no such case, as in 5.5. Figures: `specs-1105-repair4-work/fixtures.out`.)
- `the_confirmed_level_is_three_mean_deviations` (random warm readings: `above(Level 200_000)` equals `n x dev > 3 x D`,
  and equals `TP - SMA > 3 x MD` computed in exact rationals apart from the module; the mirror at -200 likewise).
- `the_reading_never_passes_one_hundred_n_over_three` (CCI-3: 233_333; random warm readings: `2 x |dev| <= D`; for
  `dev != 0`, equality exactly when the current candle is the only one strictly on its side of the mean; at
  `dev == 0`, equality exactly when the window is flat, `D == 0`. Repair 1, check N3. The model's run met equality with
  `dev != 0` on 467 of its 8,631 warm readings; `repair1.out` checks the clause on 60,000 random windows, 0
  contradictions. The ceiling at `u16::MAX`, a `milli` above `i32::MAX` (repair 3, check 3 N1c; check 2 N5): 65,534
  candles (1, 1, 1), then (2, 2, 2): S 196_608, dev 196_602, D 393_204 = 2 dev, `milli` exactly 2_184_500_000 =
  `100_000 n / 3`, above `i32::MAX`, which is why `milli()` returns `Option<i64>`; `repair3.out`).
- `a_period_of_six_or_less_never_reads_above_two_hundred` (the period-6 ceiling fixture reads exactly 200_000 and is not
  above; random series at periods 1..=6: `above(Level 200_000)` never true).
- `a_flat_window_has_no_value_and_compares_false` (period 3: (101, 99, 100), (102, 98, 100), (100, 100, 100), all T 300:
  D 0, `flat()` true, `milli` None, `above` and `below` false at -10_000_000, -200_000, 0, 200_000 and 10_000_000).
- `a_period_of_one_is_always_flat` (random candles at period 1: every reading flat).
- `a_period_of_two_reads_plus_or_minus_sixty_six_point_six` ((110, 90, 100), (120, 100, 110): 66_666; then
  (100, 90, 95): -66_667; then (100, 90, 95) again: flat).
- `milli_is_the_floor_of_the_exact_ratio` (random non-flat readings: `milli x 3D <= 200_000 n dev < (milli + 1) x 3D`).
- `the_ring_equals_a_brute_force_window` (BRUTE FORCE: at every candle S and D recomputed from the last `period`
  candles by an independent loop; 1,000 random series, periods 1..=40).
- `parity_with_the_published_definition` (exact rationals: TP = (H + L + C)/3, SMA, MD, `(TP - SMA)/(0.015 MD)` as
  `num/den` in `i128`, written apart from the module; every comparison at +/-200, 100 and 0 against it and `milli` as
  its floor, on random series and the three fixtures; the model's run, 8,631 warm readings, found 0 disagreements).
- `the_largest_prices_and_period_never_overflow` (period `u16::MAX` of alternating (1, 1, 1) and (`i64::MAX`,
  `i64::MAX`, `i64::MAX`), and period 2 of the same; levels +/-10_000_000; the const bounds restated at runtime).
- `a_price_gap_enters_the_window_and_leaves_it_after_period_candles` (S and D equal the brute force at every step; the
  gap candle's T gone exactly `period` candles later).
- `a_time_gap_reads_as_contiguous_candles`.
- `no_reading_depends_on_a_later_candle`.
- `a_corrupt_candle_is_refused_and_changes_nothing` (each `Corrupt` kind; whole state compared, ring and sum).
- `the_ring_never_grows_after_new` (length `period` once warm and capacity unchanged after 100,000 folds at periods 1,
  20 and 1_000).
- `the_state_is_a_fixed_size` (`size_of::<Cci>()` at most 96 bytes, ESTIMATE).
- `the_previous_reading_judges_with_the_current_candle_excluded` (as 5.5).
- `the_bench_measures_every_row_at_tiny_sizes` and `a_cost_is_read_by_rank_and_judged_flat_or_not` (cover
  `measure_step`, `measure_period`, `measure_reading_period`, `summarize`, `flat_within`, a zero first p99 failing).

### 6.6 Benches, rows and ledger blocks
- **C-PSRB-CCI-01**: one `fold` plus `reading()` and both comparisons, after 2_000, 20_000 and 200_000 candles, period
  20. p99 flat within 3.0x of the first size, inside a 2 µs budget (target 400 ns, ESTIMATE): O(1) in the history at
  the confirmed period.
- **C-PSRB-CCI-02**: `fold` alone at periods 20, 200 and 2_000 after 20_000 candles: the ring's length never enters the
  fold's cost. Same ceiling and budget. It judges `fold` alone, so no judged row measures `reading()` across periods;
  the line below puts that slope on record (repair 1, check N4).
- The wrapper also prints, after the two rows and without an id, one labelled line: "reading at periods 20, 200, 2000:
  O(period) by design, L-PSRB-CCI-02, not judged flat", with the three p99 figures from `measure_reading_period`. It
  judges nothing and fails nothing; it puts the measured slope on record.
- docs/04 section `## The Commodity Channel Index for the PSRB family (PSRB-CCI; D-8650 to D-8655)` with
  `| Id | Invariant | Proof | |`:
  - PSRB-CCI-I01: the window is the last `period` typical prices, the current one included; the first reading after
    exactly `period` candles at every period up to `u16::MAX` (warm-up and fixture CCI-1 tests).
  - PSRB-CCI-I02: comparisons exact in thirds and n-multiples, `200_000 n dev` against `3 m D`, equal to the published
    definition in exact rationals; `CCI > +200` iff `(TP - SMA) > 3 MD`; a tie is neither; `milli` is the floor, and a
    floored `milli` equal to the level is not a tie (tie, level, floor and parity tests:
    `a_tie_at_plus_two_hundred_is_neither_above_nor_below`, `the_confirmed_level_is_three_mean_deviations`,
    `a_floored_milli_equal_to_the_level_is_not_a_tie`, `milli_is_the_floor_of_the_exact_ratio` and
    `parity_with_the_published_definition`; repair 4, check 4 B3).
  - PSRB-CCI-I03: a flat window has no value and compares false; a period of 1 is always flat, 2 reads +/-66.666...;
    `|CCI| <= 100 n / 3`, so a period of 6 or less never reads above +200 (flat, period and ceiling tests).
  - PSRB-CCI-I04: the ring's S and D equal a brute-force window (brute-force test).
  - PSRB-CCI-I05: every product fits `i128` (const-asserted) and every level outside +/-10,000 points is refused at both
    edges; every refusal changes nothing; the ring never grows (extremes, level-edge, refusal and ring tests).
  - PSRB-CCI-I06: no reading depends on a later candle or on a timestamp.
  - C-PSRB-CCI-01, C-PSRB-CCI-02 rows naming the two bench tests.
- docs/06 section `## The Commodity Channel Index for the PSRB family: limits (PSRB-CCI; D-8650 to D-8655)`, the same
  columns: L-PSRB-CCI-01 the level domain (+/-10,000 points), a chosen domain refused beyond, inside the
  `3 |m| D < 2^127` bound (which would allow about 357,914 points; at 10,000 every product stays below `2^123`);
  L-PSRB-CCI-02 **`reading()` is O(period), not O(1)**: one pass over the ring for D, because the mean deviation is
  taken around the moving mean; `fold` is O(1); at period 20, 20 steps; measured by C-PSRB-CCI-01 at period 20 and
  printed unjudged at 200 and 2_000; an O(log period) order-statistic tree is the named alternative, not built; the
  cost is accepted (repair 1, check N4; D-8653) and reported to the operator; L-PSRB-CCI-03 memory O(period), 16
  bytes a candle, about 1 MiB at `u16::MAX` (ESTIMATE: `VecDeque` may reserve more than asked), allocated once;
  L-PSRB-CCI-04 what was
  measured, the definition UNVERIFIED against TradingView's output, and no real candle until PSRB-STRAT.
- docs/07 section `## PSRB-CCI: the Commodity Channel Index — 2026-11-05 (D-8650 to D-8655)`, one row.
- docs/05 entries (`### D-865n — PSRB-CCI: ... — 2026-11-05`):
  - D-8650: CCI as `indicators::cci`, no vocabulary position; the published form on `hlc3` with the population mean
    deviation from the same SMA, the current candle included; runtime period `u16` 1..=65,535; the first reading after
    `period` candles; carried across sessions.
  - D-8651: units: thirds and n-multiples (T, S, dev, D all integers), the identity `CCI = 200 n dev / (3 D)` and its
    one-line derivation, exact comparisons by cross-multiplication, `milli` the floor for records only, `Level` in
    thousandths, a chosen domain of +/-10,000 points well inside the product bound; bounds const-asserted.
  - D-8652: the flat window (`D == 0`) a named state with no value, every comparison false (TradingView's `na`,
    UNVERIFIED); the ceiling `|CCI| <= 100 n / 3` with its proof, and what follows: a period of 6 or less never reads
    above +200, period 1 always flat, period 2 +/-66.666...; PSRB-STRAT's searched CCI periods for a +/-200 level start
    at 7.
  - D-8653: cost: `fold` O(1); `reading()` O(period), the one non-O(1) reading of the PSRB parts, named in
    L-PSRB-CCI-02; why the window is held whole (the mean moves, every term of D changes); the order-statistic tree as
    the named O(log period) alternative, not built: O(period) is accepted (decided, default accept; repair 1, check
    N4), reported to the operator, who may reverse it; C-PSRB-CCI-02 judges `fold` alone and the unjudged line prints
    `reading()`'s slope.
  - D-8654: refusals and validation order; `unwrap_or(0)` where the ring's `Option` cannot be `None`; timestamps not
    read; no look-ahead; the ring allocated once, in `new` (a clone reserves only its length; the measured path never
    clones).
  - D-8655: C-PSRB-CCI-01 and -02, their sizes, ceiling and 2 µs budget, the unjudged reading line; the helpers copied
    per part.

### 6.7 NEEDS THE MAC
As 5.7, with the C-PSRB-CCI rows and the unjudged reading line printed whole.

---

## 7. Integration, check and landing (deltas from PARTS 7)

1. **Kit step:** 3.6 (PSRB-SEAM-2 and `cloud/psrb-kit-20261105`, the private scan, push the kit branch only).
2. **Start** the two sessions in one operator-typed turn with `briefs/PSRB-WILLR.md` and `briefs/PSRB-CCI.md`.
3. **Intake** as 3.6, then check 1 on the Mac per part (5.7, 6.7), with `XW/anchors2-check.sh` re-run against the
   first-wave tips of that day; a red merge there stops the intake and names the conflict.
4. **Landing** as 3.6: after the first three, `fix/psrb-willr`, then `fix/psrb-cci`; ten C-PSRB ids in gate 14's table.
   PSRB still lands after the Fibonacci families (OI item 7, "PSRB still lands after the Fibonacci families").
5. **No session for PSRB-STRAT starts today**; its inputs are PARTS 6.2's plus these two parts and AD.

---

## 8. UNVERIFIED, open, not proven here

- **CCI's cost, decided and reported (repair 1, check N4; D-8653):** `reading()` stays O(period) (20 steps at the
  confirmed period; the fold O(1)), and no O(log period) order-statistic tree is built (it would cost about 4-6 h
  more, ESTIMATE, and a larger state). The decision goes to the operator in the evening report; the operator may
  reverse it before PSRB-STRAT's first run. Nothing in this file waits on the operator.
- TradingView's built-in Williams %R and CCI scripts were not read in this step. The forms here are the standard
  published ones as this file states them (Williams %R over the highest high and lowest low of the window; CCI on
  `hlc3` with the constant 0.015 and the mean absolute deviation from the same SMA). UNVERIFIED against TradingView's
  help pages and output; PSRB-LAW's charter rows (PARTS 6.1) should add these two pages beside the four it lists.
- The `na` readings (a flat Williams %R window, a CCI window with `D = 0`) follow Pine's rule that a division by zero
  gives `na`, as recalled here; UNVERIFIED.
- The anchors were proved against the refs present on 5 Nov ~09:3x IST (789 refs; 95 hold `f01d7a0e4`) and against the
  first wave's tips of that moment. Check 3 (~11:5x IST) re-ran the ref and window checks over 790 refs, the first
  wave's tips unchanged, with no new conflict (3.5). Refs created later and the first wave's later commits were not
  seen; 7.3 re-runs the proof.
- The rustfmt claim (a module line alone in its own group is not reordered) rests on FNO-LADDER's passed `fmt` gate
  with the same form (section 1), not on a run here.
- Every size, hour, budget and p99 target is an ESTIMATE; no bench ran. The pop counts and the cross-check figures come
  from the node BigInt models `XW/model/wr-cci.mjs` and `cpr-wr6.mjs` (in the kit as `wr-cci.mjs.md` and
  `cpr-wr6.mjs.md`), written in this step apart from the hand computations, not from Rust; the sessions' brute-force
  and parity tests are what prove the Rust.
- Repair 1's figures (the back-pop and removal counts by the white-box method at both counter starts, the floored-level
  gaps, the CCI equality clause, the level bound, the `u128` product bound) come from the node BigInt model
  `specs-1105-repair1-work/repair1.mjs` (output `repair1.out`; in the kit as `repair1.mjs.md`, 3.6), not from Rust.
- Repair 3's figures (the staircase at both generator readings and both counter starts, the CCI ceiling at
  `u16::MAX`, the `k` bound at `u16::MAX`, C2's bounds) come from the node BigInt model
  `specs-1105-repair3-work/repair3.mjs` (output `repair3.out`; in the kit as `repair3.mjs.md`, 3.6), not from Rust.
- Repair 4's evidence (check 4 B1 and B3) is Rust built in a scratch directory by the pinned `rustc` 1.97.1, never in
  P (`specs-1105-repair4-work/run.sh`): section 2's gate 12 readings come from gate 12's own source at the base
  (`.github/gates_bounds.rs` and `.github/invariant_paths.rs` at `1ad6e24ef8e0`, by `git show`, each with a test
  appended and no line of either changed; `g12_tail.rs`, output `gate12.out`), run on a small tree that holds every
  example of section 2, `PARTS.md` 2.4 and both briefs, every form they call no proof, the real docs/04 row
  `C-I-01` (:1972), trend.rs:145-151, AD 2.3's getter and the RSI tip's real `rsi_tests.rs`, and a `benches/ratio.rs`
  that declares the bench fn the real one declares at :500. Its declaration table holds every `fn` of every `.rs` file
  under `crates/`, as CI's does (`source_scan fns` over every tracked `crates/*.rs`, ci.yml:3744-3752 at the base);
  its MODULES and INLINE tables are empty, the narrowest reading: a crate-first path resolves there by its file's own
  path alone, and CI's fuller tables can only add segments a file answers to. The two floored ties of 5.5 and 6.5 and
  the floor's direction come from `r4_fixtures_driver.rs` (output `fixtures.out`). The kit needs neither: 2, 5.5
  and 6.5 state every figure.
- `VecDeque::with_capacity` reserves at least the asked capacity and does not reallocate while the length stays
  within it (the Rust documentation as recalled here; UNVERIFIED in this step); the deque and ring tests pin it. A
  clone reserves only its length: read in the pinned 1.97.1 std source (alloc/src/collections/vec_deque/mod.rs:119-124;
  repair 3, check 3 N3), so the measured path never clones (section 2).

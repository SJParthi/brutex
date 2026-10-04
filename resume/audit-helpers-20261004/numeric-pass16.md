# Numeric pass 16: corporate actions and survivorship in the equity sweep (tag num16, commit 1f4de71)

**Verdict.** 3 new findings: 1 medium (p16num-1, proven by a throwaway test that ran) and 2 low (p16num-2 and p16num-3, both doc findings, not run). The corporate-action gap is disclosed at length: D-0018, D-0694, D-1540, 06-limits §40.4 and §41.3, the D-0694 section, and the constant `runner::audit::CORPORATE_ACTIONS_UNCHECKED` on every ranked stock surface. That disclosure is mostly honest. The defects are:
- a hole in D-1540's measurement, which never sees the one overnight that the stored doors' previous-day conditions do read;
- the vendor's adjustment basis, which is never recorded;
- dividends and the other action kinds, which are never named.

Scope read:
- runner/src/audit.rs:110-345 and its D-1540 tests (:3330-3480);
- cli/src/stored.rs:2150-2200 and :2980-3260 (the daily and exact-minute context loaders);
- cli/src/lib.rs:6255-6280, 2683-2730, and the four `overnight_note` call sites (:3674, :4104, :6274, :6492);
- cli/src/pool.rs:680-705 and research.rs:195-230;
- indicators/src/anchored.rs:1-45 and session.rs:435-470;
- core/src/universe.rs:900-915;
- the following docs: docs/00 (:640-660, :722), docs/02 (grep), docs/05 D-0018 (:395-440), docs/06 §40.4, §41.3, §41.5 and the D-0694 and D-1540 sections (:9335-9460, :14267-14276), docs/08 (grep), docs/04 ST-02 and docs/11 gaps-6.

## Checklist

| question | answer at 1f4de71 | evidence |
|---|---|---|
| (1) Adjusted or unadjusted? Is it stated? | **Not stated anywhere.** No line in docs/00, 02, 06 or 08 records whether Groww's or Zerodha's cash candles come split- or bonus-adjusted. docs/00:650 marks only "whether GDFL's and Zerodha's price series agree ... including corporate-action adjustment" as UNVERIFIED. D-0018 (:431) rejects back-adjusting *in this repo*, which is a different fact. Every report text assumes the bars are unadjusted ("A month holding an unadjusted split still ranks"). | **p16num-2** |
| (2) Is any action detected, refused, flagged or adjusted? | Nothing is detected, refused or adjusted. The statement `CORPORATE_ACTIONS_UNCHECKED` (audit.rs:169) appears on every ranked stock surface. D-1540 adds `largest_overnight_move` (audit.rs:217) to the stored single-instrument doors, with no threshold. The trade simulation is intraday with a forced 15:10 exit (outcome.rs:70), so no trade spans a split, as D-1540 states. The following all see the fake gap: the gap bits 48/49 (session.rs:449-455), the prior-range bits 50/51, `DailyLevels`, the Prev5 ladder, `prev_day_bits` (anchored.rs:31-36) and EMA200 on the signal series. All of this is disclosed, **except** that the measurement skips the first signal day's overnight. | **p16num-1** |
| Rights issue, face-value change, symbol change | Rights issues and face-value changes are not named in `CORPORATE_ACTIONS_UNCHECKED` ("split, bonus or demerger"). On symbol changes: D-0308 records that a stale master resolves renamed symbols to the old row, and research.rs:208 says listing history is NOT VERIFIED. | folded into p16num-3 |
| (3) Survivorship | `FNO_UNDERLYINGS` (universe.rs:914) is a single list derived from 2026-08-01 masters and is applied to all history back to 2020. Two consequences: stocks dropped from F&O are absent, and stocks admitted after 2020 are swept over years when they were not F&O. The membership caveat appears in ST-02 (04-invariants:2271), research_family.rs:4, D-entries ~32775 and ~33734, and research-plan's NOT VERIFIED block (research.rs:207-208). There is **no section in 06-limits and nothing on the ranked surfaces** (pool.rs:688-705, sweep-stored, range-*, top). | gaps-7 row below (PARTIAL), not a new ID |
| (4) Dividends | Not stated anywhere for equities. `dividend` appears in crates only for option carry (pricing.rs:58, :579-581; already p15num-1). Every trade closes by 15:10 on its own session, so no dividend entitlement is missed in P&L. That part is correct. But an ex-date open drop is a fake gap of the dividend's size, and it fires bit 49 the same way a split does. Nothing names it. | **p16num-3** |
| Named split | Neither docs/00 nor docs/08 records any NSE split or bonus. The only equity ticker seen is `RELIANCE.NSE` (docs/00:649), with no action recorded. A concrete split date is therefore **UNVERIFIED**. The proof below is generic (synthetic 2:1). | — |

## New findings

### p16num-1 (medium): D-1540's "largest overnight move" never measures the overnight into the first signal session, though the stored doors' previous-day conditions read exactly that overnight. A split effective on a span's first trading day fires gap_down_day and the whole daily-anchored family, while the report names a different, harmless date.

- `crates/cli/src/lib.rs:6274` (and :3674, :4104, :6492, which are the same shape):
  ```rust
  header.push_str(&stored::overnight_note(&loaded.key, &loaded.bars));
  ```
  `crates/runner/src/audit.rs:217-222`:
  ```rust
  pub fn largest_overnight_move(bars: &[indicators::Candle]) -> Option<OvernightMove> {
      ...
      for pair in bars.windows(2) {
  ```
  `crates/cli/src/stored.rs:3180-3190`:
  ```rust
  let warm_from = previous_month(from)?;
  let daily = load_span(root, vendor, underlying, "1day", warm_from, to)?;
  ```
- **Why it is wrong.**
  - `overnight_note` is given only the signal span (`loaded.bars` / `span.bars`), and `windows(2)` measures only overnights *inside* it.
  - The same doors load a daily reference that starts one month earlier (stored.rs:3180-3190). They also load the prior exact-minute session for GapFib (stored.rs:3253). `AnchoredColumn::build_required` refuses a first signal day that has no prior daily anchor (anchored.rs:1012-1040). So the first signal day *always* compares its open with the prior session's close.
  - Through that anchor, bits 48-51 and 66-68 (`PreviousSession`), `DailyLevels`, `prev_day_bits` and `Prev5` (anchored.rs:31-36) all read the pre-split session. GapFib reads the pre-split exact-minute tail.
  - So for a split effective on a span's first session, the following happen together:
    - bit 49 (`gap_down_day`) fires on the fake gap;
    - pivots, CPR and the Fibonacci levels sit at twice the price;
    - the overnight line names some other session (+1% in the repro) and tells the reader to "check that date against the exchange's corporate-action record".
  - The reader checks the named date, finds nothing, and trusts the result. CLAUDE.md §4 bans this: the safeguard reports a clean answer while the failure sits one bar to its left.
  - The same blind spot covers a split in the last five sessions of the warm-up month, which `Prev5` carries into the first sessions of the span.
  - For a one-month `sweep-stored`, this is 1 of the ~20 overnights the conditions read. Splits take effect on arbitrary sessions, so about 5% of single-month split cases are missed outright.
- **Repro (ran).** I wrote a throwaway runner test in a scratch worktree at 1f4de71 (since removed).
  - Fixture: five eligible daily references with a pre-split close of 20,002.00. The signal span starts on the split day at 10,001.00, and the next session opens at 10,101.00.
  - `largest_overnight_move(&signal)` returned `OvernightMove { day: 20006, prior_close: 1000100, open: 1010100, ppm: 9999 }`, which is +1.00% on the second day.
  - `AnchoredEvaluator::new(Widths::pinned(), Availability::Absent, Thresholds::CLASSICAL, &refs).step(&signal[0])` gave `gap_up(48)=false gap_down(49)=true`.
  - Result: `test ... a_split_on_the_first_signal_day_fires_gap_down_and_escapes_the_overnight_line ... ok`.
- **Minimal fix.** Measure over the overnight the conditions actually read. Seed `largest_overnight_move` with the last accepted prior session: either `DailyContext`'s last eligible daily close before the first signal day, or `ExactMinuteContext`'s prior-session close. That means one extra pair at the front, which stays O(bars). Better, measure over the daily stream (warm-up month included), because that is the stream the anchored families consume. Add a test in which the split sits on the span's first day.

### p16num-2 (low): no document records whether the stored cash-equity prices are vendor-adjusted or unadjusted.

- `docs/00-charter.md:650`:
  ```
  | Whether GDFL's and Zerodha's price series agree in level, including corporate-action adjustment for the cash equities | UNVERIFIED |
  ```
  docs/02-store-format.md and docs/08-vendor-samples.md have no adjustment statement at all. 06-limits D-0694 (:9340) assumes the answer: "A month holding an unadjusted split still ranks".
- **Why it is wrong.**
  - CLAUDE.md §3 rule 1 needs a sourced fact or the word UNVERIFIED. This fact is the one that decides whether the whole D-0018/D-0694/D-1540 apparatus is needed. No line says "Groww (Zerodha) historical candles: adjustment basis UNVERIFIED".
  - Whether either broker's candle API back-adjusts after an action is not recorded.
  - If one does, then months pulled before and after an action date sit in the store on different bases. The append-only store keeps both, and nothing records the pull date's basis. The fake gap would then fall at the boundary between pull dates, not on the ex-date.
  - That case is not claimed as fact here, because it depends on the very fact that is missing.
- **Repro.** Not run: this is a documentation grep (`grep -niE "unadjusted|adjusted for|back.adjust" docs/0[0268]*`).
- **Minimal fix.** Add a charter row per feed: "cash 1min/1day candles adjusted for splits/bonuses: UNVERIFIED". Add a sentence to docs/02 saying that the store keeps vendor bytes as served and records no adjustment basis.

### p16num-3 (low): dividends, rights issues and face-value changes are nowhere named as unhandled. The statement on every stock report names only "split, bonus or demerger".

- `crates/runner/src/audit.rs:169-173`:
  ```rust
  pub const CORPORATE_ACTIONS_UNCHECKED: &str = "  CORPORATE ACTIONS ARE UNCHECKED (D-0018, D-0694). No split, bonus or\n  \
       demerger detection has run over these bars, ...
  ```
- **Why it is wrong.**
  - An ex-dividend open (especially a special dividend) and an ex-rights open are each an overnight drop with no market cause. Each fires bit 49 and moves the daily-anchored levels, exactly as a split does but smaller.
  - Neither the constant, nor docs/06, nor docs/00 says that dividends are ignored.
  - Ignoring dividends is *correct* for P&L, because every trade is flat by 15:10 (outcome.rs:70) and can never hold through an ex-date. That reasoning is nowhere written down, so a reader cannot tell whether the omission was decided or missed.
  - A face-value split is a split, so it is covered in substance but not by name.
- **Repro.** Not run: `grep -rniE dividend crates docs/06-limits.md docs/00-charter.md` finds only option-carry hits.
- **Minimal fix.** Widen the sentence to "split, bonus, rights, demerger or dividend". Add one 06-limits paragraph covering three points:
  - dividends never enter P&L, because no position is held through an ex-date;
  - ex-date gaps still fire the gap and previous-day conditions;
  - no detector exists for any of these actions.

## Verification

| ID | verdict | evidence |
|---|---|---|
| gaps-6 (corporate actions, audit-20261003) | PARTIAL | D-1540 adds `largest_overnight_move` (runner/src/audit.rs:217) on six stored doors (cli lib.rs:3674, :4104, :6274, :6492), with the test `every_stored_stock_report_names_its_largest_overnight_move` (audited_stored_tests.rs:2410). The statement is on every ranked stock surface (D-0694 and the AF-19 correction). It misses the span-leading overnight that the anchored conditions read: see p16num-1. No refusing detector exists (honestly UNVERIFIED). |
| gaps-7 (survivorship, numeric-pass4) | PARTIAL | It is now declared in ST-02 (docs/04-invariants.md:2271, "membership is not point-in-time history"), runner/src/research_family.rs:4, cli/src/research.rs:207-208 ("A current member list is NOT the historical F&O universe from 2020") and the D-entries (docs/05 ~32775, ~33734). Still missing: a docs/06-limits section; any membership statement on the ranked surfaces (pool.rs:688-705 opening; sweep-stored, range-all/rung, top); and any mention of the forward half, where stocks admitted after 2020 are swept over their pre-admission years. universe.rs:914 is still a single 2026-08-01 snapshot. |
| p15num-1 (option carry = 0) | not re-checked here | out of theme; it was seen only as the sole `dividend` hit in crates (pull/src/pricing.rs:579) |

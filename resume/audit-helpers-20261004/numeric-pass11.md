# numeric pass 11 (num11): are the condition vocabulary's bits computed correctly?

Tree: /home/claude/wt/zero3 at 1f4de71. Scope: crates/vocab (TABLE, 370 rows), docs/03-vocabulary.md, crates/indicators (every evaluator family). Audit by reading source. One throwaway test was run, in a scratch worktree that has since been removed.

**Counts: 4 new findings (1 medium, 3 low). No high. Five prior items in scope were re-verified: 0 fixed, 5 not fixed.**

## Family-by-family verdict

| Family (bits) | Formula, and the source named in code | Warm-up | Session, holiday and month boundaries | Look-ahead | Rounding | Verdict |
|---|---|---|---|---|---|---|
| EMA20/200 (0-5) | SMA seed, then `2/(n+1)` recursion. TA-Lib seed (trend.rs:197, D-1542) | emitted only at `folded >= period`; column gate `warmed_up` | continuous across sessions (standard); spliced across a withheld day (**p11num-1**) | emits against the EMA through N-1, then folds N | floored (known ind1-2) | correct |
| ATR10 / SuperTrend (64-65) | Wilder TR and `1/n`, mean seed; hl2 ± m·ATR, ratcheted stop | `atr.warm()` | first TR of a day includes the overnight gap (standard) | emit, then fold | div_euclid | correct (flip variant known, hunt-indicators-7) |
| Pivots, CPR, R1-R5 (6-18, 54-55, 60-63, 74-85, 178-189, 274-275) | classic floor pivots (daily.rs:16-20, D-0078). Checked: R3 = H + 2(P−L), S3 = L − 2(H−P) | needs `yesterday` | anchored path uses sealed 1day rows `day < signal_day`; Muhurat days skipped (charter §3) | none | i128, div_euclid, narrowed once | correct. Doc stale (**p11num-3**) |
| Previous-day / 5-day / current-day / gap Fibonacci (19-29, 69-71, 106-142) | per-mille rungs; gap sheet (docs/09 §4) | per ladder | gap X1 from the full exact-minute stream, not the withheld signal | curday and gap emit before fold | sub-paisa, single test | correct |
| Session shape, prior-3, day position, clock, day type, gap mid (30-51, 66-68) | UNVERIFIED thresholds, declared | per day | `previous` set at rollover / from the daily row | fold then emit by design (description) | cross-multiplied | correct. docs/03 §3 contradicts the fold order (**p11num-4**) |
| VWAP and sigma bands (52-53, 143-152, 190-197) | hlc3 VWAP, exact floor sigma | sigma needs n ≥ 2 | resets each IST day | fold then emit by design | floored (Z1-slice09-F1 open) | correct except known items |
| ORB (86-105) | IST clock windows | per window close | per day | window closed before emit | exact | correct (coarse-rung over-collect documented, D-0527) |
| Patterns (153-177, 198-234) | "classical", UNVERIFIED | depth reset per day | per day | push, then emit (completed candles) | cross-multiplied | 211 wrong region (**p11num-2**); 208/209 known (Z1-slice08-F1) |
| Structure, swings (56-59, 72-73, 278-279) | 5-bar fractal, latch | needs both swings | carried across sessions; spliced across a withheld day (**p11num-1**) | pivot N-2 published at N | exact | correct |
| Crossings and ordinals (280-364) | derived from this bar's mask and the last definite side | per session | cleared at rollover | none | n/a | correct |
| Weekday (365-369) | epoch Thursday, rem_euclid (lib.rs:172-186) | none | weekend sessions set nothing (documented) | none | n/a | correct |

**INDIAVIX.** No identifier containing "vix" appears in crates/vocab or crates/indicators. No condition reads VIX. This is clean against CLAUDE.md §1.

**Unreachable or always-set bits.** None found among the 328 live positions. 81 `near` + 247 `plain` = 328, plus 3 retired and 39 void = 370, which matches docs/03 §8. 52/53 duplicate 143/144 by design; this is documented at vwap.rs:513-517. 225 is set only by a zero-range bar, so it is reachable. 63 and 71 are both computed, contrary to their doc (p11num-3).

**RSI, MACD, Bollinger.** None is in the vocabulary, so there is nothing to check. The VWAP sigma bands are the only band family.

## New findings

### p11num-1 (medium): withholding a holed day removes it from indicator history, which silently changes trend bits on later, swept days

- `crates/cli/src/lib.rs:3478-3483`:
  `let (kept, withheld) = crate::minute_gaps::withhold(&loaded.bars, &holed_days); loaded.bars = kept;`
  The column is then built from that spliced slice (`stored_anchored_column(&loaded.bars, ...)`). The same pattern is at lib.rs:4032, 6955 and 16288, and at pool.rs:878.
- **Why it is wrong.** `crates/cli/src/minute_gaps.rs:47-56` claims the day is "removed from the sample", and that "withholding declines to answer it". The code does more than that. The day also leaves the evaluator's input, so every family that carries state across sessions is computed on day D-1 joined directly to day D+1, as if the market had not traded on D:
  - EMA20 and EMA200 (0-5)
  - ATR and SuperTrend (64-65); the first true range of D+1 spans two days
  - swings, BOS and CHoCH (56-59, 72-73) and structure-in-force (278-279)

  Swept days after D therefore carry different masks from a run in which D had been folded. No banner says so. The run identity binds the withheld days, but nothing states that later bits were changed. On coarse rungs the effect lasts longer: EMA200 on 60min spans about 28 sessions.
- **Repro (ran).** Scratch test `indicators/tests/num11_withheld_splice.rs`:
  - Setup: 14 sessions of 375 bars on a deterministic random walk. Run A folds the holed day with its one interior minute missing. Run B drops the whole day, as `withhold` does.
  - Result: of the 1,125 bars after the held day, 40 differ on trend bits. Per bit: 0:6, 1:6, 2:16, 3:16, 4:28, 5:28, 64:15, 65:15.
- **Minimal fix.** Fold the holed day through the evaluator and exclude only its rows from the column. Column already carries `source` indices and `clear_before`, so this can be done by dropping rows whose source day is withheld. Alternatively, state in the banner and in minute_gaps.rs that later trend bits are computed on a spliced series. This needs a D-entry because it changes results.

### p11num-2 (low): `pat_in_neck` (211) tests the wrong region; a classical in-neck sets `pat_thrusting` (212)

- `crates/indicators/src/pattern.rs:625-633`:
  `bar1.bearish() && bar0.bullish() && bar0.open < bar1.low && bar0.close > bar1.low && bar0.close <= bar1.close`
- **Why it is wrong.**
  - Nison (Japanese Candlestick Charting Techniques) and TA-Lib `CDLINNECK` define in-neck as the white candle closing at, or slightly *into*, the prior black real body: `close >= prior close` and `close <= prior close + near`.
  - The code accepts only `prior low < close <= prior close`, which is the prior lower shadow. That region is neither in-neck nor on-neck. It meets the classical definition only at exact equality.
  - A close one tick above the prior close (the textbook in-neck) is filed under 212 thrusting (`close > bar1.close && close < mid`).
  - The fixture at pattern.rs:2506-2511 pins the wrong shape: bar0 closes at 1005 against a prior close of 1010.
- **Repro.** Not run; this follows directly from the predicate. bar1 = (1100, 1110, 1000, 1010) and bar0 = (990, 1015, 985, 1012) set 212 and not 211.
- **Minimal fix.** In-neck becomes `close >= bar1.close && close <= bar1.close + near`, with `near` added to the UNVERIFIED `Thresholds`. Thrusting starts above that band. This needs a D-entry because it changes results.

### p11num-3 (low, doc-false): `daily::bits` and pattern.rs still say 63 and 71 are not computed

- `crates/indicators/src/daily.rs:534-540`:
  `# What it deliberately does not set ... **63 narrow_cpr_day** — no tracked document says how narrow is narrow ... **71 near_fib_424** — an orphan`
- `crates/indicators/src/pattern.rs:27`:
  `Contrast position 63 narrow_cpr_day, which this crate refuses to compute at all`
- **Why it is wrong.**
  - `bits` calls `bits_with(..., CprWidth::CLASSICAL)`, which sets 63, 274 or 275 (daily.rs:612-623).
  - 71 is rung 4.236 of `fib::PREV_DAY_UP` (fib.rs:107) and is set by `prev_day_bits`.
- **Repro.** Not run; read from source.
- **Minimal fix.** Delete the 63 and 71 bullets and the pattern.rs:27 sentence, or reword them to point at `CprWidth` (UNVERIFIED cuts) and at fib.rs.

### p11num-4 (low, doc-false): docs/03-vocabulary.md §3 and §4 describe behaviour the code does not have

- **docs/03:51.** §3 says state "updates **after** the bar is emitted, never before". vwap.rs:480-492 (`self.fold(bar)?; Ok(self.bits(...))`) and session.rs:260-305 fold before they emit, deliberately, under the D-0080 description rule.
- **docs/03:63.** §4 says that on a daily timeframe, bits 44-47 and 52-53 "are cleared". No code clears them, and 1day is not a swept rung (`cli::stored::rung` doc at stored.rs:1770-1778, `EVERY_RUNG`).
- **docs/03:67.** "bits 52-53 permanently abstain on the two engine instruments." Since D-0506 and D-0507 the engine sweeps 208 equities with `Availability::Present` (cli/src/stored.rs:2065). On equities the whole 20-position VWAP family is live.
- **Repro.** Not run; read from source.
- **Minimal fix.**
  - Restate §3 with the anchor/description split from D-0080.
  - Drop the daily-clearing sentence, or say that 1day is never swept.
  - Say VWAP is Absent on indices, futures and options and Present on equities.

## Verification of prior items in this scope (state at 1f4de71)

| ID | Verdict | Evidence |
|---|---|---|
| Z1-slice08-F1 (208/209 same-colour) | NOT FIXED | pattern.rs:617-622 still `bar1.bullish() && bar0.bullish()` / `bearish && bearish` |
| Z1-slice09-F1 (VWAP floored compare) | NOT FIXED | vwap.rs:309 `self.pv.div_euclid(3 * self.v)`; vwap.rs:518-525 compares close against it |
| Z1-slice08-F3 (PriceNotPositive doc says four fields) | NOT FIXED | lib.rs:384-387 doc unchanged; lib.rs:1778 and evaluator.rs:927 test `low <= 0` only |
| Z1-slice08-F4 (gap source variant name) | NOT FIXED | anchored.rs:274 and anchored.rs:450 still `SignalSeriesLastThreeIntradayBars` |
| ind1-1 / ind1-2 (floored levels compared strictly) | NOT FIXED | trend.rs Ema::value `div_euclid(SCALE)` compared via `side`; vwap.rs:309 |

Tally: 0 FIXED, 0 PARTIAL, 5 NOT FIXED, 0 WRONG FIX.

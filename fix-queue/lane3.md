# Cloud fix lane 3

Handoff only. Branch `fix-queue` is never merged. Source: the batch-2 audit queue (state/c4/wave2-after-merge.json on the local drive), 2 Oct 2026. Base every fix on origin/main.

**Done means:** a test that fails before the fix and passes after; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` and the CI gates green; for a cost finding, the operation is O(1) or the honest bound is named in docs/06-limits.md. One branch and one PR per item, named `fix/cloud-<id>`.

## ET-indicators-0 · high bug · indicators

**Where:** `crates/indicators/src/vwap.rs:333`

**Finding:** sigma = sqrt(floor(p2v/v) - floor(pv/v)^2). Flooring the mean before squaring adds up to about 2*M*frac(M) to the variance on the x9 scale (M = 3 x price). Band positions 145-152 and 190-197 are then decided on a sigma inflated 8x to 5000x on realistic inputs. F-9A880B records it as OPEN but says it 'cannot bite any run this engine sweeps'. That is false since equities sweep with Availability::Present (cli/src/stored.rs:1921-1924). The existing oracle test uses a fixture whose mean is exact (prices 200 and 300, volume 1), so it cannot see this. O(1) fix: with q = pv div v and r = pv mod v, var = (p2v - q*(pv + r) - floor(r^2/v)) / v.

**Evidence:**
  - The existing oracle-style tests all use fixtures whose mean divides exactly (r = 0), so none of them can see this: vwap.rs:710-719 (200/300 at volume 1 gives sigma 50) and vwap.rs:980-985 (1,000,000/1,000,200 at volume 100 each gives 100). Near bits are also affected, because sigma is the scale argument to `set_near`: position 145 (vwap.rs:468) and the near_up/near_down positions in the band loop.
  - crates/indicators/src/vwap.rs:333 floors the mean first: `mean = pv.div_euclid(v)`. :334 floors the mean square: `mean_sq = p2v.div_euclid(v)`. :338-339 then compute `variance = mean_sq - mean*mean`. Sigma is the scale for position 145 and sets the band levels for 146-152 and 190-197 (:463 near, :482-503 band loop, :537-540 band_levels). crates/cli/src/stored.rs:1921-1925 maps `Kind::Equity => Availability::Present`.

**Expected fix and test:** None

## ET-indicators-12 · high bug · indicators

**Where:** `crates/indicators/src/vwap.rs:318`

**Finding:** vwap::Vwap + isqrt_i128 (52-53, 143-152, 190-197) is partial: Accumulators are checked and fixed-size. But sigma floors the mean BEFORE squaring it (vwap.rs:333-339). attack01 broke: exact sigma 0/3/38 paisa, module sigma 912/47/324. attack01b: close 10,020 sets inside_band 152/192/197 where the exact bands put it above 146/148/193. Recorded as F-9A880B OPEN, but that row claims it 'cannot bite any run this engine sweeps', and cli/src/stored.rs:1921-1924 binds Present for Kind::Equity.

**Evidence:**
  - crates/indicators/src/vwap.rs:333-339 computes `mean = pv.div_euclid(v)`, then `mean_sq = p2v.div_euclid(v)`, then `mean.checked_mul(mean)`, and subtracts. The doc comment at :315-316 says the value is "divided down once, so no per-bar rounding accumulates".
  - *The defect is in the code.** In crates/indicators/src/vwap.rs:333-339, `sigma()` computes `mean = pv.div_euclid(v)`, which floors the mean on the x3 scale. The `max(0)` at :343 only guards the other direction. **(c) Synthetic random-walk equity sessions.** 20 sessions x 375 bars, about 0.03% per-minute steps, volume 500-20,000.

**Expected fix and test:** None

## ET-indicators-1 · medium bug · indicators

**Where:** `crates/indicators/src/anchored.rs:108`

**Finding:** An all-zero (or any low <= 0) daily bar is admitted as an ELIGIBLE previous-day anchor. It installs the pivot/CPR ladder, both previous-day Fibonacci ladders, Prev5 and PreviousSession from 0/0/0. Evaluator::stepped refuses the same record (PriceNotPositive, evaluator.rs ~:297). The Corrupt::PriceNotPositive doc measures this contamination at 26.7% of one condition's P&L on the ordinary path. The live caller is cli/src/stored.rs:2776, which calls DailyReference::new on each stored 1day bar. It is latent: that doc reports a store scan with zero all-zero records, which I did not re-measure. Fix: call bar.check_evaluable().

**Evidence:**
  - crates/indicators/src/anchored.rs:108: `DailyReference::new` calls `bar.check()`, not `check_evaluable()`. crates/indicators/src/lib.rs:1681-1706: `check` rejects inverted, overflowing, outside-range and negative-volume bars. lib.rs:1749-1762: `check_evaluable` adds `low <= 0 -> PriceNotPositive`. crates/indicators/src/daily.rs:181-250: `from_previous_session(0,0,0)` has no sign check, so it returns Ok levels.
  - crates/indicators/src/anchored.rs:108. `Candle::check` (crates/indicators/src/lib.rs:1681-1710) has only relative clauses (HighBelowLow, RangeOverflows, PriceOutsideRange) plus NegativeVolume. `check_evaluable` (lib.rs:1749-1763) adds `low <= 0 => PriceNotPositive`. Its own doc (lib.rs:1741-1743) says ordered nonpositive prices "cannot enter any production indicator fold".

**Expected fix and test:** None

## W3-indicators1-0 · medium cost · indicators

**Where:** `crates/indicators/src/anchored.rs:383`

**Finding:** AnchoredEvaluator::step_with_warmth -> AnchoredEvaluator::advance_before (anchored.rs:440) (crates/indicators/src/anchored.rs:383): per one signal bar offered to the stored-daily anchored evaluator (every bar of AnchoredColumn::build / build_required, the stored production path used by cli/src/lib.rs:2448, candidate_universe.rs:1668, step3_orchestrator.r ..., the cost is amortised O(1) only while every bar is accepted; O(D) per refused bar; O(S*D) worst case per run; it grows with daily references between the committed cursor and the offered bar's IST day. This is the whole daily series D before the first accepted bar. The walk is redone for every consecutive refused bar, so the worst case per run is O(S_refused x D). Auditor verdict: false-o1-claim. Documented: Documented as O(1) in two places, and wrongly. anchored.rs:17-20: "One monotonic cursor advances through daily bars, and each daily bar is consumed at most once ... total work is O(signal + daily)".

**Evidence:**
  - anchored.rs:381-387 advances the cursor on a copy, then runs `step_known(bar)?`. `advance_before` (anchored.rs:441-462) walks `while let Some(reference) = self.references.get(self.cursor) { if reference.day >= signal_day { break; } ...` from the committed cursor. `Column::build_from` (column.rs:559-581) keeps going after `Err(corrupt)`.
  - At crates/indicators/src/anchored.rs:381-387, `step_with_warmth` does `let mut next = *self; ... `advance_before` (anchored.rs:440-462) is `while let Some(reference) = self.references.get(self.cursor) { if reference.day >= signal_day { break; } self.cursor = self.cursor.saturating_add(1); ...

**Expected fix and test:** None

## W3-indicators1-1 · medium bug · indicators

**Where:** `crates/indicators/src/anchored.rs:383`

**Finding:** AnchoredEvaluator::step_with_warmth / advance_before (crates/indicators/src/anchored.rs:383): A burst of refused signal bars (corrupt, zero-priced or duplicated) before the first accepted bar, or spanning many days after the last one. Each refused bar re-walks every daily reference since the committed cursor. Code path: I ran this in a throwaway worktree at 29025433 (release build, under concurrent load). Per refused bar: 110 ns with 10 references and 277,355 ns with 40,000. census.consumed stayed 0 after 2,000 refusals. The only refusal test, a_refused_signal_consumes_neither_reference_nor_census (anchored.rs:1785), uses a same-day TimestampNotIncreasing bar whose walk is empty.

**Evidence:**
  - MECHANISM (anchored.rs:381-387): The next bar therefore repeats the whole while-loop in advance_before (anchored.rs:440-462): one `install_external_daily_reference` per reference whose day is below signal_day. Column::build_from (column.rs:558-582) keeps folding after an `Err(corrupt)` arm, so the re-walk happens once per refused bar.
  - *The code path.** In `crates/indicators/src/anchored.rs:381-387`, `step_with_warmth` does these steps in order: The loop at lines 441-462 (`while let Some(reference) = self.references.get(self.cursor) { if reference.day >= signal_day { break; } ...

**Expected fix and test:** None

## UC-3 · medium bug · indicators

**Where:** `crates/indicators/src/anchored.rs:108`

**Finding:** anchored::DailyReference / AnchoredEvaluator / AnchoredColumn is partial: DailyReference::new validates with bar.check(), not check_evaluable() (anchored.rs:108). attack02 broke: an all-zero Eligible daily bar is admitted, has_yesterday=true, and 13 pivot-family bits fire on the next day. Evaluator::step refuses the same record as PriceNotPositive.

**Evidence:**
  - crates/indicators/src/anchored.rs:108: `DailyReference::new` calls `bar.check()`, not `check_evaluable()`. crates/indicators/src/lib.rs:1681-1702: `check` accepts an all-zero candle. crates/indicators/src/lib.rs:1749-1762: only `check_evaluable` refuses `low <= 0` with `PriceNotPositive`.
  - crates/indicators/src/anchored.rs:108: DailyReference::new validates with `bar.check()`. crates/indicators/src/lib.rs:1681-1705 shows that check() refuses only these: HighBelowLow, RangeOverflows, PriceOutsideRange and NegativeVolume. The positive-price refusal lives only in `check_evaluable` (lib.rs:1749-1763), which returns PriceNotPositive when low <= 0.

**Expected fix and test:** None

## GAP12-5 · medium bug · 

**Where:** `crates/indicators/src/anchored.rs:815-836 (clamp); doc claims 651-656 and 1537-1541`

**Finding:** The overlay's stub-bucket clamp maps a session truncated inside its last bucket onto an earlier minute at 30/60min (and 10min, 1/2min), while 3/5/15min refuse the same data 

**Evidence:**
  - i
  - n
  - d

**Expected fix and test:** Fix: the clamp target must be the day's SESSION CLOSE minute, never the day's last stored minute. Map to ts(signal_day, latest_session_minute_of_day) and refuse MissingClosingMinute{expected = that minute} when it is absent. Better still, have the caller pass the per-day close from pull::calendar/venue (cli already has kind_of), which also fixes the next finding. Update the docs at 615-666. Test (fails today because 30/60 return Ok): `a_truncated_final_stub_bucket_refuses_on_every_stub_rung`. On hourly_fixture-style days, day 2 either loses 15:29 or stops at 15:16, with the signal folded from those minutes. Assert Err(MissingClosingMinute{ expected_ts_micros: 15:29 of day 2 }) at 10/30/60min, matching the 15min answer.

## ET-indicators-2 · medium bug · indicators

**Where:** `crates/indicators/src/gap.rs:97`

**Finding:** The spec (docs/09-design-sources.md:394-395) defines X1 and X2 as the previous day's LAST and today's FIRST 3-minute candle. The code folds 3 bars. With a missing minute, a bar outside the candle enters X1 or X2, and on rungs of 3 minutes or more the candle becomes 3 rung bars (9-45 minutes). The store is about 1.32% missing minutes (column.rs:898 comment: 618,296 against about 626,625). The ORB module chose clock spans for exactly this reason. The exact-minute overlay path uses the same GapFib, and its cadence check allows holes. O(1) fix: key the 3-slot ring by minute-of-day.

**Evidence:**
  - crates/indicators/src/gap.rs:97 sets CANDLE_MINUTES = 3. fold() (gap.rs:343-366) counts arrivals. The tail is a 3-slot ring advanced `% CANDLE_MINUTES` (344-347). The opening candle closes when today_bars reaches 3 (349-364), and establish() runs then. The spec in docs/09-design-sources.md:394-395 names the "last 3-minute candle" and "first 3-minute candle".
  - gap.rs:97 sets `CANDLE_MINUTES = 3`. gap.rs:349-364 counts `today_bars` up to 3 and calls `establish()` on the third bar. gap.rs:344-347 makes the tail a 3-slot ring of the last 3 bars. gap.rs:304-311 folds that ring into `yesterday`. The spec at docs/09-design-sources.md:394-395 says the previous day's last 3-minute candle and today's first 3-minute candle.

**Expected fix and test:** None

## ET-indicators-11 · medium bug · indicators

**Where:** `crates/indicators/src/gap.rs:174`

**Finding:** gap::GapFib (132-142) is partial: Implemented and live, but the 3-minute candle is 3 BARS (CANDLE_MINUTES=3 at gap.rs:97, today_bars counter at :349). The spec asks for 3 clock minutes (docs/09-design-sources.md:394-395). attack07 broke: with 09:16 missing, the 09:18 bar entered X2 (x2=1,090,000 against 1,051,000).

**Evidence:**
  - CODE: gap.rs:97 sets `CANDLE_MINUTES = 3`. `fold` (gap.rs:343-365) counts bars through `today_bars` and never reads `ts_micros`. The tail (gap.rs:178, :344-347) is a 3-slot ring holding the last three BARS. `close_the_session` (gap.rs:300-318) reduces that ring. docs/09-design-sources.md:394-395 names "last/first 3-minute candle", and :416-421 ties candles to the fold grid anchored at IST midnight.
  - CODE: gap.rs:97 sets `CANDLE_MINUTES = 3`. `fold` (gap.rs:343-367) counts bars: `today_bars < CANDLE_MINUTES`, then `establish` runs on the third bar. The tail is a 3-slot ring of the last three bars (gap.rs:344-347), and `close_the_session` reduces it (gap.rs:300-319).

**Expected fix and test:** None

## GAP12-7 · low bug · 

**Where:** `crates/indicators/src/anchored.rs:788-803 (latest_session_micros over the whole slice), 819`

**Finding:** The overlay's clamp is decided by minutes on LATER days: one off-session minute anywhere in the slice flips every earlier day's final stub bucket from mapped to refused 

**Evidence:**
  - i
  - n
  - d

**Expected fix and test:** Fix: the same change as the truncated-bucket finding. Take each day's close from a caller-supplied per-day session close (cli: kind_of/venue), or at minimum compute the clamp threshold causally from days <= signal_day. Test: `a_later_days_off_session_minute_cannot_change_an_earlier_mapping` overlays days D0..D1 (Ok), then the same plus D2 with a 16:20 minute. Assert D0/D1 still map, with byte-identical GapFib/ORB masks for their rows. It fails today with Err(source 6).

## GAP4-47 · low bug · 

**Where:** `crates/indicators/src/anchored.rs:788, 802, 819-837; doc 641-657; test 1732-1771`

**Finding:** Exact-minute overlay clamp reads the whole minute slice: a later-day minute flips an earlier day's stub bucket from mapped to refused 

**Evidence:**
  - C
  - o
  - d

**Expected fix and test:** Fix: make clause 1 causal. Preferred: the caller supplies the per-day session close. indicators may not depend on pull, so cli passes `pull::calendar::kind_of(day)` final window `.to` as a parameter or closure. Alternative: keep it data-derived, but use a running max over minutes with `ts <= demanded`, maintained by a second monotone cursor (amortised O(1)), and refuse the clamp when no earlier session established a close. Update doc :641-657.

Test (fails before, passes after):
- Promote the probe. With a later-day 16:20 or 15:40 minute appended, every rung's overlay stays Ok, and bits, known and sources are byte-identical to the prefix run. Today 2/10/30/60min return Err(MissingClosingMinute { source: per_day-1, .. }).
- Rewrite the :1732 test to use a 60-minute rung and a later-day 16:2

## AC-whp-tb-4 · low bug · indicators

**Where:** `crates/indicators/src/evaluator.rs:3633`

**Finding:** The module's only import is `use super::Side;`, an enum. `an_unknown_side_does_not_erase_the_side_before_it` and `the_first_definite_side_of_a_session_is_recorded_and_not_reported` each rebuild the rule in a local loop ('The rule, applied exactly as `crossings_of` applies it') and assert on that loop's count. No production function is called. The doc claims 'it fails the moment the code goes back to reading the previous bar', but reverting `crossings_of` cannot affect either test. Real coverage exists in crossing_known_readiness.rs, so these two tests are dead weight that claim a guard they do not provide.

**Evidence:**
  - I could not refute this finding. It holds as stated, and the code is the same on main and in C2. Evidence (fix/c2-final:crates/indicators/src/evaluator.rs): - :3633-3635 is `mod band_crossing_tests { use super::Side;`. The module imports nothing else. - :3663 says "it fails the moment the code goes back to reading the previous bar." :3676 says "The rule, applied exactly as `crossings_of` applies it." Each is followed by a local loop, `let mut last = Side::Unknown; ... for now in walk { if ...
  - At fix/c2-final:crates/indicators/src/evaluator.rs:3633-3634 the module is `mod band_crossing_tests { use super::Side;`.

**Expected fix and test:** None

## GAP12-10 · low doc-false · 

**Where:** `crates/indicators/src/evaluator.rs:149-155, 169`

**Finding:** evaluator.rs says five of the six Muhurats never reach disk because the pull hardcodes a 15:30 close; dhan's store holds 43 bars of the 2021-11-04 Muhurat and the pull reads venue tables 

**Evidence:**
  - e
  - v
  - a

**Expected fix and test:** Fix: rewrite 149-160 to say that some vendors do land Muhurat-day bars inside the window (dhan 2021-11-04, 14:47-15:29), that the list is what keeps them out of anchors, and that fetch::land uses venue hours. Annotate 18_935 with the measured on-disk shape. Test: an existing-style case `a_muhurat_session_inside_the_pull_window_does_not_become_yesterday` feeds 43 bars at 14:47-15:29 on 18_935 between two regular sessions and asserts the next day's daily/prev-day bits equal the fixture without that day.

## ET-indicators-7 · low bug · indicators

**Where:** `crates/indicators/src/lib.rs:4`

**Finding:** These are stale or contradictory statements. lib.rs claims 11 and then 52 of 232 positions are computable, while 328 of 328 are emitted. lib.rs says PastPrefix enforces no-look-ahead, but it has no production caller. limits §51 says isqrt never runs on swept data, but equities run it. vocabulary.md:573 calls 56-59 'break events ... handful of bars', while :589 and the code make them states that fire every bar (30 of 30 in attack11, F-A9DB2D). tests/module_doc_counts.rs does not check the lib.rs header.

**Evidence:**
  - crates/indicators/src/lib.rs:4 says "eleven of the vocabulary's 232 live positions" and :21 says "52 of the vocabulary'''s 232". The header table at :12-19 does not agree with either number: its rows add up to 205, and it leaves out trend, gap, anchored, the crossings and the weekday rows.
  - crates/indicators/src/lib.rs:4 says eleven of 232 live positions are computable. lib.rs:21 says 52 of 232. The table's own Count column (lib.rs:12-19) sums to 205. Not in the claim: evaluator.rs:1358-1359 says "238 positions today", which is also stale. modules() at crates/indicators/tests/module_doc_counts.rs:52-72 lists only daily, fib, orb, pattern, session and vwap, and :141 asserts checked == 6.

**Expected fix and test:** None

## ET-indicators-10 · low cost · indicators

**Where:** `crates/indicators/src/lib.rs:24`

**Finding:** Claimed: '[PastPrefix] enforces it by removing the future from the borrow'; per-bar cost 'measured ... 24.4 -> 22.4 ns' Actual: PastPrefix has no production caller (grep). No-look-ahead holds because Column::build streams bars (confirmed by attack04). The 24.4 ns figure is an old CurDayFib-only measurement. A whole bar costs ~670-1160 ns in my scratch run and 404,052-496,491 ps in C-I-05 (limits D-0690 section).

**Evidence:**
  - `git grep -n PastPrefix -- ':!*.md'` returns only crates/indicators/src/lib.rs:26 (the header claim), :174 and :178 (the declaration, before the first #[cfg(test)] at :646), and :1159, :1165, :1168, :1424 and :1432, which are all inside test modules. So the header sentence at lib.rs:25-28, "[PastPrefix] enforces it by removing the future from the borrow", names a mechanism that nothing uses.
  - the doc sentence at lib.rs:26 the definition at lib.rs:174 and :178 test uses at lib.rs:1159, 1165, 1168, 1424 and 1432, which are in the test module. So the sentence at lib.rs:25-28 ("[`PastPrefix`] enforces it by removing the future from the borrow") is false as stated. lib.rs:39-43 describes one family's work: "one session comparison, eleven cross-multiplied rung tests, two extreme updates".

**Expected fix and test:** None

## UC-1 · low bug · indicators

**Where:** `crates/indicators/src/lib.rs:174`

**Finding:** PastPrefix is test-only: grep shows no production caller. lib.rs:24-28 still says 'PastPrefix enforces it'. CLAUDE.md §3 rule 7 already says the real mechanism is the streaming fold.

**Evidence:**
  - The type is declared at lines 174-207 and a doc comment names it at line 26. (2) The module header at lib.rs:24-28 still says "[`PastPrefix`] enforces it by removing the future from the borrow, so a look-ahead read is a compile error". docs/04-invariants.md:103 has the same stale wording: it credits V-02 to an "(index-guarded accessor)".
  - `git grep -n PastPrefix -- ':!web'` finds the type only at crates/indicators/src/lib.rs:174-208 (definition), in the module doc at lib.rs:26, and in two tests at lib.rs:1159-1168 and 1424-1432. Both tests sit inside the `#[cfg(test)] mod tests` block, which runs from lib.rs:646-657 to 1461. `Column::build_from` walks `for (index, bar) in bars.iter().enumerate()` (crates/indicators/src/column.rs:558).

**Expected fix and test:** None

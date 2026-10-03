# pass1 / pul: crates/pull (fold, session/calendar, vendor price parsing, decoders) at 331b05c

**Verdict.** One new finding, low severity. The production path holds. Every production caller of `pull::fold` passes a `Timeframe::KNOWN` width (ingest.rs:2359 and :2480, cli fold_audit.rs:364). Every one of those widths divides 86,400, so the open-anchored grid lands on 09:15 IST every day. The public fold API has no such guarantee, yet its docs promise "any width ... all are correct". For any width that does not divide 86,400, the grid drifts from day to day. That produces the mislabelled *leading* stub that the module's own comment calls "a lie", and `complete_minutes` certifies it with zero diagnostics. Price parsing, timestamp parsing, counts and session edges were all checked and are clean, or are already documented.

## Findings

### pul-1: low: fold grid drifts off 09:15 for widths that do not divide 86,400; `complete_minutes` certifies a pre-open leading stub

- **Where:** crates/pull/src/fold.rs:229-237 (anchor) and :312-323 (bucket start). The doc claims are at fold.rs:699-704 (`fold_from_snapshots`: "1 second, 7 seconds, 90 seconds and one day all go through here and all are correct") and fold.rs:241-243 ("Every width in `Timeframe::KNOWN` divides 86,400, so adding a day to the anchor would be equivalent").
- **Code:**
  ```rust
  const OPEN_ANCHOR_MICROS: i64 = IST_ANCHOR_MICROS
      - (store::path::Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT as i64) * 60 * 1_000_000;
  ...
  let start = shifted.div_euclid(width).checked_mul(width).and_then(|edge| edge.checked_sub(anchor))
  ```
- **Why it is wrong:** The anchor pins one grid edge to 09:15 IST on 1970-01-01 only. On day d, the edge sits at 09:15 minus ((d * 86400) mod w). That is zero only when w divides 86,400. `fold_from_bars` gates only `bucket % source == 0`, and `fold_from_snapshots` gates nothing (only 0 is refused, by `Bucket::of_secs`). So 7 s, 420 s (7 min), 660 s (11 min), 3,607 s and 25,200 s (7 h) all pass, and on most days each opens the session with a bar stamped **before 09:15** that holds only part of its declared width. This is exactly the defect the open anchor was introduced to remove (fold.rs:163-177: "a 30-minute bar stamped 09:00 containing 09:15-09:29 ... every later reader takes it as full"). `complete_minutes_with_calendar` counts `scheduled_minutes` by intersecting the bucket with the session, so it certifies a 3-minute bar stamped 09:11 as COMPLETE under a 420 s header. No look-ahead results (the edges stay on minute boundaries because the anchor and 86,400 are multiples of 60). The fault is a mislabel: the stamp and declared width are wrong, and the content is short.
- **Reachability:** public API only. No production caller passes a non-KNOWN width today. Related to but distinct from attackdata-5 (b), which recorded "`Bucket::of_secs` takes any width (7 s, `u32::MAX`)" and the u32::MAX epoch stamp. It did not record the per-day grid drift, the pre-open leading stub, or `complete_minutes` certifying it.
- **Repro (probe ran):** `$SCRATCH/probes/pul` (path deps on crates/pull and crates/store). Input: 375 one-minute bars 09:15–15:29 IST per day, plus 10 one-second snapshots from 09:15:00.
  ```
  7min from minutes, day 0: first bar stamped day 0 09:15:00 IST (open day 0 09:15:00 IST), n=54
  7min from minutes, day 20727: first bar stamped day 20727 09:15:00 IST (open day 20727 09:15:00 IST), n=54
  7min from minutes, day 20728: first bar stamped day 20728 09:10:00 IST (open day 20728 09:15:00 IST), n=55
  7s from snapshots, day 20728: first bar stamped day 20728 09:14:54 IST
  420s rung on 2026-08-04: first COMPLETE bar stamped 09:11 open=10000 close=10002 (minutes held: 3) ; last 15:29 ; complete=55 diags=0
  660s rung on 2026-08-04: first COMPLETE bar stamped 09:15 open=10000 close=10010 (minutes held: 11) ; last 15:29 ; complete=35 diags=0
  25200s rung on 2026-08-04: first COMPLETE bar stamped 08:15 open=10000 close=10359 (minutes held: 360) ; last 15:15 ; complete=2 diags=0
  ```
  Expected: the first bar of every regular session is stamped 09:15 (the module's stated invariant), or the width is refused. Actual: 09:10 / 09:14:54 / 09:11 / 08:15 depending on the day, and `complete_minutes` reports `diags=0`.
- **Fix direction:** anchor at the open of the snapshot's own IST day (per-day origin), or refuse widths that do not divide 86,400 by name. Then correct the "all are correct" doc either way.

## Checked and clean

- **fold OHLCV** (fold.rs:152-380): open is first in file order, close last, high max, low min. Volume uses `checked_add` and refuses on overflow. A null OI never overwrites. Out-of-order input is refused. Bucket start is checked arithmetic with `div_euclid`. Capacity reservation is capped at `snapshots.len()`. Per snapshot cost is O(1).
- **Rung alignment for KNOWN widths:** the open anchor gives 09:15 for 2, 3, 5, 10, 15, 30 and 60 min. DAY_1 is anchored at IST midnight. The partial *trailing* stub (e.g. 15:29 on 2 min, 15:30–15:39 on the derivatives 385-min session) is by design (D-1447), and `scheduled_minutes` counts it consistently.
- **No look-ahead in fold:** bars are left-labelled `[t, t+tf)`, which matches charter row "Bar timestamp ... OPEN (left edge) ... VERIFIED". `fold_from_bars` refuses non-multiple widths (`NarrowerThanSource`).
- **complete_minutes_with_calendar:** the cursor is amortised O(1) per bucket, `scheduled_minutes` uses at most `MAX_WINDOWS` intersections in i128, and `ist_day_of` is checked. Exceptional-day per-bucket lines are already hunt-pull-3.
- **session.rs:** `Day::days_from_epoch` / `from_days` use checked `+719_468`. `IstMoment::from_epoch_secs` checks the add and guards the u32 day before the cast. `Window::verdict` uses venue hours from a bounded table (`hours_on` walks at most `MAX_LATER_SESSION_ROWS` rows, so O(1) per bar). The close is exclusive.
- **http.rs `one_price`:** text-based half-up via `Paisa::from_rupee_text_half_up`, with no float arithmetic. Non-zero-snaps-to-zero is refused, and so are negatives. serde_json exponent renderings (`1e-6`, `1e-7`, `1.2345678901234566e+16`, all confirmed by probe) are refused as NotDecimal (loud). `1.5e3` renders as `"1500.0"`, which is accepted and exact. The second-rounding risk past 15 significant digits is DOCUMENTED (D-1494, docs/06-limits.md:13972).
- **core `from_rupee_text_half_up`:** negative half-up rounds toward +inf (-0.145 → -14). It is length-bounded, every step is checked, and a bare sign or bare `.` is refused.
- **csv::paisa:** refuses a third decimal, `+`, an empty whole part and anything non-digit. Mul and add are checked. `-0.50` → -50 is correct.
- **one_number / volume:** `i64::MIN` is refused. Floats go through csv::paisa and fractions are refused. A u64 above i64::MAX is refused.
- **rolling::paisa:** now has the snap-to-zero and negative guards (hunt-pull-2 FIXED, D-1492). The rolling `number` volume/ts zeroing is DOCUMENTED (D-0952).
- **rolling::shift_six (IV millionths):** checked arithmetic. Integer storage of IV is a documented choice (`Overlay` header). Two edges are noted but not reported, because IV is positive and vendor IVs are nowhere near those magnitudes. First, the negative branch rounds half away from zero, not half-up. Second, a JSON IV < 1e-5 renders in exponent form and becomes OI_NULL.
- **Timestamps:** `local_seconds` uses an exact 19-byte shape with range-checked clock fields. `stated_offset` is bounded at 14 h / 59 m. Millis are truncated by `div_euclid(1000)` in `land`, but `broker_stamp_on_grid` refuses a sub-second or off-grid stamp before the fold (ingest.rs:1141).
- **ingest `from_window` dedup:** a pre-sized HashMap with one insert per row (O(1) expected), and conflicting duplicates are refused.
- **Decoders:** no `Vec::remove` / insert(0) in loops. `retain` is one pass. `kept` is a single mask pass.
- **gaps:** walks 1,440 minutes per day (constant per day, already in the hunt-pull table). F&O 385-minute session undercount (hunt-pull-1) is still open at HEAD: `classify_against` uses `Session::full()`. The index gap audit on 2026-08-04 matches the venue table (`NseIndex.hours_on` = (555, 930) by probe), so there is no new defect there.
- **tenor `validated_band_seconds` vs `below_validated_band`:** exact integer division (basis / 50) that agrees with the f64 0.02 comparison.

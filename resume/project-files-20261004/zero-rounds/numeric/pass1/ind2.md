# ind2: look-ahead in crates/indicators and its callers in runner and cli (commit 331b05c)

## Verdict

**NO NEW FINDINGS.** I found no path in `crates/indicators`, or in the runner/cli callers in my slice, where a mask at bar N reads anything after bar N. I also found no path where a mask decided at a bar's close is filled at that same bar's open. A probe that actually ran supports this. I multiplied every bar at or after a cut by 3x (prices) and 7x (volume), including the stored minutes and the daily references derived from them. The masks before the cut did not change, on the native 1-minute path or on the anchored+overlay path for the 15, 10, 2 and 60 minute rungs.

## Findings

None.

## Probe (ran)

Location: `$SCRATCH/probes/ind2/src/main.rs`. It uses path deps on indicators, runner and vocab. Build: `CARGO_TARGET_DIR=$SCRATCH/target cargo run --release`.
- Native: `Column::build` with `Availability::Present` over 14 synthetic 375-minute sessions, with about 1.7% random minute holes. Two checks: (a) perturb every bar from the cut onward and compare the rows sourced before the cut; (b) check that the column of a prefix equals the prefix of the full column.
- Anchored: `AnchoredColumn::build` plus `overlay_exact_minute_orb_and_gapfib` on `runner::resample` rungs. Daily references are rebuilt from the perturbed minutes, so the cut day's own daily bar is also poisoned.

Verbatim output:
```
native 1min cut@3000: rows before cut 1155, mismatches 0
native prefix@3000: compared 1155 rows, mismatches 0
native 1min cut@3001: rows before cut 1156, mismatches 0
native prefix@3001: compared 1156 rows, mismatches 0
native 1min cut@3375: rows before cut 1530, mismatches 0
native prefix@3375: compared 1530 rows, mismatches 0
native 1min cut@4100: rows before cut 2255, mismatches 0
native prefix@4100: compared 2255 rows, mismatches 0
anchored+overlay 15min: rows before cut 33/150, mismatches 0
anchored+overlay 60min: rows before cut 0/0, mismatches 0
anchored+overlay 10min: rows before cut 154/332, mismatches 0
anchored+overlay 2min: rows before cut 814/1692, mismatches 0
```
(The 60-minute rung never warms within 14 sessions because EMA200 needs 200 bars, so that row proves nothing.)

## Checked and clean

- **`vwap::availability_of` (vwap.rs:288)** reads the whole slice. Its only callers are tests and `runner/benches/ratio.rs`. Production uses `cli::stored::vwap_availability` (stored.rs:2063), which is decided by `InstrumentKey.kind` and reads no bar. All runner production sites pass `Availability::Absent`.
- **`Evaluator::stepped` (evaluator.rs:805)** keeps the order: roll over, emit, then fold. Every anchor is read before the fold: the running day high/low/open, curday fib, ORB, GapFib, crossings `last_side`, and prev5. A receding timestamp is refused (`TimestampNotIncreasing`), so a later session cannot be installed as `yesterday`. `close_the_books` is a no-op in external-daily mode, and non-regular sessions are not handed forward.
- **`AnchoredEvaluator::advance_before` (anchored.rs:484)** installs only references with `day < signal_day`. Same-day daily OHLC is never an anchor. The probe poisoned the cut day's daily bar and saw no earlier change.
- **ORB (orb.rs)**: a window is closed by clock minute before the emit and folded after it. Nothing is emitted inside the window. On coarse rungs the window over-collects prices from after the boundary, but those come from the same already-closed bar. That is not look-ahead; it is documented under D-0527 and was hunt-indicators-9.
- **GapFib (gap.rs)**: the opening candle closes only when a bar from a later 3-minute bucket arrives, and the leg is settled before the emit. A 2-minute rung's first candle includes 09:18. This is documented ("by START minute"), and it is still in the past at the time of emission.
- **`overlay_exact_minute` (anchored.rs:780)**: it maps only the minute stamped exactly at `t+len-1min`, clamped to the caller's per-day `session_close`, which is `pull::calendar` via `stored::nse_session_close_minute`. It requires that minute's close to equal the signal close. No later minute is ever substituted, and no whole-slice threshold is read. This was fixed under D-0943 and confirmed by the probe.
- **`runner::align::onto_execution`**: entry is the 1-minute bar stamped exactly at `ts+len`, on the same IST day, otherwise `None`. `Column::reproject_checked` marks the result `Sourced::Fill`. `trade::walk` adds the +1 step only for `Sourced::Signal`, and requires the entry to be exactly `step_at(entry)` after the signal (a cadence measured on `0..=entry`, D-1410) and on the same session. `signal_candle_stop` enters at the aligned minute's open, and the stop is the signal bar's low/high, which is known at its close.
- **`runner::resample`**: buckets are clock-keyed at the 09:15 anchor and emitted only once provably complete. A trailing bucket that is partial because the data ended can never be entered: alignment needs the minute at `ts+len`, and the overlay needs the exact closing minute with an equal close.
- **`outcome::forward`**: entry at the signal close, close to close. This is a stated assumption in docs/05. Excursions are taken over `[i+1, exit]`.
- **Trade admission depends on the day's 15:09 row existing.** This is a data-presence condition, not price information, and it is documented in docs/06-limits.md around line 7063.
- **The VIX entry/exit-minute full-candle annotation** (cli/index_stop_vix.rs `POLICY`) is reference only and never enters ranking or identity. Not a finding.
- **`PastPrefix`** still has no production caller. This is KNOWN and documented (D-0212, hunt-indicators-12).
- **Numerics in indicators**: no floats, and no lossy `as` casts outside tests or const-asserted sites. Level arithmetic uses i128 or checked operations. All of this was covered in depth by hunt-indicators and not re-reported.
- **Per-bar cost**: the Evaluator modules are a fixed set. The overlay is O(minutes + signal) per run, which is documented. `isqrt` is KNOWN (W3-indicators2-0).

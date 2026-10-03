# SLICE=apis — numeric / look-ahead / O(1) pass 1

Scope: `crates/api/src/server.rs`, `sweeprun.rs`, `autopilot.rs` @ 331b05c. Audit only.

## Verdict

These three files have been audited many times already. Every per-request scan, pagination path and query-param parse I traced is either bounded or already written down: W1-api5-2..11 and D-0352/D-0686 in `docs/06-limits.md`, plus hunt-api.
- `mask_words` and the u64 attempt ids are sent as decimal strings (`mask_words`, `attempt_key`, `command_acceptance`, `requested_attempt`).
- No query param reaches an allocation size without a cap.

I found one new numeric defect, and it is small: the greeks pricing joins a bar's **close** premium with a tenor measured at the bar's **opening stamp**. I also found one cosmetic string defect.

## Findings

### apis-1 — low — server.rs:13320-13369 (`price_group`)

```rust
let ts = row.bar.ts_micros;
...
let tenor = match pull::tenor::Tenor::between(ts, inputs.expiry) {
...
quotes.push(pull::pricing::Quote {
    ts_micros: ts,
    spot,
    ...
    premium: row.bar.close,
    tenor,
```

**What is wrong.** The bar stamp is the bar's open (the fold's open anchor). The premium is the bar's close, which printed one cadence later, and `SpotBook::of` also joins the index **close** at that stamp (`pull/src/pricing.rs:379`). So price and spot are close-of-bar values, but the time to expiry is measured from the bar's open. Every priced row is therefore given one extra cadence of life:
- 60 s for minute bars.
- For `Cadence::Daily` (`asked.granularity.cadence()`, server.rs:12752), the gap is the whole span from the stamp to the session close. This part is INFERRED, not probed.

Near expiry the error dominates. Implied vol and greeks are computed against a √T that is too large: by √1.5 on the 15:37 bar and by √2 on the 15:38 bar. The last bar (15:39, which closes at 15:40, the expiry instant) is priced with 60 s left when its close printed at zero tenor. It should be refused as `AlreadyExpired`.

This is not documented: the `Quote` docs say "premium … The bar's close" and "tenor … see crate::tenor", and `docs/06-limits.md`, `docs/11-findings.md` and the prior reports have no entry.

**Impact.** Greeks are a stored reference sidecar, and option contracts are never swept, so ranking is unaffected. The stored IV/greeks for the final minutes of every expiry day are wrong.

**Repro.** Probe `$SCRATCH/probes/apis` (path deps on `crates/pull` and `crates/core`). It calls `Tenor::between` for expiry 2026-09-01 (derivatives close 15:40), once at the bar stamp and once at stamp+60 s, which is when that bar's close printed. Output, verbatim:

```
bar stamped 09:15: tenor(stamp)=Ok(23100)  tenor(stamp+1min, when its close printed)=Ok(23040)
   years(stamp)/years(close) = 1.0026  sqrt ratio = 1.0013
bar stamped 15:37: tenor(stamp)=Ok(180)  tenor(stamp+1min, when its close printed)=Ok(120)
   years(stamp)/years(close) = 1.5000  sqrt ratio = 1.2247
bar stamped 15:38: tenor(stamp)=Ok(120)  tenor(stamp+1min, when its close printed)=Ok(60)
   years(stamp)/years(close) = 2.0000  sqrt ratio = 1.4142
bar stamped 15:39: tenor(stamp)=Ok(60)  tenor(stamp+1min, when its close printed)=Err(AlreadyExpired { seconds_past: 0 })
```

Expected: the 15:39 bar is refused, and the 15:38 bar is priced with 60 s. Actual: 60 s and 120 s.

**Fix direction.** Measure the tenor at `ts + cadence` (the instant the close printed), or document that the tenor is measured at the bar open and accept the bias.

### apis-2 — info — server.rs:13338

```rust
"the vendor sent no underlying level beside this bar, so there                  is nothing to price it against",
```

A line continuation lost its `\`. The refusal reason that operators see therefore contains an 18-space run. It is cosmetic only: deduplication is exact-string, so it still works.

## Checked and clean

- `page_number` (server.rs:829): `usize` parse, negative/overflow → 0. This default is documented. Every consumer clamps with `page.min(last_page)` and `saturating_mul` (audit_html 14970, store_html 15209/15234, bars_get 15763, catalog page/search).
- `WindowAsk::parse` offset/limit (server.rs:3305-3327): non-number refused. `limit.min(MAX_WINDOW_LIMIT)`, `Vec::with_capacity(limit.min(PAGE_BARS))`, and the offset partition `page_of` (D-1446). Known/fixed.
- `bars_json` from/to (server.rs:2320). Bounds come from `parse_day` (4-digit year, `Day` 1970..=9999), so `days_from_epoch*86_400*1e6` cannot overflow. Asking for a window past the last bar reads the whole month; that is known as W1-api5-8.
- `basis_points` (server.rs:4217): divide-by-zero refused, overflow checked, half-away-from-zero rounding is correct for negative remainders (D-0069).
- `month_before` / autopilot `ordinal`, `from_ordinal`, `month_after`, `months_owed`: bounded by `YearMonth` 1970..=9999, no wrap.
- sweeprun `asked_from_wire`: years are bounded to u16 before the `*12` (prior abort fix). `max_points` ≤0 refused. `top` = 0 refused. `requested_attempt`: canonical, length-capped u64.
- u64 ids on the wire: `Progress::to_json` gives `attempt_key` as a string, `command_acceptance` gives `attempt` as a string, `browser_attempt_unknown` gives `requested_attempt` as a string. `observe_elsewhere` also emits a numeric `attempt`/`run`, but `attempt_key` is present and the web reads `attempt_key`/strings. No `ConditionMask` words are serialized in these three files.
- Timing: `now_micros` saturates; elapsed uses `saturating_sub().max(0)`. `elapsed_micros` (index-stop) is a string. Autopilot `elapsed_ms`/`waiting_ms`/`absorbed_ms` are saturating u64 and far below 2^53 in practice. `render_elapsed` truncates, but it is display only.
- 429/5xx ladder: `throttle_ladder` uses `attempt - 1` and is reached only from loops starting at 1. A compile-time assert bounds the sum and the shift.
- `millionths_to_decimal` (server.rs:13399): exact to 2^53 and not a price.
- Per-request scans in `instruments_json`, `store_json`/`census_now`, `store_html` filtered, `calendar_json`, `verify_json`, `indexmap_json`, `spot_targets`: all known (W1-api5-2/3/5/6/7/9/11, D-0686).
- Autopilot `tracked_series` sort is cached per masters generation (W1-api1-1). `frontier` uses a monotone hint.
- `vocab_json`: its `break` arms are unreachable because `definition` is `TABLE.get` over `0..COUNT`.
- `store_body` timestamps are µs (< 2^53 until 2255). `chg` is integer bps.

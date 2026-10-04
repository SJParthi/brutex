# Numbers audit, pass 10: data shaping before the engine

**Head audited:** `1f4de71` (origin/final/all-fixes-zero), read-only checkout `/home/claude/wt/zero3`.

**Verdict at 1f4de71:** NOT ZERO. There is 1 new medium finding (p10num-1) and 1 new low finding (p10num-2). The rung folding, the derived-rung completeness gate, the text-to-paisa write boundary, volume overflow and gap minute counting all hold. Known ids (pul-1, p2misc-2, ind1-*, STO-*, core-1 and others) are not re-reported here.

## Table

| # | Area | Code | Result |
|---|---|---|---|
| 1 | Rung grid anchor | pull/src/fold.rs:207-213 (open anchor for widths under one day, IST midnight for one day); runner/src/resample.rs:236-247 | OK. Every intraday rung opens at 09:15. A short bar is left only at the end of a session. |
| 2 | Missing minutes inside a bucket | fold.rs:858-1003 `complete_minutes_with_calendar`, called by ingest.rs:2490 `derive` (minute source only, :2471) | OK. A bucket is written only when its count equals the scheduled count, every stamp is minute-aligned, there are no duplicates and every minute is in session. Otherwise it is withheld and named. derive_all re-reads the whole committed month (ingest.rs:2210-2240), so an append made in two batches does not create false gaps. |
| 3 | Partial last bucket | fold.rs:186-196; tests/anchor.rs | OK. The stub is stamped correctly and pinned per venue (D-1447). |
| 4 | Outage day 2021-02-24 (09:15-11:39, 15:45-16:59) and Muhurat/DR sessions | calendar.rs:274-284; fold.rs:986-990 | OK. Exceptional sessions are withheld once per day. A 15:15-stamped hour bar that holds only 15:45 onwards cannot be written. |
| 5 | OI across a bucket | fold.rs:357-359; resample.rs:416-422 | OK. The last value that is not the null is kept, never summed. See p10num-2 for the index zero. |
| 6 | Second resampler `runner::resample` | resample.rs:276 | Has no completeness check, but has no production caller (only tests in identity.rs:1015, validate.rs:5770, outcome.rs:3740). Not a finding at this head. |
| 7 | Text rupees to paisa, half-up once | core/src/price.rs:222-290; http.rs:1491-1528 `number_text`, :1540-1600 | OK. Exact digit walk. Negative ties go toward +inf. An exponent is shifted exactly and bounded by MAX_PRICE_TEXT. A non-zero value under half a paisa is refused. |
| 8 | Archive CSV price | pull/src/csv.rs:374-393 `paisa`, :776-801 | OK. A third decimal is refused (documented D-1494). A negative price is refused. `.5`, `+5` and `1e3` are refused. |
| 9 | Tick grid 5 vs 1 paisa | none | No per-instrument tick check exists. CLAUDE.md §7 fixes only a two-decimal grid. Not a finding. |
| 10 | Volume sums | fold.rs:346-353 (checked add, NegativeVolume refused :272); resample.rs:328; indicators vwap i128 | OK. No wrap is possible. Release overflow-checks cannot fire on these paths. |
| 11 | Gap minute counting | pull/src/gaps.rs:429-448; calendar `Session::bars` | OK. 375/385/360/220/105/60 are pinned by tests (gaps.rs:641-1143). Unmeasured days add 0 and are named. |
| 12 | fold_audit reference vs derive on cash days on or after 2026-08-03 | cli/src/fold_audit.rs:205 (no cash schedule) vs ingest.rs:2295 | Loud false disagreement. It is documented and pinned (fold_audit.rs:629-664, docs/06-limits). Not new. |
| 13 | Corporate actions, equities | runner/src/audit.rs:169 `CORPORATE_ACTIONS_UNCHECKED`; cli pool.rs:703, research.rs:228, lib.rs:19241, stored.rs | Unadjusted. Every stock report carries the label. The cross-vendor adjustment basis is UNVERIFIED (D-entry item 7, docs/05 ~48150). Not a finding. |
| 14 | **GDFL quote rows folded as trades** | csv.rs:888-902 and fold.rs:337-374 | **p10num-1** |
| 15 | **Index OI stored as a measured 0** | csv.rs:183-190, :839-852 | **p10num-2** |

## New findings

### p10num-1 (medium): GDFL quote-update rows (LTQ = 0) are folded into bar prices, so stored bars claim prices that did not trade in their interval

**Where.** `crates/pull/src/csv.rs:888-902`:

```rust
// A snapshot row carries ONE price, not four. Open, high, low and close
// are all that price ...
rows.push(RawRow { timestamp: epoch_utc, open: price, high: price, low: price, close: price, volume, open_interest });
```

Every GDFL row is kept, whatever its `LTQ`. `price` is the `LTP` column (offset 3, :194-199). `archive.rs:800` passes the rows to `fetch::land_rows` and then to `ingest::fold_in_place` (ingest.rs:2354-2365), which calls `fold::fold`. `fold` takes open from the first row, high/low from the extremes and close from the last row, with no regard to volume (fold.rs:337-374).

**Why it is wrong.** docs/08-vendor-samples.md:140 and csv.rs:105-106 both say: "`LTQ` is `0` on most rows — those are **quote** updates, not trades." On a quote row, `LTP` is the *last traded* price. That trade may have happened in an earlier minute. The fold therefore does three things:

- A minute's open, high or low can come from a trade made before the minute began.
- A minute with no trade at all becomes a stored bar with volume 0 and the stale price as OHLC.
- Daily bars folded from snapshots are affected the same way.

Bars are append-only (CLAUDE.md §3 rule 8), so these values cannot be repaired once written. Nothing records the exclusion rule. D-entry item 9 (docs/05-decisions.md ~48163) leaves "which field a one-second record is folded from" open for equities, yet the code already folds LTP from every row. This affects GDFL F&O futures and cash equities, which carry trades. Index files are different: their `LTQ` is always 0 and LTP is the index value, so they must keep every row.

**Repro (ran).** Throwaway test in a scratch worktree at 1f4de71, `cargo test -p pull --test p10num`, passed. Rows, decoded with `csv::decode(.., Columns::Gdfl)` and folded with `fold(.., Bucket::MINUTE)`:

```
09:19:58 LTP 100.00 LTQ 25   (trade)
09:20:00 LTP 100.00 LTQ 0    (quote update)
09:20:05 LTP 102.00 LTQ 50
09:20:40 LTP 103.00 LTQ 10
09:21:10 LTP 103.00 LTQ 0    (quote update only)
```

| Bar | open | high | low | close | volume |
|---|---|---|---|---|---|
| 09:20, folded | 10000 | 10300 | 10000 | 10300 | 60 |
| 09:20, from trades only | 10200 | 10300 | 10200 | 10300 | 60 |
| 09:21 | 10300 | 10300 | 10300 | 10300 | 0 |

The folded 09:20 bar has a low of ₹100.00, a price that never traded in that minute. The 09:21 bar exists although nothing traded in that minute.

**Minimal fix.** Pass a per-shape flag `Columns::trade_rows_only()`, true for `Gdfl` on non-index tickers. On those shapes, have `decode` skip rows with `LTQ == 0` and count them in a new `Tally::quote_rows`, so nothing is dropped silently. The alternative is to let `fold` ignore the price of a zero-volume row once its bucket holds a traded row, and to emit no bar for a bucket with no traded row. Either way:

- Record the rule as a D-entry that closes item 9.
- Add an invariant pinned by the five-row sequence above.
- Treat any GDFL F&O or equity minutes already stored as needing a versioned repair. Do not rewrite them in place.

### p10num-2 (low): index rows from archives store open interest as a measured 0, not the null sentinel

**Where.** `csv.rs:183-190`: `TrueDataIndex` reads `open_interest: 4`, which is the literal `0` column of `date, time, price, 0, 0`. GDFL index files likewise carry `0` in `OpenInterest` (charter :631, D-entry item 9: "LTP is the only field that is not zero"). At :839-852, `"0".parse::<i64>()` gives `Some(0)`, and `fetch.rs:964` stores 0, not `i64::MIN`.

**Why it is wrong.** CLAUDE.md §7 says "`i64::MIN` is the open-interest null sentinel. Zero means zero." docs/08-vendor-samples.md says that for indices these fields are "structurally absent rather than zero." The stored bar claims an index had a measured open interest of zero. The same index minute can then carry 0 from one feed and `i64::MIN` from another. No sweep reads index OI today, so the impact is limited to the stored record's honesty and to byte differences between feeds.

**Repro (not run).** `csv::decode("20221003,09:15:00,100.00,0,0\n", Columns::TrueDataIndex)` gives a `RawRow { open_interest: Some(0), .. }`, and after landing the bar has `open_interest == 0`.

**Minimal fix.** Mark the OI column as absent for `TrueDataIndex` and for GDFL `*.NSE_IDX` tickers, so they yield `None` and then `i64::MIN`. Add a D-entry and a test. Already-stored index bars stay as written (append-only) and are noted in docs/06-limits.

## Verification of earlier ids touched by this theme

| id | Status at 1f4de71 | Evidence |
|---|---|---|
| pul-1 (single 1970 anchor) | NOT FIXED as worded, harmless for stored rungs | fold.rs:207-213 is still one continuous anchor. Every `Timeframe::KNOWN` width divides 86,400, so the edges equal a per-day restart. resample.rs restarts daily (D-1430). |
| p2misc-2 (minute_gaps interior only) | Not re-examined; out of this theme | — |
| hunt-pull-1 (venue-dated gaps) | FIXED | gaps.rs:641-675 (385/375 by date) |

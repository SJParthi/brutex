# slice11 — 7 findings

Scope: crates/api/src/{assets,audit,audit_json,autopilot,backtest,bars,boolean_*,boolean*json,booleanlaunch*,boolean*_projection,calendar}.rs (production parts; tests consulted). Probes were run as a temporary crates/api/tests/zz_probe_slice11.rs, now deleted. Probe output is pasted below.

## F1 [medium] `/backtest.json` reports `appendable:true` for a ledger with a torn tail, and every `cli` append refuses that ledger
- where: crates/api/src/backtest.rs:914-918 (`appendable = version == VERSION && refusal.is_none()`), :1143 (`partial_tail` is computed and ignored for this flag)
- what: `appendable` exists so the operator learns before a sweep starts whether the sweep can be recorded (comment at :884-912: "Hours of CPU for a sentence that was knowable from sixteen header bytes"). `cli::results::Results::open_with` (cli/src/results.rs:945-965) and `absorb_new_records` (:1404-1410) both refuse a ledger whose payload is not a whole number of strides. The api flag ignores `partial_tail`, so it says the ledger can be appended to while every recorded run is refused. This is the hours-of-wasted-sweep case the flag was built to prevent.
- evidence (probe: v3 header + one 261-byte record + 5 stray bytes, then `api::backtest::read(root,10).to_json()`):
  `appendable=Some("true") partial_tail=Some("true") refusal=Some("null")`
- fix: `appendable: self.version == VERSION && self.refusal.is_none() && !self.partial_tail`, plus a test.

## F2 [low] One refused store write halts a feed with "the store refused the same write twice" if any earlier failure that month was a transport error
- where: crates/api/src/autopilot.rs:1366-1378 (`repeat = classify(&reason) == Trouble::Store && self.attempts > 0`)
- what: `attempts` counts every failed tick for the month, of any class. A transport failure followed by a single disk refusal therefore counts as a "repeat" and makes the feed terminal (`Halt::Store`). The reason text then states something that did not happen. The documented policy (:1297, "The same store refusal twice halts") is not what the code does.
- evidence (probe on public `FeedState::observe`): `after transport: Wait { secs: 30 }` then `after ONE store refusal: Halt { reason: "the store refused the same write twice: NIFTY — disk full writing /store/x.bin. ..." }`
- fix: keep a per-month "last failure was Store" flag (or a store-refusal count) and halt only when the previous failure was also `Trouble::Store`.

## F3 [low] The "at most one event per request" bound for `records unreadable` is false: it is one event per month file, up to 240 per request
- where: crates/api/src/bars.rs:374-378 (doc: "exactly one call site — [`read_page`], outside every loop ... at most one event per request"); docs/06-limits.md §44 (lines ~3253-3256, same claim). Code: `note_unreadable_records` is called inside `slots` (bars.rs:454). `slots` is called per file inside the `for file in files` loops of `seek_page` (:877) and the sorted/extremes path of `window` (:1010-1015).
- what: no `read_page` exists in bars.rs (that name belongs to audit.rs). A sorted or `extremes=1` window over up to `MAX_WINDOW_MONTHS` = 240 damaged months emits up to 240 Warn lines for one request. That is the per-request volume growth §44 says is structurally impossible.
- evidence: trace above. `window` → `for file in &files { slots(file, 0, held) }` → each `slots` call ends in `note_unreadable_records(...)`, which emits whenever `faults` is non-empty.
- fix: return the faults from `slots` without emitting, and emit once from `window`/`page` with the aggregated count and first reason. Or correct the bars.rs doc and limits §44 to "one per month file read".

## F4 [low] The LOOKBACK doc says only the very first record of the window has no change; in fact the first record of every month file has none
- where: crates/api/src/bars.rs:816-823 ("strictly better than the browser ... where the first row of EVERY month had nothing behind it. The only row that still has none is the first record of the first file"); code :871-876 (`start.checked_sub(LOOKBACK)` returns `None` at the start of any file, giving `Behind::Nothing`), and the scan path :1013 (`with_change(Behind::Nothing, ...)` per file).
- evidence (probe: Jan 10 bars and Feb 10 bars, Ts page at offset 8, limit 4):
  `close=1009 chg=Some(10)` / `close=2000 chg=None why=first_bar_in_file` / `close=2001 chg=Some(5)`. The first Feb bar, mid-window, has no change.
- fix: read the previous file's last record as the lookback when `start == 0` and a previous file exists (seek path). Otherwise correct the doc to "the first row of every month, as before".

## F5 [low] `SortKey::OpenInterest` is documented "nulls last", but ascending puts nulls first
- where: crates/api/src/bars.rs:568 (`/// Open interest, nulls last.`) vs :596-608 (null maps to `i64::MIN`, the smallest value) and :1024-1028 (ascending is `x.cmp(&y)`)
- evidence (probe, `SortKey::OpenInterest`, ascending, limit 6): the first five rows are `oi=-9223372036854775808 null=true`, then `oi=6`.
- fix: either order nulls last in both directions (a primary `is_null` key that is not inverted) or change the variant doc to "nulls sort smallest (first ascending, last descending)".

## F6 [low] Autopilot safety-default docs contradict the code: they say it flies by default or without `run`
- where: crates/api/src/autopilot.rs:2057-2058 (`Control::serving`: "The control a SERVING process gets: **flying**, unless the environment says [`AUTOPILOT_PAUSE`]") and :1934-1937 (`AUTOPILOT_RUN`: "Kept accepted although it is no longer required ... it is simply no longer the only way to")
- what: `stays_paused_from` (:1987-1989) grounds everything except exactly `run`, so the default is PAUSED and `run` is the only way to fly. These are the docs on the "does starting the binary contact a vendor" decision point, which :2060-2063 says is "the one place the default lives". They state the opposite polarity.
- evidence: `value.is_none_or(|v| v != OsStr::new(AUTOPILOT_RUN))`. `serving()` calls `pause()` unless `flies_on_startup()`.
- fix: reword both docs to "paused unless BRUTEX_AUTOPILOT=run; `run` is the only value that flies".

## F7 [low] Two autopilot status messages claim things that will not happen
- where (a): crates/api/src/autopilot.rs:2940-2953. When write probe 8 of 8 (`STORE_PROBES`) fails, the message says "The next probe is in {wait}s" (`wait` = `probe_secs(7)` = 3600). `made` becomes 8, so the next pass answers `Due::Spent` (:537) and no ninth probe ever runs.
- where (b): crates/api/src/autopilot.rs:3069-3076 and :3032-3039 (`Settled::NoUniverse`: "The masters are read once, at startup, so this cannot resolve itself — fix the masters directory and restart", with phase `Halted`). The masters ARE re-parsed at runtime: `mastersrun::reload` calls `Site::reparse` (mastersrun.rs:382, server.rs:5439). `SeriesCache::get` rebuilds on the new `generation` (autopilot.rs:2348-2372), and `fly` keeps looping after `round` returns `IDLE_POLL_SECS`. A masters refresh therefore resumes the backfill without a restart, and the page tells the operator a restart is required.
- fix: (a) when `attempt == STORE_PROBES`, say the allowance is now spent rather than naming a next probe. (b) say "refresh the masters (or restart); this re-checks every minute", or keep the phase `Idle`.

## Checked and found sound (no finding)
- calendar.rs: weekday arithmetic (`(days+3)%7+1`), month lengths, floor clamp, CSS class collisions.
- assets.rs: single percent-decode, segment rule, canonicalised root `starts_with`, favicon alias goes through every guard.
- audit.rs: fixed-stride image/decode/CRC domain, locked append with torn-tail refusal, newest-first paging bounded by `MAX_PAGE_RECORDS`.
- audit_json.rs: required feed, page clamp; the rollup cache's `Weak::ptr_eq` cannot alias, because the held Weak keeps the allocation.
- backtest.rs: v2/v3 stride, seal-before-widen, newest-first O(limit) read, `best_complete` total order.
- bars.rs: Indian grouping, IST clock/day, `seek_page` asc/desc offsets across files, `page_of` partition equivalence.
- boolean*json / projections: query parsing (repeat/unknown/empty refusals, limit 1..=256, completion pin required past page 0), cache admission via `must_admit`, cursor and next arithmetic, field counts (44/39). booleanlaunch: digest/positive parsing, `Status::observe` monotonicity; `finished_micros = started` is overwritten by the caller (sweeprun.rs:3159).
- Stale-but-minor docs I did not raise: backtest.rs:16-20 and :479-482 say api does not depend on `cli`/`vocab` (Cargo.toml has both); audit.rs:60-62 says "eight terms" (now nine); `RESUME_CANNOT_CLEAR`/`Halt` say "three" halt classes and sites (there are four classes).

## Known, still present
- webcontract-5 (assets without cache headers) and o1surface2-2 (autopilot reads manifests twice) are still as recorded. AC-whp-law-2 (backtest.rs:428 "Signal bars swept") is unchanged.

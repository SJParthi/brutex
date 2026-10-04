# Crash and edge-input audit, pass 16: time and clock handling (ce16)

**Head:** `final/all-fixes-zero` 1f4de71 (read-only checkout /home/claude/wt/zero3).
**Verdict:** 4 new findings, all low: CE-84 to CE-87. None of them is a crash route. They share one root: the **wall clock is used as an interval timer or an age**. When NTP steps the clock, or the host boots with a wrong RTC, the timer stalls or reads 0. That outcome is either silent or contradicts what the page says.

No IST/UTC, month-boundary, leap-day, 2038 or `new Date(string)` defect was found. The table at the end lists what held.

Not re-reported:
- pass 7 (CE-52..55, calendar and session);
- CE-72 (mapping `toLocaleString()` without a zone);
- CE-50 (pre-1970 `finished_micros`);
- CE-46 (Stop during the autopilot clock wait).

No cargo run. Every finding is low and read from source.

---

### CE-84 (low): the autopilot's store-probe and stall-recheck timers run on the wall clock. An NTP step back postpones them by the size of the step, and a clock that was once ahead freezes them. The page meanwhile says "the next probe is in 60s".

**Sites:**
- crates/api/src/autopilot.rs:3002, where the probe deadline is stamped in wall-clock epoch seconds:
  `due_unix: now.saturating_add(i64::try_from(wait).unwrap_or(i64::MAX)),`
  The `now` it uses is `ingest::epoch_secs(SystemTime::now())`, taken at :2974.
- :548 `if now_unix < probe.due_unix { return Due::Later {..} }`
- :1489 and :1516 `stall.at_unix = now_unix;`
- :1501 `let due = now_unix.saturating_sub(stall.at_unix) >= STALL_RECHECK_SECS;` (6 h)

**Why it is wrong:**
- Both values are intervals ("probe again in 60 s … 1 h", "recheck a stall after 6 h"), but they are measured on `SystemTime`.
- **Clock stepped back by X.** Every armed probe and every stamped stall waits an extra X. The detail line still says "The next probe is in {wait}s" (:3015), and the spent message promises "over two hours and three minutes" (:540).
- **Clock that was once ahead.** A host that booted with a fast RTC and was later corrected keeps `due_unix` / `at_unix` in the future. `now - at_unix` is negative, so the stall is never due until real time catches up, which can be months or years. A store-halted feed is never re-probed.
- No event names either case. The rest of the module already uses monotonic time for exactly this kind of wait: `nap` and the countdown use `tokio::time::sleep`, and `tick` uses `Instant` for `took`.

**Repro:** not run. It needs a stepped host clock, and the effect follows from the source: `store_due` and `reconsider` are pure over `now_unix`, so `store_due(Some(&Probe{made:1,due_unix:T+60}), T-86_400)` answers `Later`.

**Minimal fix:**
- Hold `std::time::Instant` (or `tokio::time::Instant`) in `Probe` and `Stall` for the deadline.
- Keep `epoch_secs` only for the stamps shown on the page.
- Alternatively, treat `now < at_unix` (a clock behind a stamp) as "due" and emit a warn naming the clock step.

---

### CE-85 (low): `/masters/status.json` decides "a master is newer than the parse" by comparing a file's mtime with the wall-clock instant of the parse. A clock skew either hides a changed master or demands a restart that cannot clear the flag.

**Sites:**
- crates/api/src/server.rs:5519 `let parsed_at = std::time::SystemTime::now();` (and :5632 on first load)
- crates/api/src/mastersrun.rs:726-729:
  ```rust
  let newer = held.as_ref().and_then(|m| m.modified().ok())
      .is_some_and(|at| at > parsed_at);
  ```
  which feeds `"restart_required":{any_newer}` (:744-750).

**Why it is wrong:** `parsed_at` is this host's clock, and mtime is whatever the filesystem recorded. The two are compared as if they were one clock.

1. **Clock ahead when the site parsed, corrected later.** A master replaced in that window has `mtime < parsed_at`, so the result is `newer_than_parse:false` and `restart_required:false`. The process keeps resolving symbols from the old bytes and says nothing. This is the §4 silent direction: the doc at :689-692 says this route is "what says so".
2. **File mtime ahead of this host's clock** (host clock behind, or a master copied with preserved times from a host whose clock was ahead). The result is `restart_required:true`. A restart re-stamps `parsed_at = now`, which is still before the mtime, so the flag survives the very remedy it names.

**Repro:** not run (source). `status_rows(dir, parsed_at)` is pure over its inputs, so a test can set a file's mtime with `File::set_times` (std, already used in telemetry/src/lib.rs:490) one hour after `parsed_at` and observe `restart_required:true` across a re-stamp.

**Minimal fix:** capture each source's `(modified, len)` at parse time inside `Parsed`, and report `newer_than_parse` as **changed since the parse** (`!=`). Do not compare against the wall clock.

---

### CE-86 (low): the sweep, descent and command completion events record `elapsed_micros` as the wall-clock difference, clamped to 0. An NTP step back during a run records "took 0 µs", and the event does not say what the figure is based on.

**Sites:** crates/api/src/sweeprun.rs:2160-2161, :2295-2296 and :3448-3449.

```rust
let elapsed = now_micros().saturating_sub(started);
let _outcome = emit_completion(&finished, "sweep", elapsed.max(0).unsigned_abs());
```

`started` is `now_micros()` (SystemTime, :1809-1815), and `emit_completion` writes it as `"elapsed_micros"` (:1621).

**Why it is wrong:**
- This is a duration, and it is taken from `SystemTime`. A backward step larger than the run gives a negative value, and `.max(0)` turns it into a recorded measurement of 0 µs. A forward step inflates it.
- That breaks CLAUDE.md §3 rule 6 ("Never claim a measurement you did not take"), and the `.max(0)` is a fallback that hides the failure (§4).
- The same file already does this correctly for the index-stop progress fields. At :213-222 a negative difference becomes `null` and the field is labelled `"elapsed_basis":"wall-clock"`. The completion event does neither.

**Repro:** not run (needs a clock step; arithmetic from source).

**Minimal fix:** take an `Instant` beside `started` where the task is spawned, and emit `began.elapsed()`. Or keep the wall-clock figure, but emit `null` plus a warn when it is negative, and label its basis as :222 does.

---

### CE-87 (low): an external CLI sweep that died is reported as `"running"` past the 15-minute staleness bound whenever its last event is stamped ahead of this host's clock. `age_millis` is clamped to 0, which hides the stale state.

**Site:** crates/api/src/sweeprun.rs:2704-2709:

```rust
let age = now.saturating_sub(last.at_unix_millis).max(0);
... ("command started", "running") if age <= STALE_AFTER_MILLIS && ... => ("running", ""),
```

**Why it is wrong:**
- The CLI's telemetry sink clamps every stamp up to its floor (telemetry/src/sink.rs:597-601), and it resumes that floor from the last line on disk (:840-848, D-1325/D-1538).
- So after any backward clock step of X, the CLI's events are stamped up to X in the future. A CLI that dies in that window gives `now - at < 0`, and `.max(0)` turns that into `age_millis: 0`. The status is then "running" with an empty `why`, for up to X + 15 min, instead of the module's own "unknown … silence is not completion".
- Launch stays blocked either way, because `launch_clear` requires completed or refused. The defect is the false "running" verdict and the hidden negative age.
- The sink does count `clock_held` and names a floor resumed ahead of the clock in its own `Health`, but this route reads neither.

**Repro:** not run (needs a clock step; the logic is from source).

**Minimal fix:** when `now < last.at_unix_millis`, answer `"unknown"` with a why naming the clock: "the newest CLI event is stamped {Δ} ms ahead of this server's clock; activity cannot be aged". Emit the signed age instead of the clamped one.

---

## What held (checked at 1f4de71)

| Area | Site(s) | Why it holds |
|---|---|---|
| Host TZ / TZ unset | whole workspace | No `chrono`, `Local`, `localtime` or `TZ` read in any crate. Every civil date comes from `pull::session::IstMoment`/`Day` (fixed +05:30), or from Hinnant arithmetic with an explicit IST offset (store/path.rs:615-647, api bars.rs:96-134, calendar_of.rs:674-680, runner outcome.rs:1191-1208, cli institutional_evidence.rs:2211, index_stop_store.rs:561-565 and :626-629, boolean_* readers). Telemetry is UTC on purpose and labelled `Z` (telemetry/clock.rs:13-19). |
| "today" / "yesterday" / settled month | api ingest.rs:1897-1927 `ist_day`/`ist_moment`; autopilot.rs:2568 `yesterday_ist`; ingest.rs:1855 `last_settled_day` | All are IST, from one clock read per request (server.rs:1810, :11245). A pre-1970 or unrepresentable clock is a named `ClockUnusable`, never a default. The month rule steps back from the IST 1st, so it needs no month-length table. |
| Spot "session has closed" gate | server.rs:8300-8358 `finished_day_only` | IST minute against `Venue::NseCash` close from the table. `hours_on` is Verified for every day (vendor.rs:1166-1195), so a running session refuses. (The comment's "a holiday answers Err" is stale but harmless: a holiday refuses until 15:30, which is conservative.) |
| Autopilot waits / backoff / grace countdown | autopilot.rs:2626-2634, :2783-2796 `nap`; :3510-3530 | Monotonic `tokio::time::sleep` and `Instant`. Only the probe and stall timers are wall-clock (CE-84). |
| Rate governor day window | pull/rate.rs:1342 | A monotonic `Instant` origin, so a clock step cannot refill or drain the daily allowance. |
| Log ordering under NTP step | telemetry sink.rs:520-601, :840-848 | `ms` is clamped to never go backwards, resumed from disk, and counted in `clock_held` (D-1538). A floor ahead of the clock is named in `last_error`. |
| CLI live-run staleness | cli/live.rs:681-693, :962 | An mtime ahead of `now` is `Freshness::Unknown`, counted as `undated`, never "fresh". |
| SigV4 date | pull/ssm.rs:973-994 | UTC civil date from epoch seconds. A pre-1970 clock is a named refusal. |
| Vendor text stamps | pull/http.rs:3559-3666, :3734-3772 | Exactly 19 bytes, `HH<=23 MM<=59 SS<=59`. Milliseconds or a fractional tail are refused loudly. An offset is required for `IsoDateTimeOffset`, bounded at ±14:59. A missing offset is never assumed IST. |
| Vendor stamp off the minute / with seconds | store/file.rs:3058-3080 `Admission::admit` | `OffGrid` is refused at the write boundary for intraday rungs, anchored at 09:15 IST. `EpochMillisUtc` (fetch.rs:807, `div_euclid(1000)`) would floor milliseconds silently, but no feed descriptor uses it (vendor.rs:4453/4715/5401). That is latent, not reported. |
| Text-stamp IST to UTC | fetch.rs:787-819 | The offset is subtracted exactly once. `IsoDateTimeOffset` is passed through (D-0135). CSV converts once at decode (csv.rs:806, :952). |
| 15:30 closing tick | session.rs:1133-1143 | `>= close_minute` is `AtOrAfterSessionClose`, dropped and counted (fetch.rs:858-880). |
| IST day boundary (00:00-05:30 UTC) | session.rs:714, gaps.rs:214-222, Window::verdict | `IstMoment` and Euclidean division, so 18:30Z+ lands on the next IST day. The window's `wire_to = to.succ()` extra day is dropped as `AfterWindow`. |
| Month of a bar | pull ingest.rs:1962-1990 `month_at`/`month_of`; store path.rs:615; cli index_stop_vix.rs:597, global_replay.rs:1700 | IST month on both writer and reader. The store refuses `OutsideMonth` against IST bounds. |
| Leap day | session `Day` (pass 7); store path.rs:640-647 | Hinnant day arithmetic, no tables. |
| 2038 / 32-bit seconds | grep over all crates | No epoch value is held in `i32`/`u32`. The `u32` fields named `*_secs` are durations or rung lengths. `cash_auction::lifecycle_day` (cash_auction.rs:~120) parses NSE's 1980-epoch seconds as `i32`, which overflows on 2048-01-19. That matches NSE's own signed 32-bit field, and an overflowing value is refused loudly as `OutOfRange` (Invalid), so it is not reported. |
| Elapsed time elsewhere | autopilot.rs:3511 `Instant`; server.rs:18576 shutdown deadline; pull rate | Monotonic. Only sweeprun's completion figure is wall-clock (CE-86). |
| Web: `new Date(string)` / local zone | web/src (all `new Date(`/`Date.parse`) | No zone-less date-time string is parsed. `Date.parse('YYYY-MM-DD')` is UTC midnight per spec, and db:4781 adds `Z` explicitly. Every civil field uses `getUTC*` after a `+19_800_000` shift (ingest:939/1244, db:4767, autopilot:376/884, markets:1467, backtest:4986), or uses `Intl` with `timeZone:'Asia/Kolkata'` (dates.js:102, candidate-trades.js:173, invocation-audit.js:57, IndexStopChart:7, backtest:5813/5853/5897). trade-analytics.js week keys are Monday-based on IST days. boolean-catalog.js:23 formats a civil day in UTC, which is correct for `day*86400000`. The only zone-inherited sites are CE-72's. |
| Web vs server clock | audit/+page.svelte:400-405 | Samples are ordered by the server's `at`. The browser clock is used only for the page's own read time. |

**Tally:** 4 new (0 high, 0 medium, 4 low): CE-84, CE-85, CE-86, CE-87. Nothing was assigned for verification in this pass.

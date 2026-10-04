# Crash and edge-input audit, pass 7: calendar and session edges

**Verdict at `final/all-fixes-zero` 1f4de71:** 4 new findings (2 medium, 2 low), CE-52 to CE-55. The two mediums are in the CE-14/CE-43 class:
- CE-52: two session authorities disagree about 2021-02-24.
- CE-53: a Muhurat-only holiday is accepted as a contract expiry.

CE-43 is still NOT FIXED. crates/api/src/server.rs:14472-14474 is still `expiry_of(..).is_ok()`, and it is not re-reported here. A probe was run in a throwaway worktree (since removed). Its output is quoted under each finding.

---

### CE-52 (medium): ingest drops the 2021-02-24 reopening (15:45-16:59), but the cash and F&O gap audits count those 75 minutes as `vendor-hole`. Two session authorities disagree, and no re-pull can fill the gap.

**Sites, side 1 (the calendar owes the reopening):**
- crates/pull/src/calendar.rs:285-296: `IRREGULAR[0]` is `(SYSTEMS_OUTAGE_DAY, windows [555..=699], [945..=1019])`, which is 220 bars.
- crates/pull/src/calendar.rs:1338-1344: `Calendar::from_observed` forces the same session onto every derived calendar.
- crates/pull/src/fold.rs:1057-1059: `minute_session` returns that exceptional calendar shape unchanged.
- crates/pull/src/gaps.rs:356-363 (`cash_kind`) keeps it for a 2021 cash day.
- crates/pull/src/gaps.rs:416-420 (Derivatives) keeps it as well.
- crates/pull/src/gaps.rs:447-465: every missing minute inside a window is pushed as `Reason::VendorHole`.

**Sites, side 2 (ingest drops it):**
- crates/pull/src/session.rs:1133-1143, `Window::verdict`, decides the session from `venue.hours_on(day)` alone.
- crates/pull/src/vendor.rs:1432-1513: all three venue tables say 09:15-15:30 before 2026-08-03. They have no row for the outage day.
- crates/pull/src/fetch.rs:823-840 and the "AND NOW DROPPED, WHICHEVER KIND IT IS" branch below it drop every `AtOrAfterSessionClose` row.
- crates/pull/src/ingest.rs:1491-1494 (`keep_in_session`) does the same on the rolling path.

**Why it is wrong:**
- Ingest guarantees that a bar at 15:45-16:59 IST on 2021-02-24 is never stored, for every venue.
- The gap audit for any NSE cash stock or F&O series still expects those 75 minutes and labels them `vendor-hole`.
- docs/06-limits.md §110 says so explicitly: "CASH and derivative audits continue to use the verified 220-minute exchange session".
- indicators/src/evaluator.rs:282-288 records the opposite fact: the reopening "is entirely outside the pull's [09:15, 15:30) window, so ingest drops it -- correctly, and permanently: re-pulling cannot recover a bar the window excludes".
- So the operator sees a loss pinned on the vendor that this build itself causes. Recovery can never close it. The "one authority" of CLAUDE.md §5 is split: `pull::calendar` says owed, and `vendor::Venue::hours_on` says out of session.
- The same split explains the evening Muhurats of 2020-2024 (18:00/18:15-19:15). calendar.rs:243-247 and :324-331 say those days lack minute bars because "Zerodha's minute history not reaching them". But `land` drops any evening Muhurat bar a vendor serves, so the store cannot support that attribution. evaluator.rs:221-226 says the pull drops them.

**Repro: ran.** The throwaway test calls `Window::new(2021-02-24, 2021-02-24).verdict(secs, Cadence::Minute, venue)`:
- Minute 600 gives `Ok(None)` for NseIndex, NseCash and NseDerivatives.
- Minutes 945 and 1019 give `Ok(Some(AtOrAfterSessionClose))` for all three venues.
- `calendar::kind_of(18682)` gives `Open(Session { windows: [555..=699, 945..=1019], count: 2 })`.
- The repo's own test `the_outage_day_separates_market_minutes_from_unverified_index_bars` (gaps.rs:948) asserts `classify_against(..).lost_minutes() == 166`. Those 166 include the 75 reopening minutes.

**Minimal fix (either one):**
- (a) Make `Window::verdict` consult `calendar::kind_of` first: for an exceptional `Open(session)` day, `session.expects(minute)` decides. A test pins that 15:45 on 2021-02-24 is kept for cash and F&O.
- (b) If dropping is the intended policy, make `classify_against` / `classify_derivative_against` mark the outside-venue-hours windows of an exceptional day `Unmeasured`, and correct §110 and the LENGTH_UNMEASURED doc.

Either way, record a D-entry.

---

### CE-53 (medium): an expiry that falls on a Muhurat-only holiday is accepted. Contracts are filed under a day with no regular session, and the tenor is wrong. This is the CE-14 class, through the gap its fix left open.

**Site:** crates/pull/src/rolling.rs:185-195.

```rust
if matches!(
    crate::calendar::kind_of(i64::from(settled.ordinal())),
    crate::calendar::DayKind::Closed
) {
```

**Why it is wrong:**
- Only `Closed` is refused. Diwali days with only a Muhurat session are `OpenLengthUnmeasured` (2020-2024) or `Open(irregular 13:45-14:44)` (2025-10-21). calendar.rs:304-305 itself calls 2025-10-21 "a weekday the market was otherwise shut", and indicators::evaluator lists all of them as non-regular.
- On those days no regular contract expires in a regular session. The weekly that would have expired there is not settled at a 15:30 close on that day.
- `expiry_of` still returns the day, so:
  - `rolling_key` (api/src/server.rs:14063) files every bar of that week under it;
  - `Tenor::between` (pull/src/tenor.rs:241-245) prices it to a 15:30 close on that day from `Venue::NseDerivatives.hours_on`.
- There is no refusal and no event: CE-14's silent wrong-expiry-key outcome, for exactly the days CE-14's fix did not cover.
- As with CE-14, the move-to-the-previous-day rule is not charter-sourced, so the fix must refuse, not step back.

**Repro: ran** (throwaway test, Dhan spec, code "1", WEEK):
- NIFTY from 2021-11-01: `Ok(Expiry 2021-11-04)`. BANKNIFTY from 2021-11-01: `Ok(Expiry 2021-11-04)`.
- `kind_of(18935)` = `OpenLengthUnmeasured`.
- NIFTY from 2025-10-20 and from 2025-10-21: `Ok(Expiry 2025-10-21)`.
- `kind_of(20382)` = `Open(Session { windows: [825..=884], count: 1 })`.

**Minimal fix:**
- Refuse unless `kind_of(settled) == DayKind::Open(Session::full())`. Today that refuses only the Muhurat days and leaves past-`LAST_DAY` (`Unmeasured`) as the documented pass-through.
- Add a test for 2021-11-04 and 2025-10-21.
- Until CE-43 is fixed, this new refusal also becomes a silent cadence/chunk skip through `cadence_has_contracts_on`, so land it with or after CE-43.

---

### CE-54 (low): the rolling pre-flight checks only the chunk's last day, so one closed-day expiry, or the BANKNIFTY weekly withdrawal, inside a 45-day chunk refuses the whole chunk, permanently

**Site:** crates/api/src/server.rs:12782-12783.

```rust
pull::rolling::expiry_of(asked.underlying.as_str(), &rolling, flag, code, window.to())
    .map_err(|why| format!("{label}: {why}"))?;
```

`window` here is one chunk of `max_days_per_call: 45` (vendor.rs:4326).

**Why it is wrong:**
- The pre-flight is meant to ask "does a regime exist" (:12772-12780). Because it asks on the chunk's end, it fails for the whole chunk whenever the end alone hits a CE-14 closed expiry, or lies past the 2024-11-13 BANKNIFTY withdrawal.
- The doc at :14448-14457 says the per-chunk test exists precisely because "that boundary falls inside a chunk". The pre-flight then throws that chunk away, including its 5-6 good weekly contracts.
- The failure is loud: it is counted in `failed` with the reason. But the reason names the end-week, not the lost weeks. Chunk boundaries are fixed by the window, so re-running the same window loses the same weeks again.
- Per-bar `rolling_key` (:14063) already refuses exactly the bad run, so the pre-flight adds nothing except the over-reach.

**Repro: ran** (`expiry_of` premise; api path not run):
- NIFTY WEEK from 2024-08-14 gives `Err(.."marks closed"..)`.
- BANKNIFTY WEEK from 2024-11-20 gives `Err(.."withdrawn"..)`, while from 2024-10-07 it gives `Ok(2024-10-09)`.
- So a chunk 2024-07-01..2024-08-14 (NIFTY) or 2024-10-07..2024-11-20 (BANKNIFTY) is refused whole.

**Minimal fix:** pre-flight on `window.from()` with the same distinct-outcome rule CE-43's fix introduces, or drop the pre-flight and let `rolling_key` refuse per run.

---

### CE-55 (low): a second, unpinned copy of the non-regular-day calendar lives in `indicators`

**Sites:**
- crates/indicators/src/evaluator.rs:245-303 `CHARTER_NON_REGULAR_IST_DAYS` (9 days) and `Calendar::charter()` :337-345.
- The pull copy is crates/pull/src/calendar.rs:279-338 `IRREGULAR` (4) plus `LENGTH_UNMEASURED` (5).

**Why it is wrong:**
- CLAUDE.md §5 names `pull::calendar` as the one session authority.
- The two sets agree today: 18580, 18682, 18935, 19289, 19673, 19784, 19861, 20028, 20382. But nothing compares them.
- `indicators` cannot depend on `pull` (gate 22), so the copy is structurally needed. `cli` holds both and uses them together (cli/src/index_consistency.rs:1209-1215 and :1561-1567), and no test there pins them equal.
- Adding the next Muhurat to one list and not the other silently changes either `Eligible`/`Excluded` or the anchor exclusion, with no build failure.

**Repro:** not run (source comparison).

**Minimal fix:** a `cli` test asserting `{d : kind_of(d) is Open(non-full) or OpenLengthUnmeasured} == CHARTER_NON_REGULAR_IST_DAYS` over `FIRST_DAY..=LAST_DAY`.

---

## Sites checked, and why each holds

**Calendar primitives:**
- **pull::calendar::kind_of / expected_bars / sessions_between.** These hold:
  - Range-checked, so out of range is `Unmeasured`, never `Closed`.
  - `sessions_between` refuses partial or inverted ranges with `None`.
  - Muhurat days count as sessions.
- **Derived `Calendar` (from_observed, withhold_closed, kind_of, last_day).** Checked arithmetic; out of span is `Unmeasured`. api calendar_of withholds months whose daily rung was not read (D-1443). An empty daily file is "proved" and its in-span days become `Closed`, but `/gaps.json` uses a peer UNION (`agree`), so one feed's daily hole cannot close a day for everyone.
- **`Runtime::kind_of`** always returns the static answer, so observation never overrides the table.

**Day and window types (pull::session):**
- **Day::new / succ / months_before / end_of_month / from_days.** Leap day 2024-02-29 is computed, not tabulated. Year rollover is tested. `months_before` clamps 31 May less 3 months to the last day of February. `succ` past 9999-12-31 is a named `NoNextDay`.
- **Window::new.** `from > to` is a named `WindowRunsBackwards`, and `from == to` is legal. `wire_to = to.succ()` covers the vendor's exclusive `toDate`. `verdict` drops the extra day as `AfterWindow`.
- **IST conversion.** `IstMoment::from_epoch_secs` refuses before 1970-01-01 00:00 IST, not on the sign of the input. These all use Euclidean division and are correct at the UTC midnight / 05:30 IST boundaries:
  - pull gaps.rs:214;
  - api autopilot `day_of`;
  - runner `exact_ist_day`, `actual_ist_day`, `ist_minute_of_day`;
  - expression_validation `day`;
  - resample.
- **api `day_window_bounds`** (server.rs:2122-2130). A reversed range is refused. The inclusive `to` becomes a half-open `+1 day`, and 9999-12-31 fits in i64 micros.

**Session minutes:**
- **First and last minute.** 09:15 is minute 555 and the last bar opens at 15:29. The close is exclusive (`>= close_minute` is `AtOrAfterSessionClose`), and `FULL_BARS == 375` is const-asserted. After 2026-08-03 the derivatives close is 15:40 from the table, and fold and gaps read it through `minute_session` (D-1529).
- **runner SessionBounds / FORCED_EXIT_MINUTE 15:10.** A non-regular day without a unique 15:09 bar drops overrun trades. This is documented policy (outcome.rs:60-69, :99-101), not silent.
- **request_minutes.rs:68-90.** Exceptional and unmeasured days are named "request minute coverage UNVERIFIED" and are not counted complete.
- **fold.rs:880-930.** Exceptional days keep their calendar shape. Absent buckets and tails become named diagnostics, and a `minute_session` refusal is "withheld".

**Cash eligibility after the 2026-08-03 auction change:**
- **cash_session_cache::is_required.** A day after `LAST_DAY` (2026-09-04) gets a loud `Err("UNVERIFIED NSE cash session ... (Unmeasured)")`, never a skip.
- **api `audit_cash_schedule_window`** (server.rs:3082-3099). It filters to static full sessions, and a missing schedule surfaces downstream as `Unmeasured` through `cash_kind`.

**Expiry computation:**
- **costs::expiry.** Regime rows hold:
  - NIFTY: weekly Thursday, then Tuesday from 2025-09-02.
  - BANKNIFTY: weekly Wednesday from 2023-09-04, withdrawn from 2024-11-14.
  - Monthly: re-read per resolved month at day 15, which handles the December to January roll. Leap February is tested (2024-02-29).
  - Before 2020-01-01: refused as `Unverified`.
- Holidays are the caller's job, and are covered by CE-14, CE-43 and CE-53.

**Recovery and web:**
- **api recovery.rs:1290-1300 and :1517-1540.** A closed day holding a daily bar is a named error. `Unmeasured` and `OpenLengthUnmeasured` count as `unverified`, not held.
- **web ingest isSession / holidaysKnownFor.** Inside the derived span the owed list decides. Outside it, or on a withheld day, the page assumes weekday = session and says so in `calendarGapFor`. Weekday is computed from `Date.UTC`, and dates.js uses one `Asia/Kolkata` formatter.
- **cli range parsers** (lib.rs:1649, stored.rs:2587, pool.rs:2170, and the boolean_* commands). A backwards range or month order is refused by name.

**Static calendar end date:**
- `LAST_DAY` is 2026-09-04, a month before today.
- `/gaps.json` reports `stale: true` when the table answered.
- `expiry_of` passes past-table days through, as documented.
- Every other static-table consumer turns `Unmeasured` into a named refusal or an `unverified` count. None of them reads it as `Closed`.

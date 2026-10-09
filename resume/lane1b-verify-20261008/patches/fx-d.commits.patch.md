From 66a4fc37ef327dc0a4fcd6a8f449a529fa67795c Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:41:36 +0000
Subject: [PATCH 1/8] cli: judge a cash share's Boolean calendar against its
 dated close (G3-1)

Before: Boolean research admits NSE cash shares, but prepare_column built
both calendar receipts with calendar_receipt_v2_for_bars, which expects the
index session (last minute 15:29) on every day. From 2026-08-03 an eligible
share's minutes end at 15:14 on closing-auction days, so any span holding
one refused as an incomplete store. The ORB/GapFib overlay asked
nse_session_close_minute too. D-2102 said no share reaches that overlay.

After: stored::calendar_receipt_v2_for_venue_bars takes the share's
CashCloses. On a CAS-dated day the expected last window ends at the dated
close (tag 6). An unread close refuses as "cash session close UNVERIFIED"
when a bar is offered and is Unmeasured (tag 7) when none is. A bar past the
dated close refuses as contradicting the master. Receipts built with closes
are policy 4 (CALENDAR_RECEIPT_POLICY_V2_DATED_CASH). Index receipts are
policy 3 and byte-identical. The Boolean path threads the closes into both
receipts, require_exact_calendar_for_venue and
build_candidate_columns_for_venue, and binds the closes' digest into its
source identity. D-4749 corrects D-2102.

Fail before:
  boolean_candidate_tests.rs:686: Err("calendar receipt V2 for 60 seconds
  over IST days 20666..=20696 is not complete: status Incomplete, offered
  7560, expected 7875, missing 315, unexpected 0")

D-4748, D-4749, L1FD-01.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/boolean_candidate_tests.rs | 170 +++++++++++++++
 crates/cli/src/boolean_candidate_v1.rs    |  34 ++-
 crates/cli/src/candidate_universe.rs      |  51 ++++-
 crates/cli/src/stored.rs                  | 245 ++++++++++++++++++++--
 docs/04-invariants.md                     |   6 +
 docs/05-decisions.md                      |  79 +++++++
 docs/06-limits.md                         |  13 ++
 7 files changed, 576 insertions(+), 22 deletions(-)

diff --git a/crates/cli/src/boolean_candidate_tests.rs b/crates/cli/src/boolean_candidate_tests.rs
index 59555dd4..e6220fcd 100644
--- a/crates/cli/src/boolean_candidate_tests.rs
+++ b/crates/cli/src/boolean_candidate_tests.rs
@@ -544,6 +544,176 @@ fn complete_generated_months_measure_actual_accepted_periods_before_stats_fixtur
     Ok(())
 }
 
+/// Writes 2026-07 and 2026-08 for `name`: every measured open day's minutes,
+/// each day's last minute at `last(day)`, one daily bar per day, and, when
+/// `flag` is given, one receipted session master per closing-auction full
+/// session naming `name` with that flag, except the `skip`th. Returns the CAS
+/// days a master was due for. Generated, never market evidence.
+fn prepare_cas_span(
+    fixture: &Fixture,
+    name: &str,
+    last: impl Fn(i64) -> u16,
+    flag: Option<u8>,
+    skip: Option<usize>,
+) -> Result<Vec<i64>, String> {
+    use std::io::Write as _;
+    let key = crate::stored::swept_index(name)?;
+    let symbol = u32::from_le_bytes(
+        brutex_core::universe::fnv1a(name)
+            .to_le_bytes()
+            .get(..4)
+            .ok_or("fixture symbol prefix")?
+            .try_into()
+            .map_err(|_| "fixture symbol width")?,
+    );
+    let mut cas_days = Vec::new();
+    for month in [7, 8] {
+        let mut minutes = Vec::new();
+        let mut daily = Vec::new();
+        for day in 1..=31 {
+            let Ok(date) = pull::session::Day::new(2026, month, day) else {
+                continue;
+            };
+            let stamp = i64::from(date.days_from_epoch());
+            let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(stamp) else {
+                continue;
+            };
+            if indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&stamp) {
+                continue;
+            }
+            if pull::vendor::cash_auction_eligibility_required(date)
+                && session == pull::calendar::Session::full()
+            {
+                cas_days.push(stamp);
+            }
+            let start = minutes.len();
+            for minute in 555..=last(stamp) {
+                let open = 2_000_000 + i64::from((minute + u16::from(day)) % 31) * 100;
+                minutes.push(Bar {
+                    ts_micros: stamp * 86_400_000_000 + i64::from(minute) * 60_000_000
+                        - indicators::IST_OFFSET_MICROS,
+                    open,
+                    high: open + 1000,
+                    low: open - 900,
+                    close: open + if minute % 3 == 0 { -50 } else { 50 },
+                    volume: 100,
+                    open_interest: i64::MIN,
+                });
+            }
+            daily.push(aggregate(
+                minutes.get(start..).ok_or("fixture session slice")?,
+            )?);
+        }
+        for (rung, bars) in [
+            (Timeframe::MINUTE_1, minutes.as_slice()),
+            (Timeframe::DAY_1, daily.as_slice()),
+        ] {
+            let path = StorePath::for_key(
+                brutex_core::vendor::Vendor::Zerodha,
+                &key,
+                rung,
+                YearMonth::new(2026, month).map_err(display)?,
+                FileKind::Bars,
+            )
+            .map_err(display)?;
+            let mut writer =
+                BarFile::open_or_create(&fixture.root, path, symbol).map_err(display)?;
+            writer.append(bars).map_err(display)?;
+        }
+    }
+    if let Some(flag) = flag {
+        let isin = brutex_core::universe::nse_isin(name).ok_or("fixture share has no ISIN")?;
+        for (index, day) in cas_days.iter().enumerate() {
+            if skip == Some(index) {
+                continue;
+            }
+            let csv = format!(
+                "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,{name},EQ,{},{flag}\n",
+                isin.as_str()
+            );
+            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
+            gz.write_all(csv.as_bytes()).map_err(display)?;
+            let civil = pull::session::Day::from_days(u32::try_from(*day).map_err(display)?)
+                .map_err(display)?;
+            pull::cash_session_cache::install(
+                &fixture.root.join("session-masters"),
+                civil,
+                &gz.finish().map_err(display)?,
+            )?;
+        }
+    }
+    Ok(cas_days)
+}
+
+/// G3-1 / GAP12-6 (D-4748): the Boolean research path judges each venue's
+/// calendar against ITS close, from the one dated authority the stored path
+/// already reads (`stored::CashCloses`), with no second calendar.
+///
+/// From 2026-08-03 an eligible share's continuous session ends at 15:14 on
+/// every closing-auction day, so its complete August used to be refused as an
+/// incomplete calendar against the index's 15:29. On the SAME days the index
+/// keeps its own close; an ineligible share keeps 15:29; a day whose master is
+/// absent refuses by naming the unverified close, never as a holed store; and
+/// a share whose store runs past the dated close contradicts its master and
+/// refuses rather than being certified complete.
+#[test]
+fn a_cas_share_and_an_index_on_the_same_days_are_each_judged_against_their_own_close()
+-> Result<(), String> {
+    const ELIGIBLE_CLOSE: u16 = 914;
+    const FULL_CLOSE: u16 = 929;
+    let long = policy(Side::Long)?;
+    let short = policy(Side::Short)?;
+    let catalog = programs()?;
+    let dated = |close: u16| move |day: i64| if day >= 20_668 { close } else { FULL_CLOSE };
+    let run = |name: &str,
+               close: u16,
+               flag: Option<u8>,
+               skip: Option<usize>|
+     -> Result<Result<usize, String>, String> {
+        let fixture = Fixture::new()?;
+        let cas_days = prepare_cas_span(&fixture, name, dated(close), flag, skip)?;
+        assert!(
+            cas_days.len() > 10,
+            "fixture premise: August 2026 is CAS-era"
+        );
+        let config = config(&fixture.root)?;
+        let mut request = fixture.request(name, &catalog, &config, &long, &short)?;
+        request.from = (2026, 8);
+        request.to = (2026, 8);
+        let source = load_source(&request)?;
+        Ok(prepare_column(&request, &source).map(|(_, sessions)| sessions.days.len()))
+    };
+
+    let share = run("RELIANCE", ELIGIBLE_CLOSE, Some(1), None)?;
+    eprintln!("eligible share ending 15:14 on CAS days: {share:?}");
+    assert!(share.as_ref().is_ok_and(|days| *days > 0), "{share:?}");
+
+    let index = run("NIFTY", FULL_CLOSE, None, None)?;
+    assert_eq!(index.as_ref().ok(), share.as_ref().ok(), "{index:?}");
+
+    let ineligible = run("RELIANCE", FULL_CLOSE, Some(0), None)?;
+    assert_eq!(
+        ineligible.as_ref().ok(),
+        share.as_ref().ok(),
+        "{ineligible:?}"
+    );
+
+    let unverified = run("RELIANCE", ELIGIBLE_CLOSE, Some(1), Some(3))?;
+    eprintln!("eligible share with one master absent: {unverified:?}");
+    let why = unverified.err().unwrap_or_default();
+    assert!(why.contains("cash session close UNVERIFIED"), "{why}");
+    assert!(!why.contains("is not complete"), "{why}");
+
+    let contradicted = run("RELIANCE", FULL_CLOSE, Some(1), None)?;
+    eprintln!("eligible share whose store runs to 15:29: {contradicted:?}");
+    let why = contradicted.err().unwrap_or_default();
+    assert!(
+        why.contains("past this share's dated session close"),
+        "{why}"
+    );
+    Ok(())
+}
+
 /// D-1143, source shape: `produce_side` runs once per program and side and
 /// must seal its run against the catalogue's hoisted digests rather than hash
 /// the signal, minute and daily bars again. The answers are byte-identical, so
diff --git a/crates/cli/src/boolean_candidate_v1.rs b/crates/cli/src/boolean_candidate_v1.rs
index f4dd1dc7..00493565 100644
--- a/crates/cli/src/boolean_candidate_v1.rs
+++ b/crates/cli/src/boolean_candidate_v1.rs
@@ -423,6 +423,13 @@ fn load_source(request: &Request<'_>) -> Result<Source, String> {
     binding.update(b"brutex-boolean-strict-input-policy-v1\0");
     binding.update(&request.inputs.max_bytes().to_le_bytes());
     binding.update(&request.inputs.max_records().to_le_bytes());
+    // A SHARE'S DATED CLOSES ARE A SOURCE (G3-1, D-4748): its receipts and
+    // overlay are judged against them, so the same bars beside a different
+    // master are a different source. An index holds none and is unchanged.
+    if let Some(cash) = data.exact_minute.cash_digest() {
+        binding.update(b"brutex-boolean-cash-session-closes-v1\0");
+        binding.update(&cash);
+    }
     let identity = guard.bind_digest(binding.finalize());
     guard.require_current()?;
     Ok(Source {
@@ -563,11 +570,21 @@ fn prepare_column(request: &Request<'_>, source: &Source) -> Result<(Column, Ses
     let (first, last) = super::requested_span_days(span)?;
     let rung = u32::try_from(crate::stored::rung_length_micros(request.rung)? / 1_000_000)
         .map_err(display)?;
-    let signal_calendar =
-        crate::stored::calendar_receipt_v2_for_bars(&data.signal.bars, rung, first, last)?
-            .require_complete()?;
+    // EACH VENUE AGAINST ITS OWN CLOSE (G3-1, D-4748). A cash share's minutes
+    // end at its dated close on a closing-auction day, so both receipts, their
+    // exact rebuilds and the overlay ask the closes `RangeInputs` already
+    // loaded, never the index's 15:29. An index holds none: `None`, unchanged.
+    let cash = data.exact_minute.cash.as_ref();
+    let signal_calendar = crate::stored::calendar_receipt_v2_for_venue_bars(
+        &data.signal.bars,
+        rung,
+        first,
+        last,
+        cash,
+    )?
+    .require_complete()?;
     let execution_calendar =
-        crate::stored::calendar_receipt_v2_for_bars(execution, 60, first, last)?
+        crate::stored::calendar_receipt_v2_for_venue_bars(execution, 60, first, last, cash)?
             .require_complete()?;
     let coverage = super::CandidateCalendarCoverageV1::from_complete(
         rung,
@@ -575,32 +592,35 @@ fn prepare_column(request: &Request<'_>, source: &Source) -> Result<(Column, Ses
         signal_calendar,
         execution_calendar,
     )?;
-    super::require_exact_calendar(
+    super::require_exact_calendar_for_venue(
         "Boolean signal",
         &data.signal.bars,
         rung,
         coverage,
         signal_calendar,
+        cash,
     )?;
-    super::require_exact_calendar(
+    super::require_exact_calendar_for_venue(
         "Boolean execution",
         execution,
         60,
         coverage,
         execution_calendar,
+        cash,
     )?;
     let evaluation = super::CandidateEvaluationInputsV1 {
         widths: request.widths,
         availability: crate::stored::vwap_availability(&data.signal.key),
         thresholds: request.thresholds,
     };
-    let (_, column) = super::build_candidate_columns(
+    let (_, column) = super::build_candidate_columns_for_venue(
         &data.signal.bars,
         &data.daily.references,
         &data.exact_minute.bars,
         execution,
         rung,
         &evaluation,
+        cash,
     )?;
     let session_index = Sessions::new(execution, &column, first, last)?;
     Ok((column, session_index))
diff --git a/crates/cli/src/candidate_universe.rs b/crates/cli/src/candidate_universe.rs
index 2bd12520..210e9bd9 100644
--- a/crates/cli/src/candidate_universe.rs
+++ b/crates/cli/src/candidate_universe.rs
@@ -1604,6 +1604,7 @@ impl CandidateSearchColumnBuilderV1<'_> {
                 })?,
             self.rung_seconds,
             &self.evaluation,
+            None,
         )
     }
 }
@@ -1615,6 +1616,31 @@ pub(crate) fn build_candidate_columns(
     execution_bars: &[Candle],
     rung_seconds: u32,
     evaluation: &CandidateEvaluationInputsV1,
+) -> Result<(Column, Column), CandidateUniverseRefusal> {
+    build_candidate_columns_for_venue(
+        signal_bars,
+        daily_references,
+        reference_minute_context,
+        execution_bars,
+        rung_seconds,
+        evaluation,
+        None,
+    )
+}
+
+/// [`build_candidate_columns`] for one venue: `cash` is a cash share's dated
+/// closes, which the exact-minute overlay asks through
+/// [`crate::stored::session_close_for`], and `None` is the index calendar
+/// (G3-1, D-4748). Only Boolean research reaches it with a share; every
+/// index-only caller keeps [`build_candidate_columns`].
+pub(crate) fn build_candidate_columns_for_venue(
+    signal_bars: &[Candle],
+    daily_references: &[DailyReference],
+    reference_minute_context: &[Candle],
+    execution_bars: &[Candle],
+    rung_seconds: u32,
+    evaluation: &CandidateEvaluationInputsV1,
+    cash: Option<&crate::stored::CashCloses>,
 ) -> Result<(Column, Column), CandidateUniverseRefusal> {
     let signal_column = build_candidate_signal_column(
         signal_bars,
@@ -1622,6 +1648,7 @@ pub(crate) fn build_candidate_columns(
         reference_minute_context,
         rung_seconds,
         evaluation,
+        cash,
     )?;
     let signal_length_micros = signal_length_micros(rung_seconds)?;
     let alignment = runner::align::onto_execution(
@@ -1670,6 +1697,7 @@ pub(crate) fn candidate_signal_swept_v1(
         reference_minute_context,
         rung_seconds,
         evaluation,
+        None,
     )
     .map(|column| column.census().swept)
 }
@@ -1681,12 +1709,17 @@ pub(crate) fn candidate_signal_swept_v1(
 /// Search V4 successor: every causal prefix must use the same evaluator and
 /// overlay semantics as the complete Candidate column, not a second closure
 /// that merely happens to emit compatible masks.
+///
+/// `cash` is a cash share's dated closes, or `None` for the index calendar.
+/// The overlay asks [`crate::stored::session_close_for`], the one close rule
+/// the stored overlay and minute-gap census ask (G3-1, D-4748).
 fn build_candidate_signal_column(
     signal_bars: &[Candle],
     daily_references: &[DailyReference],
     reference_minute_context: &[Candle],
     rung_seconds: u32,
     evaluation: &CandidateEvaluationInputsV1,
+    cash: Option<&crate::stored::CashCloses>,
 ) -> Result<Column, CandidateUniverseRefusal> {
     let mut evaluator = AnchoredEvaluator::new(
         evaluation.widths,
@@ -1716,7 +1749,7 @@ fn build_candidate_signal_column(
         signal_length_micros,
         evaluation.widths,
         Calendar::charter(),
-        crate::stored::nse_session_close_minute,
+        |day| crate::stored::session_close_for(cash, day),
         &mut signal_column,
     )
     .map_err(|why| {
@@ -4234,11 +4267,25 @@ pub(crate) fn require_exact_calendar(
     coverage: CandidateCalendarCoverageV1,
     offered: CompleteCalendarReceiptV2,
 ) -> Result<(), CandidateUniverseRefusal> {
-    let exact = crate::stored::calendar_receipt_v2_for_bars(
+    require_exact_calendar_for_venue(name, bars, rung_seconds, coverage, offered, None)
+}
+
+/// [`require_exact_calendar`] for one venue, rebuilding the receipt with the
+/// same closes the offered receipt was built with (G3-1, D-4748).
+pub(crate) fn require_exact_calendar_for_venue(
+    name: &str,
+    bars: &[Candle],
+    rung_seconds: u32,
+    coverage: CandidateCalendarCoverageV1,
+    offered: CompleteCalendarReceiptV2,
+    cash: Option<&crate::stored::CashCloses>,
+) -> Result<(), CandidateUniverseRefusal> {
+    let exact = crate::stored::calendar_receipt_v2_for_venue_bars(
         bars,
         rung_seconds,
         coverage.first_day,
         coverage.last_day,
+        cash,
     )
     .and_then(crate::stored::CalendarReceiptV2::require_complete)
     .map_err(|why| format!("candidate {name} exact complete calendar refused: {why}"))?;
diff --git a/crates/cli/src/stored.rs b/crates/cli/src/stored.rs
index 06143830..35b51f1a 100644
--- a/crates/cli/src/stored.rs
+++ b/crates/cli/src/stored.rs
@@ -931,6 +931,20 @@ pub const CALENDAR_RECEIPT_SCHEMA_V2: u32 = 2;
 /// digest — the defect the whole receipt exists to prevent.
 pub const CALENDAR_RECEIPT_POLICY_V2: u32 = 3;
 
+/// Policy version of a V2 receipt built against a cash share's dated closes
+/// (G3-1, D-4748).
+///
+/// Policy 3, plus one rule: on a day [`CashCloses`] dates (a closing-auction
+/// full session from 2026-08-03), the expected session's last window ends at
+/// the share's dated close, the day hashes tag 6 instead of 1, and its dated
+/// windows are hashed in place of the calendar's. A dated day whose close was
+/// not read refuses when a bar is offered on it and is `Unmeasured` (tag 7)
+/// when none is. Every receipt built from [`calendar_receipt_v2_for_venue_bars`]
+/// with closes carries this version, pre-CAS spans included, so a receipt
+/// judged against a share's master never shares a version with one judged
+/// against the index calendar (§3 rule 8). An index receipt is policy 3.
+pub const CALENDAR_RECEIPT_POLICY_V2_DATED_CASH: u32 = 4;
+
 const NSE_OPEN_MINUTE_V2: i64 = 555;
 const CALENDAR_POLICY_DIGEST_DOMAIN_V2: &[u8] = b"brutex.calendar-policy.v2\0";
 const CALENDAR_POLICY_RUNGS_V2: [u32; 8] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];
@@ -1492,12 +1506,68 @@ fn expected_buckets_v2(
     Ok(expected)
 }
 
+/// The session a V2 receipt expects on one open day for one venue (G3-1,
+/// D-4748).
+#[derive(Clone, Copy)]
+enum ReceiptSessionV2 {
+    /// The calendar's own session: an index, or a share off a dated day.
+    Calendar(Session),
+    /// A share's dated session: the calendar's, its final window ending at
+    /// the dated close carried beside it.
+    Dated(Session, u16),
+    /// A dated day whose close the share's closes do not hold.
+    Unread,
+}
+
+/// What a receipt over `closes`' venue expects on open `day`, from the one
+/// dated authority the stored read asks ([`CashCloses::session_close_minute`]),
+/// so the receipt, the overlay and the minute-gap census cannot disagree about
+/// a share's close. `None` is the index calendar, unchanged. O(1): one civil
+/// conversion, and on a dated day one close lookup.
+fn receipt_session_v2(
+    day: i64,
+    session: Session,
+    closes: Option<&CashCloses>,
+) -> Result<ReceiptSessionV2, Refusal> {
+    let Some(closes) = closes.filter(|_| cas_dated_day(day).is_some()) else {
+        return Ok(ReceiptSessionV2::Calendar(session));
+    };
+    let Some(close) = closes.session_close_minute(day) else {
+        return Ok(ReceiptSessionV2::Unread);
+    };
+    let mut dated = session;
+    let window = usize::from(dated.count)
+        .checked_sub(1)
+        .and_then(|last| dated.windows.get_mut(last))
+        .filter(|window| window.from <= close && close <= window.to)
+        .ok_or_else(|| {
+            format!(
+                "calendar receipt V2: the dated session close at IST minute {close} on IST day {day} lies outside the calendar's final window"
+            )
+        })?;
+    window.to = close;
+    Ok(ReceiptSessionV2::Dated(dated, close))
+}
+
+/// The refusal for a bar offered on a dated day whose close was not read.
+fn unread_close_v2(day: i64, closes: Option<&CashCloses>) -> Refusal {
+    format!(
+        "cash session close UNVERIFIED: dated CAS eligibility required for IST day {day} ({}); the calendar receipt cannot expect a session close it does not hold, and a share's day is never judged against the index's",
+        closes
+            .and_then(|closes| closes.unverified_reason(day))
+            .unwrap_or(
+                "no master was read for it, and no charter source gives a share's close on a session that is not full"
+            )
+    )
+}
+
 /// Validate and hash every offered rung timestamp without a membership set.
 fn hash_offered_calendar_v2<I>(
     timestamps: I,
     rung_minutes: i64,
     first_day: i64,
     last_day: i64,
+    closes: Option<&CashCloses>,
     hasher: &mut brutex_core::blake3::Hasher,
 ) -> Result<OfferedCalendarFactsV2, Refusal>
 where
@@ -1551,7 +1621,17 @@ where
         }
         let decision = match pull::calendar::kind_of(day) {
             DayKind::Open(session) => {
+                let (session, dated_close) = match receipt_session_v2(day, session, closes)? {
+                    ReceiptSessionV2::Calendar(session) => (session, None),
+                    ReceiptSessionV2::Dated(session, close) => (session, Some(close)),
+                    ReceiptSessionV2::Unread => return Err(unread_close_v2(day, closes)),
+                };
                 let expected = expected_buckets_v2(day, session, rung_minutes)?;
+                if let Some(close) = dated_close.filter(|_| !expected.contains(bucket)) {
+                    return Err(format!(
+                        "calendar receipt V2 timestamp {timestamp} is bucket {bucket} on IST day {day}, past this share's dated session close at IST minute {close} read from its NSE session master: the store contradicts its session master"
+                    ));
+                }
                 if !expected.contains(bucket) {
                     return Err(format!(
                         "calendar receipt V2 timestamp {timestamp} is bucket {bucket} on measured open IST day {day}, but that bucket intersects no measured session window"
@@ -1637,6 +1717,7 @@ fn hash_open_session_v2(
 fn hash_calendar_day_v2(
     day: i64,
     rung_minutes: i64,
+    closes: Option<&CashCloses>,
     hasher: &mut brutex_core::blake3::Hasher,
 ) -> Result<CalendarDayFactsV2, Refusal> {
     hasher.update(b"D");
@@ -1661,14 +1742,37 @@ fn hash_calendar_day_v2(
         });
     }
     match pull::calendar::kind_of(day) {
-        DayKind::Open(session) => {
-            hasher.update(&[1]);
-            Ok(CalendarDayFactsV2 {
-                expected: hash_open_session_v2(day, session, rung_minutes, hasher)?,
-                unmeasured: false,
-                withheld: 0,
-            })
-        }
+        // A SHARE'S DATED DAY HAS ITS OWN TAG (G3-1, D-4748). Its windows are
+        // the dated ones, so tag 1 over them would let a share's 15:14 day and
+        // a hypothetical calendar 15:14 day hash alike. A dated day with no
+        // close read and no bar offered (the offered walk refuses one with a
+        // bar) cannot be measured: tag 7, never the index's 15:29.
+        DayKind::Open(session) => match receipt_session_v2(day, session, closes)? {
+            ReceiptSessionV2::Calendar(session) => {
+                hasher.update(&[1]);
+                Ok(CalendarDayFactsV2 {
+                    expected: hash_open_session_v2(day, session, rung_minutes, hasher)?,
+                    unmeasured: false,
+                    withheld: 0,
+                })
+            }
+            ReceiptSessionV2::Dated(session, _) => {
+                hasher.update(&[6]);
+                Ok(CalendarDayFactsV2 {
+                    expected: hash_open_session_v2(day, session, rung_minutes, hasher)?,
+                    unmeasured: false,
+                    withheld: 0,
+                })
+            }
+            ReceiptSessionV2::Unread => {
+                hasher.update(&[7]);
+                Ok(CalendarDayFactsV2 {
+                    expected: 0,
+                    unmeasured: true,
+                    withheld: 0,
+                })
+            }
+        },
         DayKind::Closed => {
             hasher.update(&[2]);
             Ok(CalendarDayFactsV2 {
@@ -1732,6 +1836,7 @@ pub fn calendar_receipt_v2(
         rung_seconds,
         first_day,
         last_day,
+        None,
     )
 }
 
@@ -1754,20 +1859,63 @@ pub fn calendar_receipt_v2_for_bars(
     rung_seconds: u32,
     first_day: i64,
     last_day: i64,
+) -> Result<CalendarReceiptV2, Refusal> {
+    calendar_receipt_v2_for_venue_bars(bars, rung_seconds, first_day, last_day, None)
+}
+
+/// [`calendar_receipt_v2_for_bars`] for one venue: `cash` is a cash share's
+/// dated closes, `None` the index calendar (G3-1, D-4748).
+///
+/// With `None` the receipt is byte-identical to
+/// [`calendar_receipt_v2_for_bars`], which is this function. With a share's
+/// closes it is policy [`CALENDAR_RECEIPT_POLICY_V2_DATED_CASH`]: each
+/// closing-auction full session from 2026-08-03 expects the share's dated
+/// close, read from the one authority the stored overlay and minute-gap census
+/// ask, never the index's 15:29. A bar on a dated day whose close was not read
+/// refuses as "cash session close UNVERIFIED", and a bar past the dated close
+/// refuses as the store contradicting its session master.
+///
+/// # Cost
+///
+/// O(B + D), as [`calendar_receipt_v2`], plus one O(1) close lookup per
+/// offered bar and per day on a dated day. **UNVERIFIED as a measured bound**,
+/// read off the source.
+///
+/// # Errors
+///
+/// Every refusal of [`calendar_receipt_v2`], and the two above.
+pub fn calendar_receipt_v2_for_venue_bars(
+    bars: &[Candle],
+    rung_seconds: u32,
+    first_day: i64,
+    last_day: i64,
+    cash: Option<&CashCloses>,
 ) -> Result<CalendarReceiptV2, Refusal> {
     calendar_receipt_v2_from_iter(
         bars.iter().map(|bar| bar.ts_micros),
         rung_seconds,
         first_day,
         last_day,
+        cash,
     )
 }
 
+/// The receipt policy a venue's closes select: policy 4 for a cash share's
+/// dated closes, policy 3 for the index calendar (G3-1, D-4748).
+const fn receipt_policy_v2(closes: Option<&CashCloses>) -> u32 {
+    if closes.is_some() {
+        CALENDAR_RECEIPT_POLICY_V2_DATED_CASH
+    } else {
+        CALENDAR_RECEIPT_POLICY_V2
+    }
+}
+
 fn calendar_receipt_v2_from_iter<I>(
     timestamps: I,
     rung_seconds: u32,
     first_day: i64,
     last_day: i64,
+    closes: Option<&CashCloses>,
 ) -> Result<CalendarReceiptV2, Refusal>
 where
     I: IntoIterator<Item = i64>,
@@ -1791,23 +1939,30 @@ where
     }
 
     let mut hasher = brutex_core::blake3::Hasher::new();
+    let policy = receipt_policy_v2(closes);
     hasher.update(b"brutex.calendar-receipt.v2\0");
     hasher.update(&CALENDAR_RECEIPT_SCHEMA_V2.to_le_bytes());
-    hasher.update(&CALENDAR_RECEIPT_POLICY_V2.to_le_bytes());
+    hasher.update(&policy.to_le_bytes());
     hasher.update(&rung_seconds.to_le_bytes());
     hasher.update(&first_day.to_le_bytes());
     hasher.update(&last_day.to_le_bytes());
     hasher.update(&day_span.to_le_bytes());
 
-    let offered =
-        hash_offered_calendar_v2(timestamps, rung_minutes, first_day, last_day, &mut hasher)?;
+    let offered = hash_offered_calendar_v2(
+        timestamps,
+        rung_minutes,
+        first_day,
+        last_day,
+        closes,
+        &mut hasher,
+    )?;
     let mut expected = 0_u64;
     let mut unmeasured = false;
     let mut withheld_buckets = 0_u64;
     let mut withheld_days = 0_u32;
     let mut day = first_day;
     loop {
-        let facts = hash_calendar_day_v2(day, rung_minutes, &mut hasher)?;
+        let facts = hash_calendar_day_v2(day, rung_minutes, closes, &mut hasher)?;
         expected = expected
             .checked_add(facts.expected)
             .ok_or_else(|| "calendar receipt V2 expected bucket count overflowed u64".to_owned())?;
@@ -1857,7 +2012,7 @@ where
 
     Ok(CalendarReceiptV2 {
         schema_version: CALENDAR_RECEIPT_SCHEMA_V2,
-        policy_version: CALENDAR_RECEIPT_POLICY_V2,
+        policy_version: policy,
         rung_seconds,
         first_day,
         last_day,
@@ -5416,6 +5571,70 @@ mod tests {
         }
     }
 
+    /// G3-1, D-4748: a share's V2 receipt is policy 4 and judges a CAS day
+    /// against the share's dated close, never the index's 15:29. Without
+    /// closes the same bars give the index receipt, policy 3, byte-identical
+    /// to `calendar_receipt_v2_for_bars`. A pre-CAS day expects the calendar
+    /// under either policy, yet is not one piece of evidence under both.
+    #[test]
+    fn a_share_receipt_is_policy_four_and_expects_its_dated_close() {
+        let day = OPEN_MONDAY_2026_08_03;
+        let session = |on: i64, last: i64| {
+            (555..=last)
+                .map(|minute| minute_on_ist_day(on, minute, 2_500_000))
+                .collect::<Vec<_>>()
+        };
+        let store = root("receipt-dated");
+        install_master(&store, day, 1);
+        let dated = load_cash_closes(&store, "RELIANCE", [day]).expect("closes load");
+        let unread = load_cash_closes(Path::new(NO_STORE), "RELIANCE", [day]).expect("closes load");
+        let bars = session(day, 914);
+        let receipt = |bars: &[Candle], on: i64, cash: Option<&CashCloses>| {
+            calendar_receipt_v2_for_venue_bars(bars, 60, on, on, cash)
+        };
+        let index = receipt(&bars, day, None).expect("the index receipt");
+        assert_eq!(Ok(index), calendar_receipt_v2_for_bars(&bars, 60, day, day));
+        assert_eq!(index.policy_version(), CALENDAR_RECEIPT_POLICY_V2);
+        assert_eq!(
+            (index.expected(), index.missing(), index.status()),
+            (375, 15, CalendarStatusV1::Incomplete)
+        );
+        let share = receipt(&bars, day, Some(&dated)).expect("the share receipt");
+        assert_eq!(
+            share.policy_version(),
+            CALENDAR_RECEIPT_POLICY_V2_DATED_CASH
+        );
+        assert_eq!(
+            (
+                share.offered(),
+                share.expected(),
+                share.missing(),
+                share.status()
+            ),
+            (360, 360, 0, CalendarStatusV1::Complete)
+        );
+        let pre_cas = accepted_open_before(day);
+        let full = session(pre_cas, 929);
+        let pre_index = receipt(&full, pre_cas, None).expect("a pre-CAS index receipt");
+        let pre_share = receipt(&full, pre_cas, Some(&dated)).expect("a pre-CAS share receipt");
+        assert_eq!(
+            (pre_share.expected(), pre_share.status()),
+            (pre_index.expected(), CalendarStatusV1::Complete)
+        );
+        assert_ne!(pre_share.digest(), pre_index.digest());
+        let empty = receipt(&[], day, Some(&unread)).expect("no bar is offered");
+        assert_eq!(empty.status(), CalendarStatusV1::Unmeasured);
+        let why = receipt(&bars, day, Some(&unread)).expect_err("an unread close refuses");
+        assert!(why.contains("cash session close UNVERIFIED"), "{why}");
+        let why = receipt(&session(day, 929), day, Some(&dated))
+            .expect_err("bars past the dated close contradict the master");
+        assert!(
+            why.contains("past this share's dated session close at IST minute 914"),
+            "{why}"
+        );
+        let _ignored = std::fs::remove_dir_all(&store);
+    }
+
     /// Dated closes answer the calendar off CAS days, the master's close on
     /// them, and `None` without a master; the digest moves with every answer
     /// and every reason, and an index holds no closes at all (D-2102).
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 31e467ea..48ace945 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7052,3 +7052,9 @@ old line regex the same input and watched it pass.
 | G18-api-27 | The seek path's `records unreadable` line names the first file that refused a record, not the first file read (D-2046) | `api::bars::window_tests::the_unreadable_line_names_the_first_damaged_file_not_the_first_file` | ✓ |
 | G18-api-28 | The route test's HTTP exchange is bounded at 30 s per read and write, so a server that admits or answers nothing fails it rather than hanging (D-2047) | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end` | ✓ |
 | G18-api-29 | A dropped calendar `Landing` marks its flight `Abandoned` (or answered), removes it from the flight table, and wakes every follower (D-2047) | `api::calendar_of::tests::a_calendar_landing_releases_its_flight_and_wakes_its_followers_when_dropped` | ✓ |
+
+### Lane 1-b fixer D, G3 and G5 verification items (D-4748 onward)
+
+| Id | Invariant | Proof | |
+|---|---|---|---|
+| L1FD-01 | Boolean research judges each venue against its own close. An eligible share's CAS-day calendar ends at its dated close and receipts under policy 4. The index on the same days is unchanged at policy 3. A day with an unreadable close refuses as "cash session close UNVERIFIED", never as an incomplete store, and a bar past the dated close refuses as the store contradicting its master. A dated day with no close and no bar is `Unmeasured`, and a pre-CAS share receipt expects the calendar yet differs in digest from the index's (D-4748) | `cli::candidate_universe::boolean_candidate_v1::tests::a_cas_share_and_an_index_on_the_same_days_are_each_judged_against_their_own_close` · `cli::stored::tests::a_share_receipt_is_policy_four_and_expects_its_dated_close` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 552ce57a..cc72e831 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65025,3 +65025,82 @@ on a live leader would derive a second time and lose the single-flight
 guarantee D-1443 exists for. **Honest limit:** the `Landing` kill depends on
 test order. A rename that sorted a single-flight test ahead of it would
 restore the timeout, so the ordering is pinned in the test's own doc.
+
+### D-4748 — The Boolean research path judges a cash share's calendar against its dated close — 2026-10-09
+
+**Finding.** G3-1, the Boolean sibling of GAP12-6 (medium). Boolean research
+admits a cash share (`ResearchFamilyV1::new` takes tag 3), and its
+`prepare_column` built both calendar receipts with
+`stored::calendar_receipt_v2_for_bars`, whose expected buckets come from
+`pull::calendar::kind_of` alone: the index session, last minute 15:29. From
+2026-08-03 an eligible share's stored minutes end at 15:14 on every
+closing-auction day, so any span holding one was refused as "calendar receipt
+V2 ... is not complete". Past that check, `build_candidate_signal_column`
+overlaid ORB and `GapFib` with `stored::nse_session_close_minute`.
+
+**Decision.**
+
+- `stored::calendar_receipt_v2_for_venue_bars(bars, rung, first, last, cash)`
+  takes the share's `CashCloses`, the dated authority the stored path already
+  reads (D-2102). It is the only receipt implementation:
+  `calendar_receipt_v2_for_bars` is it with `None`, byte for byte.
+- On a CAS-dated day the last window of the expected session ends at the
+  share's dated close. The day hashes tag 6, not 1, and the dated windows
+  are hashed in place of the calendar's.
+- A day whose close cannot be read refuses as "cash session close UNVERIFIED",
+  naming the reason, never as an incomplete store. A bar past the dated close
+  refuses as the store contradicting its session master.
+- A dated day whose close was not read, with no bar offered on it, hashes
+  tag 7 and leaves the receipt `Unmeasured`: completeness is never claimed
+  against a close the run does not hold.
+- A receipt built with closes carries **policy 4**,
+  `CALENDAR_RECEIPT_POLICY_V2_DATED_CASH`, so it never shares a version with
+  a policy-3 receipt (§3 rule 8). An index receipt is unchanged: policy 3, the
+  same bytes.
+- The Boolean path passes `data.exact_minute.cash` to both receipts, to
+  `require_exact_calendar_for_venue` and to `build_candidate_columns_for_venue`.
+  The overlay then asks `stored::session_close_for`, the one rule the stored
+  path's overlay and census ask.
+- The Boolean source identity binds the closes' digest, so the same bars
+  judged against a different master are a different source.
+
+**What changes.**
+
+- Boolean research over an NSE cash share: its receipts are policy 4 and its
+  source identity binds the closes, so every such family, coordinate and
+  catalogue identity changes, pre-CAS spans included.
+- Spans holding a CAS day now complete instead of refusing.
+- Index runs and every other receipt caller are unchanged. Candidate V4,
+  Global Replay, Search V4, `index_stop` and the Step 3 sizing census are
+  index-only, and they keep `None`.
+
+Tests (L1FD-01):
+
+- `a_cas_share_and_an_index_on_the_same_days_are_each_judged_against_their_own_close`.
+  It covers an eligible share ending at 15:14, an index on the same days, an
+  ineligible share, one absent master, and a store running past the dated
+  close.
+- `a_share_receipt_is_policy_four_and_expects_its_dated_close`. On one CAS
+  day the index receipt expects 375 minutes and misses 15, while the share's
+  expects 360 and is complete. It also checks that a pre-CAS day expects the
+  same count under both policies with different digests, that an unread close
+  is `Unmeasured` with no bar and refused with one, and that a bar past 15:14
+  is refused.
+
+**Rejected.** A per-day closure parameter on the receipt. A closure could
+answer differently from the overlay. Passing the closes value lets both ask
+one function.
+
+### D-4749 — Correction to D-2102: a share does reach the Candidate universe's overlay — 2026-10-09
+
+**Corrects.** D-2102, "Unchanged on purpose", says: "The Candidate universe's
+overlay still asks the index calendar. Its families are NIFTY and BANKNIFTY
+only (`candidate_universe::require_series_family`), so no share reaches it."
+
+That holds for the Candidate V4 grid. It was false for
+`candidate_universe::boolean_candidate_v1`, a submodule of the same file. It
+admits cash families and called the same `build_candidate_columns`, so an
+eligible share reached the index calendar there (G3-1).
+
+**Now.** D-4748 threads the share's closes through that builder. The index-only
+callers still pass none, and that is the statement D-2102 should have made.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 12e9bf09..b8e6e10a 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15967,3 +15967,16 @@ on every request. D-0904 names the same directory walk for three other
 routes; this one was named only in D-1445's list of audited routes. The second
 open is the freshness check, and it is kept. **No timing was taken. The cost
 is UNVERIFIED as a measurement.**
+
+## Two read-side costs from lane 1-b fixer D — D-4748 and D-4756, 9 October 2026
+
+### A cash share's calendar receipt asks its dated close once per bar and once per day (D-4748)
+
+`stored::calendar_receipt_v2_for_venue_bars` keeps the receipt at O(B + D) time
+for B offered timestamps and D requested days, with O(1) auxiliary space.
+Given a share's `CashCloses`, every offered bar and every requested day on or
+after 2026-08-03 also pays one `CashCloses::session_close_minute`: two
+`pull::calendar::kind_of` calls and one schedule lookup. Days before then pay one
+civil-day conversion. An index receipt, built with `None`, pays nothing more.
+**UNVERIFIED as a measured bound.** No bench times either receipt, so this is
+read off the source.
-- 
2.43.0


From 4b8462e3427e7827ab7bda93e5c393ef40dec0e4 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:41:56 +0000
Subject: [PATCH 2/8] cli: bind a stable reason class in the cash-closes digest
 (G3-4)

Before: load_cash_closes hashed each unverified day's free-text reason.
Some reasons carry the store's absolute path or lock/OS wording, so the same
bars and the same unusable master gave different stored run identities at
two store paths, which breaks section 3 rule 5.

After: an unverified day hashes [2, class]: 1 no ISIN, 2 master unusable, 3
master read and not naming the share (plus that master's SHA-256). The text
is kept for display only.

Fail before:
  stored.rs:5579: "lock-dir: the same unreadable master at two paths is one
  identity" (the two digests differed)

D-4750, L1FD-02.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/stored.rs | 161 ++++++++++++++++++++++++++++++++++-----
 docs/04-invariants.md    |   1 +
 docs/05-decisions.md     |  42 ++++++++++
 3 files changed, 186 insertions(+), 18 deletions(-)

diff --git a/crates/cli/src/stored.rs b/crates/cli/src/stored.rs
index 35b51f1a..a20802ae 100644
--- a/crates/cli/src/stored.rs
+++ b/crates/cli/src/stored.rs
@@ -545,9 +545,10 @@ pub fn nse_session_close_minute(day: i64) -> Option<u16> {
 /// than a close being invented. An exceptional (non-full) session on such a day
 /// answers `None` too: no source in the charter gives a share's close there.
 ///
-/// `digest` binds every answer and every reason, and enters the stored run
-/// identity, so the same bars judged against a different master are a
-/// different run.
+/// `digest` binds every answer and every unverified reason's CLASS, never its
+/// text (D-4750), and enters the stored run identity, so the same bars judged
+/// against a different master are a different run, and the same bars beside
+/// the same unusable master are one run wherever the store is mounted.
 #[derive(Debug, Clone)]
 pub struct CashCloses {
     schedule: pull::cash_auction::Schedule,
@@ -557,8 +558,8 @@ pub struct CashCloses {
 }
 
 /// Two sets of closes are equal when their digests are: the digest binds the
-/// share, every dated day, flag and master hash, and every unverified reason,
-/// and `Schedule` itself offers no comparison.
+/// share, every dated day, flag and master hash, and every unverified reason
+/// class, and `Schedule` itself offers no comparison.
 impl PartialEq for CashCloses {
     fn eq(&self, other: &Self) -> bool {
         self.digest == other.digest
@@ -616,7 +617,7 @@ impl CashCloses {
         self.unverified.get(&day).map(String::as_str)
     }
 
-    /// Identity term binding every dated answer and every refusal reason.
+    /// Identity term binding every dated answer and every refusal reason class.
     #[must_use]
     pub const fn digest(&self) -> [u8; 32] {
         self.digest
@@ -709,15 +710,30 @@ pub fn load_cash_closes(
     hash.update(&u64::try_from(share.len()).unwrap_or(u64::MAX).to_le_bytes());
     hash.update(share.as_bytes());
     for (day, civil) in required {
+        // THE REASON'S CLASS ENTERS THE DIGEST, ITS TEXT DOES NOT (G3-4,
+        // D-4750). The text can carry the store's absolute path or a lock's OS
+        // error, so hashing it made one span's identity depend on where the
+        // store was mounted. The class names which step failed: 1 no ISIN, 2
+        // the receipted master could not be used, 3 it was read and does not
+        // name this share, bound with its content hash. Marker 2 never equals
+        // the old marker 0, so no digest of either encoding equals the other's.
         let answer = isin
             .as_ref()
-            .ok_or_else(|| format!("{share} has no exact NSE ISIN in the universe table"))
+            .ok_or_else(|| {
+                (
+                    1_u8,
+                    None,
+                    format!("{share} has no exact NSE ISIN in the universe table"),
+                )
+            })
             .and_then(|isin| {
-                pull::cash_session_cache::read_local_lifecycle(&masters, civil).and_then(|master| {
-                    master
-                        .eligibility(share, isin.as_str())
-                        .map(|eligible| (eligible, master.provenance().sha256))
-                })
+                let master = pull::cash_session_cache::read_local_lifecycle(&masters, civil)
+                    .map_err(|why| (2, None, why))?;
+                let sha256 = master.provenance().sha256;
+                master
+                    .eligibility(share, isin.as_str())
+                    .map(|eligible| (eligible, sha256))
+                    .map_err(|why| (3, Some(sha256), why))
             });
         hash.update(&day.to_le_bytes());
         match answer {
@@ -727,10 +743,11 @@ pub fn load_cash_closes(
                 hash.update(&[1, u8::from(eligible)]);
                 hash.update(&sha256);
             }
-            Err(why) => {
-                hash.update(&[0]);
-                hash.update(&u64::try_from(why.len()).unwrap_or(u64::MAX).to_le_bytes());
-                hash.update(why.as_bytes());
+            Err((class, master, why)) => {
+                hash.update(&[2, class]);
+                if let Some(sha256) = master {
+                    hash.update(&sha256);
+                }
                 unverified.insert(day, why);
             }
         }
@@ -5500,10 +5517,15 @@ mod tests {
     /// Install one receipted NSE session master naming RELIANCE with `flag`
     /// for `day` under `store/session-masters`, exactly as the pull does.
     fn install_master(store: &Path, day: i64, flag: u8) {
+        install_master_naming(store, day, "RELIANCE", flag);
+    }
+
+    /// [`install_master`] for a master that names `symbol` and nothing else.
+    fn install_master_naming(store: &Path, day: i64, symbol: &str, flag: u8) {
         use std::io::Write as _;
-        let isin = brutex_core::universe::nse_isin("RELIANCE").expect("RELIANCE has an ISIN");
+        let isin = brutex_core::universe::nse_isin(symbol).expect("the symbol has an ISIN");
         let csv = format!(
-            "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,RELIANCE,EQ,{},{flag}\n",
+            "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,{symbol},EQ,{},{flag}\n",
             isin.as_str()
         );
         let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
@@ -5721,6 +5743,109 @@ mod tests {
         let _ignored = std::fs::remove_dir_all(&ineligible);
     }
 
+    /// G3-4, D-4750: the closes' digest binds WHY a date went unverified as a
+    /// stable class, never the free text, which carries the store's absolute
+    /// path and OS or lock-contention wording. Two stores at different paths
+    /// holding the same unreadable master (its lock path a directory, or its
+    /// receipt damaged) produced different digests, and so different run
+    /// identities for the same bars (§3 rule 5). The text stays for display.
+    #[test]
+    fn an_unreadable_master_binds_its_reason_class_and_not_its_path() {
+        let civil = cas_dated_day(OPEN_MONDAY_2026_08_03).expect("a CAS-era day");
+        let lock = format!(
+            ".NSE_CM_security_{:02}{:02}{:04}.csv.gz.lock",
+            civil.day(),
+            civil.month(),
+            civil.year()
+        );
+        let receipt = format!(
+            "NSE_CM_security_{:02}{:02}{:04}.csv.gz.receipt",
+            civil.day(),
+            civil.month(),
+            civil.year()
+        );
+        let days = [OPEN_MONDAY_2026_08_03];
+        let absent = load_cash_closes(Path::new(NO_STORE), "RELIANCE", days).expect("loads");
+        for (case, damage) in [
+            (
+                "lock-dir",
+                (|store: &Path, lock: &str, _: &str| {
+                    std::fs::create_dir_all(store.join("session-masters").join(lock))
+                        .expect("a directory where the lock belongs");
+                }) as fn(&Path, &str, &str),
+            ),
+            ("receipt", |store: &Path, _: &str, receipt: &str| {
+                install_master(store, OPEN_MONDAY_2026_08_03, 1);
+                std::fs::write(store.join("session-masters").join(receipt), b"damaged")
+                    .expect("the receipt is rewritten");
+            }),
+        ] {
+            let near = root(&format!("class-{case}"));
+            let far = root(&format!(
+                "class-{case}-at-another-much-longer-absolute-path"
+            ));
+            for store in [&near, &far] {
+                damage(store, &lock, &receipt);
+            }
+            let one = load_cash_closes(&near, "RELIANCE", days).expect("loads");
+            let two = load_cash_closes(&far, "RELIANCE", days).expect("loads");
+            let (why_one, why_two) = (
+                one.unverified_reason(OPEN_MONDAY_2026_08_03)
+                    .expect("held unverified"),
+                two.unverified_reason(OPEN_MONDAY_2026_08_03)
+                    .expect("held unverified"),
+            );
+            assert_ne!(
+                why_one, why_two,
+                "premise {case}: the free text names each store's own path"
+            );
+            assert!(
+                why_one.contains(near.to_string_lossy().as_ref()),
+                "{case}: {why_one}"
+            );
+            assert_eq!(
+                one.digest(),
+                two.digest(),
+                "{case}: the same unreadable master at two paths is one identity"
+            );
+            assert_eq!(
+                one.digest(),
+                absent.digest(),
+                "{case}: absent and unreadable answer the same None and bind one class"
+            );
+            assert_eq!(one.session_close_minute(OPEN_MONDAY_2026_08_03), None);
+            let _ignored = std::fs::remove_dir_all(&near);
+            let _ignored = std::fs::remove_dir_all(&far);
+        }
+        // For one share, a dated answer, an unreadable master and a master
+        // that does not name it are three facts and three digests.
+        let named = root("class-named");
+        install_master(&named, OPEN_MONDAY_2026_08_03, 1);
+        let other = root("class-other");
+        install_master_naming(&other, OPEN_MONDAY_2026_08_03, "TCS", 1);
+        let dated = load_cash_closes(&named, "RELIANCE", days).expect("loads");
+        let not_named = load_cash_closes(&other, "RELIANCE", days).expect("loads");
+        assert!(
+            not_named
+                .unverified_reason(OPEN_MONDAY_2026_08_03)
+                .is_some()
+        );
+        let digests = [dated.digest(), absent.digest(), not_named.digest()];
+        for (i, a) in digests.iter().enumerate() {
+            for b in digests.iter().skip(i + 1) {
+                assert_ne!(a, b, "dated, unreadable and not named are three facts");
+            }
+        }
+        // A word with no ISIN binds its class wherever the store is.
+        let no_isin = load_cash_closes(&named, "NOSUCHSHARE", days).expect("loads");
+        let no_isin_absent =
+            load_cash_closes(Path::new(NO_STORE), "NOSUCHSHARE", days).expect("loads");
+        assert!(no_isin.unverified_reason(OPEN_MONDAY_2026_08_03).is_some());
+        assert_eq!(no_isin.digest(), no_isin_absent.digest());
+        let _ignored = std::fs::remove_dir_all(&named);
+        let _ignored = std::fs::remove_dir_all(&other);
+    }
+
     #[test]
     fn exact_minute_context_refuses_holes_and_malformed_cadence() {
         let signal = [minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000)];
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 48ace945..1bb44c58 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7058,3 +7058,4 @@ old line regex the same input and watched it pass.
 | Id | Invariant | Proof | |
 |---|---|---|---|
 | L1FD-01 | Boolean research judges each venue against its own close. An eligible share's CAS-day calendar ends at its dated close and receipts under policy 4. The index on the same days is unchanged at policy 3. A day with an unreadable close refuses as "cash session close UNVERIFIED", never as an incomplete store, and a bar past the dated close refuses as the store contradicting its master. A dated day with no close and no bar is `Unmeasured`, and a pre-CAS share receipt expects the calendar yet differs in digest from the index's (D-4748) | `cli::candidate_universe::boolean_candidate_v1::tests::a_cas_share_and_an_index_on_the_same_days_are_each_judged_against_their_own_close` · `cli::stored::tests::a_share_receipt_is_policy_four_and_expects_its_dated_close` | ✓ |
+| L1FD-02 | The cash-closes digest binds each unverified day's reason class (no ISIN, master unusable, or master read and not naming the share, with that master's SHA-256), never the reason's text. Two store roots at different paths with the same unusable master give one digest (D-4750) | `cli::stored::tests::an_unreadable_master_binds_its_reason_class_and_not_its_path` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index cc72e831..e01e3835 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65104,3 +65104,45 @@ eligible share reached the index calendar there (G3-1).
 
 **Now.** D-4748 threads the share's closes through that builder. The index-only
 callers still pass none, and that is the statement D-2102 should have made.
+
+### D-4750 — The cash-closes digest binds a reason class, not the reason's text — 2026-10-09
+
+**Finding.** G3-4 (low). `stored::load_cash_closes` hashed each unverified
+day's free-text reason, and `bind_cash_closes` folds that digest into the
+stored anchored identity. Some reasons embed the absolute path of
+`<store>/session-masters/...` or OS and lock text. One example is "UNVERIFIED
+local lifecycle lock {path} unavailable: {why}". So the same bars and the same
+missing master gave different run identities depending on where the store was
+mounted and on transient lock state. That breaks §3 rule 5.
+
+**Corrects.** D-2102's sentence "The closes' digest binds every day, flag,
+master hash and unverified reason". It now binds every day, flag and master
+hash, and each unverified day's reason **class**.
+
+**Decision.** An unverified day hashes marker byte `2` and one class byte.
+The class says which step failed:
+
+- **1:** no exact NSE ISIN in the universe table.
+- **2:** the receipted master could not be used. It was absent, locked,
+  unreadable or corrupt.
+- **3:** the master was read and does not name this exact symbol and ISIN.
+  This class also binds that master's SHA-256, which is its content and not
+  its location.
+
+The free text is kept for display, and `unverified_reason` still returns it.
+The old encoding (`0`, length, text) is never written again. The new marker
+differs from it, so no digest of one encoding can equal one of the other.
+
+**What changes.** Only the identity of a stored run over an NSE cash share
+whose span holds at least one CAS full-session day whose close could not be
+read: the signal days plus the `GapFib` prior session. A span whose every CAS
+day was dated hashes exactly as before. So does every index run, and every
+share span before 2026-08-03.
+
+Test: `an_unreadable_master_binds_its_reason_class_and_not_its_path` (L1FD-02).
+Two store roots at different paths, each with a master made unusable two ways,
+give equal digests, equal to the absent master's. The three classes stay
+distinct for one share.
+
+**Rejected.** Classifying by matching the reason's text. That would make the
+identity depend on the wording of a message in another crate.
-- 
2.43.0


From 4d476bbbde56a8d3f8a3093a8802e5230e1a1556 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:42:42 +0000
Subject: [PATCH 3/8] cli: sweep-stored's withheld line names the swept count
 (AC-whp-law-2)

Before: with minute-gap sessions withheld, sweep-stored said "The sweep uses
the remaining {retained} signal bars". The sweep's support is counted over
the swept rows, which exclude warm-up and unknown-only rows, so the stated
denominator was not the one used.

After: "Support uses the {swept} swept bar(s) of the remaining {retained}
signal bars", where swept is outcome.census.swept, the figure the ledger
records. This matches the span audit's wording.

Fail before:
  audited_stored_tests.rs:2243: the report lacks "Support uses the 1500
  swept bar(s) of the remaining 2625 signal bars"

D-4751, L1FD-03.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/audited_stored_tests.rs | 10 +++++++++-
 crates/cli/src/lib.rs                  |  8 +++++++-
 docs/04-invariants.md                  |  1 +
 docs/05-decisions.md                   | 24 ++++++++++++++++++++++++
 4 files changed, 41 insertions(+), 2 deletions(-)

diff --git a/crates/cli/src/audited_stored_tests.rs b/crates/cli/src/audited_stored_tests.rs
index 7f1a83d2..610ce952 100644
--- a/crates/cli/src/audited_stored_tests.rs
+++ b/crates/cli/src/audited_stored_tests.rs
@@ -2236,13 +2236,21 @@ fn the_ordinary_stored_sweep_withholds_and_names_a_holed_session() {
         }
 
         let report = fixture.sweep(rung).expect("the holed month sweeps");
+        // THE SWEPT COUNT BESIDE THE RETAINED ONE (AC-whp-law-2, D-4751). This
+        // pinned "The sweep uses the remaining {retained} signal bars", which
+        // counted warming bars the column never swept while the ledger below
+        // held `swept`. The line now says what `screen`'s does (D-2101).
         assert!(
             report.contains(&format!(
                 "MINUTE-GAP SESSIONS WITHHELD: {withheld} signal bar(s); IST dates: 2025-05-05. \
-                 The sweep uses the remaining {retained} signal bars."
+                 Support uses the {swept} swept bar(s) of the remaining {retained} signal bars."
             )),
             "{report}"
         );
+        assert!(
+            !report.contains("The sweep uses the remaining"),
+            "the old sentence named the retained slice as swept: {report}"
+        );
         assert!(report.contains(&format!("· {retained} bars ·")), "{report}");
         assert!(report.contains("RESULT RECORDED"), "{report}");
         let mut ledger = crate::results::Results::open_read(&fixture.root).expect("sweep ledger");
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 4ae07321..eef0b642 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -4346,12 +4346,18 @@ fn stored_month_kernel(
     // `withheld > 0`. A holed minute day the signal rung holds no bar of
     // removes nothing, and "0 signal bar(s)" would name a withholding that
     // did not happen.
+    //
+    // THE SWEPT COUNT, BESIDE THE RETAINED ONE (AC-whp-law-2, D-4751). The
+    // sweep's support is counted over the column's swept rows, which the
+    // ledger records as this run's bars; the retained slice also holds the
+    // warm-up. The span audit's line has said so since D-2101.
     if let Some(gaps) = minute_gaps.as_ref().filter(|gaps| gaps.signal_bars() > 0) {
         let _ = writeln!(
             out,
-            "MINUTE-GAP SESSIONS WITHHELD: {} signal bar(s); IST dates: {}. The sweep uses the remaining {} signal bars.",
+            "MINUTE-GAP SESSIONS WITHHELD: {} signal bar(s); IST dates: {}. Support uses the {} swept bar(s) of the remaining {} signal bars.",
             gaps.signal_bars(),
             gaps.day_names().join(" "),
+            outcome.census.swept,
             loaded.bars.len(),
         );
     }
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 1bb44c58..8ae68c1e 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7059,3 +7059,4 @@ old line regex the same input and watched it pass.
 |---|---|---|---|
 | L1FD-01 | Boolean research judges each venue against its own close. An eligible share's CAS-day calendar ends at its dated close and receipts under policy 4. The index on the same days is unchanged at policy 3. A day with an unreadable close refuses as "cash session close UNVERIFIED", never as an incomplete store, and a bar past the dated close refuses as the store contradicting its master. A dated day with no close and no bar is `Unmeasured`, and a pre-CAS share receipt expects the calendar yet differs in digest from the index's (D-4748) | `cli::candidate_universe::boolean_candidate_v1::tests::a_cas_share_and_an_index_on_the_same_days_are_each_judged_against_their_own_close` · `cli::stored::tests::a_share_receipt_is_policy_four_and_expects_its_dated_close` | ✓ |
 | L1FD-02 | The cash-closes digest binds each unverified day's reason class (no ISIN, master unusable, or master read and not naming the share, with that master's SHA-256), never the reason's text. Two store roots at different paths with the same unusable master give one digest (D-4750) | `cli::stored::tests::an_unreadable_master_binds_its_reason_class_and_not_its_path` | ✓ |
+| L1FD-03 | `sweep-stored`'s withheld-sessions line names the swept bar count, which the ledger records, beside the retained count, and never calls the retained slice what the sweep uses (D-4751) | `cli::audited_stored::tests::the_ordinary_stored_sweep_withholds_and_names_a_holed_session` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index e01e3835..f659ea8d 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65146,3 +65146,27 @@ distinct for one share.
 
 **Rejected.** Classifying by matching the reason's text. That would make the
 identity depend on the wording of a message in another crate.
+
+### D-4751 — `sweep-stored` names the swept bars beside the retained ones — 2026-10-09
+
+**Finding.** AC-whp-law-2. When minute-gap sessions are withheld, the
+`sweep-stored` page said "The sweep uses the remaining {retained} signal
+bars". The retained count is `loaded.bars.len()`. The sweep's support is
+counted over the column's swept rows, which exclude the warm-up and the
+unknown-only rows. So the sentence stated a denominator the sweep did not
+use. The span audit's page already says "Support uses the {can_hit} swept
+bar(s) of the remaining {retained} signal bars".
+
+**Decision.** The `sweep-stored` line now reads "Support uses the {swept} swept
+bar(s) of the remaining {retained} signal bars". `swept` is
+`outcome.census.swept`, the figure the run's own ledger records as its bars.
+
+The other bar-count lines were checked:
+
+- `sweep-all`'s tally already sums `census.swept`.
+- The descent states `can_hit`.
+- The execution notes count signal bars, and say so.
+
+Test: `the_ordinary_stored_sweep_withholds_and_names_a_holed_session`
+(L1FD-03), re-pinned on the new sentence. It also asserts that the old
+sentence is gone.
-- 
2.43.0


From 9f09e1ed9ce96afc1bf0d282c7b67adc49f1b8a2 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:42:58 +0000
Subject: [PATCH 4/8] cli: every multi-run page takes the pooled banner; ledger
 refusals none

Before: range-all, the single-stop search and sweep-all report many runs but
opened with STORED_PROVENANCE, which promises one instrument, one month and
one identity (GAP15-21, G3-2). ledger-all, ledger-v6 and ledger-v6-replay
pushed the pooled banner first, so a feed word refused before any store read
still claimed bars were read from files, contrary to D-1705.

After: those three pages open with STORED_POOLED_PROVENANCE. range-all uses
the new stored_pooled_provenance(underlying), which adds a stock's equity
note. The ledger pages start empty and call
ledger_all::lead_with_pooled_banner once, immediately before their first
store-derived figure. Success pages are byte-identical. A source-shape guard
covers all eight multi-run page builders.

Fail before:
  lib.rs:25799: "lib.rs::range_opening reports many runs and must open with
  the pooled banner"
  batch.rs:1297: "an index walk carries no equity note: === THESE BARS ARE
  REAL..."
  pool.rs:3813: "ledger-v6: a refusal before any store read carries no banner"
  pool.rs:3863: "ledger_all must not open every page, refusals included, with
  the banner"
  equity_statement_tests.rs:236, index_stop_search_tests.rs:77

D-4752, L1FD-04.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/batch.rs                   | 21 ++++--
 crates/cli/src/equity_statement_tests.rs  | 59 +++++++++++----
 crates/cli/src/index_stop_search.rs       |  4 +-
 crates/cli/src/index_stop_search_tests.rs |  8 +++
 crates/cli/src/ledger_all.rs              | 21 +++++-
 crates/cli/src/ledger_v6.rs               | 10 ++-
 crates/cli/src/lib.rs                     | 87 ++++++++++++++++++++++-
 crates/cli/src/pool.rs                    | 75 +++++++++++++++++--
 docs/04-invariants.md                     |  1 +
 docs/05-decisions.md                      | 41 +++++++++++
 10 files changed, 297 insertions(+), 30 deletions(-)

diff --git a/crates/cli/src/batch.rs b/crates/cli/src/batch.rs
index 51a7fa7c..172d4c27 100644
--- a/crates/cli/src/batch.rs
+++ b/crates/cli/src/batch.rs
@@ -933,7 +933,9 @@ fn render(
     tally: &Tally,
     rows: &[Row],
 ) -> String {
-    let mut out = String::from(crate::STORED_PROVENANCE);
+    // EVERY INSTRUMENT BY EVERY MONTH, EACH ITS OWN RUN, SO THE POOLED BANNER
+    // (GAP15-21, D-4752). Each run's identity is printed under its month.
+    let mut out = String::from(crate::STORED_POOLED_PROVENANCE);
     out.push_str(equity_note);
     let _ = writeln!(
         out,
@@ -1024,7 +1026,7 @@ fn month_lines(out: &mut String, rows: &[Row]) {
             //
             // Indented past the label so a reader scanning months is not made to
             // read 64 hex characters per line, and printed rather than omitted
-            // because `STORED_PROVENANCE` two screens above tells them it is
+            // because `STORED_POOLED_PROVENANCE` two screens above tells them it is
             // here. A run whose identity is not recorded is a run `CLAUDE.md` §3
             // rule 3 does not permit.
             if let Some(ref hex) = row.identity {
@@ -1216,6 +1218,7 @@ mod tests {
             "the verb prints it as a refusal"
         );
         assert!(!why.contains(crate::STORED_PROVENANCE), "{why}");
+        assert!(!why.contains(crate::STORED_POOLED_PROVENANCE), "{why}");
     }
 
     /// **A walk whose one swept month could not be filed says it swept, and
@@ -1277,6 +1280,7 @@ mod tests {
             "{why}"
         );
         assert!(!why.contains(crate::STORED_PROVENANCE), "{why}");
+        assert!(!why.contains(crate::STORED_POOLED_PROVENANCE), "{why}");
     }
 
     /// **A walk that offers a stock states corporate actions are unchecked;
@@ -1287,10 +1291,14 @@ mod tests {
     /// reader must know what a stock's figures are made of before reading one.
     #[test]
     fn a_walk_offering_a_stock_states_corporate_actions_are_unchecked_and_an_index_walk_does_not() {
+        // EVERY INSTRUMENT AND EVERY MONTH, SO THE POOLED BANNER (GAP15-21,
+        // D-4752). The walk sweeps one run per instrument-month and its tally
+        // sums them all; the single-run banner promised one identity and "that
+        // instrument and that month".
         let indices = beside_a_swept_month(&[]);
         assert!(
-            indices.starts_with(&format!("{}feed zerodha", crate::STORED_PROVENANCE)),
-            "an index walk is unchanged: {indices}"
+            indices.starts_with(&format!("{}feed zerodha", crate::STORED_POOLED_PROVENANCE)),
+            "an index walk carries no equity note: {indices}"
         );
         assert!(!indices.contains("CORPORATE ACTIONS"), "{indices}");
 
@@ -1298,12 +1306,15 @@ mod tests {
         assert!(
             mixed.starts_with(&format!(
                 "{}{}feed zerodha",
-                crate::STORED_PROVENANCE,
+                crate::STORED_POOLED_PROVENANCE,
                 runner::audit::CostScope::CashEquity.report_note()
             )),
             "{mixed}"
         );
         assert!(mixed.contains("RELIANCE"), "{mixed}");
+        for page in [&indices, &mixed] {
+            assert!(!page.contains(crate::STORED_PROVENANCE), "{page}");
+        }
     }
 
     /// **A month held anywhere but its own key's path is refused by name, and
diff --git a/crates/cli/src/equity_statement_tests.rs b/crates/cli/src/equity_statement_tests.rs
index 3852317f..ddb66c23 100644
--- a/crates/cli/src/equity_statement_tests.rs
+++ b/crates/cli/src/equity_statement_tests.rs
@@ -37,11 +37,14 @@ fn span(underlying: &str) -> crate::stored::Span {
     }
 }
 
-/// Every one-instrument banner a stored report opens with, for `underlying`.
+/// Every one-run banner a stored report opens with, for `underlying`.
 ///
 /// `audit-range` stands for the span banner, which `screen` and the strict
-/// audited range open with too.
-fn banners(underlying: &str) -> [(&'static str, String); 6] {
+/// audited range open with too. `range-all` is not here: it reports eight
+/// rungs, each its own run, and opens with the pooled banner
+/// (GAP15-21, D-4752), pinned by
+/// `range_all_opens_with_the_pooled_banner_and_a_stocks_note`.
+fn banners(underlying: &str) -> [(&'static str, String); 5] {
     [
         (
             "audit-range",
@@ -53,16 +56,6 @@ fn banners(underlying: &str) -> [(&'static str, String); 6] {
                 "abc123",
             ),
         ),
-        (
-            "range-all",
-            range_opening(
-                "zerodha",
-                underlying,
-                &["5min", "1min"],
-                ((2025, 1), (2025, 6)),
-                None,
-            ),
-        ),
         (
             "descend",
             descend_banner(
@@ -156,8 +149,8 @@ const SET_OFF: [&str; 2] = ["elite descent", "top"];
 /// with a blank line of their own, and a stock's page carried one more blank
 /// line there than its index page did: three before `TOP COMBINATIONS` where
 /// an index had two. On those two the note's closing blank line takes the
-/// place of the index page's, and is not a second one. `range-all`,
-/// `descend`, `audit-stored` and the span banner run from the provenance
+/// place of the index page's, and is not a second one. `descend`,
+/// `audit-stored` and the span banner run from the provenance
 /// straight into their feed line, so there the note's closing blank line is
 /// one the stock's page has and the index page does not. It sets the note
 /// off from the report, and the index page has no note to set off.
@@ -218,6 +211,42 @@ fn a_stock_banner_is_its_index_banner_with_the_note_put_in_and_nothing_else() {
     }
 }
 
+/// **`range-all` opens with the pooled banner, a stock's with its note after
+/// it, and never with the single-run banner.** GAP15-21, D-4752.
+///
+/// The page spans eight rungs, each its own run and identity, over a span of
+/// months, so "the run identity beneath names the exact column" and "a figure
+/// here describes that instrument and that month" were both false of it.
+#[test]
+fn range_all_opens_with_the_pooled_banner_and_a_stocks_note() {
+    let page = |underlying: &str| {
+        range_opening(
+            "zerodha",
+            underlying,
+            &["5min", "1min"],
+            ((2025, 1), (2025, 6)),
+            None,
+        )
+    };
+    let note = CostScope::CashEquity.report_note();
+    for (stock, index) in [("RELIANCE", "NIFTY"), ("TCS", "BANKNIFTY")] {
+        let stock_text = page(stock);
+        let index_text = page(index);
+        for text in [&stock_text, &index_text] {
+            assert!(!text.contains(STORED_PROVENANCE), "{text}");
+        }
+        let stock_rest = stock_text
+            .strip_prefix(&format!("{}{note}", super::STORED_POOLED_PROVENANCE))
+            .expect("a stock's range page leads with the pooled banner and its note");
+        let index_rest = index_text
+            .strip_prefix(super::STORED_POOLED_PROVENANCE)
+            .expect("an index's range page leads with the pooled banner");
+        assert!(index_rest.starts_with("feed zerodha"), "{index_text}");
+        assert!(!index_text.contains("CORPORATE ACTIONS"), "{index_text}");
+        assert_eq!(stock_rest.replace(stock, index), index_rest);
+    }
+}
+
 /// **The note `api` serves beside a recorded run is the stored banner's own.**
 /// AF-19.
 ///
diff --git a/crates/cli/src/index_stop_search.rs b/crates/cli/src/index_stop_search.rs
index 5207ba2c..acb48329 100644
--- a/crates/cli/src/index_stop_search.rs
+++ b/crates/cli/src/index_stop_search.rs
@@ -210,7 +210,9 @@ fn execute_observed_with(
         .thread_name(|index| format!("index-stop-{index}"))
         .build()
         .map_err(display)?;
-    let mut report = String::from(crate::STORED_PROVENANCE);
+    // MANY RUNS, SO THE POOLED BANNER (GAP15-21, D-4752): one run per rung
+    // and month window, never "that instrument and that month".
+    let mut report = String::from(crate::STORED_POOLED_PROVENANCE);
     let _ = writeln!(
         report,
         "\nSingle-stop search {}: {} on {:?}; {} parallel timeframe workers. Both long/short and both printed fill readings; criteria use pessimistic gross results. Exit is the completed signal-candle stop or 15:10 IST, with no target/trailing/horizon grid.\n",
diff --git a/crates/cli/src/index_stop_search_tests.rs b/crates/cli/src/index_stop_search_tests.rs
index 944748a6..8d2cf709 100644
--- a/crates/cli/src/index_stop_search_tests.rs
+++ b/crates/cli/src/index_stop_search_tests.rs
@@ -71,6 +71,14 @@ fn complete_native_search_reopens_all_ancestors_and_resumes_without_duplicating_
         crate::index_stop::tests::load_generated,
     )?;
     assert!(report.contains("Search exhausted: false"));
+    // SEVERAL RUNGS OVER TRAINING AND LATER MONTHS, SO THE POOLED BANNER
+    // (GAP15-21, D-4752), never the single-run one's "that instrument and
+    // that month".
+    assert!(
+        report.starts_with(crate::STORED_POOLED_PROVENANCE),
+        "{report}"
+    );
+    assert!(!report.contains(crate::STORED_PROVENANCE), "{report}");
     let first = observed.last().ok_or("complete progress missing")?;
     assert_eq!(first.completed_batches, 1);
     assert!(!first.exhausted);
diff --git a/crates/cli/src/ledger_all.rs b/crates/cli/src/ledger_all.rs
index 10a7ac6a..8c55c009 100644
--- a/crates/cli/src/ledger_all.rs
+++ b/crates/cli/src/ledger_all.rs
@@ -805,6 +805,21 @@ pub(crate) fn gates_refused_event(verb: &str, missing: usize) -> telemetry::Even
         .with("gates", ALL_GATES.len())
 }
 
+/// Puts [`crate::STORED_POOLED_PROVENANCE`] at the head of `out` unless it
+/// already leads it (G3-2, D-4752).
+///
+/// The three ledger pages call this immediately before their first
+/// store-derived figure and on success, never at the top of the page: a page
+/// refused before any store figure carries no banner, as D-1705 decided, and a
+/// success page is byte-identical to the one that pushed the banner first.
+/// Idempotent, so a page writing eight rungs carries it once. One prefix check
+/// and at most one O(page) insertion per page; never per bar or candidate.
+pub(crate) fn lead_with_pooled_banner(out: &mut String) {
+    if !out.starts_with(crate::STORED_POOLED_PROVENANCE) {
+        out.insert_str(0, crate::STORED_POOLED_PROVENANCE);
+    }
+}
+
 /// Runs the durable all-rung Step-3 chain and returns the operator's report.
 ///
 /// # What it writes
@@ -822,8 +837,10 @@ pub(crate) fn gates_refused_event(verb: &str, missing: usize) -> telemetry::Even
 /// for the whole run, outside any loop over bars or candidates. `CLAUDE.md` §3
 /// rule 4 bounds five per-operation costs and this is none of them.
 pub(crate) fn ledger_all(request: &LedgerAllRequest<'_>) -> String {
+    // NO BANNER YET (G3-2, D-4752): it leads only once a store figure is
+    // written, by `lead_with_pooled_banner`, so a refusal before any read
+    // carries none, as D-1705 decided.
     let mut out = String::new();
-    out.push_str(crate::STORED_POOLED_PROVENANCE);
     let _ = writeln!(
         out,
         "\nLEDGER-ALL  {} {:04}-{:02}..{:04}-{:02}  support {} ppm  stop ceiling {} points",
@@ -843,6 +860,7 @@ pub(crate) fn ledger_all(request: &LedgerAllRequest<'_>) -> String {
 
     match run_chain(request, &mut out) {
         Ok(written) => {
+            lead_with_pooled_banner(&mut out);
             let _ = writeln!(
                 out,
                 "\nCOMMITTED. {written} of {} rung Selection blocks were written rather than \
@@ -923,6 +941,7 @@ fn run_chain(request: &LedgerAllRequest<'_>, out: &mut String) -> Result<usize,
         SELECTION_STAGE,
         selection_written,
     ));
+    lead_with_pooled_banner(out);
     let _ = writeln!(
         out,
         "\nBLOCKS WRITTEN RATHER THAN BYTE-IDENTICALLY REUSED, of {} rungs each\n  \
diff --git a/crates/cli/src/ledger_v6.rs b/crates/cli/src/ledger_v6.rs
index 7b9ff80c..1b55107e 100644
--- a/crates/cli/src/ledger_v6.rs
+++ b/crates/cli/src/ledger_v6.rs
@@ -241,8 +241,9 @@ impl RungRoots {
 /// they are for `ledger-all` -- the two verbs share those, and a second run
 /// reuses rather than rewrites them.
 pub(crate) fn ledger_v6(request: &LedgerAllRequest<'_>) -> String {
+    // NO BANNER YET (G3-2, D-4752), as `ledger_all` states: the route puts it
+    // before the first rung's Selection, and a refusal before any carries none.
     let mut out = String::new();
-    out.push_str(crate::STORED_POOLED_PROVENANCE);
     let _ = writeln!(
         out,
         "\nLEDGER-V6  {} {:04}-{:02}..{:04}-{:02}  support {} ppm  stop ceiling {} points\n\
@@ -263,6 +264,7 @@ pub(crate) fn ledger_v6(request: &LedgerAllRequest<'_>) -> String {
 
     match run_route(request, &mut out) {
         Ok(selections) => {
+            crate::ledger_all::lead_with_pooled_banner(&mut out);
             let rungs = selections.len();
             let _ = writeln!(
                 out,
@@ -408,6 +410,7 @@ fn run_route(
 
         let (execution, summary) = committed_route;
         sizing_inputs.require_current()?;
+        crate::ledger_all::lead_with_pooled_banner(out);
         let selection =
             render_selection(rung, &roots.selection, execution, out).inspect_err(|why| {
                 crate::note(&rung_refused_event(
@@ -659,7 +662,9 @@ pub(crate) fn ledger_v6_replay(
     oos_from: (u16, u8),
     oos_to: (u16, u8),
 ) -> String {
-    let mut out = String::from(crate::STORED_POOLED_PROVENANCE);
+    // NO BANNER YET (G3-2, D-4752): `run_route` puts it before its first
+    // Selection, and a refusal before any carries none.
+    let mut out = String::new();
     let _ = writeln!(
         out,
         "GLOBAL REPLAY V4 — actual Selection V6 prefixes, OOS {}-{:02} through {}-{:02}",
@@ -668,6 +673,7 @@ pub(crate) fn ledger_v6_replay(
     crate::note(&run_started_event("ledger-v6-replay", request));
     match replay_route(request, oos_from, oos_to, &mut out) {
         Ok(audit) => {
+            crate::ledger_all::lead_with_pooled_banner(&mut out);
             if audit.witnesses == 0 {
                 out.push_str("\nNo strategies were selected. This is a complete zero-stream schedule; no OOS market bars or VIX references were loaded, so it does not attest market-data coverage for the requested period.\n");
             }
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index eef0b642..1952b9e2 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -3308,6 +3308,15 @@ pub(crate) fn stored_provenance(underlying: &str) -> String {
     out
 }
 
+/// [`stored_provenance`] for a page over many runs of one symbol: the pooled
+/// banner, which promises no single run, then the same note a stock's page
+/// carries (GAP15-21, G3-2, D-4752).
+pub(crate) fn stored_pooled_provenance(underlying: &str) -> String {
+    let mut out = String::from(STORED_POOLED_PROVENANCE);
+    out.push_str(&stored::equity_note_for(underlying));
+    out
+}
+
 /// What a report over `underlying` states before any figure, for a reader
 /// outside this crate. D-0694, AF-19.
 ///
@@ -17323,6 +17332,9 @@ pub(crate) fn in_input_order<T, R>(items: &[T], each: impl FnMut(&T) -> R) -> Ve
 }
 
 /// The comparable-run provenance and support explanation above a range table.
+///
+/// The table is one run per rung over a span of months, so it opens with the
+/// pooled banner and never the single-run one (GAP15-21, D-4752).
 fn range_opening(
     vendor_word: &str,
     underlying: &str,
@@ -17331,7 +17343,7 @@ fn range_opening(
     support_ppm: Option<u64>,
 ) -> String {
     let (from, to) = span;
-    let mut out = stored_provenance(underlying);
+    let mut out = stored_pooled_provenance(underlying);
     let _ = writeln!(
         out,
         "feed {vendor_word} · {underlying} · {} · {}-{:02}..{}-{:02} · support {}",
@@ -25729,6 +25741,79 @@ mod tests {
         }
     }
 
+    /// **Every page that reports more than one run opens with the pooled
+    /// banner, and none of them with the single-run one.** GAP15-21, G3-2,
+    /// D-1705, D-4752.
+    ///
+    /// D-1705 moved five pages to [`STORED_POOLED_PROVENANCE`] and three more
+    /// kept [`STORED_PROVENANCE`] over many runs: `range-all` (eight rungs over
+    /// a span of months), the single-stop search (several rungs over training
+    /// and later months) and `sweep-all` (every instrument and month). Each
+    /// page's opening function is listed here by file and name; its body must
+    /// name the pooled banner, directly or through
+    /// [`stored_pooled_provenance`] or
+    /// `ledger_all::lead_with_pooled_banner`, and must name neither the
+    /// single-run constant nor the single-run helpers. A new multi-run page is
+    /// added to this list when it is written; each page is also pinned by
+    /// rendering it, beside its own tests.
+    #[test]
+    fn every_multi_run_page_opens_with_the_pooled_banner() {
+        const PAGES: [(&str, &str, &str); 8] = [
+            ("pool.rs", include_str!("pool.rs"), "opening"),
+            ("ledger_v6.rs", include_str!("ledger_v6.rs"), "ledger_v6"),
+            (
+                "ledger_v6.rs",
+                include_str!("ledger_v6.rs"),
+                "ledger_v6_replay",
+            ),
+            ("ledger_all.rs", include_str!("ledger_all.rs"), "ledger_all"),
+            (
+                "boolean_catalog_prepared.rs",
+                include_str!("boolean_catalog_prepared.rs"),
+                "research_heading",
+            ),
+            ("lib.rs", include_str!("lib.rs"), "range_opening"),
+            (
+                "index_stop_search.rs",
+                include_str!("index_stop_search.rs"),
+                "execute_observed_with",
+            ),
+            ("batch.rs", include_str!("batch.rs"), "render"),
+        ];
+        for (file, source, name) in PAGES {
+            let start = ["\nfn ", "\npub(crate) fn ", "\npub fn "]
+                .iter()
+                .find_map(|prefix| source.find(&format!("{prefix}{name}(")))
+                .unwrap_or_else(|| panic!("{file} has no top-level fn {name}"));
+            let body = source
+                .get(start + 1..)
+                .and_then(|rest| rest.find("\n}\n").and_then(|end| rest.get(..end)))
+                .unwrap_or_else(|| panic!("{file}::{name} has no closing brace"));
+            assert!(
+                [
+                    "STORED_POOLED_PROVENANCE",
+                    "stored_pooled_provenance(",
+                    "lead_with_pooled_banner("
+                ]
+                .iter()
+                .any(|pooled| body.contains(pooled)),
+                "{file}::{name} reports many runs and must open with the pooled banner"
+            );
+            for single in [
+                "STORED_PROVENANCE)",
+                "STORED_PROVENANCE,",
+                "STORED_PROVENANCE;",
+                "stored_provenance(",
+                "stored_provenance_of(",
+            ] {
+                assert!(
+                    !body.contains(single),
+                    "{file}::{name} reports many runs and names the single-run banner: {single}"
+                );
+            }
+        }
+    }
+
     /// THE STORED AUDIT SAYS WHICH RUNG ITS TRADES FILLED ON.
     ///
     /// # The same class of defect the two provenance banners exist to refuse
diff --git a/crates/cli/src/pool.rs b/crates/cli/src/pool.rs
index 49961a4d..f5056a6c 100644
--- a/crates/cli/src/pool.rs
+++ b/crates/cli/src/pool.rs
@@ -3773,12 +3773,17 @@ mod tests {
         assert!(refused.contains("REFUSED: TCS refused"), "{refused}");
     }
 
-    /// **`ledger-v6`, `ledger-v6-replay` and `ledger-all` open with the pooled
-    /// banner.** GAP15-21, D-1705.
+    /// **`ledger-v6`, `ledger-v6-replay` and `ledger-all` promise no single
+    /// run, and a page refused before any store figure carries no banner at
+    /// all.** GAP15-21, D-1705, G3-2 (D-4752).
     ///
     /// Each covers eight rungs over a span of months, and each printed the
     /// single-run banner. A feed word no vendor has refuses before any store is
-    /// read, so the page is the banner, its own heading and the refusal.
+    /// read, so the page is its own heading and the refusal. It used to open
+    /// with the pooled banner, "the bars below were read from files", over no
+    /// bars at all, which D-1705's "refusal pages still carry no banner"
+    /// denied. The pooled banner leads only once a store figure is written:
+    /// `the_ledger_pages_take_the_pooled_banner_before_their_first_store_figure`.
     #[test]
     fn the_ledger_pages_promise_no_single_instrument_month_or_identity() {
         let _knobs = crate::knobs::serially();
@@ -3806,9 +3811,10 @@ mod tests {
         ];
         for (verb, page) in pages {
             assert!(
-                page.starts_with(crate::STORED_POOLED_PROVENANCE),
-                "{verb}:\n{page}"
+                !page.contains(crate::STORED_POOLED_PROVENANCE),
+                "{verb}: a refusal before any store read carries no banner:\n{page}"
             );
+            assert!(!page.contains(crate::STORED_PROVENANCE), "{verb}:\n{page}");
             for promise in SINGLE_RUN_PROMISES {
                 assert!(!page.contains(promise), "{verb} {promise}:\n{page}");
             }
@@ -3817,6 +3823,65 @@ mod tests {
         assert!(!root.exists(), "a refused feed creates no tree");
     }
 
+    /// **The ledger pages take the pooled banner before their first store
+    /// figure, once, and a success page is byte-identical to what it was.**
+    /// G3-2, D-4752, D-1705.
+    ///
+    /// The helper puts the banner at the head of the page and is idempotent,
+    /// so a page that writes several rungs carries it once. The source shape
+    /// pins where it is called: `ledger-all`'s chain before `BLOCKS WRITTEN`,
+    /// `ledger-v6`'s route before each rung's Selection rendering, and each
+    /// page's success arm; and none of the three pages starts its text with
+    /// the banner any more, which is what put it on refusal pages.
+    #[test]
+    fn the_ledger_pages_take_the_pooled_banner_before_their_first_store_figure() {
+        let mut page = String::from("\nLEDGER-ALL  zerodha\n");
+        crate::ledger_all::lead_with_pooled_banner(&mut page);
+        assert_eq!(
+            page,
+            format!("{}\nLEDGER-ALL  zerodha\n", crate::STORED_POOLED_PROVENANCE)
+        );
+        page.push_str("Selection 1min\n");
+        crate::ledger_all::lead_with_pooled_banner(&mut page);
+        assert_eq!(page.matches(crate::STORED_POOLED_PROVENANCE).count(), 1);
+        assert!(page.ends_with("Selection 1min\n"), "{page}");
+
+        let body = |source: &'static str, name: &str| -> &'static str {
+            let at = source.find(&format!("fn {name}(")).unwrap_or_default();
+            let rest = source.get(at..).unwrap_or_default();
+            rest.get(..rest.find("\n}\n").unwrap_or(rest.len()))
+                .unwrap_or_default()
+        };
+        let all = include_str!("ledger_all.rs");
+        let v6 = include_str!("ledger_v6.rs");
+        for (name, page) in [
+            ("ledger_all", body(all, "ledger_all")),
+            ("ledger_v6", body(v6, "ledger_v6")),
+            ("ledger_v6_replay", body(v6, "ledger_v6_replay")),
+        ] {
+            assert!(!page.is_empty(), "{name} must exist");
+            assert!(
+                !page.contains("STORED_POOLED_PROVENANCE"),
+                "{name} must not open every page, refusals included, with the banner"
+            );
+            let ok = page.find("Ok(").unwrap_or(usize::MAX);
+            let lead = page.find("lead_with_pooled_banner(").unwrap_or(usize::MAX);
+            let refused = page.find("Err(why)").unwrap_or_default();
+            assert!(
+                ok < lead && lead < refused,
+                "{name}: the success arm leads with it"
+            );
+        }
+        let chain = body(all, "run_chain");
+        let lead = chain.find("lead_with_pooled_banner(out)");
+        let blocks = chain.find("BLOCKS WRITTEN");
+        assert!(lead.is_some() && lead < blocks, "{chain}");
+        let route = body(v6, "run_route");
+        let lead = route.find("lead_with_pooled_banner(out)");
+        let selection = route.find("render_selection(rung");
+        assert!(lead.is_some() && lead < selection, "{route}");
+    }
+
     /// **`in_input_order` runs one call at a time, in input order, even when
     /// the first is the slowest.** GAP13-13, R9-cli-o1-0, D-1701.
     #[test]
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 8ae68c1e..7e33703e 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7060,3 +7060,4 @@ old line regex the same input and watched it pass.
 | L1FD-01 | Boolean research judges each venue against its own close. An eligible share's CAS-day calendar ends at its dated close and receipts under policy 4. The index on the same days is unchanged at policy 3. A day with an unreadable close refuses as "cash session close UNVERIFIED", never as an incomplete store, and a bar past the dated close refuses as the store contradicting its master. A dated day with no close and no bar is `Unmeasured`, and a pre-CAS share receipt expects the calendar yet differs in digest from the index's (D-4748) | `cli::candidate_universe::boolean_candidate_v1::tests::a_cas_share_and_an_index_on_the_same_days_are_each_judged_against_their_own_close` · `cli::stored::tests::a_share_receipt_is_policy_four_and_expects_its_dated_close` | ✓ |
 | L1FD-02 | The cash-closes digest binds each unverified day's reason class (no ISIN, master unusable, or master read and not naming the share, with that master's SHA-256), never the reason's text. Two store roots at different paths with the same unusable master give one digest (D-4750) | `cli::stored::tests::an_unreadable_master_binds_its_reason_class_and_not_its_path` | ✓ |
 | L1FD-03 | `sweep-stored`'s withheld-sessions line names the swept bar count, which the ledger records, beside the retained count, and never calls the retained slice what the sweep uses (D-4751) | `cli::audited_stored::tests::the_ordinary_stored_sweep_withholds_and_names_a_holed_session` | ✓ |
+| L1FD-04 | Every page over many runs opens with `STORED_POOLED_PROVENANCE`, never `STORED_PROVENANCE`. A stock's page adds its equity note, and a ledger page refused before any store figure carries no banner (D-4752) | `cli::tests::every_multi_run_page_opens_with_the_pooled_banner` · `cli::equity_statement_tests::range_all_opens_with_the_pooled_banner_and_a_stocks_note` · `cli::pool::tests::the_ledger_pages_take_the_pooled_banner_before_their_first_store_figure` · `cli::pool::tests::the_ledger_pages_promise_no_single_instrument_month_or_identity` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index f659ea8d..b15b4935 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65170,3 +65170,44 @@ The other bar-count lines were checked:
 Test: `the_ordinary_stored_sweep_withholds_and_names_a_holed_session`
 (L1FD-03), re-pinned on the new sentence. It also asserts that the old
 sentence is gone.
+
+### D-4752 — Every multi-run page opens with the pooled banner, and a ledger refusal carries none — 2026-10-09
+
+**Findings.** GAP15-21 and G3-2 (low). D-1705 moved five pages to
+`STORED_POOLED_PROVENANCE`. Three more pages are also over many runs, and they
+kept `STORED_PROVENANCE`, which promises one identity and "that instrument and
+that month":
+
+- `range-all` (`range_opening`): eight rungs over a span of months.
+- The single-stop search (`index_stop_search::execute_observed_with`): rungs
+  by two month windows.
+- `sweep-all` (`batch::render`): every instrument by every month.
+
+Separately, `ledger-all`, `ledger-v6` and `ledger-v6-replay` pushed the pooled
+banner as their first text. A feed word refused before any store read
+therefore still opened with "the bars below were read from files". D-1705
+says "Refusal pages still carry no banner".
+
+**Decision.**
+
+- The three pages open with the pooled banner. `range-all` uses the new
+  `stored_pooled_provenance(underlying)`: the pooled banner, then the same
+  equity note `stored_provenance` appends for a stock.
+- The ledger pages no longer start with the banner. `ledger_all::lead_with_pooled_banner`
+  puts it at the head of the page, once, immediately before the first
+  store-derived figure:
+  - in `ledger-all`'s chain, before `BLOCKS WRITTEN`;
+  - in `ledger-v6`'s route, before each rung's Selection rendering;
+  - in each page's success arm.
+- A success page is byte-identical to before. A page refused before any store
+  figure carries no banner.
+
+Tests (L1FD-04):
+
+- `every_multi_run_page_opens_with_the_pooled_banner`, a source-shape guard
+  over all eight multi-run page builders;
+- `range_all_opens_with_the_pooled_banner_and_a_stocks_note`;
+- `the_ledger_pages_take_the_pooled_banner_before_their_first_store_figure`;
+- `the_ledger_pages_promise_no_single_instrument_month_or_identity`, now
+  asserting no banner on a refusal;
+- the `sweep-all` and single-stop page tests, re-pinned.
-- 
2.43.0


From 62ed48a4545371ff80f96025c641c456fe36c4ef Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:43:06 +0000
Subject: [PATCH 5/8] cli: pin the audit's per-training walk-forward rungs
 (G3-6)

Before: both_shapes passes walk_forward_rungs() (FoldRungs::PerTraining) to
the anchored walk-forward, but no test failed if the argument went back to
FoldRungs::Fixed(grid_rungs(&bars)), the whole-span count.

After: a test-only thread-local recorder keeps each fold's (train_bars,
resolved_rungs) from audit_bars_work. A new test drives audit_bars over
sessions whose later third is wider and asserts that every fold resolved
grid_rungs of exactly its own training prefix. Production is unchanged.

The test passes on the current code. With the argument temporarily reverted
to Fixed(grid_rungs(&bars)) it failed:
  lib.rs:26780: "the fold of 1500 training bars, whole span 7
  left: Some(7) right: Some(2)"

D-4753, L1FD-05.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/lib.rs | 73 +++++++++++++++++++++++++++++++++++++++++++
 docs/04-invariants.md |  1 +
 docs/05-decisions.md  | 21 +++++++++++++
 3 files changed, 95 insertions(+)

diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 1952b9e2..d1b75188 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -6150,6 +6150,15 @@ fn walk_forward_rungs() -> runner::validate::FoldRungs<'static> {
     )
 }
 
+// Each anchored walk-forward fold's training length and resolved rung count,
+// as the last audit on this thread received them from `both_shapes`, so a test
+// can pin the policy the audit really passed (G3-6, D-4753). Test builds only.
+#[cfg(test)]
+thread_local! {
+    static WALK_FORWARD_FOLD_RUNGS: std::cell::RefCell<Vec<(usize, Option<usize>)>> =
+        const { std::cell::RefCell::new(Vec::new()) };
+}
+
 /// The identity word for [`walk_forward_rungs`]' policy, appended to [`policy_of`] as
 /// its twenty-first term. Zero is never written: an identity minted before the
 /// term existed has twenty terms, and `with_policy` folds the length first.
@@ -22445,6 +22454,14 @@ fn audit_bars_work(
             )
         })
     });
+    #[cfg(test)]
+    WALK_FORWARD_FOLD_RUNGS.with(|seen| {
+        *seen.borrow_mut() = folds
+            .folds
+            .iter()
+            .map(|fold| (fold.train_bars, fold.resolved_rungs))
+            .collect();
+    });
     // PBO, WHICH USED TO BE A `None` FOR A REASON THAT IS NOW FIXED.
     //
     // `pbo::place` ranks a fold's candidates in-sample, finds where the winner
@@ -26713,6 +26730,62 @@ mod tests {
         );
     }
 
+    /// G3-6, D-4753: the AUDIT hands `both_shapes` the per-training policy,
+    /// so each anchored fold prices with the rung count of exactly its own
+    /// training bars. The test above pins `walk_forward_rungs` itself; nothing
+    /// pinned its call site, and reverting that argument to the whole-span
+    /// count (`FoldRungs::Fixed(grid_rungs(&bars))`) passed every cli test
+    /// while identity term 21 still claimed per-fold sizing. The final third
+    /// is widened so the whole span's count differs from each fold's.
+    #[test]
+    fn the_audit_prices_each_walk_forward_fold_with_its_own_training_rung_count() {
+        let _guard = crate::knobs::serially();
+        crate::knobs::clear_all();
+        let mut bars = synthetic::sessions(12);
+        let late = bars.len() / 3 * 2;
+        for bar in bars.get_mut(late..).expect("the final third") {
+            bar.high = bar.high.saturating_add(bar.high / 50);
+        }
+        let whole = grid_rungs(&bars);
+        super::WALK_FORWARD_FOLD_RUNGS.with(|seen| seen.borrow_mut().clear());
+        let report = super::audit_bars(
+            &evaluator(),
+            bars.clone(),
+            "GENERATED TEST FIXTURE",
+            1_400,
+            None,
+            super::AuditOptions {
+                prepared_column: None,
+                replay: None,
+                execution: None,
+                native_minute_execution: true,
+                recording: None,
+                rules: crate::Rules::BASELINE,
+                lens: runner::rank::Lens::Detectability,
+                ceiling: Some(50_000),
+                validate: true,
+                cost: runner::audit::CostScope::IndexSpot,
+            },
+        );
+        let seen = super::WALK_FORWARD_FOLD_RUNGS.with(std::cell::RefCell::take);
+        assert!(!seen.is_empty(), "the audit walked no fold:\n{report}");
+        for (train, resolved) in &seen {
+            let training = bars
+                .get(..*train)
+                .expect("an anchored fold trains on a prefix of the span");
+            assert_eq!(
+                *resolved,
+                Some(grid_rungs(training)),
+                "the fold of {train} training bars, whole span {whole}"
+            );
+        }
+        assert!(
+            seen.iter().any(|(_, resolved)| *resolved != Some(whole)),
+            "premise: some fold's own count must differ from the whole span's {whole}, \
+             or a revert to the whole-span count would pass: {seen:?}"
+        );
+    }
+
     /// The eight rules `Rules::operator()` made variable are each folded.
     ///
     /// Split from the sibling above to stay under `clippy::too_many_lines`,
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 7e33703e..1360e905 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7061,3 +7061,4 @@ old line regex the same input and watched it pass.
 | L1FD-02 | The cash-closes digest binds each unverified day's reason class (no ISIN, master unusable, or master read and not naming the share, with that master's SHA-256), never the reason's text. Two store roots at different paths with the same unusable master give one digest (D-4750) | `cli::stored::tests::an_unreadable_master_binds_its_reason_class_and_not_its_path` | ✓ |
 | L1FD-03 | `sweep-stored`'s withheld-sessions line names the swept bar count, which the ledger records, beside the retained count, and never calls the retained slice what the sweep uses (D-4751) | `cli::audited_stored::tests::the_ordinary_stored_sweep_withholds_and_names_a_holed_session` | ✓ |
 | L1FD-04 | Every page over many runs opens with `STORED_POOLED_PROVENANCE`, never `STORED_PROVENANCE`. A stock's page adds its equity note, and a ledger page refused before any store figure carries no banner (D-4752) | `cli::tests::every_multi_run_page_opens_with_the_pooled_banner` · `cli::equity_statement_tests::range_all_opens_with_the_pooled_banner_and_a_stocks_note` · `cli::pool::tests::the_ledger_pages_take_the_pooled_banner_before_their_first_store_figure` · `cli::pool::tests::the_ledger_pages_promise_no_single_instrument_month_or_identity` | ✓ |
+| L1FD-05 | The audit's anchored walk-forward resolves each fold's exit rungs from exactly that fold's training prefix, not the whole span (D-4753) | `cli::tests::the_audit_prices_each_walk_forward_fold_with_its_own_training_rung_count` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index b15b4935..8fa0a395 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65211,3 +65211,24 @@ Tests (L1FD-04):
 - `the_ledger_pages_promise_no_single_instrument_month_or_identity`, now
   asserting no banner on a refusal;
 - the `sweep-all` and single-stop page tests, re-pinned.
+
+### D-4753 — The audit's walk-forward is pinned to price each fold on its own training rungs — 2026-10-09
+
+**Finding.** G3-6 (test gap), GAP4-46's cli wiring. `both_shapes` passes
+`walk_forward_rungs()` to the anchored walk-forward. That is
+`FoldRungs::PerTraining(&grid_rungs)`, so each fold resolves its exit rungs
+from its own training slice. No test failed if the argument went back to
+`FoldRungs::Fixed(grid_rungs(&bars))`, the whole-span count. Under that count
+every fold reads rungs sized by bars it may not see.
+
+**Decision.** A test-only recorder, `WALK_FORWARD_FOLD_RUNGS`, keeps each
+anchored fold's `(train_bars, resolved_rungs)` from `audit_bars_work`. The new
+test drives `audit_bars` over generated sessions whose later third is wider,
+so the whole-span rung count differs from at least one fold's training count.
+It asserts every fold resolved `grid_rungs` of exactly its own training
+prefix. Production behaviour is unchanged.
+
+Test: `the_audit_prices_each_walk_forward_fold_with_its_own_training_rung_count`
+(L1FD-05). It passes on the current code. Proof by mutation: with the call
+reverted to `Fixed(grid_rungs(&bars))` it fails, and the recorded failure is
+in the commit body.
-- 
2.43.0


From 555b90d4b46798e6e9badcac0aa7cde2cae0211f Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:43:07 +0000
Subject: [PATCH 6/8] cli: name a CAS prior session past its dated close
 accurately (G3-7)

Before: an eligible share's CAS-day prior session ending at 15:29 lies past
its dated 15:14 close, but the refusal said "Early or truncated bars cannot
seed GapFib", and DCC-01 pinned "truncated".

After: the terminal-geometry refusal names the cause from the observed last
minute. Past the close it says "Its final bar runs past its dated session
close, so the store holds bars that close says cannot exist, and they cannot
seed GapFib". At or short of the close the message is byte-identical to
before. The same inputs refuse and pass as before.

Fail before:
  stored.rs:5425: "...observed final three were Some(927), Some(928),
  Some(929). Early or truncated bars cannot seed GapFib"

D-4754, L1FD-06.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/stored.rs | 41 ++++++++++++++++++++++++++++++++++------
 docs/04-invariants.md    |  1 +
 docs/05-decisions.md     | 26 +++++++++++++++++++++++++
 3 files changed, 62 insertions(+), 6 deletions(-)

diff --git a/crates/cli/src/stored.rs b/crates/cli/src/stored.rs
index a20802ae..5ccba417 100644
--- a/crates/cli/src/stored.rs
+++ b/crates/cli/src/stored.rs
@@ -3641,6 +3641,22 @@ pub fn load_daily_context_bounded(
     daily_context_from_span(daily, signal)
 }
 
+/// Why a prior session whose terminal geometry is wrong cannot seed `GapFib`,
+/// by which side of its close the final bar fell (G3-7, D-4754).
+///
+/// PAST THE CLOSE IS NOT SHORT OF IT. An eligible share's CAS day ending 15:29
+/// holds bars after its dated 15:14 close: the store contradicts the close it
+/// was judged against, a different fault from a session that stopped early,
+/// and it was refused as "truncated". A final bar at or before the close keeps
+/// the old words, byte for byte.
+fn prior_session_terminal_fault(last: Option<u16>, close: u16) -> &'static str {
+    if last > Some(close) {
+        "Its final bar runs past its dated session close, so the store holds bars that close says cannot exist, and they"
+    } else {
+        "Early or truncated bars"
+    }
+}
+
 /// The minute the accepted prior session's final bar must open at (GAP12-6,
 /// D-1663, D-2102). `kind_of` is the index's venue-blind calendar, and from
 /// 2026-08-03 an NSE cash share's continuous session ends at 15:15 when that
@@ -3766,7 +3782,8 @@ pub(crate) fn exact_minute_context_from_span(
         || last != Some(last_minute)
     {
         return Err(format!(
-            "the accepted prior exact 1min session on IST day {prior_session_day} does not end with canonical terminal-minute geometry {expected_third}, {expected_second}, {last_minute}; observed final three were {third_last:?}, {second_last:?}, {last:?}. Early or truncated bars cannot seed GapFib"
+            "the accepted prior exact 1min session on IST day {prior_session_day} does not end with canonical terminal-minute geometry {expected_third}, {expected_second}, {last_minute}; observed final three were {third_last:?}, {second_last:?}, {last:?}. {} cannot seed GapFib",
+            prior_session_terminal_fault(last, last_minute)
         ));
     }
     let prior_session_bars = u32::try_from(prior_session_bars).map_err(|_| {
@@ -5554,12 +5571,23 @@ mod tests {
 
     /// GAP12-6 closed (D-2102): a share's CAS-day prior session is judged
     /// against ITS dated close. Eligible ends at 15:14 and seeds; the same day
-    /// ending 15:29 is truncated against that close. Ineligible is the mirror.
-    /// The flag is read from the receipted master; nothing is downloaded.
+    /// ending 15:29 runs PAST that close, which contradicts the master and is
+    /// named so, never "truncated" (G3-7, D-4754). Ineligible ending 15:14 is
+    /// short of its 15:29 close: that is the truncated case. The flag is read
+    /// from the receipted master; nothing is downloaded.
     #[test]
     fn a_cas_prior_session_is_judged_against_the_shares_dated_close() {
         let signal = [minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000)];
-        for (flag, seeds, truncated) in [(1_u8, 914_i64, 929_i64), (0, 929, 914)] {
+        for (flag, seeds, other, cause, not_cause) in [
+            (
+                1_u8,
+                914_i64,
+                929_i64,
+                "runs past its dated session close",
+                "truncated",
+            ),
+            (0, 929, 914, "Early or truncated", "runs past"),
+        ] {
             let store = root(&format!("cas-dated-{flag}"));
             install_master(&store, OPEN_MONDAY_2026_08_03, flag);
             let context =
@@ -5582,9 +5610,10 @@ mod tests {
             assert_eq!(context.session_close_minute(OPEN_TUESDAY_2026_08_04), None);
             assert_eq!(context.cash_digest(), Some(cash.digest()));
             let why =
-                exact_minute_context_from_span(reliance_prior_session(truncated), &signal, &store)
+                exact_minute_context_from_span(reliance_prior_session(other), &signal, &store)
                     .expect_err("the other close is not this share's");
-            assert!(why.contains("truncated"), "{why}");
+            assert!(why.contains(cause), "{why}");
+            assert!(!why.contains(not_cause), "{why}");
             assert!(
                 why.contains(&format!("{}, {}, {seeds}", seeds - 2, seeds - 1)),
                 "{why}"
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 1360e905..b8148bc6 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7062,3 +7062,4 @@ old line regex the same input and watched it pass.
 | L1FD-03 | `sweep-stored`'s withheld-sessions line names the swept bar count, which the ledger records, beside the retained count, and never calls the retained slice what the sweep uses (D-4751) | `cli::audited_stored::tests::the_ordinary_stored_sweep_withholds_and_names_a_holed_session` | ✓ |
 | L1FD-04 | Every page over many runs opens with `STORED_POOLED_PROVENANCE`, never `STORED_PROVENANCE`. A stock's page adds its equity note, and a ledger page refused before any store figure carries no banner (D-4752) | `cli::tests::every_multi_run_page_opens_with_the_pooled_banner` · `cli::equity_statement_tests::range_all_opens_with_the_pooled_banner_and_a_stocks_note` · `cli::pool::tests::the_ledger_pages_take_the_pooled_banner_before_their_first_store_figure` · `cli::pool::tests::the_ledger_pages_promise_no_single_instrument_month_or_identity` | ✓ |
 | L1FD-05 | The audit's anchored walk-forward resolves each fold's exit rungs from exactly that fold's training prefix, not the whole span (D-4753) | `cli::tests::the_audit_prices_each_walk_forward_fold_with_its_own_training_rung_count` | ✓ |
+| L1FD-06 | A CAS prior session ending past its dated close is refused as running past it, and one ending short of it as truncated. Neither message names the other cause (D-4754) | `cli::stored::tests::a_cas_prior_session_is_judged_against_the_shares_dated_close` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 8fa0a395..1802944e 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65232,3 +65232,29 @@ Test: `the_audit_prices_each_walk_forward_fold_with_its_own_training_rung_count`
 (L1FD-05). It passes on the current code. Proof by mutation: with the call
 reverted to `Fixed(grid_rungs(&bars))` it fails, and the recorded failure is
 in the commit body.
+
+### D-4754 — A CAS prior session that runs past its dated close is named so, not "truncated" — 2026-10-09
+
+**Finding.** G3-7 (low, wording). Suppose an eligible share's CAS-day prior
+session ends at 15:29. Its last bars lie past the dated 15:14 close, which
+contradicts its own master. `exact_minute_context_from_span` refused it as
+"Early or truncated bars cannot seed GapFib", and DCC-01 pinned that word.
+GAP12-6's original complaint was this same misnamed cause.
+
+**Decision.** The terminal-geometry refusal keeps its one message and names
+the fault from the observed last minute (`prior_session_terminal_fault`):
+
+- **Past the session's last minute:** "Its final bar runs past its dated
+  session close, so the store holds bars that close says cannot exist, and
+  they cannot seed GapFib".
+- **At or short of it:** "Early or truncated bars cannot seed GapFib",
+  byte-identical to before.
+
+Both still state the canonical terminal geometry and the observed final three
+minutes. Only the named cause changes. Every input that refused still
+refuses, and every input that passed still passes.
+
+Test: DCC-01, `a_cas_prior_session_is_judged_against_the_shares_dated_close`
+(L1FD-06). It is re-pinned: the eligible share ending at 15:29 names
+"runs past its dated session close" and not "truncated", and the ineligible
+share ending at 15:14 names "Early or truncated" and not "runs past".
-- 
2.43.0


From 4e30e488b976761c240b2446874dab86d2c88483 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:43:07 +0000
Subject: [PATCH 7/8] cli: validation evidence uses the audit's exact placement
 (G3-9)

Before: ValidationEvidenceV1::from_runner derived its fold diagnostic with
the legacy runner::pbo::place adapter, and refused a supplied Pbo that
differed. The audit's Pbo has been exact since D-1724, so handing it the
audit's own figure would refuse on any exact half-rank fold.

After: derive_anchored_fold_diagnostic is crate::overfitting_of, the
function the audit page uses. Its absent case refuses by name.

Fail before:
  institutional_evidence.rs:3056: "the audit's exact diagnostic must
  reconcile"

D-4755, L1FD-07.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/institutional_evidence.rs | 93 ++++++++++++++++++------
 docs/04-invariants.md                    |  1 +
 docs/05-decisions.md                     | 24 ++++++
 3 files changed, 94 insertions(+), 24 deletions(-)

diff --git a/crates/cli/src/institutional_evidence.rs b/crates/cli/src/institutional_evidence.rs
index 22008879..53e462c8 100644
--- a/crates/cli/src/institutional_evidence.rs
+++ b/crates/cli/src/institutional_evidence.rs
@@ -67,7 +67,7 @@ use runner::admission::{
 };
 use runner::bootstrap::{RomanoWolfReceipt, Verdict};
 use runner::grid::{Cell, TradeRow};
-use runner::pbo::{Pbo, Placement, place, probability_of_overfitting};
+use runner::pbo::Pbo;
 use runner::validate::Validated;
 
 use crate::institutional_statistics::InstitutionalStatisticsAuthorityV1;
@@ -563,7 +563,7 @@ impl ValidationEvidenceV1 {
             }
         }
 
-        let legacy_diagnostic = derive_anchored_fold_legacy_diagnostic(validated)?;
+        let legacy_diagnostic = derive_anchored_fold_diagnostic(validated)?;
         if supplied_legacy_diagnostic.is_some_and(|supplied| *supplied != legacy_diagnostic) {
             return Err(
                 "supplied anchored-fold legacy diagnostic does not equal the fold-derived diagnostic"
@@ -612,28 +612,20 @@ impl ValidationEvidenceV1 {
     }
 }
 
-fn derive_anchored_fold_legacy_diagnostic(validated: &Validated) -> Result<Pbo, String> {
-    let mut placements = Vec::new();
-    placements
-        .try_reserve_exact(validated.folds.len())
-        .map_err(|why| format!("could not reserve anchored-fold legacy placements: {why}"))?;
-    for fold in &validated.folds {
-        let placement = if fold.in_sample_all.is_empty() {
-            Placement {
-                candidates: 0,
-                winner_rank: 0,
-            }
-        } else {
-            place(&fold.in_sample_all, &fold.out_of_sample_all).ok_or_else(|| {
-                format!(
-                    "walk-forward fold {} could not produce an aligned legacy placement",
-                    fold.index
-                )
-            })?
-        };
-        placements.push(placement);
-    }
-    Ok(probability_of_overfitting(&placements))
+/// The fold-derived diagnostic: the audit's own exact placement, through
+/// [`crate::overfitting_of`], so the evidence and the audit page compute one
+/// figure from one function (G3-9, D-4755). This was the legacy adapter
+/// (`runner::pbo::place` and `probability_of_overfitting`), which rounds an
+/// exact half-rank toward the better half; D-1724 moved the audit off it, and
+/// a supplied audit figure would then have been refused on any such fold.
+/// `from_runner` has already returned for no folds and refused misaligned
+/// ones, so the absent arm is unreachable here; it refuses by name rather than
+/// defaulting.
+fn derive_anchored_fold_diagnostic(validated: &Validated) -> Result<Pbo, String> {
+    crate::overfitting_of(validated).ok_or_else(|| {
+        "walk-forward folds have no exact placement: absent, or carrying unaligned candidate families"
+            .to_owned()
+    })
 }
 
 /// Identity-bound family-test projections available from current runner APIs.
@@ -3014,6 +3006,59 @@ mod tests {
         assert!(why.contains("misaligned in/out-of-sample candidate families"));
     }
 
+    /// G3-9, D-4755: the diagnostic this constructor derives is the audit's
+    /// own exact placement (`crate::overfitting_of`, D-1724), so the figure a
+    /// live audit prints is accepted when supplied, and the legacy adapter's
+    /// rounder figure is refused. The fold's winner ties one rival out of
+    /// sample below a third: midrank 1.5 of 2, strictly in the bottom half.
+    /// The legacy adapter halves the doubled rank to 1, the median, and counts
+    /// no overfit fold; the exact placement counts one.
+    #[test]
+    fn the_supplied_diagnostic_is_the_audits_exact_placement_not_the_legacy_adapter() {
+        let fold = FoldResult {
+            index: 0,
+            considered: 3,
+            priced: 3,
+            chosen: Some(
+                runner::replay_mask::from_stored_words([1, 0, 0, 0, 0, 0])
+                    .expect("bit zero is a live canonical condition"),
+            ),
+            out_of_sample_exit: Some(10),
+            in_sample_all: vec![10, 1, 1],
+            out_of_sample_all: vec![5, 9, 5],
+            ..FoldResult::default()
+        };
+        let validated = Validated {
+            folds: vec![fold],
+            refused: None,
+        };
+        let exact = crate::overfitting_of(&validated).expect("one rankable fold");
+        assert_eq!(
+            (exact.folds, exact.overfit_folds, exact.median_placement),
+            (1, 1, 750_000),
+            "premise: the audit's exact figure counts the half-rank"
+        );
+        let legacy = probability_of_overfitting(&[
+            place(&[10, 1, 1], &[5, 9, 5]).expect("aligned legacy placement")
+        ]);
+        assert_ne!(
+            legacy, exact,
+            "premise: the two placements disagree on this fold, or this proves nothing"
+        );
+        assert!(
+            ValidationEvidenceV1::from_runner(digest(1), digest(2), &validated, Some(&exact))
+                .is_ok_and(|evidence| matches!(evidence, EvidenceSourceV1::Measured(_))),
+            "the audit's exact diagnostic must reconcile"
+        );
+        let why =
+            ValidationEvidenceV1::from_runner(digest(1), digest(2), &validated, Some(&legacy))
+                .expect_err("the legacy adapter's figure is not this fold set's");
+        assert!(
+            why.contains("does not equal the fold-derived diagnostic"),
+            "{why}"
+        );
+    }
+
     fn generated_validation_fold() -> FoldResult {
         FoldResult {
             index: 0,
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index b8148bc6..b8d02ff2 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7063,3 +7063,4 @@ old line regex the same input and watched it pass.
 | L1FD-04 | Every page over many runs opens with `STORED_POOLED_PROVENANCE`, never `STORED_PROVENANCE`. A stock's page adds its equity note, and a ledger page refused before any store figure carries no banner (D-4752) | `cli::tests::every_multi_run_page_opens_with_the_pooled_banner` · `cli::equity_statement_tests::range_all_opens_with_the_pooled_banner_and_a_stocks_note` · `cli::pool::tests::the_ledger_pages_take_the_pooled_banner_before_their_first_store_figure` · `cli::pool::tests::the_ledger_pages_promise_no_single_instrument_month_or_identity` | ✓ |
 | L1FD-05 | The audit's anchored walk-forward resolves each fold's exit rungs from exactly that fold's training prefix, not the whole span (D-4753) | `cli::tests::the_audit_prices_each_walk_forward_fold_with_its_own_training_rung_count` | ✓ |
 | L1FD-06 | A CAS prior session ending past its dated close is refused as running past it, and one ending short of it as truncated. Neither message names the other cause (D-4754) | `cli::stored::tests::a_cas_prior_session_is_judged_against_the_shares_dated_close` | ✓ |
+| L1FD-07 | Validation evidence reconciles a supplied fold diagnostic against the audit's exact placement (`cli::overfitting_of`). The legacy adapter's figure is refused where the two differ (D-4755) | `cli::institutional_evidence::tests::the_supplied_diagnostic_is_the_audits_exact_placement_not_the_legacy_adapter` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 1802944e..cb7571ac 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65258,3 +65258,27 @@ Test: DCC-01, `a_cas_prior_session_is_judged_against_the_shares_dated_close`
 (L1FD-06). It is re-pinned: the eligible share ending at 15:29 names
 "runs past its dated session close" and not "truncated", and the ineligible
 share ending at 15:14 names "Early or truncated" and not "runs past".
+
+### D-4755 — Validation evidence reconciles against the audit's exact placement — 2026-10-09
+
+**Finding.** G3-9 (low, latent). `ValidationEvidenceV1::from_runner` derived
+its fold diagnostic with the legacy adapter: `runner::pbo::place` and
+`probability_of_overfitting`. It refused a supplied `Pbo` that differed. The
+live audit's `Pbo` has been exact since D-1724 (`cli::overfitting_of`, through
+`place_v1` and `anchored_walk_forward_bottom_half_rate_v1`). So wiring this
+constructor with the audit's own figure would refuse on any exact half-rank
+fold.
+
+**Decision.** `derive_anchored_fold_diagnostic` is `crate::overfitting_of`. The
+evidence and the audit page now compute one figure from one function. Its
+absent case is unreachable here, because `from_runner` returns `Unmeasured`
+for no folds and refuses misaligned ones first. It still refuses by name
+rather than defaulting. The PBO admission fields stay `Unmeasured`, as
+before.
+
+**What changes.** No stored output changes, because there is no production
+caller. A test that supplied the legacy figure for a fold where the two
+differ now gets the refusal.
+
+Test: `the_supplied_diagnostic_is_the_audits_exact_placement_not_the_legacy_adapter`
+(L1FD-07). In its one fold the two figures differ.
-- 
2.43.0


From 20577d698a7907a308dcea2e2c402b02b59527ef Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:43:27 +0000
Subject: [PATCH 8/8] cli: fold each daily bar at its IST midnight in cli
 verify (F-CEC7A0)

Before: V-04 (no time-of-day or VWAP bit on the daily rung) was proved only
for daily bars stamped at IST midnight. The store admits a daily bar at any
whole second, and cli verify folded the stored stamps unchanged, so a daily
series stamped at the 09:15 open set early_morning (position 44) on every
row.

After: verify_series restamps each 1day bar at its IST day's midnight before
the evaluator sees it. Two daily bars on one IST day are refused by name.
Intraday spans pass through unchanged. This is read-side only: no stored
byte, format version or ingest path changes. The F-CEC7A0 row stays OPEN.
A docs/11 narrative bullet marks it IN PROGRESS.

Fail before:
  verify_daily_tests.rs:180: "NIFTY 0915: V-04 positions set on the daily
  rung: {44}"
  verify_daily_tests.rs:209: "one IST day cannot hold two daily bars" (the
  fold returned Ok)

D-4756, L1FD-08.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/lib.rs                |  49 +++++-
 crates/cli/src/verify_daily_tests.rs | 244 +++++++++++++++++++++++++++
 docs/04-invariants.md                |   1 +
 docs/05-decisions.md                 |  47 ++++++
 docs/06-limits.md                    |  10 ++
 docs/11-findings.md                  |   7 +
 6 files changed, 356 insertions(+), 2 deletions(-)
 create mode 100644 crates/cli/src/verify_daily_tests.rs

diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index d1b75188..239c2c79 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -97,6 +97,8 @@ mod readonly_file;
 mod results_report_tests;
 #[cfg(test)]
 mod screen_policy_tests;
+#[cfg(test)]
+mod verify_daily_tests;
 
 #[cfg_attr(
     not(test),
@@ -8550,10 +8552,53 @@ pub fn verify(vendor_word: &str, underlying: &str) -> String {
     out
 }
 
+/// The first `limit` bars of `span` exactly as `cli verify` folds them.
+///
+/// ON THE DAILY RUNG EACH BAR IS RESTAMPED AT ITS IST DAY'S MIDNIGHT, the
+/// stamp `pull::fold` writes for that rung, before the evaluator sees it
+/// (F-CEC7A0, D-4756). The store admits a daily bar at any whole second and
+/// vendors stamp one at midnight, the open or the close (D-0915); folded as
+/// stored, a 09:15 daily bar landed inside the session and set `early_morning`
+/// (position 44), so V-04 held only for the midnight stamp. Read-side only:
+/// no stored byte changes, and prices, volume and the IST day are kept. Two
+/// daily bars on one IST day are refused by name, never folded as two sessions
+/// or merged into one. Two Euclidean divisions and one comparison per bar,
+/// O(`limit`).
+fn verify_series(span: &stored::Span, limit: usize) -> Result<Vec<indicators::Candle>, String> {
+    const DAY_MICROS: i64 = 86_400_000_000;
+    if span.timeframe != "1day" {
+        return Ok(span.bars.iter().take(limit).copied().collect());
+    }
+    let mut previous = None;
+    span.bars
+        .iter()
+        .take(limit)
+        .map(|bar| {
+            let day = indicators::ist_day(bar.ts_micros);
+            if previous == Some(day) {
+                return Err(format!(
+                    "the stored daily series holds two daily bars on IST day {day}; `cli verify` folds one bar per IST session and refuses rather than choose one"
+                ));
+            }
+            previous = Some(day);
+            // The time into its IST day, removed: `ist_day`'s own arithmetic,
+            // so the restamped bar keeps exactly the day it was stored on.
+            let into_day = bar
+                .ts_micros
+                .saturating_add(indicators::IST_OFFSET_MICROS)
+                .rem_euclid(DAY_MICROS);
+            Ok(indicators::Candle {
+                ts_micros: bar.ts_micros.saturating_sub(into_day),
+                ..*bar
+            })
+        })
+        .collect()
+}
+
 /// Matching refusals or empty columns cannot establish measured properties.
 fn measured_series_checks(span: &stored::Span) -> Result<[Check; 2], String> {
     let determinism = {
-        let short: Vec<indicators::Candle> = span.bars.iter().take(600).copied().collect();
+        let short = verify_series(span, 600)?;
         // THIS IS THE SWEEP, NOT A TRADE AUDIT. The loaded series is daily and
         // therefore cannot legally enter `audit_bars` without a separately
         // stored one-minute execution path. Sending it through that door would
@@ -8594,7 +8639,7 @@ fn measured_series_checks(span: &stored::Span) -> Result<[Check; 2], String> {
     // conditions must not change because LATER bars exist, so a column built on
     // a prefix must agree with the same rows of a column built on the whole.
     let causality = {
-        let whole: Vec<indicators::Candle> = span.bars.iter().take(900).copied().collect();
+        let whole = verify_series(span, 900)?;
         let prefix: Vec<indicators::Candle> = whole
             .iter()
             .take(whole.len().saturating_sub(1).min(600))
diff --git a/crates/cli/src/verify_daily_tests.rs b/crates/cli/src/verify_daily_tests.rs
new file mode 100644
index 00000000..4f6233e1
--- /dev/null
+++ b/crates/cli/src/verify_daily_tests.rs
@@ -0,0 +1,244 @@
+//! V-04 on every daily stamp the store admits, folded as `cli verify` folds
+//! it. F-CEC7A0, D-4756.
+//!
+//! D-1790 proved V-04 for daily bars stamped at IST midnight, in `indicators`.
+//! The store admits a daily bar at any whole second (`store::file`'s
+//! `OffGrid` doc, D-0915: vendors stamp a daily bar at midnight, the open or
+//! the close), and `cli verify` folds the stored daily series through the
+//! production evaluator. These tests write daily bars at the three stamps
+//! through `store`, load them with `stored::load_span`, and fold them through
+//! [`super::verify_series`], [`super::evaluator_stored`] and `Column::build`,
+//! the path `measured_series_checks` takes.
+#![allow(clippy::expect_used, reason = "a failed fixture must fail its test")]
+
+use std::collections::BTreeSet;
+use std::path::{Path, PathBuf};
+
+use indicators::column::Column;
+use store::file::BarFile;
+use store::format::Bar;
+use store::path::{FileKind, StorePath, Timeframe, YearMonth};
+
+/// The months each fixture store holds: fifteen, so a daily series warms.
+const MONTHS: [(u16, u8); 15] = [
+    (2024, 1),
+    (2024, 2),
+    (2024, 3),
+    (2024, 4),
+    (2024, 5),
+    (2024, 6),
+    (2024, 7),
+    (2024, 8),
+    (2024, 9),
+    (2024, 10),
+    (2024, 11),
+    (2024, 12),
+    (2025, 1),
+    (2025, 2),
+    (2025, 3),
+];
+
+const DAY_MICROS: i64 = 86_400_000_000;
+
+fn temp_root(tag: &str) -> PathBuf {
+    let root = std::env::temp_dir().join(format!(
+        "brutex-verify-daily-{tag}-{}-{:?}",
+        std::process::id(),
+        std::thread::current().id()
+    ));
+    let _ignored = std::fs::remove_dir_all(&root);
+    std::fs::create_dir_all(&root).expect("a fixture store root");
+    root
+}
+
+/// One daily bar per measured open IST day of `MONTHS`, every bar stamped at
+/// IST minute-of-day `minute` (and `extra`, when given, a second bar on the
+/// first open day at that minute), written through `store` for `underlying`.
+fn daily_store(tag: &str, underlying: &str, minute: i64, extra: Option<i64>) -> PathBuf {
+    let root = temp_root(tag);
+    let key = crate::stored::swept_index(underlying).expect("a swept instrument");
+    let symbol = u32::from_le_bytes(
+        brutex_core::universe::fnv1a(underlying)
+            .to_le_bytes()
+            .get(..4)
+            .and_then(|prefix| prefix.try_into().ok())
+            .expect("four bytes"),
+    );
+    let mut ordinal = 0_i64;
+    for (year, month) in MONTHS {
+        let mut bars = Vec::new();
+        for day in 1..=31 {
+            let Ok(date) = pull::session::Day::new(year, month, day) else {
+                continue;
+            };
+            let civil = i64::from(date.days_from_epoch());
+            if !matches!(
+                pull::calendar::kind_of(civil),
+                pull::calendar::DayKind::Open(_)
+            ) || indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&civil)
+            {
+                continue;
+            }
+            let at = |minute: i64| {
+                civil * DAY_MICROS + minute * 60_000_000 - indicators::IST_OFFSET_MICROS
+            };
+            let mid = 2_500_000 + ((ordinal * 137) % 811 - 405) * 70;
+            let bar = |ts_micros: i64| Bar {
+                ts_micros,
+                open: mid,
+                high: mid + 9_000 + (ordinal % 11) * 200,
+                low: mid - 9_000 - (ordinal % 7) * 200,
+                close: mid + ((ordinal % 5) - 2) * 400,
+                volume: 1_000 + ordinal,
+                open_interest: i64::MIN,
+            };
+            bars.push(bar(at(minute)));
+            if let Some(second) = extra.filter(|_| ordinal == 0) {
+                bars.push(bar(at(second)));
+            }
+            ordinal += 1;
+        }
+        let path = StorePath::for_key(
+            brutex_core::vendor::Vendor::Zerodha,
+            &key,
+            Timeframe::DAY_1,
+            YearMonth::new(year, month).expect("a month"),
+            FileKind::Bars,
+        )
+        .expect("a store path");
+        let mut writer = BarFile::open_or_create(&root, path, symbol).expect("a daily file");
+        writer
+            .append(&bars)
+            .expect("the store admits every stamp here");
+    }
+    root
+}
+
+fn load(root: &Path, underlying: &str) -> crate::stored::Span {
+    crate::stored::load_span(
+        root,
+        brutex_core::vendor::Vendor::Zerodha,
+        underlying,
+        "1day",
+        *MONTHS.first().expect("a first month"),
+        *MONTHS.last().expect("a last month"),
+    )
+    .expect("the daily span loads")
+}
+
+/// Every vocabulary position set in any row of `rows`.
+fn set_positions(rows: &[vocab::ConditionMask]) -> BTreeSet<u16> {
+    (0..u16::try_from(vocab::table::TABLE.len()).expect("the table fits u16"))
+        .filter(|&p| rows.iter().any(|row| row.get(u32::from(p))))
+        .collect()
+}
+
+/// **V-04 holds for a daily bar stamped at midnight, at the 09:15 open and
+/// at the 15:30 close, for an index and for a share, folded as `cli verify`
+/// folds it.** F-CEC7A0, D-4756.
+///
+/// Before D-4756 the 09:15 stamp set `early_morning` (bit 44) on every daily
+/// row: `cli verify` folded the stored stamps as they were, and V-04 held only
+/// because a midnight bar is before the open. The three stamps now fold to the
+/// same column, bit for bit, known mask included.
+#[test]
+fn every_daily_stamp_the_store_admits_folds_with_no_time_of_day_or_vwap_bit() {
+    let mut cleared: BTreeSet<u16> = [44, 45, 46, 47].into_iter().collect();
+    cleared.extend(indicators::vwap::positions());
+    for underlying in ["NIFTY", "RELIANCE"] {
+        let mut columns = Vec::new();
+        for (stamp, minute) in [("0000", 0_i64), ("0915", 555), ("1530", 930)] {
+            let root = daily_store(&format!("{underlying}-{stamp}"), underlying, minute, None);
+            let span = load(&root, underlying);
+            assert!(span.complete(), "{underlying} {stamp}: every month is held");
+            let series = super::verify_series(&span, usize::MAX).expect("the series folds");
+            assert_eq!(series.len(), span.bars.len(), "no bar is dropped");
+            for (folded, stored) in series.iter().zip(&span.bars) {
+                assert_eq!(
+                    indicators::ist_day(folded.ts_micros),
+                    indicators::ist_day(stored.ts_micros),
+                    "a daily bar keeps its IST day"
+                );
+                assert_eq!(
+                    (folded.open, folded.high, folded.low, folded.close),
+                    (stored.open, stored.high, stored.low, stored.close),
+                );
+                assert_eq!(folded.volume, stored.volume);
+            }
+            let mut evaluator =
+                super::evaluator_stored(crate::stored::vwap_availability(&span.key))
+                    .expect("the pinned evaluator");
+            let column = Column::build(&series, &mut evaluator);
+            assert!(
+                !column.is_empty(),
+                "{underlying} {stamp}: the daily series must warm and emit rows"
+            );
+            let set = set_positions(column.bits());
+            let breached: BTreeSet<u16> = set.intersection(&cleared).copied().collect();
+            eprintln!(
+                "{underlying} daily stamped {stamp} IST: time-of-day/VWAP positions set {breached:?}; {} rows",
+                column.bits().len()
+            );
+            assert!(
+                breached.is_empty(),
+                "{underlying} {stamp}: V-04 positions set on the daily rung: {breached:?}"
+            );
+            columns.push((stamp, column.bits().to_vec(), column.known().to_vec()));
+            let _ignored = std::fs::remove_dir_all(&root);
+        }
+        let (_, bits, known) = columns.first().expect("three columns").clone();
+        for (stamp, other_bits, other_known) in &columns {
+            assert!(
+                *other_bits == bits && *other_known == known,
+                "{underlying} {stamp}: a daily stamp moved the folded column"
+            );
+        }
+    }
+}
+
+/// **Two daily bars on one IST day are refused by name, never folded as two
+/// sessions or collapsed into one.** F-CEC7A0, D-4756.
+///
+/// The store admits them, a midnight bar and a 15:30 bar being strictly
+/// increasing. Mapping both to one stamp would hand the evaluator a duplicate
+/// timestamp, and keeping both would fold one session's OHLC twice.
+#[test]
+fn two_daily_bars_on_one_ist_day_are_refused_by_name() {
+    let root = daily_store("twice", "NIFTY", 0, Some(930));
+    let span = load(&root, "NIFTY");
+    let first_day = indicators::ist_day(span.bars.first().expect("bars").ts_micros);
+    let why = super::verify_series(&span, usize::MAX)
+        .expect_err("one IST day cannot hold two daily bars");
+    assert!(why.contains(&first_day.to_string()), "{why}");
+    assert!(why.contains("two daily bars"), "{why}");
+    let _ignored = std::fs::remove_dir_all(&root);
+}
+
+/// **Only the daily rung is restamped, and `limit` still bounds the fold.**
+/// F-CEC7A0, D-4756.
+///
+/// The same bars labelled with an intraday rung pass through with every
+/// stamp as stored: restamping an intraday bar to its midnight would merge a
+/// whole session into one instant. On the daily rung the fold keeps the first
+/// `limit` bars, each at its IST midnight.
+#[test]
+fn only_the_daily_rung_is_restamped_and_the_limit_bounds_the_fold() {
+    let root = daily_store("rungs", "NIFTY", 555, None);
+    let span = load(&root, "NIFTY");
+    let intraday = crate::stored::Span {
+        timeframe: "1min",
+        ..load(&root, "NIFTY")
+    };
+    let raw = super::verify_series(&intraday, usize::MAX).expect("an intraday span folds");
+    assert_eq!(raw, intraday.bars, "an intraday stamp is never moved");
+    let daily = super::verify_series(&span, 7).expect("the daily span folds");
+    assert_eq!(daily.len(), 7, "the fold takes exactly `limit` bars");
+    for (folded, stored) in daily.iter().zip(&span.bars) {
+        assert_eq!(
+            folded.ts_micros,
+            stored.ts_micros - 555 * 60_000_000,
+            "a 09:15 daily bar folds at its IST midnight"
+        );
+    }
+    let _ignored = std::fs::remove_dir_all(&root);
+}
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index b8d02ff2..073f2df1 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7064,3 +7064,4 @@ old line regex the same input and watched it pass.
 | L1FD-05 | The audit's anchored walk-forward resolves each fold's exit rungs from exactly that fold's training prefix, not the whole span (D-4753) | `cli::tests::the_audit_prices_each_walk_forward_fold_with_its_own_training_rung_count` | ✓ |
 | L1FD-06 | A CAS prior session ending past its dated close is refused as running past it, and one ending short of it as truncated. Neither message names the other cause (D-4754) | `cli::stored::tests::a_cas_prior_session_is_judged_against_the_shares_dated_close` | ✓ |
 | L1FD-07 | Validation evidence reconciles a supplied fold diagnostic against the audit's exact placement (`cli::overfitting_of`). The legacy adapter's figure is refused where the two differ (D-4755) | `cli::institutional_evidence::tests::the_supplied_diagnostic_is_the_audits_exact_placement_not_the_legacy_adapter` | ✓ |
+| L1FD-08 | V-04 holds for every daily stamp the store admits (IST midnight, the 09:15 open, the 15:30 close), for an index and a share, folded as `cli verify` folds it. The three stamps give one column, bit for bit. Two daily bars on one IST day are refused by name, and an intraday span is never restamped (D-4756, F-CEC7A0) | `cli::verify_daily_tests::every_daily_stamp_the_store_admits_folds_with_no_time_of_day_or_vwap_bit` · `cli::verify_daily_tests::two_daily_bars_on_one_ist_day_are_refused_by_name` · `cli::verify_daily_tests::only_the_daily_rung_is_restamped_and_the_limit_bounds_the_fold` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index cb7571ac..e1644324 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65282,3 +65282,50 @@ differ now gets the refusal.
 
 Test: `the_supplied_diagnostic_is_the_audits_exact_placement_not_the_legacy_adapter`
 (L1FD-07). In its one fold the two figures differ.
+
+### D-4756 — `cli verify` folds each daily bar at its IST midnight, so V-04 holds for every daily stamp the store admits — 2026-10-09
+
+**Finding.** F-CEC7A0 (gap). V-04 says time-of-day and VWAP bits are clear on
+a daily timeframe. D-1790 proved it in `indicators` for bars stamped at IST
+midnight, the daily rung's anchor (`pull/tests/anchor.rs`). The store admits
+a daily bar at any whole second: vendors stamp one at midnight, at the open or
+at the close (D-0915). `cli verify` folds the stored daily series through the
+production evaluator as stored.
+
+MEASURED before this change, by the new test: NIFTY daily bars over fifteen
+months, written through `store` and folded as `cli verify` folds them, gave a
+108-row column. With the midnight stamp no V-04 position was set. With the
+09:15 stamp, `early_morning` (position 44) was set, and the test stopped
+there. The share and the 15:30 stamp were not reached before the change, so
+nothing is claimed about them from that run. After it, all six folds (two
+instruments, three stamps) give one column per instrument, bit for bit, with
+no V-04 position set.
+
+**Decision.** `cli::verify_series` restamps each bar of a `1day` span at its
+IST day's midnight before it reaches the evaluator. That is the stamp
+`pull::fold` writes for the daily rung. Two daily bars on one IST day are
+refused by name ("two daily bars") rather than folded as two sessions or
+merged into one. The prices, volume and IST day of every bar are unchanged.
+
+This is read-side only. **No stored byte changes**, no store format version
+moves, and no ingest path changes. `cli verify` is the only production fold of
+a daily series: `swept_rung` refuses `1day`, and the daily reference context
+reads OHLC values, never stamps.
+
+**Rejected.**
+
+- Refusing a non-midnight daily bar at the store's write boundary. That
+  would leave every daily file already written at 09:15 or 15:30 refused on
+  read, or require rewriting it. Both break append-only history (§3 rule 8).
+- Refusing it in `cli verify`. That would make verify unusable on a feed that
+  stamps at the open, and the bar's IST day is the only fact a daily stamp
+  carries.
+
+Tests (L1FD-08):
+
+- `every_daily_stamp_the_store_admits_folds_with_no_time_of_day_or_vwap_bit`.
+  It also asserts that the three stamps fold to one column, bit for bit,
+  known mask included.
+- `two_daily_bars_on_one_ist_day_are_refused_by_name`.
+- `only_the_daily_rung_is_restamped_and_the_limit_bounds_the_fold`: an
+  intraday span is never restamped.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index b8e6e10a..a6b9e6b7 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15980,3 +15980,13 @@ after 2026-08-03 also pays one `CashCloses::session_close_minute`: two
 civil-day conversion. An index receipt, built with `None`, pays nothing more.
 **UNVERIFIED as a measured bound.** No bench times either receipt, so this is
 read off the source.
+
+### `cli verify` maps each daily bar to its IST midnight (D-4756)
+
+`verify_series` was a copy of the first `limit` bars. On the `1day` rung it now
+also computes each bar's IST day, compares it with the previous bar's, and
+writes the midnight stamp. That is two Euclidean divisions and one comparison
+per bar, so it stays O(limit) time with no allocation beyond the copy it
+already made. It runs twice per `cli verify`, at limits 600 and 900, and never
+on a sweep.
+**UNVERIFIED as a measured bound**, read off the source.
diff --git a/docs/11-findings.md b/docs/11-findings.md
index 80c3f349..d9799ce8 100644
--- a/docs/11-findings.md
+++ b/docs/11-findings.md
@@ -1072,3 +1072,10 @@ stays IN PROGRESS naming its branch commit until the squash merge to `main`.
 - **`F-8D5073`** (`gap`) — The vocabulary documents stated counts and kinds the table does not hold: void Near rows marked untoleranced, sixteen void names for thirteen, 70 free positions for 14, and a group table stopping at 273. Where: `docs/03-vocabulary.md` (rows 235–271, crossings section); `docs/04-invariants.md` CX-04, CX-05; `crates/vocab/src/table.rs:14-29`; `crates/vocab/src/lib.rs`. Disposition: IN PROGRESS — fixed on `attack/permutations` 3c5c237d (D-3406, XPERM-06); lands with the squash merge to `main`.
 - **`F-0486DA`** (`wrong`) — worst_reward_risk_bp scored a single observation i64::MAX, above payoff_bp, so one lucky move topped the asymmetry ranking. Where: `crates/runner/src/outcome.rs` (`Edge::worst_reward_risk_bp`); `crates/runner/src/rank.rs` (`ByAsymmetry`). Disposition: IN PROGRESS — fixed on `attack/permutations` 24c7e3a (D-3407, XPERM-07); lands with the squash merge to `main`.
 - **`F-1D5275`** (`wrong`) — trades_needed_for documented measured values its ceiling-rounded record does not return, and a monotone threshold it does not have. Where: `crates/runner/src/grid.rs` (`trades_needed_for` doc, `Cell::at_rate`). Disposition: IN PROGRESS — fixed on `attack/permutations` bc1e1f45 (D-3408, XPERM-08); lands with the squash merge to `main`.
+
+### Lane 1-b fixer D — 2026-10-09
+
+Narrative only, as above. No row is added or removed. The `F-CEC7A0` row above
+stays OPEN until the squash merge reaches `main`.
+
+- **`F-CEC7A0`** (`gap`) — V-04 was proved only for daily bars stamped at IST midnight, while the store admits any whole-second stamp and `cli verify` folded the stored stamp as it was: a 09:15 daily bar set `early_morning` (position 44). Where: `crates/cli/src/lib.rs` (`verify_series`, `measured_series_checks`); `crates/cli/src/verify_daily_tests.rs`; `docs/04-invariants.md` V-04, L1FD-08. Disposition: IN PROGRESS — fixed on `final/all-fixes` (D-4756); lands with the squash merge to `main`.
-- 
2.43.0


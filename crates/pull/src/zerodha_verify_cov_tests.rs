#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): what the verifier reads a vendor stamp as
//! in each encoding, and what a day the exchange did not trade, or no
//! calendar of record attests, permits a series. Every stamp is invented.
//!
//! BASE-COV-PULL-PS3: the verifier's in-memory edges, each driven and its
//! answer asserted: the gap ledger's unreadable lines, the oracle's drops, a
//! session of unrecorded length and a day no calendar attests, the 1day and
//! 1min records field by field, the refold's open interest and its stray
//! bucket, a receipt with no body, the cold workers' fold, the merge's
//! entries of a day, a refused run's progress slots and the words of a
//! refusal. Every instrument is `ZZ`-named and every price invented (near
//! 11,111.00, or a few paisa), which no real print names.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail; and one test per edge, each \
              holding every arm of its row."
)]

use super::tests::{
    answer_of, bar_at, day, full_day, index, minute_record, reason_word, regular, row_of, scratch,
    share, spooled, utc_s, whole_day, write_ledger,
};
use super::*;

/// One stamp, read in each of the five encodings: milliseconds as they are,
/// seconds times a thousand (an offset already applied), and an IST text
/// stamp read as UTC moved back by 05:30 (extremes: the epoch and a stamp
/// before it).
#[test]
fn a_vendor_stamp_is_utc_milliseconds_in_each_encoding() {
    let ist = i128::from(IST_OFFSET_SECS) * 1_000;
    for stamp in [0_i64, -1, 1_700_000_000] {
        let seconds = i128::from(stamp) * 1_000;
        assert_eq!(
            [
                utc_millis(stamp, TimestampEncoding::EpochMillisUtc),
                utc_millis(stamp, TimestampEncoding::EpochSecondsUtc),
                utc_millis(stamp, TimestampEncoding::IsoDateTimeOffset),
                utc_millis(stamp, TimestampEncoding::IstDateTimeText),
                utc_millis(stamp, TimestampEncoding::IsoDateTimeText),
            ],
            [
                i128::from(stamp),
                seconds,
                seconds,
                seconds - ist,
                seconds - ist
            ],
            "{stamp}"
        );
    }
}

/// A Sunday permits nothing (rows dropped), and a day before the calendar
/// of record and one after it permit nothing either, refused as unattested,
/// on each segment.
#[test]
fn a_day_not_traded_or_not_attested_permits_no_minute() {
    let number =
        |year: u16, month: u8, day: u8| Day::new(year, month, day).unwrap().days_from_epoch();
    for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
        assert_eq!(
            permit(segment, number(2024, 3, 3)),
            Permit::Closed,
            "{segment:?}"
        );
        assert_eq!(
            permit(segment, number(2010, 1, 4)),
            Permit::Unattested,
            "{segment:?}"
        );
        assert_eq!(
            permit(segment, number(2027, 6, 1)),
            Permit::Unattested,
            "{segment:?}"
        );
    }
}

// ------------------------------------------------------------- the ledger --

/// The gap ledger names, by line number, every line it cannot read, and
/// reads every line around them (extremes: a line that is not UTF-8, a `gap`
/// and a `derive` line whose key is empty, a one-digit day, a one-digit
/// month on a `gap` and on a `derive` line); an empty line between two lines
/// is no line of either kind and never a fault. A ledger whose name holds a
/// directory is refused with the host's words, never read as an absent
/// ledger, and a kept ledger keeps what it had read.
#[test]
fn the_gap_ledger_names_every_line_it_cannot_read() {
    let mut text: Vec<u8> = Vec::new();
    for line in [
        &b"gap NSE-INDEX-ZZIDX 2024-04-23 v=1 why=vendor_empty run=1"[..],
        b"",
        b"gap NSE-INDEX-ZZIDX 2024-04-2\xff v=1 why=vendor_empty run=1",
        b"gap  2024-04-24 v=1 why=vendor_empty run=1",
        b"derive  5min 2024-04 v=1 run=1",
        b"gap NSE-INDEX-ZZIDX 2024-04-5 v=1 why=vendor_empty run=1",
        b"gap NSE-INDEX-ZZIDX 2024-4-26 v=1 why=vendor_empty run=1",
        b"derive NSE-INDEX-ZZIDX 5min 2024-4 v=1 run=1",
        b"derive NSE-INDEX-ZZIDX 5min 2024-05 v=1 run=1",
    ] {
        text.extend_from_slice(line);
        text.push(b'\n');
    }
    let ledger = GapLedger::parse(&text);
    assert_eq!(ledger.lines, 9);
    assert_eq!(
        ledger.faults,
        [3, 4, 5, 6, 7, 8],
        "an empty line is no fault"
    );
    let key = "NSE-INDEX-ZZIDX";
    assert_eq!(
        ledger
            .gap(key, regular())
            .map(|line| words::reason_word(line.why)),
        Some(Some("vendor_empty"))
    );
    for unread in [day(2024, 4, 5), day(2024, 4, 24), day(2024, 4, 26)] {
        assert_eq!(ledger.gap(key, unread), None, "{unread}");
        assert_eq!(ledger.gap("", unread), None, "{unread}");
    }
    let (april, may) = (
        YearMonth::new(2024, 4).unwrap(),
        YearMonth::new(2024, 5).unwrap(),
    );
    assert_eq!(
        ledger.derive_version(key, Timeframe::MINUTE_5, may),
        Some(1)
    );
    assert_eq!(ledger.derive_version(key, Timeframe::MINUTE_5, april), None);
    assert!(ledger.covers(key, april));
    assert!(ledger.covers(key, may));
    assert!(!ledger.covers("", april), "an empty key covers nothing");

    // A DIRECTORY AT THE LEDGER'S NAME: the host's words, never "no ledger".
    let root = scratch("ledger-dir");
    write_ledger(
        &root,
        "gap NSE-INDEX-ZZIDX 2024-04-23 v=1 why=vendor_empty run=1\n",
    );
    let mut kept = GapLedger::read(&root).unwrap();
    let path = root.join("audit").join("zerodha.gaps");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let refused = kept.refresh(&root).unwrap_err();
    assert!(
        refused.starts_with(&format!("{}: ", path.display())),
        "{refused}"
    );
    assert_eq!(kept.lines, 1, "a refused refresh keeps what was read");
    assert!(kept.gap(key, regular()).is_some());
    assert_eq!(GapLedger::read(&root).unwrap_err(), refused);
}

// ------------------------------------------------------ calendar's edges --

/// A session of unrecorded length (the Muhurat of 8 November 2026, which
/// NSE/CMTR/71775 names with its timings "to be notified"): the day permits
/// no minute on any segment, so the oracle withholds every row of it rather
/// than landing or refusing one; the 1min record is `withheld_by_rule
/// (withheld_unmeasured_session)` whether its answer was kept (and withheld)
/// or none was; and a derived rung is withheld the same way, never refolded
/// against windows nobody recorded.
#[test]
fn a_session_of_unrecorded_length_owes_and_permits_no_minute() {
    let muhurat = day(2026, 11, 8);
    let number = muhurat.days_from_epoch();
    assert_eq!(
        calendar::exchange_kind_of(i64::from(number)),
        DayKind::OpenLengthUnmeasured
    );
    for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
        assert_eq!(permit(segment, number), Permit::Unsized, "{segment:?}");
    }
    let evening: Vec<Bar> = (1_080..1_140)
        .map(|minute| bar_at(muhurat, minute, 1_111_100))
        .collect();
    let rows: Vec<RawRow> = evening.iter().map(row_of).collect();
    let made = oracle(
        &rows,
        TimestampEncoding::IsoDateTimeOffset,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(muhurat),
    );
    assert_eq!(made.judged, 0, "withheld, never refused");
    let answer = &made.days[&number];
    assert!(answer.withheld);
    assert!(answer.bars.is_empty(), "never landed");
    assert!(answer.refused.is_empty());

    // The answer kept (and withheld), the store holding none of the evening.
    let kept = DayAnswer {
        receipt: 4,
        asked: whole_day(muhurat),
        kept: true,
        ..answer.clone()
    };
    let answers = [kept];
    let five = Timeframe::MINUTE_5;
    let inputs = DayInputs {
        answers: &answers,
        derived: vec![(five, Some(&[][..]), Some(RULES_D3321))],
        covered: true,
        ..DayInputs::default()
    };
    let records = verify_day(&index(), muhurat, &inputs);
    assert_eq!(records.len(), 2);
    let minute = &records[0].record;
    assert_eq!(
        (
            records[0].timeframe,
            minute.facts.code,
            reason_word(minute),
            minute.receipt
        ),
        (
            Timeframe::MINUTE_1,
            Code::WithheldByRule,
            "withheld_unmeasured_session",
            4
        )
    );
    let derived = &records[1].record;
    assert_eq!(
        (
            records[1].timeframe,
            derived.facts.code,
            reason_word(derived)
        ),
        (five, Code::WithheldByRule, "withheld_unmeasured_session")
    );

    // No answer kept and the evening stored: still withheld, never
    // `no_response_kept`, every stored minute counted.
    let inputs = DayInputs {
        minutes: &evening,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), muhurat, &inputs);
    assert_eq!(
        (record.facts.code, reason_word(&record)),
        (Code::WithheldByRule, "withheld_unmeasured_session")
    );
    assert_eq!(record.facts.store_units, 60);
}

/// A day no calendar of record attests (4 January 2010, before the record,
/// and 1 June 2027, past it): the oracle refuses a minute row on it
/// `timestamp_refused`, as the landing does; with no answer kept its 1min
/// record is `not_compared`, naming the side of the record the day lies on
/// (`calendar_unverified` before, `calendar_past_record` past); an answer the
/// landing refused keeps the worse `source_refused`; and a derived rung is
/// `not_compared (calendar_unverified)`, never refolded.
#[test]
fn a_day_no_calendar_of_record_attests_is_not_compared() {
    for (on, word) in [
        (day(2010, 1, 4), "calendar_unverified"),
        (day(2027, 6, 1), "calendar_past_record"),
    ] {
        let number = on.days_from_epoch();
        let stored = full_day(on, &[]);
        let made = oracle(
            &[row_of(&stored[0])],
            TimestampEncoding::IsoDateTimeOffset,
            Timeframe::MINUTE_1,
            Segment::Index,
            whole_day(on),
        );
        assert_eq!(made.judged, 1, "{on}");
        let refused = &made.days[&number].refused;
        assert_eq!(
            refused
                .iter()
                .map(|(minute, why)| (*minute, words::reason_word(*why)))
                .collect::<Vec<_>>(),
            [(555, Some(RowCause::Timestamp.code()))],
            "{on}"
        );

        let fifteen = Timeframe::MINUTE_15;
        let inputs = DayInputs {
            minutes: &stored,
            derived: vec![(fifteen, Some(&[][..]), Some(RULES_D3321))],
            covered: true,
            ..DayInputs::default()
        };
        let records = verify_day(&index(), on, &inputs);
        assert_eq!(
            (
                records[0].record.facts.code,
                reason_word(&records[0].record)
            ),
            (Code::NotCompared, word),
            "{on}"
        );
        assert_eq!(
            (
                records[1].timeframe,
                records[1].record.facts.code,
                reason_word(&records[1].record)
            ),
            (fifteen, Code::NotCompared, "calendar_unverified"),
            "{on}"
        );

        // The kept answer's every row refused by the oracle, as the landing
        // refused them: `source_refused` is worse than `not_compared` and
        // stays.
        let answers = [answer_of(on, &stored, 1)];
        let inputs = DayInputs {
            minutes: &stored,
            answers: &answers,
            covered: true,
            ..DayInputs::default()
        };
        let record = minute_record(&index(), on, &inputs);
        assert_eq!(
            (record.facts.code, reason_word(&record)),
            (Code::SourceRefused, RowCause::Timestamp.code()),
            "{on}"
        );
        assert_eq!((record.facts.extra, record.facts.missing), (0, 0), "{on}");
    }
}

// ------------------------------------------------------------- the oracle --

/// What the landing's own filter drops, the oracle drops and never refuses
/// (extremes): a row before the asked window and one after it, a row on a
/// Sunday, a row at 08:20 on a regular day (outside the session), and a row
/// stamped two days before 1970 (no IST day number at all). A day bar lands
/// at minute 0 of its day whatever the session, since a 1day rung has no grid
/// and no session filter, yet still only inside its asked window. A special
/// session's row lands inside its recorded window only. And the landing's own
/// refused rows: one stamped before 1970 names no day and is passed over, and
/// a receipt that counts refused rows the oracle never saw sets
/// `quality_differs` on every day, by the receipt's count.
#[test]
fn the_oracle_drops_what_the_landing_filter_drops() {
    let on = regular();
    let number = on.days_from_epoch();
    let sunday = day(2024, 4, 21);
    let row = |at: Day, minute: u32| row_of(&bar_at(at, minute, 1_111_100));
    let minute = |rows: &[RawRow], asked: (i64, i64)| {
        oracle(
            rows,
            TimestampEncoding::IsoDateTimeOffset,
            Timeframe::MINUTE_1,
            Segment::Index,
            asked,
        )
    };

    // Outside the window asked, 10:00:00 to 11:40:00, yet inside the
    // session: dropped either side by the window alone.
    let made = minute(
        &[row(on, 599), row(on, 650), row(on, 701)],
        (utc_s(on, 600), utc_s(on, 700)),
    );
    assert_eq!(made.judged, 0);
    let answer = &made.days[&number];
    assert_eq!(answer.bars.keys().copied().collect::<Vec<_>>(), [650]);
    assert!(answer.refused.is_empty());

    // A Sunday, 08:20, and a stamp before 1970: no day is even named.
    let mut before_1970 = row(on, 600);
    before_1970.timestamp = -2 * 86_400;
    let made = minute(
        &[row(sunday, 600), row(on, 500), before_1970],
        (utc_s(sunday, 0), whole_day(on).1),
    );
    assert_eq!(made.judged, 0);
    assert!(made.days.is_empty(), "{:?}", made.days.keys());

    // A day bar of a Sunday lands at minute 0, inside its window only.
    let mut daily = bar_at(sunday, 0, 1_111_100);
    daily.ts_micros = utc_s(sunday, 0) * 1_000_000;
    let at_1day = |asked: (i64, i64)| {
        oracle(
            &[row_of(&daily)],
            TimestampEncoding::IsoDateTimeOffset,
            Timeframe::DAY_1,
            Segment::Index,
            asked,
        )
    };
    let made = at_1day(whole_day(sunday));
    assert_eq!(
        made.days[&sunday.days_from_epoch()].bars,
        HashMap::from([(0, daily)])
    );
    assert!(at_1day(whole_day(on)).days.is_empty());

    // The Muhurat of 1 November 2024: its recorded window lands, 10:00 does
    // not.
    let muhurat = day(2024, 11, 1);
    let Owes::Windows((windows, count)) = repair_plan::owed("INDEX", muhurat.days_from_epoch())
    else {
        panic!("the Muhurat of record owes windows");
    };
    assert!(count >= 1);
    let (from, to) = windows[0];
    assert!(
        !(from..to).contains(&600),
        "10:00 is outside the evening window"
    );
    let made = minute(
        &[row(muhurat, from), row(muhurat, to - 1), row(muhurat, 600)],
        whole_day(muhurat),
    );
    let mut landed: Vec<u32> = made.days[&muhurat.days_from_epoch()]
        .bars
        .keys()
        .copied()
        .collect();
    landed.sort_unstable();
    assert_eq!(landed, [from, to - 1]);

    // The landing's refused rows: one before 1970 names no day.
    let mut made = minute(&[row(on, 600)], whole_day(on));
    made.take_landing_refusals(
        &[response_keep::RefusedRow {
            local_second: -1,
            cause: RowCause::Price,
        }],
        0,
    );
    assert_eq!(made.days.len(), 1);
    assert_eq!(made.days[&number].quality, None);
    assert!(made.days[&number].refused.is_empty());
    // As many judged as the oracle (none), and three counted by the receipt.
    made.take_landing_refusals(&[], 3);
    assert_eq!(made.days[&number].quality, Some((3, 0)));
}

// ------------------------------------------------------- one day's check --

/// A key longer than a verdict record holds (`store::verdict::KEY_LEN`, 64
/// bytes) gets no record at all, never one under a cut key that could name
/// another series; a key of exactly 64 bytes gets its records (extremes:
/// the longest key and one byte past it).
#[test]
fn a_key_longer_than_a_verdict_record_holds_gets_no_record() {
    let on = regular();
    let stored = full_day(on, &[]);
    let inputs = DayInputs {
        minutes: &stored,
        covered: true,
        ..DayInputs::default()
    };
    let named = |symbol: String| Series {
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol,
        contract: None,
    };
    let longest = named(format!(
        "ZZ{}",
        "Q".repeat(verdict::KEY_LEN - "NSE-INDEX-ZZ".len())
    ));
    assert_eq!(longest.key_text().len(), verdict::KEY_LEN);
    let records = verify_day(&longest, on, &inputs);
    assert_eq!(records.len(), 1, "the 1min rung: nothing else is held");
    assert_eq!(records[0].record.key.text(), longest.key_text());
    let past = named(format!("{}Q", longest.symbol));
    assert!(Key::instrument(&past.key_text()).is_err());
    assert!(verify_day(&past, on, &inputs).is_empty());
}

/// The 1day record of `inputs` for the index fixture, if one is made.
fn day_record(on: Day, inputs: &DayInputs<'_>) -> Option<SpoolRecord> {
    verify_day(&index(), on, inputs)
        .into_iter()
        .find(|made| made.timeframe == Timeframe::DAY_1)
        .map(|made| made.record)
}

/// The 1day record against its kept answer, field by field (extremes: each
/// pairing of a served bar and a stored one, neither included): equal is
/// `match`; a price, a volume or an open interest apart is `value_mismatch`
/// with that one difference counted; served and not stored is
/// `missing_in_store (unexplained)`; stored and not served is
/// `extra_in_store (unexplained)`; neither is the vendor's hole. With no kept
/// answer, one whose body was not kept says so, and minutes with no day bar
/// are the store's missing bar. A day bar whose minutes do not increase is
/// not `daycheck`ed (the fold refuses them), never flagged. A record is made
/// for a day bar, or for a day answer, even where nothing says the 1day month
/// is held. On another security's day (CMP-18) a bar that matches is
/// `store_without_source (before_first_day_bar)` and one that differs keeps
/// `value_mismatch` (a combination: foreign and answered at once).
#[test]
fn the_1day_record_compares_its_kept_answer_field_by_field() {
    let on = regular();
    let minutes = full_day(on, &[]);
    let mut held = bar_at(on, 0, 1_111_100);
    held.ts_micros = utc_s(on, 0) * 1_000_000;
    let served = |bar: Option<Bar>| DayAnswer {
        receipt: 7,
        asked: whole_day(on),
        kept: true,
        bars: bar.map(|b| HashMap::from([(0, b)])).unwrap_or_default(),
        ..DayAnswer::default()
    };
    let facts_of = |record: &SpoolRecord| {
        (
            record.facts.code,
            reason_word(record),
            record.facts.value_diff,
            record.facts.volume_diff,
            record.facts.oi_diff,
            record.facts.missing,
            record.facts.extra,
        )
    };

    // Served and stored, equal or one field apart.
    let mut price = held;
    price.close += 1;
    let mut volume = held;
    volume.volume += 1;
    let mut interest = held;
    interest.open_interest = 9;
    let answers = [served(Some(held))];
    for (stored, want) in [
        (held, (Code::Match, "", 0, 0, 0, 0, 0)),
        (price, (Code::ValueMismatch, "", 1, 0, 0, 0, 0)),
        (volume, (Code::ValueMismatch, "", 0, 1, 0, 0, 0)),
        (interest, (Code::ValueMismatch, "", 0, 0, 1, 0, 0)),
    ] {
        let inputs = DayInputs {
            day_bar: Some(stored),
            day_month_held: true,
            day_answers: &answers,
            covered: true,
            ..DayInputs::default()
        };
        let record = day_record(on, &inputs).unwrap();
        assert_eq!(facts_of(&record), want, "{stored:?}");
        assert_eq!(
            (
                record.facts.source_units,
                record.facts.store_units,
                record.receipt
            ),
            (1, 1, 7)
        );
    }

    // Served and not stored; stored and not served; neither.
    let empty = [served(None)];
    for (day_bar, answers, want) in [
        (
            None,
            &answers,
            (Code::MissingInStore, "unexplained", 0, 0, 0, 1, 0),
        ),
        (
            Some(held),
            &empty,
            (Code::ExtraInStore, "unexplained", 0, 0, 0, 0, 1),
        ),
        (
            None,
            &empty,
            (Code::VendorHole, "vendor_empty", 0, 0, 0, 0, 0),
        ),
    ] {
        let inputs = DayInputs {
            day_bar,
            day_month_held: true,
            day_answers: answers,
            covered: true,
            ..DayInputs::default()
        };
        assert_eq!(facts_of(&day_record(on, &inputs).unwrap()), want);
    }

    // No kept answer: a body not kept says so; minutes and no day bar are
    // the store's missing bar.
    let unkept = [DayAnswer {
        kept: false,
        not_kept: reason_of("storage_full"),
        ..served(Some(held))
    }];
    let inputs = DayInputs {
        day_bar: Some(held),
        day_month_held: true,
        day_answers: &unkept,
        covered: true,
        ..DayInputs::default()
    };
    let record = day_record(on, &inputs).unwrap();
    assert_eq!(record.facts.code, Code::NoResponseKept);
    assert_ne!(record.facts.flags & flags::BODY_NOT_KEPT, 0);
    let inputs = DayInputs {
        minutes: &minutes,
        day_month_held: true,
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        facts_of(&day_record(on, &inputs).unwrap()),
        (Code::MissingInStore, "unexplained", 0, 0, 0, 1, 0)
    );

    // `daycheck` over minutes in order flags the one-minute day bar; over
    // minutes that do not increase it cannot fold them and flags nothing.
    let daychecked = |minutes: &[Bar]| {
        let inputs = DayInputs {
            minutes,
            day_bar: Some(held),
            day_month_held: true,
            covered: true,
            ..DayInputs::default()
        };
        let record = day_record(on, &inputs).unwrap();
        (
            record.facts.flags & flags::DAYCHECK_DIFFERS != 0,
            record
                .details
                .iter()
                .any(|d| d.kind == words::DETAIL_DAYCHECK),
        )
    };
    assert_eq!(daychecked(&minutes), (true, true));
    let mut swapped = minutes.clone();
    swapped.swap(3, 4);
    assert_eq!(daychecked(&swapped), (false, false));

    // A day bar, or a day answer, with nothing saying the month is held.
    let inputs = DayInputs {
        day_bar: Some(held),
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        day_record(on, &inputs).map(|r| r.facts.code),
        Some(Code::NoResponseKept)
    );
    let inputs = DayInputs {
        day_answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        day_record(on, &inputs).map(|r| r.facts.code),
        Some(Code::MissingInStore)
    );

    // Another security's day, its minutes held and its bar answered.
    for (stored, want) in [
        (held, Code::StoreWithoutSource),
        (price, Code::ValueMismatch),
    ] {
        let inputs = DayInputs {
            minutes: &minutes,
            day_bar: Some(stored),
            day_month_held: true,
            day_answers: &answers,
            covered: true,
            first_day_bar: Some(day(2024, 5, 2)),
            ..DayInputs::default()
        };
        let record = day_record(on, &inputs).unwrap();
        assert_eq!(record.facts.code, want);
        assert_eq!(
            reason_word(&record),
            if want == Code::StoreWithoutSource {
                "before_first_day_bar"
            } else {
                ""
            }
        );
    }
}

/// The 1min record against a kept answer, each difference by its own name:
/// a volume apart is `value_mismatch` with `volume_diff`, never a price
/// difference; a stored minute before the window the answer was asked for is
/// neither extra nor vouched for (`no_response_kept`); a stored minute whose
/// rows the answer's oracle refused is never `extra`; the landing's refused
/// count against the oracle's is `quality_differs` sub-kind 1; a re-pull of
/// another length is the vendor's revision and an identical re-pull is not.
/// An owed minute the store lacks is spoken for by the answer inside its
/// asked window and by the ledger outside it (asked from 10:00, 09:30 is the
/// ledger's); a hole in a window that ends at the answer's own last row is
/// `vendor_empty`, never `answer_short`; a minute the answer had cannot be
/// `vendor_empty` by the ledger. And a day with no kept answer whose ledger
/// line came from another store but does not say `repaired` keeps the
/// unkept answer's own reason.
#[test]
fn the_minute_record_names_each_difference_a_kept_answer_shows() {
    let on = regular();
    let number = on.days_from_epoch();
    let stored = full_day(on, &[]);
    let record_of = |answers: &[DayAnswer]| {
        let inputs = DayInputs {
            minutes: &stored,
            answers,
            covered: true,
            ..DayInputs::default()
        };
        minute_record(&index(), on, &inputs)
    };

    // A volume apart.
    let mut louder = stored.clone();
    louder[30].volume += 1;
    let record = record_of(&[answer_of(on, &louder, 1)]);
    assert_eq!(
        (
            record.facts.code,
            record.facts.volume_diff,
            record.facts.value_diff,
            record.facts.first_diff
        ),
        (Code::ValueMismatch, 1, 0, 585)
    );

    // Asked from 10:00: the morning's stored minutes are not extra, and no
    // kept answer vouches for them.
    let mut late = answer_of(on, &stored[45..], 2);
    late.asked.0 = utc_s(on, 600);
    let record = record_of(&[late]);
    assert_eq!(
        (record.facts.code, record.facts.extra, record.facts.missing),
        (Code::NoResponseKept, 0, 0)
    );

    // The answer's two candles at 10:00 conflict, so both are refused: the
    // store's 10:00 minute is never extra.
    let mut rows: Vec<RawRow> = stored.iter().map(row_of).collect();
    let mut twin = rows[45];
    twin.close += 1;
    rows.push(twin);
    let made = oracle(
        &rows,
        TimestampEncoding::IsoDateTimeOffset,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(on),
    );
    let refused = DayAnswer {
        receipt: 3,
        asked: whole_day(on),
        kept: true,
        ..made.days[&number].clone()
    };
    let record = record_of(&[refused]);
    assert_eq!(
        (record.facts.code, reason_word(&record), record.facts.extra),
        (Code::SourceRefused, "conflicting_candles", 0)
    );

    // The landing refused five rows by name, the oracle judges three.
    let mut counted = answer_of(on, &stored, 4);
    counted.quality = Some((5, 3));
    let record = record_of(&[counted]);
    assert_eq!(record.facts.code, Code::Match);
    assert_ne!(record.facts.flags & flags::QUALITY_DIFFERS, 0);
    let quality: Vec<&[u8]> = record
        .details
        .iter()
        .filter(|d| d.kind == words::DETAIL_QUALITY_DIFFERS)
        .map(|d| d.payload.as_slice())
        .collect();
    let mut want = 1_u16.to_le_bytes().to_vec();
    want.extend(5_u64.to_le_bytes());
    want.extend(3_u64.to_le_bytes());
    assert_eq!(quality, [want.as_slice()]);

    // A re-pull one minute shorter is the vendor's revision; an identical
    // one is nothing.
    let shorter = record_of(&[answer_of(on, &stored, 1), answer_of(on, &stored[1..], 2)]);
    assert_eq!(shorter.facts.code, Code::Match);
    assert_ne!(shorter.facts.flags & flags::VENDOR_REVISED, 0);
    assert_eq!(
        shorter.details,
        [Detail {
            kind: words::DETAIL_RECEIPTS,
            payload: vec![1, 0, 0, 0, 2, 0, 0, 0],
        }]
    );
    let same = record_of(&[answer_of(on, &stored, 1), answer_of(on, &stored, 2)]);
    assert_eq!(same.facts.code, Code::Match);
    assert_eq!(same.facts.flags & flags::VENDOR_REVISED, 0);
    assert!(
        same.details
            .iter()
            .all(|d| d.kind != words::DETAIL_RECEIPTS)
    );

    // Who speaks for an owed minute the store lacks. Asked from 10:00, the
    // answer says nothing of 09:30: the ledger does (none here, in a covered
    // month: the store's own hole).
    let lacking = full_day(on, &[570]);
    let mut from_ten = answer_of(on, &stored[45..], 5);
    from_ten.asked.0 = utc_s(on, 600);
    let answers = [from_ten];
    let inputs = DayInputs {
        minutes: &lacking,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        (
            record.facts.code,
            reason_word(&record),
            record.facts.missing
        ),
        (Code::MissingInStore, "unexplained", 1)
    );
    // A hole inside a window that ends at the answer's own last row is the
    // vendor's hole, never `answer_short`: the answer did reach its end.
    let holed = full_day(on, &[700]);
    let mut to_its_end = answer_of(on, &holed, 6);
    to_its_end.asked.1 = utc_s(on, 929);
    assert_eq!(to_its_end.last_stamp, to_its_end.asked.1);
    let answers = [to_its_end];
    let inputs = DayInputs {
        minutes: &holed,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        (
            record.facts.code,
            reason_word(&record),
            record.facts.missing
        ),
        (Code::VendorHole, "vendor_empty", 1)
    );
    // The answer HAD the minute the store lacks: a `vendor_empty` line cannot
    // speak for it, so it is the store's own hole.
    let answers = [answer_of(on, &stored, 7)];
    let inputs = DayInputs {
        minutes: &holed,
        answers: &answers,
        gap: Some(GapLine {
            why: reason_of("vendor_empty"),
            origin_store: false,
        }),
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        (
            record.facts.code,
            reason_word(&record),
            record.facts.missing
        ),
        (Code::MissingInStore, "unexplained", 1)
    );

    // No kept answer; the ledger's line came from another store and does not
    // say `repaired`: the unkept answer's own reason, never
    // `landed_from_store`.
    let unkept = [DayAnswer {
        receipt: 9,
        asked: whole_day(on),
        kept: false,
        not_kept: reason_of("pack_full"),
        ..DayAnswer::default()
    }];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &unkept,
        gap: Some(GapLine {
            why: reason_of("refused_day"),
            origin_store: true,
        }),
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        (record.facts.code, reason_word(&record)),
        (Code::NoResponseKept, "pack_full")
    );
    assert_ne!(record.facts.flags & flags::BODY_NOT_KEPT, 0);
}

// ------------------------------------------------------------- the refold --

/// The refold's edges: a day the exchange did not trade folds its minutes
/// into buckets of which none stands (every minute is stray, none owed); a
/// bucket's open interest is the last one its minutes carried, an absent one
/// never overwriting it; a stored bucket whose open interest differs is
/// `value_mismatch` with `oi_diff` alone; and a stored bar at an INDEX bucket
/// the rule withholds (one minute missing) is `extra_in_store
/// (unexplained)`, the one difference of the day.
#[test]
fn the_refold_carries_open_interest_and_names_a_bucket_it_cannot_stand() {
    let on = regular();
    let number = on.days_from_epoch();
    let five = Timeframe::MINUTE_5;

    let sunday = day(2024, 4, 21);
    let stray: Vec<Bar> = (600..610)
        .map(|minute| bar_at(sunday, minute, 1_111_100))
        .collect();
    let (buckets, unjudged) = refold(&index(), sunday.days_from_epoch(), &stray, five);
    assert_eq!(unjudged, None);
    assert_eq!(
        buckets
            .iter()
            .map(|b| (b.start, b.held, b.owed, b.complete, b.stray))
            .collect::<Vec<_>>(),
        [(600, 5, 0, false, true), (605, 5, 0, false, true)]
    );

    let mut minutes = full_day(on, &[]);
    minutes[0].open_interest = 10;
    minutes[2].open_interest = 12;
    let (buckets, _) = refold(&index(), number, &minutes, five);
    assert_eq!(
        (buckets[0].bar.open_interest, buckets[1].bar.open_interest),
        (12, OI_NULL),
        "the last carried, never an absent one after it"
    );
    let mut wrong: Vec<Bar> = buckets.iter().map(|b| b.bar).collect();
    wrong[0].open_interest = 11;
    let inputs = DayInputs {
        minutes: &minutes,
        derived: vec![(five, Some(wrong.as_slice()), Some(RULES_D3321))],
        covered: true,
        ..DayInputs::default()
    };
    let record = &verify_day(&index(), on, &inputs)[1].record;
    assert_eq!(
        (
            record.facts.code,
            record.facts.oi_diff,
            record.facts.value_diff,
            record.facts.volume_diff,
            record.facts.first_diff
        ),
        (Code::ValueMismatch, 1, 0, 0, 555)
    );

    let short = full_day(on, &[600]);
    let (buckets, _) = refold(&index(), number, &short, five);
    let every: Vec<Bar> = buckets.iter().map(|b| b.bar).collect();
    assert_eq!(every.len(), 75, "the withheld bucket folded and stored too");
    let inputs = DayInputs {
        minutes: &short,
        derived: vec![(five, Some(every.as_slice()), Some(RULES_D3321))],
        covered: true,
        ..DayInputs::default()
    };
    let record = &verify_day(&index(), on, &inputs)[1].record;
    assert_eq!(
        (
            record.facts.code,
            reason_word(record),
            record.facts.extra,
            record.facts.missing,
            record.facts.first_diff
        ),
        (Code::ExtraInStore, "unexplained", 1, 0, 600)
    );
    assert_eq!(
        (record.facts.source_units, record.facts.store_units),
        (74, 75)
    );
}

// ----------------------------------------------------------- the receipts --

/// A receipt whose body is not in its pack names why, by the table's own
/// words (extremes: every `kept` a receipt can carry): not kept for a full
/// disk is `storage_full`, for any other reason `pack_full`; a refused
/// answer's head is `refused_answer`; and a receipt whose body is in a pack,
/// its own or one it points at, has no such word.
#[test]
fn a_receipt_with_no_body_in_its_pack_names_why() {
    use response_keep::{Door, Kept, Reason, Receipt};
    let base = Receipt {
        asked_from: 0,
        asked_to: 0,
        first_stamp: 0,
        last_stamp: 0,
        body_offset: 0,
        deflated_len: 0,
        raw_len: 0,
        rows_decoded: 0,
        rows_landed: 0,
        rows_refused: 0,
        run: 0,
        status: 200,
        rung: 3,
        kept: Kept::Kept,
        reason: Reason::Kept,
        rules_version: 1,
        door: Door::FromWindow,
        segment: 0,
        digest: [0; 32],
        key_hash: 0,
        body_of: 0,
        refusals_from: 0,
        refusals: 0,
    };
    for (kept, reason, word) in [
        (Kept::NotKept, Reason::StorageFull, Some("storage_full")),
        (Kept::NotKept, Reason::PackFull, Some("pack_full")),
        (Kept::NotKept, Reason::Kept, Some("pack_full")),
        (
            Kept::RefusedHead,
            Reason::RefusedAnswer,
            Some("refused_answer"),
        ),
        (Kept::Kept, Reason::Kept, None),
        (Kept::Duplicate, Reason::DuplicateBody, None),
        (Kept::InFirstMonth, Reason::LaterMonth, None),
    ] {
        let said = not_kept_word(&Receipt {
            kept,
            reason,
            ..base
        });
        assert_eq!(said, word, "{kept:?} {reason:?}");
        assert_ne!(
            said.map_or(1, reason_of),
            0,
            "{kept:?}: a word of the table"
        );
    }
}

/// A contract series (an expired future; here a future, and an index series
/// carrying a contract) has no instrument a kept answer is filed under: its
/// answers are not looked up (the module's limits) and its peer is not
/// GDFL's, so neither touches the store; an index and a share have theirs.
#[test]
fn a_contract_series_has_no_kept_answers_and_no_peer() {
    let root = scratch("contract");
    let month = regular().year_month().unwrap();
    let future = Series {
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: "ZZIDX".to_owned(),
        contract: Contract::parse("2024-04-25-FUT"),
    };
    let carried = Series {
        segment: Segment::Index,
        ..future.clone()
    };
    assert!(future.instrument().is_none());
    assert!(carried.instrument().is_none());
    assert_eq!(index().instrument().map(|key| key.kind), Some(Kind::Index));
    assert_eq!(share().instrument().map(|key| key.kind), Some(Kind::Equity));
    let mut next = 0;
    let (answers, faults) = month_answers(&root, &future, Timeframe::MINUTE_1, month, &mut next);
    assert!(answers.is_empty());
    assert!(faults.is_empty());
    assert_eq!(next, 0, "no receipt numbered");
    let (mut faults, mut busy) = (Vec::new(), Vec::new());
    assert!(peer_month(&root, &future, month, (&mut faults, &mut busy)).is_none());
    assert!(faults.is_empty());
    assert!(busy.is_empty());
    assert!(
        std::fs::read_dir(&root).unwrap().next().is_none(),
        "nothing was made under the root"
    );
}

// ----------------------------------------------------- the cold workers --

/// The cold workers' answers fold in the series' listing order, or refuse by
/// name: no worker started, an answer for a series past the listing, and a
/// worker that panicked are each a refusal in words, and a series' own
/// refusal is returned as it is. A report whose examples are full takes no
/// more, whatever the folds that follow.
#[test]
fn the_cold_answers_fold_in_listing_order_or_refuse_by_name() {
    assert_eq!(
        fold_finished(Vec::new(), 2),
        Err("no cold verification worker could start".to_owned())
    );
    let one = |days: u64, words: &str| ColdReport {
        series: 1,
        days,
        examples: vec![("1min".to_owned(), Code::VendorHole, words.to_owned())],
        ..ColdReport::default()
    };
    assert_eq!(
        fold_finished(vec![vec![(2, Ok(one(1, "past")))]], 2),
        Err("a cold worker answered for no series".to_owned())
    );
    assert_eq!(
        fold_finished(
            vec![vec![(usize::MAX, Err("a cold worker panicked".to_owned()))]],
            2
        ),
        Err("a cold worker panicked".to_owned())
    );
    assert_eq!(
        fold_finished(
            vec![
                vec![(0, Ok(one(1, "first")))],
                vec![(1, Err("series 1 refused".to_owned()))]
            ],
            2
        ),
        Err("series 1 refused".to_owned())
    );
    let folded = fold_finished(
        vec![
            vec![(1, Ok(one(2, "second")))],
            vec![(0, Ok(one(1, "first")))],
        ],
        2,
    )
    .unwrap();
    assert_eq!((folded.series, folded.days, folded.workers), (2, 3, 2));
    assert_eq!(
        folded.examples,
        [("1min".to_owned(), Code::VendorHole, "first".to_owned())],
        "the first in the listing, whatever worker answered first"
    );

    let mut full = ColdReport::default();
    for at in 0..MAX_EXAMPLES {
        full.examples
            .push((format!("rung{at}"), Code::VendorHole, String::new()));
    }
    let mut whole = ColdReport::default();
    whole.fold(full);
    whole.fold(one(1, "rare"));
    assert_eq!(whole.examples.len(), MAX_EXAMPLES);
    assert!(whole.examples.iter().all(|(_, _, words)| words != "rare"));
}

// -------------------------------------------------------------- the merge --

/// One entry per key of a day bucket, and the day's worst code over the
/// first receipt of each key: a later receipt of a key that differs from its
/// first in any one count (value, volume, open interest, missing, each
/// alone) flags the vendor's revision once, naming the first two receipts
/// that differ; an identical later receipt changes nothing.
#[test]
fn a_key_whose_later_receipts_differ_is_flagged_once() {
    let on = regular();
    let record = |key: &str, code: Code, receipt: u32, set: fn(&mut Facts)| {
        let mut made = spooled(on, key, code, receipt);
        set(&mut made.facts);
        made
    };
    let owned = [
        record("NSE-INDEX-ZZREV", Code::Match, 1, |_| {}),
        record("NSE-INDEX-ZZREV", Code::Match, 2, |_| {}),
        record("NSE-INDEX-ZZREV", Code::Match, 3, |f| f.value_diff = 1),
        record("NSE-INDEX-ZZREV", Code::Match, 4, |f| f.volume_diff = 1),
        record("NSE-INDEX-ZZREV", Code::Match, 5, |f| f.oi_diff = 1),
        record("NSE-INDEX-ZZREV", Code::Match, 6, |f| f.missing = 1),
        record("NSE-INDEX-ZZTWO", Code::VendorHole, 1, |_| {}),
    ];
    let bucket: Vec<&SpoolRecord> = owned.iter().collect();
    let (entries, worst) = entries_of(&bucket);
    assert_eq!(worst, Code::VendorHole);
    assert_eq!(
        entries.iter().map(|e| e.key.text()).collect::<Vec<_>>(),
        ["NSE-INDEX-ZZREV", "NSE-INDEX-ZZTWO"]
    );
    let revised = &entries[0];
    assert_eq!(
        revised.facts.code,
        Code::Match,
        "the first receipt's record"
    );
    assert_eq!(revised.facts.value_diff, 0);
    assert_ne!(revised.facts.flags & flags::VENDOR_REVISED, 0);
    assert_eq!(revised.details.len(), 1, "flagged once");
    assert_eq!(revised.details[0].kind, words::DETAIL_RECEIPTS);
    assert_eq!(revised.details[0].payload, [1, 0, 0, 0, 3, 0, 0, 0]);
    assert_eq!(entries[1].facts.flags & flags::VENDOR_REVISED, 0);
    assert!(entries[1].details.is_empty());
}

// --------------------------------------------------------------- progress --

/// A refused run's progress record keeps its newest refusals only, one slot
/// each up to `PROGRESS_SLOTS`, the oldest dropped first (extreme: one more
/// refusal than slots); and words past a slot's `SLOT_WHY` bytes are cut at
/// a character boundary, never inside one (here a two-byte `é` straddles the
/// cut: a combination of both edges in every call).
#[test]
fn a_refused_run_keeps_its_newest_refusals_cut_at_a_character() {
    use crate::gdfl_run::{PROGRESS_SLOTS, ProgressFile, RunState, SLOT_WHY};
    let root = scratch("progress-slots");
    let progress = ProgressFile::of_record(
        root.join(PROGRESS_MERGE),
        progress_record("verify-merge", 1, 7, 1, None),
    );
    let why = |n: usize| format!("{n:02}{}é and after", "x".repeat(SLOT_WHY - 3));
    assert!(
        !why(0).is_char_boundary(SLOT_WHY),
        "the cut falls inside the é"
    );
    for n in 0..=PROGRESS_SLOTS {
        progress_end(&progress, false, Some(&why(n)));
    }
    let record = progress.record();
    assert_eq!(record.state, RunState::Refused.code());
    assert_eq!(
        record.refusals_total,
        u64::try_from(PROGRESS_SLOTS).unwrap() + 1
    );
    let kept: Vec<&str> = record.refusals.iter().map(|r| r.why.as_str()).collect();
    let want: Vec<String> = (1..=PROGRESS_SLOTS)
        .map(|n| why(n)[..SLOT_WHY - 1].to_owned())
        .collect();
    assert_eq!(kept, want, "the oldest dropped, each cut before the é");
    assert_eq!(progress.fault(), None);
}

// ------------------------------------------------------------------ words --

/// A merge refusal and a month refusal keep the words they were given: a
/// lock the host refused is `merge_lock_refused` with the host's words, a
/// spool tree that would not list is the spool's own words, and a month's
/// refusal read as text is its `why`.
#[test]
fn a_merge_or_month_refusal_says_the_words_it_was_given() {
    assert_eq!(
        MergeError::Lock("an invented refusal".to_owned()).to_string(),
        "merge_lock_refused: an invented refusal"
    );
    let busy = || SpoolError::Busy {
        run: 9,
        holder: "an invented holder".to_owned(),
    };
    assert_eq!(MergeError::Spool(busy()).to_string(), busy().to_string());
    assert_eq!(
        String::from(MonthRefusal {
            busy: true,
            why: "month_busy: an invented hold".to_owned(),
        }),
        "month_busy: an invented hold"
    );
}

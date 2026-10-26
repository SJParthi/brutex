//! `pull::gaps::not_owed` and the classifier's fifth reason (R4-19; findings
//! G4, Z3, TC-04, COR-07). FORCEMOT, INE451A01017 and the interval
//! 2023-10-26..=2024-02-13 are the documented facts
//! (`docs/00-charter.md`, "FORCEMOT NSE interruption"); every other symbol,
//! ISIN, listing and removal day here is invented, and no bar is a vendor's.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test-only assertions"
)]

use super::*;
use crate::listing::{Source, Unknown};

/// Days since the epoch of a calendar date.
fn on(year: u16, month: u8, date: u8) -> i64 {
    i64::from(Day::new(year, month, date).unwrap().days_from_epoch())
}

/// Micros since the epoch for a day and an IST minute-of-day.
fn at(day: i64, minute: u16) -> i64 {
    (day * 86_400 + i64::from(minute) * 60 - 19_800) * 1_000_000
}

/// Every minute of a regular session, for one day.
fn full_day(day: i64) -> Vec<i64> {
    (calendar::OPEN_MINUTE..=calendar::LAST_MINUTE)
        .map(|minute| at(day, minute))
        .collect()
}

/// Whether the table calls `day` a session of record (sized or not).
fn session(day: i64) -> bool {
    matches!(
        calendar::kind_of(day),
        DayKind::Open(_) | DayKind::OpenLengthUnmeasured
    )
}

/// The FORCEMOT key every owed path builds, with no master.
fn forcemot() -> Key<'static> {
    Key::series("NSE", "CASH", "FORCEMOT", None)
}

/// A master-stated span for an invented share, listed `listed` and removed
/// `removed`.
fn master_span(listed: Result<Day, Unknown>, removed: Option<Day>) -> Span<'static> {
    static SHA: [u8; 32] = [7; 32];
    Span {
        first: listed,
        last: removed,
        source: Source::NseSecurityMaster {
            file: "NSE_CM_security_03082026.csv.gz",
            sha256: &SHA,
        },
    }
}

/// A cash key for an invented share carrying `span` and no ISIN.
fn invented(span: Span<'static>) -> Key<'static> {
    Key {
        exchange: "NSE",
        segment: "CASH",
        symbol: "INVENTEDSHR",
        isin: None,
        span: Some(span),
        coverage: Coverage::Unstated,
    }
}

fn day_of(year: u16, month: u8, date: u8) -> Day {
    Day::new(year, month, date).unwrap()
}

/// **G4 and Z3, at the classifier: a FORCEMOT session NSE withdrew is not
/// owed and loses nothing.** October 2023 to February 2024, every session
/// FORCEMOT owed held whole and every session of the withdrawal empty, as the
/// operator's store and both vendors have it. Before R4-19 the cash door
/// counted each withdrawn session's 375 minutes lost (`Key::NONE` still
/// does, below); now each reads `instrument-inactive` with both NSE sources,
/// the 76 sessions of the interval are named once, and nothing is owed or
/// lost there. The withdrawal and the readmission each fall inside a month, so
/// October and February are split: the 25th and the 14th are owed.
#[test]
fn forcemot_daily_gap_reason_is_inactive() {
    let (first, last) = (on(2023, 10, 1), on(2024, 2, 29));
    let (from, through) = (on(2023, 10, 26), on(2024, 2, 13));
    let inside = |day: i64| (from..=through).contains(&day);
    let mut stored = Vec::new();
    let mut owed_sessions = 0_u32;
    for day in first..=last {
        if calendar::kind_of(day) == DayKind::Open(calendar::Session::full()) && !inside(day) {
            stored.extend(full_day(day));
            owed_sessions += 1;
        }
    }
    let key = forcemot();
    assert_eq!(
        key.isin.map(|isin| isin.as_str().to_owned()).as_deref(),
        Some("INE451A01017"),
        "the universe's exact ISIN, the one the NSE sources name"
    );
    let ledger = classify_cash(&stored, first, last, None, &key);
    assert_eq!(ledger.invalid_timestamps, 0);
    assert!(!ledger.truncated);
    assert_eq!(ledger.lost_minutes(), 0, "{:?}", ledger.gaps);
    assert!(
        ledger
            .gaps
            .iter()
            .all(|gap| gap.reason != Reason::VendorHole),
        "no withdrawn minute is a vendor hole"
    );
    assert_eq!(
        ledger.expected,
        owed_sessions * 375,
        "only the sessions FORCEMOT owed are owed"
    );
    let interval = crate::cash_auction::independently_inactive(
        "FORCEMOT",
        "INE451A01017",
        day_of(2023, 12, 1),
    )
    .expect("the documented interval");
    assert!(interval.withdrawal_source.ends_with("/CML58560.pdf"));
    assert!(
        interval
            .readmission_source
            .starts_with("https://www.forcemotors.com/")
    );
    let inactive = Reason::NotOwed(NotOwed::InstrumentInactive(interval));
    assert_eq!(inactive.word(), "instrument-inactive");
    assert!(!inactive.is_loss());
    // BOTH CIRCULARS BY NUMBER, beside both documents, in the one wording
    // every page and terminal prints.
    let evidence = NotOwed::InstrumentInactive(interval).evidence();
    for named in [
        "2023-10-26..=2024-02-13",
        "withdrawal circular CML58560 (https://archives.nseindia.com/content/circulars/CML58560.pdf)",
        "readmission circular CML60618 (in https://www.forcemotors.com/",
    ] {
        assert!(evidence.contains(named), "{named}: {evidence}");
    }
    let sessions = (from..=through).filter(|day| session(*day)).count();
    assert_eq!(
        sessions, 76,
        "the interval's sessions of record, the Muhurat of 2023-11-12 and the Saturday \
         2024-01-20 among them"
    );
    for day in (from..=through).filter(|day| session(*day)) {
        let mine: Vec<&Gap> = ledger.gaps.iter().filter(|gap| gap.day == day).collect();
        assert!(
            mine.iter().any(|gap| gap.reason == inactive),
            "{day}: named inactive"
        );
        assert!(
            mine.iter()
                .all(|gap| gap.reason == inactive || gap.reason == Reason::OutsideWindow),
            "{day}: nothing else but the windows' outside: {mine:?}"
        );
    }
    // THE DAYS EITHER SIDE ARE OWED, AND HELD.
    for day in [on(2023, 10, 25), on(2024, 2, 14)] {
        assert!(
            ledger
                .gaps
                .iter()
                .filter(|gap| gap.day == day)
                .all(|gap| gap.reason == Reason::OutsideWindow),
            "{day} is owed and whole"
        );
    }
    let named = ledger.not_owed();
    assert_eq!(named.len(), 1, "one answer, named once: {named:?}");
    assert_eq!(named[0].why, NotOwed::InstrumentInactive(interval));
    assert_eq!(named[0].days, 76);
    assert_eq!((named[0].first_day, named[0].last_day), (from, through));
    assert_eq!(named[0].minutes, ledger.not_owed_minutes());
    assert_eq!(
        NotOwed::InstrumentInactive(interval).code(),
        "instrument_inactive"
    );
    // BEFORE R4-19, every withdrawn sized session was 375 minutes lost: the
    // key that names no share still answers that, so the change is the key's.
    let unkeyed = classify_cash(&stored, first, last, None, &Key::NONE);
    let sized_minutes: u32 = (from..=through)
        .filter_map(|day| match calendar::kind_of(day) {
            DayKind::Open(session) => Some(u32::from(session.bars())),
            _ => None,
        })
        .sum();
    assert_eq!(unkeyed.lost_minutes(), sized_minutes);
    assert_eq!(unkeyed.not_owed_minutes(), 0);
}

/// **`not_owed` answers in its stated order, and only on its evidence.**
/// Each reason at its exact boundary; every way a key can miss the
/// documented identity excuses nothing; a day that is no calendar day gets no
/// instrument fact; the reading evidence answers for any series, after the
/// instrument facts.
#[test]
fn not_owed_answers_in_order_and_only_on_its_evidence() {
    let key = forcemot();
    assert_eq!(
        not_owed(&key, on(2023, 10, 25)),
        None,
        "the last day traded"
    );
    for day in [on(2023, 10, 26), on(2023, 12, 25), on(2024, 2, 13)] {
        assert!(
            matches!(not_owed(&key, day), Some(NotOwed::InstrumentInactive(_))),
            "{day}"
        );
    }
    assert_eq!(not_owed(&key, on(2024, 2, 14)), None, "readmitted");
    // EXACTLY THE DOCUMENTED IDENTITY, OR NOTHING.
    let reliance = brutex_core::universe::nse_isin("RELIANCE");
    assert!(reliance.is_some());
    let wrong = [
        Key::series("NSE", "CASH", "FORCEMOT ", None),
        Key::series("NSE", "CASH", "forcemot", None),
        Key::series("BSE", "CASH", "FORCEMOT", None),
        Key::series("NSE", "INDEX", "FORCEMOT", None),
        Key::series("NSE", "FNO", "FORCEMOT", None),
        Key::series("NSE", "cash", "FORCEMOT", None),
        Key {
            isin: reliance,
            ..forcemot()
        },
        Key {
            exchange: "BSE",
            ..forcemot()
        },
        Key::NONE,
    ];
    for other in wrong {
        assert_eq!(
            not_owed(&other, on(2023, 12, 1)),
            None,
            "{}/{}/{}",
            other.exchange,
            other.segment,
            other.symbol
        );
    }
    // NO CALENDAR DAY, NO INSTRUMENT FACT.
    for day in [i64::MIN, -1, i64::from(u32::MAX) + 1, i64::MAX] {
        assert_eq!(not_owed(&key, day), None, "{day}");
    }
    // THE LISTING, ON A MASTER'S WORD ONLY.
    let listed = day_of(2021, 6, 15);
    let share = invented(master_span(Ok(listed), None));
    assert_eq!(
        not_owed(&share, on(2021, 6, 14)),
        Some(NotOwed::BeforeListing { listed_on: listed })
    );
    assert_eq!(not_owed(&share, on(2021, 6, 15)), None, "the listing day");
    assert_eq!(
        NotOwed::BeforeListing { listed_on: listed }.code(),
        "before_listing"
    );
    let inferred = invented(Span::inferred(listed, day_of(2026, 8, 3)));
    assert_eq!(
        not_owed(&inferred, on(2021, 6, 14)),
        None,
        "the vendor's own span accuses nothing"
    );
    for unknown in [Unknown::EpochZero, Unknown::Blank, Unknown::Conflicting] {
        let share = invented(master_span(Err(unknown), None));
        assert_eq!(
            not_owed(&share, on(2000, 1, 3)),
            None,
            "{unknown:?}: owed as today"
        );
    }
    // THE REMOVAL DAY IS OWED; THE DAY AFTER IS NOT.
    let removed = day_of(2023, 6, 16);
    let gone = invented(master_span(Ok(listed), Some(removed)));
    assert_eq!(not_owed(&gone, on(2023, 6, 16)), None);
    assert_eq!(
        not_owed(&gone, on(2023, 6, 19)),
        Some(NotOwed::AfterDelisting {
            delisted_on: removed
        })
    );
    assert_eq!(
        NotOwed::AfterDelisting {
            delisted_on: removed
        }
        .word(),
        "after-delisting"
    );
    // UNREAD (TC-04), FOR ANY SERIES, AFTER THE INSTRUMENT FACTS.
    let absent = Coverage::Stated {
        held_through: None,
        requested: Requested::NONE,
    };
    let nifty = Key::series("NSE", "INDEX", "NIFTY", None).read(absent);
    assert_eq!(not_owed(&nifty, on(2026, 10, 1)), Some(NotOwed::Unread));
    assert_eq!(not_owed(&nifty, i64::MIN), Some(NotOwed::Unread));
    assert!(matches!(
        not_owed(&key.read(absent), on(2023, 12, 1)),
        Some(NotOwed::InstrumentInactive(_))
    ));
    assert_eq!(
        not_owed(&share.read(absent), on(2021, 6, 14)),
        Some(NotOwed::BeforeListing { listed_on: listed })
    );
    assert_eq!(
        not_owed(&key.read(absent), on(2024, 2, 14)),
        Some(NotOwed::Unread)
    );
    let held = Coverage::Stated {
        held_through: Some(on(2026, 9, 30)),
        requested: Requested::NONE,
    };
    let read = Key::series("NSE", "INDEX", "NIFTY", None).read(held);
    assert_eq!(not_owed(&read, on(2026, 9, 30)), None, "read through it");
    assert_eq!(not_owed(&read, on(2026, 10, 1)), Some(NotOwed::Unread));
    let asked = [1_u64];
    let requested = Coverage::Stated {
        held_through: Some(on(2026, 9, 30)),
        requested: Requested {
            from: on(2026, 10, 1),
            bits: &asked,
        },
    };
    let ledgered = Key::series("NSE", "INDEX", "NIFTY", None).read(requested);
    assert_eq!(
        not_owed(&ledgered, on(2026, 10, 1)),
        None,
        "the ledger records the request: a hole there is the vendor's"
    );
    assert_eq!(not_owed(&ledgered, on(2026, 10, 2)), Some(NotOwed::Unread));
    assert_eq!(NotOwed::Unread.code(), NotOwed::Unread.word());
    assert!(!Coverage::Unstated.unread(on(2026, 10, 1)));
    assert_eq!(Coverage::default(), Coverage::Unstated);
}

/// **A requested day is one bit, read right at every word boundary.**
#[test]
fn requested_days_are_one_bit_each_at_every_word_boundary() {
    let bits = [1_u64 | (1_u64 << 63), 1_u64];
    let days = Requested {
        from: 100,
        bits: &bits,
    };
    assert!(days.holds(100), "bit 0");
    assert!(!days.holds(101));
    assert!(days.holds(163), "bit 63");
    assert!(days.holds(164), "bit 0 of the second word");
    assert!(!days.holds(165));
    assert!(!days.holds(99), "before the first day");
    assert!(!days.holds(228), "past the last word");
    assert!(!days.holds(i64::MIN), "an offset that overflows");
    assert!(!days.holds(i64::MAX));
    assert!(!Requested::NONE.holds(0));
}

/// **COR-07 at the classifier: a listing day mid-month is owed from that
/// day.** The sessions before it read `before-listing`, owe nothing and lose
/// nothing; from it on the share owes every session, so a hole after the
/// listing is still a vendor hole.
#[test]
fn a_listing_day_mid_month_is_owed_from_that_day() {
    let listed = day_of(2023, 6, 14);
    let key = invented(master_span(Ok(listed), None));
    let (first, last) = (on(2023, 6, 1), on(2023, 6, 30));
    let listing = i64::from(listed.days_from_epoch());
    let mut stored = Vec::new();
    let mut owed = 0_u32;
    for day in listing..=last {
        if calendar::kind_of(day) == DayKind::Open(calendar::Session::full()) {
            stored.extend(
                full_day(day)
                    .into_iter()
                    .filter(|stamp| !(day == on(2023, 6, 20) && ist(*stamp).1 == 600)),
            );
            owed += 375;
        }
    }
    let ledger = classify_cash(&stored, first, last, None, &key);
    assert_eq!(ledger.expected, owed);
    assert_eq!(ledger.lost_minutes(), 1, "the hole after the listing");
    assert!(ledger.gaps.iter().any(|gap| gap.day == on(2023, 6, 20)
        && gap.from == 600
        && gap.reason == Reason::VendorHole));
    let before = Reason::NotOwed(NotOwed::BeforeListing { listed_on: listed });
    for day in first..listing {
        if session(day) {
            assert!(
                ledger
                    .gaps
                    .iter()
                    .any(|gap| gap.day == day && gap.reason == before),
                "{day}"
            );
        }
    }
    assert!(
        ledger
            .gaps
            .iter()
            .filter(|gap| gap.day >= listing)
            .all(|gap| gap.reason != before)
    );
    assert_eq!(before.word(), "before-listing");
}

/// **A day no calendar attests is still before the listing** (COR-07, the
/// IRFC case: bars a share holds in 2018, before its 2021 listing, on days
/// the cash door's table cannot vouch for). The instrument owed nothing
/// whatever the exchange did, so each day reads one run of `before-listing`,
/// not `unmeasured`; a bar on it is still held. Whether a day was read needs
/// a session to read, so `unread` evidence leaves such a day `unmeasured`.
#[test]
fn a_day_no_calendar_attests_before_listing_reads_before_listing_not_unmeasured() {
    let (first, last) = (on(2018, 5, 2), on(2018, 5, 4));
    assert_eq!(
        calendar::kind_of(first),
        DayKind::Unmeasured,
        "before the table"
    );
    let listed = day_of(2021, 6, 15);
    let key = invented(master_span(Ok(listed), None));
    let ledger = classify_cash(&[at(first, 600)], first, last, None, &key);
    assert_eq!(ledger.held, 1, "the bar is still held");
    assert_eq!((ledger.expected, ledger.lost_minutes()), (0, 0));
    assert_eq!(ledger.unmeasured_minutes(), 0);
    assert_eq!(ledger.not_owed_minutes(), 3 * 1_440);
    let before = Reason::NotOwed(NotOwed::BeforeListing { listed_on: listed });
    assert_eq!(
        ledger.gaps,
        (first..=last)
            .map(|day| Gap {
                day,
                from: 0,
                to: 1_439,
                reason: before,
            })
            .collect::<Vec<_>>()
    );
    let unread = Key::series("NSE", "INDEX", "NIFTY", None).read(Coverage::Stated {
        held_through: None,
        requested: Requested::NONE,
    });
    let index = classify_owed(&[], first, last, None, Door::SpotIndex, &unread);
    assert_eq!(
        index.unmeasured_minutes(),
        3 * 1_440,
        "no session, no reading"
    );
    assert_eq!(index.not_owed_minutes(), 0);
    // A CLOSED DAY STAYS CLOSED, whatever the instrument's facts.
    let sunday = on(2023, 12, 3);
    assert_eq!(calendar::kind_of(sunday), DayKind::Closed);
    let closed = classify_cash(&[], sunday, sunday, None, &forcemot());
    assert!(closed.gaps.iter().all(|gap| gap.reason == Reason::Closed));
}

/// **A delisted share owes nothing after its removal day, and still owes
/// the removal day itself** (COR-07; whether `RmvlDt` is the last day traded
/// is UNVERIFIED, so it is never excused).
#[test]
fn a_delisted_share_owes_nothing_after_its_removal_day() {
    let removed = day_of(2023, 6, 16);
    let key = invented(master_span(Ok(day_of(2010, 1, 4)), Some(removed)));
    let (first, last) = (on(2023, 6, 15), on(2023, 6, 21));
    let ledger = classify_cash(&[], first, last, None, &key);
    let after = Reason::NotOwed(NotOwed::AfterDelisting {
        delisted_on: removed,
    });
    assert_eq!(ledger.expected, 2 * 375, "the 15th and the removal day");
    assert_eq!(ledger.lost_minutes(), 2 * 375);
    for day in [on(2023, 6, 19), on(2023, 6, 20), on(2023, 6, 21)] {
        assert!(
            ledger
                .gaps
                .iter()
                .any(|gap| gap.day == day && gap.reason == after)
        );
    }
    assert!(
        ledger
            .gaps
            .iter()
            .filter(|gap| gap.day <= on(2023, 6, 16))
            .all(|gap| gap.reason != after)
    );
    let named = ledger.not_owed();
    assert_eq!(named.len(), 1);
    assert_eq!((named[0].days, named[0].minutes), (3, 3 * 375));
}

/// **TC-04 at the classifier: an unread day is not a loss, and a read one
/// still is.** A spot index read through the 27th: the 28th, past its last
/// held day with no ledger request, is `unread` and owes nothing; once the
/// ledger records the request, the same empty session is the vendor's hole.
/// A month whose file is absent is unread whole.
#[test]
fn an_unread_day_is_not_a_loss_and_a_read_one_still_is() {
    let (read, unread) = (on(2026, 8, 27), on(2026, 8, 28));
    let stored = full_day(read);
    let through = Coverage::Stated {
        held_through: Some(read),
        requested: Requested::NONE,
    };
    let key = Key::series("NSE", "INDEX", "NIFTY", None).read(through);
    let ledger = classify_owed(&stored, read, unread, None, Door::SpotIndex, &key);
    assert_eq!(ledger.expected, 375);
    assert_eq!(ledger.lost_minutes(), 0);
    assert_eq!(ledger.not_owed_minutes(), 375);
    assert!(
        ledger
            .gaps
            .iter()
            .any(|gap| gap.day == unread && gap.reason == Reason::NotOwed(NotOwed::Unread))
    );
    let bits = [1_u64];
    let asked = key.read(Coverage::Stated {
        held_through: Some(read),
        requested: Requested {
            from: unread,
            bits: &bits,
        },
    });
    let ledgered = classify_owed(&stored, read, unread, None, Door::SpotIndex, &asked);
    assert_eq!(ledgered.expected, 750);
    assert_eq!(
        ledgered.lost_minutes(),
        375,
        "requested and empty: the vendor's"
    );
    let absent = key.read(Coverage::Stated {
        held_through: None,
        requested: Requested::NONE,
    });
    let whole = classify_owed(&[], read, unread, None, Door::SpotIndex, &absent);
    assert_eq!((whole.expected, whole.lost_minutes()), (0, 0));
    assert_eq!(whole.not_owed_minutes(), 750);
    // The doors that take no key answer exactly as before.
    let plain = classify_spot_index_against(&stored, read, unread, None);
    assert_eq!((plain.expected, plain.lost_minutes()), (750, 375));
}

/// **A recorded halt on a session not owed stays the halt** (a combination:
/// 2020-03-13's market-wide halt on a share listed the next session). Its
/// halted minutes are outside the window as on every door; the rest of the
/// session reads `before-listing`; nothing is owed or lost.
#[test]
fn a_halt_on_a_day_before_listing_stays_the_halt_and_the_rest_is_not_owed() {
    let day = on(2020, 3, 13);
    let halt = calendar::halt_on(day).expect("2020-03-13 is a recorded halt");
    let listed = day_of(2020, 3, 16);
    let key = invented(master_span(Ok(listed), None));
    let ledger = classify_cash(&[], day, day, None, &key);
    assert_eq!((ledger.expected, ledger.lost_minutes()), (0, 0));
    assert!(
        ledger
            .gaps
            .iter()
            .any(|gap| gap.reason == Reason::OutsideWindow
                && gap.from == halt.halted.from
                && gap.to == halt.halted.to)
    );
    assert_eq!(
        ledger.not_owed_minutes(),
        375 - u32::from(halt.halted.bars())
    );
}

/// **A bar on a session not owed is still held, and the rest of it is still
/// not owed** (a bar NSE never traded is evidence, never dropped here): one
/// bar on a withdrawn FORCEMOT day, and one from before the range, which is
/// skipped as every door skips it. The run is split around the bar, and the
/// day is named once.
#[test]
fn a_bar_on_a_session_not_owed_is_held_and_the_session_still_owes_nothing() {
    let day = on(2023, 12, 1);
    let stored = [at(day - 3, 600), at(day, 600)];
    let ledger = classify_cash(&stored, day, day, None, &forcemot());
    assert_eq!(ledger.held, 2);
    assert_eq!(ledger.invalid_timestamps, 0);
    assert_eq!((ledger.expected, ledger.lost_minutes()), (0, 0));
    assert_eq!(ledger.not_owed_minutes(), 374);
    let named = ledger.not_owed();
    assert_eq!((named.len(), named[0].days), (1, 1), "{named:?}");
    assert_eq!(
        ledger
            .gaps
            .iter()
            .filter(|gap| matches!(gap.reason, Reason::NotOwed(_)))
            .count(),
        2,
        "split around the held minute"
    );
}

/// **The bound on the answer holds for runs not owed, loudly** (maximum): a
/// FORCEMOT range of more than six years with no bar has more runs than
/// [`MAX_GAPS`]; the ledger says it was truncated, the withdrawal lies inside
/// what it lists, and no run it lists on a withdrawn session is a vendor
/// hole. Bad input with a key is still refused before anything is excused.
#[test]
fn runs_not_owed_still_truncate_loudly_and_bad_input_is_still_refused() {
    let (first, last) = (on(2020, 1, 1), on(2026, 8, 21));
    let ledger = classify_cash(&[], first, last, None, &forcemot());
    assert!(ledger.truncated, "{} runs", ledger.gaps.len());
    assert_eq!(ledger.gaps.len(), MAX_GAPS);
    let (from, through) = (on(2023, 10, 26), on(2024, 2, 13));
    assert!(
        ledger.gaps.last().is_some_and(|gap| gap.day > through),
        "the listed runs reach past the withdrawal"
    );
    let withdrawn: Vec<&Gap> = ledger
        .gaps
        .iter()
        .filter(|gap| (from..=through).contains(&gap.day))
        .collect();
    assert!(
        withdrawn
            .iter()
            .any(|gap| matches!(gap.reason, Reason::NotOwed(NotOwed::InstrumentInactive(_))))
    );
    assert!(withdrawn.iter().all(|gap| gap.reason != Reason::VendorHole));
    let day = on(2023, 12, 1);
    let off_grid = [at(day, 600) + 1];
    let refused = classify_cash(&off_grid, day, day, None, &forcemot());
    assert_eq!(refused.invalid_timestamps, 1);
    assert!(refused.gaps.is_empty());
    assert_eq!(refused.expected, 0);
}

/// **The key is built once, from the names, and the listing is asked only
/// for an NSE cash share** (the out-of-the-box case: a key for a share whose
/// store holds no master, and a listing handed to an index).
#[test]
fn a_key_is_built_from_exact_names_and_asks_the_listing_only_for_a_cash_share() {
    let scratch = std::env::temp_dir().join(format!(
        "brutex-gaps-key-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(scratch.join(crate::listing::MASTER_DIR)).unwrap();
    let table = Listing::load(&scratch.join(crate::listing::MASTER_DIR));
    let cash = Key::series("NSE", "CASH", "FORCEMOT", Some(&table));
    assert_eq!(
        cash.isin.map(|isin| isin.as_str().to_owned()).as_deref(),
        Some("INE451A01017")
    );
    assert_eq!(cash.span, None, "no master checks: nothing is known");
    assert_eq!(cash.coverage, Coverage::Unstated);
    let index = Key::series("NSE", "INDEX", "NIFTY", Some(&table));
    assert_eq!((index.isin, index.span), (None, None));
    assert_eq!(Key::series("NSE", "CASH", "NOSUCHSHARE", None).isin, None);
    assert_eq!(Key::NONE.read(Coverage::Unstated), Key::NONE);
    let _cleanup = std::fs::remove_dir_all(&scratch);
}

/// **Every reason has one word and, when not owed, one code**, the four codes
/// `web/` spells (R4-01), and only a vendor hole is a loss.
#[test]
fn every_reason_has_one_word_and_only_a_vendor_hole_is_lost() {
    let interval =
        crate::cash_auction::independently_inactive("FORCEMOT", "INE451A01017", day_of(2024, 1, 2))
            .unwrap();
    let listed = day_of(2021, 6, 15);
    let not_owed = [
        (
            NotOwed::InstrumentInactive(interval),
            "instrument_inactive",
            "instrument-inactive",
        ),
        (
            NotOwed::BeforeListing { listed_on: listed },
            "before_listing",
            "before-listing",
        ),
        (
            NotOwed::AfterDelisting {
                delisted_on: listed,
            },
            "after_delisting",
            "after-delisting",
        ),
        (NotOwed::Unread, "unread", "unread"),
    ];
    for (why, code, word) in not_owed {
        assert_eq!((why.code(), why.word()), (code, word));
        assert_eq!(Reason::NotOwed(why).word(), word);
        assert!(!Reason::NotOwed(why).is_loss());
    }
    for (reason, word, lost) in [
        (Reason::Closed, "closed", false),
        (Reason::OutsideWindow, "outside-window", false),
        (Reason::VendorHole, "vendor-hole", true),
        (Reason::Unmeasured, "unmeasured", false),
    ] {
        assert_eq!((reason.word(), reason.is_loss()), (word, lost));
    }
}

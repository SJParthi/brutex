#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::generator::day;

const ALL: Facts = Facts { listed: true, kind: DayKind::Full, member: Answer::Yes, in_span: true, has_minutes: true };

/// T-F06-4: the 2^5 masks of M1..M5 (M2 and M3 drawn as the day kind and the answer)
/// on an ordinary day, the outage day and the vendor-late day.
#[test]
fn t_f06_every_mask_on_every_day_kind() {
    let days = [day(2018, 9, 4), outage_day().unwrap(), vendor_late_day().unwrap()];
    for (di, d) in days.iter().enumerate() {
        for bits in 0..32u32 {
            let f = Facts {
                listed: bits & 1 == 0,
                kind: if bits & 2 == 0 { DayKind::Full } else { DayKind::Closed },
                member: if bits & 4 == 0 { Answer::Yes } else { Answer::No },
                in_span: bits & 8 == 0,
                has_minutes: bits & 16 == 0,
            };
            let got = admit(*d, &f);
            match (bits, di) {
                (0, 0) => assert_eq!(got, Verdict::Trade),
                (0, 2) => assert_eq!(got, Verdict::TradeFallback),
                _ => assert!(matches!(got, Verdict::NoTrade(_)), "{bits} {di}"),
            }
        }
    }
}

/// X-F06: the day mask at its edges.
#[test]
fn x_f06_admit_extremes() {
    let ordinary = day(2018, 9, 4);
    assert_eq!(admit(ordinary, &Facts { kind: DayKind::Muhurat, ..ALL }), Verdict::NoTrade(vec![Fail::NotFullSession(DayKind::Muhurat)]));
    assert_eq!(admit(ordinary, &Facts { kind: DayKind::Short, ..ALL }), Verdict::NoTrade(vec![Fail::NotFullSession(DayKind::Short)]));
    assert_eq!(admit(day(2018, 9, 3), &ALL), Verdict::Trade);
    assert_eq!(admit(day(2026, 10, 30), &ALL), Verdict::Trade);
    assert_eq!(admit(day(2026, 11, 2), &Facts { in_span: false, ..ALL }), Verdict::NoTrade(vec![Fail::OutsideSpan]));
    assert_eq!(admit(outage_day().unwrap(), &ALL), Verdict::NoTrade(vec![Fail::SkipRow]));
    // combo: the vendor-late day with membership unknown: M3 wins over the fallback.
    assert_eq!(
        admit(vendor_late_day().unwrap(), &Facts { member: Answer::Unknown, ..ALL }),
        Verdict::NoTrade(vec![Fail::NotMember(Answer::Unknown)])
    );
    // combo: the outage day outside the span lists both reasons, in order.
    assert_eq!(admit(outage_day().unwrap(), &Facts { in_span: false, ..ALL }), Verdict::NoTrade(vec![Fail::OutsideSpan, Fail::SkipRow]));
    // oob: every check failing at once, in the fixed order.
    assert_eq!(
        admit(outage_day().unwrap(), &Facts { listed: false, kind: DayKind::Closed, member: Answer::No, in_span: false, has_minutes: false }),
        Verdict::NoTrade(vec![Fail::NotListed, Fail::NotFullSession(DayKind::Closed), Fail::OutsideSpan, Fail::NoMinutes, Fail::SkipRow])
    );
}

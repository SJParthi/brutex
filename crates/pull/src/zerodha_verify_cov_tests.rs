#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): what the verifier reads a vendor stamp as
//! in each encoding, and what a day the exchange did not trade, or no
//! calendar of record attests, permits a series. Every stamp is invented.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

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
    let n = |y: u16, m: u8, d: u8| Day::new(y, m, d).unwrap().days_from_epoch();
    for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
        assert_eq!(
            permit(segment, n(2024, 3, 3)),
            Permit::Closed,
            "{segment:?}"
        );
        assert_eq!(
            permit(segment, n(2010, 1, 4)),
            Permit::Unattested,
            "{segment:?}"
        );
        assert_eq!(
            permit(segment, n(2027, 6, 1)),
            Permit::Unattested,
            "{segment:?}"
        );
    }
}

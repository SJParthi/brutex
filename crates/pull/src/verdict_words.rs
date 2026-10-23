//! The words a verdict record's opaque numbers stand for: its reason (`u16`)
//! and its detail kinds (`u16`). `store::verdict` keeps both opaque (D-4809:
//! their tables live in `pull`, which `store` cannot name); this is the one
//! table for every feed (the rules audit's RA-260: one table, never one per
//! verifier).
//!
//! # Append-only (D-4809)
//!
//! A number, once written, is never renamed, reused or renumbered. A new word
//! takes the next number at the END of its table, in the change that first
//! writes it. IA-9 (Zerodha) minted this table; IA-6 (GDFL) appends its
//! importer causes after the last entry, and keeps every number here.
//!
//! The detail kinds are the design's table (§6.4, "Details"), numbered there,
//! and every kind is named here even where no writer of this build sets it,
//! so a reader names every kind it meets.

/// Every reason, by number. Reason 0 is "none" and never listed.
pub const REASONS: [(u16, &str); 37] = [
    // The Zerodha gap ledger's `why=` words (D-3335, D-3344, D-3350, R3-18).
    (1, "vendor_empty"),
    (2, "withheld_unmeasured_session"),
    (3, "calendar_unverified"),
    (4, "calendar_past_record"),
    (5, "unordered_stamps"),
    (6, "refused_day"),
    (7, "refused_front"),
    (8, "repaired"),
    (9, "stored_short"),
    // The landing's per-row refusal words (D-3351; WP-13d's identity rules).
    (10, "timestamp_refused"),
    (11, "cash_eligibility_unknown"),
    (12, "price_off_grid"),
    (13, "off_grid_candle"),
    (14, "conflicting_candles"),
    (15, "before_first_day_bar"),
    (16, "foreign_level"),
    // The verifier's own words.
    (17, "unexplained"),
    (18, "index_bucket_incomplete"),
    (19, "pre_ledger"),
    (20, "receipts_full"),
    (21, "duplicate_body"),
    (22, "landed_from_store"),
    (23, "month_busy"),
    (24, "neither_vendor_has_it"),
    (25, "answer_short"),
    (26, "out_of_order"),
    (27, "duplicate"),
    (28, "month_refused"),
    (29, "pack_full"),
    (30, "storage_full"),
    (31, "refused_answer"),
    (32, "body_unreadable"),
    // RA-267: a derived rung holds no bar of a day its 1min source holds.
    (33, "rung_day_absent"),
    // RA-301: the gap-ledger word for the days a pull that halted (a dead
    // token, a stop) never asked for; read as `not_compared`, never a hole.
    // Its writer is the pull runtime's (WP-62); this table names it first so
    // the verifier reads it the day that line is written.
    (34, "pull_halted"),
    // R4-17 (WP-13d's identity rules, D-4991 to D-4995): the landing's three
    // per-row refusal words that merged after this table was minted. The
    // oracle takes them from the landing's refused rows, so without a number
    // each would be written as reason 0, a refusal with no word.
    (35, "non_positive_price"),
    (36, "before_listing"),
    (37, "in_session_jump"),
];

/// Every detail kind, by number (design §6.4). Kind 0 is never written.
pub const DETAILS: [(u16, &str); 21] = [
    (1, "name"),
    (2, "refusal"),
    (3, "observed"),
    (4, "quality_differs"),
    (5, "lineage"),
    (6, "manifest"),
    (7, "unknown_stems"),
    (8, "misfiled"),
    (9, "receipts"),
    (10, "peer"),
    (11, "spot"),
    (12, "daycheck"),
    (13, "placement"),
    (14, "rule"),
    (15, "dup_minutes"),
    (16, "scope"),
    (17, "repair"),
    (18, "landing_refused"),
    (19, "underlying_files"),
    // RA-318: an answer carrying fields the documented shape does not name.
    (20, "vendor_shape"),
    // IA-9 repair 1: a day bar `daycheck` could not read, and why.
    (21, "daycheck_not_run"),
];

/// The number of a reason word, or `None` for a word no table names.
#[must_use]
pub fn reason(word: &str) -> Option<u16> {
    REASONS
        .iter()
        .copied()
        .find(|(_, known)| *known == word)
        .map(|(number, _)| number)
}

/// The word of a reason number, or `None` for 0 and for a number this build
/// does not know (a later build's: shown as `reason_<n>`, never dropped).
#[must_use]
pub fn reason_word(number: u16) -> Option<&'static str> {
    REASONS
        .iter()
        .copied()
        .find(|(known, _)| *known == number)
        .map(|(_, word)| word)
}

/// The number of a detail kind's word.
#[must_use]
pub fn detail(word: &str) -> Option<u16> {
    DETAILS
        .iter()
        .copied()
        .find(|(_, known)| *known == word)
        .map(|(number, _)| number)
}

/// The word of a detail kind.
#[must_use]
pub fn detail_word(kind: u16) -> Option<&'static str> {
    DETAILS
        .iter()
        .copied()
        .find(|(known, _)| *known == kind)
        .map(|(_, word)| word)
}

/// Detail kind 9, `receipts`: the two receipts of a `vendor_revised` pair.
pub const DETAIL_RECEIPTS: u16 = 9;
/// Detail kind 10, `peer`: minutes absent while GDFL traded, the first,
/// minutes absent at both.
pub const DETAIL_PEER: u16 = 10;
/// Detail kind 12, `daycheck`: D-3347's counts.
pub const DETAIL_DAYCHECK: u16 = 12;
/// Detail kind 14, `rule`: the rule set and why a unit is
/// `built_under_earlier_rule`.
pub const DETAIL_RULE: u16 = 14;
/// Detail kind 15, `dup_minutes`: identical duplicate minutes counted once.
pub const DETAIL_DUP_MINUTES: u16 = 15;
/// Detail kind 18, `landing_refused`: rows the landing refused by name.
pub const DETAIL_LANDING_REFUSED: u16 = 18;
/// Detail kind 4, `quality_differs`: a count the verifier and the landing
/// disagree on.
pub const DETAIL_QUALITY_DIFFERS: u16 = 4;

/// Detail kind 20, `vendor_shape`: the fields of a kept answer outside the
/// documented shape (u32 count, then the first field's path; RA-318).
pub const DETAIL_VENDOR_SHAPE: u16 = 20;

/// Detail kind 21, `daycheck_not_run`: a stored day bar `daycheck` could not
/// compare, and why in one byte ([`DAYCHECK_BEYOND_MINUTES`],
/// [`DAYCHECK_NO_MINUTE`], [`DAYCHECK_MINUTES_UNREAD`]; IA-9 repair 1).
pub const DETAIL_DAYCHECK_NOT_RUN: u16 = 21;
/// `daycheck_not_run`'s byte 1: the day lies outside the series' held 1min
/// months (a series held at 1day only, a 1day month before its first 1min
/// month).
pub const DAYCHECK_BEYOND_MINUTES: u8 = 1;
/// `daycheck_not_run`'s byte 2: the 1min month holds no minute of the day.
pub const DAYCHECK_NO_MINUTE: u8 = 2;
/// `daycheck_not_run`'s byte 3: the 1min month would not open or read.
pub const DAYCHECK_MINUTES_UNREAD: u8 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_number_and_word_is_named_once_in_order() {
        for table in [REASONS.as_slice(), DETAILS.as_slice()] {
            for (at, (number, word)) in table.iter().enumerate() {
                assert_eq!(usize::from(*number), at + 1, "{word}: numbered in order");
                assert!(
                    !word.is_empty()
                        && word
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit())
                );
                assert_eq!(
                    table.iter().filter(|(_, w)| w == word).count(),
                    1,
                    "{word} once"
                );
            }
        }
        for (number, word) in REASONS {
            assert_eq!(reason(word), Some(number));
            assert_eq!(reason_word(number), Some(word));
        }
        for (number, word) in DETAILS {
            assert_eq!(detail(word), Some(number));
            assert_eq!(detail_word(number), Some(word));
        }
        assert_eq!(reason_word(0), None);
        assert_eq!(reason_word(u16::MAX), None);
        assert_eq!(reason("no_such_word"), None);
        assert_eq!(detail_word(0), None);
        for (kind, word) in [
            (DETAIL_RECEIPTS, "receipts"),
            (DETAIL_PEER, "peer"),
            (DETAIL_DAYCHECK, "daycheck"),
            (DETAIL_RULE, "rule"),
            (DETAIL_DUP_MINUTES, "dup_minutes"),
            (DETAIL_LANDING_REFUSED, "landing_refused"),
            (DETAIL_QUALITY_DIFFERS, "quality_differs"),
            (DETAIL_VENDOR_SHAPE, "vendor_shape"),
            (DETAIL_DAYCHECK_NOT_RUN, "daycheck_not_run"),
        ] {
            assert_eq!(detail_word(kind), Some(word));
        }
    }

    #[test]
    fn every_landing_refusal_word_and_ledger_why_has_a_reason() {
        use crate::fetch::RowCause;
        // EXHAUSTIVE, so a cause the landing gains fails to compile here until
        // it is listed below and given a number (R4-17 added three after this
        // table was minted, and each read as reason 0).
        let listed = |cause: RowCause| match cause {
            RowCause::Timestamp
            | RowCause::CashEligibility
            | RowCause::Price
            | RowCause::OffGrid
            | RowCause::Conflicting
            | RowCause::BeforeFirstDayBar
            | RowCause::ForeignLevel
            | RowCause::NonPositivePrice
            | RowCause::BeforeListing
            | RowCause::InSessionJump => cause,
        };
        for cause in [
            RowCause::Timestamp,
            RowCause::CashEligibility,
            RowCause::Price,
            RowCause::OffGrid,
            RowCause::Conflicting,
            RowCause::BeforeFirstDayBar,
            RowCause::ForeignLevel,
            RowCause::NonPositivePrice,
            RowCause::BeforeListing,
            RowCause::InSessionJump,
        ] {
            assert!(
                reason(listed(cause).code()).is_some(),
                "{} has a number",
                cause.code()
            );
        }
        for why in [
            "vendor_empty",
            "withheld_unmeasured_session",
            "calendar_unverified",
            "calendar_past_record",
            "unordered_stamps",
            "refused_day",
            "refused_front",
            "repaired",
            "stored_short",
        ] {
            assert!(reason(why).is_some(), "{why} has a number");
        }
    }
}

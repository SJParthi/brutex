//! BASE-COV-PULL: the words of the feed table and the const checks only the
//! compiler ran (Route B counts no compile-time run): at run time they answer
//! as they did when they let the build through.

use super::*;

#[test]
fn a_prefixed_pair_answers_its_own_prefix() {
    let scheme = AuthScheme::PrefixedPair {
        prefix: "token ",
        separator: ":",
    };
    assert_eq!(scheme.prefix(), "token ");
    assert_eq!(AuthScheme::Raw.prefix(), "");
    assert_eq!(AuthScheme::Bearer.prefix(), "Bearer ");
}

#[test]
fn a_claims_standing_says_where_it_came_from() {
    assert_eq!(
        ClaimStanding::VendorDocument.to_string(),
        "the vendor's published documentation"
    );
    assert_eq!(
        ClaimStanding::OperatorObservation.to_string(),
        "the operator's own observation"
    );
}

#[test]
fn an_auction_note_is_dated_from_the_rule_and_keeps_what_it_is_given() {
    let note = auction_note(AuctionHandling::PrintKeptApart, "an invented source");
    assert_eq!(note.from, AUG_3_2026);
    assert_eq!(note.continuous_close_minute, CAS_ELIGIBLE_CONTINUOUS_CLOSE);
    assert_eq!(note.handling, AuctionHandling::PrintKeptApart);
    assert_eq!(note.source, "an invented source");
}

/// The layout of an archive feed agrees with its own row, and a layout laid
/// against a broker's row, which has no archive, agrees with nothing.
#[test]
fn a_layout_agrees_with_its_own_archive_and_with_no_broker() {
    let gdfl = Feed::Gdfl.descriptor();
    assert!(shape_agrees(&GDFL_FNO, gdfl));
    assert!(!shape_agrees(&GDFL_FNO, Feed::Dhan.descriptor()));
}

// ───── BASE-COV-PULL-PS1 (D-7335): the checks' failing sides ─────

/// **A LAYOUT THAT DISAGREES WITH ITS ARCHIVE IS NAMED BY THE FIRST CHECK IT
/// FAILS**: one column short of its own shape fails the count, and the GDFL
/// layout (ten columns, a header) laid against the `TrueData` archive (no
/// header) passes the count of its own shape and fails the header. Extremes
/// at once: the count one short, and a header expected where none is.
#[test]
fn a_layout_that_disagrees_with_its_archive_is_named_by_its_first_failing_check() {
    let truedata = Feed::TrueData.descriptor();
    assert!(shape_agrees(&TRUEDATA_INDEX, truedata));
    let short = ColumnLayout {
        columns: TRUEDATA_INDEX
            .columns
            .split_last()
            .map_or(&[][..], |(_, rest)| rest),
        ..TRUEDATA_INDEX
    };
    assert_eq!(short.columns.len() + 1, TRUEDATA_INDEX.columns.len());
    assert!(!shape_agrees(&short, truedata));
    assert_eq!(GDFL_FNO.columns.len(), GDFL_FNO.shape.count());
    assert!(GDFL_FNO.shape.has_header());
    assert!(!shape_agrees(&GDFL_FNO, truedata));
}

/// A verified continuous row from `start` (a day count), well shaped when
/// `open < close`; the source is invented.
#[allow(clippy::expect_used, reason = "a test that cannot panic cannot fail")]
fn row(start: u32, open: u32, close: u32) -> SessionRow {
    SessionRow::verified(
        Day::from_days(start).expect("a day count of the 1970s"),
        open,
        close,
        SessionKind::Continuous,
        "an invented source",
    )
}

/// An index-venue table of `anchor` and `later`.
const fn table(
    anchor: SessionRow,
    later: [Option<SessionRow>; MAX_LATER_SESSION_ROWS],
) -> SessionTable {
    SessionTable {
        venue: Venue::NseIndex,
        anchor,
        later,
    }
}

/// **A SESSION TABLE OUT OF ITS SHIPPING SHAPE IS REFUSED BY EACH CHECK**:
/// a later row starting on the anchor's day does not ascend; a row whose open
/// is not before its close is not well shaped, wherever it sits (the anchor
/// beside two rows, the first of two, the only one, and a second after an
/// empty slot); and a table that covers every day but does not ascend is not
/// the shipping shape. Extremes at once: two later rows on one day, and an
/// open equal to its close.
#[test]
fn a_session_table_out_of_its_shipping_shape_is_refused_by_each_check() {
    let good = |start| row(start, 555, 930);
    let bad = |start| row(start, 930, 930);
    assert!(!table(good(0), [Some(good(0)), None]).rows_ascend());
    assert!(table(good(0), [Some(good(5)), None]).rows_ascend());
    assert!(!table(good(0), [Some(bad(5)), None]).rows_are_well_shaped());
    assert!(!table(bad(0), [Some(good(5)), Some(good(9))]).rows_are_well_shaped());
    assert!(!table(good(0), [Some(bad(5)), Some(good(9))]).rows_are_well_shaped());
    assert!(!table(good(0), [Some(good(5)), Some(bad(9))]).rows_are_well_shaped());
    assert!(!table(good(0), [None, Some(bad(9))]).rows_are_well_shaped());
    assert!(table(good(0), [None, Some(good(9))]).rows_are_well_shaped());
    assert!(!table(good(0), [Some(good(5)), Some(good(5))]).is_shipping_shape());
    assert!(table(good(0), [Some(good(5)), Some(good(9))]).is_shipping_shape());
    assert!(!table(good(1), [None, None]).is_shipping_shape());
}

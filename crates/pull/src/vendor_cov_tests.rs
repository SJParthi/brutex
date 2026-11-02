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

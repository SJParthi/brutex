//! Zerodha's index rows decode, and they decode to the ticker the engine sweeps.
//!
//! # The three defects one word caused
//!
//! Kite's published instrument-type alphabet is `EQ, FUT, CE, PE` — read from
//! its own column table — and carries no index word. The class lives in
//! `segment`, spelled `INDICES`. `decode_master_row` resolved everything from
//! the type word, so an index row arrived as `EQ` and:
//!
//! 1. the space-collapse, gated on `IDX`, never fired — so `NIFTY BANK`
//!    reached `Symbol::new`, which admits no space, and was `Malformed`;
//! 2. had it parsed, `(Segment, Kind)` would have been `(Cash, Equity)` — an
//!    index filed as a share;
//! 3. so the row never entered the master at all, and the pull refused with
//!    *"zerodha does not list BANKNIFTY"* for a vendor that lists it.
//!
//! Measured on the live file, 19 Aug 2026: `kept 10049`, **`row_errors 136`**,
//! against **exactly 136** NSE rows whose segment is `INDICES`.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use brutex_core::instrument::{Kind, Segment};
use brutex_core::vendor::{Decoded, MasterRow, Vendor, decode_master_row};

/// One Zerodha index row, exactly as the vendor's CSV writes it.
fn zerodha_index<'a>(tradingsymbol: &'a str, token: &'a str) -> MasterRow<'a> {
    MasterRow {
        vendor_id: token,
        exchange: "NSE",
        // THE COLUMN THAT CARRIES THE CLASS on this vendor, and the whole
        // reason this test exists.
        segment: "INDICES",
        underlying: "",
        trading_symbol: tradingsymbol,
        // `EQ`, NOT `IDX`. The vendor publishes no index code.
        instrument_type: "EQ",
        listing_class: "",
        isin: "",
        expiry: "",
        strike_rupees: "",
        option_side: "",
    }
}

/// **The two the engine sweeps, and the reference one, all by their canonical
/// ticker.**
#[test]
fn the_swept_indices_decode_to_the_ticker_the_engine_names() {
    // (what Zerodha writes, its token, what the engine sweeps it as)
    let rows = [
        ("NIFTY 50", "256265", "NIFTY"),
        ("NIFTY BANK", "260105", "BANKNIFTY"),
        // Needs no alias: the collapse alone yields what Groww already writes.
        ("INDIA VIX", "264969", "INDIAVIX"),
    ];
    for (spelled, token, canonical) in rows {
        let decoded = decode_master_row(Vendor::Zerodha, zerodha_index(spelled, token))
            .unwrap_or_else(|why| panic!("{spelled} must decode, got {why:?}"));
        let Decoded::Keep(listing) = decoded else {
            panic!("{spelled} must be KEPT, not skipped: {decoded:?}");
        };
        let (key, id) = (listing.key, listing.vendor_id);
        assert_eq!(
            key.underlying.as_str(),
            canonical,
            "{spelled} must become the exchange's ticker"
        );
        // AND IT MUST BE AN INDEX, not a share. Before the fix `ty` was `EQ`
        // and `listing_class` is empty for this vendor, so the row would have
        // landed on the `(Cash, Equity)` arm.
        assert_eq!(key.segment, Segment::Index, "{spelled}");
        assert_eq!(key.kind, Kind::Index, "{spelled}");
        assert_eq!(id.as_str(), token, "{spelled}: the token must survive");
    }
}

/// A space is still `Malformed` everywhere it is not an index name.
///
/// The promotion is gated on the segment word, so it cannot repair a stray
/// space in a derivative ticker — which is corruption, and turning it into a
/// real instrument is the §4 fallback that hides a failure.
#[test]
fn the_promotion_cannot_repair_a_row_that_is_not_an_index() {
    // A CASH ROW, because that is the case where an error is the answer.
    //
    // `NFO-FUT` would prove nothing: it is in this vendor's DECLINE list, so
    // the row is skipped before its symbol is ever parsed and `is_err` is
    // false for a reason that has nothing to do with the space. The first
    // draft of this test asserted on that row and passed for the wrong reason
    // in one direction and failed for the wrong reason in the other.
    //
    // `NSE` is stored, so the symbol IS parsed, and a space there is what it
    // has always been: malformed, loudly.
    let mut cash = zerodha_index("NIF TY", "1");
    cash.segment = "NSE";
    assert!(
        decode_master_row(Vendor::Zerodha, cash).is_err(),
        "a space in a CASH ticker is corruption, and the index promotion must \
         not reach it"
    );

    // AND THE DECLINED SEGMENTS ARE STILL DECLINED, not errors and not stored.
    // This is the half the operator asked for on 19 Aug 2026: Zerodha is spot
    // only, and an expired contract must never be attempted even if selected.
    for segment in ["NFO-FUT", "NFO-OPT", "MCX", "BSE"] {
        let mut derivative = zerodha_index("NIFTY26AUGFUT", "2");
        derivative.segment = segment;
        derivative.instrument_type = "FUT";
        derivative.expiry = "2026-08-27";
        let decoded = decode_master_row(Vendor::Zerodha, derivative)
            .unwrap_or_else(|why| panic!("{segment} is declined, not malformed: {why:?}"));
        assert!(
            decoded.skip().is_some(),
            "{segment} must be SKIPPED for this vendor, never stored"
        );
    }
}

/// The promotion is inert for every vendor that publishes an index code.
#[test]
fn no_other_vendor_is_reclassified_by_a_segment_word() {
    for vendor in [Vendor::Groww, Vendor::Dhan] {
        assert_eq!(
            vendor.index_segment_word(),
            None,
            "{vendor:?} writes an index code in the TYPE column; consulting its \
             segment could reclassify a row"
        );
        assert_eq!(vendor.index_alias("NIFTY50"), None, "{vendor:?}");
        assert_eq!(vendor.index_alias("NIFTYBANK"), None, "{vendor:?}");
    }
    assert_eq!(Vendor::Zerodha.index_segment_word(), Some("INDICES"));
    // An alias appears only where the collapsed form is NOT already the
    // ticker. A row restating a collapse would be a second answer.
    assert_eq!(Vendor::Zerodha.index_alias("INDIAVIX"), None);
    assert_eq!(Vendor::Zerodha.index_alias("NIFTY50"), Some("NIFTY"));
    assert_eq!(Vendor::Zerodha.index_alias("NIFTYBANK"), Some("BANKNIFTY"));
    // A name no vendor sends is not aliased into something that exists.
    assert_eq!(Vendor::Zerodha.index_alias(""), None);
    assert_eq!(Vendor::Zerodha.index_alias("NIFTY"), None);
    assert_eq!(Vendor::Zerodha.index_alias("nifty50"), None);
}

/// **THE ALIAS AND ITS INVERSE MUST NOT DRIFT, so one is walked through the other.**
///
/// `index_alias` renames the exchange's name to the store's key and
/// `index_alias_source` undoes it. They are two `match` arms in two functions,
/// which is two places to edit and one place to forget — and forgetting is not
/// a compile error, it is `/indexmap.json` quietly refusing an instrument.
///
/// Measured before the inverse existed: the join saw only the store's key, so
/// `NIFTY` matched 80 published NSE names and `BANKNIFTY` matched none. **The
/// two instruments `CLAUDE.md` §1 names as the entire engine surface were the
/// two its own exchange join could not resolve.**
#[test]
fn every_alias_round_trips_through_its_inverse() {
    for vendor in Vendor::ALL {
        for source in ["NIFTY50", "NIFTYBANK", "INDIAVIX", "NIFTY", "BANKNIFTY", ""] {
            if let Some(renamed) = vendor.index_alias(source) {
                assert_eq!(
                    vendor.index_alias_source(renamed),
                    Some(source),
                    "{vendor:?}: {source} -> {renamed} does not come back"
                );
            }
            if let Some(original) = vendor.index_alias_source(source) {
                assert_eq!(
                    vendor.index_alias(original),
                    Some(source),
                    "{vendor:?}: {source} <- {original} does not go forward"
                );
            }
        }
    }
}

/// The two the engine sweeps, named explicitly, because a loop that happens to
/// cover nothing still passes.
#[test]
fn the_swept_instruments_carry_the_exchange_name_they_were_renamed_from() {
    assert_eq!(Vendor::Zerodha.index_alias_source("NIFTY"), Some("NIFTY50"));
    assert_eq!(
        Vendor::Zerodha.index_alias_source("BANKNIFTY"),
        Some("NIFTYBANK")
    );
    // Not renamed, so nothing to undo -- `INDIAVIX` is what Zerodha lists.
    assert_eq!(Vendor::Zerodha.index_alias_source("INDIAVIX"), None);
    assert_eq!(Vendor::Zerodha.index_alias_source(""), None);
    for vendor in [Vendor::Groww, Vendor::Dhan, Vendor::TrueData, Vendor::Gdfl] {
        assert_eq!(vendor.index_alias_source("NIFTY"), None, "{vendor:?}");
    }
}

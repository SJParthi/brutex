#![cfg(test)]
//! BASE-COV-PULL-PS2 (fork B): arms of the capital-market reader the suite
//! left unreached. The September 2018 header variant says its own line; the
//! row-width rules at their edges (no row read, rows of two widths, a count
//! past the bound); a stored scope passes over the index rows and symbol
//! changes no source applies; a series stem longer than a symbol and a code
//! is no series refusal. Every name, count and code here is invented.

#![expect(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use brutex_core::alias::{Status, SymbolChange};

use super::*;

/// The two header variants each say their own line, character for
/// character, and the two lines differ only by the space in `Open Interest`.
#[test]
fn each_header_variant_says_its_own_line() {
    let spaced = HeaderVariant::OpenInterestSpaced.text();
    let modern = HeaderVariant::OpenInterest.text();
    assert_eq!(spaced, HEADER_OPEN_INTEREST_SPACED);
    assert_eq!(modern, HEADER_OPEN_INTEREST);
    assert_ne!(spaced, modern);
    assert!(spaced.ends_with("Open Interest"), "{spaced}");
    assert_eq!(spaced.replace("Open Interest", "OpenInterest"), modern);
    // The spaced line is the one header a first line of that text is read as.
    assert_eq!(HeaderShape::of(spaced.as_bytes()).columns, CM_FIELDS);
}

/// `changed` reads no change from no row read, whatever was noted, and
/// `continues` reads no continuation from rows of two widths, however many.
#[test]
fn row_widths_read_nothing_from_no_rows_and_nothing_from_two_widths() {
    // No row at all: nothing changed, nothing continues.
    let none = RowWidths::default();
    assert_eq!(none.changed(0), None);
    assert!(!none.continues(11, 0));
    // Rows of one width: a change only when every row read had it.
    let mut one = RowWidths::default();
    for _ in 0..3 {
        one.note(11);
    }
    assert_eq!(one.changed(3), Some(11));
    assert_eq!(one.changed(0), None, "no row read reads no change");
    assert_eq!(one.changed(4), None, "a row of the header's width was read");
    // Exactly the bound of rows of one width before a last line of it.
    let mut bound = RowWidths::default();
    for _ in 0..MAX_BAD_ROWS {
        bound.note(11);
    }
    assert!(bound.continues(11, MAX_BAD_ROWS));
    assert!(!bound.continues(12, MAX_BAD_ROWS), "another width");
    assert!(!bound.continues(11, MAX_BAD_ROWS - 1), "under the bound");
    // Past the bound, but two widths among them: never a continuation, and
    // never a change.
    let mut mixed = RowWidths::default();
    for _ in 0..MAX_BAD_ROWS {
        mixed.note(11);
    }
    mixed.note(12);
    let rows = MAX_BAD_ROWS + 1;
    assert!(!mixed.continues(11, rows));
    assert!(!mixed.continues(12, rows));
    assert_eq!(mixed.changed(rows), None);
    // The extremes of the count: u32::MAX rows read of none noted.
    assert_eq!(none.changed(u32::MAX), None);
    assert!(!none.continues(u32::MAX, u32::MAX));
}

/// An index row and a symbol change no source applies (`UNVERIFIED`), each
/// listed so a reader sees it, are passed over by a stored scope: no slot,
/// no share. The same rows sourced are filed (the contrast that shows the
/// status, not the names, decides).
#[test]
fn a_stored_scope_passes_over_rows_no_source_applies() {
    static UNVERIFIED_ROWS: [IndexRow; 1] = [IndexRow {
        fno: None,
        nse_name: "ZQ INVENTED INDEX",
        index_symbol: "ZQ INVENTED",
        kite: "ZQ INVENTED",
        store_key: "ZQINVENTED",
        reference_only: false,
        status: Status::Unverified("an invented row no source names"),
    }];
    static SOURCED_ROWS: [IndexRow; 1] = [IndexRow {
        fno: None,
        nse_name: "ZQ INVENTED INDEX",
        index_symbol: "ZQ INVENTED",
        kite: "ZQ INVENTED",
        store_key: "ZQINVENTED",
        reference_only: false,
        status: Status::Sourced(&[]),
    }];
    let unverified_change = SymbolChange {
        old: "ZQOLDCO",
        new: "ZQNEWCO",
        effective: Ymd::new(2024, 3, 1),
        status: Status::Unverified("an invented change no source names"),
    };
    let sourced_change = SymbolChange {
        status: Status::Sourced(&[]),
        ..unverified_change
    };
    let none: [&str; 0] = [];
    let passed = CmScope::stored_with(&UNVERIFIED_ROWS, &[unverified_change], none).unwrap();
    assert_eq!(passed.extra_slots(CmKind::Indices), 0);
    assert_eq!(passed.extra_slots(CmKind::Stocks), 0);
    let index_stem = format!("ZQ INVENTED{}", INDICES.stem_suffix);
    let old_stem = format!("ZQOLDCO{}", STOCKS.stem_suffix);
    let new_stem = format!("ZQNEWCO{}", STOCKS.stem_suffix);
    assert_eq!(passed.extra_slot(CmKind::Indices, &index_stem), None);
    assert_eq!(passed.extra_slot(CmKind::Stocks, &old_stem), None);
    let filed = CmScope::stored_with(&SOURCED_ROWS, &[sourced_change], none).unwrap();
    assert_eq!(filed.extra_slots(CmKind::Indices), 1);
    assert_eq!(filed.extra_slots(CmKind::Stocks), 2);
    assert_eq!(filed.extra_slot(CmKind::Indices, &index_stem), Some(0));
    assert_eq!(filed.extra_slot(CmKind::Stocks, &old_stem), Some(0));
    assert_eq!(filed.extra_slot(CmKind::Stocks, &new_stem), Some(1));
    // Both kinds at once, mixed: only the sourced ones count.
    let mixed =
        CmScope::stored_with(&UNVERIFIED_ROWS, &[unverified_change, sourced_change], none).unwrap();
    assert_eq!(
        (
            mixed.extra_slots(CmKind::Indices),
            mixed.extra_slots(CmKind::Stocks)
        ),
        (0, 2)
    );
}

/// A stocks stem whose name is longer than a symbol, a dot and the longest
/// series code is no series refusal: refused past its length before it is
/// read, as an out-of-scope stem. A name exactly at the bound is read.
#[test]
fn a_series_stem_longer_than_a_symbol_and_a_code_is_no_series_refusal() {
    let none: [&str; 0] = [];
    let scope = CmScope::stored(none).unwrap();
    let day = Day::new(2024, 4, 2).unwrap();
    let stem = |name: &str| format!("{name}{}", STOCKS.stem_suffix);
    // One past the bound, with a dot and an unknown code inside it.
    let share = "Z".repeat(SYMBOL_CAPACITY);
    let past = format!("{share}.T0ZZZZZZZ");
    assert_eq!(past.len(), SYMBOL_CAPACITY + 1 + MAX_SERIES_CODE + 1);
    assert_eq!(
        scope.series_refusal(CmKind::Stocks, &stem(&past), day),
        None
    );
    // Far past it.
    let far = "Q".repeat(4_096);
    assert_eq!(scope.series_refusal(CmKind::Stocks, &stem(&far), day), None);
    // A stem of the other tree, and the swept scope, are never asked.
    assert_eq!(
        scope.series_refusal(CmKind::Indices, &stem(&past), day),
        None
    );
    assert_eq!(
        CmScope::swept().series_refusal(CmKind::Stocks, &stem(&past), day),
        None
    );
}

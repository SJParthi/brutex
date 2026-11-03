//! BASE-COV-PULL-PS1 (D-7335): the constituent link in each attribute
//! spelling, one that never closes, and a file over the row bound. Every page,
//! symbol and identifier below is invented.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use core::fmt::Write as _;

use super::*;

/// **THE CONSTITUENT LINK IS FOUND IN EITHER QUOTE, PAST AN UNQUOTED ONE, AND
/// A LINK THAT NEVER CLOSES IS NO LINK**: an unquoted `href` is passed over, a
/// single-quoted one is read to its closing quote, and a value whose quote
/// never closes ends the scan with `NoConstituentLink` naming the page,
/// never a truncated URL. Extremes at once: the unquoted attribute is itself
/// a constituent CSV (a shape this scan does not read), and the unterminated
/// value is the only other link on its page.
#[test]
fn the_constituent_link_is_found_in_either_quote_and_never_in_an_open_one() {
    let page = "<a href=https://zq.invalid//IndexConstituent/ind_zqbare.csv>bare</a>\
        <a href='https://zq.invalid//IndexConstituent/ind_zqsingle.csv'>one</a>";
    assert_eq!(
        constituent_link(page, "zq-page").expect("the single-quoted link"),
        "https://zq.invalid//IndexConstituent/ind_zqsingle.csv"
    );
    let open = "<a href=\"https://zq.invalid//IndexConstituent/ind_zqopen.csv>never closed";
    let NseError::NoConstituentLink { page } =
        constituent_link(open, "zq-open-page").expect_err("a value that never closes")
    else {
        panic!("an open value is no link");
    };
    assert_eq!(page, "zq-open-page");
    let bare = "<a href=https://zq.invalid//IndexConstituent/ind_zqbare.csv>bare</a>";
    assert!(matches!(
        constituent_link(bare, "zq-bare-page"),
        Err(NseError::NoConstituentLink { .. })
    ));
}

/// **A FILE OVER THE ROW BOUND IS REFUSED BY ITS COUNT** before any row is
/// read: one row past [`MAX_CONSTITUENTS`] is `TooManyRows` naming both
/// numbers, and exactly the bound decodes. Extremes at once: the bound and one
/// past it, under the byte bound both times.
#[test]
fn a_file_over_the_row_bound_is_refused_by_its_count() {
    let body = |rows: usize| {
        let mut text = format!("{CONSTITUENT_HEADER}\n");
        for row in 0..rows {
            let _ = writeln!(
                text,
                "ZQ Invented {row} Ltd.,Invented,ZQ{row},EQ,INZQ{row:08}"
            );
        }
        text
    };
    let over = body(MAX_CONSTITUENTS + 1);
    assert!(over.len() < MAX_DOCUMENT_BYTES);
    let NseError::TooManyRows { rows, cap } =
        constituents(&over).expect_err("one row past the bound")
    else {
        panic!("the bound is TooManyRows");
    };
    assert_eq!((rows, cap), (MAX_CONSTITUENTS + 1, MAX_CONSTITUENTS));
    assert_eq!(
        NseError::TooManyRows { rows, cap }.to_string(),
        format!("the file holds {rows} rows; this build accepts at most {cap}")
    );
    let held = body(MAX_CONSTITUENTS);
    assert_eq!(
        constituents(&held).expect("exactly the bound").len(),
        MAX_CONSTITUENTS
    );
}

//! BASE-COV-PULL-PS1 (D-7335): the options reader's arms no test had reached
//! (Route B, recipe v4): a year zip the host will not open for a reason other
//! than its absence, a row past the fixed field array, and a last line with no
//! newline after it that carries the rows' changed column count, read through
//! `read_file`, which says the shape. Every ticker, price and file is
//! invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::{Method, bts, csv, put, row, scratch, zip};

/// An invented option of the day form: `ZQOPT`, expiring 4 April 2024,
/// strike 77,000, a call.
const TICKER: &str = "ZQOPT04APR2477000CE";

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// The entry an options day folder files `ticker` under.
fn entry(on: Day, ticker: &str) -> String {
    format!("{}\\Options\\{ticker}.NFO.csv", day_folder_name(on))
}

/// **A YEAR ZIP THE HOST WILL NOT OPEN IS REFUSED BY THE HOST'S KIND, AS ROOT
/// TOO.** `2025.zip` is a link through a plain file, so opening it fails
/// `NotADirectory` whoever runs the test (the module's own case, a zip of
/// mode 0o000, binds no root, so on this host the arm never ran). Its own
/// year's day refuses it by that kind, a day held nowhere answers it as the
/// unread zip it is, and a day held at its own place in a zip that opens
/// still reads.
#[test]
fn a_year_zip_the_host_will_not_open_is_refused_by_its_kind() {
    let root = scratch("ps1.nfo.year-link");
    let plain = put(&root, "plain", b"invented");
    let dir = root.join("zips");
    let held = day(2024, 4, 1);
    let file = csv(&[row(
        &format!("{TICKER}{ROW_SUFFIX}"),
        held,
        33_300,
        "4.35",
        75,
        1_000,
    )]);
    let day_zip = zip(&[(&entry(held, TICKER), &file, Method::Deflated)]);
    put(
        &dir,
        "2024.zip",
        &zip(&[(&NfoZips::day_zip_name(held), &day_zip, Method::Stored)]),
    );
    std::os::unix::fs::symlink(plain.join("2025.zip"), dir.join("2025.zip")).unwrap();
    let zips = NfoZips::new(&dir);
    let refused = Some(CmRefusal::ArchiveUnavailable {
        kind: std::io::ErrorKind::NotADirectory,
    });
    assert_eq!(
        zips.day(day(2025, 4, 1)).err(),
        refused,
        "its own year's day"
    );
    assert_eq!(
        zips.day(day(2024, 4, 2)).err(),
        refused,
        "a day held nowhere that reads"
    );
    let listing = zips.day(held).unwrap().unwrap();
    assert_eq!(
        zips.fetch(&listing.entries()[0]).unwrap(),
        file,
        "a day at its own place"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// **A ROW PAST THE FIXED FIELD ARRAY IS MALFORMED, NEVER WRITTEN PAST IT.**
/// The array holds `FIELDS + MAX_TRAILING_EMPTY` (18) fields. `parse_row`
/// asked for one trailing field more than that, a width only a direct call
/// can ask (`header_trailing` admits at most `MAX_TRAILING_EMPTY`), refuses a
/// row of 19 fields at the first field it has no slot for: a malformed row of
/// no time, at its line. At the array's own bound the same row reads.
#[test]
fn a_row_past_the_fixed_field_array_is_malformed() {
    let on = day(2024, 4, 1);
    let stem = format!("{TICKER}{ROW_SUFFIX}");
    let date = format!("{:02}/{:02}/{:04}", on.day(), on.month(), on.year());
    let base = row(&stem, on, 33_300, "4.35", 75, 1_000);
    let past = format!("{base}{}", ",".repeat(MAX_TRAILING_EMPTY + 1));
    assert_eq!(
        parse_row(past.as_bytes(), 7, (&stem, &date), MAX_TRAILING_EMPTY + 1),
        Err(RowError::Row {
            fault: RowFault::Malformed,
            sod: None,
            refusal: NfoRefusal::MalformedRow { line: 7 },
        })
    );
    let fits = format!("{base}{}", ",".repeat(MAX_TRAILING_EMPTY));
    assert_eq!(
        parse_row(fits.as_bytes(), 7, (&stem, &date), MAX_TRAILING_EMPTY),
        Ok((
            NfoRow {
                line: 7,
                sod: 33_300,
                ltp: 435,
                ltq: 75,
                oi: 1_000,
            },
            false
        ))
    );
}

/// **A LAST LINE WITH NO NEWLINE THAT CARRIES THE ROWS' CHANGED COUNT IS THE
/// CHANGE, NOT A CUT, AND `read_file` SAYS ITS SHAPE.** Under a header of two
/// empty trailing fields (D-4829: twelve names), 65 rows each carry a
/// thirteenth field, the last with no newline after it. The 64 rows before it
/// (D-3318's bound) all had its count, so the last line continues the shape
/// rather than cutting the file, and the file is `columns_changed` by the
/// rows' shape. With one row fewer the last line is a cut row, refused at its
/// line. Read through `read_file` the refusal is the same, and the shape has
/// been said: a second note of it says nothing. (The module's options case
/// ends every row with a newline, and no options test reached `read_file`'s
/// `columns_changed` arm. These twelve names under thirteen-field rows are no
/// other test's shape: the said-once memory is one per process.)
#[test]
fn a_last_line_of_the_changed_count_is_the_change_and_read_file_says_it() {
    let on = day(2024, 4, 1);
    let stem = format!("{TICKER}{ROW_SUFFIX}");
    let header = format!("{HEADER_OPEN_INTEREST},,");
    let text = |count: u32| {
        let mut out = format!("{header}\r\n");
        for i in 0..count {
            if i > 0 {
                out.push_str("\r\n");
            }
            out.push_str(&row(
                &stem,
                on,
                33_300 + i,
                "4.35",
                75,
                1_000 + u64::from(i),
            ));
            out.push_str(",,,");
        }
        out.into_bytes()
    };
    let bound = MAX_BAD_ROWS + 1;
    let shape = HeaderShape::of_rows(header.as_bytes(), 13);
    let changed = NfoRefusal::ColumnsChanged(ColumnSet {
        shape,
        header: 12,
        rows: bound,
    });
    assert_eq!(decode(&text(bound), &stem, on), Err(changed.clone()));
    assert_eq!(
        decode(&text(MAX_BAD_ROWS), &stem, on),
        Err(NfoRefusal::MalformedRow {
            line: MAX_BAD_ROWS + 1
        }),
        "one row fewer: the last line is a cut row"
    );
    let root = scratch("ps1.nfo.columns-cut");
    put(
        &root,
        "options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&[(&entry(on, TICKER), &text(bound))]),
    );
    let store = NfoTickStore::new(&root);
    let listing = store.day(on).unwrap().unwrap();
    assert_eq!(
        read_file(&store, &listing, &listing.entries()[0]),
        Err(changed)
    );
    assert!(
        !crate::gdfl_format::note_columns_changed("options", on, "again", &shape),
        "read_file said the shape"
    );
    std::fs::remove_dir_all(root).unwrap();
}

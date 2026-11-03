//! BASE-COV-PULL-PS1 (D-7335): the refusals a fetch names, and the length each
//! array of a window carries. Every row, price and identifier below is
//! invented.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::refusal::Disposition;
use crate::vendor::Listing;

/// **EVERY FETCH REFUSAL SAYS WHAT IT REFUSED**: a bars request holding a
/// field of the contract lookup, a listing this feed has no word for, and a
/// vendor's refusal with and without the name it gave. Extremes at once: the
/// longest disposition beside an empty detail, and the status the vendor
/// names beside one this build names.
#[test]
fn every_fetch_refusal_says_what_it_refused() {
    let said = FetchError::NotABarsParam {
        field: "expiryCode",
    }
    .to_string();
    assert!(
        said.starts_with("the request field \"expiryCode\" belongs to the expired-F&O"),
        "{said}"
    );
    assert!(said.contains("Nothing was sent"), "{said}");
    let said = FetchError::ListingNotSpellable {
        listing: Listing::Equity,
        field: "segment",
    }
    .to_string();
    assert!(
        said.starts_with(
            "this feed names the instrument's class in the request field \"segment\" and no \
             word for Equity has been recorded"
        ),
        "{said}"
    );
    assert!(said.contains("UNVERIFIED"), "{said}");
    for disposition in Disposition::ALL {
        let named = FetchError::VendorRefused {
            status: 403,
            detail: String::new(),
            named: Some(disposition),
        }
        .to_string();
        assert_eq!(
            named,
            format!("the vendor refused with status 403 and named it: {disposition}. It said: ")
        );
    }
    let unnamed = FetchError::VendorRefused {
        status: 502,
        detail: "an invented outage".to_owned(),
        named: None,
    }
    .to_string();
    assert_eq!(
        unnamed,
        "the vendor refused with status 502: an invented outage"
    );
}

/// Parallel arrays of `n` rows each, every value invented.
fn arrays(n: usize) -> ParallelArrays {
    let column = |base: i64| {
        (0..n)
            .map(|i| base + i64::try_from(i).expect("small"))
            .collect()
    };
    ParallelArrays {
        open: column(4_100),
        high: column(4_130),
        low: column(4_090),
        close: column(4_120),
        volume: column(7),
        timestamp: (0..n)
            .map(|i| 1_664_768_700 + 60 * i64::try_from(i).expect("small"))
            .collect(),
        open_interest: Vec::new(),
    }
}

/// One array cut short, and the seven lengths the refusal must name.
type Cut = (fn(&mut ParallelArrays), [usize; 7]);

/// **A WINDOW WHOSE ARRAYS DISAGREE IS REFUSED BY EVERY LENGTH**: each of
/// high, low, close, volume and timestamp one row short of open, and an open
/// interest neither empty nor full, is a `LengthDisagreement` naming that
/// length; an empty open interest and a full one both decode. Extremes: a
/// window of no rows, and every array but open empty at once.
#[test]
fn a_window_whose_arrays_disagree_is_refused_by_every_length() {
    let short = |cut: fn(&mut ParallelArrays)| {
        let mut a = arrays(3);
        cut(&mut a);
        RawWindow::decode(&a).expect_err("one array is short")
    };
    let cuts: [Cut; 6] = [
        (|a| a.high.truncate(2), [3, 2, 3, 3, 3, 3, 0]),
        (|a| a.low.truncate(2), [3, 3, 2, 3, 3, 3, 0]),
        (|a| a.close.truncate(2), [3, 3, 3, 2, 3, 3, 0]),
        (|a| a.volume.truncate(2), [3, 3, 3, 3, 2, 3, 0]),
        (|a| a.timestamp.truncate(2), [3, 3, 3, 3, 3, 2, 0]),
        (|a| a.open_interest = vec![1, 2], [3, 3, 3, 3, 3, 3, 2]),
    ];
    for (cut, want) in cuts {
        let FetchError::LengthDisagreement {
            open,
            high,
            low,
            close,
            volume,
            timestamp,
            open_interest,
        } = short(cut)
        else {
            panic!("a short array is a length disagreement");
        };
        assert_eq!(
            [open, high, low, close, volume, timestamp, open_interest],
            want
        );
    }
    let mut lonely = arrays(2);
    lonely.high.clear();
    lonely.low.clear();
    lonely.close.clear();
    lonely.volume.clear();
    lonely.timestamp.clear();
    assert!(matches!(
        RawWindow::decode(&lonely),
        Err(FetchError::LengthDisagreement {
            open: 2,
            high: 0,
            timestamp: 0,
            ..
        })
    ));
    assert!(
        RawWindow::decode(&arrays(0))
            .expect("no rows agree")
            .rows
            .is_empty()
    );
    let mut full = arrays(2);
    full.open_interest = vec![9, 10];
    let rows = RawWindow::decode(&full).expect("every array full").rows;
    assert_eq!(
        rows.iter().map(|row| row.open_interest).collect::<Vec<_>>(),
        [Some(9), Some(10)]
    );
    let rows = RawWindow::decode(&arrays(2)).expect("no interest").rows;
    assert_eq!(rows[1].open_interest, None);
    assert_eq!((rows[1].open, rows[1].timestamp), (4_101, 1_664_768_760));
}

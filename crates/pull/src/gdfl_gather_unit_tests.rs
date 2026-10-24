//! IA-17b: THE GATHER'S OWN HELPERS, AT THE UNIT (D-4899).
//!
//! The walk that gives each gathered file the bars its month's repair
//! added (`give_added`), the merge of a file gathered inside or before what
//! an offer holds (`merge_into`), the window an offer widens to for a day
//! outside it (`widen_window`), and a file with no bars. The import's own
//! tests (`gdfl_gather_tests.rs`) read them in day order only, where none
//! of these arms is reached.
//!
//! Every bar is invented here; no vendor row is quoted.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// One invented bar at `second` since the epoch.
fn bar(second: i64) -> Bar {
    Bar {
        ts_micros: second * 1_000_000,
        open: 500_000,
        high: 500_004,
        low: 499_996,
        close: 500_001,
        volume: 0,
        open_interest: store::format::OI_NULL,
    }
}

fn bars(seconds: std::ops::Range<i64>) -> Vec<Bar> {
    seconds.map(bar).collect()
}

/// A request over `from..=to`.
fn request(from: Day, to: Day) -> BarRequest {
    BarRequest {
        instrument_id: String::new(),
        listing: ImportKind::Indices.listing(),
        window: crate::session::Window::new(from, to).unwrap(),
        granularity: crate::vendor::Granularity::Second1,
    }
}

fn april() -> super::super::HeldKey {
    (
        "NIFTY".to_owned(),
        None,
        day(2024, 4, 1).year_month().unwrap(),
    )
}

fn waiting(on: Day) -> Deferred {
    Deferred {
        key: format!("indices {on} *"),
        day: on,
        close: None,
        failures: Vec::new(),
        committed: 0,
    }
}

/// **EACH FILE IS GIVEN ITS OWN ADDED BARS, IN ONE WALK, AND A FILE OUT OF
/// ORDER WALKS AGAIN.** Stamps before a file's first are passed over; a
/// file whose span begins at or before the last file's end restarts the
/// walk and counts what falls inside its own span; a file whose day is not
/// in the waiting list is given nothing and moves no day's count.
#[test]
fn each_gathered_file_is_given_the_stamps_inside_its_own_span() {
    let offered = |at: usize, span: (i64, i64)| OfferedFile {
        at,
        name: format!("file-{at}"),
        span,
    };
    let offer = Offer {
        key: april(),
        bars: Vec::new(),
        appended: FailKind::NotHeld,
        files: vec![
            offered(0, (10, 20)),
            offered(1, (30, 40)),
            offered(2, (15, 35)),
            offered(9, (60, 70)),
        ],
        auction: Vec::new(),
        request: request(day(2024, 4, 1), day(2024, 4, 30)),
    };
    let added = [5, 12, 18, 25, 33, 36, 50];
    let mut deferred = vec![
        waiting(day(2024, 4, 1)),
        waiting(day(2024, 4, 2)),
        waiting(day(2024, 4, 3)),
    ];

    give_added(&offer, &added, &mut deferred);

    let counts: Vec<usize> = deferred.iter().map(|one| one.committed).collect();
    assert_eq!(
        counts,
        vec![2, 2, 3],
        "12 and 18; 33 and 36; 18, 25 and 33 walked again"
    );
}

/// **A FILE GATHERED INSIDE OR BEFORE WHAT AN OFFER HOLDS IS MERGED IN
/// ORDER.** A file inside the held span whose seconds are alike adds
/// nothing and keeps the held ones after it; one before them is merged
/// ahead; one that holds a second differently is refused naming the
/// second, and the offer is left as it was.
#[test]
fn a_file_inside_or_before_the_offer_merges_in_order_and_a_changed_second_is_refused() {
    let mut held = bars(100..110);
    merge_into(&mut held, bars(102..104)).unwrap();
    assert_eq!(
        held,
        bars(100..110),
        "alike seconds kept once, the rest after"
    );

    merge_into(&mut held, bars(95..97)).unwrap();
    let stamps: Vec<i64> = held.iter().map(|one| one.ts_micros / 1_000_000).collect();
    let mut expected = vec![95, 96];
    expected.extend(100..110);
    assert_eq!(stamps, expected);

    let mut changed = bars(104..106);
    changed[1].close += 1;
    let before = held.clone();
    assert_eq!(merge_into(&mut held, changed), Err(105_000_000));
    assert_eq!(held, before, "a refused merge changes nothing");
}

/// **AN OFFER'S WINDOW WIDENS TO A DAY OUTSIDE IT**, to the widest of its
/// own window, the day's request and the day, so the repair's session
/// check keeps every gathered day's bars; a day inside it changes nothing.
#[test]
fn an_offers_window_widens_to_a_day_outside_it_and_never_for_one_inside() {
    let mut offer = request(day(2024, 4, 10), day(2024, 4, 12));

    widen_window(
        &mut offer,
        &request(day(2024, 4, 3), day(2024, 4, 5)),
        day(2024, 4, 20),
    );
    assert_eq!(
        (offer.window.from(), offer.window.to()),
        (day(2024, 4, 3), day(2024, 4, 20))
    );

    widen_window(
        &mut offer,
        &request(day(2024, 4, 1), day(2024, 4, 30)),
        day(2024, 4, 11),
    );
    assert_eq!(
        (offer.window.from(), offer.window.to()),
        (day(2024, 4, 3), day(2024, 4, 20)),
        "a day inside the window widens nothing"
    );
}

/// **A FILE WITH NO BARS GATHERS NOTHING**: no offer, no waiting close.
#[test]
fn a_file_with_no_bars_gathers_nothing() {
    let mut gather = Gather::default();
    gather
        .take(
            GatheredFile::for_test(
                &april(),
                Vec::new(),
                request(day(2024, 4, 1), day(2024, 4, 30)),
            ),
            ("indices 2024-04-01 *", day(2024, 4, 1)),
        )
        .unwrap();
    assert!(gather.is_empty());
    assert_eq!(gather.offered(&april()), None);
}

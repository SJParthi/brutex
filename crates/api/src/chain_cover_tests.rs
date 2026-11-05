//! Rows of `chain.rs` no test reached before BASE-COV-API repair 3 (shard
//! AS1, D-8661): the month after a month at the calendar's edges, the
//! lineage ledger's every refusal and its kept read, a census refusal when
//! the import journal would not read, a ladder over strikes none of which is
//! stored yet, and a spot asked at an instant no calendar can place. Every
//! strike and ledger here is invented.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;

fn month(year: u16, number: u8) -> YearMonth {
    YearMonth::new(year, number).expect("a month")
}

/// **THE MONTH AFTER A MONTH, AT THE CALENDAR'S EDGES** (BASE-COV-API repair
/// 3, D-8661): `next_month`. Extremes: January; November; December, into
/// the next year; the first month of the store's calendar (1970-01); the
/// last but one (9999-11); and, at once, December of the last year the
/// calendar holds (none follows) beside December of the first; out of the
/// box, a month walked twelve times from January back to January.
#[test]
fn the_month_after_a_month_holds_at_the_calendars_edges() {
    assert_eq!(next_month(month(2026, 1)), Some(month(2026, 2)));
    assert_eq!(next_month(month(2026, 11)), Some(month(2026, 12)));
    assert_eq!(next_month(month(2026, 12)), Some(month(2027, 1)));
    assert_eq!(next_month(month(1970, 1)), Some(month(1970, 2)));
    assert_eq!(next_month(month(9999, 11)), Some(month(9999, 12)));
    assert_eq!(next_month(month(9999, 12)), None, "no year past 9999");
    assert_eq!(next_month(month(1970, 12)), Some(month(1971, 1)));
    let mut walked = month(2026, 1);
    for _ in 0..12 {
        walked = next_month(walked).expect("a month");
    }
    assert_eq!(walked, month(2027, 1));
}

/// What `lineage_of` said, in one word and its reason.
fn said(lineage: &Lineage) -> String {
    match lineage {
        Lineage::NotKept => "not kept".to_owned(),
        Lineage::Absent => "absent".to_owned(),
        Lineage::Read(_) => "read".to_owned(),
        Lineage::Unreadable(why) => format!("unreadable: {why}"),
    }
}

/// **THE LINEAGE LEDGER IS READ, KEPT OR REFUSED BY NAME** (BASE-COV-API
/// repair 3, D-8661): `lineage_of`. Extremes: a feed that keeps no ledger;
/// no ledger on disk; an empty ledger, read; the same ledger asked again
/// (kept); the ledger grown (read again); its time moved with its length
/// kept (read again); a ledger past the 64 MiB the route reads; one that is
/// not UTF-8; a ledger path under a file; and, at once, a folder at the
/// ledger's name (it measures and will not read), and a second store's
/// ledger asked after the first was kept (read for itself); out of the box,
/// a ledger of one byte past the bound made sparse.
#[test]
fn the_lineage_ledger_is_read_kept_or_refused_by_name() {
    let root = crate::scratch::path("chain-cover-lineage");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    assert_eq!(said(&lineage_of(&root, Vendor::Dhan)), "not kept");
    assert_eq!(said(&lineage_of(&root, Vendor::Gdfl)), "absent");
    let path = pull::gdfl_lineage::lineage_path(&root);
    std::fs::create_dir_all(path.parent().expect("imports")).expect("imports");
    std::fs::write(&path, b"").expect("an empty ledger");
    assert_eq!(said(&lineage_of(&root, Vendor::Gdfl)), "read");
    assert_eq!(said(&lineage_of(&root, Vendor::Gdfl)), "read", "kept");
    std::fs::write(&path, b"\n").expect("a grown ledger");
    assert_eq!(said(&lineage_of(&root, Vendor::Gdfl)), "read", "grown");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the ledger");
    file.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000))
        .expect("a moved time");
    drop(file);
    assert_eq!(said(&lineage_of(&root, Vendor::Gdfl)), "read", "time moved");

    let other = crate::scratch::path("chain-cover-lineage-other");
    let _ = std::fs::remove_dir_all(&other);
    let other_path = pull::gdfl_lineage::lineage_path(&other);
    std::fs::create_dir_all(other_path.parent().expect("imports")).expect("imports");
    std::fs::write(&other_path, b"").expect("a second ledger");
    assert_eq!(
        said(&lineage_of(&other, Vendor::Gdfl)),
        "read",
        "another store"
    );

    std::fs::write(&path, [0xff, 0xfe]).expect("not UTF-8");
    let why = said(&lineage_of(&root, Vendor::Gdfl));
    assert!(why.ends_with("not UTF-8 text"), "{why}");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the ledger");
    file.set_len(MAX_LINEAGE_BYTES + 1).expect("sparse");
    drop(file);
    let why = said(&lineage_of(&root, Vendor::Gdfl));
    assert!(
        why.contains(&format!(
            "{} bytes, past the {MAX_LINEAGE_BYTES}",
            MAX_LINEAGE_BYTES + 1
        )),
        "{why}"
    );
    std::fs::remove_file(&path).expect("the big ledger gone");
    std::fs::create_dir_all(&path).expect("a folder at the ledger's name");
    let why = said(&lineage_of(&root, Vendor::Gdfl));
    assert!(why.starts_with("unreadable: "), "{why}");

    let under = crate::scratch::path("chain-cover-lineage-under");
    let _ = std::fs::remove_dir_all(&under);
    std::fs::create_dir_all(&under).expect("a scratch root");
    std::fs::write(under.join("imports"), b"a file where the folder goes").expect("a file");
    let why = said(&lineage_of(&under, Vendor::Gdfl));
    assert!(
        why.starts_with("unreadable: ") && why.contains("gdfl.lineage"),
        "{why}"
    );
    for dir in [root, other, under] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// **A CENSUS WHOSE IMPORT JOURNAL WOULD NOT READ PUTS ITS REFUSAL ON EVERY
/// INSTANT, A LADDER WITH NO STRIKE STORED YET NAMES NO ATM, AND AN INSTANT
/// NO CALENDAR CAN PLACE IS REFUSED** (BASE-COV-API repair 3, D-8661).
/// Extremes: a census of no feed (absent first); a journal that would not
/// read; a degraded census; a census behind its import before the instant;
/// strikes all first stored after the instant; the latest and the earliest
/// instant a spot can be asked at; and, at once, a journal that would not
/// read on a degraded census (the journal's refusal first), and strikes
/// stored after the instant with no spot at all (the spot's refusal first);
/// out of the box, a strike of no side stored at all.
#[test]
fn a_census_journal_that_will_not_read_and_a_ladder_of_nothing_stored_are_refused() {
    let view = |journal: Option<&str>, degraded: Option<&str>, state: &'static str| CensusView {
        state,
        generation: None,
        degraded: degraded.map(str::to_owned),
        behind_days: 0,
        behind_from: None,
        journal_unread: journal.map(str::to_owned),
    };
    let expiry = pull::session::Day::new(2026, 8, 27).expect("a day");
    let at = 1_787_000_000_000_000;
    assert_eq!(
        view(Some("no journal"), None, "absent")
            .refusal(at, expiry)
            .map(|(code, _)| code),
        Some("census_absent")
    );
    let (code, why) = view(Some("the journal would not read"), None, "held")
        .refusal(at, expiry)
        .expect("refused");
    assert_eq!(code, "census_state_unknown");
    assert!(why.contains("the journal would not read"), "{why}");
    assert_eq!(
        view(Some("both"), Some("damaged"), "held")
            .refusal(at, expiry)
            .map(|(code, _)| code),
        Some("census_state_unknown"),
        "the journal's refusal before the degraded one"
    );
    assert_eq!(
        view(None, Some("damaged"), "held")
            .refusal(at, expiry)
            .map(|(code, _)| code),
        Some("census_degraded")
    );

    let later = Side {
        first: Some(at + 60_000_000),
        adjusted: Vec::new(),
    };
    let strikes = [
        Strike {
            paisa: 2_400_000,
            call: later.clone(),
            put: later.clone(),
        },
        Strike {
            paisa: 2_450_000,
            call: Side::default(),
            put: Side::default(),
        },
    ];
    let ladder_now = ladder(&strikes, at, Ok(2_420_000));
    assert!(ladder_now.stored.is_empty() && ladder_now.atm.is_none());
    assert_eq!(
        ladder_now.refusal.map(|(code, _)| code),
        Some("no_strike_stored_by_then")
    );
    let no_spot = ladder(
        &strikes,
        at,
        Err(("no_spot", "no spot at this instant".to_owned())),
    );
    assert_eq!(no_spot.refusal.map(|(code, _)| code), Some("no_spot"));

    let root = crate::scratch::path("chain-cover-months");
    let mut months = Months {
        root: &root,
        feed: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        key: "NIFTY",
        timeframe: Timeframe::MINUTE_1,
        opened: Vec::new(),
    };
    let mut faults = Vec::new();
    for instant in [i64::MAX, i64::MIN] {
        let why = months
            .spot(instant, AsOf::Exact, &mut faults)
            .expect_err("no calendar places it");
        assert!(
            why.contains("is not an instant any calendar can place"),
            "{why}"
        );
    }
    assert!(
        faults.is_empty() && months.opened.is_empty(),
        "nothing opened"
    );
}

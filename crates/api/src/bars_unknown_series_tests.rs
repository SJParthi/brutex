//! A window key that names no series is refused by name (AG-03, finding
//! app-2), and a held series is answered exactly as before.
//!
//! Before this, `/bars/window.json` answered a key whose plan held no month
//! at all — an unknown symbol, a pre-rename name (`ZOMATO`), a BSE key, a
//! rung the feed never stores, a nonsense segment — `200` with every asked
//! month counted as `months_outside_life` and no fault: the same body, but
//! for `key`, that a REAL series asked outside its life gets. Nothing told
//! "no such series" from "nothing in range".
//!
//! Every store here is synthetic: invented symbols where the table is not the
//! subject, and prices no real instrument was ever quoted at.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// An index symbol no table names, so only the store decides it.
const INDEX: &str = "AGTHREEIDX";

/// A share symbol no table names.
const SHARE: &str = "AGTHREESHR";

/// A fresh store root no other live process names.
fn scratch(tag: &str) -> std::path::PathBuf {
    let root = crate::scratch::path(&format!("ag03-{tag}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    root
}

/// The census key `text` names.
fn key(text: &str) -> SeriesKey {
    SeriesKey::parse(text).expect("a census key")
}

/// A month.
fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).expect("a month")
}

/// Midnight UTC on the first of `month`, in micros.
fn month_start_micros(month: YearMonth) -> i64 {
    let (y, m) = (i64::from(month.year()), i64::from(month.month()));
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) * 86_400 * 1_000_000
}

/// Writes `n` bars of `key` at `timeframe` into `month`, from 09:15 IST on
/// the first, one rung apart. Every first of a month these tests use is an
/// NSE trading day. Prices are invented: 11.11 rupees and up.
fn plant(
    root: &std::path::Path,
    vendor: Vendor,
    key: &SeriesKey,
    timeframe: Timeframe,
    month: YearMonth,
    n: usize,
) {
    let path = StorePath::new(PathParts {
        vendor,
        exchange: &key.exchange,
        segment: &key.segment,
        symbol: &key.symbol,
        contract: key.contract,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .expect("a legal path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the cross-check the reader folds the same way"
    )]
    let symbol_id = brutex_core::universe::fnv1a(&key.symbol) as u32;
    let mut file = BarFile::open_or_create(root, path, symbol_id).expect("a bar file");
    let open_ist = month_start_micros(month) + 13_500 * 1_000_000;
    let step = i64::from(timeframe.secs()) * 1_000_000;
    let rows: Vec<Bar> = (0..n)
        .map(|i| {
            let nth = i64::try_from(i).expect("small");
            let close = 1_111 + nth;
            Bar {
                ts_micros: open_ist + nth * step,
                open: close,
                high: close + 3,
                low: close - 3,
                close,
                volume: 7,
                open_interest: OI_NULL,
            }
        })
        .collect();
    file.append(&rows).expect("the batch appends");
}

/// The window over `keys` at `timeframe` on `vendor`, `from` to `to`, newest
/// first, the page of 50.
fn ask(
    root: &std::path::Path,
    vendor: Vendor,
    keys: &[&str],
    timeframe: Timeframe,
    from: YearMonth,
    to: YearMonth,
) -> Result<Window, String> {
    window(
        root,
        &WindowQuery {
            vendor,
            timeframe,
            series: keys.iter().map(|text| key(text)).collect(),
            from,
            to,
            sort: SortKey::Ts,
            desc: true,
            offset: None,
            limit: 50,
            want_extremes: false,
            at: None,
            filter: Filter::default(),
            lookback: true,
            coverage: false,
        },
    )
}

/// A store holding zerodha 1min of the index and of `NSE-CASH-ETERNAL` in
/// 2024-03, five bars each.
fn held_store(tag: &str) -> std::path::PathBuf {
    let root = scratch(tag);
    for text in [format!("NSE-INDEX-{INDEX}"), "NSE-CASH-ETERNAL".to_owned()] {
        plant(
            &root,
            Vendor::Zerodha,
            &key(&text),
            Timeframe::MINUTE_1,
            ym(2024, 3),
            5,
        );
    }
    root
}

#[test]
fn an_unknown_symbol_is_refused_on_the_window() {
    let root = held_store("unknown");
    let why = ask(
        &root,
        Vendor::Zerodha,
        &["NSE-INDEX-NOTASYMBOL"],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("a key that names nothing is refused, not answered empty");
    assert_eq!(
        why,
        "unknown_series: NSE-INDEX-NOTASYMBOL names no series zerodha holds at 1min"
    );
    // THE HELD SERIES BESIDE IT IS ANSWERED, and whole.
    let held = ask(
        &root,
        Vendor::Zerodha,
        &[&format!("NSE-INDEX-{INDEX}")],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect("a held series answers");
    assert_eq!(held.total, 5);
    assert_eq!(held.months_outside_life, 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_renamed_old_name_is_refused_naming_its_successor() {
    let root = held_store("renamed");
    let one = |vendor: Vendor, text: &str| {
        ask(
            &root,
            vendor,
            &[text],
            Timeframe::MINUTE_1,
            ym(2024, 3),
            ym(2024, 3),
        )
    };
    // ZOMATO -> ETERNAL, from the alias table /spot.json resolves through
    // (D-3332): the successor is NAMED, never silently served.
    assert_eq!(
        one(Vendor::Zerodha, "NSE-CASH-ZOMATO").expect_err("refused"),
        "unknown_series: NSE-CASH-ZOMATO (now NSE-CASH-ETERNAL) names no series zerodha \
         holds at 1min"
    );
    assert_eq!(
        one(Vendor::Zerodha, "NSE-CASH-ETERNAL")
            .expect("the successor is held")
            .total,
        5
    );
    // An index asked by its F&O symbol is FILED under the table's key.
    assert_eq!(
        one(Vendor::Gdfl, "NSE-INDEX-FINNIFTY").expect_err("refused"),
        "unknown_series: NSE-INDEX-FINNIFTY (filed as NSE-INDEX-NIFTYFINSERVICE) names no \
         series gdfl holds at 1min"
    );
    // An index asked under the cash segment: the table says where it lives.
    assert_eq!(
        one(Vendor::Zerodha, "NSE-CASH-NIFTY").expect_err("refused"),
        "unknown_series: NSE-CASH-NIFTY (filed as NSE-INDEX-NIFTY) names no series zerodha \
         holds at 1min"
    );
    // NO SUCCESSOR IS GUESSED where the table records no keys: a feed outside
    // KEYED_FEEDS, a BSE key, a contract key.
    assert_eq!(
        one(Vendor::Dhan, "NSE-CASH-ZOMATO").expect_err("refused"),
        "unknown_series: NSE-CASH-ZOMATO names no series dhan holds at 1min"
    );
    assert_eq!(
        one(Vendor::Zerodha, "BSE-CASH-ZOMATO").expect_err("refused"),
        "unknown_series: BSE-CASH-ZOMATO names no series zerodha holds at 1min"
    );
    assert_eq!(
        one(Vendor::Gdfl, "NSE-FNO-ZOMATO-2024-03-28-990000-CE").expect_err("refused"),
        "unknown_series: NSE-FNO-ZOMATO-2024-03-28-990000-CE names no series gdfl holds at 1min"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_rung_the_feed_never_stores_is_refused() {
    let root = scratch("rung");
    let index = format!("NSE-INDEX-{INDEX}");
    plant(
        &root,
        Vendor::Gdfl,
        &key(&index),
        Timeframe::SECOND_1,
        ym(2024, 3),
        60,
    );
    let at = |timeframe| {
        ask(
            &root,
            Vendor::Gdfl,
            &[&index],
            timeframe,
            ym(2024, 3),
            ym(2024, 3),
        )
    };
    assert_eq!(
        at(Timeframe::MINUTE_1).expect_err("gdfl stores no 1min"),
        format!("unknown_series: {index} names no series gdfl holds at 1min")
    );
    assert_eq!(at(Timeframe::SECOND_1).expect("held at 1s").total, 60);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_bse_key_with_no_files_is_refused() {
    let root = held_store("bse");
    let why = ask(
        &root,
        Vendor::Zerodha,
        &["BSE-INDEX-SENSEX"],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("refused");
    assert_eq!(
        why,
        "unknown_series: BSE-INDEX-SENSEX names no series zerodha holds at 1min"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_window_with_one_unknown_key_among_held_ones_is_refused_naming_it() {
    let root = held_store("mixed");
    let index = format!("NSE-INDEX-{INDEX}");
    let why = ask(
        &root,
        Vendor::Zerodha,
        &[&index, "NSE-INDEX-NOTASYMBOL", "NSE-CASH-ETERNAL"],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("never a partial answer that hides one");
    assert_eq!(
        why,
        "unknown_series: 1 of 3 keys names no series zerodha holds at 1min: NSE-INDEX-NOTASYMBOL"
    );
    let two = ask(
        &root,
        Vendor::Zerodha,
        &["NSE-CASH-ZOMATO", &index, "NSE-FOO-NIFTY"],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("refused");
    assert_eq!(
        two,
        "unknown_series: 2 of 3 keys name no series zerodha holds at 1min: NSE-CASH-ZOMATO \
         (now NSE-CASH-ETERNAL), NSE-FOO-NIFTY (filed as NSE-INDEX-NIFTY)"
    );
    // The two held keys together answer, both totals named.
    let both = ask(
        &root,
        Vendor::Zerodha,
        &[&index, "NSE-CASH-ETERNAL"],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect("held");
    assert_eq!(both.total, 10);
    assert_eq!(both.series.len(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_held_series_outside_its_life_still_answers_200_unchanged() {
    let root = scratch("outside");
    let share = format!("NSE-CASH-{SHARE}");
    for month in [ym(2024, 3), ym(2024, 4)] {
        plant(
            &root,
            Vendor::Zerodha,
            &key(&share),
            Timeframe::MINUTE_1,
            month,
            4,
        );
    }
    for (from, to) in [(ym(2023, 1), ym(2023, 2)), (ym(2024, 6), ym(2024, 7))] {
        let answer = ask(
            &root,
            Vendor::Zerodha,
            &[&share],
            Timeframe::MINUTE_1,
            from,
            to,
        )
        .expect("a held series asked outside its life answers (D-3326, DB-065)");
        assert_eq!(answer.total, 0);
        assert_eq!(answer.months_read, 0);
        assert_eq!(answer.months_missing, 2);
        assert_eq!(answer.months_outside_life, 2);
        assert!(
            answer.missing_months.is_empty(),
            "{:?}",
            answer.missing_months
        );
        assert!(answer.faults.is_empty(), "{:?}", answer.faults);
        assert!(answer.bars.is_empty());
        assert_eq!(answer.series.len(), 1);
        assert_eq!(answer.series[0].key, share);
        assert_eq!(answer.series[0].total, 0);
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_index_hole_is_still_a_hole() {
    let root = scratch("hole");
    let index = format!("NSE-INDEX-{INDEX}");
    for month in [ym(2024, 2), ym(2024, 4)] {
        plant(
            &root,
            Vendor::Zerodha,
            &key(&index),
            Timeframe::MINUTE_1,
            month,
            3,
        );
    }
    let answer = ask(
        &root,
        Vendor::Zerodha,
        &[&index],
        Timeframe::MINUTE_1,
        ym(2024, 2),
        ym(2024, 4),
    )
    .expect("a held index with a hole answers");
    assert_eq!(answer.total, 6);
    assert_eq!(answer.missing_months, vec!["2024-03".to_owned()]);
    assert_eq!(answer.months_outside_life, 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_rung_directory_that_holds_no_bin_is_refused() {
    let root = scratch("nobin");
    let dir = root.join("bars/zerodha/NSE/INDEX/EMPTYIDX/1min");
    std::fs::create_dir_all(&dir).expect("the rung directory");
    std::fs::write(dir.join("README.txt"), b"not a month").expect("a stray file");
    std::fs::write(dir.join("2024-13.bin"), b"").expect("a name that is not a month");
    let why = ask(
        &root,
        Vendor::Zerodha,
        &["NSE-INDEX-EMPTYIDX"],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("a directory holding no month file names no series");
    assert_eq!(
        why,
        "unknown_series: NSE-INDEX-EMPTYIDX names no series zerodha holds at 1min"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_rung_directory_that_cannot_be_listed_keeps_its_own_error() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = scratch("unlisted");
    let index = format!("NSE-INDEX-{INDEX}");
    plant(
        &root,
        Vendor::Zerodha,
        &key(&index),
        Timeframe::MINUTE_1,
        ym(2024, 3),
        2,
    );
    let dir = root.join(format!("bars/zerodha/NSE/INDEX/{INDEX}/1min"));
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let listable = std::fs::read_dir(&dir).is_ok();
    let answer = ask(
        &root,
        Vendor::Zerodha,
        &[&index],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    );
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod back");
    if listable {
        // A superuser lists it anyway: then it is simply held.
        assert_eq!(answer.expect("held").total, 2);
    } else {
        let why = answer.expect_err("an unlistable directory is not an empty one");
        assert!(why.starts_with("cannot list "), "{why}");
        assert!(!why.contains("unknown_series"), "{why}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn max_series_keys_all_unknown_are_refused_in_one_bounded_sentence() {
    let root = held_store("maxseries");
    let texts: Vec<String> = (0..MAX_SERIES)
        .map(|n| format!("NSE-FNO-{INDEX}-2099-01-01-{}-CE", 1_000 + n))
        .collect();
    let keys: Vec<&str> = texts.iter().map(String::as_str).collect();
    let why = ask(
        &root,
        Vendor::Zerodha,
        &keys,
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("refused");
    let head = format!(
        "unknown_series: {MAX_SERIES} of {MAX_SERIES} keys name no series zerodha holds at \
         1min: "
    );
    assert!(why.starts_with(&head), "{why}");
    assert!(
        why.ends_with(&format!(", and {} more", MAX_SERIES - MAX_NAMED_UNKNOWN)),
        "{why}"
    );
    let named = why
        .strip_prefix(&head)
        .and_then(|rest| rest.split_once(", and "))
        .map(|(list, _)| list.split(", ").collect::<Vec<_>>())
        .expect("a list");
    assert_eq!(named, keys[..MAX_NAMED_UNKNOWN].to_vec());
    assert!(why.len() < 1_024, "bounded: {} bytes", why.len());
    // EXACTLY THE CAP NAMES EVERY ONE, with no count of the rest.
    let at_cap = ask(
        &root,
        Vendor::Zerodha,
        &keys[..MAX_NAMED_UNKNOWN],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("refused");
    assert!(!at_cap.contains(" more"), "{at_cap}");
    assert!(at_cap.ends_with(keys[MAX_NAMED_UNKNOWN - 1]), "{at_cap}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_key_differing_only_in_case_keeps_its_own_refusal() {
    let root = held_store("case");
    let why = ask(
        &root,
        Vendor::Zerodha,
        &["NSE-INDEX-agthreeidx"],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("refused before any plan");
    assert!(why.contains("wrong case"), "{why}");
    assert!(!why.starts_with("unknown_series"), "{why}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_expired_contract_that_has_files_is_held() {
    let root = scratch("expired");
    let contract = format!("NSE-FNO-{INDEX}-2019-03-28-1150000-CE");
    plant(
        &root,
        Vendor::Gdfl,
        &key(&contract),
        Timeframe::SECOND_1,
        ym(2019, 3),
        30,
    );
    let answer = ask(
        &root,
        Vendor::Gdfl,
        &[&contract],
        Timeframe::SECOND_1,
        ym(2019, 3),
        ym(2019, 5),
    )
    .expect("an expired contract that has files is held: 200");
    assert_eq!(answer.total, 30);
    // The months after its expiry are outside ITS life, exactly as before.
    assert_eq!(answer.months_outside_life, 2);
    assert!(answer.missing_months.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_empty_symbol_is_refused_and_never_answered_empty() {
    // The route's parser names no series for it (its 400 "is not a census
    // instrument key"), and a key built around it is refused by the window.
    assert!(SeriesKey::parse("NSE-INDEX-").is_none());
    let root = held_store("empty");
    let answer = window(
        &root,
        &WindowQuery::single(
            Vendor::Zerodha,
            SeriesKey::new("NSE", "INDEX", "", None),
            Timeframe::MINUTE_1,
            ym(2024, 3),
            ym(2024, 3),
            SortKey::Ts,
            true,
            0,
            50,
            false,
        ),
    );
    let why = answer.expect_err("an empty symbol is refused");
    assert!(!why.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_garbage_segment_is_refused() {
    let root = held_store("segment");
    // A symbol the alias table knows is pointed at where its price lives;
    // one it does not know is refused with no successor guessed.
    for (text, successor) in [
        ("NSE-FOO-NIFTY".to_owned(), " (filed as NSE-INDEX-NIFTY)"),
        (format!("NSE-FOO-{INDEX}"), ""),
    ] {
        let why = ask(
            &root,
            Vendor::Zerodha,
            &[&text],
            Timeframe::MINUTE_1,
            ym(2024, 3),
            ym(2024, 3),
        )
        .expect_err("refused");
        assert_eq!(
            why,
            format!("unknown_series: {text}{successor} names no series zerodha holds at 1min")
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_one_month_and_a_max_window_months_range_both_refuse_an_unknown_key() {
    let root = held_store("range");
    let from = ym(2006, 1);
    let last = MAX_WINDOW_MONTHS - 1;
    let to = ym(
        2006 + u16::try_from(last / 12).expect("small"),
        1 + u8::try_from(last % 12).expect("small"),
    );
    let index = format!("NSE-INDEX-{INDEX}");
    for (from, to) in [(ym(2024, 3), ym(2024, 3)), (from, to)] {
        let why = ask(
            &root,
            Vendor::Zerodha,
            &["NSE-INDEX-NOTASYMBOL"],
            Timeframe::MINUTE_1,
            from,
            to,
        )
        .expect_err("refused at both lengths");
        assert!(why.starts_with("unknown_series: "), "{why}");
    }
    // The held index over the longest range answers: one month read, and
    // what lies outside it is counted as before.
    let held = ask(
        &root,
        Vendor::Zerodha,
        &[&index],
        Timeframe::MINUTE_1,
        from,
        to,
    )
    .expect("held over the longest range");
    assert_eq!(held.total, 5);
    assert_eq!(held.months_read, 1);
    assert_eq!(held.months_missing, MAX_WINDOW_MONTHS - 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_no_feed_default_is_refused_naming_its_feed() {
    // A window with no `feed=` reads Dhan today (RT-DB-01, R4-02's rule): a
    // series held only under zerodha is now refused naming dhan, instead of
    // answering `total 0, months_outside_life 1`.
    let root = held_store("nofeed");
    let index = format!("NSE-INDEX-{INDEX}");
    let why = ask(
        &root,
        Vendor::Dhan,
        &[&index],
        Timeframe::MINUTE_1,
        ym(2024, 3),
        ym(2024, 3),
    )
    .expect_err("refused under the feed it was read on");
    assert_eq!(
        why,
        format!("unknown_series: {index} names no series dhan holds at 1min")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_superseded_key_names_the_key_the_lineage_moved_it_to() {
    // INVENTED contracts: the walk is the subject, not NIFTY's re-key.
    let root = scratch("superseded");
    let held = key(&format!("NSE-FNO-{INDEX}-2023-06-29-1850000-CE"));
    plant(
        &root,
        Vendor::Gdfl,
        &held,
        Timeframe::SECOND_1,
        ym(2023, 6),
        3,
    );
    let gone = key(&format!("NSE-FNO-{INDEX}-2023-06-29-1860000-CE"));
    let untouched = key(&format!("NSE-FNO-{INDEX}-2023-06-29-1870000-CE"));
    let row = |of: &SeriesKey, by: &str| SupersededBy {
        key: of.text(),
        by: by.to_owned(),
        kind: "closed_day",
        recorded: true,
    };
    let months = [ym(2023, 6)];
    let planned = |of: &SeriesKey| {
        SeriesPlan::build(
            &root,
            Vendor::Gdfl,
            Timeframe::SECOND_1,
            of.clone(),
            &months,
            Filter::default(),
            &Cancel::never(),
        )
        .expect("a plan")
    };
    let plans = [planned(&held), planned(&gone), planned(&untouched)];
    let query = WindowQuery::single(
        Vendor::Gdfl,
        held.clone(),
        Timeframe::SECOND_1,
        ym(2023, 6),
        ym(2023, 6),
        SortKey::Ts,
        true,
        0,
        50,
        false,
    );
    // THE HELD KEY'S ROW IS CONSUMED BY THE HELD PLAN, and the unknown key
    // after it gets its own row, never its neighbour's.
    let superseded = [
        row(&held, "NSE-FNO-X-2023-06-28-1850000-CE"),
        row(&gone, "NSE-FNO-X-2023-06-28-1860000-CE"),
    ];
    assert_eq!(
        refuse_unknown_series(&query, &plans, &superseded).expect_err("refused"),
        format!(
            "unknown_series: 2 of 3 keys name no series gdfl holds at 1s: {} (now \
             NSE-FNO-X-2023-06-28-1860000-CE), {}",
            gone.text(),
            untouched.text()
        )
    );
    // Only the HELD key superseded: no unknown key borrows its successor.
    assert_eq!(
        refuse_unknown_series(&query, &plans, &superseded[..1]).expect_err("refused"),
        format!(
            "unknown_series: 2 of 3 keys name no series gdfl holds at 1s: {}, {}",
            gone.text(),
            untouched.text()
        )
    );
    // Every key held: nothing to refuse, rows or not.
    assert_eq!(
        refuse_unknown_series(&query, &plans[..1], &superseded[..1]),
        Ok(())
    );
    // The lineage's successor comes first; without one a contract key gets
    // none guessed, and neither does a symbol no table row names.
    assert_eq!(
        successor_of(Vendor::Gdfl, &gone, Some("NSE-FNO-X-1")),
        Some("now NSE-FNO-X-1".to_owned())
    );
    assert_eq!(successor_of(Vendor::Gdfl, &gone, None), None);
    assert_eq!(
        successor_of(Vendor::Zerodha, &key("NSE-INDEX-NOTASYMBOL"), None),
        None
    );
    let _ = std::fs::remove_dir_all(&root);
}

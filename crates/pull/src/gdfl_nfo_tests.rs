//! Tests of the GDFL options reader. Every value is invented at run time.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::sync::Arc;

use super::*;
use crate::gdfl_fixtures::{Method, bts, bts_stating, csv, mon, put, row, scratch, zip};

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// A trade day the calendar records as a regular session.
fn trade() -> Day {
    day(2024, 4, 1)
}

fn contract(ticker: &str, on: Day) -> String {
    decode_ticker(ticker, on).unwrap().contract.as_str().to_owned()
}

// ── the contract a ticker names (FORMAT.md §6) ──────────────────────────────

#[test]
fn a_weekly_ticker_names_its_expiry_strike_and_side() {
    let got = decode_ticker("NIFTY04APR2422000CE", trade()).unwrap();
    assert_eq!(got.underlying.as_str(), "NIFTY");
    assert_eq!(got.contract.as_str(), "2024-04-04-2200000-CE");
    assert_eq!(contract("BANKNIFTY10APR2451500PE", trade()), "2024-04-10-5150000-PE");
}

#[test]
fn a_decimal_strike_is_paisa_exactly() {
    assert_eq!(contract("ABC25APR24107.5PE", trade()), "2024-04-25-10750-PE");
    assert_eq!(contract("ABC25APR24107.25PE", trade()), "2024-04-25-10725-PE");
}

#[test]
fn an_underlying_keeps_hyphens_ampersands_and_leading_digits() {
    for (ticker, under) in [
        ("NAM-INDIA25APR24240PE", "NAM-INDIA"),
        ("M&M25APR241500CE", "M&M"),
        ("360ONE25APR241000PE", "360ONE"),
        ("BAJAJ-AUTO25APR249000CE", "BAJAJ-AUTO"),
        ("NIFTYNXT5025APR2465000CE", "NIFTYNXT50"),
    ] {
        let got = decode_ticker(ticker, trade()).unwrap();
        assert_eq!(got.underlying.as_str(), under, "{ticker}");
    }
}

#[test]
fn the_monthly_form_states_no_expiry_day_and_is_refused_by_name() {
    assert_eq!(
        decode_ticker("ACC18OCT1280PE", day(2018, 10, 1)),
        Err(NfoRefusal::MonthlyExpiryUnstated {
            ticker: "ACC18OCT1280PE".to_owned()
        })
    );
}

#[test]
fn an_expiry_on_a_weekend_is_no_first_form_reading() {
    // 2024-04-06 is a Saturday.
    assert!(matches!(
        decode_ticker("NIFTY06APR2422000CE", trade()),
        Err(NfoRefusal::MonthlyExpiryUnstated { .. })
    ));
}

#[test]
fn an_expiry_on_a_weekday_holiday_is_kept_as_the_vendor_stated_it() {
    // 2024-03-29 was Good Friday, a trading holiday; the name is the identity.
    assert_eq!(contract("NIFTY29MAR2422000CE", day(2024, 3, 27)), "2024-03-29-2200000-CE");
}

#[test]
fn an_expiry_before_the_trade_day_or_past_the_horizon_is_refused() {
    assert!(decode_ticker("NIFTY28MAR2422000CE", trade()).is_err());
    // The trade day itself is an expiry day and is admitted.
    assert!(decode_ticker("NIFTY01APR2422000CE", trade()).is_ok());
    let name = |d: Day| format!("X{:02}{}{:02}100CE", d.day(), mon(d), d.year() % 100);
    // A trade day whose horizon and the day after it are both weekdays.
    let on = (0..7)
        .map(|k| Day::from_days(trade().days_from_epoch() + k).unwrap())
        .find(|t| {
            let edge = Day::from_days(t.days_from_epoch() + EXPIRY_HORIZON_DAYS).unwrap();
            is_weekday(edge) && is_weekday(edge.succ().unwrap())
        })
        .unwrap();
    let edge = Day::from_days(on.days_from_epoch() + EXPIRY_HORIZON_DAYS).unwrap();
    assert!(decode_ticker(&name(edge), on).is_ok(), "{edge}: the last admitted day");
    let past = edge.succ().unwrap();
    assert!(decode_ticker(&name(past), on).is_err(), "{past}: one past the horizon");
}

#[test]
fn every_other_shape_is_refused_by_name() {
    let unparsed = |t: &str| {
        assert!(
            matches!(
                decode_ticker(t, trade()),
                Err(NfoRefusal::TickerUnparsed { .. } | NfoRefusal::MonthlyExpiryUnstated { .. })
            ),
            "{t}"
        );
    };
    for strict in ["NIFTY04APR2422000XE".to_owned(), "CE".to_owned(), "ABCDE".repeat(13)] {
        assert_eq!(
            decode_ticker(&strict, trade()),
            Err(NfoRefusal::TickerUnparsed { ticker: strict.clone() }),
        );
    }
    unparsed("NIFTY04APR2422000XE");
    unparsed("CE");
    unparsed("NIFTY04APR24CE");
    unparsed("NIFTY04APR24.5CE");
    unparsed("NIFTY04APR2422000.CE");
    unparsed("NIFTY04APR2422000.123CE");
    unparsed("NIFTY04APR240CE");
    unparsed("NIFTY04ABC2422000CE");
    unparsed("NIFTY4APR2422000CE");
    unparsed(&format!("{}04APR2422000CE", "N".repeat(TICKER_CAP)));
    assert!(matches!(
        decode_ticker("nifty04APR2422000CE", trade()),
        Err(NfoRefusal::UnderlyingRefused { .. })
    ));
    assert!(matches!(
        decode_ticker(&format!("{}04APR2422000CE", "N".repeat(30)), trade()),
        Err(NfoRefusal::UnderlyingRefused { .. })
    ));
    assert!(matches!(
        decode_ticker("N04APR24100000000CE", trade()),
        Err(NfoRefusal::ContractUnrenderable { .. })
    ));
    assert!(decode_ticker("N04APR2499999999CE", trade()).is_ok(), "ten paisa digits fit");
}

#[test]
fn no_ticker_reads_as_two_contracts() {
    // Every split that parses as DD MON YY STRIKE, counted over a sweep of
    // shapes built to tempt a second reading: digit-ended underlyings,
    // month-like letters, strikes with dots.
    let readings = |t: &str| {
        let body = &t.as_bytes()[..t.len() - 2];
        (1..body.len())
            .filter(|&at| {
                let rest = &body[at..];
                rest.get(0..2).and_then(two_digits).is_some()
                    && rest.get(2..5).and_then(month_of).is_some()
                    && rest.get(5..7).and_then(two_digits).is_some()
                    && rest.get(7..).and_then(strike_paisa).is_some()
            })
            .count()
    };
    let unders = ["A", "A1", "A12", "12JAN", "JAN12", "X01JAN", "X01JAN25", "A-B", "M&M1"];
    let mids = ["01JAN25", "31DEC99", "12MAY12", "05AUG26"];
    let strikes = ["1", "10", "12.5", "1225", "01", "0.05"];
    let mut seen = 0;
    for u in unders {
        for m in mids {
            for s in strikes {
                let t = format!("{u}{m}{s}CE");
                assert!(readings(&t) <= 1, "{t}");
                seen += 1;
            }
        }
    }
    assert_eq!(seen, 9 * 4 * 6);
}

#[test]
fn weekday_and_month_helpers_hold_their_edges() {
    assert!(!is_weekday(day(2024, 4, 6)));
    assert!(!is_weekday(day(2024, 4, 7)));
    assert!(is_weekday(day(2024, 4, 8)));
    assert!(is_weekday(day(2024, 4, 5)));
    assert_eq!(month_of(b"JAN"), Some(1));
    assert_eq!(month_of(b"DEC"), Some(12));
    assert_eq!(month_of(b"Jan"), None);
    assert_eq!(two_digits(b"9"), None);
    assert_eq!(two_digits(b"a9"), None);
    assert_eq!(two_digits(b"9a"), None);
    assert_eq!(strike_paisa(b"0"), None);
    assert_eq!(strike_paisa(&[0xFF]), None);
    assert_eq!(strike_paisa(b".5"), None);
    assert_eq!(strike_paisa(b"1.x"), None);
    assert_eq!(strike_paisa(b"1x"), None);
    assert_eq!(strike_paisa(b"0.05"), Some(5));
}

#[test]
fn every_refusal_says_what_it_is() {
    let all = [
        NfoRefusal::TickerUnparsed { ticker: "T".into() },
        NfoRefusal::TickerAmbiguous { ticker: "T".into() },
        NfoRefusal::MonthlyExpiryUnstated { ticker: "T".into() },
        NfoRefusal::UnderlyingRefused { ticker: "T".into() },
        NfoRefusal::ContractUnrenderable { ticker: "T".into() },
        NfoRefusal::DuplicateTicker { ticker: "T".into() },
        NfoRefusal::HeaderUnknown,
        NfoRefusal::TickerMismatch { line: 7 },
        NfoRefusal::DateMismatch { line: 7 },
        NfoRefusal::MalformedRow { line: 7 },
        NfoRefusal::PriceRefused { line: 7 },
        NfoRefusal::OpenInterestRefused { line: 7 },
        NfoRefusal::RowsOverCap { cap: 7 },
        NfoRefusal::Source(CmRefusal::HeaderUnknown),
    ];
    let texts: Vec<String> = all.iter().map(ToString::to_string).collect();
    for (why, text) in all.iter().zip(&texts) {
        assert!(!text.is_empty(), "{why:?}");
    }
    let unique: std::collections::HashSet<&String> = texts.iter().collect();
    assert_eq!(unique.len(), texts.len(), "each refusal reads differently");
    assert!(texts[7].contains("line 7"));
    assert_eq!(
        NfoRefusal::from(CmRefusal::HeaderUnknown),
        NfoRefusal::Source(CmRefusal::HeaderUnknown)
    );
}

// ── rows ────────────────────────────────────────────────────────────────────

const STEM: &str = "NIFTY04APR2422000CE.NFO";

fn rows(spec: &[(u32, &str, u64, u64)]) -> Vec<String> {
    spec.iter()
        .map(|&(sod, ltp, ltq, oi)| row(STEM, trade(), sod, ltp, ltq, oi))
        .collect()
}

#[test]
fn every_row_decodes_in_file_order_with_its_open_interest() {
    let bytes = csv(&rows(&[(33_301, "9.9", 0, 260), (33_300, "10.05", 75, 300)]));
    let got = decode(&bytes, STEM, trade()).unwrap();
    assert_eq!(
        got.rows,
        vec![
            NfoRow { line: 2, sod: 33_301, ltp: 990, ltq: 0, oi: 260 },
            NfoRow { line: 3, sod: 33_300, ltp: 1_005, ltq: 75, oi: 300 },
        ]
    );
}

#[test]
fn line_endings_headers_and_a_missing_final_newline_are_read() {
    let lf = format!(
        "{HEADER_OPEN_INTEREST_SPACED}\n{}\n{}",
        rows(&[(33_300, "1", 1, 0)])[0],
        rows(&[(33_301, "2", 1, 0)])[0]
    );
    let got = decode(lf.as_bytes(), STEM, trade()).unwrap();
    assert_eq!(got.rows.len(), 2);
    assert_eq!(got.rows[1].ltp, 200);
    let header_only = format!("{HEADER_OPEN_INTEREST}\r\n");
    assert!(decode(header_only.as_bytes(), STEM, trade()).unwrap().rows.is_empty());
}

#[test]
fn every_row_fault_is_refused_by_name_at_its_line() {
    let good = rows(&[(33_300, "10", 1, 5)])[0].clone();
    let check = |line: String, want: NfoRefusal| {
        let bytes = csv(&[rows(&[(33_299, "10", 1, 5)])[0].clone(), line.clone()]);
        assert_eq!(decode(&bytes, STEM, trade()), Err(want), "{line}");
    };
    check(good.replacen(STEM, "OTHER.NFO", 1), NfoRefusal::TickerMismatch { line: 3 });
    check(good.replacen("/04/", "/05/", 1), NfoRefusal::DateMismatch { line: 3 });
    check(good.replacen("09:15:00", "9:15:00", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen("09:15:00", "24:00:00", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen("09:15:00", "09:60:00", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen("09:15:00", "09:15:60", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.rsplit_once(',').unwrap().0.to_owned(), NfoRefusal::MalformedRow { line: 3 });
    check(format!("{good},0"), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen(",10,0,0,0,0,", ",10,x,0,0,0,", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen(",10,0,0,0,0,", ",10,0,x,0,0,", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen(",10,0,0,0,0,", ",10,0,0,x,0,", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen(",10,0,0,0,0,", ",10,0,0,0,x,", 1), NfoRefusal::MalformedRow { line: 3 });
    check(good.replacen(",0,0,1,5", ",0,0,x,5", 1), NfoRefusal::MalformedRow { line: 3 });
    check(
        good.replacen(",0,0,1,5", ",0,0,99999999999999999999,5", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(good.replacen(",1,5", ",1,x", 1), NfoRefusal::MalformedRow { line: 3 });
    check(
        good.replacen(",1,5", ",1,9223372036854775808", 1),
        NfoRefusal::OpenInterestRefused { line: 3 },
    );
    check(good.replacen(",10,", ",10.001,", 1), NfoRefusal::PriceRefused { line: 3 });
    check(good.replacen(",10,", ",-10,", 1), NfoRefusal::PriceRefused { line: 3 });
    check(good.replacen(",10,", ",0.04,", 1), NfoRefusal::PriceRefused { line: 3 });
    check(
        good.replacen(",10,", ",92233720368547758.08,", 1),
        NfoRefusal::PriceRefused { line: 3 },
    );
    let mut bad = csv(&[good.clone()]);
    bad.insert(bad.len() - 3, 0xFF);
    assert_eq!(decode(&bad, STEM, trade()), Err(NfoRefusal::MalformedRow { line: 2 }));
}

#[test]
fn a_zero_price_is_admitted_only_on_a_row_with_no_trade() {
    let bytes = csv(&rows(&[(33_300, "0", 0, 9)]));
    assert_eq!(decode(&bytes, STEM, trade()).unwrap().rows[0].ltp, 0);
    let at_max = csv(&rows(&[(33_300, "92233720368547758.07", 1, 9_223_372_036_854_775_807)]));
    let got = decode(&at_max, STEM, trade()).unwrap();
    assert_eq!((got.rows[0].ltp, got.rows[0].oi), (i64::MAX, i64::MAX));
}

#[test]
fn the_header_and_the_row_bound_are_refused_by_name() {
    assert_eq!(decode(b"", STEM, trade()), Err(NfoRefusal::HeaderUnknown));
    assert_eq!(decode(b"Ticker,Date\r\n", STEM, trade()), Err(NfoRefusal::HeaderUnknown));
    let three = csv(&rows(&[(1, "1", 1, 0), (2, "1", 1, 0), (3, "1", 1, 0)]));
    assert!(decode_capped(&three, STEM, trade(), 3).is_ok());
    assert_eq!(
        decode_capped(&three, STEM, trade(), 2),
        Err(NfoRefusal::RowsOverCap { cap: 2 })
    );
}

// ── listings ────────────────────────────────────────────────────────────────

fn entry(day: Day, ticker: &str) -> String {
    format!("{}\\Options\\{ticker}.NFO.csv", day_folder_name(day))
}

#[test]
fn only_option_files_of_the_day_folder_are_filed_by_ticker() {
    let d = trade();
    let folder = day_folder_name(d);
    assert_eq!(folder, "GFDLNFO_TICK_01042024");
    assert_eq!(entry_ticker(&folder, &entry(d, "NIFTY04APR2422000CE")), Some("NIFTY04APR2422000CE"));
    assert_eq!(entry_ticker(&folder, &format!("{folder}/Options/A01APR241CE.NFO.CSV")), Some("A01APR241CE"));
    for other in [
        format!("{folder}\\Futures\\NIFTY-I.NFO.csv"),
        format!("{folder}\\Options\\"),
        format!("{folder}\\Options\\.NFO.csv"),
        format!("{folder}\\Options\\X\\Y.NFO.csv"),
        format!("{folder}\\Options\\Y.csv"),
        format!("{folder}Options\\Y.NFO.csv"),
        format!("{folder}\\OptionsY.NFO.csv"),
        "GFDLNFO_TICK_02042024\\Options\\Y.NFO.csv".to_owned(),
    ] {
        assert_eq!(entry_ticker(&folder, &other), None, "{other}");
    }
    let mut listing: NfoDay<u8> = NfoDay::new(d);
    listing.push(&entry(d, "A01APR241CE"), 3, 4, 0);
    listing.push(&format!("{folder}\\Futures\\X.NFO.csv"), 3, 4, 1);
    assert_eq!(listing.day(), d);
    assert_eq!(listing.skipped(), 1);
    assert_eq!(listing.entries().len(), 2);
    assert_eq!(listing.locate("A01APR241CE").unwrap().unwrap().locator, 0);
    assert_eq!(listing.locate("B01APR241CE").unwrap(), None);
    assert_eq!(listing.ticker_of(&listing.entries()[1]), None);
}

#[test]
fn a_ticker_listed_twice_is_refused_by_name() {
    let d = trade();
    let mut listing: NfoDay<u8> = NfoDay::new(d);
    listing.push(&entry(d, "A01APR241CE"), 3, 4, 0);
    listing.push(&format!("{}\\Options\\A01APR241CE.NFO.CSV", day_folder_name(d)), 3, 4, 1);
    assert_eq!(
        listing.locate("A01APR241CE"),
        Err(NfoRefusal::DuplicateTicker {
            ticker: "A01APR241CE".to_owned()
        })
    );
}

fn file_bytes(ticker: &str, spec: &[(u32, &str, u64, u64)]) -> Vec<u8> {
    let stem = format!("{ticker}.NFO");
    csv(&spec
        .iter()
        .map(|&(sod, ltp, ltq, oi)| row(&stem, trade(), sod, ltp, ltq, oi))
        .collect::<Vec<_>>())
}

#[test]
fn the_tick_store_and_the_zips_give_the_same_file() {
    let d = trade();
    let one = file_bytes("NIFTY04APR2422000CE", &[(33_300, "10", 75, 300)]);
    let two = file_bytes("NIFTY04APR2422000PE", &[(33_301, "12.5", 50, 100)]);
    let (e1, e2) = (entry(d, "NIFTY04APR2422000CE"), entry(d, "NIFTY04APR2422000PE"));
    let root = scratch("nfo-both");
    let store = NfoTickStore::new(&root.join("ts"));
    let path = store.day_path(d);
    assert!(path.ends_with("options/2024/APR_2024/GFDLNFO_TICK_01042024.bts"), "{path:?}");
    put(&root.join("ts"), "options/2024/APR_2024/GFDLNFO_TICK_01042024.bts", &bts(&[(&e1, &one), (&e2, &two)]));
    let day_zip = zip(&[(&e1, &one, Method::Deflated), (&e2, &two, Method::Stored)]);
    let year = zip(&[
        ("APR_2024/", b"", Method::Stored),
        ("APR_2024/GFDLNFO_TICK_01042024.zip", &day_zip, Method::Stored),
    ]);
    put(&root.join("zips"), "2024.zip", &year);
    let zips = NfoZips::new(&root.join("zips"));

    let from_store = store.day(d).unwrap().unwrap();
    let from_zips = zips.day(d).unwrap().unwrap();
    for (a, b) in from_store.entries().iter().zip(from_zips.entries()) {
        assert_eq!(a.entry, b.entry);
        let got_a = read_file(&store, &from_store, a).unwrap();
        let got_b = read_file(&zips, &from_zips, b).unwrap();
        assert_eq!(got_a, got_b);
        assert_eq!(store.fetch(a).unwrap(), zips.fetch(b).unwrap());
    }
    let got = read_file(&store, &from_store, &from_store.entries()[0]).unwrap();
    assert_eq!(got.name, "NIFTY04APR2422000CE");
    assert_eq!(got.ticker.contract.as_str(), "2024-04-04-2200000-CE");
    assert_eq!(got.file.rows[0].oi, 300);
    // A day neither holds.
    assert!(store.day(day(2024, 4, 2)).unwrap().is_none());
    assert!(zips.day(day(2024, 4, 2)).unwrap().is_none());
    assert!(zips.day(day(2025, 4, 2)).unwrap().is_none(), "no year zip at all");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_fault_of_a_source_is_refused_by_name() {
    let d = trade();
    let ticker = "NIFTY04APR2422000CE";
    let one = file_bytes(ticker, &[(33_300, "10", 75, 300)]);
    let e1 = entry(d, ticker);
    let root = scratch("nfo-faults");
    // A stated CRC that is not the bytes'.
    put(&root, "a/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts", &bts_stating(&[(&e1, &one, 7)]));
    let store = NfoTickStore::new(&root.join("a"));
    let listing = store.day(d).unwrap().unwrap();
    assert!(matches!(
        read_file(&store, &listing, &listing.entries()[0]),
        Err(NfoRefusal::Source(CmRefusal::SourceCrcMismatch { .. }))
    ));
    // A corrupt block.
    let mut broken = bts(&[(&e1, &one)]);
    broken[9] ^= 0xFF;
    put(&root, "b/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts", &broken);
    let store = NfoTickStore::new(&root.join("b"));
    let listing = store.day(d).unwrap().unwrap();
    assert!(matches!(
        read_file(&store, &listing, &listing.entries()[0]),
        Err(NfoRefusal::Source(CmRefusal::TickStoreMalformed { .. }))
    ));
    // Not a day file at all.
    put(&root, "c/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts", b"nope");
    assert!(matches!(
        NfoTickStore::new(&root.join("c")).day(d),
        Err(CmRefusal::TickStoreMalformed { .. })
    ));
    // A root that is a file: every open fails with "not a directory".
    let file_root = put(&root, "plain", b"x");
    assert!(matches!(
        NfoTickStore::new(&file_root).day(d),
        Err(CmRefusal::TickStoreUnavailable { .. })
    ));
    assert!(matches!(
        NfoZips::new(&file_root).day(d),
        Err(CmRefusal::ArchiveUnavailable { .. })
    ));
    // A file that is not an option file of the day, read anyway.
    let mut listing: NfoDay<crate::gdfl_nfo::StoreLocator> = NfoDay::new(d);
    let fine = NfoTickStore::new(&root.join("a")).day(d).unwrap().unwrap();
    let mut odd = fine.entries()[0].clone();
    odd.entry = "elsewhere.csv".into();
    listing.push("elsewhere.csv", odd.len, odd.crc32, odd.locator.clone());
    assert!(matches!(
        read_file(&store, &listing, &odd),
        Err(NfoRefusal::TickerUnparsed { .. })
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_fault_of_the_yearly_zip_is_refused_by_name() {
    let d = trade();
    let ticker = "NIFTY04APR2422000CE";
    let one = file_bytes(ticker, &[(33_300, "10", 75, 300)]);
    let e1 = entry(d, ticker);
    let name = "APR_2024/GFDLNFO_TICK_01042024.zip";
    let day_zip = zip(&[(&e1, &one, Method::Deflated)]);
    let open = |year: Vec<u8>| {
        let len = u64::try_from(year.len()).unwrap();
        listing_in(Arc::new(year), len, d)
    };
    // Two day zips for one day.
    assert_eq!(
        open(zip(&[(name, &day_zip, Method::Stored), (name, &day_zip, Method::Stored)])).err(),
        Some(CmRefusal::ArchiveDuplicateDay { day: d })
    );
    // A day zip that is itself compressed in the year zip.
    assert!(matches!(
        open(zip(&[(name, &day_zip, Method::Deflated)])).err(),
        Some(CmRefusal::ArchiveMemberCompressed { .. })
    ));
    // Not a zip.
    assert!(matches!(open(b"nope".to_vec()).err(), Some(CmRefusal::ArchiveMalformed { .. })));
    // A member whose deflate stream is cut: corrupt or short, never accepted.
    let year = zip(&[(name, &day_zip, Method::Stored)]);
    let listing = open(year.clone()).unwrap().unwrap();
    let file = &listing.entries()[0];
    let good = member_bytes(&*file.locator.file, file.locator.at, &file.entry, file.len).unwrap();
    assert_eq!(good, one);
    let mut cut = year;
    // The member's deflated bytes start after the outer and inner headers.
    let at = cut
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x03, 0x04])
        .unwrap()
        + 30
        + e1.len()
        + 2;
    cut[at] ^= 0xFF;
    cut[at + 1] ^= 0xFF;
    let listing = open(cut).unwrap().unwrap();
    let file = &listing.entries()[0];
    let got = member_bytes(&*file.locator.file, file.locator.at, &file.entry, file.len);
    match got {
        Err(CmRefusal::ArchiveMemberCorrupt { .. }) => {}
        Ok(bytes) => assert!(crate::gdfl_cm::verify(file, &bytes).is_err()),
        other => panic!("{other:?}"),
    }
}

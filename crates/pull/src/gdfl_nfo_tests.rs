#![cfg(test)]
//! Tests of the GDFL options reader. Every value is invented at run time.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::unwrap_used,
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
    decode_ticker(ticker, on)
        .unwrap()
        .contract
        .as_str()
        .to_owned()
}

// ── the contract a ticker names (FORMAT.md §6, revised) ─────────────────────

fn under(t: &str, on: Day) -> String {
    decode_ticker(t, on).unwrap().underlying.as_str().to_owned()
}

#[test]
fn a_modern_ticker_is_the_day_form_only() {
    let got = decode_ticker("NIFTY04APR2422000CE", trade()).unwrap();
    assert_eq!(got.underlying.as_str(), "NIFTY");
    assert_eq!(got.contract.as_str(), "2024-04-04-2200000-CE");
    assert_eq!(got.form, TickerForm::Day);
    assert_eq!(
        contract("BANKNIFTY10APR2451500PE", trade()),
        "2024-04-10-5150000-PE"
    );
    // From 2020 on every day-form name ALSO parses as YY MON STRIKE
    // (`ACC30` / APR / 201000); only the era keeps them apart.
    let on = day(2020, 4, 1);
    let got = decode_ticker("ACC30APR201000CE", on).unwrap();
    assert_eq!(
        (got.underlying.as_str(), got.form),
        ("ACC", TickerForm::Day)
    );
    assert_eq!(got.contract.as_str(), "2020-04-30-100000-CE");
}

#[test]
fn decimal_strikes_are_paisa_exactly_and_written_canonically() {
    for (strike, paisa) in [
        ("2.5", 250),
        ("77.5", 7_750),
        ("1012.5", 101_250),
        ("107.25", 10_725),
        ("0.5", 50),
    ] {
        let t = format!("ABC25APR24{strike}PE");
        let got = decode_ticker(&t, trade()).unwrap();
        assert_eq!(got.strike, paisa, "{t}");
        assert_eq!(got.ticker_text(), t);
    }
    for odd in [
        "100.00", "100.0", "0100", "107.50", "00.5", "1.", ".5", "1.234", "0",
    ] {
        let t = format!("ABC25APR24{odd}PE");
        assert!(decode_ticker(&t, trade()).is_err(), "{t}: not canonical");
    }
}

#[test]
fn an_underlying_keeps_hyphens_ampersands_and_digits() {
    for (ticker, want) in [
        ("NAM-INDIA25APR24240PE", "NAM-INDIA"),
        ("M&M25APR241500CE", "M&M"),
        ("360ONE25APR241000PE", "360ONE"),
        ("BAJAJ-AUTO25APR249000CE", "BAJAJ-AUTO"),
        ("NIFTYNXT5025APR2465000CE", "NIFTYNXT50"),
    ] {
        assert_eq!(under(ticker, trade()), want, "{ticker}");
    }
}

#[test]
fn before_the_switch_a_stock_reads_monthly_and_its_expiry_is_the_last_thursday() {
    let on = day(2018, 12, 3);
    // Read as 18 Dec 2019 strike 5 by the day form; the era says Dec 2018.
    let got = decode_ticker("ADANIENT18DEC195CE", on).unwrap();
    assert_eq!(got.form, TickerForm::Monthly);
    assert_eq!(got.contract.as_str(), "2018-12-27-19500-CE");
    assert_eq!(got.ticker_text(), "ADANIENT18DEC195CE");
    for (t, want) in [
        ("AMBUJACEM18DEC235CE", "2018-12-27-23500-CE"),
        ("APOLLOTYRE18DEC205PE", "2018-12-27-20500-PE"),
        ("TV18BRDCST18DEC24PE", "2018-12-27-2400-PE"),
        ("NIFTY18DEC10500CE", "2018-12-27-1050000-CE"),
    ] {
        assert_eq!(contract(t, on), want, "{t}");
    }
    assert_eq!(under("TV18BRDCST18DEC24PE", on), "TV18BRDCST");
    // The last Thursday of March 2018, the 29th, was Mahavir Jayanti: the
    // previous trading day, the 28th, is the expiry.
    assert_eq!(
        contract("ACC18MAR1300CE", day(2018, 3, 1)),
        "2018-03-28-130000-CE"
    );
    // A stock name in the day form before the switch has no reading.
    assert!(matches!(
        decode_ticker("ACC28FEB191260PE", day(2019, 1, 15)),
        Err(NfoRefusal::TickerUnparsed { .. })
    ));
    assert_eq!(
        contract("ACC28FEB191260PE", day(2019, 2, 1)),
        "2019-02-28-126000-PE"
    );
}

#[test]
fn before_the_switch_an_index_weekly_reads_the_day_form_on_any_weekday() {
    // Tue 2018-11-06: a holiday-shifted weekly.
    let got = decode_ticker("BANKNIFTY06NOV1825000CE", day(2018, 11, 1)).unwrap();
    assert_eq!(
        (got.form, got.contract.as_str()),
        (TickerForm::Day, "2018-11-06-2500000-CE")
    );
    assert_eq!(
        contract("BANKNIFTY06DEC1826000CE", day(2018, 12, 3)),
        "2018-12-06-2600000-CE"
    );
}

#[test]
fn a_name_with_two_readings_or_none_is_refused_by_name() {
    // An index weekly's day reading wins over its monthly one at the same
    // split: 19 Sep 2018 (the holiday-shifted Wednesday), not Sep 2019
    // strike 18,24,800.
    assert_eq!(
        contract("BANKNIFTY19SEP1824800PE", day(2018, 9, 10)),
        "2018-09-19-2480000-PE"
    );
    // Two readings, or none, refuse by name.
    let reading = |at| {
        Ok(Reading {
            at,
            form: TickerForm::Day,
            expiry: trade(),
            strike: 1,
        })
    };
    assert_eq!(
        single(vec![reading(1), reading(2)], "T").map(|r| r.at),
        Err(NfoRefusal::TickerAmbiguous {
            ticker: "T".to_owned()
        })
    );
    assert_eq!(
        single(vec![], "T").map(|r| r.at),
        Err(NfoRefusal::TickerUnparsed {
            ticker: "T".to_owned()
        })
    );
    assert_eq!(single(vec![reading(4)], "T").map(|r| r.at), Ok(4));
    // Traded after the expiry the rule derives.
    assert_eq!(
        decode_ticker("ACC18DEC1300CE", day(2018, 12, 28)),
        Err(NfoRefusal::ExpiryPassed {
            ticker: "ACC18DEC1300CE".to_owned()
        })
    );
    // A month the calendar never measured.
    assert_eq!(
        decode_ticker("ACC17DEC1300CE", day(2017, 12, 1)),
        Err(NfoRefusal::MonthlyExpiryUnmeasured {
            ticker: "ACC17DEC1300CE".to_owned()
        })
    );
    // A monthly before the trade month, or past the horizon, is no reading.
    assert!(decode_ticker("ACC18NOV1300CE", day(2018, 12, 3)).is_err());
    assert!(decode_ticker("ACC25JAN1300CE", day(2018, 12, 3)).is_err());
}

#[test]
fn an_expiry_on_a_weekday_holiday_is_kept_as_the_vendor_stated_it() {
    // 2024-03-29 was Good Friday, a trading holiday; the name is the identity.
    assert_eq!(
        contract("NIFTY29MAR2422000CE", day(2024, 3, 27)),
        "2024-03-29-2200000-CE"
    );
    // A weekend is no day-form reading.
    assert!(decode_ticker("NIFTY06APR2422000CE", trade()).is_err());
}

#[test]
fn an_expiry_before_the_trade_day_or_past_the_horizon_is_refused() {
    assert!(decode_ticker("NIFTY28MAR2422000CE", trade()).is_err());
    assert!(
        decode_ticker("NIFTY01APR2422000CE", trade()).is_ok(),
        "the trade day itself"
    );
    let name = |d: Day| format!("X{:02}{}{:02}100CE", d.day(), mon(d), d.year() % 100);
    let on = (0..7)
        .map(|k| Day::from_days(trade().days_from_epoch() + k).unwrap())
        .find(|t| {
            let edge = Day::from_days(t.days_from_epoch() + EXPIRY_HORIZON_DAYS).unwrap();
            is_weekday(edge) && is_weekday(edge.succ().unwrap())
        })
        .unwrap();
    let edge = Day::from_days(on.days_from_epoch() + EXPIRY_HORIZON_DAYS).unwrap();
    assert!(
        decode_ticker(&name(edge), on).is_ok(),
        "{edge}: the last admitted day"
    );
    assert!(decode_ticker(&name(edge.succ().unwrap()), on).is_err());
}

#[test]
fn every_other_shape_is_refused_by_name() {
    for t in [
        "NIFTY04APR2422000XE".to_owned(),
        "CE".to_owned(),
        "NIFTY04APR24CE".to_owned(),
        format!("{}04APR2422000CE", "N".repeat(TICKER_CAP)),
    ] {
        assert_eq!(
            decode_ticker(&t, trade()),
            Err(NfoRefusal::TickerUnparsed { ticker: t.clone() }),
            "{t}"
        );
    }
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
    assert!(
        decode_ticker("N04APR2499999999CE", trade()).is_ok(),
        "ten paisa digits fit"
    );
}

/// A small seeded generator, so the fuzz is the same run every time.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
        from[usize::try_from(self.next() % from.len() as u64).unwrap()]
    }
}

#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "the six parts of a ticker, named by their letters"
)]
fn a_million_strings_decode_with_an_exact_round_trip_or_a_named_refusal() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let unders = [
        "NIFTY",
        "BANKNIFTY",
        "ACC",
        "TV18BRDCST",
        "M&M",
        "360ONE",
        "X1",
        "ADANIENT",
        "NAM-INDIA",
        "A",
    ];
    let days = ["01", "04", "06", "18", "24", "28", "30", "31", "00", "1"];
    let mons = [
        "JAN", "FEB", "MAR", "APR", "JUN", "AUG", "SEP", "DEC", "XYZ", "Dec",
    ];
    let years = ["18", "19", "20", "24", "26", "30", "99", "00", "1"];
    let strikes = [
        "5", "24", "195", "2.5", "77.5", "1012.5", "10500", "22000", "100.00", "0", "1.234", "",
    ];
    let sides = ["CE", "PE", "XE", ""];
    let trades = [
        day(2018, 9, 3),
        day(2018, 12, 3),
        day(2019, 1, 31),
        day(2019, 2, 1),
        day(2024, 4, 1),
        day(2026, 8, 3),
    ];
    let alphabet = b"0123456789ACDEFJNOPRSUY.-&";
    let (mut decoded, mut refused) = (0_u32, 0_u32);
    for i in 0..1_000_000_u32 {
        let t = if i % 2 == 0 {
            let monthly = rng.next().is_multiple_of(3);
            let (u, d, m, y, s, side) = (
                rng.pick(&unders),
                rng.pick(&days),
                rng.pick(&mons),
                rng.pick(&years),
                rng.pick(&strikes),
                rng.pick(&sides),
            );
            if monthly {
                format!("{u}{y}{m}{s}{side}")
            } else {
                format!("{u}{d}{m}{y}{s}{side}")
            }
        } else {
            let len = 1 + rng.next() % 30;
            (0..len)
                .map(|_| char::from(alphabet[usize::try_from(rng.next() % 26).unwrap()]))
                .collect()
        };
        let on = trades[usize::try_from(rng.next() % 6).unwrap()];
        match decode_ticker(&t, on) {
            Ok(got) => {
                assert_eq!(got.ticker_text(), t, "round trip on {on}");
                decoded += 1;
            }
            Err(why) => {
                assert!(!why.to_string().is_empty());
                refused += 1;
            }
        }
    }
    assert_eq!(decoded + refused, 1_000_000);
    assert!(
        decoded > 10_000,
        "the sweep decodes a real share: {decoded}"
    );
}

#[test]
fn the_monthly_rule_walks_back_over_holidays_and_refuses_the_unmeasured() {
    assert_eq!(monthly_expiry(2018, 12), Some(day(2018, 12, 27)));
    assert_eq!(
        monthly_expiry(2018, 3),
        Some(day(2018, 3, 28)),
        "Thu 29 Mar was a holiday"
    );
    // Every month of the name census's monthly expiry table
    // (state/name-census/monthly-expiry-table.txt: last Thursday, previous
    // trading day on a holiday, equal to the vendor's last trade and dated
    // twins).
    for (y, m, d) in [
        (2018, 9, 27),
        (2018, 10, 25),
        (2018, 11, 29),
        (2018, 12, 27),
        (2019, 1, 31),
        (2019, 2, 28),
        (2019, 3, 28),
        (2019, 6, 27),
        (2019, 9, 26),
        (2019, 12, 26),
        (2020, 6, 25),
        (2020, 12, 31),
        (2021, 6, 24),
        (2021, 12, 30),
        (2022, 12, 29),
    ] {
        assert_eq!(monthly_expiry(y, m), Some(day(y, m, d)), "{y}-{m}");
    }
    assert_eq!(monthly_expiry(2017, 12), None);
    assert_eq!(monthly_expiry(2018, 13), None);
    assert_eq!(DAY_FORM_FROM_DAYS, day(2019, 2, 1).days_from_epoch());
    let ticker = decode_ticker("ACC18DEC1300PE", day(2018, 12, 3)).unwrap();
    assert_eq!(
        (ticker.expiry, ticker.side, ticker.strike),
        (day(2018, 12, 27), OptionSide::Put, 130_000)
    );
}

#[test]
fn no_split_bounds_slip_past_the_body() {
    let mut out = Vec::new();
    readings_at(b"AB", 3, trade(), &mut out, "ABCE");
    assert!(out.is_empty());
}

#[test]
fn weekday_and_month_helpers_hold_their_edges() {
    assert!(!is_weekday(day(2024, 4, 6)));
    assert!(!is_weekday(day(2024, 4, 7)));
    assert!(is_weekday(day(2024, 4, 8)));
    assert!(is_weekday(day(2024, 4, 5)));
    for (at, name) in MONTHS.iter().enumerate() {
        assert_eq!(
            month_of(name.as_bytes()),
            Some(u8::try_from(at + 1).unwrap()),
            "{name}"
        );
    }
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
        NfoRefusal::MonthlyExpiryUnmeasured { ticker: "T".into() },
        NfoRefusal::ExpiryPassed { ticker: "T".into() },
        NfoRefusal::ContractNamedTwice { ticker: "T".into() },
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
    assert!(texts.iter().any(|t| t.contains("line 7")));
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
    let bytes = csv(&rows(&[
        (33_301, "9.9", 0, 260),
        (33_300, "10.05", 75, 300),
    ]));
    let got = decode(&bytes, STEM, trade()).unwrap();
    assert_eq!(
        got.rows,
        vec![
            NfoRow {
                line: 2,
                sod: 33_301,
                ltp: 990,
                ltq: 0,
                oi: 260
            },
            NfoRow {
                line: 3,
                sod: 33_300,
                ltp: 1_005,
                ltq: 75,
                oi: 300
            },
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
    assert!(
        decode(header_only.as_bytes(), STEM, trade())
            .unwrap()
            .rows
            .is_empty()
    );
}

#[test]
fn every_row_fault_is_refused_by_name_at_its_line() {
    let good = rows(&[(33_300, "10", 1, 5)])[0].clone();
    let check = |line: String, want: NfoRefusal| {
        let bytes = csv(&[rows(&[(33_299, "10", 1, 5)])[0].clone(), line.clone()]);
        assert_eq!(decode(&bytes, STEM, trade()), Err(want), "{line}");
    };
    check(
        good.replacen(STEM, "OTHER.NFO", 1),
        NfoRefusal::TickerMismatch { line: 3 },
    );
    check(
        good.replacen("/04/", "/05/", 1),
        NfoRefusal::DateMismatch { line: 3 },
    );
    check(
        good.replacen("09:15:00", "9:15:00", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen("09:15:00", "24:00:00", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen("09:15:00", "09:60:00", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen("09:15:00", "09:15:60", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.rsplit_once(',').unwrap().0.to_owned(),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(format!("{good},0"), NfoRefusal::MalformedRow { line: 3 });
    check(
        good.replacen(",10,0,0,0,0,", ",10,x,0,0,0,", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",10,0,0,0,0,", ",10,x.5,0,0,0,", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",10,0,0,0,0,", ",10,0,,0,0,", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",10,0,0,0,0,", ",10,0,x,0,0,", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",10,0,0,0,0,", ",10,0,0,x,0,", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",10,0,0,0,0,", ",10,0,0,0,x,", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",0,0,1,5", ",0,0,x,5", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",0,0,1,5", ",0,0,99999999999999999999,5", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",1,5", ",1,x", 1),
        NfoRefusal::MalformedRow { line: 3 },
    );
    check(
        good.replacen(",1,5", ",1,9223372036854775808", 1),
        NfoRefusal::OpenInterestRefused { line: 3 },
    );
    check(
        good.replacen(",10,", ",10.001,", 1),
        NfoRefusal::PriceRefused { line: 3 },
    );
    check(
        good.replacen(",10,", ",-10,", 1),
        NfoRefusal::PriceRefused { line: 3 },
    );
    check(
        good.replacen(",10,", ",0,", 1),
        NfoRefusal::PriceRefused { line: 3 },
    );
    check(
        good.replacen(",10,", ",0.00,", 1),
        NfoRefusal::PriceRefused { line: 3 },
    );
    let penny = csv(&[good.replacen(",10,", ",0.01,", 1)]);
    assert_eq!(
        decode(&penny, STEM, trade()).unwrap().rows[0].ltp,
        1,
        "one paisa traded is a price"
    );
    check(
        good.replacen(",10,", ",92233720368547758.08,", 1),
        NfoRefusal::PriceRefused { line: 3 },
    );
    let mut bad = csv(std::slice::from_ref(&good));
    bad.insert(bad.len() - 3, 0xFF);
    assert_eq!(
        decode(&bad, STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 2 })
    );
}

#[test]
fn a_zero_price_is_admitted_only_on_a_row_with_no_trade() {
    let bytes = csv(&rows(&[(33_300, "0", 0, 9)]));
    assert_eq!(decode(&bytes, STEM, trade()).unwrap().rows[0].ltp, 0);
    let at_max = csv(&rows(&[(
        33_300,
        "92233720368547758.07",
        1,
        9_223_372_036_854_775_807,
    )]));
    let got = decode(&at_max, STEM, trade()).unwrap();
    assert_eq!((got.rows[0].ltp, got.rows[0].oi), (i64::MAX, i64::MAX));
}

#[test]
fn the_header_and_the_row_bound_are_refused_by_name() {
    assert_eq!(decode(b"", STEM, trade()), Err(NfoRefusal::HeaderUnknown));
    assert_eq!(
        decode(b"Ticker,Date\r\n", STEM, trade()),
        Err(NfoRefusal::HeaderUnknown)
    );
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
    assert_eq!(
        entry_ticker(&folder, &entry(d, "NIFTY04APR2422000CE")),
        Some("NIFTY04APR2422000CE")
    );
    assert_eq!(
        entry_ticker(&folder, &format!("{folder}/Options/A01APR241CE.NFO.CSV")),
        Some("A01APR241CE")
    );
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
    let mut listing: NfoDay<u8> = NfoDay::new(d, 2);
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
    let mut listing: NfoDay<u8> = NfoDay::new(d, 2);
    listing.push(&entry(d, "A01APR241CE"), 3, 4, 0);
    listing.push(
        &format!("{}\\Options\\A01APR241CE.NFO.CSV", day_folder_name(d)),
        3,
        4,
        1,
    );
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
    let (e1, e2) = (
        entry(d, "NIFTY04APR2422000CE"),
        entry(d, "NIFTY04APR2422000PE"),
    );
    let root = scratch("nfo-both");
    let store = NfoTickStore::new(&root.join("ts"));
    let path = store.day_path(d);
    assert!(
        path.ends_with("options/2024/APR_2024/GFDLNFO_TICK_01042024.bts"),
        "{path:?}"
    );
    put(
        &root.join("ts"),
        "options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&[(&e1, &one), (&e2, &two)]),
    );
    let day_zip = zip(&[(&e1, &one, Method::Deflated), (&e2, &two, Method::Stored)]);
    let year = zip(&[
        ("APR_2024/", b"", Method::Stored),
        (
            "APR_2024/GFDLNFO_TICK_01042024.zip",
            &day_zip,
            Method::Stored,
        ),
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
    assert!(
        zips.day(day(2025, 4, 2)).unwrap().is_none(),
        "no year zip at all"
    );
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
    put(
        &root,
        "a/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts_stating(&[(&e1, &one, 7)]),
    );
    let store = NfoTickStore::new(&root.join("a"));
    let listing = store.day(d).unwrap().unwrap();
    assert!(matches!(
        read_file(&store, &listing, &listing.entries()[0]),
        Err(NfoRefusal::Source(CmRefusal::SourceCrcMismatch { .. }))
    ));
    // A corrupt block.
    let mut broken = bts(&[(&e1, &one)]);
    broken[9] ^= 0xFF;
    put(
        &root,
        "b/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &broken,
    );
    let store = NfoTickStore::new(&root.join("b"));
    let listing = store.day(d).unwrap().unwrap();
    assert!(matches!(
        read_file(&store, &listing, &listing.entries()[0]),
        Err(NfoRefusal::Source(CmRefusal::TickStoreMalformed { .. }))
    ));
    // Not a day file at all.
    put(
        &root,
        "c/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        b"nope",
    );
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
    let mut listing: NfoDay<crate::gdfl_nfo::StoreLocator> = NfoDay::new(d, 2);
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
        listing_in(&Arc::new(year), len, d)
    };
    // Two day zips for one day.
    assert_eq!(
        open(zip(&[
            (name, &day_zip, Method::Stored),
            (name, &day_zip, Method::Stored)
        ]))
        .err(),
        Some(CmRefusal::ArchiveDuplicateDay { day: d })
    );
    // A day zip that is itself compressed in the year zip.
    assert!(matches!(
        open(zip(&[(name, &day_zip, Method::Deflated)])).err(),
        Some(CmRefusal::ArchiveMemberCompressed { .. })
    ));
    // Not a zip.
    assert!(matches!(
        open(b"nope".to_vec()).err(),
        Some(CmRefusal::ArchiveMalformed { .. })
    ));
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

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
use crate::gdfl_fixtures::{Method, bts, bts_stating, csv, hms, mon, put, row, scratch, zip};

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

/// DB-G51 (D-3331): a stated expiry on a closed day is no longer kept as
/// stated. Before the fix `NIFTY29MAR2422000CE` (Good Friday 2024, a holiday
/// no measured re-dating names) decoded to a contract keyed on the holiday;
/// it now settles on the previous open session, the charter's NSE rule, and
/// keeps the vendor's text as its alias.
#[test]
fn an_expiry_on_a_closed_day_settles_on_the_previous_session_and_keeps_its_text() {
    let got = decode_ticker("NIFTY29MAR2422000CE", day(2024, 3, 27)).unwrap();
    assert_eq!(got.contract.as_str(), "2024-03-28-2200000-CE");
    assert_eq!(
        (got.expiry, got.stated),
        (day(2024, 3, 28), day(2024, 3, 29))
    );
    assert_eq!(got.settlement.as_str(), "closed_day_rule");
    assert_eq!(got.ticker_text(), "NIFTY29MAR2422000CE");
    // Traded on the settled day itself: still the contract's last session.
    assert_eq!(
        contract("NIFTY29MAR2422000CE", day(2024, 3, 28)),
        "2024-03-28-2200000-CE"
    );
    // A weekend is no day-form reading.
    assert!(decode_ticker("NIFTY06APR2422000CE", trade()).is_err());
}

/// DB-018 / DB-G50 (D-3331): one contract, one series. Every text the vendor
/// gave a re-dated contract decodes to ONE contract segment, keyed on the day
/// it settled, and each name still renders back to its own text.
#[test]
fn every_text_of_a_re_dated_contract_is_one_contract_keyed_on_its_settled_day() {
    let one = |names: &[(&str, Day)], want: &str| {
        for &(name, on) in names {
            let got = decode_ticker(name, on).unwrap();
            assert_eq!(got.contract.as_str(), want, "{name} on {on}");
            assert_eq!(got.ticker_text(), name, "the vendor's text is kept");
        }
    };
    // 29 June 2023 became a holiday: `29JUN23` became `28JUN23` on the 28th.
    one(
        &[
            ("ACC29JUN231800CE", day(2023, 6, 27)),
            ("ACC28JUN231800CE", day(2023, 6, 28)),
        ],
        "2023-06-28-180000-CE",
    );
    // A NIFTY long-dated contract of 2019 keyed on the same settled day; the
    // census holds three such keyed on the closed 2023-06-29 today.
    let leaps = decode_ticker("NIFTY29JUN239500PE", day(2019, 2, 20)).unwrap();
    assert_eq!(leaps.contract.as_str(), "2023-06-28-950000-PE");
    assert_eq!(leaps.stated, day(2023, 6, 29));
    assert_eq!(leaps.expiry, day(2023, 6, 28));
    assert_eq!(leaps.settlement.as_str(), "aliased");
    // The weekday-regime change of 2025-01-01.
    one(
        &[
            ("BANKNIFTY29JAN2550000PE", day(2025, 1, 1)),
            ("BANKNIFTY30JAN2550000PE", day(2025, 1, 2)),
        ],
        "2025-01-30-5000000-PE",
    );
    // A chain of three texts: 26MAR26, 31MAR26, then 30MAR26.
    one(
        &[
            ("NIFTY26MAR2623000PE", day(2025, 7, 31)),
            ("NIFTY31MAR2623000PE", day(2025, 8, 1)),
            ("NIFTY31MAR2623000PE", day(2025, 12, 26)),
            ("NIFTY30MAR2623000PE", day(2025, 12, 29)),
        ],
        "2026-03-30-2300000-PE",
    );
    // MIDCPNIFTY `22JAN24`: never renamed, settled on the Saturday session.
    let mid = decode_ticker("MIDCPNIFTY22JAN2410000CE", day(2024, 1, 20)).unwrap();
    assert_eq!(mid.contract.as_str(), "2024-01-20-1000000-CE");
    assert_eq!(mid.ticker_text(), "MIDCPNIFTY22JAN2410000CE");
    // An open stated day is its own settlement, labelled so.
    let plain = decode_ticker("NIFTY04APR2422000CE", trade()).unwrap();
    assert_eq!(plain.stated, plain.expiry);
    assert_eq!(plain.settlement.as_str(), "as_stated");
    // A monthly name's derived expiry settles as itself.
    let monthly = decode_ticker("ACC18DEC1300CE", day(2018, 12, 3)).unwrap();
    assert_eq!(
        (monthly.stated, monthly.settlement.as_str()),
        (day(2018, 12, 27), "as_stated")
    );
}

/// A retired text the vendor ships again on or after its rename day would
/// name a different contract: refused by name.
#[test]
fn a_retired_text_traded_after_its_rename_is_refused() {
    for on in [day(2023, 6, 28), day(2023, 6, 29)] {
        assert!(
            matches!(
                decode_ticker("ACC29JUN231800CE", on),
                Err(NfoRefusal::ExpiryUnsettled {
                    why: crate::expiry_alias::SettleRefusal::TextAfterRename { .. },
                    ..
                })
            ),
            "{on}"
        );
    }
}

/// A source whose fetch is never expected: a refusal before any byte is read
/// comes back as itself, not as this source's error.
struct NoFetch;

impl NfoSource for NoFetch {
    type Locator = u8;

    fn day(&self, _: Day) -> Result<Option<NfoDay<u8>>, CmRefusal> {
        Ok(None)
    }

    fn fetch(&self, _: &ListedFile<u8>) -> Result<Vec<u8>, CmRefusal> {
        Err(CmRefusal::NotSwept)
    }
}

/// DB-027 / DB-G52 (D-3331): the three NIFTY `25JUN20` strikes of 2019-05-30
/// (19,12,500 to 19,13,000 with the day's ladder near 11,700) are refused by
/// name before a byte is read, so they are never stored and never listed.
/// Before the fix `read_file` decoded them as contracts.
#[test]
fn a_strike_far_off_the_days_ladder_is_refused_before_it_is_read() {
    let on = day(2019, 5, 30);
    let mut listing: NfoDay<u8> = NfoDay::new(on, 8);
    let real = [
        "NIFTY30MAY1911700CE",
        "NIFTY30MAY1911800PE",
        "NIFTY27JUN1911700CE",
        "NIFTY27JUN1912500CE",
        "NIFTY25JUN2010900CE",
    ];
    let impossible = [
        "NIFTY25JUN201912500CE",
        "NIFTY25JUN201912700CE",
        "NIFTY25JUN201913000CE",
    ];
    for (at, name) in real.iter().chain(&impossible).enumerate() {
        listing.push(&entry(on, name), 1, 1, u8::try_from(at).unwrap());
    }
    // The lower median of the eight decodable strikes is 11,800.
    assert_eq!(listing.strike_median("NIFTY"), Some(1_180_000));
    assert_eq!(listing.strike_median("BANKNIFTY"), None);
    for (at, name) in impossible.iter().enumerate() {
        let file = &listing.entries()[real.len() + at];
        let got = read_file(&NoFetch, &listing, file).unwrap_err();
        let NfoRefusal::StrikeImplausible {
            ticker,
            strike,
            median,
        } = &got
        else {
            panic!("{name}: {got:?}")
        };
        assert_eq!((ticker.as_str(), *median), (*name, 1_180_000));
        assert!(*strike > 190_000_000, "{strike}");
        assert!(got.to_string().contains("strike implausible"), "{got}");
    }
    // A real strike passes the check and reaches the source.
    assert_eq!(
        read_file(&NoFetch, &listing, &listing.entries()[3]),
        Err(NfoRefusal::Source(CmRefusal::NotSwept))
    );
    // The boundary: exactly ten times the median (D-3331's k) is admitted,
    // one paisa more is not.
    assert_eq!(STRIKE_PLAUSIBLE_MULTIPLE, 10, "D-3331 decision 2");
    let mut edge = decode_ticker("NIFTY27JUN1911700CE", on).unwrap();
    edge.strike = 1_180_000 * STRIKE_PLAUSIBLE_MULTIPLE;
    assert_eq!(listing.check_strike(&edge, "X"), Ok(()));
    edge.strike += 1;
    assert!(matches!(
        listing.check_strike(&edge, "X"),
        Err(NfoRefusal::StrikeImplausible { .. })
    ));
    // A pushed entry drops the median: it is the whole day's, recomputed.
    listing.push(&entry(on, "NIFTY27JUN1990000CE"), 1, 1, 9);
    listing.push(&entry(on, "NIFTY27JUN1990500CE"), 1, 1, 10);
    assert_eq!(listing.strike_median("NIFTY"), Some(1_250_000));
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

/// D-4816 (bar definition 5): every row fault refuses ITS ROW by name, with
/// its line and its fault, and the rest of the file is read; alone in its
/// file (no row kept) the same row is the file's refusal, which names it as
/// definition 4 did. A Ticker or Date of another file still refuses the
/// whole file. Faults are the capital-market reader's `RowFault`s.
#[test]
fn every_row_fault_is_refused_by_name_at_its_line() {
    use RowFault::{Malformed, OffGrid, ZeroLtp};
    let good = rows(&[(33_300, "10", 1, 5)])[0].clone();
    let before = rows(&[(33_299, "10", 1, 5)])[0].clone();
    let whole = |line: String, want: NfoRefusal| {
        let bytes = csv(&[before.clone(), line.clone()]);
        assert_eq!(decode(&bytes, STEM, trade()), Err(want), "{line}");
    };
    whole(
        good.replacen(STEM, "OTHER.NFO", 1),
        NfoRefusal::TickerMismatch { line: 3 },
    );
    whole(
        good.replacen("/04/", "/05/", 1),
        NfoRefusal::DateMismatch { line: 3 },
    );
    let malformed = |line| NfoRefusal::MalformedRow { line };
    let price = |line| NfoRefusal::PriceRefused { line };
    let oi = |line| NfoRefusal::OpenInterestRefused { line };
    let check = |line: String, fault: RowFault, want: &dyn Fn(u32) -> NfoRefusal| {
        let bytes = csv(&[before.clone(), line.clone()]);
        let got = decode(&bytes, STEM, trade()).unwrap_or_else(|why| panic!("{line}: {why}"));
        let kept: Vec<u32> = got.rows.iter().map(|row| row.line).collect();
        assert_eq!(kept, [2], "{line}");
        let named: Vec<(u32, RowFault)> = got
            .refused
            .named
            .iter()
            .map(|row| (row.line, row.fault))
            .collect();
        assert_eq!(named, [(3, fault)], "{line}");
        assert_eq!(got.refused.total(), 1, "{line}");
        let alone = csv(std::slice::from_ref(&line));
        assert_eq!(decode(&alone, STEM, trade()), Err(want(2)), "{line} alone");
    };
    for (line, fault, want) in [
        (
            good.replacen("09:15:00", "9:15:00", 1),
            Malformed,
            &malformed as &dyn Fn(u32) -> NfoRefusal,
        ),
        (
            good.replacen("09:15:00", "24:00:00", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen("09:15:00", "09:60:00", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen("09:15:00", "09:15:60", 1),
            Malformed,
            &malformed,
        ),
        (
            good.rsplit_once(',').unwrap().0.to_owned(),
            Malformed,
            &malformed,
        ),
        (format!("{good},0"), Malformed, &malformed),
        (
            good.replacen(",10,0,0,0,0,", ",10,x,0,0,0,", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen(",10,0,0,0,0,", ",10,x.5,0,0,0,", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen(",10,0,0,0,0,", ",10,0,,0,0,", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen(",10,0,0,0,0,", ",10,0,x,0,0,", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen(",10,0,0,0,0,", ",10,0,0,x,0,", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen(",10,0,0,0,0,", ",10,0,0,0,x,", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen(",0,0,1,5", ",0,0,x,5", 1),
            Malformed,
            &malformed,
        ),
        (
            good.replacen(",0,0,1,5", ",0,0,99999999999999999999,5", 1),
            Malformed,
            &malformed,
        ),
        (good.replacen(",1,5", ",1,x", 1), Malformed, &malformed),
        (
            good.replacen(",1,5", ",1,9223372036854775808", 1),
            Malformed,
            &oi,
        ),
        (good.replacen(",10,", ",10.001,", 1), OffGrid, &price),
        (good.replacen(",10,", ",-10,", 1), Malformed, &price),
        (good.replacen(",10,", ",0,", 1), ZeroLtp, &price),
        (good.replacen(",10,", ",0.00,", 1), ZeroLtp, &price),
        (
            good.replacen(",10,", ",92233720368547758.08,", 1),
            OffGrid,
            &price,
        ),
    ] {
        check(line, fault, want);
    }
    let penny = csv(&[good.replacen(",10,", ",0.01,", 1)]);
    assert_eq!(
        decode(&penny, STEM, trade()).unwrap().rows[0].ltp,
        1,
        "one paisa traded is a price"
    );
    let mut bad = csv(std::slice::from_ref(&good));
    bad.insert(bad.len() - 3, 0xFF);
    assert_eq!(
        decode(&bad, STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 2 })
    );
    let mut after = csv(&[before.clone(), good.clone()]);
    after.insert(after.len() - 3, 0xFF);
    let got = decode(&after, STEM, trade()).unwrap();
    assert_eq!(
        (got.rows.len(), got.refused.malformed),
        (1, 1),
        "not UTF-8 refuses its row (F10)"
    );
}

/// **NO OPTIONS ROW ALLOCATES A FIELD LIST** (`cloud/attack-o1-20261016`).
///
/// The capital-market reader's `parse_row` splits into a fixed `[&str; 10]`
/// and `csv::decode_rows` was made to (D-1203); this reader still collected a
/// `Vec<&str>` per row, on a tree of 27.6 million rows a day. The allocation
/// has no observable output, so its absence is asserted on the source of
/// `decode_capped`, as `csv::the_decode_loop_collects_no_field_vector` does.
/// The needles are assembled at run time so this test cannot match itself.
#[test]
fn the_options_row_loop_collects_no_field_vector() {
    let source = include_str!("gdfl_nfo.rs");
    let start = source
        .find(&format!("{}{}", "fn decode_", "capped("))
        .expect("decode_capped exists");
    let body = &source[start..];
    let body = &body[..body.find("\n}\n").expect("decode_capped ends")];
    for needle in [
        format!("{}{}", "Vec<&", "str>"),
        format!("{}{}", ".coll", "ect()"),
        format!("{}{}", "Vec::", "new()"),
    ] {
        assert!(
            !body.contains(&needle),
            "decode_capped holds {needle:?}: a per-row allocation or an unreserved \
             row vector is back"
        );
    }
    assert!(body.contains(&format!("{}{}", "with_capacity(", "bound)")));
}

/// An eleventh field is refused at the line it is on, however many follow:
/// a row of a thousand commas is the same malformed row as one of eleven
/// fields, refused by name while the rows before it are read (D-4816), and
/// the file's own `MalformedRow` when it is the only row.
#[test]
fn a_row_of_many_fields_is_malformed_at_its_line() {
    let good = rows(&[(33_300, "10", 1, 5)])[0].clone();
    let wide = format!("{good}{}", ",0".repeat(1_000));
    let bytes = csv(&[rows(&[(33_299, "10", 1, 5)])[0].clone(), wide.clone()]);
    let got = decode(&bytes, STEM, trade()).unwrap();
    assert_eq!(got.rows.len(), 1);
    assert_eq!(
        got.refused
            .named
            .iter()
            .map(|row| (row.line, row.fault))
            .collect::<Vec<_>>(),
        [(3, RowFault::Malformed)]
    );
    assert_eq!(
        decode(&csv(&[wide]), STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 2 })
    );
    let bytes = csv(&rows(&[(33_299, "10", 1, 5), (33_300, "10", 1, 5)]));
    assert_eq!(
        decode(&bytes, STEM, trade()).map(|file| file.rows.len()),
        Ok(2)
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
    // The verified member travels with the decode (IA-3, design §5.1 seam
    // 4): the bytes the rows came from, byte for byte, from either source.
    assert_eq!(&*got.bytes, one.as_slice());
    let from_zip = read_file(&zips, &from_zips, &from_zips.entries()[0]).unwrap();
    assert_eq!(from_zip.bytes, got.bytes);
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

#[test]
fn the_edges_mutation_found_hold() {
    // A monthly traded ON its derived expiry day is read (2018-12-27).
    assert_eq!(
        contract("ACC18DEC1300CE", day(2018, 12, 27)),
        "2018-12-27-130000-CE"
    );
    // A ticker of exactly the cap is read; one byte more is not.
    let at_cap = format!("{}04APR2422000CE", "N".repeat(TICKER_CAP - 14));
    let past = format!("N{at_cap}");
    assert!(matches!(
        decode_ticker(&past, trade()),
        Err(NfoRefusal::TickerUnparsed { .. })
    ));
    assert_eq!(at_cap.len(), TICKER_CAP);
    assert!(!matches!(
        decode_ticker(&at_cap, trade()),
        Err(NfoRefusal::TickerUnparsed { .. })
    ));
    // Two entries that are not option files are counted as two skips.
    let mut listing: NfoDay<u8> = NfoDay::new(trade(), 2);
    listing.push("x", 1, 1, 0);
    listing.push("y", 1, 1, 1);
    assert_eq!(listing.skipped(), 2);
}

/// D-3331 against the operator's REAL options zips, read-only: every name
/// of every vendor day in `BRUTEX_WP12B_FROM..=BRUTEX_WP12B_TO` (default the
/// whole archive) is decoded and checked against its day's ladder, and no
/// file's bytes are read. No vendor row reaches the repository: only counts,
/// names and derived ratios are printed. Over the whole archive the
/// settlement refuses no real name, the closed-day rule answers none (every
/// closed stated day the archive holds is a row), no two names of a day
/// settle on one contract, the table re-keys exactly its own count of
/// distinct names, and the strike check refuses exactly the three NIFTY
/// `25JUN20` names of 2019-05-30. It prints the day-ladder ratios
/// `STRIKE_PLAUSIBLE_MULTIPLE` is chosen against: each underlying-day's
/// highest strike over its lower median, and its lower median over its
/// lowest strike.
#[test]
#[ignore = "requires BRUTEX_GDFL_NFO_ARCHIVES (the vendor's yearly options zips); read-only"]
fn the_real_archive_settles_every_name_and_refuses_only_the_impossible_strikes() {
    let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
    let bound = |var: &str, fallback: Day| {
        std::env::var(var).ok().map_or(fallback, |text| {
            let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
            day(
                parts[0],
                u8::try_from(parts[1]).unwrap(),
                u8::try_from(parts[2]).unwrap(),
            )
        })
    };
    let (from, to) = (
        bound("BRUTEX_WP12B_FROM", day(2018, 9, 3)),
        bound("BRUTEX_WP12B_TO", day(2026, 9, 24)),
    );
    let zips = NfoZips::new(Path::new(&dir));
    let (mut days, mut names, mut rekeyed) = (0_u64, 0_u64, 0_u64);
    let mut rekeyed_names: HashSet<String> = HashSet::new();
    let mut unsettled: Vec<String> = Vec::new();
    let mut implausible: Vec<String> = Vec::new();
    // Two names of one vendor day that settle on one contract: the split
    // D-3331 closes would have become a same-day collision instead.
    let mut collisions: Vec<String> = Vec::new();
    let mut by_rule: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    // (ratio x 100, day, underlying) of the highest strike over the lower
    // median, and of the lower median over the lowest strike.
    let mut high: Vec<(i64, Day, String)> = Vec::new();
    let mut low: Vec<(i64, Day, String)> = Vec::new();
    let mut on = from;
    while on <= to {
        if let Some(listing) = zips.day(on).unwrap() {
            days += 1;
            let mut seen: HashMap<(String, String), String> = HashMap::new();
            let mut ladders: HashMap<String, (i64, i64)> = HashMap::new();
            for file in listing.entries() {
                let Some(name) = listing.ticker_of(file) else {
                    continue;
                };
                names += 1;
                match decode_ticker(name, on) {
                    Ok(ticker) => {
                        if ticker.expiry != ticker.stated {
                            rekeyed += 1;
                            rekeyed_names.insert(name.to_owned());
                        }
                        if ticker.settlement == crate::expiry_alias::Settlement::ClosedDayRule {
                            *by_rule
                                .entry(format!("{} {}", ticker.underlying.as_str(), ticker.stated))
                                .or_default() += 1;
                        }
                        let key = (
                            ticker.underlying.as_str().to_owned(),
                            ticker.contract.as_str().to_owned(),
                        );
                        if let Some(first) = seen.insert(key, name.to_owned()) {
                            collisions.push(format!("{on} {first} {name}"));
                        }
                        if listing.check_strike(&ticker, name).is_err() {
                            implausible.push(format!("{on} {name}"));
                        } else {
                            let span = ladders
                                .entry(ticker.underlying.as_str().to_owned())
                                .or_insert((ticker.strike, ticker.strike));
                            span.0 = span.0.min(ticker.strike);
                            span.1 = span.1.max(ticker.strike);
                        }
                    }
                    Err(NfoRefusal::ExpiryUnsettled { why, .. }) => {
                        unsettled.push(format!("{on} {name}: {why}"));
                    }
                    Err(_) => {}
                }
            }
            for (under, (lo, hi)) in ladders {
                let median = listing.strike_median(&under).unwrap();
                high.push((hi * 100 / median, on, under.clone()));
                if lo > 0 {
                    low.push((median * 100 / lo, on, under));
                }
            }
            high.sort_unstable_by(|a, b| b.cmp(a));
            high.truncate(3);
            low.sort_unstable_by(|a, b| b.cmp(a));
            low.truncate(3);
        }
        on = on.succ().unwrap();
    }
    eprintln!(
        "real archive {from}..={to}: {days} vendor days, {names} names, {rekeyed} re-keyed file-days ({} distinct names) to a settled day, {} settled by the closed-day rule {by_rule:?}, {} unsettled, {} same-day collisions, {} implausible: {implausible:?} {:?} {:?}; admitted ladders, highest strike / lower median x100: {high:?}; lower median / lowest strike x100: {low:?}",
        rekeyed_names.len(),
        by_rule.values().sum::<u64>(),
        unsettled.len(),
        collisions.len(),
        implausible.len(),
        &unsettled[..unsettled.len().min(20)],
        &collisions[..collisions.len().min(20)]
    );
    assert!(days > 0, "no vendor day in range");
    assert!(unsettled.is_empty(), "{unsettled:?}");
    assert!(collisions.is_empty(), "{collisions:?}");
    // Every closed stated day of the archive is a row: the rule answers
    // only days the archive has not shown.
    assert!(by_rule.is_empty(), "{by_rule:?}");
    // No admitted ladder comes within reach of the multiple.
    assert!(
        high.iter()
            .all(|(ratio, ..)| *ratio < STRIKE_PLAUSIBLE_MULTIPLE * 100),
        "{high:?}"
    );
    let impossible = [
        "2019-05-30 NIFTY25JUN201912500CE",
        "2019-05-30 NIFTY25JUN201912700CE",
        "2019-05-30 NIFTY25JUN201913000CE",
    ];
    if (from..=to).contains(&day(2019, 5, 30)) {
        implausible.sort();
        assert_eq!(implausible, impossible);
    } else {
        assert!(implausible.is_empty(), "{implausible:?}");
    }
    if (from, to) == (day(2018, 9, 3), day(2026, 9, 24)) {
        assert_eq!(
            u32::try_from(rekeyed_names.len()).unwrap(),
            crate::expiry_alias::ALIASES
                .iter()
                .map(|row| row.contracts)
                .sum::<u32>(),
            "the table's contract counts are the archive's re-keyed names"
        );
    }
}

/// D-3331 against the operator's REAL zips and REAL store, read-only: every
/// option name of each underlying in `BRUTEX_WP12B_UNDERLYINGS` first traded
/// in `BRUTEX_WP12B_FROM..=BRUTEX_WP12B_TO` is decoded, and the contract set
/// is compared with the contract directories `BRUTEX_STORE_FNO` (the store's
/// `bars/gdfl/NSE/FNO`) holds for it. Names and directory names only: no
/// file's bytes are read and nothing is written. Prints what the decoder no
/// longer files (the refused names, the closed stated days) and what it files
/// instead (their settled days); every other contract must agree.
#[test]
#[ignore = "requires BRUTEX_GDFL_NFO_ARCHIVES and BRUTEX_STORE_FNO; read-only"]
fn the_real_names_decode_to_the_stored_contracts_less_the_refused_and_the_resettled() {
    let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
    let store =
        std::path::PathBuf::from(std::env::var_os("BRUTEX_STORE_FNO").expect("BRUTEX_STORE_FNO"));
    let unders = std::env::var("BRUTEX_WP12B_UNDERLYINGS").unwrap_or_else(|_| "NIFTY".to_owned());
    let bound = |var: &str, fallback: Day| {
        std::env::var(var).ok().map_or(fallback, |text| {
            let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
            day(
                parts[0],
                u8::try_from(parts[1]).unwrap(),
                u8::try_from(parts[2]).unwrap(),
            )
        })
    };
    let (from, to) = (
        bound("BRUTEX_WP12B_FROM", day(2018, 9, 3)),
        bound("BRUTEX_WP12B_TO", day(2020, 9, 15)),
    );
    let wanted: HashSet<&str> = unders.split(',').collect();
    let zips = NfoZips::new(Path::new(&dir));
    let mut decoded: HashMap<String, std::collections::BTreeSet<String>> = HashMap::new();
    let mut refused: Vec<String> = Vec::new();
    let mut refused_contracts: HashSet<(String, String)> = HashSet::new();
    // (underlying, name) -> (the stated contract, the settled one, how).
    let mut resettled: std::collections::BTreeMap<(String, String), (String, String, &str)> =
        std::collections::BTreeMap::new();
    let mut on = from;
    while on <= to {
        if let Some(listing) = zips.day(on).unwrap() {
            for file in listing.entries() {
                let Some(name) = listing.ticker_of(file) else {
                    continue;
                };
                match decode_ticker(name, on) {
                    Ok(ticker) if wanted.contains(ticker.underlying.as_str()) => {
                        if let Err(why) = listing.check_strike(&ticker, name) {
                            refused.push(format!("{on} {why}"));
                            refused_contracts.insert((
                                ticker.underlying.as_str().to_owned(),
                                ticker.contract.as_str().to_owned(),
                            ));
                            continue;
                        }
                        if ticker.expiry != ticker.stated {
                            let stated = crate::gdfl_lineage::ContractDay {
                                expiry: ticker.stated,
                                strike: ticker.strike,
                                side: ticker.side,
                                lot: crate::gdfl_lineage::LotEvidence::NoTrade,
                                first_oi: None,
                                close_oi: None,
                            }
                            .contract()
                            .unwrap();
                            resettled.insert(
                                (ticker.underlying.as_str().to_owned(), name.to_owned()),
                                (
                                    stated.as_str().to_owned(),
                                    ticker.contract.as_str().to_owned(),
                                    ticker.settlement.as_str(),
                                ),
                            );
                        }
                        decoded
                            .entry(ticker.underlying.as_str().to_owned())
                            .or_default()
                            .insert(ticker.contract.as_str().to_owned());
                    }
                    Ok(_) => {}
                    Err(why) => {
                        if wanted.iter().any(|u| name.starts_with(u)) {
                            refused.push(format!("{on} {name}: {why}"));
                        }
                    }
                }
            }
        }
        on = on.succ().unwrap();
    }
    for under in &wanted {
        let held: std::collections::BTreeSet<String> = std::fs::read_dir(store.join(under))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        let mine = decoded.remove(*under).unwrap_or_default();
        let only_held: Vec<&String> = held.difference(&mine).collect();
        let only_mine: Vec<&String> = mine.difference(&held).collect();
        eprintln!(
            "{under} {from}..={to}: {} decoded contracts, {} stored; only stored {}: {only_held:?}; only decoded {}: {only_mine:?}",
            mine.len(),
            held.len(),
            only_held.len(),
            only_mine.len()
        );
        // What this decoder changed, checked both ways: every name it now
        // re-keys had its STATED contract stored and files its SETTLED one,
        // which the store lacks; every name it refuses as implausible was
        // stored and is not filed. Anything else that differs (a contract
        // first traded after `to`, a file an older reader refused) is
        // printed, and is not this decoder's doing.
        let closed = |seg: &str| {
            let day = crate::gdfl_lineage::parse_day(&seg[..10]).unwrap();
            crate::calendar::sourced_kind_of(i64::from(day.days_from_epoch()))
                == crate::calendar::DayKind::Closed
        };
        let mut changed = 0_usize;
        for ((_, name), (stated, settled, _)) in resettled.iter().filter(|((u, _), _)| u == under) {
            assert!(
                held.contains(stated),
                "{under}: {name} was stored as {stated}"
            );
            assert!(
                mine.contains(settled) && !held.contains(settled),
                "{under}: {settled}"
            );
            changed += 1;
        }
        for (u, seg) in refused_contracts.iter().filter(|(u, _)| u == under) {
            assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
            changed += 1;
        }
        let other_held: Vec<&&String> = only_held
            .iter()
            .filter(|seg| {
                !closed(seg) && !refused_contracts.contains(&((*under).to_owned(), (**seg).clone()))
            })
            .collect();
        let other_mine: Vec<&&String> = only_mine
            .iter()
            .filter(|seg| !resettled.values().any(|(_, settled, _)| settled == **seg))
            .collect();
        eprintln!(
            "{under}: {changed} names changed by this decoder; other differences, not this decoder's: stored only {other_held:?}, decoded only {other_mine:?}"
        );
    }
    eprintln!("refused {}: {refused:?}", refused.len());
    eprintln!("resettled {}: {resettled:?}", resettled.len());
}

/// D-3331 / D-3343 against the operator's REAL options zips, read-only: the
/// corporate-action detector on the ex-dates the audit measured (DB-G71,
/// DB-G72, DB-G77), and on the next ordinary day as the control. Only counts
/// are printed; no vendor row reaches the repository.
#[test]
#[ignore = "requires BRUTEX_GDFL_NFO_ARCHIVES (the vendor's yearly options zips); read-only"]
fn the_real_ex_dates_are_detected_from_the_data_and_ordinary_days_are_not() {
    use crate::gdfl_lineage::{ContractDay, OiCarry, detect};
    let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
    let zips = NfoZips::new(Path::new(&dir));
    // The contract-days of `under` on `on`, with each file's rows kept.
    let day_of = |under: &str, on: Day| {
        let mut out: Vec<(ContractDay, Vec<(u32, i64)>)> = Vec::new();
        let Some(listing) = zips.day(on).unwrap() else {
            return out;
        };
        for file in listing.entries() {
            let Some(name) = listing.ticker_of(file) else {
                continue;
            };
            let Ok(ticker) = decode_ticker(name, on) else {
                continue;
            };
            if ticker.underlying.as_str() != under {
                continue;
            }
            let read = read_file(&zips, &listing, file).unwrap();
            let rows: Vec<(u32, u64, i64)> = read
                .file
                .rows
                .iter()
                .map(|r| (r.sod, r.ltq, r.oi))
                .collect();
            out.push((
                ContractDay::from_rows(&read.ticker, rows.iter().copied()),
                rows.iter().map(|&(sod, _, oi)| (sod, oi)).collect(),
            ));
        }
        out
    };
    // (underlying, the vendor day before, the ex-date, the next day, the lot
    // step the audit measured as after : before).
    let cases = [
        (
            "BRITANNIA",
            day(2018, 11, 28),
            day(2018, 11, 29),
            day(2018, 11, 30),
            (2, 1),
        ),
        (
            "NTPC",
            day(2019, 3, 18),
            day(2019, 3, 19),
            day(2019, 3, 20),
            (6, 5),
        ),
        (
            "HCLTECH",
            day(2019, 12, 4),
            day(2019, 12, 5),
            day(2019, 12, 6),
            (2, 1),
        ),
        (
            "BAJAJFINSV",
            day(2022, 9, 12),
            day(2022, 9, 13),
            day(2022, 9, 14),
            (10, 1),
        ),
        (
            "RELIANCE",
            day(2024, 10, 25),
            day(2024, 10, 28),
            day(2024, 10, 29),
            (2, 1),
        ),
        (
            "BAJFINANCE",
            day(2025, 6, 13),
            day(2025, 6, 16),
            day(2025, 6, 17),
            (10, 1),
        ),
    ];
    for (under, before, ex, next, (num, den)) in cases {
        let symbol = brutex_core::symbol::Symbol::new(under).unwrap();
        let (eve, post, later) = (
            day_of(under, before),
            day_of(under, ex),
            day_of(under, next),
        );
        // The pre-ex state: the latest contract-day of every key over the
        // hundred days before the ex-date (a contract lists at most three
        // months ahead), still unexpired on it.
        let mut latest: std::collections::BTreeMap<String, (ContractDay, Vec<(u32, i64)>)> =
            std::collections::BTreeMap::new();
        let mut on = Day::from_days(before.days_from_epoch() - 100).unwrap();
        while on <= before {
            for (c, rows) in day_of(under, on) {
                if c.expiry >= ex {
                    latest.insert(c.contract().unwrap().to_string(), (c, rows));
                }
            }
            on = on.succ().unwrap();
        }
        let pre: Vec<(ContractDay, Vec<(u32, i64)>)> = latest.into_values().collect();
        let strip = |days: &[(ContractDay, Vec<(u32, i64)>)]| -> Vec<ContractDay> {
            days.iter().map(|(c, _)| *c).collect()
        };
        let eve_only = detect(symbol, ex, &strip(&eve), &strip(&post)).expect(under);
        let event = detect(symbol, ex, &strip(&pre), &strip(&post)).expect(under);
        assert!(event.lineage.len() >= eve_only.lineage.len(), "{under}");
        let count = |carry: OiCarry| event.lineage.iter().filter(|l| l.oi == carry).count();
        let reused = event.lineage.iter().filter(|l| l.reused).count();
        // The stale ex-date rows of every pair whose OI carried unscaled.
        let mut stale_rows = 0_usize;
        let mut records: Vec<String> = Vec::new();
        for pair in event.lineage.iter().filter(|l| l.oi == OiCarry::Unadjusted) {
            let new = post
                .iter()
                .find(|(c, _)| c.contract() == Some(pair.new))
                .unwrap();
            let stale = pair.stale_oi(&new.1);
            assert!(
                stale.rows > 0 && stale.until_sod.is_some(),
                "{under}: {pair:?}"
            );
            stale_rows += stale.rows;
            records.push(pair.record(&new.1).line());
        }
        // The records parse back (D-3331's lines); the unix second is derived
        // from the vendor's stamps, the only part of a row a record carries.
        for line in &records {
            assert!(
                crate::gdfl_lineage::LineageRecord::parse(line).is_some(),
                "{line}"
            );
        }
        // The first post-ex OI over the pre-ex close, per mille, of every pair
        // whose carry neither scales nor equals: a derived ratio, not a row.
        let odd: Vec<i64> = event
            .lineage
            .iter()
            .filter(|l| l.oi == OiCarry::Unexplained)
            .map(|pair| {
                let old = pre
                    .iter()
                    .find(|(c, _)| c.contract() == Some(pair.old))
                    .unwrap();
                let new = post
                    .iter()
                    .find(|(c, _)| c.contract() == Some(pair.new))
                    .unwrap();
                new.0.first_oi.unwrap() * 1_000 / old.0.close_oi.unwrap()
            })
            .collect();
        eprintln!(
            "{under} {ex}: lot {} -> {}, {} pre-ex keys ({} traded the day before, {} paired from it), {} post-ex, {} paired, {} unpaired, {} stitched keys ({reused} of them renamed), OI scaled {} unadjusted {} ({stale_rows} stale rows) indistinct {} unexplained {} {odd:?} missing {}",
            event.lot_before,
            event.lot_after,
            pre.len(),
            eve.len(),
            eve_only.lineage.len(),
            post.len(),
            event.lineage.len(),
            event.unpaired.len(),
            event.stitched.len(),
            count(OiCarry::Scaled),
            count(OiCarry::Unadjusted),
            count(OiCarry::Indistinct),
            count(OiCarry::Unexplained),
            count(OiCarry::Missing),
        );
        assert_eq!(
            event.lot_after * den,
            event.lot_before * num,
            "{under}: the lot steps by {num}/{den}"
        );
        assert!(
            eve_only.lineage.len() * 2 > eve.len(),
            "{under}: most contracts of the day before pair"
        );
        // The next ordinary day is no adjustment.
        assert_eq!(
            detect(symbol, next, &strip(&post), &strip(&later)),
            None,
            "{under} {next}"
        );
        // Every retired key traded again on the ex-date or after it, to the
        // last retired expiry: the stitched series.
        let retired = event.retired();
        let last = pre.iter().map(|(c, _)| c.expiry).max().unwrap();
        let mut again: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut on = ex;
        while on <= last {
            // Names only: no file is read for this.
            if let Some(listing) = zips.day(on).unwrap() {
                for file in listing.entries() {
                    let decoded = listing.ticker_of(file).map(|t| decode_ticker(t, on));
                    if let Some(Ok(ticker)) = decoded
                        && ticker.underlying.as_str() == under
                        && retired.contains(&ticker.contract)
                    {
                        again.insert(ticker.contract.to_string());
                    }
                }
            }
            on = on.succ().unwrap();
        }
        eprintln!(
            "{under}: {} retired keys, {} traded again after the ex-date",
            retired.len(),
            again.len()
        );
        assert!(again.len() >= event.stitched.len(), "{under}");
        // The audit's own counts (DB-G71, DB-G72, DB-G77), reproduced.
        match under {
            "NTPC" => assert_eq!(again.len(), 35, "NTPC's stitched series"),
            "BRITANNIA" => assert_eq!(retired.len(), 56, "BRITANNIA's series ending early"),
            "BAJFINANCE" => assert_eq!(count(OiCarry::Unadjusted), 18, "stale ex-date OI"),
            _ => {}
        }
    }
}

/// D-3331's per-name costs on REAL days, read-only (names only, no file
/// read): `decode_ticker` with its settlement, the day's strike medians on
/// the first ask, and `check_strike` after it. Prints p50 and p99 of each,
/// timed per call with `Instant`; the ratio harness owns the gate cases.
#[test]
#[ignore = "requires BRUTEX_GDFL_NFO_ARCHIVES (the vendor's yearly options zips); read-only"]
fn the_decoders_costs_on_real_days_are_measured() {
    let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
    let zips = NfoZips::new(Path::new(&dir));
    let pct = |mut ns: Vec<u128>, q: usize| {
        ns.sort_unstable();
        ns[(ns.len() * q / 100).min(ns.len() - 1)]
    };
    for on in [day(2023, 6, 27), day(2026, 8, 3)] {
        let listing = zips.day(on).unwrap().expect("a vendor day");
        let names: Vec<&str> = listing
            .entries()
            .iter()
            .filter_map(|f| listing.ticker_of(f))
            .collect();
        let mut decode_ns = Vec::with_capacity(names.len());
        let mut decoded = Vec::with_capacity(names.len());
        for name in &names {
            let at = std::time::Instant::now();
            let got = decode_ticker(name, on);
            decode_ns.push(at.elapsed().as_nanos());
            decoded.extend(got.ok().map(|t| (t, *name)));
        }
        let at = std::time::Instant::now();
        let first = decoded
            .first()
            .map(|(t, _)| listing.strike_median(t.underlying.as_str()));
        let medians_ns = at.elapsed().as_nanos();
        let mut check_ns = Vec::with_capacity(decoded.len());
        for (ticker, name) in &decoded {
            let at = std::time::Instant::now();
            let _ = listing.check_strike(ticker, name);
            check_ns.push(at.elapsed().as_nanos());
        }
        eprintln!(
            "{on}: {} names, {} decoded; decode_ticker p50 {} ns p99 {} ns; strike medians once {} us; check_strike p50 {} ns p99 {} ns",
            names.len(),
            decoded.len(),
            pct(decode_ns.clone(), 50),
            pct(decode_ns, 99),
            medians_ns / 1_000,
            pct(check_ns.clone(), 50),
            pct(check_ns, 99),
        );
        assert!(first.flatten().is_some(), "{on}: a median");
        assert_eq!(decoded.len(), names.len(), "{on}: every real name decodes");
    }
}

/// WP-16 (design F6): A YEAR ZIP IS INDEXED ONCE AND EACH DAY IS ONE
/// LOOKUP. The first day asked opens the year zip and reads its central
/// directory; every later day of that year is found in the index and read
/// through the same open file, so a run never reopens or rewalks it — shown
/// here by the one index the first day built being the one the second day is
/// read through, by any clone. A zip moved away after that is refused for its
/// day (IA-3b item 4: a source whose path no longer names the file a run
/// listed is never read as if it did), while a new source finds no year zip.
/// Two records of one day zip in the index refuse that day by name; an absent
/// year is asked again, so a zip that appears later is read.
#[test]
fn a_year_zip_is_indexed_once_and_each_day_is_one_lookup() {
    let (d1, d2) = (day(2024, 4, 1), day(2024, 4, 2));
    let ticker = "NIFTY04APR2422000CE";
    let stem = format!("{ticker}.NFO");
    let one = file_bytes(ticker, &[(33_300, "10", 75, 300)]);
    let two = csv(&[row(&stem, d2, 33_301, "11", 50, 310)]);
    let day_zip = |d: Day, bytes: &[u8]| zip(&[(&entry(d, ticker), bytes, Method::Deflated)]);
    let (z1, z2) = (day_zip(d1, &one), day_zip(d2, &two));
    let year = zip(&[
        ("APR_2024/GFDLNFO_TICK_01042024.zip", &z1, Method::Stored),
        ("APR_2024/GFDLNFO_TICK_02042024.zip", &z2, Method::Stored),
        ("APR_2024/GFDLNFO_TICK_03042024.zip", &z2, Method::Stored),
        ("APR_2024/GFDLNFO_TICK_03042024.zip", &z2, Method::Stored),
    ]);
    let root = scratch("nfo-year-once");
    let at = put(&root.join("zips"), "2024.zip", &year);
    let zips = NfoZips::new(&root.join("zips"));
    let first = zips.day(d1).unwrap().unwrap();
    assert_eq!(
        read_file(&zips, &first, &first.entries()[0])
            .unwrap()
            .file
            .rows[0]
            .oi,
        300
    );

    // The second day comes from the same index, its file opened once: no
    // second index of the year is built, by any clone.
    let index = |source: &NfoZips| {
        let years = source.years.lock().unwrap();
        assert_eq!(years.zips.len(), 1, "one year indexed");
        Arc::clone(years.zips.get(&2024).unwrap())
    };
    let before = index(&zips);
    let shared = zips.clone();
    let second = shared.day(d2).unwrap().unwrap();
    let got = read_file(&shared, &second, &second.entries()[0]).unwrap();
    assert_eq!(
        got.file.rows[0].oi, 310,
        "read through the year zip opened once"
    );
    assert!(Arc::ptr_eq(&before, &index(&zips)), "never indexed again");
    // IA-3b (item 4, RA-323): a year zip moved away under a run is never
    // read as if it were still there. The open file would still read, but its
    // path no longer names it, so the next read is refused for its day by the
    // reader's own `archive_unavailable` (the loss probe asks the root),
    // and a new source finds no year zip at all.
    std::fs::rename(&at, root.join("moved.zip")).unwrap();
    assert!(
        NfoZips::new(&root.join("zips")).day(d2).unwrap().is_none(),
        "a new source finds no year zip"
    );
    assert_eq!(
        read_file(&shared, &second, &second.entries()[0]).err(),
        Some(NfoRefusal::Source(CmRefusal::ArchiveUnavailable {
            kind: std::io::ErrorKind::NotFound
        }))
    );
    assert_eq!(
        zips.day(day(2024, 4, 3)).err(),
        Some(CmRefusal::ArchiveDuplicateDay {
            day: day(2024, 4, 3)
        })
    );
    assert!(
        zips.day(day(2024, 4, 4)).unwrap().is_none(),
        "not in the index"
    );

    // An absent year is not remembered as absent.
    assert!(zips.day(day(2025, 4, 1)).unwrap().is_none());
    let z3 = day_zip(day(2025, 4, 1), &one);
    let later = zip(&[("APR_2025/GFDLNFO_TICK_01042025.zip", &z3, Method::Stored)]);
    put(&root.join("zips"), "2025.zip", &later);
    assert!(zips.day(day(2025, 4, 1)).unwrap().is_some());
    std::fs::remove_dir_all(root).unwrap();
}

/// FM-043 (DATA-007, D-4834's rule for options): an options day zip packed
/// into another year's zip (2025-12-31 inside 2026.zip) is noted when that
/// zip's directory is walked, and its day refuses `day_zip_misfiled` naming
/// where the bytes are and where its name files them, never `day_not_held`;
/// the other zip's own days still read. Asked before the other zip was ever
/// walked, with no zip of the day's own year, and with one that holds other
/// days only. A copy at its own place beside one elsewhere is two copies of
/// one day, `archive_duplicate_day` (D-4818's one rule).
#[test]
fn a_day_zip_in_the_next_years_zip_is_named() {
    let (eve, own, before) = (day(2025, 12, 31), day(2026, 1, 2), day(2025, 12, 30));
    let ticker = "NIFTY29JAN2626000CE";
    let file = |d: Day| csv(&[row(&format!("{ticker}.NFO"), d, 33_300, "10", 75, 300)]);
    let day_zip = |d: Day| zip(&[(&entry(d, ticker), &file(d), Method::Deflated)]);
    let (z_eve, z_own, z_before) = (day_zip(eve), day_zip(own), day_zip(before));
    let found = "DEC_2025/GFDLNFO_TICK_31122025.zip";
    let root = scratch("nfo-fm043");
    let dir = root.join("zips");
    put(
        &dir,
        "2026.zip",
        &zip(&[
            ("DEC_2025/", b"", Method::Stored),
            (found, &z_eve, Method::Stored),
            ("JAN_2026/GFDLNFO_TICK_02012026.zip", &z_own, Method::Stored),
        ]),
    );
    let misfiled = CmRefusal::DayZipMisfiled {
        found: format!("2026.zip:{found}"),
        expected: format!("2025.zip:{found}"),
    };
    let words = misfiled.to_string();
    assert!(
        words.contains("2026.zip:") && words.contains("2025.zip:"),
        "{words}"
    );

    // No 2025.zip at all, and 2026.zip never walked before the day is asked.
    let zips = NfoZips::new(&dir);
    assert_eq!(zips.day(eve).err(), Some(misfiled.clone()));
    let listing = zips.day(own).unwrap().unwrap();
    assert_eq!(listing.entries().len(), 1);
    assert_eq!(zips.fetch(&listing.entries()[0]).unwrap(), file(own));

    // A 2025.zip that holds another day of December, not this one.
    let only_before = zip(&[(
        "DEC_2025/GFDLNFO_TICK_30122025.zip",
        &z_before,
        Method::Stored,
    )]);
    put(&dir, "2025.zip", &only_before);
    let zips = NfoZips::new(&dir);
    assert_eq!(zips.day(eve).err(), Some(misfiled.clone()));
    assert!(zips.day(before).unwrap().is_some());
    assert!(zips.day(own).unwrap().is_some());

    // The day at its own place as well: two copies of one day.
    let both = zip(&[
        (
            "DEC_2025/GFDLNFO_TICK_30122025.zip",
            &z_before,
            Method::Stored,
        ),
        (found, &z_eve, Method::Stored),
    ]);
    put(&dir, "2025.zip", &both);
    let zips = NfoZips::new(&dir);
    assert_eq!(
        zips.day(eve).err(),
        Some(CmRefusal::ArchiveDuplicateDay { day: eve })
    );
    assert!(zips.day(before).unwrap().is_some());
    assert!(zips.day(own).unwrap().is_some());
    std::fs::remove_dir_all(root).unwrap();
}

/// FM-043's extremes (IA-3-OPT, D-4818's one rule over the year zips): a day
/// zip under another month folder, at the year zip's root, in a fiscal-year
/// `2024-25.zip`, or with two copies elsewhere is named, never read as not
/// held; names that are not a `*.zip` (a `.DS_Store`, `2024.zip.part`,
/// `README.txt`) are never walked; a zip that cannot be read refuses its
/// own days, and a day held nowhere readable answers the first unread zip's
/// refusal until it reads; a directory that is a file refuses by name and
/// one that is absent holds nothing; and threads sharing one source see one
/// answer.
#[test]
fn a_misfiled_or_unread_options_day_zip_is_never_read_as_not_held() {
    let ticker = "NIFTY25APR2422000CE";
    let file = |d: Day| csv(&[row(&format!("{ticker}.NFO"), d, 33_300, "10", 75, 300)]);
    let day_zip = |d: Day| zip(&[(&entry(d, ticker), &file(d), Method::Deflated)]);
    let (d1, d2, d3, d4, d5) = (
        day(2024, 4, 1),
        day(2024, 4, 2),
        day(2024, 4, 3),
        day(2024, 4, 4),
        day(2024, 4, 5),
    );
    let (z1, z2, z3, z4) = (day_zip(d1), day_zip(d2), day_zip(d3), day_zip(d4));
    let root = scratch("nfo-fm043-extremes");
    let dir = root.join("zips");
    put(
        &dir,
        "2024.zip",
        &zip(&[
            ("APR_2024/", b"", Method::Stored),
            ("APR_2024/GFDLNFO_TICK_01042024.zip", &z1, Method::Stored),
            ("MAY_2024/GFDLNFO_TICK_02042024.zip", &z2, Method::Stored),
            ("GFDLNFO_TICK_03042024.zip", &z3, Method::Stored),
            ("MAY_2024/GFDLNFO_TICK_04042024.zip", &z4, Method::Stored),
            ("APR_2024/GFDLNFO_TICK_31022024.zip", &z4, Method::Stored),
            ("APR_2024/notes.txt", b"invented", Method::Stored),
        ]),
    );
    put(
        &dir,
        "2023.zip",
        &zip(&[("APR_2024/GFDLNFO_TICK_04042024.zip", &z4, Method::Stored)]),
    );
    // A fiscal-year zip holding a day, a zip of another name holding no day
    // zip (walked, nothing noted), and names that are not a `*.zip` (never
    // opened, so their bytes, which are no zip, refuse nothing).
    put(
        &dir,
        "2024-25.zip",
        &zip(&[(
            "APR_2024/GFDLNFO_TICK_05042024.zip",
            &day_zip(d5),
            Method::Stored,
        )]),
    );
    put(
        &dir,
        "20245.zip",
        &zip(&[("README.txt", b"invented", Method::Stored)]),
    );
    for stray in [".DS_Store", "2024.zip.part", "README.txt"] {
        put(&dir, stray, b"not a year zip");
    }
    let zips = NfoZips::new(&dir);
    let got = zips.day(d1).unwrap().unwrap();
    assert_eq!(zips.fetch(&got.entries()[0]).unwrap(), file(d1));
    assert_eq!(
        zips.day(d2).err(),
        Some(CmRefusal::DayZipMisfiled {
            found: "2024.zip:MAY_2024/GFDLNFO_TICK_02042024.zip".into(),
            expected: "2024.zip:APR_2024/GFDLNFO_TICK_02042024.zip".into(),
        })
    );
    assert_eq!(
        zips.day(d3).err(),
        Some(CmRefusal::DayZipMisfiled {
            found: "2024.zip:GFDLNFO_TICK_03042024.zip".into(),
            expected: "2024.zip:APR_2024/GFDLNFO_TICK_03042024.zip".into(),
        })
    );
    assert_eq!(
        zips.day(d4).err(),
        Some(CmRefusal::ArchiveDuplicateDay { day: d4 })
    );
    assert_eq!(
        zips.day(d5).err(),
        Some(CmRefusal::DayZipMisfiled {
            found: "2024-25.zip:APR_2024/GFDLNFO_TICK_05042024.zip".into(),
            expected: "2024.zip:APR_2024/GFDLNFO_TICK_05042024.zip".into(),
        })
    );
    let d8 = day(2024, 4, 8);
    assert!(zips.day(d8).unwrap().is_none(), "held nowhere");
    assert_eq!(
        NfoZips::day_zip_name(day(2025, 12, 31)),
        "DEC_2025/GFDLNFO_TICK_31122025.zip"
    );

    // Threads sharing one fresh source: one walk, one answer each day.
    let shared = NfoZips::new(&dir);
    let answers: Vec<Vec<String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let zips = shared.clone();
                scope.spawn(move || {
                    [d1, d2, d3, d4, d5, d8]
                        .map(|d| match zips.day(d) {
                            Ok(Some(listing)) => format!("held {}", listing.entries().len()),
                            Ok(None) => "none".to_owned(),
                            Err(why) => why.to_string(),
                        })
                        .to_vec()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for answer in &answers {
        assert_eq!(answer, &answers[0]);
    }
    assert_eq!(answers[0][0], "held 1");
    assert_eq!(answers[0][5], "none");

    // A year zip that is not a zip: its own days refuse it, a day held
    // nowhere readable answers it, a day held at its own place still reads,
    // and once the zip reads, both read as they are.
    let broken = root.join("broken");
    put(
        &broken,
        "2024.zip",
        &zip(&[("APR_2024/GFDLNFO_TICK_01042024.zip", &z1, Method::Stored)]),
    );
    put(&broken, "2025.zip", b"not a zip at all, invented");
    let zips = NfoZips::new(&broken);
    assert!(zips.day(d1).unwrap().is_some());
    let own = zips.day(day(2025, 4, 1)).expect_err("its own day refuses");
    assert_eq!(zips.day(d2).err(), Some(own.clone()), "{own}");
    let mended = day_zip(day(2025, 4, 1));
    put(
        &broken,
        "2025.zip",
        &zip(&[(
            "APR_2025/GFDLNFO_TICK_01042025.zip",
            &mended,
            Method::Stored,
        )]),
    );
    assert!(zips.day(day(2025, 4, 1)).unwrap().is_some());
    assert!(zips.day(d2).unwrap().is_none(), "no unread zip is left");

    // Any other zip that is not one is unread by its name, never skipped:
    // a day held nowhere answers its refusal; a day held at its place reads.
    let junk = root.join("junk");
    put(
        &junk,
        "2024.zip",
        &zip(&[("APR_2024/GFDLNFO_TICK_01042024.zip", &z1, Method::Stored)]),
    );
    put(&junk, "junk.zip", b"not a zip at all, invented");
    let zips = NfoZips::new(&junk);
    assert!(zips.day(d1).unwrap().is_some());
    assert_eq!(zips.day(d2).err(), Some(own));

    // A zip whose name is not a lowercase `.zip` is unread by its name:
    // never read as a year zip of another spelling, never passed over.
    let upper = root.join("upper");
    put(
        &upper,
        "2024.zip",
        &zip(&[("APR_2024/GFDLNFO_TICK_01042024.zip", &z1, Method::Stored)]),
    );
    put(&upper, "Extra.ZIP", b"invented");
    let zips = NfoZips::new(&upper);
    assert!(zips.day(d1).unwrap().is_some());
    assert_eq!(
        zips.day(d2).err(),
        Some(CmRefusal::ArchiveMalformed {
            what: "a zip name that is not UTF-8 ending in a lowercase .zip",
        })
    );

    // A year zip that cannot be opened, where the platform refuses it.
    let locked = root.join("locked");
    let path = put(&locked, "2026.zip", b"invented");
    put(
        &locked,
        "2024.zip",
        &zip(&[("APR_2024/GFDLNFO_TICK_01042024.zip", &z1, Method::Stored)]),
    );
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::File::open(&path).is_err() {
            let zips = NfoZips::new(&locked);
            let denied = Some(CmRefusal::ArchiveUnavailable {
                kind: std::io::ErrorKind::PermissionDenied,
            });
            assert_eq!(zips.day(day(2026, 1, 2)).err(), denied);
            assert_eq!(zips.day(d2).err(), denied);
            assert!(zips.day(d1).unwrap().is_some());
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    // The directory is a file, or is absent.
    let plain = put(&root, "plain", b"invented");
    assert!(matches!(
        NfoZips::new(&plain).day(d1),
        Err(CmRefusal::ArchiveUnavailable { .. })
    ));
    assert!(
        NfoZips::new(&root.join("absent"))
            .day(d1)
            .unwrap()
            .is_none()
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// WP-12e (DB-B02, DB-B03, DB-B13) against the operator's REAL options zips,
/// read-only: the unchanged-lot arm of the detector on ex-dates the audit
/// measured, with the vendor day after each as the control, and on ordinary
/// NIFTY and BANKNIFTY days (which must declare nothing). The pre-ex state is
/// the latest contract-day of every key over `BRUTEX_WP12E_LOOKBACK` days
/// (default 30). `BRUTEX_WP12E_ONLY` runs the cases whose underlying it
/// names. Only counts are printed; no vendor row reaches the repository.
#[test]
#[ignore = "requires BRUTEX_GDFL_NFO_ARCHIVES (the vendor's yearly options zips); read-only"]
fn the_real_unchanged_lot_events_are_detected_and_ordinary_days_are_not() {
    use crate::gdfl_lineage::{ContractDay, EventKind, detect};
    let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
    let only = std::env::var("BRUTEX_WP12E_ONLY").ok();
    let lookback: u32 = std::env::var("BRUTEX_WP12E_LOOKBACK")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(30);
    let zips = NfoZips::new(Path::new(&dir));
    let day_of = |under: &str, on: Day| {
        let mut out: Vec<ContractDay> = Vec::new();
        let Some(listing) = zips.day(on).unwrap() else {
            return out;
        };
        for file in listing.entries() {
            let Some(name) = listing.ticker_of(file) else {
                continue;
            };
            let Ok(ticker) = decode_ticker(name, on) else {
                continue;
            };
            if ticker.underlying.as_str() != under {
                continue;
            }
            let read = read_file(&zips, &listing, file).unwrap();
            out.push(ContractDay::from_rows(
                &read.ticker,
                read.file.rows.iter().map(|r| (r.sod, r.ltq, r.oi)),
            ));
        }
        out
    };
    // (underlying, the vendor day before, the ex-date, the vendor day after,
    // the kind expected, the shift in paisa for an offset).
    let offset = |shift| Some(EventKind::StrikeOffset { shift });
    let cases = [
        (
            "OIL",
            day(2020, 2, 19),
            day(2020, 2, 20),
            day(2020, 2, 24),
            offset(900),
        ),
        (
            "COALINDIA",
            day(2020, 3, 18),
            day(2020, 3, 19),
            day(2020, 3, 20),
            offset(1_200),
        ),
        (
            "VEDL",
            day(2022, 7, 25),
            day(2022, 7, 26),
            day(2022, 7, 27),
            offset(1_950),
        ),
        (
            "HEROMOTOCO",
            day(2023, 2, 16),
            day(2023, 2, 17),
            day(2023, 2, 20),
            offset(6_500),
        ),
        (
            "VEDL",
            day(2024, 9, 9),
            day(2024, 9, 10),
            day(2024, 9, 11),
            offset(2_000),
        ),
        (
            "TATACHEM",
            day(2020, 3, 3),
            day(2020, 3, 4),
            day(2020, 3, 5),
            Some(EventKind::Relisted),
        ),
        // Ordinary days: an index across a weekly expiry, and the last full
        // options day the store holds.
        (
            "NIFTY",
            day(2020, 9, 10),
            day(2020, 9, 11),
            day(2020, 9, 14),
            None,
        ),
        (
            "BANKNIFTY",
            day(2020, 9, 14),
            day(2020, 9, 15),
            day(2020, 9, 16),
            None,
        ),
        (
            "RELIANCE",
            day(2020, 9, 14),
            day(2020, 9, 15),
            day(2020, 9, 16),
            None,
        ),
    ];
    for (under, before, ex, next, want) in cases {
        if only.as_deref().is_some_and(|name| name != under) {
            continue;
        }
        let symbol = brutex_core::symbol::Symbol::new(under).unwrap();
        // An index day is hundreds of files: its pre-ex state is the eve.
        let back = if want.is_none() && under != "RELIANCE" {
            0
        } else {
            lookback
        };
        let mut latest: std::collections::BTreeMap<String, ContractDay> =
            std::collections::BTreeMap::new();
        let mut on = Day::from_days(before.days_from_epoch() - back).unwrap();
        while on <= before {
            for c in day_of(under, on) {
                if c.expiry >= ex {
                    latest.insert(c.contract().unwrap().to_string(), c);
                }
            }
            on = on.succ().unwrap();
        }
        let pre: Vec<ContractDay> = latest.into_values().collect();
        let (post, later) = (day_of(under, ex), day_of(under, next));
        let at = std::time::Instant::now();
        let got = detect(symbol, ex, &pre, &post);
        let took = at.elapsed();
        eprintln!(
            "{under} {ex}: pre-ex keys {}, ex-date keys {}, detect {} us -> {}",
            pre.len(),
            post.len(),
            took.as_micros(),
            got.as_ref().map_or_else(
                || "none".to_owned(),
                |a| format!(
                    "{:?} lot {}->{} pairs {} unpaired {} stitched {} reused {}",
                    a.kind,
                    a.lot_before,
                    a.lot_after,
                    a.lineage.len(),
                    a.unpaired.len(),
                    a.stitched.len(),
                    a.lineage.iter().filter(|l| l.reused).count()
                )
            )
        );
        assert_eq!(got.as_ref().map(|a| a.kind), want, "{under} {ex}");
        if let Some(action) = &got {
            assert_eq!(
                action.lot_before, action.lot_after,
                "{under}: the lot is unchanged"
            );
            for pair in &action.lineage {
                let line = pair.record(&[]).line();
                assert!(
                    crate::gdfl_lineage::LineageRecord::parse(&line).is_some(),
                    "{line}"
                );
            }
        }
        // The day after is an ordinary day.
        let control = detect(symbol, next, &post, &later);
        assert_eq!(
            control.map(|a| a.kind),
            None,
            "{under} {next}: the control day"
        );
    }
}

/// WP-12e against the operator's REAL options zips, read-only: the whole
/// detector over EVERY stock underlying of every vendor day in
/// `BRUTEX_WP12E_FROM..=BRUTEX_WP12E_TO`, carrying each underlying's pre-ex
/// state from day to day as the import does. It lists every event declared
/// (the false positives and the misses are read off that list against the
/// audit's) and the cost of each `detect` call. `BRUTEX_WP12E_INDICES=1`
/// includes the index underlyings. Only counts are printed.
#[test]
#[ignore = "requires BRUTEX_GDFL_NFO_ARCHIVES and BRUTEX_WP12E_FROM/TO; read-only"]
fn the_real_window_sweep_lists_every_event_the_detector_declares() {
    use crate::gdfl_lineage::{ContractDay, detect};
    use std::collections::{BTreeMap, HashMap};
    let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
    let bound = |var: &str| {
        let text = std::env::var(var).expect(var);
        let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
        day(
            parts[0],
            u8::try_from(parts[1]).unwrap(),
            u8::try_from(parts[2]).unwrap(),
        )
    };
    let (from, to) = (bound("BRUTEX_WP12E_FROM"), bound("BRUTEX_WP12E_TO"));
    let indices = std::env::var_os("BRUTEX_WP12E_INDICES").is_some();
    let zips = NfoZips::new(Path::new(&dir));
    let mut state: HashMap<String, HashMap<(Day, i64, OptionSide), ContractDay>> = HashMap::new();
    let (mut days, mut underlying_days, mut unread) = (0_u32, 0_u32, 0_u32);
    let mut events: Vec<String> = Vec::new();
    let mut nanos: Vec<u128> = Vec::new();
    let mut on = from;
    while on <= to {
        if let Some(listing) = zips.day(on).unwrap() {
            days += 1;
            let mut by: BTreeMap<String, Vec<ContractDay>> = BTreeMap::new();
            for file in listing.entries() {
                let Some(name) = listing.ticker_of(file) else {
                    continue;
                };
                let Ok(ticker) = decode_ticker(name, on) else {
                    continue;
                };
                let under = ticker.underlying.as_str();
                if !indices && brutex_core::alias::index_by_fno(under).is_some() {
                    continue;
                }
                let Ok(read) = read_file(&zips, &listing, file) else {
                    unread += 1;
                    continue;
                };
                by.entry(under.to_owned())
                    .or_default()
                    .push(ContractDay::from_rows(
                        &read.ticker,
                        read.file.rows.iter().map(|r| (r.sod, r.ltq, r.oi)),
                    ));
            }
            for (under, todays) in by {
                let latest = state.entry(under.clone()).or_default();
                latest.retain(|key, _| key.0 >= on);
                let before: Vec<ContractDay> = latest.values().copied().collect();
                let symbol = brutex_core::symbol::Symbol::new(&under).unwrap();
                let at = std::time::Instant::now();
                let got = detect(symbol, on, &before, &todays);
                nanos.push(at.elapsed().as_nanos());
                underlying_days += 1;
                if let Some(action) = got {
                    events.push(format!(
                        "{on} {under} {:?} lot {}->{} pairs {} unpaired {} stitched {} (pre-ex keys {}, day keys {})",
                        action.kind,
                        action.lot_before,
                        action.lot_after,
                        action.lineage.len(),
                        action.unpaired.len(),
                        action.stitched.len(),
                        before.len(),
                        todays.len()
                    ));
                    latest.clear();
                }
                for c in todays {
                    latest.insert((c.expiry, c.strike, c.side), c);
                }
            }
        }
        on = on.succ().unwrap();
    }
    nanos.sort_unstable();
    let at = |q: usize| {
        nanos
            .get(nanos.len().saturating_sub(1) * q / 100)
            .copied()
            .unwrap_or(0)
    };
    eprintln!(
        "{from}..={to}: {days} vendor days, {underlying_days} underlying-days, {unread} files not read, {} events",
        events.len()
    );
    eprintln!(
        "detect per underlying-day, n={}: p50 {} ns, p99 {} ns, max {} ns",
        nanos.len(),
        at(50),
        at(99),
        at(100)
    );
    for event in &events {
        eprintln!("  {event}");
    }
    assert!(days > 0 && underlying_days > 0, "no vendor day in range");
}

/// The row loop `decode_capped` ran until D-5800, kept here as the oracle the
/// fixed array is checked against on real days: one `Vec<&str>` per row and an
/// unreserved row vector. Test-only; nothing in the crate calls it.
fn decode_by_field_vector(bytes: &[u8], stem: &str, day: Day) -> Result<NfoFile, NfoRefusal> {
    let cap = crate::fetch::MAX_ROWS;
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let mut lines = body.split(|&b| b == b'\n');
    match lines
        .next()
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
    {
        Some(h)
            if h == HEADER_OPEN_INTEREST.as_bytes()
                || h == HEADER_OPEN_INTEREST_SPACED.as_bytes() => {}
        _ => return Err(NfoRefusal::HeaderUnknown),
    }
    let date = format!("{:02}/{:02}/{:04}", day.day(), day.month(), day.year());
    let mut rows = Vec::new();
    let mut quoted_rows = 0_u32;
    for (index, raw) in lines.enumerate() {
        let line = u32::try_from(index.saturating_add(2)).unwrap_or(u32::MAX);
        if rows.len() >= cap {
            return Err(NfoRefusal::RowsOverCap { cap });
        }
        let malformed = NfoRefusal::MalformedRow { line };
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        let text = core::str::from_utf8(raw).map_err(|_| malformed.clone())?;
        let fields: Vec<&str> = text.split(',').collect();
        let &[
            ticker,
            row_date,
            time,
            ltp,
            bid,
            bid_qty,
            ask,
            ask_qty,
            ltq,
            oi,
        ] = fields.as_slice()
        else {
            return Err(malformed);
        };
        if ticker != stem {
            return Err(NfoRefusal::TickerMismatch { line });
        }
        if row_date != date {
            return Err(NfoRefusal::DateMismatch { line });
        }
        let sod = second_of_day(time).ok_or_else(|| malformed.clone())?;
        if !(is_decimal(bid) && is_decimal(ask) && is_digits(bid_qty) && is_digits(ask_qty)) {
            return Err(malformed);
        }
        if !(is_zero_decimal(bid) && is_zero_decimal(ask)) {
            quoted_rows = quoted_rows.saturating_add(1);
        }
        let ltq: u64 = if is_digits(ltq) {
            ltq.parse().ok()
        } else {
            None
        }
        .ok_or_else(|| malformed.clone())?;
        if !is_digits(oi) {
            return Err(malformed);
        }
        let oi: i64 = oi
            .parse()
            .map_err(|_| NfoRefusal::OpenInterestRefused { line })?;
        let paisa = if is_decimal(ltp) {
            crate::csv::paisa(ltp)
        } else {
            None
        }
        .ok_or(NfoRefusal::PriceRefused { line })?;
        if ltq > 0 && paisa < 1 {
            return Err(NfoRefusal::PriceRefused { line });
        }
        rows.push(NfoRow {
            line,
            sod,
            ltp: paisa,
            ltq,
            oi,
        });
    }
    Ok(NfoFile {
        rows,
        quoted_rows,
        refused: crate::gdfl_cm::RowsRefused::default(),
        trailing_empty: 0,
    })
}

/// D-4816's whole delta, file by file: `per_row` (bar definition 5's
/// reader) against `whole` (the whole-file rule of definition 4 and before,
/// the oracle above). Where the old reader read a file the new one reads the
/// same rows and refuses none; where the old reader named a row fault at
/// line L, the new one either names the same refusal, or reads the file and
/// names L as its first refused row, or refuses later, past L, by a rule the
/// old reader never reached; a header the old reader refused is read only
/// with empty trailing fields (D-4829).
fn definition_5_differs_only_where_4_refused(
    per_row: &Result<NfoFile, NfoRefusal>,
    whole: &Result<NfoFile, NfoRefusal>,
) -> bool {
    let row_fault = |why: &NfoRefusal| match why {
        NfoRefusal::MalformedRow { line }
        | NfoRefusal::PriceRefused { line }
        | NfoRefusal::OpenInterestRefused { line } => Some(*line),
        _ => None,
    };
    match (per_row, whole) {
        (Ok(new), Ok(old)) => {
            new.rows == old.rows
                && new.quoted_rows == old.quoted_rows
                && new.refused.total() == 0
                && new.trailing_empty == 0
        }
        (Err(new), Err(old)) if new == old => true,
        (Ok(new), Err(NfoRefusal::HeaderUnknown)) => new.trailing_empty > 0,
        (Ok(new), Err(old)) => row_fault(old)
            .is_some_and(|at| new.refused.named.first().is_some_and(|row| row.line == at)),
        (Err(new), Err(old)) => row_fault(old).is_some_and(|at| match new {
            NfoRefusal::TooManyBadRows { .. } | NfoRefusal::RowsOverCap { .. } => true,
            NfoRefusal::TickerMismatch { line } | NfoRefusal::DateMismatch { line } => *line > at,
            other => row_fault(other).is_some_and(|line| line > at),
        }),
        (Err(_), Ok(_)) => false,
    }
}

/// The oracle agrees with the fixed array on invented files under D-4816's
/// relation (bar definition 5 differs from 4 only where 4 refused a file): a
/// good file, a row of eleven fields after a good one, a row of nine, a
/// quoted row and a bad price, so the real-day check below compares two
/// readers that are known to agree here. Every file of the good kind, and
/// each lone bad row, is still byte for byte the same answer.
#[test]
fn the_field_vector_oracle_agrees_with_the_fixed_array_on_invented_files() {
    let good = rows(&[(33_300, "10.05", 75, 300), (33_301, "0", 0, 300)]);
    let wide = format!("{},0", good[0]);
    let narrow = good[0].rsplit_once(',').map(|(head, _)| head.to_owned());
    let quoted = good[0].replace(",0,0,0,0,", ",10.00,75,10.10,75,");
    let bad_price = good[0].replace("10.05", "10.x");
    for file in [
        csv(&good),
        csv(&[good[0].clone(), wide]),
        csv(&[narrow.expect("a comma")]),
        csv(&[quoted]),
        csv(&[bad_price]),
        b"Ticker,Date\r\n".to_vec(),
    ] {
        let (new, old) = (
            decode(&file, STEM, trade()),
            decode_by_field_vector(&file, STEM, trade()),
        );
        assert!(
            definition_5_differs_only_where_4_refused(&new, &old),
            "{new:?} against {old:?}"
        );
    }
    let (eleven, alone) = (
        csv(&[good[0].clone(), format!("{},0", good[0])]),
        csv(&[format!("{},0", good[0])]),
    );
    assert_eq!(
        decode(&alone, STEM, trade()),
        decode_by_field_vector(&alone, STEM, trade())
    );
    assert!(decode(&eleven, STEM, trade()).is_ok());
    assert!(decode_by_field_vector(&eleven, STEM, trade()).is_err());
}

/// **D-5800 ON REAL DAYS, READ-ONLY.** Every option file of NIFTY,
/// BANKNIFTY and RELIANCE on the first options day of the tick store
/// (2018-09-03, the early day form), a special session (the 2023-11-12
/// Muhurat session) and 2026-08-03 (11,524 files) decodes through the fixed
/// array with the outcome the field vector gave, under D-4816's relation
/// (`definition_5_differs_only_where_4_refused`): the same rows, or the same
/// refusal, or the file read where the old rule refused it whole. `BRUTEX_GDFL_TICKSTORE` is the tick store root, opened
/// read-only by `NfoTickStore`; only counts are printed, no vendor row
/// reaches the repository.
#[test]
#[ignore = "requires BRUTEX_GDFL_TICKSTORE (the tick store root); read-only"]
fn the_fixed_array_decodes_real_days_as_the_field_vector_did() {
    let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
    let store = NfoTickStore::new(Path::new(&root));
    for on in [day(2018, 9, 3), day(2023, 11, 12), day(2026, 8, 3)] {
        let listing = store.day(on).unwrap().expect("a tick-store day");
        let (mut files, mut rows, mut refused) = (0_u64, 0_u64, 0_u64);
        let (mut fixed_ns, mut vector_ns) = (0_u128, 0_u128);
        for file in listing.entries() {
            let Some(name) = listing.ticker_of(file) else {
                continue;
            };
            let Ok(ticker) = decode_ticker(name, on) else {
                continue;
            };
            if !["NIFTY", "BANKNIFTY", "RELIANCE"].contains(&ticker.underlying.as_str()) {
                continue;
            }
            let bytes = store.fetch(file).unwrap();
            crate::gdfl_cm::verify(file, &bytes).unwrap();
            let stem = format!("{name}{ROW_SUFFIX}");
            let at = std::time::Instant::now();
            let fixed = decode(&bytes, &stem, on);
            fixed_ns += at.elapsed().as_nanos();
            let at = std::time::Instant::now();
            let vector = decode_by_field_vector(&bytes, &stem, on);
            vector_ns += at.elapsed().as_nanos();
            assert!(
                definition_5_differs_only_where_4_refused(&fixed, &vector),
                "{on} {name}: {fixed:?} against {vector:?}"
            );
            files += 1;
            match fixed {
                Ok(read) => rows += u64::try_from(read.rows.len()).unwrap(),
                Err(_) => refused += 1,
            }
        }
        eprintln!(
            "{on}: {files} NIFTY/BANKNIFTY/RELIANCE option files, {rows} rows, {refused} refused \
             files, the readers agree under D-4816; fixed array {} ms, field vector {} ms (debug build)",
            fixed_ns / 1_000_000,
            vector_ns / 1_000_000
        );
        assert!(files > 0, "{on}: the day holds the three underlyings");
    }
}

// ── IA-4: per-row parity (D-4816) and the trailing-empty header (D-4829) ────

/// `n` invented rows of `STEM` from 09:15:00, one a second, every one traded
/// at an invented price no real option file carries.
fn many(n: u32) -> Vec<String> {
    (0..n)
        .map(|i| row(STEM, trade(), 33_300 + i % 22_000, "10.05", 1, 300))
        .collect()
}

/// The lines a decoded file kept and the `(line, fault)` it named.
fn shape(file: &NfoFile) -> (Vec<u32>, Vec<(u32, RowFault)>) {
    (
        file.rows.iter().map(|row| row.line).collect(),
        file.refused
            .named
            .iter()
            .map(|row| (row.line, row.fault))
            .collect(),
    )
}

/// `IA-MX-F6`: a row of eleven fields is refused by name, and the file read.
#[test]
fn an_option_row_with_eleven_fields_is_refused_and_the_file_read() {
    let mut lines = many(3);
    lines[1] = format!("{},0", lines[1]);
    let got = decode(&csv(&lines), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::Malformed)]));
    assert_eq!(got.refused.malformed, 1);
}

/// `IA-MX-F10` (critic IA-4-1): a row that is not UTF-8 is refused by name
/// and the file read; under definition 4 it refused the whole file, which
/// the oracle of definition 4 still says.
#[test]
fn an_option_row_that_is_not_utf8_is_refused_and_the_file_read() {
    let mut bytes = csv(&many(3));
    let at = bytes.windows(8).position(|w| w == b"09:15:01").unwrap();
    bytes.insert(at, 0xC3);
    let got = decode(&bytes, STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::Malformed)]));
    assert_eq!(
        decode_by_field_vector(&bytes, STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 3 }),
        "definition 4 refused the whole file"
    );
}

/// `IA-MX-F13`: 2018-11-09's shape, invented: 4,353 data rows, line 451 a
/// ten-field row whose time is `99:a9:99`. 4,352 rows are read and line 451
/// is named; under definition 4 the contract-day was lost.
#[test]
fn an_option_row_with_a_bad_time_is_refused_by_line_and_the_rest_read() {
    let mut lines = many(4_353);
    lines[449] = lines[449].replacen(&hms(33_300 + 449), "99:a9:99", 1);
    let got = decode(&csv(&lines), STEM, trade()).unwrap();
    assert_eq!(got.rows.len(), 4_352);
    assert_eq!(shape(&got).1, [(451, RowFault::Malformed)]);
    assert!(got.rows.iter().all(|row| row.line != 451));
    assert_eq!(got.refused.named[0].sod, None, "no time read");
}

/// `IA-MX-F38`: 2018-11-06's shape, invented: 13,991 data rows, line 13,668
/// a ten-field row whose LTP is four characters with a letter in it. 13,990
/// rows are read and line 13,668 is named `malformed` with its second; alone
/// in its file the same row is still the file's `price_refused`, as
/// definition 4 named the whole file (the contract-day it lost, journal line
/// 8,029).
#[test]
fn an_option_row_with_letters_in_its_price_is_refused_by_line_and_the_rest_read() {
    let mut lines = many(13_991);
    let at = 13_668 - 2;
    lines[at] = lines[at].replacen(",10.05,", ",1O.5,", 1);
    let got = decode(&csv(&lines), STEM, trade()).unwrap();
    assert_eq!(got.rows.len(), 13_990);
    assert_eq!(shape(&got).1, [(13_668, RowFault::Malformed)]);
    assert!(got.rows.iter().all(|row| row.line != 13_668));
    let sod = 33_300 + u32::try_from(at).unwrap() % 22_000;
    assert_eq!(got.refused.named[0].sod, Some(sod), "its time was read");
    assert_eq!(
        decode(&csv(&lines[at..=at]), STEM, trade()),
        Err(NfoRefusal::PriceRefused { line: 2 })
    );
}

/// FM-077's decode half (the e2e failure matrix; its owner IMPORT-BAD-ROW,
/// R4-22, names each refused row in the durable audit stream and closes the
/// row): ONE traded row at zero in an options file refuses that row by name,
/// line and second, and the file is kept with every other row in file order.
/// Under definition 4 the same bytes refused the whole file, and with it the
/// contract-day, as the definition-4 reader kept as an oracle still says.
#[test]
fn one_zero_priced_traded_row_is_refused_and_the_file_kept() {
    let mut lines = many(5);
    lines[2] = lines[2].replacen(",10.05,", ",0.00,", 1);
    let bytes = csv(&lines);
    let got = decode(&bytes, STEM, trade()).unwrap();
    assert_eq!(
        shape(&got),
        (vec![2, 3, 5, 6], vec![(4, RowFault::ZeroLtp)])
    );
    assert_eq!(
        (
            got.refused.total(),
            got.refused.zero_ltp,
            got.refused.named[0].sod
        ),
        (1, 1, Some(33_302))
    );
    assert!(got.rows.iter().all(|row| row.ltp == 1_005), "{got:?}");
    assert_eq!(
        decode_by_field_vector(&bytes, STEM, trade()),
        Err(NfoRefusal::PriceRefused { line: 4 }),
        "definition 4 refused the whole file"
    );
}

/// `IA-MX-F20` (critic IA-4-1): a TRADED row at zero is refused by name as
/// `zero_ltp`, the file read; an untraded row at zero is still kept.
#[test]
fn a_traded_option_row_at_zero_is_refused_and_the_file_read() {
    let mut lines = many(3);
    lines[0] = lines[0]
        .replacen(",10.05,", ",0,", 1)
        .replacen(",1,300", ",0,300", 1);
    lines[1] = lines[1].replacen(",10.05,", ",0.00,", 1);
    let got = decode(&csv(&lines), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::ZeroLtp)]));
    assert_eq!(
        (got.refused.zero_ltp, got.refused.named[0].sod),
        (1, Some(33_301))
    );
}

/// `IA-MX-F21` (critic IA-4-1): an LTP with a third decimal is refused as
/// `off_grid`, never snapped; the rest of the file is read.
#[test]
fn an_option_price_with_a_third_decimal_refuses_its_row() {
    let mut lines = many(3);
    lines[2] = lines[2].replacen(",10.05,", ",10.055,", 1);
    let got = decode(&csv(&lines), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 3], vec![(4, RowFault::OffGrid)]));
    assert_eq!(got.refused.off_grid, 1);
}

/// `IA-MX-F23`: 2026-04-16's shape, invented: one negative LTQ refuses its
/// row as `malformed` (D-3335 `rows_refused_malformed`) and the file lands.
#[test]
fn a_negative_option_quantity_refuses_its_row_and_the_file_lands() {
    let mut lines = many(4);
    lines[1] = lines[1].replacen(",1,300", ",-1,300", 1);
    let got = decode(&csv(&lines), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4, 5], vec![(3, RowFault::Malformed)]));
    assert_eq!(got.refused.malformed, 1);
}

/// `IA-MX-F33`: a malformed quote field refuses its row in BOTH readers,
/// the same fault, the rest of each file read.
#[test]
fn a_malformed_quote_field_refuses_its_row_in_both_readers() {
    for bent in [",x,0,0,0,", ",0,x,0,0,", ",0,0,1.5.5,0,", ",0,0,0,-1,"] {
        let mut lines = many(3);
        lines[1] = lines[1].replacen(",0,0,0,0,", bent, 1);
        let got = decode(&csv(&lines), STEM, trade()).unwrap();
        assert_eq!(
            shape(&got),
            (vec![2, 4], vec![(3, RowFault::Malformed)]),
            "{bent}"
        );
        let cm_stem = "ZZTEST.NSE";
        let cm: Vec<String> = lines.iter().map(|l| l.replacen(STEM, cm_stem, 1)).collect();
        let expect = crate::gdfl_cm::Expect {
            kind: crate::gdfl_cm::CmKind::Stocks,
            stem: cm_stem,
            day: trade(),
        };
        let cash = crate::gdfl_cm::decode(&csv(&cm), &expect).unwrap();
        let named: Vec<(u32, RowFault)> = cash
            .refused
            .named
            .iter()
            .map(|row| (row.line, row.fault))
            .collect();
        assert_eq!(
            named,
            [(3, RowFault::Malformed)],
            "{bent}: the capital-market reader"
        );
    }
}

/// `IA-MX-F39`: a blank line after line 1, in the middle or as CRLF CRLF at
/// the end, is a malformed row in both readers: refused by name, never the
/// file.
#[test]
fn a_blank_line_is_a_malformed_row() {
    let lines = many(2);
    let middle = vec![lines[0].clone(), String::new(), lines[1].clone()];
    let got = decode(&csv(&middle), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::Malformed)]));
    let mut tail = csv(&lines);
    tail.extend_from_slice(b"\r\n");
    let got = decode(&tail, STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 3], vec![(4, RowFault::Malformed)]));
    let cm_stem = "ZZTEST.NSE";
    let cm: Vec<String> = middle
        .iter()
        .map(|l| l.replacen(STEM, cm_stem, 1))
        .collect();
    let expect = crate::gdfl_cm::Expect {
        kind: crate::gdfl_cm::CmKind::Stocks,
        stem: cm_stem,
        day: trade(),
    };
    let cash = crate::gdfl_cm::decode(&csv(&cm), &expect).unwrap();
    assert_eq!(
        (
            cash.rows.len(),
            cash.refused.malformed,
            cash.refused.named[0].line
        ),
        (2, 1, 3),
        "the capital-market reader"
    );
}

/// `IA-MX-F40`: a repeated header (a later line starting `Ticker,`). The
/// design reads it as a malformed row by its time `Time`; both readers check
/// the Ticker field FIRST, and `Ticker` is not the file's stem, so it is a
/// row of another file, which refuses the whole file in both readers
/// (D-4816 keeps that rule; 0 such lines measured, `FMT-12`). The same line
/// carrying this file's own Ticker and Date (its time `Time`) is the
/// malformed row the design means.
#[test]
fn a_repeated_header_line_is_a_malformed_row() {
    let lines = many(2);
    let again = vec![
        lines[0].clone(),
        HEADER_OPEN_INTEREST.to_owned(),
        lines[1].clone(),
    ];
    let got = decode(&csv(&again), STEM, trade());
    assert_eq!(got, Err(NfoRefusal::TickerMismatch { line: 3 }));
    let cm_stem = "ZZTEST.NSE";
    let cm: Vec<String> = again.iter().map(|l| l.replacen(STEM, cm_stem, 1)).collect();
    let expect = crate::gdfl_cm::Expect {
        kind: crate::gdfl_cm::CmKind::Stocks,
        stem: cm_stem,
        day: trade(),
    };
    assert_eq!(
        crate::gdfl_cm::decode(&csv(&cm), &expect).map(|f| f.rows.len()),
        Err(CmRefusal::TickerMismatch { line: 3 }),
        "the capital-market reader: the same rule"
    );
    let spelled =
        HEADER_OPEN_INTEREST
            .replacen("Ticker", STEM, 1)
            .replacen("Date", "01/04/2024", 1);
    let again = vec![lines[0].clone(), spelled, lines[1].clone()];
    let got = decode(&csv(&again), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::Malformed)]));
}

/// `IA-MX-F41`: a double-quoted field fails its own rule: a quoted price,
/// quantity or open interest refuses its row; a quoted ticker is a row of
/// another file.
#[test]
fn a_quoted_field_is_refused_by_its_rule() {
    let lines = many(3);
    for (bent, fault) in [
        (
            lines[1].replacen(",10.05,", ",\"10.05\",", 1),
            RowFault::Malformed,
        ),
        (
            lines[1].replacen(",1,300", ",\"1\",300", 1),
            RowFault::Malformed,
        ),
        (
            lines[1].replacen(",1,300", ",1,\"300\"", 1),
            RowFault::Malformed,
        ),
    ] {
        let file = vec![lines[0].clone(), bent.clone(), lines[2].clone()];
        let got = decode(&csv(&file), STEM, trade()).unwrap();
        assert_eq!(shape(&got), (vec![2, 4], vec![(3, fault)]), "{bent}");
    }
    let quoted = lines[1].replacen(STEM, &format!("\"{STEM}\""), 1);
    let file = vec![lines[0].clone(), quoted, lines[2].clone()];
    assert_eq!(
        decode(&csv(&file), STEM, trade()),
        Err(NfoRefusal::TickerMismatch { line: 3 })
    );
}

/// `IA-MX-F42` and F9: a run of NUL bytes ending the file is a last line
/// with no newline that is not ten fields: a cut file, refused WHOLE as
/// before D-4816. The same NUL line with a newline after it is a malformed
/// row, refused by name.
#[test]
fn a_nul_tail_is_read_as_a_cut_last_line() {
    let mut torn = csv(&many(3));
    torn.extend_from_slice(&[0_u8; 64]);
    assert_eq!(
        decode(&torn, STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 5 })
    );
    torn.push(b'\n');
    let got = decode(&torn, STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 3, 4], vec![(5, RowFault::Malformed)]));
    // A row cut short with no newline: the file, whole.
    let mut cut = csv(&many(3));
    cut.truncate(cut.len() - 6);
    assert_eq!(
        decode(&cut, STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 4 })
    );
    // A complete last row with no newline is read (§8 F8), and a last row
    // whose fault is its price, not its shape, is refused alone, as the
    // capital-market reader does.
    let mut complete = csv(&many(3));
    complete.truncate(complete.len() - 2);
    assert_eq!(decode(&complete, STEM, trade()).unwrap().rows.len(), 3);
    let mut lines = many(3);
    lines[2] = lines[2].replacen(",10.05,", ",10.055,", 1);
    let mut priced = csv(&lines);
    priced.truncate(priced.len() - 2);
    let got = decode(&priced, STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 3], vec![(4, RowFault::OffGrid)]));
}

/// `IA-MX-F7` (design §8 F7, D-4816): a last line ending in a bare CR with no
/// LF after it is a file cut between its CR and its LF, whether the row
/// before the CR is complete or cut short: the whole file is refused at that
/// line, by the one rule the capital-market reader applies to the same
/// bytes. The same complete row with its CRLF, or with no line end at all
/// (§8 F8), is read; a stray CR on a line that has its LF is that row's own
/// fault, refused alone. Before this, the options reader stripped the CR and
/// read a cut file as a whole one.
#[test]
fn an_option_file_cut_between_cr_and_lf_is_refused_whole() {
    let cm_of = |bytes: &[u8]| {
        let text = String::from_utf8(bytes.to_vec())
            .unwrap()
            .replace(STEM, "ZZTEST.NSE");
        let expect = crate::gdfl_cm::Expect {
            kind: crate::gdfl_cm::CmKind::Stocks,
            stem: "ZZTEST.NSE",
            day: trade(),
        };
        crate::gdfl_cm::decode(text.as_bytes(), &expect).map(|file| file.rows.len())
    };
    for n in [1_u32, 3] {
        let last = n + 1;
        let whole = csv(&many(n));
        let mut cut = whole.clone();
        assert_eq!(cut.pop(), Some(b'\n'));
        assert_eq!(cut.last(), Some(&b'\r'), "the cut keeps the CR");
        assert_eq!(
            decode(&cut, STEM, trade()),
            Err(NfoRefusal::MalformedRow { line: last }),
            "{n} rows, cut between CR and LF"
        );
        assert_eq!(
            cm_of(&cut),
            Err(CmRefusal::MalformedRow { line: last }),
            "the capital-market reader refuses the same cut the same way"
        );
        let mut bare = cut.clone();
        assert_eq!(bare.pop(), Some(b'\r'));
        let kept = usize::try_from(n).unwrap();
        assert_eq!(decode(&bare, STEM, trade()).unwrap().rows.len(), kept);
        assert_eq!(cm_of(&bare), Ok(kept));
        assert_eq!(decode(&whole, STEM, trade()).unwrap().rows.len(), kept);
        // A row cut short and ending in a CR: the file, whole, at that line.
        let mut short = bare.clone();
        short.truncate(short.len() - 6);
        short.push(b'\r');
        assert_eq!(
            decode(&short, STEM, trade()),
            Err(NfoRefusal::MalformedRow { line: last })
        );
    }
    let mut lines = many(3);
    lines[1].push('\r');
    let got = decode(&csv(&lines), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::Malformed)]));
}

/// `IA-MX-F43`: a line longer than 256 bytes is split like any other and its
/// rules refuse it; no cap is added. A long LTP and a long ticker.
#[test]
fn an_overlong_line_is_a_malformed_row() {
    let lines = many(3);
    let long_price = lines[1].replacen(",10.05,", &format!(",1{}.05,", "0".repeat(300)), 1);
    let file = vec![lines[0].clone(), long_price.clone(), lines[2].clone()];
    assert!(long_price.len() > 256);
    let got = decode(&csv(&file), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::OffGrid)]));
    let long_tail = format!("{}{}", lines[1], ",".repeat(300));
    let file = vec![lines[0].clone(), long_tail, lines[2].clone()];
    let got = decode(&csv(&file), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 4], vec![(3, RowFault::Malformed)]));
}

/// `IA-MX-F36`, D-4829: 2025-03-11's shape, invented: the header followed by
/// k = 4 empty fields and every row by 4 empty fields is read, the fields
/// dropped and k counted; k from 1 to 8 is admitted, 9 is not; a row that
/// breaks the shape (no empty fields, a value in one, one too few) is a
/// malformed row; both observed headers take the rule.
#[test]
fn a_header_with_empty_trailing_fields_is_read_when_every_row_matches() {
    let body = |header: &str, k: usize, rows: &[String]| {
        let mut text = format!("{header}{}\r\n", ",".repeat(k));
        for r in rows {
            text.push_str(r);
            text.push_str("\r\n");
        }
        text.into_bytes()
    };
    let plain = many(1_446);
    let padded: Vec<String> = plain.iter().map(|r| format!("{r},,,,")).collect();
    let got = decode(&body(HEADER_OPEN_INTEREST, 4, &padded), STEM, trade()).unwrap();
    let want = decode(&csv(&plain), STEM, trade()).unwrap();
    assert_eq!((got.rows.clone(), got.trailing_empty), (want.rows, 4));
    assert_eq!((got.refused.total(), want.trailing_empty), (0, 0));
    for k in 1..=MAX_TRAILING_EMPTY {
        for header in [HEADER_OPEN_INTEREST, HEADER_OPEN_INTEREST_SPACED] {
            let rows: Vec<String> = many(2)
                .iter()
                .map(|r| format!("{r}{}", ",".repeat(k)))
                .collect();
            let got = decode(&body(header, k, &rows), STEM, trade()).unwrap();
            assert_eq!((got.rows.len(), usize::from(got.trailing_empty)), (2, k));
        }
    }
    let nine: Vec<String> = many(1).iter().map(|r| format!("{r},,,,,,,,,")).collect();
    assert_eq!(
        decode(&body(HEADER_OPEN_INTEREST, 9, &nine), STEM, trade()),
        Err(NfoRefusal::HeaderUnknown)
    );
    let mut broken = padded[..4].to_vec();
    broken[1] = plain[1].clone();
    broken[2] = format!("{},,,x,", plain[2]);
    broken[3] = format!("{},,,", plain[3]);
    let got = decode(&body(HEADER_OPEN_INTEREST, 4, &broken), STEM, trade()).unwrap();
    assert_eq!(
        shape(&got),
        (
            vec![2],
            vec![
                (3, RowFault::Malformed),
                (4, RowFault::Malformed),
                (5, RowFault::Malformed)
            ]
        )
    );
    // A non-empty "field" after the names, or a space, is no header.
    for bad in [
        format!("{HEADER_OPEN_INTEREST},x"),
        format!("{HEADER_OPEN_INTEREST}, "),
        format!("{HEADER_OPEN_INTEREST} ,"),
    ] {
        assert_eq!(
            decode(&body(&bad, 0, &padded[..1]), STEM, trade()),
            Err(NfoRefusal::HeaderUnknown),
            "{bad:?}"
        );
    }
}

/// Extremes at once (D-4816 with D-4829): the widest header admitted
/// (`k` = [`MAX_TRAILING_EMPTY`]), refused rows at the bound's exact edge
/// (one in every [`BAD_ROWS_PER`] of the rows read, past [`MAX_BAD_ROWS`]),
/// a duplicate and an out-of-order row first, and a complete last row with no
/// newline: every good row is kept in file order, every bad one counted, and
/// `k` is told. One more bad row refuses the file by the bound with both
/// counts; the same file cut between its last CR and LF is refused whole at
/// its last line; and a refused LAST row, where the newline is missing, is
/// still refused alone when its fault is not its shape.
#[test]
fn the_widest_header_at_the_bound_with_no_final_newline_keeps_every_good_row() {
    let k = MAX_TRAILING_EMPTY;
    let pad = ",".repeat(k);
    let over = MAX_BAD_ROWS + 1;
    let read = over * BAD_ROWS_PER;
    let file = |bad: u32, end: &[u8]| {
        let mut lines = many(read);
        let first = lines[0].clone();
        lines[1] = first.clone();
        lines[2] = row(STEM, trade(), 33_200, "10.10", 2, 301);
        for line in lines
            .iter_mut()
            .skip(3)
            .step_by(97)
            .take(usize::try_from(bad).unwrap())
        {
            *line = line.replacen(",1,300", ",-1,300", 1);
        }
        let mut text = format!("{HEADER_OPEN_INTEREST}{pad}\r\n");
        for line in &lines {
            text.push_str(line);
            text.push_str(&pad);
            text.push_str("\r\n");
        }
        let mut bytes = text.into_bytes();
        bytes.truncate(bytes.len() - 2);
        bytes.extend_from_slice(end);
        bytes
    };
    let got = decode(&file(over, b""), STEM, trade()).unwrap();
    let kept = usize::try_from(read - over).unwrap();
    assert_eq!(
        (
            got.rows.len(),
            got.refused.total(),
            usize::from(got.trailing_empty)
        ),
        (kept, over, k)
    );
    assert_eq!(got.refused.malformed, over);
    assert_eq!(got.refused.named.len(), crate::gdfl_cm::NAMED_ROWS);
    assert!(got.rows.windows(2).all(|pair| pair[0].line < pair[1].line));
    assert_eq!(
        (got.rows[0].sod, got.rows[1].sod, got.rows[2].sod),
        (33_300, 33_300, 33_200),
        "a duplicate and an earlier second are kept where the file has them"
    );
    assert_eq!(got.rows.last().unwrap().line, read + 1);
    assert_eq!(
        decode(&file(over + 1, b""), STEM, trade()),
        Err(NfoRefusal::TooManyBadRows {
            refused: over + 1,
            rows: read,
        })
    );
    assert_eq!(
        decode(&file(over, b"\r"), STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: read + 1 })
    );
    let crlf = decode(&file(over, b"\r\n"), STEM, trade()).unwrap();
    assert_eq!(crlf.rows, got.rows);
    let mut priced = file(over - 1, b"");
    let tail = format!(",10.05,0,0,0,0,1,300{pad}");
    assert!(priced.ends_with(tail.as_bytes()));
    let at = priced.len() - tail.len();
    priced.splice(at.., format!(",10.055,0,0,0,0,1,300{pad}").into_bytes());
    let last = decode(&priced, STEM, trade()).unwrap();
    assert_eq!(
        (last.refused.total(), last.refused.off_grid),
        (over, 1),
        "the last row's price, refused alone at the bound's edge"
    );
    assert_eq!(last.refused.named.first().map(|row| row.line), Some(5));
}

/// D-4816's bound IS the capital-market reader's (D-3318): a file may refuse
/// up to [`MAX_BAD_ROWS`] rows whatever its size, and more only while they
/// are at most one in every [`BAD_ROWS_PER`] of the rows read; past both the
/// file is refused whole, by name, with both counts. Every refused row is
/// counted, the first `NAMED_ROWS` named.
#[test]
fn too_many_bad_rows_refuse_an_options_file_at_the_capital_markets_bound() {
    let bent = |n: u32, bad: u32| {
        let mut lines = many(n);
        for line in lines.iter_mut().take(usize::try_from(bad).unwrap()) {
            *line = line.replacen(",1,300", ",x,300", 1);
        }
        csv(&lines)
    };
    let got = decode(&bent(MAX_BAD_ROWS + 1, MAX_BAD_ROWS), STEM, trade()).unwrap();
    assert_eq!((got.rows.len(), got.refused.total()), (1, MAX_BAD_ROWS));
    assert_eq!(got.refused.named.len(), crate::gdfl_cm::NAMED_ROWS);
    let over = MAX_BAD_ROWS + 1;
    let rows = over * BAD_ROWS_PER;
    let got = decode(&bent(rows, over), STEM, trade()).unwrap();
    assert_eq!(
        got.refused.total(),
        over,
        "one in every {BAD_ROWS_PER}: read"
    );
    assert_eq!(
        decode(&bent(rows - 1, over), STEM, trade()),
        Err(NfoRefusal::TooManyBadRows {
            refused: over,
            rows: rows - 1,
        })
    );
    let why = NfoRefusal::TooManyBadRows {
        refused: over,
        rows: rows - 1,
    }
    .to_string();
    assert!(
        why.contains(&format!("{over} of {} rows refused", rows - 1)),
        "{why}"
    );
    // Every row bad: the first row's own refusal names the file.
    assert_eq!(
        decode(&bent(3, 3), STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 2 })
    );
}

/// The row bound counts every row READ, kept or refused, as the
/// capital-market reader's does: refused rows cannot carry a file past it.
#[test]
fn the_row_bound_counts_refused_rows_too() {
    let mut lines = many(3);
    lines[0] = lines[0].replacen(",1,300", ",x,300", 1);
    let bytes = csv(&lines);
    assert_eq!(
        decode_capped(&bytes, STEM, trade(), 3).map(|f| f.rows.len()),
        Ok(2)
    );
    assert_eq!(
        decode_capped(&bytes, STEM, trade(), 2),
        Err(NfoRefusal::RowsOverCap { cap: 2 })
    );
}

/// Extreme shapes under per-row refusal: an empty file and a bare header;
/// one row, good or bad; duplicate and out-of-order rows, kept in file
/// order and never refused for it (placing them is the import's rule); a
/// refused row between two of one second.
#[test]
fn per_row_refusal_keeps_every_good_row_in_file_order_at_the_extremes() {
    assert_eq!(decode(b"", STEM, trade()), Err(NfoRefusal::HeaderUnknown));
    let bare = decode(&csv(&[]), STEM, trade()).unwrap();
    assert_eq!((bare.rows.len(), bare.refused.total()), (0, 0));
    let one = many(1);
    assert_eq!(decode(&csv(&one), STEM, trade()).unwrap().rows.len(), 1);
    let bad_one = vec![one[0].replacen(",1,300", ",-1,300", 1)];
    assert_eq!(
        decode(&csv(&bad_one), STEM, trade()),
        Err(NfoRefusal::MalformedRow { line: 2 })
    );
    let a = row(STEM, trade(), 33_310, "10.05", 1, 300);
    let b = row(STEM, trade(), 33_305, "10.10", 2, 301);
    let bad = a.replacen(",1,300", ",1,", 1);
    let file = vec![a.clone(), a.clone(), bad, b];
    let got = decode(&csv(&file), STEM, trade()).unwrap();
    assert_eq!(shape(&got), (vec![2, 3, 5], vec![(4, RowFault::Malformed)]));
    let sods: Vec<u32> = got.rows.iter().map(|row| row.sod).collect();
    assert_eq!(sods, [33_310, 33_310, 33_305]);
}

/// **D-4816 AND D-4829 ON THE MEASURED FILES, READ-ONLY.** The four shapes
/// the design measured on the real tick store (§8 F13, F23, F36, F38), each
/// read through this build's reader and the definition-4 oracle: 2018-11-09
/// `AXISBANK18NOV580PE` (a bad time at line 451 of 4,353 rows), 2018-11-06
/// `BANKNIFTY15NOV1826700CE` (a letter in one LTP at line 13,668 of
/// 13,991), every file of 2026-04-16 whose oracle answer is a refusal (the
/// ten negative-LTQ files), and 2025-03-11 `BANKNIFTY26JUN2546500CE` (four
/// empty trailing fields). Each file the oracle refused whole is read with
/// its bad rows named and every other row kept. Only counts and line
/// numbers are printed; no vendor row reaches the repository.
#[test]
#[ignore = "requires BRUTEX_GDFL_TICKSTORE (the tick store root); read-only"]
fn the_measured_bad_option_files_are_read_row_by_row() {
    let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
    let store = NfoTickStore::new(Path::new(&root));
    let named = [
        (day(2018, 11, 9), Some("AXISBANK18NOV580PE")),
        (day(2018, 11, 6), Some("BANKNIFTY15NOV1826700CE")),
        (day(2026, 4, 16), None),
        (day(2025, 3, 11), Some("BANKNIFTY26JUN2546500CE")),
    ];
    for (on, only) in named {
        let listing = store.day(on).unwrap().expect("a tick-store day");
        let mut recovered = 0_u32;
        for file in listing.entries() {
            let Some(name) = listing.ticker_of(file) else {
                continue;
            };
            if only.is_some_and(|wanted| wanted != name) {
                continue;
            }
            let bytes = store.fetch(file).unwrap();
            crate::gdfl_cm::verify(file, &bytes).unwrap();
            let stem = format!("{name}{ROW_SUFFIX}");
            let old = decode_by_field_vector(&bytes, &stem, on);
            if old.is_ok() && only.is_none() {
                continue;
            }
            let new = decode(&bytes, &stem, on);
            assert!(
                definition_5_differs_only_where_4_refused(&new, &old),
                "{on} {name}: {new:?} against {old:?}"
            );
            let got = new.unwrap_or_else(|why| panic!("{on} {name}: {why}"));
            let lines: Vec<String> = got
                .refused
                .named
                .iter()
                .map(|row| format!("{}:{}", row.line, row.fault.code()))
                .collect();
            eprintln!(
                "{on} {name}: definition 4 {}; definition 5 kept {} rows, refused {} ({}), trailing empty {}",
                old.as_ref().map_or_else(ToString::to_string, |f| format!(
                    "read {} rows",
                    f.rows.len()
                )),
                got.rows.len(),
                got.refused.total(),
                lines.join(" "),
                got.trailing_empty
            );
            recovered += 1;
        }
        assert!(recovered > 0, "{on}: no file of the measured shape found");
    }
}

/// The cost of per-row refusal, measured: one invented file of 4,353 rows
/// (2018-11-09's size) decoded n = 200 times clean, with one refused row and
/// with 64 (the bound), p50, p99 and max printed. Per-row refusal adds one
/// counter increment a refused row, so the three must stay within one
/// another's spread: the bound asserted is a generous 3x on p50. The three
/// files are decoded in turn inside each of the 200 rounds, so a burst of
/// load on a shared machine falls on all three alike.
#[test]
fn per_row_refusal_costs_what_a_clean_file_costs() {
    let lines = many(4_353);
    let bent = |bad: usize| {
        let mut lines = lines.clone();
        for line in lines.iter_mut().skip(1).step_by(60).take(bad) {
            *line = line.replacen(",1,300", ",-1,300", 1);
        }
        csv(&lines)
    };
    let files = [
        ("clean", bent(0), 0),
        ("one", bent(1), 1),
        ("bound", bent(64), 64),
    ];
    let mut nanos: [Vec<u128>; 3] = Default::default();
    for _ in 0..200 {
        for ((_, bytes, refused), spent) in files.iter().zip(nanos.iter_mut()) {
            let at = std::time::Instant::now();
            let got = decode(bytes, STEM, trade()).unwrap();
            spent.push(at.elapsed().as_nanos());
            assert_eq!(got.refused.total(), *refused);
        }
    }
    let mut p50s = Vec::new();
    for ((label, _, refused), spent) in files.iter().zip(nanos.iter_mut()) {
        spent.sort_unstable();
        let (p50, p99, max) = (spent[100], spent[198], spent[199]);
        eprintln!(
            "decode 4,353 rows, {label} ({refused} refused): n=200 p50={p50}ns p99={p99}ns max={max}ns"
        );
        p50s.push(p50);
    }
    assert!(p50s[2] < p50s[0].saturating_mul(3), "{p50s:?}");
}

/// D-4816 and design §8 F7, F8 ON REAL DAYS, READ-ONLY: every option file of
/// every tick-store day from 2018-09-03 to 2018-12-31, the months that hold
/// the newline-less `.NFO.CSV` days (§8 F8: 5,093 of 5,094 files on
/// 2018-10-03), ends in its LF or in its last row's last field, and none in
/// a bare CR, so the cut-between-CR-and-LF rule
/// (`an_option_file_cut_between_cr_and_lf_is_refused_whole`) refuses no file
/// those days hold. Only counts are printed; no vendor row reaches the
/// repository.
#[test]
#[ignore = "requires BRUTEX_GDFL_TICKSTORE (the tick store root); read-only"]
fn no_real_option_file_of_2018_ends_in_a_bare_cr() {
    let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
    let store = NfoTickStore::new(Path::new(&root));
    let (mut on, last) = (day(2018, 9, 3), day(2018, 12, 31));
    let (mut days, mut files, mut lf, mut bare_cr, mut no_newline) =
        (0_u32, 0_u64, 0_u64, 0_u64, 0_u64);
    while on <= last {
        if let Some(listing) = store.day(on).unwrap() {
            days += 1;
            let mut newline_less = 0_u64;
            for file in listing.entries() {
                let bytes = store.fetch(file).unwrap();
                crate::gdfl_cm::verify(file, &bytes).unwrap();
                files += 1;
                match bytes.last() {
                    Some(b'\n') => lf += 1,
                    Some(b'\r') => {
                        bare_cr += 1;
                        eprintln!("{on}: {} ends in a bare CR", file.entry);
                    }
                    _ => newline_less += 1,
                }
            }
            if newline_less > 0 {
                eprintln!("{on}: {newline_less} option files end with no newline");
            }
            no_newline += newline_less;
        }
        on = on.succ().unwrap();
    }
    eprintln!(
        "2018-09-03..2018-12-31: {days} days, {files} option files: {lf} end in LF, \
         {no_newline} in a row's last field, {bare_cr} in a bare CR"
    );
    assert!(
        days > 0 && files > 0,
        "the tick store holds the 2018 options days"
    );
    assert_eq!(bare_cr, 0, "a real file ends in a bare CR");
}

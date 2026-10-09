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

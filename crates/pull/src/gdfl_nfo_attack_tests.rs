#![cfg(test)]
//! Adversarial tests of the GDFL options reader's names, listings and rows
//! (round 1, area `gdfl-names`). Every ticker here is invented at run time
//! from a rule; none is quoted from a vendor file. The PRNG is splitmix64
//! with fixed seeds, so every run walks the same cases (CLAUDE.md §3 rule 5).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;
use std::time::Instant;

use super::*;
use brutex_core::universe::FNO_UNDERLYINGS;

fn d(y: u16, m: u8, dd: u8) -> Day {
    Day::new(y, m, dd).unwrap()
}

fn plus(day: Day, days: u32) -> Day {
    Day::from_days(day.days_from_epoch() + days).unwrap()
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn mon_name(month: u8) -> &'static str {
    MONTHS[usize::from(month) - 1]
}

/// The shortest decimal of a strike in paisa: `100`, `107.5`, `107.25`.
fn strike_text(paisa: i64) -> String {
    let (whole, frac) = (paisa / 100, paisa % 100);
    if frac == 0 {
        format!("{whole}")
    } else if frac % 10 == 0 {
        format!("{whole}.{}", frac / 10)
    } else {
        format!("{whole}.{frac:02}")
    }
}

/// The dated form, `UNDERLYING DD MON YY STRIKE CE|PE`, written canonically.
fn encode(under: &str, expiry: Day, paisa: i64, call: bool) -> String {
    format!(
        "{under}{:02}{}{:02}{}{}",
        expiry.day(),
        mon_name(expiry.month()),
        expiry.year() % 100,
        strike_text(paisa),
        if call { "CE" } else { "PE" }
    )
}

/// The store contract segment the dated form names.
fn segment(expiry: Day, paisa: i64, call: bool) -> String {
    format!(
        "{:04}-{:02}-{:02}-{paisa}-{}",
        expiry.year(),
        expiry.month(),
        expiry.day(),
        if call { "CE" } else { "PE" }
    )
}

/// Re-encodes a decoded ticker from its contract segment alone.
fn reencode(got: &OptionTicker) -> String {
    let seg = got.contract.as_str();
    let parts: Vec<&str> = seg.split('-').collect();
    assert_eq!(parts.len(), 5, "{seg}");
    let expiry = d(
        parts[0].parse().unwrap(),
        parts[1].parse().unwrap(),
        parts[2].parse().unwrap(),
    );
    let paisa: i64 = parts[3].parse().unwrap();
    encode(got.underlying.as_str(), expiry, paisa, parts[4] == "CE")
}

/// Whether a decoded ticker's name is the canonical spelling of its contract
/// except for trailing zeros in the strike's fraction (`100.0`, `107.50`).
fn reencodes(ticker: &str, got: &OptionTicker) -> bool {
    let again = reencode(got);
    if again == ticker {
        return true;
    }
    // The only admitted non-canonical spelling: a fraction padded with zeros.
    let (a, b) = (&ticker[..ticker.len() - 2], &again[..again.len() - 2]);
    a.starts_with(b) && {
        let tail = &a[b.len()..];
        let tail = tail.strip_prefix('.').unwrap_or(tail);
        !tail.is_empty() && tail.bytes().all(|c| c == b'0')
    }
}

fn first_weekday_on_or_after(day: Day) -> Day {
    let mut at = day;
    while !is_weekday(at) {
        at = at.succ().unwrap();
    }
    at
}

// ── the two readings of one name ───────────────────────────────────────────

/// A monthly-form name (`YY MON STRIKE`) whose strike begins with two digits
/// that, taken as a year, put a weekday inside the horizon: before the fix
/// the dated form claimed it, with the month's YEAR read as an expiry DAY and
/// the strike's first two digits read as the year.
#[test]
fn dpn_01_a_monthly_name_whose_strike_starts_with_a_year_is_never_a_dated_contract() {
    // 2019-12-18 is a Wednesday and 2023-12-18 a Monday, both inside
    // 2,200 days of 2018-12-03.
    for (ticker, on) in [
        ("ADANIENT18DEC195CE", d(2018, 12, 3)),
        ("BANKNIFTY18DEC23500CE", d(2018, 12, 3)),
        ("ACC18OCT2150PE", d(2018, 10, 1)),
    ] {
        assert_eq!(
            decode_ticker(ticker, on),
            Err(NfoRefusal::FormsAmbiguous {
                ticker: ticker.to_owned()
            }),
            "{ticker} on {on}"
        );
    }
}

/// Every monthly-form name of every trade month of the monthly era, for
/// every two-digit strike prefix: not one may decode to a contract.
#[test]
fn dpn_02_no_monthly_era_name_decodes_to_a_contract() {
    let tails = ["", "0", "5", "50", "2.5"];
    let unders = ["ADANIENT", "NIFTYNXT50", "360ONE", "M&M", "BAJAJ-AUTO"];
    let mut tried = 0_u64;
    let mut misread: Vec<String> = Vec::new();
    let mut check = |ticker: String, on: Day| {
        tried += 1;
        let got = decode_ticker(&ticker, on);
        if let Err(why) = &got {
            assert!(
                matches!(
                    why,
                    NfoRefusal::FormsAmbiguous { .. }
                        | NfoRefusal::MonthlyExpiryUnstated { .. }
                        | NfoRefusal::TickerUnparsed { .. }
                        | NfoRefusal::UnderlyingRefused { .. }
                ),
                "{ticker} on {on}: {why:?}"
            );
        }
        if let Ok(got) = got {
            if misread.len() < 8 {
                misread.push(format!("{ticker} on {on} -> {}", got.contract.as_str()));
            } else {
                misread.push(String::new());
            }
        }
    };
    for year in 2018_u16..=2019 {
        for month in 1_u8..=12 {
            let firsts = [
                first_weekday_on_or_after(d(year, month, 1)),
                d(year, month, 1).end_of_month(),
            ];
            for on in firsts {
                for ahead in 0_u8..3 {
                    let (cy, cm) = if month + ahead > 12 {
                        (year + 1, month + ahead - 12)
                    } else {
                        (year, month + ahead)
                    };
                    for under in unders {
                        for yy in 0..100 {
                            for tail in tails {
                                for side in ["CE", "PE"] {
                                    check(
                                        format!(
                                            "{under}{:02}{}{yy:02}{tail}{side}",
                                            cy % 100,
                                            mon_name(cm)
                                        ),
                                        on,
                                    );
                                }
                            }
                        }
                    }
                    for under in FNO_UNDERLYINGS {
                        for yy in 17..=26 {
                            check(
                                format!("{under}{:02}{}{yy}50CE", cy % 100, mon_name(cm)),
                                on,
                            );
                        }
                    }
                }
            }
        }
    }
    eprintln!(
        "dpn_02: {tried} monthly-era names, {} decoded to a contract",
        misread.len()
    );
    assert!(tried > 600_000, "{tried}");
    assert!(
        misread.is_empty(),
        "{} of {tried} monthly names misread, e.g. {:?}",
        misread.len(),
        &misread[..8.min(misread.len())]
    );
}

/// The era rule at its edges: one dated name whose `DD` is also a plausible
/// monthly year, on the last day of the monthly era and the first after it.
#[test]
fn dpn_03_the_era_cutover_is_the_trade_day_and_nothing_else() {
    // 2020-01-20 is a Monday; read as the monthly form it is January 2020,
    // strike 2,032,000, which a trade on 2019-12-31 could also be.
    let ticker = "BANKNIFTY20JAN2032000CE";
    assert!(
        matches!(
            decode_ticker(ticker, d(2019, 12, 31)),
            Err(NfoRefusal::FormsAmbiguous { .. })
        ),
        "inside the monthly era both readings stand"
    );
    let after = decode_ticker(ticker, d(2020, 1, 1)).unwrap();
    assert_eq!(after.contract.as_str(), "2020-01-20-3200000-CE");
    // Without a plausible monthly reading the dated form stands in the era
    // too: `07` is no year a 2019 trade could hold.
    assert_eq!(
        decode_ticker("BANKNIFTY07FEB1932000CE", d(2019, 2, 1))
            .unwrap()
            .contract
            .as_str(),
        "2019-02-07-3200000-CE"
    );
}

/// Every dated name of every underlying after the monthly era decodes to
/// exactly its contract and re-encodes to itself; inside the era it either
/// does so or is refused, never read as anything else.
#[test]
fn dpn_04_every_dated_name_round_trips_or_is_refused_by_name() {
    let offsets = [
        0_u32, 1, 3, 6, 7, 13, 30, 31, 90, 365, 366, 1_000, 2_199, 2_200,
    ];
    let strikes = [5_i64, 250, 7_750, 101_250, 2_200_000, 9_999_999_999];
    let mut tried = 0_u64;
    let mut refused_in_era = 0_u64;
    let mut on = d(2018, 9, 3);
    let last = d(2027, 12, 31);
    while on <= last {
        for &off in &offsets {
            let expiry = plus(on, off);
            if !is_weekday(expiry) {
                continue;
            }
            for under in FNO_UNDERLYINGS {
                for &paisa in &strikes {
                    for call in [true, false] {
                        tried += 1;
                        let name = encode(under, expiry, paisa, call);
                        match decode_ticker(&name, on) {
                            Ok(got) => {
                                assert_eq!(got.underlying.as_str(), under, "{name} on {on}");
                                assert_eq!(
                                    got.contract.as_str(),
                                    segment(expiry, paisa, call),
                                    "{name} on {on}"
                                );
                                assert_eq!(reencode(&got), name);
                            }
                            Err(why) => {
                                assert!(
                                    on.year() <= MONTHLY_FORM_LAST_YEAR,
                                    "{name} on {on}: {why}"
                                );
                                assert!(
                                    matches!(why, NfoRefusal::FormsAmbiguous { .. }),
                                    "{name} on {on}: {why}"
                                );
                                refused_in_era += 1;
                            }
                        }
                    }
                }
            }
        }
        on = plus(on, 11);
    }
    eprintln!(
        "dpn_04: {tried} dated names, {refused_in_era} refused as two-form in the monthly era"
    );
    assert!(tried > 1_000_000, "{tried}");
    assert!(refused_in_era < tried / 4, "{refused_in_era} of {tried}");
}

/// A strike with a leading zero is no exchange spelling, and before the fix
/// it let a monthly name's strike `2005` read as year 20, strike `05`.
#[test]
fn dpn_05_a_strike_with_a_leading_zero_is_refused() {
    let on = d(2024, 4, 1);
    for ticker in ["N04APR240100CE", "N04APR2400.5CE", "N04APR2401CE"] {
        let got = decode_ticker(ticker, on);
        assert!(got.is_err(), "{ticker}: {got:?}");
    }
    assert_eq!(strike_paisa(b"0100"), None);
    assert_eq!(strike_paisa(b"00.5"), None);
    assert_eq!(strike_paisa(b"0.5"), Some(50));
    assert_eq!(strike_paisa(b"10"), Some(1_000));
    // 2020-12-18 is a Friday: `18DEC2005` must not be strike 5 of that day.
    let got = decode_ticker("ACC18DEC2005CE", d(2018, 12, 3));
    assert!(got.is_err(), "{got:?}");
}

/// Expiry dates that do not exist, weekend expiries, an expired contract,
/// year ends and leap days.
#[test]
fn dpn_06_calendar_edges_of_the_stated_expiry() {
    let on = d(2024, 2, 1);
    // 2024-02-29 is a Thursday in a leap year; 2023-02-29 does not exist.
    assert_eq!(
        decode_ticker("X29FEB24100CE", on)
            .unwrap()
            .contract
            .as_str(),
        "2024-02-29-10000-CE"
    );
    for bad in [
        "X30FEB24100CE",
        "X31FEB24100CE",
        "X31APR24100CE",
        "X00JAN25100CE",
        "X32JAN25100CE",
    ] {
        let got = decode_ticker(bad, on);
        assert!(
            matches!(
                got,
                Err(NfoRefusal::TickerUnparsed { .. } | NfoRefusal::MonthlyExpiryUnstated { .. })
            ),
            "{bad}: {got:?}"
        );
    }
    // 2023-02-28 was a Tuesday: expired by 2024-02-01.
    assert!(decode_ticker("X28FEB23100CE", on).is_err());
    // A year-end trade day naming the next January.
    let eve = d(2024, 12, 31);
    assert_eq!(
        decode_ticker("X02JAN25100PE", eve)
            .unwrap()
            .contract
            .as_str(),
        "2025-01-02-10000-PE"
    );
    assert_eq!(
        decode_ticker("X31DEC24100PE", eve)
            .unwrap()
            .contract
            .as_str(),
        "2024-12-31-10000-PE"
    );
    assert!(
        decode_ticker("X31DEC23100PE", eve).is_err(),
        "one year expired"
    );
    // The trade day itself on a weekend still admits a weekday expiry ahead.
    assert!(decode_ticker("X02JAN25100PE", d(2024, 12, 28)).is_ok());
    // The last representable two-digit year.
    assert!(decode_ticker("X31DEC99100CE", d(2094, 1, 1)).is_ok());
    assert!(decode_ticker("X31DEC99100CE", d(9999, 12, 31)).is_err());
}

/// Lower case, whitespace, file suffixes, separators and stray bytes: each
/// refused by name, none read as a contract.
#[test]
fn dpn_07_spelling_faults_are_refused_by_name() {
    let on = d(2024, 4, 1);
    for ticker in [
        "nifty04APR2422000CE",
        "NIFTY04apr2422000CE",
        "NIFTY04APR2422000ce",
        " NIFTY04APR2422000CE",
        "NIFTY04APR2422000CE ",
        "NIFTY 04APR2422000CE",
        "NIFTY04APR24 22000CE",
        "NIFTY04APR2422000CE.NFO",
        "NIFTY04APR2422000CE.csv",
        "../NIFTY04APR2422000CE",
        "A/B04APR2422000CE",
        "A\\B04APR2422000CE",
        "NIFTY04APR2422000FUT",
        "NIFTY04APR24FUT",
        "NIFTY04APR2422,000CE",
        "NIFTY04APR24+22000CE",
        "NIFTY04APR24-22000CE",
        "NIFTY04APR2422000\u{0}CE",
        "NIFTY04APR2422000\u{e9}CE",
        "\u{e9}04APR2422000CE",
        "",
        "C",
        "E",
        "PE",
        "04APR2422000CE",
        "NIFTY2441122000CE",
        "NIFTY24O1022000CE",
        "NIFTY24APR22000CE",
    ] {
        let got = decode_ticker(ticker, on);
        assert!(got.is_err(), "{ticker:?} read as {got:?}");
        assert!(!got.unwrap_err().to_string().is_empty());
    }
    // A strike too long for any integer.
    let long = format!("N04APR24{}CE", "9".repeat(20));
    assert!(decode_ticker(&long, on).is_err());
    let long = format!("N04APR24{}.5CE", "9".repeat(20));
    assert!(decode_ticker(&long, on).is_err());
    // At the cap and one past it.
    let at_cap = format!("{}04APR24100CE", "N".repeat(TICKER_CAP - 12));
    assert_eq!(at_cap.len(), TICKER_CAP);
    assert!(matches!(
        decode_ticker(&at_cap, on),
        Err(NfoRefusal::UnderlyingRefused { .. })
    ));
    let past = format!("N{at_cap}");
    assert!(matches!(
        decode_ticker(&past, on),
        Err(NfoRefusal::TickerUnparsed { .. })
    ));
}

/// Every `F&O` underlying in both sides and every decimal strike shape.
#[test]
fn dpn_08_every_underlying_with_decimal_and_extreme_strikes() {
    let on = d(2024, 4, 1);
    let expiry = d(2024, 4, 25);
    let mut tried = 0;
    for under in FNO_UNDERLYINGS {
        for paisa in [
            1_i64,
            5,
            50,
            250,
            7_750,
            101_250,
            107_525,
            99,
            100,
            9_999_999_999,
        ] {
            for call in [true, false] {
                let name = encode(under, expiry, paisa, call);
                let got = decode_ticker(&name, on).unwrap();
                assert_eq!(
                    got.contract.as_str(),
                    segment(expiry, paisa, call),
                    "{name}"
                );
                assert_eq!(reencode(&got), name);
                tried += 1;
            }
        }
        let wide = encode(under, expiry, 10_000_000_000, true);
        assert!(
            matches!(
                decode_ticker(&wide, on),
                Err(NfoRefusal::ContractUnrenderable { .. })
            ),
            "{wide}"
        );
    }
    assert_eq!(tried, FNO_UNDERLYINGS.len() * 20);
    // A zero strike is no strike.
    assert!(decode_ticker("X25APR240CE", on).is_err());
    assert!(decode_ticker("X25APR240.0CE", on).is_err());
    assert!(decode_ticker("X25APR240.00CE", on).is_err());
}

// ── fuzz ────────────────────────────────────────────────────────────────────

const ALPHABET: &[u8] = b"0123456789000111222JANFEBMARAPRMAYJUNJULAUGSEPOCTNOVDECCEPE.&-_ aznXQ/\\";

fn random_ticker(rng: &mut Rng) -> String {
    let len = usize::try_from(rng.below(40)).unwrap();
    let mut out = String::new();
    // Half the cases start from a plausible shape and are then bent.
    if rng.below(2) == 0 {
        let under =
            FNO_UNDERLYINGS[usize::try_from(rng.below(FNO_UNDERLYINGS.len() as u64)).unwrap()];
        out.push_str(under);
        let dd = rng.below(34);
        let m = MONTHS[usize::try_from(rng.below(12)).unwrap()];
        let yy = rng.below(100);
        let strike = rng.below(1_000_000);
        match rng.below(4) {
            0 => write!(out, "{dd:02}{m}{yy:02}{strike}").unwrap(),
            1 => write!(out, "{yy:02}{m}{strike}").unwrap(),
            2 => write!(out, "{dd:02}{m}{yy:02}{strike}.{}", rng.below(100)).unwrap(),
            _ => write!(out, "{dd}{m}{yy}{strike}").unwrap(),
        }
        out.push_str(if rng.below(2) == 0 { "CE" } else { "PE" });
        for _ in 0..rng.below(3) {
            let at = usize::try_from(rng.below(out.len() as u64 + 1)).unwrap();
            let c =
                char::from(ALPHABET[usize::try_from(rng.below(ALPHABET.len() as u64)).unwrap()]);
            if out.is_char_boundary(at) {
                out.insert(at, c);
            }
        }
    } else {
        for _ in 0..len {
            if rng.below(50) == 0 {
                out.push('\u{2603}');
            } else {
                out.push(char::from(
                    ALPHABET[usize::try_from(rng.below(ALPHABET.len() as u64)).unwrap()],
                ));
            }
        }
        if rng.below(2) == 0 {
            out.push_str("CE");
        }
    }
    out
}

#[test]
fn dpn_09_a_million_random_names_never_panic_and_every_answer_is_exact_or_named() {
    let mut rng = Rng(0x6764_666c_6e61_6d65);
    let lo = d(2015, 1, 1).days_from_epoch();
    let hi = d(2027, 12, 31).days_from_epoch();
    let (mut ok, mut padded, mut err) = (0_u64, 0_u64, 0_u64);
    for _ in 0..1_000_000 {
        let ticker = random_ticker(&mut rng);
        let on =
            Day::from_days(lo + u32::try_from(rng.below(u64::from(hi - lo + 1))).unwrap()).unwrap();
        match decode_ticker(&ticker, on) {
            Ok(got) => {
                ok += 1;
                assert!(
                    reencodes(&ticker, &got),
                    "{ticker} on {on} -> {} ({})",
                    got.contract.as_str(),
                    reencode(&got)
                );
                if reencode(&got) != ticker {
                    padded += 1;
                }
                let seg = got.contract.as_str();
                let expiry = d(
                    seg[0..4].parse().unwrap(),
                    seg[5..7].parse().unwrap(),
                    seg[8..10].parse().unwrap(),
                );
                assert!(
                    expiry >= on
                        && expiry.days_from_epoch() <= on.days_from_epoch() + EXPIRY_HORIZON_DAYS
                );
                assert!(is_weekday(expiry));
                if on.year() <= MONTHLY_FORM_LAST_YEAR {
                    assert!(
                        !monthly_plausible(&ticker, on),
                        "{ticker} on {on}: the monthly era admits a name its monthly reading also fits"
                    );
                }
            }
            Err(why) => {
                err += 1;
                assert!(!why.to_string().is_empty());
                assert!(why.to_string().contains(ticker.as_str()) || ticker.is_empty());
            }
        }
    }
    assert_eq!(ok + err, 1_000_000);
    assert!(ok > 5_000, "the fuzz reaches the accepting path: {ok}");
    eprintln!("dpn_09: ok {ok} (fraction-padded {padded}), refused {err}");
}

/// Whether the monthly form reads `ticker` as a month a trade on `on` could
/// hold within the horizon. Written independently of the reader.
fn monthly_plausible(ticker: &str, on: Day) -> bool {
    let Some(body) = ticker
        .strip_suffix("CE")
        .or_else(|| ticker.strip_suffix("PE"))
    else {
        return false;
    };
    let b = body.as_bytes();
    for at in 1..b.len() {
        let rest = &b[at..];
        if rest.len() < 6 {
            continue;
        }
        let (Some(yy), Some(m)) = (two_digits(&rest[0..2]), month_of(&rest[2..5])) else {
            continue;
        };
        if strike_paisa(&rest[5..]).is_none() {
            continue;
        }
        let first = d(2000 + u16::from(yy), m, 1);
        if first.end_of_month() >= on
            && first.days_from_epoch() <= on.days_from_epoch() + EXPIRY_HORIZON_DAYS
        {
            return true;
        }
    }
    false
}

// ── helpers, exhaustively ───────────────────────────────────────────────────

#[test]
fn dpn_10_the_byte_helpers_over_their_whole_domains() {
    let mut digits = 0;
    for a in 0..=255_u8 {
        for b in 0..=255_u8 {
            let got = two_digits(&[a, b]);
            if a.is_ascii_digit() && b.is_ascii_digit() {
                assert_eq!(got, Some((a - b'0') * 10 + (b - b'0')));
                digits += 1;
            } else {
                assert_eq!(got, None);
            }
        }
    }
    assert_eq!(digits, 100);
    let mut months = 0;
    for a in b'A'..=b'z' {
        for b in b'A'..=b'z' {
            for c in b'A'..=b'z' {
                if let Some(m) = month_of(&[a, b, c]) {
                    assert_eq!(MONTHS[usize::from(m) - 1].as_bytes(), [a, b, c]);
                    months += 1;
                }
            }
        }
    }
    assert_eq!(months, 12);
    assert_eq!(month_of(b""), None);
    assert_eq!(month_of(b"JANU"), None);
    // Every HH:MM:SS of two-digit fields.
    let mut seconds = 0;
    for h in 0..100_u32 {
        for m in 0..100_u32 {
            for s in 0..100_u32 {
                let got = second_of_day(&format!("{h:02}:{m:02}:{s:02}"));
                if h <= 23 && m <= 59 && s <= 59 {
                    assert_eq!(got, Some(h * 3_600 + m * 60 + s));
                    seconds += 1;
                } else {
                    assert_eq!(got, None);
                }
            }
        }
    }
    assert_eq!(seconds, 86_400);
    for bad in [
        "",
        "09:15",
        "09:15:00:",
        "09-15-00",
        "+9:15:00",
        "-9:15:00",
        "09:15:0\u{e9}",
        " 9:15:00",
        "0915:00",
    ] {
        assert_eq!(second_of_day(bad), None, "{bad}");
    }
}

// ── listings ────────────────────────────────────────────────────────────────

#[test]
fn dpn_11_entry_names_that_try_to_leave_the_options_folder_file_nothing() {
    let day = d(2024, 4, 1);
    let folder = day_folder_name(day);
    for entry in [
        format!("{folder}\\Options\\..\\X04APR24100CE.NFO.csv"),
        format!("{folder}/Options/../X04APR24100CE.NFO.csv"),
        format!("{folder}\\Options\\..\\..\\X.NFO.csv"),
        format!("/{folder}\\Options\\X04APR24100CE.NFO.csv"),
        format!("C:\\{folder}\\Options\\X04APR24100CE.NFO.csv"),
        format!("x/{folder}\\Options\\X04APR24100CE.NFO.csv"),
        format!("{folder}\\\\Options\\X04APR24100CE.NFO.csv"),
        format!("{folder}\\Options\\\\X04APR24100CE.NFO.csv"),
        format!("{folder}\\options\\X04APR24100CE.NFO.csv"),
        format!("{folder}\\Options\\X04APR24100CE.nfo.csv"),
        format!("{folder}\\Options\\X04APR24100CE.NFO.csv\u{0}"),
        format!("{folder}\\Options\\X04APR24100CE.NFO.csv/"),
        format!("{folder}0\\Options\\X04APR24100CE.NFO.csv"),
    ] {
        assert_eq!(entry_ticker(&folder, &entry), None, "{entry}");
    }
    // `..` as a whole ticker stays a name and never decodes.
    let dots = format!("{folder}\\Options\\...NFO.csv");
    assert_eq!(entry_ticker(&folder, &dots), Some(".."));
    assert!(decode_ticker("..", day).is_err());
    // Mixed separators are one ticker, and a ticker listed both ways is a
    // duplicate, never two files.
    let mut listing: NfoDay<u32> = NfoDay::new(day);
    listing.push(&format!("{folder}/Options\\X04APR24100CE.NFO.csv"), 1, 1, 0);
    listing.push(&format!("{folder}\\Options/X04APR24100CE.NFO.CSV"), 1, 1, 1);
    listing.push(
        &format!("{folder}\\Options\\Y04APR24100CE.NFO.csv"),
        1,
        1,
        2,
    );
    assert!(matches!(
        listing.locate("X04APR24100CE"),
        Err(NfoRefusal::DuplicateTicker { .. })
    ));
    assert_eq!(listing.locate("Y04APR24100CE").unwrap().unwrap().locator, 2);
    // A ticker listed three times is still one refusal and the others answer.
    listing.push(
        &format!("{folder}\\Options\\X04APR24100CE.NFO.csv"),
        1,
        1,
        3,
    );
    assert!(matches!(
        listing.locate("X04APR24100CE"),
        Err(NfoRefusal::DuplicateTicker { .. })
    ));
    assert_eq!(listing.locate("Z04APR24100CE"), Ok(None));
}

/// `locate` is documented as one hash probe. Before the fix it scanned every
/// duplicate name the day held first, so its cost grew with the listing.
#[test]
fn dpn_12_locate_does_not_grow_with_the_duplicates_a_day_holds() {
    let day = d(2024, 4, 1);
    let folder = day_folder_name(day);
    let build = |dups: usize| {
        let mut listing: NfoDay<u32> = NfoDay::new(day);
        for i in 0..dups {
            for k in 0..2_u32 {
                listing.push(
                    &format!("{folder}\\Options\\D{i}A04APR24100CE.NFO.csv"),
                    1,
                    1,
                    k,
                );
            }
        }
        listing.push(
            &format!("{folder}\\Options\\GOOD04APR24100CE.NFO.csv"),
            1,
            1,
            9,
        );
        listing
    };
    let median = |listing: &NfoDay<u32>| {
        let mut samples: Vec<u128> = (0..2_001)
            .map(|_| {
                let t = Instant::now();
                let got = listing.locate("GOOD04APR24100CE");
                let ns = t.elapsed().as_nanos();
                assert_eq!(got.unwrap().unwrap().locator, 9);
                ns
            })
            .collect();
        samples.sort_unstable();
        samples[samples.len() / 2]
    };
    let small = median(&build(1));
    let large = median(&build(100_000));
    eprintln!("dpn_12: locate median {small} ns with 1 duplicate, {large} ns with 100000");
    assert!(
        large < small.max(50) * 40,
        "locate grows with duplicates: {small} ns -> {large} ns"
    );
}

// ── rows ────────────────────────────────────────────────────────────────────

const STEM: &str = "NIFTY04APR2422000CE.NFO";

fn good_row(sod: u32) -> String {
    format!(
        "{STEM},01/04/2024,{:02}:{:02}:{:02},10.05,10,75,10.1,50,75,300",
        sod / 3_600,
        sod / 60 % 60,
        sod % 60
    )
}

#[test]
fn dpn_13_a_hundred_thousand_bent_files_never_panic_and_refuse_at_a_line() {
    let mut rng = Rng(0x726f_7773_6466_6c31);
    let on = d(2024, 4, 1);
    let mutate: [fn(&mut Rng, String) -> String; 9] = [
        |_, r| r.replace(',', ",,"),
        |_, r| r.replacen("10.05", "10.055", 1),
        |_, r| r.replacen("10.05", "", 1),
        |_, r| r.replacen(",300", ",-300", 1),
        |_, r| format!("{r}\r"),
        |_, r| r.replacen("01/04/2024", "1/4/2024", 1),
        |g, r| {
            let mut b = r.into_bytes();
            let at = usize::try_from(g.below(b.len() as u64)).unwrap();
            let byte = u8::try_from(g.below(256)).unwrap();
            b[at] = if byte == b'\n' { b'x' } else { byte };
            String::from_utf8_lossy(&b).into_owned()
        },
        |_, r| r.replacen(",75,300", ",0,300", 1).replacen("10.05", "0", 1),
        |_, r| r,
    ];
    let (mut ok, mut refused) = (0, 0);
    for _ in 0..100_000 {
        let n = usize::try_from(rng.below(6)).unwrap();
        let mut lines = vec![HEADER_OPEN_INTEREST.to_owned()];
        let mut bent_at = None;
        for i in 0..n {
            let mut line = good_row(33_300 + u32::try_from(i).unwrap());
            if rng.below(4) == 0 {
                let which = usize::try_from(rng.below(mutate.len() as u64)).unwrap();
                line = mutate[which](&mut rng, line);
                bent_at.get_or_insert(i + 2);
            }
            lines.push(line);
        }
        let sep = if rng.below(2) == 0 { "\r\n" } else { "\n" };
        let mut text = lines.join(sep);
        if rng.below(2) == 0 {
            text.push_str(sep);
        }
        match decode_capped(text.as_bytes(), STEM, on, 4) {
            Ok(file) => {
                ok += 1;
                assert!(file.rows.len() <= 4);
                for (k, row) in file.rows.iter().enumerate() {
                    assert_eq!(row.line, u32::try_from(k + 2).unwrap());
                    assert!(row.ltp >= 0 && row.oi >= 0);
                    assert!(row.ltq == 0 || row.ltp >= costs::rate::TICK.raw());
                    assert!(row.sod < 86_400);
                }
            }
            Err(why) => {
                refused += 1;
                match why {
                    NfoRefusal::RowsOverCap { cap } => assert_eq!((cap, n), (4, n.max(5))),
                    NfoRefusal::HeaderUnknown => panic!("the header was intact"),
                    NfoRefusal::TickerMismatch { line }
                    | NfoRefusal::DateMismatch { line }
                    | NfoRefusal::MalformedRow { line }
                    | NfoRefusal::PriceRefused { line }
                    | NfoRefusal::OpenInterestRefused { line } => {
                        let at = usize::try_from(line).unwrap();
                        assert!(at >= 2 && at <= n + 2, "line {line} of {n} rows");
                        if let Some(first) = bent_at {
                            assert!(
                                at >= first,
                                "refused line {at} before the first bent line {first}"
                            );
                        }
                    }
                    other => panic!("{other:?}"),
                }
            }
        }
    }
    assert!(ok > 1_000 && refused > 1_000, "{ok} {refused}");
}

// ── cost ────────────────────────────────────────────────────────────────────

/// `decode_ticker` per call over tickers of growing length up to the cap:
/// p50/p99/max printed, and the median at the cap bounded against the median
/// of the shortest.
#[test]
fn dpn_14_decode_ticker_cost_is_bounded_by_the_cap() {
    let on = d(2024, 4, 1);
    let mut medians = Vec::new();
    for under_len in [1_usize, 8, 24, 48] {
        let accepted = format!("{}04APR2422000CE", "N".repeat(under_len.min(24)));
        let refused = format!("{}04APR2422000CE", "n".repeat(under_len));
        let mut samples: Vec<u128> = Vec::with_capacity(20_000);
        for i in 0..20_000 {
            let name = if i % 2 == 0 { &accepted } else { &refused };
            let t = Instant::now();
            let got = decode_ticker(name, on);
            samples.push(t.elapsed().as_nanos());
            assert_eq!(got.is_ok(), i % 2 == 0, "{name}");
        }
        samples.sort_unstable();
        let (p50, p99, max) = (samples[10_000], samples[19_800], samples[19_999]);
        eprintln!(
            "dpn_14: underlying length {under_len}: p50 {p50} ns, p99 {p99} ns, max {max} ns"
        );
        medians.push(p50);
    }
    assert!(medians[3] < medians[0].max(100) * 30, "{medians:?}");
}

/// `locate` over listings of 10^3 to 10^6 entries: p50/p99/max per probe,
/// printed, and the p50 of the largest bounded against the smallest's.
#[test]
fn dpn_15_locate_per_probe_cost_over_listing_size() {
    let day = d(2024, 4, 1);
    let folder = day_folder_name(day);
    let mut p50s = Vec::new();
    for size in [1_000_usize, 10_000, 100_000, 1_000_000] {
        let mut listing: NfoDay<usize> = NfoDay::new(day);
        for i in 0..size {
            listing.push(
                &format!("{folder}\\Options\\U{i}04APR24100CE.NFO.csv"),
                1,
                1,
                i,
            );
        }
        let mut rng = Rng(u64::try_from(size).unwrap());
        let names: Vec<String> = (0..10_000)
            .map(|_| format!("U{}04APR24100CE", rng.below(u64::try_from(size).unwrap())))
            .collect();
        let mut samples: Vec<u128> = Vec::with_capacity(names.len());
        for name in &names {
            let t = Instant::now();
            let got = listing.locate(name);
            samples.push(t.elapsed().as_nanos());
            assert!(got.unwrap().is_some(), "{name}");
        }
        samples.sort_unstable();
        let (p50, p99, max) = (samples[5_000], samples[9_900], samples[9_999]);
        eprintln!("dpn_15: {size} entries: p50 {p50} ns, p99 {p99} ns, max {max} ns");
        p50s.push(p50);
    }
    assert!(p50s[3] < p50s[0].max(100) * 30, "{p50s:?}");
}

/// Random entry names never panic, and whatever they file is a bare name.
#[test]
fn dpn_16_random_entry_names_file_only_bare_names() {
    let day = d(2024, 4, 1);
    let folder = day_folder_name(day);
    let parts = [
        folder.as_str(),
        "\\",
        "/",
        "Options",
        "..",
        ".NFO.csv",
        ".NFO.CSV",
        "X04APR24100CE",
        "",
        "\u{e9}",
        "C:",
    ];
    let mut rng = Rng(0x656e_7472_6965_7331);
    let mut filed = 0;
    for _ in 0..200_000 {
        let mut entry = format!("{folder}\\Options\\X04APR24100CE.NFO.csv");
        for _ in 0..rng.below(3) {
            let at = usize::try_from(rng.below(entry.len() as u64 + 1)).unwrap();
            if !entry.is_char_boundary(at) {
                continue;
            }
            if rng.below(2) == 0 {
                entry.insert_str(
                    at,
                    parts[usize::try_from(rng.below(parts.len() as u64)).unwrap()],
                );
            } else {
                let mut end = at + usize::try_from(rng.below(4)).unwrap();
                while end < entry.len() && !entry.is_char_boundary(end) {
                    end += 1;
                }
                entry.replace_range(at..end.min(entry.len()), "");
            }
        }
        if let Some(ticker) = entry_ticker(&folder, &entry) {
            filed += 1;
            assert!(
                !ticker.is_empty() && !ticker.contains(['\\', '/']),
                "{entry:?}"
            );
            assert!(entry.starts_with(&folder), "{entry:?}");
            assert!(
                entry.ends_with(&format!("{ticker}.NFO.csv"))
                    || entry.ends_with(&format!("{ticker}.NFO.CSV")),
                "{entry:?}"
            );
        }
    }
    assert!(filed > 100, "{filed}");
}

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

/// Whether a decoded ticker's name is the canonical spelling of its
/// contract: since D-3161 refuses padded fractions, the only spelling.
fn reencodes(ticker: &str, got: &OptionTicker) -> bool {
    reencode(got) == ticker
}

fn first_weekday_on_or_after(day: Day) -> Day {
    let mut at = day;
    while !is_weekday(at) {
        at = at.succ().unwrap();
    }
    at
}

// ── the two readings of one name, and the era rule (D-3160) ────────────────

/// The first trade day of the dated form.
fn cutover() -> Day {
    let (y, m, dd) = DATED_FORM_FROM;
    d(y, m, dd)
}

fn is_index(under: &str) -> bool {
    DATED_BEFORE_CUTOVER.into_iter().any(|i| i == under)
}

/// Monthly-form names whose strike begins with two digits that, taken as a
/// year, put a weekday inside the horizon: before D-3160 the dated form
/// claimed them. Before the cutover a share's name is the monthly form only,
/// read with the sourced day (D-3165), and an index's name that reads both
/// ways is refused loudly.
#[test]
fn dpn_01_a_monthly_name_whose_strike_starts_with_a_year_is_never_a_dated_contract() {
    // 2019-12-18 is a Wednesday and 2023-12-18 a Monday, both inside
    // 2,200 days of 2018-12-03; 2021-10-18 is a Monday.
    for (ticker, on, monthly) in [
        ("ADANIENT18DEC195CE", d(2018, 12, 3), "2018-12-27-19500-CE"),
        ("ACC18OCT2150PE", d(2018, 10, 1), "2018-10-25-215000-PE"),
        (
            "TV18BRDCST18DEC195CE",
            d(2018, 12, 3),
            "2018-12-27-19500-CE",
        ),
        ("TV18BRDCST18DEC50PE", d(2018, 12, 3), "2018-12-27-5000-PE"),
    ] {
        assert_eq!(
            decode_ticker(ticker, on).unwrap().contract.as_str(),
            monthly,
            "{ticker} on {on}"
        );
    }
    let ticker = "BANKNIFTY18DEC23500CE";
    assert_eq!(
        decode_ticker(ticker, d(2018, 12, 3)),
        Err(NfoRefusal::FormsAmbiguous {
            ticker: ticker.to_owned()
        })
    );
}

/// Calls `check` with every monthly-form name of `dpn_02`'s sweep and its
/// trade day: every trade month 2018-01 to the cutover, its first weekday
/// and its last day, contract month +0..+2, eight underlyings x 100
/// two-digit strike prefixes x five tails x two sides, plus every F&O
/// underlying x yy 17..26.
fn each_monthly_name(check: &mut impl FnMut(String, Day)) {
    let tails = ["", "0", "5", "50", "2.5"];
    let unders = [
        "ADANIENT",
        "NIFTYNXT50",
        "360ONE",
        "M&M",
        "BAJAJ-AUTO",
        "TV18BRDCST",
        "NIFTY",
        "BANKNIFTY",
    ];
    let mut month_start = d(2018, 1, 1);
    while month_start < cutover() {
        let (year, month) = (month_start.year(), month_start.month());
        for on in [
            first_weekday_on_or_after(month_start),
            month_start.end_of_month(),
        ] {
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
        month_start = month_start.end_of_month().succ().unwrap();
    }
}

/// Every monthly-form name of every trade month before the cutover, for
/// every two-digit strike prefix. Before D-3165 not one decoded; now a name
/// whose month is in `MONTHLY_EXPIRIES` and alive decodes to exactly that
/// month's sourced day, and every other answer is a refusal by name. Not one
/// may decode to any other contract (`misread`), and not one that the
/// reference reads may be refused other than as two-form (`missed`).
#[test]
fn dpn_02_no_monthly_name_before_the_cutover_decodes_to_another_contract() {
    let mut tried = 0_u64;
    let (mut decoded, mut dated, mut unstated, mut expired, mut ambiguous, mut other) =
        (0_u64, 0_u64, 0_u64, 0_u64, 0_u64, 0_u64);
    let mut misread: Vec<String> = Vec::new();
    let mut missed: Vec<String> = Vec::new();
    let mut check = |ticker: String, on: Day| {
        tried += 1;
        let reference = monthly_reference(&ticker, on);
        match decode_ticker(&ticker, on) {
            Ok(got) => {
                let pair = (
                    got.underlying.as_str().to_owned(),
                    got.contract.as_str().to_owned(),
                );
                if reference.as_ref() == Some(&pair) {
                    decoded += 1;
                } else if is_index(got.underlying.as_str())
                    && on >= d(2018, 9, 3)
                    && reencodes(&ticker, &got)
                    && !monthly_plausible(&ticker, on)
                {
                    // An index weekly's dated reading with no live monthly
                    // reading: the name is not this sweep's monthly name.
                    dated += 1;
                } else {
                    misread.push(format!("{ticker} on {on} -> {}", got.contract.as_str()));
                }
            }
            Err(why) => {
                assert!(why.to_string().contains(ticker.as_str()), "{why}");
                match why {
                    NfoRefusal::MonthlyExpiryUnstated { .. } => unstated += 1,
                    NfoRefusal::ExpiryRefused { .. } => expired += 1,
                    NfoRefusal::FormsAmbiguous { .. } => ambiguous += 1,
                    NfoRefusal::TickerUnparsed { .. } | NfoRefusal::UnderlyingRefused { .. } => {
                        other += 1;
                    }
                    why => panic!("{ticker} on {on}: {why:?}"),
                }
                if reference.is_some() && !matches!(why, NfoRefusal::FormsAmbiguous { .. }) {
                    missed.push(format!("{ticker} on {on}: {why:?}"));
                }
            }
        }
    };
    each_monthly_name(&mut check);
    eprintln!(
        "dpn_02: {tried} monthly names before the cutover: {} misread, {} missed, {decoded} decoded to the sourced day, {dated} index names read as dated (no live monthly reading), {unstated} MonthlyExpiryUnstated, {expired} ExpiryRefused, {ambiguous} FormsAmbiguous, {other} other refusals",
        misread.len(),
        missed.len()
    );
    assert!(tried > 500_000, "{tried}");
    assert!(decoded > 100_000, "the sourced months decode: {decoded}");
    assert!(
        misread.is_empty(),
        "{} of {tried} monthly names misread, e.g. {:?}",
        misread.len(),
        &misread[..8.min(misread.len())]
    );
    assert!(
        missed.is_empty(),
        "{} of {tried} monthly names the table reads were refused, e.g. {:?}",
        missed.len(),
        &missed[..8.min(missed.len())]
    );
}

/// The strike text `text` in paisa if it is the canonical spelling
/// `strike_text` writes, written independently of the reader.
fn canonical_strike(text: &str) -> Option<i64> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || whole.len() > 15 {
        return None;
    }
    if !frac.bytes().all(|b| b.is_ascii_digit()) || frac.len() > 2 {
        return None;
    }
    let mut paisa: i64 = whole.parse::<i64>().ok()?.checked_mul(100)?;
    match frac.len() {
        1 => paisa += frac.parse::<i64>().ok()? * 10,
        2 => paisa += frac.parse::<i64>().ok()?,
        _ => {}
    }
    (paisa > 0 && strike_text(paisa) == text).then_some(paisa)
}

/// The monthly reading of `ticker` traded on `on`, written independently of
/// the reader: `(underlying, segment)` when some split reads `YY MON
/// STRIKE` with a canonical strike, the month is in `MONTHLY_EXPIRIES` and
/// its day lies in `[on, on + horizon]`.
fn monthly_reference(ticker: &str, on: Day) -> Option<(String, String)> {
    let (body, call) = match ticker.strip_suffix("CE") {
        Some(body) => (body, true),
        None => (ticker.strip_suffix("PE")?, false),
    };
    let b = body.as_bytes();
    for at in 1..b.len() {
        let rest = &b[at..];
        if rest.len() < 6 || !rest[0].is_ascii_digit() || !rest[1].is_ascii_digit() {
            continue;
        }
        let year = 2000 + u16::from(rest[0] - b'0') * 10 + u16::from(rest[1] - b'0');
        let Some(month) = (1_u8..=12)
            .zip(MONTHS)
            .find_map(|(n, m)| (m.as_bytes() == &rest[2..5]).then_some(n))
        else {
            continue;
        };
        let Some(paisa) = std::str::from_utf8(&rest[5..])
            .ok()
            .and_then(canonical_strike)
        else {
            continue;
        };
        let mut sourced = None;
        for ((y, m), (ey, em, ed)) in MONTHLY_EXPIRIES {
            if (y, m) == (year, month) {
                sourced = Some(d(ey, em, ed));
            }
        }
        let expiry = sourced?;
        if expiry < on || expiry.days_from_epoch() > on.days_from_epoch() + EXPIRY_HORIZON_DAYS {
            return None;
        }
        let under = std::str::from_utf8(&b[..at]).ok()?.to_owned();
        return Some((under, segment(expiry, paisa, call)));
    }
    None
}

/// The cutover pinned on both sides, for a share (`ADANIENT`, `TV18BRDCST`),
/// for the index weeklies and for an index name that reads both ways.
#[test]
fn dpn_03_the_era_cutover_is_trade_day_2019_02_01() {
    assert_eq!(DATED_FORM_FROM, (2019, 2, 1));
    let (eve, day) = (d(2019, 1, 31), d(2019, 2, 1));
    assert_eq!(eve.succ().unwrap(), day);
    // A share's name read both ways: monthly (Feb 2019, strike 195, its
    // sourced day 2019-02-28) on the eve, dated (2019-02-19, a Tuesday,
    // strike 5) from the cutover.
    for ticker in ["ADANIENT19FEB195CE", "TV18BRDCST19FEB195CE"] {
        assert_eq!(
            decode_ticker(ticker, eve).unwrap().contract.as_str(),
            "2019-02-28-19500-CE",
            "{ticker} on {eve}"
        );
        let got = decode_ticker(ticker, day).unwrap();
        assert_eq!(got.contract.as_str(), "2019-02-19-500-CE", "{ticker}");
    }
    assert_eq!(
        decode_ticker("TV18BRDCST19FEB195CE", day)
            .unwrap()
            .underlying
            .as_str(),
        "TV18BRDCST"
    );
    // A share's dated weekly-looking name before the cutover is not dated.
    assert!(decode_ticker("ADANIENT07FEB19195CE", eve).is_err());
    assert_eq!(
        decode_ticker("ADANIENT07FEB19195CE", day)
            .unwrap()
            .contract
            .as_str(),
        "2019-02-07-19500-CE"
    );
    // An index weekly is dated on both sides: `07FEB` is no live month.
    for on in [eve, day] {
        assert_eq!(
            decode_ticker("NIFTY07FEB1911000CE", on)
                .unwrap()
                .contract
                .as_str(),
            "2019-02-07-1100000-CE",
            "{on}"
        );
        assert_eq!(
            decode_ticker("BANKNIFTY07FEB1927000PE", on)
                .unwrap()
                .contract
                .as_str(),
            "2019-02-07-2700000-PE",
            "{on}"
        );
    }
    // An index name both readings fit (2019-02-19 dated; Feb 2019 monthly,
    // strike 1,927,000): refused on the eve, dated from the cutover.
    let both = "BANKNIFTY19FEB1927000CE";
    assert!(matches!(
        decode_ticker(both, eve),
        Err(NfoRefusal::FormsAmbiguous { .. })
    ));
    assert_eq!(
        decode_ticker(both, day).unwrap().contract.as_str(),
        "2019-02-19-2700000-CE"
    );
    // An index monthly name before the cutover is the monthly form.
    assert_eq!(
        decode_ticker("NIFTY19JAN11000CE", d(2019, 1, 2))
            .unwrap()
            .contract
            .as_str(),
        "2019-01-31-1100000-CE"
    );
    // From the cutover only the dated form is read: a monthly name read as
    // dated states 2012-03-19, which is refused as an expiry (D-3164).
    assert!(matches!(
        decode_ticker("ADANIENT19MAR1280CE", day),
        Err(NfoRefusal::ExpiryRefused { .. })
    ));
    assert_eq!(
        decode_ticker("ADANIENT19MAR1280CE", eve)
            .unwrap()
            .contract
            .as_str(),
        "2019-03-28-128000-CE"
    );
}

/// The second edge of the era rule, and the dated shape with a date that is
/// not one (D-3164).
#[test]
fn dpn_03b_the_index_weeklies_are_dated_from_2018_09_03() {
    let day = cutover();
    // The index weeklies are dated from 2018-09-03 and not before; the
    // census starts there, so the day before reads them as nothing dated.
    assert_eq!(INDEX_WEEKLY_DATED_FROM, (2018, 9, 3));
    assert_eq!(
        decode_ticker("NIFTY06SEP1811500CE", d(2018, 9, 3))
            .unwrap()
            .contract
            .as_str(),
        "2018-09-06-1150000-CE"
    );
    assert!(decode_ticker("NIFTY06SEP1811500CE", d(2018, 8, 31)).is_err());
    assert_eq!(
        decode_ticker("BANKNIFTY06SEP1827000PE", d(2018, 9, 3))
            .unwrap()
            .contract
            .as_str(),
        "2018-09-06-2700000-PE"
    );
    assert!(decode_ticker("BANKNIFTY06SEP1827000PE", d(2018, 8, 31)).is_err());
    // From the cutover a dated shape with a weekend or impossible date is
    // refused as such, not as a monthly name (D-3164); 2019-02-02 is a
    // Saturday.
    for bad in [
        "NIFTY02FEB1911000CE",
        "NIFTY30FEB1911000CE",
        "NIFTY31JAN1911000CE",
    ] {
        assert!(
            matches!(
                decode_ticker(bad, day),
                Err(NfoRefusal::ExpiryRefused { .. })
            ),
            "{bad}"
        );
    }
    // TV18BRDCST's own digits never start a reading.
    assert_eq!(
        decode_ticker("TV18BRDCST27JUN1950CE", d(2019, 6, 3))
            .unwrap()
            .contract
            .as_str(),
        "2019-06-27-5000-CE"
    );
}

/// Every dated name of every underlying: from the cutover it decodes to
/// exactly its contract and re-encodes to itself; before it, a share's name
/// is the monthly form, read with the sourced day when its month is in
/// `MONTHLY_EXPIRIES` (D-3165) and refused otherwise, and an index's is read
/// exactly or refused as two-form. Never read as anything else.
#[test]
fn dpn_04_every_dated_name_round_trips_or_is_refused_by_name() {
    let offsets = [
        0_u32, 1, 3, 6, 7, 13, 30, 31, 90, 365, 366, 1_000, 2_199, 2_200,
    ];
    let strikes = [5_i64, 250, 7_750, 101_250, 2_200_000, 9_999_999_999];
    let (mut tried, mut before_share, mut before_index_refused) = (0_u64, 0_u64, 0_u64);
    let mut before_share_monthly = 0_u64;
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
                            Ok(got) if on < cutover() && !is_index(under) => {
                                assert_eq!(
                                    monthly_reference(&name, on),
                                    Some((
                                        got.underlying.as_str().to_owned(),
                                        got.contract.as_str().to_owned()
                                    )),
                                    "{name} on {on}: a share before the cutover is monthly"
                                );
                                before_share_monthly += 1;
                            }
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
                                assert!(on < cutover(), "{name} on {on}: {why}");
                                if is_index(under) {
                                    assert!(
                                        matches!(why, NfoRefusal::FormsAmbiguous { .. }),
                                        "{name} on {on}: {why}"
                                    );
                                    before_index_refused += 1;
                                } else {
                                    before_share += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        on = plus(on, 11);
    }
    eprintln!(
        "dpn_04: {tried} dated names, 0 misread; before the cutover {before_share_monthly} share names read as the monthly form with the sourced day, {before_share} share names refused (the monthly era) and {before_index_refused} index names refused as two-form"
    );
    assert!(tried > 1_000_000, "{tried}");
}

/// A strike with a leading zero is no exchange spelling, and before the fix
/// it let a monthly name's strike `2005` read as year 20, strike `05`.
#[test]
fn dpn_05_a_strike_with_a_leading_zero_or_a_padded_fraction_is_refused() {
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
    // It is December 2018, strike 2,005, its sourced day (D-3165).
    let got = decode_ticker("ACC18DEC2005CE", d(2018, 12, 3)).unwrap();
    assert_eq!(got.contract.as_str(), "2018-12-27-200500-CE");
    // A fraction ending in zero is a second spelling and refused; decimal
    // strikes that end in a non-zero digit read (D-3161).
    for padded in [
        "N04APR24100.0CE",
        "N04APR24107.50PE",
        "N04APR24100.00CE",
        "N04APR24202.50CE",
    ] {
        assert!(
            matches!(
                decode_ticker(padded, on),
                Err(NfoRefusal::TickerUnparsed { .. })
            ),
            "{padded}"
        );
    }
    for (frac, paisa) in [
        (&b"100.0"[..], None),
        (b"107.50", None),
        (b"100.00", None),
        (b"0.50", None),
    ] {
        assert_eq!(strike_paisa(frac), paisa);
    }
    assert_eq!(
        decode_ticker("N04APR24202.5CE", on)
            .unwrap()
            .contract
            .as_str(),
        "2024-04-04-20250-CE"
    );
    assert_eq!(
        decode_ticker("N04APR24107.25PE", on)
            .unwrap()
            .contract
            .as_str(),
        "2024-04-04-10725-PE"
    );
    assert_eq!(
        decode_ticker("N04APR241012.5CE", on)
            .unwrap()
            .contract
            .as_str(),
        "2024-04-04-101250-CE"
    );
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
                Err(NfoRefusal::TickerUnparsed { .. }
                    | NfoRefusal::MonthlyExpiryUnstated { .. }
                    | NfoRefusal::ExpiryRefused { .. })
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
    let (mut ok, mut err, mut monthly_ok) = (0_u64, 0_u64, 0_u64);
    for _ in 0..1_000_000 {
        let ticker = random_ticker(&mut rng);
        let on =
            Day::from_days(lo + u32::try_from(rng.below(u64::from(hi - lo + 1))).unwrap()).unwrap();
        match decode_ticker(&ticker, on) {
            Ok(got)
                if on < cutover()
                    && monthly_reference(&ticker, on)
                        == Some((
                            got.underlying.as_str().to_owned(),
                            got.contract.as_str().to_owned(),
                        )) =>
            {
                // The monthly form with its sourced day (D-3165); the
                // reference already holds the expiry inside the window.
                ok += 1;
                monthly_ok += 1;
            }
            Ok(got) => {
                ok += 1;
                assert!(
                    reencodes(&ticker, &got),
                    "{ticker} on {on} -> {} ({})",
                    got.contract.as_str(),
                    reencode(&got)
                );
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
                if on < cutover() {
                    assert!(
                        on >= d(2018, 9, 3),
                        "{ticker} on {on}: dated before the index weeklies"
                    );
                    assert!(
                        is_index(got.underlying.as_str()),
                        "{ticker} on {on}: a share is dated before the cutover"
                    );
                    assert!(
                        !monthly_plausible(&ticker, on),
                        "{ticker} on {on}: an index name both readings fit was read"
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
    eprintln!(
        "dpn_09: ok {ok} (of which {monthly_ok} monthly with the sourced day), refused {err}"
    );
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

/// The two measured headers read, CRLF or LF; any other first line is
/// refused by name, including near misses and a byte-order mark.
#[test]
fn dpn_17_exactly_the_two_measured_headers_read() {
    assert_eq!(
        HEADER_OPEN_INTEREST,
        "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest"
    );
    assert_eq!(
        HEADER_OPEN_INTEREST_SPACED,
        "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,Open Interest"
    );
    let on = d(2024, 4, 1);
    let row = good_row(33_300);
    for header in [HEADER_OPEN_INTEREST, HEADER_OPEN_INTEREST_SPACED] {
        for sep in ["\r\n", "\n"] {
            let text = format!("{header}{sep}{row}{sep}");
            let got = decode(text.as_bytes(), STEM, on).unwrap();
            assert_eq!(got.rows.len(), 1, "{header:?}");
        }
    }
    for bad in [
        format!("\u{feff}{HEADER_OPEN_INTEREST}"),
        format!("{HEADER_OPEN_INTEREST} "),
        format!(" {HEADER_OPEN_INTEREST}"),
        HEADER_OPEN_INTEREST.replace("OpenInterest", "Open interest"),
        HEADER_OPEN_INTEREST.replace("OpenInterest", "Open  Interest"),
        HEADER_OPEN_INTEREST.replace("OpenInterest", "OI"),
        HEADER_OPEN_INTEREST.to_lowercase(),
        HEADER_OPEN_INTEREST.replace(',', "\t"),
        HEADER_OPEN_INTEREST.replace(",OpenInterest", ""),
        format!("{HEADER_OPEN_INTEREST},"),
        "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ".to_owned(),
        String::new(),
    ] {
        let text = format!("{bad}\r\n{row}\r\n");
        assert_eq!(
            decode(text.as_bytes(), STEM, on),
            Err(NfoRefusal::HeaderUnknown),
            "{bad:?}"
        );
    }
}

/// Both measured extensions (`.csv` and the 2018 `.CSV`) are filed; no other
/// case of either part is.
#[test]
fn dpn_18_exactly_the_two_measured_extensions_are_filed() {
    let day = d(2018, 9, 3);
    let folder = day_folder_name(day);
    for ext in [".NFO.csv", ".NFO.CSV"] {
        let entry = format!("{folder}\\Options\\NIFTY06SEP1811500CE{ext}");
        assert_eq!(
            entry_ticker(&folder, &entry),
            Some("NIFTY06SEP1811500CE"),
            "{ext}"
        );
    }
    for ext in [
        ".NFO.Csv",
        ".NFO.cSV",
        ".nfo.csv",
        ".nfo.CSV",
        ".Nfo.csv",
        ".NFO.csv ",
        ".NFO.CSV.",
        ".NFO",
        ".csv",
    ] {
        let entry = format!("{folder}\\Options\\NIFTY06SEP1811500CE{ext}");
        assert_eq!(entry_ticker(&folder, &entry), None, "{ext:?}");
    }
    let mut listing: NfoDay<u8> = NfoDay::new(day);
    listing.push(
        &format!("{folder}\\Options\\NIFTY06SEP1811500CE.NFO.CSV"),
        1,
        1,
        0,
    );
    listing.push(
        &format!("{folder}\\Options\\NIFTY06SEP1811500PE.NFO.csv"),
        1,
        1,
        1,
    );
    listing.push(
        &format!("{folder}\\Options\\NIFTY06SEP1811500PE.nfo.csv"),
        1,
        1,
        2,
    );
    assert_eq!(
        listing
            .locate("NIFTY06SEP1811500CE")
            .unwrap()
            .unwrap()
            .locator,
        0
    );
    assert_eq!(
        listing
            .locate("NIFTY06SEP1811500PE")
            .unwrap()
            .unwrap()
            .locator,
        1
    );
    assert_eq!(listing.skipped(), 1);
}

/// One pre-cutover NIFTY monthly contract spelled both ways on one day:
/// since D-3165 both spellings decode to the SAME contract (its monthly
/// reading has the sourced day 2019-01-31; the dated one states it), so a
/// day holding both is refused by `gdfl_import` as `TickerAmbiguous`, never
/// merged and never two contracts. Within the horizon the dated spelling's
/// own monthly reading is live and refused as two-form.
#[test]
fn dpn_19_a_pre_cutover_index_monthly_spelled_both_ways_on_one_day() {
    let on = d(2019, 1, 15);
    let (monthly, dated) = ("NIFTY19JAN10500CE", "NIFTY31JAN1910500CE");
    let one = decode_ticker(monthly, on).unwrap();
    assert!(
        !monthly_plausible(dated, on),
        "January 2031 is past the horizon"
    );
    let two = decode_ticker(dated, on).unwrap();
    assert_eq!(two.underlying.as_str(), "NIFTY");
    assert_eq!(two.contract.as_str(), "2019-01-31-1050000-CE");
    assert_eq!(one, two, "both spellings name one contract");
    // Within the horizon the dated spelling's monthly reading is live, and
    // it is refused by name instead (`24JAN19` -> January 2024).
    let near = "NIFTY24JAN1910500CE";
    assert!(monthly_plausible(near, on));
    assert_eq!(
        decode_ticker(near, on),
        Err(NfoRefusal::FormsAmbiguous {
            ticker: near.to_owned()
        })
    );
}

// ── the sourced monthly expiry table (D-3165) ──────────────────────────────

/// The table is in month order, one row per month, and each day is the last
/// Thursday of its month: NSE's OPTIDX rule says the expiry is that day or,
/// on a holiday, the trading day before it, and the census found these.
#[test]
fn dpn_20_the_monthly_table_is_ordered_and_each_day_is_its_months_last_thursday() {
    let mut last = (0_u16, 0_u8);
    for ((year, month), (ey, em, ed)) in MONTHLY_EXPIRIES {
        assert!((year, month) > last, "{year}-{month} out of order");
        last = (year, month);
        assert_eq!((ey, em), (year, month), "the day lies in its month");
        let expiry = d(ey, em, ed);
        // 1970-01-01, day 0, was a Thursday.
        assert_eq!(expiry.days_from_epoch() % 7, 0, "{expiry} is a Thursday");
        assert!(
            plus(expiry, 7).month() != em,
            "{expiry} is the month's last Thursday"
        );
        assert_eq!(monthly_expiry(year, month), Some(expiry));
    }
    assert_eq!(MONTHLY_EXPIRIES.len(), 15);
}

/// One sourced month: `ticker` traded on `on` (before the cutover, so the
/// monthly form applies) decodes to `segment`; on the expiry day itself it
/// still decodes; the next weekday after the expiry it is refused by name.
fn table_month(month: (u16, u8), ticker: &str, on: Day, want: &str) {
    assert!(on < cutover(), "{ticker}: traded in the monthly era");
    let (year, m) = month;
    let expiry = monthly_expiry(year, m).unwrap();
    assert_eq!(
        decode_ticker(ticker, on).unwrap().contract.as_str(),
        want,
        "{ticker} on {on}"
    );
    assert!(
        want.starts_with(&format!("{expiry}-")),
        "{want} names {expiry}"
    );
    if expiry < cutover() {
        assert_eq!(
            decode_ticker(ticker, expiry).unwrap().contract.as_str(),
            want,
            "{ticker} on its expiry day"
        );
    }
    let after = first_weekday_on_or_after(expiry.succ().unwrap());
    let got = decode_ticker(ticker, after);
    assert!(
        matches!(
            got,
            Err(NfoRefusal::ExpiryRefused { .. } | NfoRefusal::TickerUnparsed { .. })
        ),
        "{ticker} on {after}, after its expiry: {got:?}"
    );
    if after < cutover() {
        assert_eq!(
            got,
            Err(NfoRefusal::ExpiryRefused {
                ticker: ticker.to_owned()
            }),
            "{ticker} on {after}: the monthly form, expired"
        );
    }
}

#[test]
fn dpn_21_2018_09_decodes_to_2018_09_27() {
    table_month(
        (2018, 9),
        "ACC18SEP1500CE",
        d(2018, 9, 3),
        "2018-09-27-150000-CE",
    );
}

#[test]
fn dpn_21_2018_10_decodes_to_2018_10_25() {
    table_month(
        (2018, 10),
        "ACC18OCT1280PE",
        d(2018, 10, 1),
        "2018-10-25-128000-PE",
    );
}

#[test]
fn dpn_21_2018_11_decodes_to_2018_11_29() {
    table_month(
        (2018, 11),
        "TV18BRDCST18NOV50CE",
        d(2018, 11, 1),
        "2018-11-29-5000-CE",
    );
}

#[test]
fn dpn_21_2018_12_decodes_to_2018_12_27() {
    table_month(
        (2018, 12),
        "ADANIENT18DEC195CE",
        d(2018, 12, 3),
        "2018-12-27-19500-CE",
    );
}

#[test]
fn dpn_21_2019_01_decodes_to_2019_01_31() {
    table_month(
        (2019, 1),
        "NIFTY19JAN10500CE",
        d(2019, 1, 15),
        "2019-01-31-1050000-CE",
    );
}

#[test]
fn dpn_21_2019_02_decodes_to_2019_02_28() {
    table_month(
        (2019, 2),
        "BANKNIFTY19FEB27000PE",
        d(2019, 1, 15),
        "2019-02-28-2700000-PE",
    );
}

#[test]
fn dpn_21_2019_03_decodes_to_2019_03_28() {
    table_month(
        (2019, 3),
        "M&M19MAR700CE",
        d(2019, 1, 31),
        "2019-03-28-70000-CE",
    );
}

#[test]
fn dpn_21_2019_06_decodes_to_2019_06_27() {
    table_month(
        (2019, 6),
        "NIFTY19JUN11000CE",
        d(2018, 12, 3),
        "2019-06-27-1100000-CE",
    );
}

#[test]
fn dpn_21_2019_09_decodes_to_2019_09_26() {
    table_month(
        (2019, 9),
        "NIFTY19SEP11500PE",
        d(2018, 12, 3),
        "2019-09-26-1150000-PE",
    );
}

#[test]
fn dpn_21_2019_12_decodes_to_2019_12_26() {
    table_month(
        (2019, 12),
        "BANKNIFTY19DEC28000CE",
        d(2019, 1, 15),
        "2019-12-26-2800000-CE",
    );
}

#[test]
fn dpn_21_2020_06_decodes_to_2020_06_25() {
    table_month(
        (2020, 6),
        "NIFTY20JUN12000CE",
        d(2018, 12, 3),
        "2020-06-25-1200000-CE",
    );
}

#[test]
fn dpn_21_2020_12_decodes_to_2020_12_31() {
    table_month(
        (2020, 12),
        "NIFTY20DEC10000PE",
        d(2018, 12, 3),
        "2020-12-31-1000000-PE",
    );
}

#[test]
fn dpn_21_2021_06_decodes_to_2021_06_24() {
    table_month(
        (2021, 6),
        "NIFTY21JUN12500CE",
        d(2019, 1, 15),
        "2021-06-24-1250000-CE",
    );
}

#[test]
fn dpn_21_2021_12_decodes_to_2021_12_30() {
    table_month(
        (2021, 12),
        "NIFTY21DEC13000CE",
        d(2019, 1, 15),
        "2021-12-30-1300000-CE",
    );
}

#[test]
fn dpn_21_2022_12_decodes_to_2022_12_29() {
    table_month(
        (2022, 12),
        "NIFTY22DEC14000PE",
        d(2019, 1, 31),
        "2022-12-29-1400000-PE",
    );
}

/// A month the table does not list stays refused by name, for a share, an
/// index near month and an index long-dated month, whatever the trade day.
#[test]
fn dpn_22_a_month_absent_from_the_table_is_still_refused_by_name() {
    for (ticker, on) in [
        ("ACC18AUG1280PE", d(2018, 8, 1)),
        ("ACC18JUL1280PE", d(2018, 6, 1)),
        ("NIFTY18AUG11000CE", d(2018, 8, 1)),
        ("NIFTY19APR11000CE", d(2019, 1, 15)),
        ("NIFTY19MAY11000CE", d(2019, 1, 15)),
        ("NIFTY23JUN11000CE", d(2019, 1, 15)),
        ("NIFTY20MAR11000CE", d(2018, 12, 3)),
        ("BANKNIFTY17DEC25000PE", d(2017, 12, 1)),
    ] {
        assert_eq!(
            decode_ticker(ticker, on),
            Err(NfoRefusal::MonthlyExpiryUnstated {
                ticker: ticker.to_owned()
            }),
            "{ticker} on {on}"
        );
        assert!(monthly_reference(ticker, on).is_none());
    }
    // Every month from 2015 to 2026 the table does not list, for a share:
    // never a contract.
    for year in 15_u16..=26 {
        for month in 1_u8..=12 {
            if monthly_expiry(2000 + year, month).is_some() {
                continue;
            }
            let ticker = format!("ACC{year:02}{}1280PE", mon_name(month));
            let on = d(2018, 1, 1);
            assert!(
                matches!(
                    decode_ticker(&ticker, on),
                    Err(NfoRefusal::MonthlyExpiryUnstated { .. })
                ),
                "{ticker}"
            );
        }
    }
}

/// With monthly names now decoding, the index two-form rule still refuses a
/// name whose dated reading is live while its monthly month overlaps the
/// window, even when the table says that month has already expired: an
/// expired monthly traded is a vendor fault, never a reason to take the
/// other reading. A name with only one live reading is read.
#[test]
fn dpn_23_an_index_name_both_forms_read_is_still_refused_when_monthly_names_decode() {
    // Both live: 2019-02-19 (a Tuesday) dated; February 2019, day 2019-02-28.
    let both = "BANKNIFTY19FEB1927000CE";
    for on in [d(2018, 12, 3), d(2019, 1, 15), d(2019, 1, 31)] {
        assert_eq!(
            decode_ticker(both, on),
            Err(NfoRefusal::FormsAmbiguous {
                ticker: both.to_owned()
            }),
            "{on}"
        );
    }
    // Dated 2023-12-18 (a Monday) is live; December 2018 has expired by
    // 2018-12-28 by the table, but the month still holds the trade day.
    let stale = "NIFTY18DEC2311000CE";
    assert_eq!(
        decode_ticker(stale, d(2018, 12, 28)),
        Err(NfoRefusal::FormsAmbiguous {
            ticker: stale.to_owned()
        })
    );
    // Only the monthly reading is live (dated 2010-01-19 is past).
    assert_eq!(
        decode_ticker("NIFTY19JAN10500CE", d(2019, 1, 15))
            .unwrap()
            .contract
            .as_str(),
        "2019-01-31-1050000-CE"
    );
    // Only the dated reading is live (its monthly reading, January 2031,
    // lies past the horizon).
    assert_eq!(
        decode_ticker("NIFTY31JAN1910500CE", d(2019, 1, 15))
            .unwrap()
            .contract
            .as_str(),
        "2019-01-31-1050000-CE"
    );
    // A share is never read both ways: its name is the monthly form only.
    assert_eq!(
        decode_ticker("ADANIENT19FEB1927000CE", d(2019, 1, 15))
            .unwrap()
            .contract
            .as_str(),
        "2019-02-28-192700000-CE"
    );
}

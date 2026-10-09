//! Adversarial attack on the decoders `core` owns: rupee text and rupee floats
//! into paisa, ISIN check digits, the fixed-width symbol, the expiry, the
//! store's contract segment, the vendor instrument-master row decoder and the
//! F&O universe probes.
//!
//! Every property test draws from a fixed-seed splitmix64, so a rerun is byte
//! identical (`CLAUDE.md` §3 rule 5). Every case asserts a value or a named
//! refusal; a panic anywhere is itself the failure.
//!
//! The `DPD-` invariants and D-3150..D-3158 are the decisions these pin.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_arithmetic,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use brutex_core::error::{InstrumentError, PriceError};
use brutex_core::instrument::{
    CONTRACT_CAPACITY, Contract, Exchange, Expiry, InstrumentKey, Kind, OptionSide, Segment,
};
use brutex_core::isin::Isin;
use brutex_core::price::{MAX_PRICE_TEXT, Paisa};
use brutex_core::symbol::{SYMBOL_CAPACITY, Symbol};
use brutex_core::universe::{
    FNO_INDEX, FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS, NIFTY_TOTAL_MARKET, nse_isin, of_equity,
};
use brutex_core::vendor::{Decoded, MasterRow, Skip, Vendor, decode_master_row};

// ---------------------------------------------------------------------------
// The PRNG. splitmix64, fixed seeds, no dependency.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        items.get(self.below(items.len())).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Rupee TEXT -> paisa.
// ---------------------------------------------------------------------------

/// An independent reading of the same contract, written a different way: the
/// fraction past two places is compared with "5000..." as text, and the whole
/// value is held in `i128` so nothing here can overflow before it is checked.
fn reference_text(text: &str) -> Result<i64, PriceError> {
    if text.len() > MAX_PRICE_TEXT {
        return Err(PriceError::TooLong);
    }
    let t = text.trim_matches(|c: char| c.is_ascii_whitespace());
    let (neg, body) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let mut dots = 0;
    for b in body.bytes() {
        if b == b'.' {
            dots += 1;
        } else if !b.is_ascii_digit() {
            return Err(PriceError::NotDecimal);
        }
    }
    if dots > 1 || body.len() == dots {
        return Err(PriceError::NotDecimal);
    }
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    let mut magnitude: i128 = 0;
    for b in whole.bytes() {
        magnitude = magnitude * 10 + i128::from(b - b'0');
        if magnitude > i128::from(i64::MAX) {
            return Err(PriceError::OutOfRange);
        }
    }
    let f = frac.as_bytes();
    let d = |i: usize| i128::from(f.get(i).map_or(0, |b| b - b'0'));
    magnitude = magnitude * 100 + d(0) * 10 + d(1);
    let tail = frac.get(2..).unwrap_or("");
    // Compare the tail with exactly one half: "5" then zeros.
    let tail_trimmed = tail.trim_end_matches('0');
    let ordering = if tail_trimmed.is_empty() {
        std::cmp::Ordering::Less
    } else {
        tail_trimmed.cmp("5")
    };
    let up = match ordering {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => !neg,
        std::cmp::Ordering::Less => false,
    };
    magnitude += i128::from(up);
    let signed = if neg { -magnitude } else { magnitude };
    i64::try_from(signed)
        .ok()
        .filter(|v| *v != i64::MIN)
        .ok_or(PriceError::OutOfRange)
}

#[test]
fn dpd_price_text_hand_picked_values_answer_exactly_or_refuse_by_name() {
    let max = i64::MAX;
    let cases: [(&str, Result<i64, PriceError>); 40] = [
        ("0", Ok(0)),
        ("-0", Ok(0)),
        ("+0", Ok(0)),
        ("0.00", Ok(0)),
        ("-0.00", Ok(0)),
        ("0.005", Ok(1)),
        ("0.015", Ok(2)),
        ("-0.005", Ok(0)),
        ("-0.015", Ok(-1)),
        ("-0.0051", Ok(-1)),
        ("0.0049999999", Ok(0)),
        ("1.005", Ok(101)),
        ("+12.5", Ok(1250)),
        ("12.", Ok(1200)),
        (".5", Ok(50)),
        (" 12.5\t", Ok(1250)),
        ("000000000000000000000000000000000000000012.50", Ok(1250)),
        ("92233720368547758.07", Ok(max)),
        ("-92233720368547758.07", Ok(-max)),
        ("92233720368547758.074", Ok(max)),
        ("92233720368547758.075", Err(PriceError::OutOfRange)),
        ("92233720368547758.08", Err(PriceError::OutOfRange)),
        ("-92233720368547758.08", Err(PriceError::OutOfRange)),
        ("99999999999999999999", Err(PriceError::OutOfRange)),
        ("1e3", Err(PriceError::NotDecimal)),
        ("NaN", Err(PriceError::NotDecimal)),
        ("inf", Err(PriceError::NotDecimal)),
        ("-inf", Err(PriceError::NotDecimal)),
        ("12,345.50", Err(PriceError::NotDecimal)),
        ("1_000", Err(PriceError::NotDecimal)),
        ("0x10", Err(PriceError::NotDecimal)),
        ("", Err(PriceError::NotDecimal)),
        (".", Err(PriceError::NotDecimal)),
        ("-", Err(PriceError::NotDecimal)),
        ("+", Err(PriceError::NotDecimal)),
        ("--1", Err(PriceError::NotDecimal)),
        ("+-1", Err(PriceError::NotDecimal)),
        ("1.2.3", Err(PriceError::NotDecimal)),
        ("\u{a0}12.5", Err(PriceError::NotDecimal)),
        ("\u{ff11}\u{ff12}", Err(PriceError::NotDecimal)),
    ];
    for (text, want) in cases {
        assert_eq!(
            Paisa::from_rupee_text_half_up(text).map(Paisa::raw),
            want,
            "{text:?}"
        );
        assert_eq!(reference_text(text), want, "the reference agrees: {text:?}");
    }
    // The length bound is inclusive at 64 and refuses 65, padding included.
    let at = format!("{}1", " ".repeat(MAX_PRICE_TEXT - 1));
    assert_eq!(Paisa::from_rupee_text_half_up(&at).map(Paisa::raw), Ok(100));
    let past = format!("{}1", " ".repeat(MAX_PRICE_TEXT));
    assert_eq!(
        Paisa::from_rupee_text_half_up(&past),
        Err(PriceError::TooLong)
    );
    assert_eq!(
        Paisa::from_rupee_text_half_up(&"9".repeat(1000)),
        Err(PriceError::TooLong)
    );
}

/// Every sign, every whole rupee 0..=199 and every fraction of zero to four
/// places: 3 x 200 x 11,111 = 6,666,600 strings, each against the reference.
#[test]
fn dpd_price_text_exhaustive_small_domain_agrees_with_the_reference() {
    let mut checked = 0_u64;
    for sign in ["", "+", "-"] {
        for whole in 0..200_u32 {
            for places in 0..=4_u32 {
                for frac in 0..10_u32.pow(places) {
                    let text = if places == 0 {
                        format!("{sign}{whole}")
                    } else {
                        format!("{sign}{whole}.{frac:0width$}", width = places as usize)
                    };
                    let got = Paisa::from_rupee_text_half_up(&text).map(Paisa::raw);
                    // floor(x + 1/2) in units of 1e-4 rupee: half-up for both signs.
                    let units =
                        i64::from(whole) * 10_000 + i64::from(frac) * 10_i64.pow(4 - places);
                    let units = if sign == "-" { -units } else { units };
                    let want = (units + 50).div_euclid(100);
                    assert_eq!(got, Ok(want), "{text}");
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 3 * 200 * 11_111);
}

/// 1,000,000 random strings over the alphabet a price column could plausibly
/// carry plus the ones it should never: never a panic, and the answer always
/// equals the independent reference, refusal reasons included.
#[test]
fn dpd_price_text_fuzz_never_panics_and_agrees_with_the_reference() {
    const ALPHABET: [&str; 24] = [
        "0",
        "1",
        "2",
        "5",
        "9",
        "4",
        "7",
        ".",
        ".",
        "-",
        "+",
        " ",
        "\t",
        "e",
        "E",
        ",",
        "_",
        "\u{a0}",
        "\u{0660}",
        "\u{ff15}",
        "N",
        "a",
        "00",
        "999999999",
    ];
    let mut rng = Rng(0x00D3_C0DE_0001);
    let mut ok = 0_u64;
    let mut refused = 0_u64;
    for _ in 0..1_000_000 {
        let len = rng.below(14);
        let mut text = String::new();
        for _ in 0..len {
            text.push_str(rng.pick(&ALPHABET));
        }
        let got = Paisa::from_rupee_text_half_up(&text).map(Paisa::raw);
        assert_eq!(got, reference_text(&text), "{text:?}");
        match got {
            Ok(p) => {
                ok += 1;
                assert_ne!(p, i64::MIN, "never the null sentinel");
            }
            Err(_) => refused += 1,
        }
    }
    assert!(
        ok > 10_000 && refused > 10_000,
        "both arms drawn: {ok} {refused}"
    );
}

/// Any representable paisa count rendered as two-place rupee text reads back
/// to itself, with or without leading zeros, trailing zeros or a `+`.
#[test]
fn dpd_price_text_round_trips_every_rendered_paisa() {
    let mut rng = Rng(0x00D3_C0DE_0002);
    let mut extremes = vec![0, 1, -1, 99, -99, 100, -100, i64::MAX, -i64::MAX];
    for _ in 0..1_000_000 {
        let raw = rng.next() as i64;
        let shift = rng.below(63);
        extremes.push(raw >> shift);
    }
    for p in extremes {
        if p == i64::MIN {
            continue;
        }
        let sign = if p < 0 { "-" } else { "" };
        let m = p.unsigned_abs();
        let plain = format!("{sign}{}.{:02}", m / 100, m % 100);
        let padded = format!("{sign}000{}.{:02}000", m / 100, m % 100);
        assert_eq!(
            Paisa::from_rupee_text_half_up(&plain).map(Paisa::raw),
            Ok(p)
        );
        assert_eq!(
            Paisa::from_rupee_text_half_up(&padded).map(Paisa::raw),
            Ok(p)
        );
        if p >= 0 {
            let plus = format!("+{plain}");
            assert_eq!(Paisa::from_rupee_text_half_up(&plus).map(Paisa::raw), Ok(p));
        }
    }
}

// ---------------------------------------------------------------------------
// Rupee FLOAT -> paisa.
// ---------------------------------------------------------------------------

#[test]
fn dpd_price_float_extremes_are_exact_or_refused_by_name() {
    let cases: [(f64, Result<i64, PriceError>); 14] = [
        (f64::NAN, Err(PriceError::NotFinite)),
        (f64::INFINITY, Err(PriceError::NotFinite)),
        (f64::NEG_INFINITY, Err(PriceError::NotFinite)),
        (-0.0, Ok(0)),
        (0.0, Ok(0)),
        (f64::MIN_POSITIVE, Ok(0)),
        (-f64::MIN_POSITIVE, Ok(0)),
        (f64::from_bits(1), Ok(0)),
        (f64::EPSILON, Ok(0)),
        (f64::MAX, Err(PriceError::OutOfRange)),
        (f64::MIN, Err(PriceError::OutOfRange)),
        (0.125, Ok(13)),
        (-0.125, Ok(-12)),
        (-92_233_720_368_547_758.08, Err(PriceError::OutOfRange)),
    ];
    for (rupees, want) in cases {
        assert_eq!(
            Paisa::from_rupees_half_up(rupees).map(Paisa::raw),
            want,
            "{rupees:e}"
        );
    }
}

/// 1,000,000 random bit patterns: never a panic, a non-finite input is always
/// `NotFinite`, and an accepted one is within half a paisa of the scaled
/// value (the scaled product is itself rounded, so the bound is checked on
/// that product, which is what the function promises).
#[test]
fn dpd_price_float_fuzz_is_within_half_a_paisa_or_refused() {
    let mut rng = Rng(0x00D3_C0DE_0003);
    let mut ok = 0_u64;
    for i in 0..1_000_000_u64 {
        // Half raw bit patterns, half values in the range a quote can occupy.
        let x = if i % 2 == 0 {
            f64::from_bits(rng.next())
        } else {
            let mantissa = (rng.next() >> 11) as f64 / (1_u64 << 53) as f64;
            let exponent = rng.below(40) as i32 - 10;
            let sign = if rng.next() & 1 == 0 { 1.0 } else { -1.0 };
            sign * mantissa * 10_f64.powi(exponent)
        };
        match Paisa::from_rupees_half_up(x) {
            Ok(p) => {
                ok += 1;
                assert!(x.is_finite());
                assert_ne!(p.raw(), i64::MIN, "never the null sentinel");
                let scaled = x * 100.0;
                let diff = p.raw() as f64 - scaled;
                // Above 2^53 the integer itself is not exact in f64; widen by an ulp.
                let slack = if scaled.abs() > 9.0e15 {
                    scaled.abs() * f64::EPSILON * 2.0
                } else {
                    0.0
                };
                assert!(
                    (-0.5 - slack..=0.5 + slack).contains(&diff),
                    "{x:e} -> {} (scaled {scaled:e})",
                    p.raw()
                );
            }
            Err(PriceError::NotFinite) => assert!(!x.is_finite(), "{x:e}"),
            Err(PriceError::OutOfRange) => {
                assert!(x.is_finite() && (x * 100.0).abs() >= 9.2e18, "{x:e}");
            }
            Err(other) => panic!("{x:e}: an unexpected refusal {other:?}"),
        }
    }
    assert!(ok > 400_000, "the in-range half mostly converts: {ok}");
}

// ---------------------------------------------------------------------------
// ISIN.
// ---------------------------------------------------------------------------

/// ISO 6166 by the textbook route: expand letters to two digits into a
/// string, then Luhn from the right over that string with the check appended.
fn reference_isin_valid(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 12 {
        return false;
    }
    for (i, &c) in b.iter().enumerate() {
        let ok = match i {
            0 | 1 => c.is_ascii_uppercase(),
            11 => c.is_ascii_digit(),
            _ => c.is_ascii_uppercase() || c.is_ascii_digit(),
        };
        if !ok {
            return false;
        }
    }
    let mut digits = String::new();
    for &c in b {
        if c.is_ascii_digit() {
            digits.push(char::from(c));
        } else {
            digits.push_str(&(u32::from(c - b'A') + 10).to_string());
        }
    }
    let mut sum = 0;
    for (i, ch) in digits.chars().rev().enumerate() {
        let mut d = ch.to_digit(10).unwrap();
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    sum % 10 == 0
}

const REAL_ISINS: [&str; 5] = [
    "INE002A01018", // RELIANCE
    "INE121A01024", // CHOLAFIN
    "US0378331005", // the textbook example
    "INE467B01029", // TCS
    "INE040A01034", // HDFCBANK
];

#[test]
fn dpd_isin_real_values_verify_and_every_other_check_digit_refuses() {
    for isin in REAL_ISINS {
        assert!(reference_isin_valid(isin), "the reference agrees on {isin}");
        assert_eq!(Isin::new(isin).unwrap().as_str(), isin);
        let body = &isin[..11];
        let mut accepted = 0;
        for d in b'0'..=b'9' {
            let candidate = format!("{body}{}", char::from(d));
            if Isin::new(&candidate).is_ok() {
                accepted += 1;
                assert_eq!(candidate, isin);
            }
        }
        assert_eq!(accepted, 1, "exactly one check digit verifies {body}");
        assert_eq!(
            Isin::new(&isin.to_ascii_lowercase()),
            Err(InstrumentError::Malformed)
        );
        assert_eq!(
            Isin::new(&format!(" {isin}")),
            Err(InstrumentError::Malformed)
        );
        assert_eq!(
            Isin::new(&format!("{isin} ")),
            Err(InstrumentError::Malformed)
        );
    }
    for len in 0..=40 {
        if len == 12 {
            continue;
        }
        assert_eq!(Isin::new(&"1".repeat(len)), Err(InstrumentError::Malformed));
    }
}

/// Every single-byte substitution of every real ISIN, all 128 ASCII bytes at
/// each of 12 positions: the decoder agrees with the reference on all 7,680.
#[test]
fn dpd_isin_every_single_ascii_substitution_agrees_with_the_reference() {
    let mut checked = 0;
    for isin in REAL_ISINS {
        for at in 0..12 {
            for byte in 0..128_u8 {
                let mut b = isin.as_bytes().to_vec();
                b[at] = byte;
                let text = String::from_utf8(b).unwrap();
                assert_eq!(
                    Isin::new(&text).is_ok(),
                    reference_isin_valid(&text),
                    "{text:?}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 5 * 12 * 128);
}

#[test]
fn dpd_isin_fuzz_agrees_with_the_reference() {
    const BODY: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = Rng(0x00D3_C0DE_0004);
    let mut valid = 0;
    for _ in 0..1_000_000 {
        let mut s = String::new();
        for i in 0..11 {
            let alphabet: &[u8] = if i < 2 { &BODY[..26] } else { BODY };
            s.push(char::from(*rng.pick(alphabet)));
        }
        s.push(char::from(b'0' + u8::try_from(rng.below(10)).unwrap()));
        let ours = Isin::new(&s);
        assert_eq!(ours.is_ok(), reference_isin_valid(&s), "{s}");
        if let Ok(isin) = ours {
            valid += 1;
            assert_eq!(isin.as_str(), s, "round trip");
        }
    }
    // One check digit in ten verifies, so about 100,000 of these must.
    assert!((90_000..110_000).contains(&valid), "{valid}");
}

// ---------------------------------------------------------------------------
// Symbol.
// ---------------------------------------------------------------------------

#[test]
fn dpd_symbol_real_awkward_names_hold_and_look_alikes_refuse() {
    for name in [
        "M&M",
        "M&MFIN",
        "BAJAJ-AUTO",
        "NAM-INDIA",
        "3MINDIA",
        "NIFTYNXT50",
        "MCDOWELL-N",
        "J&KBANK",
        "IL&FSENGG",
    ] {
        let s = Symbol::new(name).unwrap();
        assert_eq!(s.as_str(), name);
        assert_eq!(s, Symbol::new(&name.to_ascii_lowercase()).unwrap());
    }
    for bad in [
        "",
        " NIFTY",
        "NIFTY ",
        "NIFTY 50",
        "NIFTY\u{a0}",
        "N\u{0130}FTY",  // dotted capital I
        "\u{041d}IFTY",  // Cyrillic capital EN
        "\u{ff2e}IFTY",  // fullwidth N
        "NIFT\u{03a5}",  // Greek capital upsilon
        "nift\u{0131}",  // dotless i: uppercases to I under Unicode rules
        "NIFTY\u{200b}", // zero width space
        "NIFTY.NS",
        "NIFTY/BANK",
        "NIFTY\0",
    ] {
        assert_eq!(Symbol::new(bad), Err(InstrumentError::Malformed), "{bad:?}");
    }
    assert!(Symbol::new(&"A".repeat(SYMBOL_CAPACITY)).is_ok());
    assert_eq!(
        Symbol::new(&"A".repeat(SYMBOL_CAPACITY + 1)),
        Err(InstrumentError::Malformed)
    );
    assert_eq!(
        Symbol::new(&"A".repeat(1000)),
        Err(InstrumentError::Malformed)
    );
}

#[test]
fn dpd_symbol_fuzz_accepts_exactly_the_allowlist_and_uppercases() {
    const ALPHABET: [&str; 16] = [
        "A", "z", "0", "9", "-", "_", "&", " ", ".", "/", "\u{e9}", "\u{0410}", "\0", "\t",
        "\u{ff21}", "K",
    ];
    let mut rng = Rng(0x00D3_C0DE_0005);
    for _ in 0..1_000_000 {
        let len = rng.below(SYMBOL_CAPACITY + 4);
        let mut text = String::new();
        for _ in 0..len {
            text.push_str(rng.pick(&ALPHABET));
        }
        let legal = !text.is_empty()
            && text.len() <= SYMBOL_CAPACITY
            && text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'&');
        match Symbol::new(&text) {
            Ok(s) => {
                assert!(legal, "{text:?}");
                assert_eq!(s.as_str(), text.to_ascii_uppercase());
                assert_eq!(s.len(), text.len());
            }
            Err(e) => {
                assert!(!legal, "{text:?}");
                assert_eq!(e, InstrumentError::Malformed);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Expiry.
// ---------------------------------------------------------------------------

#[test]
fn dpd_expiry_exhaustive_over_years_months_and_days() {
    let mut accepted = 0;
    for year in 1980..=2110_u16 {
        for month in 0..=13_u8 {
            for day in 0..=32_u8 {
                let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
                let len = match month {
                    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                    4 | 6 | 9 | 11 => 30,
                    2 if leap => 29,
                    2 => 28,
                    _ => 0,
                };
                let legal = (1990..=2100).contains(&year) && day >= 1 && day <= len;
                let got = Expiry::new(year, month, day);
                assert_eq!(got.is_ok(), legal, "{year}-{month}-{day}");
                if let Ok(e) = got {
                    accepted += 1;
                    assert_eq!((e.year(), e.month(), e.day()), (year, month, day));
                    assert_eq!(e.to_string(), format!("{year:04}-{month:02}-{day:02}"));
                }
            }
        }
    }
    // 111 years of 365 days plus the leap days 1992..=2096 and 2000 (2100 is not).
    assert_eq!(accepted, 111 * 365 + 27);
    assert!(Expiry::new(2020, 2, 29).is_ok() && Expiry::new(2024, 2, 29).is_ok());
    assert!(Expiry::new(2100, 2, 29).is_err() && Expiry::new(2000, 2, 29).is_ok());
}

// ---------------------------------------------------------------------------
// Contract: the store's segment for a derivative.
// ---------------------------------------------------------------------------

/// The contract grammar `Contract::of` renders, read back independently into
/// the kind that rendered it. `None` for any text `of` cannot produce.
fn reference_contract(text: &str) -> Option<Kind> {
    let b = text.as_bytes();
    if b.len() < 14 || b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
        return None;
    }
    if b.get(10) != Some(&b'-') {
        return None;
    }
    let num = |s: &str| -> Option<u32> {
        if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        s.parse().ok()
    };
    let expiry = Expiry::new(
        u16::try_from(num(text.get(0..4)?)?).ok()?,
        u8::try_from(num(text.get(5..7)?)?).ok()?,
        u8::try_from(num(text.get(8..10)?)?).ok()?,
    )
    .ok()?;
    let rest = text.get(11..)?;
    if rest == "FUT" {
        return Some(Kind::Future { expiry });
    }
    let (strike, side) = rest.split_once('-')?;
    let side = match side {
        "CE" => OptionSide::Call,
        "PE" => OptionSide::Put,
        _ => return None,
    };
    if strike.starts_with('0') || strike.is_empty() || !strike.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let strike: i64 = strike.parse().ok()?;
    Some(Kind::Option {
        expiry,
        strike: Paisa::from_raw(strike),
        side,
    })
}

/// FINDING DPD-01. A zero or negative strike rendered a contract segment:
/// `2025-09-30-0-CE` and `2025-09-30--500-PE`. No option has such a strike,
/// the vendor-master path (`parse_strike`) already refuses one, and the second
/// spelling carries a doubled hyphen that `Contract::parse` then accepted back
/// as an option. `Contract::of` must refuse a non-positive strike.
#[test]
fn dpd_a_non_positive_strike_has_no_contract_segment() {
    let expiry = Expiry::new(2025, 9, 30).unwrap();
    for strike in [0, -1, -500, -9_999_999, i64::MIN] {
        for side in [OptionSide::Call, OptionSide::Put] {
            let kind = Kind::Option {
                expiry,
                strike: Paisa::from_raw(strike),
                side,
            };
            assert_eq!(Contract::of(kind), None, "strike {strike} {side:?}");
        }
    }
    let one = Kind::Option {
        expiry,
        strike: Paisa::from_raw(1),
        side: OptionSide::Put,
    };
    assert_eq!(Contract::of(one).unwrap().as_str(), "2025-09-30-1-PE");
}

/// FINDING DPD-02. `Contract::parse` claimed to be the inverse of `as_str`
/// and accepted any uppercase-digit-hyphen text: `FUT`, `A`x24, `2025-9-30-FUT`
/// (a second spelling of `2025-09-30-FUT`), `2025-02-30-FUT` (no such day),
/// `2025-09-30-0002465000-CE` (a second spelling of a strike). Each is either a
/// path that aliases a real contract's directory or a name no `Kind` renders,
/// and `is_option` answered `true` for all of the non-futures. It must accept
/// exactly what `Contract::of` produces.
#[test]
fn dpd_contract_parse_accepts_only_what_of_renders() {
    for bad in [
        "FUT",
        "X",
        "-",
        "AAAAAAAAAAAAAAAAAAAAAAAA",
        "2025-9-30-FUT",
        "2025-09-30-FU",
        "2025-02-30-FUT",
        "1989-12-31-FUT",
        "2025-09-30-0002465000-CE",
        "2025-09-30-0-CE",
        "2025-09-30--5-CE",
        "2025-09-30-2465000-XX",
        "2025-09-30-2465000-CE-",
        "2025-09-30-2465000",
        "20250930-FUT",
        "2025-09-30-FUT-FUT",
        "2025-09-30-CE-PE",
        "2025-AB-30-FUT",
        "2025-09-AB-FUT",
        "ABCD-09-30-FUT",
    ] {
        assert_eq!(Contract::parse(bad), None, "must refuse: {bad}");
        assert_eq!(reference_contract(bad), None, "the reference agrees: {bad}");
    }
}

/// Random kinds render and read back; random texts over the segment's own
/// alphabet are accepted exactly when the reference reads them, and then
/// re-render to the same bytes. 1,000,000 of each.
#[test]
fn dpd_contract_of_and_parse_are_inverses_under_fuzz() {
    const ALPHABET: [&str; 10] = ["2025-", "09-", "30-", "-", "FUT", "CE", "PE", "0", "1", "2"];
    let mut rng = Rng(0x00D3_C0DE_0006);
    for _ in 0..1_000_000 {
        let expiry = loop {
            let y = 1990 + u16::try_from(rng.below(111)).unwrap();
            let m = 1 + u8::try_from(rng.below(12)).unwrap();
            let d = 1 + u8::try_from(rng.below(31)).unwrap();
            if let Ok(e) = Expiry::new(y, m, d) {
                break e;
            }
        };
        let kind = if rng.below(4) == 0 {
            Kind::Future { expiry }
        } else {
            let digits = rng.below(12);
            let strike =
                (rng.next() as i64).rem_euclid(10_i64.pow(u32::try_from(digits).unwrap()) + 1);
            Kind::Option {
                expiry,
                strike: Paisa::from_raw(strike),
                side: if rng.below(2) == 0 {
                    OptionSide::Call
                } else {
                    OptionSide::Put
                },
            }
        };
        if let Some(c) = Contract::of(kind) {
            assert_eq!(Contract::parse(c.as_str()), Some(c));
            assert_eq!(reference_contract(c.as_str()), Some(kind));
            assert!(c.as_str().len() <= CONTRACT_CAPACITY);
            assert_eq!(c.is_future(), matches!(kind, Kind::Future { .. }));
        } else {
            let Kind::Option { strike, .. } = kind else {
                panic!("a future always renders: {kind:?}");
            };
            assert!(
                strike.raw() <= 0 || strike.raw() > 9_999_999_999,
                "only an unrepresentable strike is refused: {kind:?}"
            );
        }
    }
    let mut accepted = 0;
    for _ in 0..1_000_000 {
        let mut text = String::new();
        for _ in 0..rng.below(8) {
            text.push_str(rng.pick(&ALPHABET));
        }
        let ours = Contract::parse(&text);
        let reference = reference_contract(&text)
            .filter(|_| text.len() <= CONTRACT_CAPACITY)
            .and_then(Contract::of);
        assert_eq!(ours, reference, "{text:?}");
        if let Some(c) = ours {
            accepted += 1;
            assert_eq!(c.as_str(), text);
        }
    }
    assert!(accepted > 0, "the generator reaches legal contracts");
}

// ---------------------------------------------------------------------------
// The vendor instrument-master row decoder.
// ---------------------------------------------------------------------------

fn zerodha<'a>(segment: &'a str, symbol: &'a str, ty: &'a str) -> MasterRow<'a> {
    MasterRow {
        vendor_id: "256265",
        exchange: "NSE",
        segment,
        underlying: "",
        trading_symbol: symbol,
        instrument_type: ty,
        listing_class: "",
        isin: "",
        expiry: "",
        strike_rupees: "",
        option_side: "",
    }
}

fn groww<'a>(segment: &'a str, underlying: &'a str, ty: &'a str) -> MasterRow<'a> {
    MasterRow {
        vendor_id: "NSE-X",
        exchange: "NSE",
        segment,
        underlying,
        trading_symbol: underlying,
        instrument_type: ty,
        listing_class: "EQ",
        isin: "INE002A01018",
        expiry: "",
        strike_rupees: "",
        option_side: "",
    }
}

/// FINDING DPD-03. The exchange's test instruments are declined by marker,
/// but the marker test was case-sensitive while `Symbol::new` folds case. A
/// row spelled `031nsetest` therefore passed the marker check and was KEPT as
/// the symbol `031NSETEST` -- the very identity the decline exists to keep out
/// of the store, beside real instruments and indistinguishable from them.
#[test]
fn dpd_a_test_instrument_is_declined_in_any_case() {
    for name in ["031NSETEST", "031nsetest", "031NseTest", "061bsetest"] {
        let decoded = decode_master_row(Vendor::Groww, groww("CASH", name, "EQ"))
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(decoded.skip(), Some(Skip::TestInstrument), "{name}");
        let decoded = decode_master_row(Vendor::Zerodha, zerodha("NSE", name, "EQ"))
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(decoded.skip(), Some(Skip::TestInstrument), "{name}");
    }
}

/// FINDING DPD-04. The Zerodha index alias (`NIFTY 50` -> `NIFTY`) was looked
/// up case-sensitively on the space-collapsed name, and the symbol was then
/// case-folded. `Nifty 50` therefore missed the alias and was kept as the
/// index `NIFTY50` -- a second identity for the instrument the engine sweeps,
/// which the sweep surface does not recognise. Case must not decide identity
/// in one step and be ignored in the next.
#[test]
fn dpd_the_index_alias_does_not_depend_on_case() {
    for (spelled, canonical) in [
        ("NIFTY 50", "NIFTY"),
        ("Nifty 50", "NIFTY"),
        ("nifty 50", "NIFTY"),
        ("NIFTY BANK", "BANKNIFTY"),
        ("Nifty Bank", "BANKNIFTY"),
        ("India VIX", "INDIAVIX"),
    ] {
        let decoded = decode_master_row(Vendor::Zerodha, zerodha("INDICES", spelled, "EQ"))
            .unwrap_or_else(|e| panic!("{spelled}: {e:?}"));
        let Decoded::Keep(listing) = decoded else {
            panic!("{spelled} must be kept: {decoded:?}");
        };
        assert_eq!(listing.key.underlying.as_str(), canonical, "{spelled}");
        assert_eq!(listing.key.kind, Kind::Index);
    }
}

#[test]
fn dpd_master_rows_with_hostile_fields_refuse_loudly() {
    let wide = "A".repeat(1000);
    for vendor in Vendor::MASTERED {
        let mut row = groww("CASH", "RELIANCE", "EQ");
        row.trading_symbol = &wide;
        assert!(
            matches!(
                decode_master_row(vendor, row),
                Err(InstrumentError::FieldTooWide { .. })
            ),
            "{vendor:?}"
        );
    }
    // A look-alike index name never becomes a symbol.
    for name in ["NIFTY\u{a0}50", "\u{041d}IFTY 50", "NIFTY\t50", ""] {
        let got = decode_master_row(Vendor::Zerodha, zerodha("INDICES", name, "EQ"));
        assert_eq!(got, Err(InstrumentError::Malformed), "{name:?}");
    }
    // Lower-case segment and type words are not this vendor's alphabet.
    assert_eq!(
        decode_master_row(Vendor::Zerodha, zerodha("nse", "RELIANCE", "EQ")),
        Err(InstrumentError::Malformed)
    );
    assert_eq!(
        decode_master_row(Vendor::Zerodha, zerodha("NSE", "RELIANCE", "eq")),
        Err(InstrumentError::Malformed)
    );
}

/// A derivative row is never kept, but its structured expiry and strike are
/// still checked, so a malformed one is an error rather than a quiet skip.
/// Strikes with paise (2.5, 77.5, 1012.5) are legal; zero, negative, and
/// non-decimal ones are not.
#[test]
fn dpd_derivative_rows_are_skipped_and_their_fields_still_checked() {
    let mut row = groww("FNO", "NIFTY", "CE");
    row.expiry = "2024-10-31";
    for strike in ["2.5", "77.5", "1012.5", "24500", "0.05", "24500.005"] {
        row.strike_rupees = strike;
        assert_eq!(
            decode_master_row(Vendor::Groww, row).unwrap().skip(),
            Some(Skip::LiveContract),
            "{strike}"
        );
    }
    for strike in ["0", "-0", "-77.5", "", "1e3", "NaN", "24,500", "0.004"] {
        row.strike_rupees = strike;
        assert_eq!(
            decode_master_row(Vendor::Groww, row),
            Err(InstrumentError::Malformed),
            "{strike}"
        );
    }
    row.strike_rupees = "24500";
    for expiry in [
        "2024-1-31",
        "24-10-31",
        "2024-02-30",
        "2024-10-31 ",
        "+024-10-31",
        "2024/10/31",
        "31-10-2024",
        "",
    ] {
        row.expiry = expiry;
        assert_eq!(
            decode_master_row(Vendor::Groww, row),
            Err(InstrumentError::Malformed),
            "{expiry:?}"
        );
    }
}

/// 1,000,000 rows drawn field by field from each vendor's alphabet plus
/// hostile values: never a panic, and every KEPT row satisfies the listing
/// invariants -- an NSE index or cash equity (never a contract), a canonical
/// symbol, a non-empty vendor id, an ISIN exactly where the vendor has one.
#[test]
fn dpd_master_row_fuzz_never_panics_and_keeps_only_well_formed_listings() {
    const EXCHANGE: [&str; 6] = ["NSE", "BSE", "MCX", "nse", "", "N\u{a0}SE"];
    const SEGMENT: [&str; 14] = [
        "CASH",
        "FNO",
        "INDEX",
        "COMMODITY",
        "I",
        "E",
        "D",
        "C",
        "M",
        "NSE",
        "INDICES",
        "NFO-OPT",
        "BSE",
        "x",
    ];
    const NAME: [&str; 16] = [
        "RELIANCE",
        "M&M",
        "BAJAJ-AUTO",
        "3MINDIA",
        "NIFTY 50",
        "Nifty Bank",
        "nifty",
        "031NSETEST",
        "",
        " ",
        "ABC-BE",
        "-EQ",
        "IDEA-EQ",
        "NIFTY\u{a0}50",
        "INFY",
        "x y z",
    ];
    const TYPE: [&str; 13] = [
        "EQ", "IDX", "FUT", "CE", "PE", "INDEX", "EQUITY", "FUTSTK", "OPTIDX", "OPTCUR", "XX", "",
        "eq",
    ];
    const CLASS: [&str; 9] = ["EQ", "BE", "SM", "N0", "ZZ", "", "  EQ  ", "eq", "EQ\u{a0}"];
    const ISIN: [&str; 5] = ["INE002A01018", "INE002A01019", "NA", "", "ine002a01018"];
    const EXPIRY: [&str; 4] = ["2024-10-31", "", "2024-02-30", "31-10-2024"];
    const STRIKE: [&str; 5] = ["24500", "77.5", "0", "", "-1"];
    const SIDE: [&str; 4] = ["CE", "PE", "", "XX"];
    const ID: [&str; 5] = ["1333", "", "  ", "NSE-NIFTY-30Sep25-24650-CE", "\u{a0}1"];
    let mut rng = Rng(0x00D3_C0DE_0007);
    let mut kept = 0;
    for _ in 0..1_000_000 {
        let vendor = *rng.pick(&Vendor::MASTERED);
        let row = MasterRow {
            vendor_id: rng.pick(&ID),
            exchange: rng.pick(&EXCHANGE),
            segment: rng.pick(&SEGMENT),
            underlying: rng.pick(&NAME),
            trading_symbol: rng.pick(&NAME),
            instrument_type: rng.pick(&TYPE),
            listing_class: rng.pick(&CLASS),
            isin: rng.pick(&ISIN),
            expiry: rng.pick(&EXPIRY),
            strike_rupees: rng.pick(&STRIKE),
            option_side: rng.pick(&SIDE),
        };
        let Ok(Decoded::Keep(listing)) = decode_master_row(vendor, row) else {
            continue;
        };
        kept += 1;
        let key = listing.key;
        assert_eq!(key.exchange, Exchange::Nse, "{row:?}");
        assert!(
            matches!(
                (key.segment, key.kind),
                (Segment::Index, Kind::Index) | (Segment::Cash, Kind::Equity)
            ),
            "never a contract: {row:?}"
        );
        let sym = key.underlying.as_str();
        assert_eq!(Symbol::new(sym), Ok(key.underlying));
        assert!(!sym.contains("NSETEST"), "a test marker was kept: {row:?}");
        assert!(!listing.vendor_id.as_str().is_empty());
        let has_isin_column = !vendor.master_columns().isin.is_empty();
        assert_eq!(
            listing.isin.is_some(),
            key.kind == Kind::Equity && has_isin_column,
            "{row:?}"
        );
        if let Some(u) = listing.unsuffixed {
            assert_eq!(u.kind, Kind::Equity);
            assert!(sym.starts_with(u.underlying.as_str()) && sym.len() > u.underlying.len());
        }
    }
    assert!(kept > 100, "the generator reaches kept rows: {kept}");
}

// ---------------------------------------------------------------------------
// The F&O universe.
// ---------------------------------------------------------------------------

#[test]
fn dpd_the_fno_universe_probes_answer_members_and_only_members() {
    let mut shares = 0;
    for name in FNO_UNDERLYINGS {
        assert!(FNO_INDEX.contains(name), "{name}");
        let symbol = Symbol::new(name).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(symbol.as_str(), name, "{name} is already canonical");
        let cash = InstrumentKey::cash(Exchange::Nse, name).unwrap();
        let is_index = !FNO_INDEX_UNDERLYINGS.iter().all(|i| *i != name);
        assert_eq!(cash.is_sweepable(), !is_index, "{name}");
        if !is_index {
            shares += 1;
        }
        assert!(
            !FNO_INDEX.contains(&name.to_ascii_lowercase())
                || name.bytes().all(|b| !b.is_ascii_alphabetic())
        );
        assert!(!FNO_INDEX.contains(&format!("{name} ")));
        assert!(!FNO_INDEX.contains(&format!("{name}-EQ")));
    }
    assert_eq!(shares, 208, "CLAUDE.md §1: 208 shares");
    for probe in ["", " ", &"A".repeat(1000), "NIFTY\u{a0}", "\u{041d}IFTY"] {
        assert!(!FNO_INDEX.contains(probe), "{probe:?}");
        assert!(of_equity(probe).is_none(), "{probe:?}");
    }
    let mut isins = 0;
    for name in NIFTY_TOTAL_MARKET {
        if let Some(isin) = nse_isin(name) {
            isins += 1;
            assert!(reference_isin_valid(isin.as_str()), "{name} {isin}");
        }
    }
    assert!(isins > 700, "{isins}");
    let mut rng = Rng(0x00D3_C0DE_0008);
    for _ in 0..1_000_000 {
        let mut s = String::new();
        for _ in 0..rng.below(12) {
            s.push(char::from(b'A' + u8::try_from(rng.below(26)).unwrap()));
        }
        let member = !FNO_UNDERLYINGS.iter().all(|n| *n != s);
        assert_eq!(FNO_INDEX.contains(&s), member, "{s}");
    }
}

// ---------------------------------------------------------------------------
// O(1): the per-row decoder, the price reader and the membership probe.
// ---------------------------------------------------------------------------

fn percentiles(mut ns: Vec<u128>) -> (u128, u128, u128) {
    ns.sort_unstable();
    let at = |pct: usize| ns[(ns.len() - 1) * pct / 100];
    (at(50), at(99), *ns.last().unwrap())
}

/// Per-operation time at N = 10^3 .. 10^6 operations. The input to each
/// operation is drawn from the same fixed pool, so a cost that grows with N
/// would be a cost that grows with how many rows came before -- the shape a
/// hidden accumulator takes. Printed for the report; the assertion is a
/// deliberately loose flatness bound on the median, which is robust to the
/// shared box's noise where p99 and max are not.
#[test]
fn dpd_per_row_cost_is_flat_in_the_number_of_rows() {
    let rows = [
        groww("CASH", "RELIANCE", "EQ"),
        groww("CASH", "BAJAJ-AUTO", "EQ"),
        zerodha("INDICES", "NIFTY 50", "EQ"),
        zerodha("NSE", "M&M", "EQ"),
    ];
    let texts = ["23109.55", "0.145", "-92233720368547758.07", "77.5"];
    let mut medians = Vec::new();
    for n in [1_000_usize, 10_000, 100_000, 1_000_000] {
        let mut decode = Vec::with_capacity(n);
        let mut price = Vec::with_capacity(n);
        let mut probe = Vec::with_capacity(n);
        for i in 0..n {
            let row = rows[i % rows.len()];
            let vendor = if i % 4 < 2 {
                Vendor::Groww
            } else {
                Vendor::Zerodha
            };
            let t = std::time::Instant::now();
            let d = decode_master_row(vendor, row);
            decode.push(t.elapsed().as_nanos());
            assert!(d.is_ok());
            let t = std::time::Instant::now();
            let p = Paisa::from_rupee_text_half_up(texts[i % texts.len()]);
            price.push(t.elapsed().as_nanos());
            assert!(p.is_ok());
            let t = std::time::Instant::now();
            let hit = FNO_INDEX.contains(FNO_UNDERLYINGS[i % FNO_UNDERLYINGS.len()]);
            probe.push(t.elapsed().as_nanos());
            assert!(hit);
        }
        let (d50, d99, dmax) = percentiles(decode);
        let (p50, p99, pmax) = percentiles(price);
        let (q50, q99, qmax) = percentiles(probe);
        println!(
            "N={n:>9} decode_master_row p50={d50}ns p99={d99}ns max={dmax}ns | \
             from_rupee_text_half_up p50={p50}ns p99={p99}ns max={pmax}ns | \
             FNO_INDEX.contains p50={q50}ns p99={q99}ns max={qmax}ns"
        );
        assert!(d50 <= d99 && d99 <= dmax && p50 <= p99 && q50 <= q99);
        medians.push((d50, p50, q50));
    }
    let (first, last) = (medians[0], medians[3]);
    assert!(
        last.0 <= first.0 * 10 + 200
            && last.1 <= first.1 * 10 + 200
            && last.2 <= first.2 * 10 + 200,
        "a median grew tenfold with N: {medians:?}"
    );
}

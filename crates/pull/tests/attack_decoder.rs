//! Adversarial attack on `pull`'s contract-name decoder and master helpers:
//! `fno::read_contract` (a Groww contract name into this store's identity),
//! `masters::missing_columns` and `masters::nse_index_csv`.
//!
//! Fixed-seed splitmix64 throughout, so a rerun is byte identical. Every case
//! asserts a value or a refusal; a panic is itself the failure. `DPD-`
//! invariants, D-3150..D-3159.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
use brutex_core::price::Paisa;
use brutex_core::vendor::Vendor;
use pull::fno::read_contract;
use pull::masters::{missing_columns, nse_index_csv, required_columns};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let n = u64::try_from(items.len()).unwrap();
        &items[usize::try_from(self.next() % n).unwrap()]
    }
}

fn jan4() -> Expiry {
    Expiry::new(2024, 1, 4).unwrap()
}

/// The shapes the vendor documents, and the awkward underlyings: a hyphen,
/// an ampersand, a leading digit, a trailing digit that runs into nothing
/// because the grammar is hyphen-delimited, and strikes with paise.
#[test]
fn dpd_every_documented_shape_reads_to_the_contract_it_names() {
    let monthly = Expiry::new(2024, 1, 25).unwrap();
    for (name, keyed, underlying, contract, option) in [
        (
            "NSE-NIFTY-04Jan24-19200-CE",
            jan4(),
            "NIFTY",
            "2024-01-04-1920000-CE",
            Some((1_920_000, OptionSide::Call)),
        ),
        (
            "NSE-NIFTY-04Jan24-FUT",
            jan4(),
            "NIFTY",
            "2024-01-04-FUT",
            None,
        ),
        (
            "NSE-BAJAJ-AUTO-Jan24-8000-PE",
            monthly,
            "BAJAJ-AUTO",
            "2024-01-25-800000-PE",
            Some((800_000, OptionSide::Put)),
        ),
        (
            "NSE-M&M-Jan24-1650-CE",
            monthly,
            "M&M",
            "2024-01-25-165000-CE",
            Some((165_000, OptionSide::Call)),
        ),
        (
            "NSE-M&MFIN-Jan24-277.5-PE",
            monthly,
            "M&MFIN",
            "2024-01-25-27750-PE",
            Some((27_750, OptionSide::Put)),
        ),
        (
            "NSE-3MINDIA-Jan24-30000-CE",
            monthly,
            "3MINDIA",
            "2024-01-25-3000000-CE",
            Some((3_000_000, OptionSide::Call)),
        ),
        (
            "NSE-NIFTYNXT50-25Jan24-100000-PE",
            monthly,
            "NIFTYNXT50",
            "2024-01-25-10000000-PE",
            Some((10_000_000, OptionSide::Put)),
        ),
        (
            "NSE-IDEA-Jan24-2.5-CE",
            monthly,
            "IDEA",
            "2024-01-25-250-CE",
            Some((250, OptionSide::Call)),
        ),
        (
            "NSE-NHPC-Jan24-77.5-PE",
            monthly,
            "NHPC",
            "2024-01-25-7750-PE",
            Some((7_750, OptionSide::Put)),
        ),
        (
            "NSE-ICICIGI-Jan24-1012.5-CE",
            monthly,
            "ICICIGI",
            "2024-01-25-101250-CE",
            Some((101_250, OptionSide::Call)),
        ),
        (
            "NSE-NIFTY-04Jan24-0.05-CE",
            jan4(),
            "NIFTY",
            "2024-01-04-5-CE",
            Some((5, OptionSide::Call)),
        ),
    ] {
        let found = read_contract(name, keyed).unwrap_or_else(|| panic!("{name} must read"));
        assert_eq!(found.underlying, underlying, "{name}");
        assert_eq!(found.contract.as_str(), contract, "{name}");
        assert_eq!(found.vendor_symbol, name);
        assert_eq!(found.expiry, keyed);
        assert_eq!(
            found.option,
            option.map(|(p, s)| (Paisa::from_raw(p), s)),
            "{name}"
        );
        assert_eq!(Contract::parse(contract), Some(found.contract));
    }
}

/// FINDING DPD-05. A zero strike read: `NSE-NIFTY-04Jan24-0-CE` became the
/// contract `2024-01-04-0-CE`. No option has a zero strike and the master path
/// already refuses one; filed, it would be a series nobody can price. Closed
/// by `Contract::of` refusing a non-positive strike (D-3150).
#[test]
fn dpd_a_zero_strike_is_refused() {
    for name in [
        "NSE-NIFTY-04Jan24-0-CE",
        "NSE-NIFTY-04Jan24-0.00-PE",
        "NSE-NIFTY-04Jan24-.0-PE",
    ] {
        assert_eq!(read_contract(name, jan4()), None, "{name}");
    }
}

/// FINDING DPD-06. A second spelling of one contract read to the same
/// identity: `NSE-NIFTY-+4Jan24-...` (a `+` on the day, which `u8::from_str`
/// accepts) and `...-019200-CE` (a leading zero on the strike). The chain
/// deduplicates by NAME, so two spellings of one contract are two filings of
/// one series and two bar requests for it. Neither is a spelling the vendor
/// documents; both are refused.
#[test]
fn dpd_a_second_spelling_of_the_same_contract_is_refused() {
    for name in [
        "NSE-NIFTY-+4Jan24-19200-CE",
        "NSE-NIFTY-04Jan+4-19200-CE",
        "NSE-NIFTY-Jan+4-19200-CE",
        "NSE-NIFTY-04Jan24-019200-CE",
        "NSE-NIFTY-04Jan24-0019200-CE",
        "NSE-NIFTY-04Jan24-00.05-CE",
    ] {
        assert_eq!(read_contract(name, jan4()), None, "must refuse: {name}");
    }
    // `0.05` itself is the canonical spelling of a five-paisa strike.
    assert!(read_contract("NSE-NIFTY-04Jan24-0.05-CE", jan4()).is_some());
}

/// Everything else a hostile or broken answer could carry.
#[test]
fn dpd_malformed_names_are_refused_never_guessed() {
    let long = format!("NSE-{}-04Jan24-19200-CE", "N".repeat(1000));
    for name in [
        "",
        "-",
        "--",
        "NSE",
        "NSE-",
        "NSE-NIFTY",
        "NSE-NIFTY-04Jan24",
        "NSE-NIFTY-04Jan24-19200",
        "NSE-NIFTY-04Jan24-19200-XX",
        "NSE-NIFTY-04Jan24-19200-ce",
        "NSE-NIFTY-04Jan24-19200-FUT",
        "NSE-NIFTY-04Jan24-19200.005-CE",
        "NSE-NIFTY-04Jan24-1e4-CE",
        "NSE-NIFTY-04Jan24-NaN-CE",
        "NSE-NIFTY-04Jan24-19,200-CE",
        "NSE-NIFTY-04Jan24-99999999999999999999-CE",
        "NSE-NIFTY-04Jan24-100000000-CE",
        "NSE-NIFTY-04Jan24--19200-CE",
        "NSE-NIFTY-04Jan24- 19200-CE",
        "NSE-NIFTY-04Jan24-19200 -CE",
        "NSE-NIFTY-4Jan24-19200-CE",
        "NSE-NIFTY-004Jan24-19200-CE",
        "NSE-NIFTY-04Jan2024-19200-CE",
        "NSE-NIFTY-05Jan24-19200-CE",
        "NSE-NIFTY-04Feb24-19200-CE",
        "NSE-NIFTY-04Jan25-19200-CE",
        "NSE-NIFTY-04J\u{e4}n24-19200-CE",
        "NSE-NIFTY-0\u{0664}Jan24-19200-CE",
        "NSE-NIFTY-Jan2-19200-CE",
        "NSE--04Jan24-19200-CE",
        "NSE-NIFTY-04Jan24-FUT ",
        " NSE-NIFTY-04Jan24-FUT-",
        long.as_str(),
    ] {
        assert_eq!(read_contract(name, jan4()), None, "must refuse: {name:?}");
    }
}

/// What the reader cannot judge alone, pinned so a change is visible. These
/// read, and each is a question for the real vendor files rather than for
/// this build (see the report's vendor-data list):
///
/// * the exchange token is not checked -- `BSE-` reads, and an empty one too;
/// * the month word is case-blind -- `04JAN24` and `04Jan24` are one contract;
/// * a fraction's trailing zero is not a different strike -- `77.50` is `77.5`.
///
/// `crate::chain::month` compares the underlying with the ask, not the
/// exchange, so the first is a live alias there.
#[test]
fn dpd_spellings_left_to_vendor_evidence_are_pinned() {
    let base = read_contract("NSE-NIFTY-04Jan24-19200-CE", jan4()).unwrap();
    for alias in [
        "BSE-NIFTY-04Jan24-19200-CE",
        "-NIFTY-04Jan24-19200-CE",
        "NSE-NIFTY-04JAN24-19200-CE",
        "NSE-NIFTY-04jan24-19200-CE",
        "NSE-NIFTY-04Jan24-19200.00-CE",
        "NSE-NIFTY-04Jan24-19200.0-CE",
    ] {
        let got = read_contract(alias, jan4()).unwrap_or_else(|| panic!("{alias}"));
        assert_eq!(
            (got.underlying.as_str(), got.contract),
            ("NIFTY", base.contract),
            "{alias}"
        );
    }
}

/// 1,000,000 names assembled from legal and hostile pieces. Never a panic; a
/// name that reads names a positive strike, a contract `Contract::parse`
/// round-trips, the expiry it was keyed on, and the underlying that sits
/// between the exchange and the expiry token, verbatim.
#[test]
fn dpd_read_contract_fuzz_never_panics_and_reads_only_consistent_contracts() {
    const EXCHANGE: [&str; 5] = ["NSE", "NSE", "BSE", "", "nse"];
    const UNDERLYING: [&str; 9] = [
        "NIFTY",
        "BAJAJ-AUTO",
        "M&M",
        "3MINDIA",
        "NIFTYNXT50",
        "",
        "a b",
        "X-Y-Z",
        "NIFTY-04Jan24",
    ];
    const TOKEN: [&str; 12] = [
        "04Jan24",
        "Jan24",
        "4Jan24",
        "+4Jan24",
        "04JAN24",
        "04Jan2024",
        "32Jan24",
        "04Xyz24",
        "",
        "05Jan24",
        "Feb24",
        "Jan+4",
    ];
    const STRIKE: [&str; 16] = [
        "19200",
        "19200.5",
        "2.5",
        "77.5",
        "1012.5",
        "0",
        "00",
        "019200",
        "19200.005",
        "",
        "abc",
        "99999999999",
        "0.05",
        "19200.50",
        "-1",
        " 1",
    ];
    const TAIL: [&str; 7] = ["CE", "PE", "FUT", "XX", "ce", "", "CE-X"];
    let mut rng = Rng(0x00D3_C0DE_0101);
    let mut read = 0_u32;
    for _ in 0..1_000_000 {
        let (ex, u, tok, strike, tail) = (
            *rng.pick(&EXCHANGE),
            *rng.pick(&UNDERLYING),
            *rng.pick(&TOKEN),
            *rng.pick(&STRIKE),
            *rng.pick(&TAIL),
        );
        let name = if tail == "FUT" {
            format!("{ex}-{u}-{tok}-FUT")
        } else {
            format!("{ex}-{u}-{tok}-{strike}-{tail}")
        };
        let Some(found) = read_contract(&name, jan4()) else {
            continue;
        };
        read += 1;
        assert_eq!(found.expiry, jan4());
        assert_eq!(found.vendor_symbol, name);
        assert!(!found.underlying.is_empty(), "{name}");
        assert!(name.contains(&format!("-{}-", found.underlying)), "{name}");
        assert_eq!(
            Contract::parse(found.contract.as_str()),
            Some(found.contract)
        );
        let kind = match found.option {
            None => Kind::Future { expiry: jan4() },
            Some((strike, side)) => {
                assert!(strike > Paisa::ZERO, "{name}");
                assert!(!strike_text_has_leading_zero(&name), "{name}");
                Kind::Option {
                    expiry: jan4(),
                    strike,
                    side,
                }
            }
        };
        assert_eq!(Contract::of(kind), Some(found.contract), "{name}");
        assert!(!name.contains('+'), "a signed date part read: {name}");
    }
    assert!(
        read > 10_000,
        "the generator reaches readable names: {read}"
    );
}

fn strike_text_has_leading_zero(name: &str) -> bool {
    let mut parts = name.rsplit('-');
    parts.next();
    let strike = parts.next().unwrap_or("");
    strike.len() > 1 && strike.starts_with('0') && !strike.starts_with("0.")
}

/// Random bytes, not just grammar pieces: never a panic, and anything that
/// reads still satisfies the same identity invariants.
#[test]
fn dpd_read_contract_survives_arbitrary_text() {
    const BYTES: [&str; 16] = [
        "N",
        "S",
        "E",
        "-",
        "0",
        "4",
        "J",
        "a",
        "n",
        "2",
        "C",
        "P",
        ".",
        "\u{e9}",
        "\u{1f600}",
        " ",
    ];
    let mut rng = Rng(0x00D3_C0DE_0102);
    for _ in 0..1_000_000 {
        let mut name = String::new();
        for _ in 0..(rng.next() % 30) {
            name.push_str(rng.pick(&BYTES));
        }
        if let Some(found) = read_contract(&name, jan4()) {
            assert_eq!(
                Contract::parse(found.contract.as_str()),
                Some(found.contract)
            );
            if let Some((strike, _)) = found.option {
                assert!(strike > Paisa::ZERO, "{name}");
            }
        }
    }
}

/// Header checks: a byte-order mark, quoting or case on a column name makes
/// that column MISSING, loudly, rather than matched by a looser rule.
#[test]
fn dpd_master_headers_match_exactly_or_report_the_column_missing() {
    for vendor in Vendor::MASTERED {
        let names = required_columns(vendor);
        let header = names.join(",");
        assert!(missing_columns(&header, vendor).is_empty(), "{vendor:?}");
        assert!(missing_columns(&format!("{header}\r\n"), vendor).is_empty());
        assert!(missing_columns(&format!(" {} ", names.join(" , ")), vendor).is_empty());
        let first = names[0];
        let bom = format!("\u{feff}{header}");
        assert_eq!(missing_columns(&bom, vendor), vec![first], "{vendor:?}");
        let quoted = header.replacen(first, &format!("\"{first}\""), 1);
        assert_eq!(missing_columns(&quoted, vendor), vec![first]);
        let upper = header.replacen(first, &first.to_ascii_uppercase(), 1);
        if first != first.to_ascii_uppercase() {
            assert_eq!(missing_columns(&upper, vendor), vec![first]);
        }
        assert_eq!(missing_columns("", vendor), names);
    }
}

/// The index-list conversion refuses a separator it cannot carry, in either
/// field, and never emits a row whose columns do not line up.
#[test]
fn dpd_index_list_conversion_never_emits_a_misaligned_row() {
    const PIECES: [&str; 10] = [
        "NIFTY", " ", "50", ",", "\n", "\"", "\r", "Broad", "\u{a0}", "&",
    ];
    let mut rng = Rng(0x00D3_C0DE_0103);
    for _ in 0..100_000 {
        let mut name = String::new();
        let mut category = String::new();
        for _ in 0..(rng.next() % 5) {
            name.push_str(rng.pick(&PIECES));
        }
        for _ in 0..(rng.next() % 4) {
            category.push_str(rng.pick(&PIECES));
        }
        let body = serde_json::json!({ category.clone(): [name.clone()] }).to_string();
        match nse_index_csv(&body) {
            Ok(csv) => {
                assert!(!name.contains([',', '\n']) && !category.contains([',', '\n']));
                for line in csv.lines() {
                    assert_eq!(line.matches(',').count(), 1, "{csv:?}");
                }
            }
            Err(why) => {
                assert!(
                    name.contains([',', '\n']) || category.contains([',', '\n']),
                    "{why}"
                );
            }
        }
    }
}

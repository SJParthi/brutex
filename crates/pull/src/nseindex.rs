//! Resolving a vendor's index symbol to the name NSE publishes for it.
//!
//! A vendor does not ship NSE's names. Zerodha's instrument master calls the
//! private-bank index `NIFTY PVT BANK`; NSE publishes it as `Nifty Private
//! Bank`. Neither is wrong and neither is derivable from the other by rule,
//! so a symbol arriving from a feed cannot be checked against the exchange
//! until the two are joined.
//!
//! The join here has exactly two strengths and it always says which one it
//! used, because they are not equally good evidence:
//!
//! * [`Basis::Published`] — the vendor's symbol, collapsed, IS the NSE name.
//!   This is proof. 46 of Zerodha's 136 index symbols land here.
//! * [`Basis::Abbreviation`] — the symbol is an ordered abbreviation that
//!   exactly one NSE index accepts. This is strong but it is inference, and
//!   §3 rule 1 is why it is labelled rather than folded into the first.
//!
//! Anything else is refused and named: [`Unresolved::Ambiguous`] when more
//! than one index accepts the abbreviation, [`Unresolved::Absent`] when none
//! does. Neither guesses. A wrong mapping is worse than a missing one — it is
//! silent, and §4 bans the fallback that hides a failure.
//!
//! # Why the abbreviation rule carries a digit constraint
//!
//! Ordered-subsequence alone is not sound, and this is measured rather than
//! feared. Against NSE's published list `NIFTYGS813YR` — the 8-13 year G-Sec
//! benchmark — matched `NIFTYLARGEMIDCAP250PLUS813YRGSEC7030`, a hybrid
//! equity-and-debt index that is not the same instrument in any sense. The
//! letters simply fell in order inside a longer name. Subsequence has no
//! notion of a word boundary, so its false-positive rate rises with the
//! length of the candidate.
//!
//! The digits are what separate them: the abbreviation names one number,
//! `813`, and the expansion names three. [`digits_agree`] requires the count
//! to match and each abbreviation run to be a suffix of its expansion run —
//! a suffix, not an equality, because `BHARATBONDAPR30` legitimately
//! shortens `...APRIL2030`. Adding it refused the false match and, because
//! it also rules candidates out, raised the unambiguous count from 57 to 61.
//!
//! # Cost
//!
//! [`Catalogue::resolve`] is O(1) when the symbol is published verbatim — a
//! hash lookup — and a bounded scan of the catalogue otherwise. The scan is
//! not O(1) and this module does not pretend it is. [`Catalogue::index`]
//! exists so it is paid once, at master load, after which every lookup
//! through [`Mapping::get`] is a hash hit. That is the shape §3 rule 4 asks
//! for: the per-operation cost on the hot path is constant, and the
//! non-constant part happens once and is stated.

use std::collections::{HashMap, HashSet};

/// The evidence behind a resolution. Never discarded — a caller that treats
/// an inference as proof is the invention §3 rule 1 forbids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// The vendor's collapsed symbol is exactly the name NSE publishes.
    Published,
    /// The symbol abbreviates exactly one published name.
    Abbreviation,
}

/// Why a symbol did not resolve. Both variants are reportable; neither is a
/// failure of the feed, and the caller is expected to show them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// The abbreviation is accepted by this many published names.
    Ambiguous(usize),
    /// No published name accepts it. Correct for instruments NSE does not
    /// list as indices at all — an ETF net asset value, or India VIX, which
    /// NSE publishes outside the index catalogue.
    Absent,
}

/// Uppercase, keeping only letters and digits.
///
/// Vendors disagree about spaces, hyphens and ampersands and agree about
/// nothing else, so the separator is discarded before any comparison.
#[must_use]
pub fn collapse(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Every maximal run of digits, in order.
fn numeric_runs(text: &str) -> Vec<&str> {
    let mut runs = Vec::new();
    let mut start = None;
    for (index, byte) in text.bytes().enumerate() {
        match (byte.is_ascii_digit(), start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                runs.push(&text[from..index]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        runs.push(&text[from..]);
    }
    runs
}

/// Whether `short`'s numbers are consistent with `long`'s.
///
/// The counts must match and each of `short`'s runs must end `long`'s
/// corresponding run. The suffix rule admits a shortened year — `30` for
/// `2030` — and refuses a candidate that names numbers the abbreviation
/// never mentioned.
#[must_use]
pub fn digits_agree(short: &str, long: &str) -> bool {
    let abbreviated = numeric_runs(short);
    let published = numeric_runs(long);
    abbreviated.len() == published.len()
        && abbreviated
            .iter()
            .zip(&published)
            .all(|(part, whole)| whole.ends_with(*part))
}

/// Whether every character of `short` appears in `long`, in order.
#[must_use]
pub fn is_ordered_subsequence(short: &str, long: &str) -> bool {
    let mut wanted = short.chars();
    let mut next = wanted.next();
    for character in long.chars() {
        if next == Some(character) {
            next = wanted.next();
        }
    }
    next.is_none()
}

/// Whether `published` accepts `symbol` as an abbreviation of itself.
fn accepts(symbol: &str, published: &str) -> bool {
    is_ordered_subsequence(symbol, published) && digits_agree(symbol, published)
}

/// The indices NSE publishes, collapsed and deduplicated.
#[derive(Clone, Debug, Default)]
pub struct Catalogue {
    names: HashSet<String>,
}

impl Catalogue {
    /// Build from NSE's published names. Empty names are dropped; duplicates
    /// collapse to one.
    #[must_use]
    pub fn from_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            names: names
                .into_iter()
                .map(|name| collapse(name.as_ref()))
                .filter(|name| !name.is_empty())
                .collect(),
        }
    }

    /// How many distinct indices the catalogue holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the catalogue holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Resolve one vendor symbol.
    ///
    /// # Errors
    ///
    /// [`Unresolved::Ambiguous`] when several published names accept the
    /// abbreviation, [`Unresolved::Absent`] when none does.
    pub fn resolve(&self, symbol: &str) -> Result<(&str, Basis), Unresolved> {
        let key = collapse(symbol);
        if key.is_empty() {
            return Err(Unresolved::Absent);
        }
        if let Some(name) = self.names.get(&key) {
            return Ok((name.as_str(), Basis::Published));
        }
        let mut accepted = self.names.iter().filter(|name| accepts(&key, name));
        match (accepted.next(), accepted.next()) {
            (Some(only), None) => Ok((only.as_str(), Basis::Abbreviation)),
            (None, _) => Err(Unresolved::Absent),
            (Some(_), Some(_)) => Err(Unresolved::Ambiguous(2 + accepted.count())),
        }
    }

    /// Resolve a whole vendor master once, so later lookups are hash hits.
    #[must_use]
    pub fn index<I, S>(&self, symbols: I) -> Mapping
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut mapping = Mapping::default();
        for symbol in symbols {
            let symbol = symbol.as_ref();
            let key = collapse(symbol);
            if mapping.resolved.contains_key(&key) {
                continue;
            }
            match self.resolve(symbol) {
                Ok((name, basis)) => {
                    mapping.resolved.insert(key, (name.to_owned(), basis));
                }
                Err(why) => mapping.refused.push((symbol.to_owned(), why)),
            }
        }
        mapping.refused.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        mapping
    }
}

/// One vendor master, joined to the exchange. Built once; read O(1).
#[derive(Clone, Debug, Default)]
pub struct Mapping {
    resolved: HashMap<String, (String, Basis)>,
    refused: Vec<(String, Unresolved)>,
}

impl Mapping {
    /// The NSE name for a symbol, and the evidence for it. O(1).
    #[must_use]
    pub fn get(&self, symbol: &str) -> Option<(&str, Basis)> {
        self.resolved
            .get(&collapse(symbol))
            .map(|(name, basis)| (name.as_str(), *basis))
    }

    /// How many symbols resolved.
    #[must_use]
    pub fn resolved(&self) -> usize {
        self.resolved.len()
    }

    /// How many resolved on this basis.
    #[must_use]
    pub fn resolved_by(&self, basis: Basis) -> usize {
        self.resolved
            .values()
            .filter(|(_, held)| *held == basis)
            .count()
    }

    /// Every symbol that did not resolve, with its reason, sorted by symbol.
    #[must_use]
    pub fn refused(&self) -> &[(String, Unresolved)] {
        &self.refused
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Basis, Catalogue, Unresolved, collapse, digits_agree, is_ordered_subsequence, numeric_runs,
    };

    /// The three NSE names the digit finding turns on, published verbatim.
    fn gsec_family() -> Catalogue {
        Catalogue::from_names([
            "Nifty LargeMidcap 250 plus 8-13 yr G-Sec 70:30",
            "Nifty 8-13 yr G-Sec",
            "Nifty Private Bank",
        ])
    }

    #[test]
    fn collapsing_discards_every_separator_a_vendor_might_choose() {
        assert_eq!(collapse("Nifty Private Bank"), "NIFTYPRIVATEBANK");
        assert_eq!(collapse("NIFTY PVT BANK"), "NIFTYPVTBANK");
        assert_eq!(collapse("Nifty 8-13 yr G-Sec"), "NIFTY813YRGSEC");
        assert_eq!(collapse("Nifty Oil & Gas"), "NIFTYOILGAS");
        assert_eq!(collapse("  -&- "), "");
    }

    #[test]
    fn numeric_runs_are_maximal_and_ordered() {
        assert!(numeric_runs("NIFTYBANK").is_empty());
        assert_eq!(numeric_runs("NIFTY50"), ["50"]);
        assert_eq!(numeric_runs("NIFTY813YRGSEC"), ["813"]);
        assert_eq!(
            numeric_runs("NIFTYLARGEMIDCAP250PLUS813YRGSEC7030"),
            ["250", "813", "7030"]
        );
        assert_eq!(numeric_runs("100A200"), ["100", "200"]);
    }

    #[test]
    fn digits_agree_admits_a_shortened_year_and_refuses_an_extra_number() {
        // BHARATBONDAPR30 legitimately shortens APRIL2030 -- suffix, not equality.
        assert!(digits_agree(
            "BHARATBONDAPR30",
            "NIFTYBHARATBONDINDEXAPRIL2030"
        ));
        assert!(digits_agree("NIFTY100EQLWGT", "NIFTY100EQUALWEIGHT"));
        assert!(digits_agree("NIFTYPVTBANK", "NIFTYPRIVATEBANK"));
        // One number named against three is the false-positive shape.
        assert!(!digits_agree(
            "NIFTYGS813YR",
            "NIFTYLARGEMIDCAP250PLUS813YRGSEC7030"
        ));
        // Same count, wrong value.
        assert!(!digits_agree("NIFTY50", "NIFTY100"));
    }

    #[test]
    fn ordered_subsequence_respects_order() {
        assert!(is_ordered_subsequence("NIFTYPVTBANK", "NIFTYPRIVATEBANK"));
        assert!(is_ordered_subsequence("", "ANYTHING"));
        // Zerodha writes GS ahead of the tenor; NSE writes it behind.
        assert!(!is_ordered_subsequence("NIFTYGS813YR", "NIFTY813YRGSEC"));
        assert!(!is_ordered_subsequence("NIFTYBANKX", "NIFTYBANK"));
    }

    /// The finding that put the digit constraint in this module.
    ///
    /// `NIFTYGS813YR` is the 8-13 year G-Sec benchmark. Ordered-subsequence
    /// alone hands it a hybrid equity-and-debt index instead -- a different
    /// instrument, silently. Both candidates must be refused: the hybrid
    /// because its numbers disagree, the benchmark because Zerodha reorders
    /// `GS` ahead of the tenor and subsequence cannot see through a reorder.
    /// Refusing is correct. Naming the hybrid would be invention.
    #[test]
    fn the_hybrid_index_never_answers_for_the_gsec_benchmark() {
        let hybrid = "NIFTYLARGEMIDCAP250PLUS813YRGSEC7030";
        assert!(
            is_ordered_subsequence("NIFTYGS813YR", hybrid),
            "the unsound rule really does accept it -- that is the whole point"
        );
        assert!(!digits_agree("NIFTYGS813YR", hybrid));
        assert_eq!(
            gsec_family().resolve("NIFTY GS 8 13YR"),
            Err(Unresolved::Absent)
        );
    }

    #[test]
    fn a_symbol_published_verbatim_resolves_as_proof() {
        assert_eq!(
            gsec_family().resolve("Nifty Private Bank"),
            Ok(("NIFTYPRIVATEBANK", Basis::Published))
        );
    }

    #[test]
    fn an_abbreviation_accepted_by_one_name_resolves_as_inference() {
        assert_eq!(
            gsec_family().resolve("NIFTY PVT BANK"),
            Ok(("NIFTYPRIVATEBANK", Basis::Abbreviation))
        );
    }

    #[test]
    fn an_abbreviation_two_names_accept_is_refused_with_its_count() {
        let nse = Catalogue::from_names([
            "Nifty 100 Quality 30",
            "Nifty 100 Quantity 30",
            "Nifty 100 Quality Rated 30",
        ]);
        assert_eq!(nse.resolve("NIFTY100QLTY30"), Err(Unresolved::Ambiguous(2)));
    }

    #[test]
    fn a_symbol_no_name_accepts_is_absent_and_so_is_an_empty_one() {
        let nse = gsec_family();
        assert_eq!(nse.resolve("INDIA VIX"), Err(Unresolved::Absent));
        assert_eq!(nse.resolve("  &  "), Err(Unresolved::Absent));
    }

    #[test]
    fn the_catalogue_deduplicates_and_drops_empty_names() {
        let nse = Catalogue::from_names(["Nifty 50", "NIFTY50", "nifty-50", "", " & "]);
        assert_eq!(nse.len(), 1);
        assert!(!nse.is_empty());
        assert!(Catalogue::default().is_empty());
        assert_eq!(Catalogue::default().len(), 0);
    }

    #[test]
    fn indexing_a_master_pays_the_scan_once_and_reports_every_refusal() {
        let nse = gsec_family();
        let mapping = nse.index([
            "Nifty Private Bank",
            "NIFTY PVT BANK",
            "NIFTY PVT BANK",
            "INDIA VIX",
            "NIFTY GS 8 13YR",
        ]);
        assert_eq!(mapping.resolved(), 2);
        assert_eq!(mapping.resolved_by(Basis::Published), 1);
        assert_eq!(mapping.resolved_by(Basis::Abbreviation), 1);
        assert_eq!(
            mapping.get("nifty pvt bank"),
            Some(("NIFTYPRIVATEBANK", Basis::Abbreviation))
        );
        assert_eq!(mapping.get("NOT A SYMBOL"), None);
        assert_eq!(
            mapping.refused(),
            [
                ("INDIA VIX".to_owned(), Unresolved::Absent),
                ("NIFTY GS 8 13YR".to_owned(), Unresolved::Absent),
            ]
        );
    }
}

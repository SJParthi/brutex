//! Parses a lake contract directory name into the workspace's own identity.
//!
//! The lake files a contract under a directory whose name is the contract:
//!
//! ```text
//! bars/NSE/FNO/NSE-NIFTY-01Apr20-10000-CE/1minute/2020/03.parquet
//!              ^^^^^^^^^^^^^^^^^^^^^^^^^^
//! ```
//!
//! Two shapes exist, both verified by walking all 116,086 F&O directories:
//!
//! | Shape | Count | Example |
//! |---|---|---|
//! | `<EX>-<UNDERLYING>-<DDMonYY>-<STRIKE>-CE` | 57,400 | `NSE-NIFTY-01Apr20-10000-CE` |
//! | `<EX>-<UNDERLYING>-<DDMonYY>-<STRIKE>-PE` | 58,527 | `NSE-BANKNIFTY-01Apr20-13900-PE` |
//! | `<EX>-<UNDERLYING>-<DDMonYY>-FUT` | 159 | `NSE-BANKNIFTY-24Apr24-FUT` |
//!
//! The futures shape has **no strike** and four parts rather than five. It is
//! easy to miss — it is 0.14% of the tree — and a parser that assumed five
//! parts would refuse 159 real contracts.
//!
//! # What this does about case
//!
//! The month is the one field the lake writes in mixed case: `01Apr20`, never
//! `01APR20`. Every other field is upper case — the exchange, the underlying,
//! and the `CE`/`PE`/`FUT` suffix.
//!
//! So: **the three-letter month is matched case-insensitively; every other
//! field must be exactly as the lake writes it.** A tolerance is granted only
//! where the data actually varies, because a parser that shrugs at
//! `nse-nifty-01apr20-10000-ce` would also shrug at a writer quietly changing
//! its conventions, and `CLAUDE.md` §4 bans the fallback that hides a change.
//! [`Display`](std::fmt::Display) always renders the canonical spelling, so a
//! name taken from the lake round-trips byte for byte.
//!
//! **The month is the only tolerance, and the test for whether one is allowed
//! is injectivity, not convenience.** Case folding is injective over the twelve
//! month tokens, so no two contract names can ever collapse onto one identity
//! through it. A leading zero on the strike is not: `010000` and `10000` are
//! two different directory names, and accepting both would give them one
//! `InstrumentKey` — the identity `CLAUDE.md` §3 rule 3 hashes into a run — and
//! render one of them back as a path that does not exist. It is refused, in
//! `parse_strike`, for that reason and not for tidiness.
//!
//! Nothing here widens the engine surface. `CLAUDE.md` §1 fixes what is
//! *swept* at two NSE indices; futures and options may be **stored**, and this
//! module only reads what is stored.

use core::fmt;

use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind, OptionSide, Segment};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;

use crate::error::ContractError;

/// The twelve month tokens, in the lake's own spelling, index + 1 = month.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The longest contract name this parser will look at, in bytes.
///
/// The longest real name in the lake is 30 bytes
/// (`NSE-BANKNIFTY-01Apr20-13900-PE`), so 64 is generous by a factor of two.
///
/// **This bound is not decoration, and it was added because a bench measured
/// its absence.** Without it, deciding that a 4 KiB name is malformed cost
/// 33.6x what parsing a real one costs — the digit check walked the whole
/// string and the error then allocated a copy of it. That is unbounded work
/// spent on input that was never going to be accepted, and a caller walking
/// 116,086 directories is exactly who pays for it. `crates/core`'s
/// `InstrumentError::FieldTooWide` exists for the same reason, and
/// `docs/05-decisions.md` D-0033 is the precedent: a refusal to spend
/// unbounded time deciding is itself a decision worth naming.
const MAX_NAME_BYTES: usize = 64;

/// Paisa per rupee, for the strike. Integer arithmetic only.
///
/// Every strike in the lake is a whole number of rupees — verified across all
/// 115,927 option directories, none of which carries a decimal point. So the
/// strike never goes near [`Paisa::from_rupees_half_up`]: it is multiplied by
/// 100 as an integer, which cannot round and cannot lose a paisa.
const PAISA_PER_RUPEE: i64 = 100;

/// Whether a contract is an option or a future, with the fields that kind
/// requires and no others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractKind {
    /// A futures contract. No strike, no side.
    Future,
    /// An options contract.
    Option {
        /// Strike, in paisa.
        strike: Paisa,
        /// Call or put.
        side: OptionSide,
    },
}

/// A parsed lake contract directory name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractName {
    exchange: Exchange,
    underlying: Symbol,
    expiry: Expiry,
    kind: ContractKind,
}

impl ContractName {
    /// Parses a lake contract directory name.
    ///
    /// # Errors
    ///
    /// A [`ContractError`] naming the part that failed. Every failure is a
    /// refusal: this never returns a partially understood contract, because a
    /// contract filed under the wrong expiry or the wrong side is worse than
    /// one that failed to load.
    ///
    /// # Examples
    ///
    /// ```
    /// use lake::contract::ContractName;
    /// let c = ContractName::parse("NSE-NIFTY-01Apr20-10000-CE")?;
    /// assert_eq!(c.to_string(), "NSE-NIFTY-01Apr20-10000-CE");
    /// # Ok::<(), lake::error::ContractError>(())
    /// ```
    pub fn parse(name: &str) -> Result<Self, ContractError> {
        // Bound the work BEFORE anything walks the string. Everything below
        // this line is therefore bounded by MAX_NAME_BYTES rather than by
        // whatever the caller handed over.
        if name.len() > MAX_NAME_BYTES {
            return Err(ContractError::TooLong { len: name.len() });
        }
        let mut parts = name.split('-');
        let (Some(ex), Some(under), Some(exp), Some(fourth)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ContractError::WrongPartCount {
                found: name.split('-').count(),
            });
        };
        let fifth = parts.next();
        if parts.next().is_some() {
            return Err(ContractError::WrongPartCount {
                found: name.split('-').count(),
            });
        }

        let exchange = Exchange::parse(ex).map_err(|_| ContractError::UnknownExchange {
            found: ex.to_owned(),
        })?;
        let underlying = Symbol::new(under).map_err(|_| ContractError::BadUnderlying {
            found: under.to_owned(),
        })?;
        let expiry = parse_expiry(exp)?;

        let kind = match fifth {
            // Four parts: the fourth and last token must be FUT.
            None => {
                if fourth == "FUT" {
                    ContractKind::Future
                } else {
                    return Err(ContractError::UnknownSide {
                        found: fourth.to_owned(),
                    });
                }
            }
            // Five parts: strike then side.
            Some(side) => ContractKind::Option {
                strike: parse_strike(fourth)?,
                side: parse_side(side)?,
            },
        };

        Ok(Self {
            exchange,
            underlying,
            expiry,
            kind,
        })
    }

    /// The exchange.
    #[must_use]
    pub const fn exchange(&self) -> Exchange {
        self.exchange
    }

    /// The underlying symbol — `NIFTY`, not the whole contract name.
    #[must_use]
    pub const fn underlying(&self) -> &Symbol {
        &self.underlying
    }

    /// The expiry date.
    #[must_use]
    pub const fn expiry(&self) -> Expiry {
        self.expiry
    }

    /// Whether this is a future or an option, with that kind's fields.
    #[must_use]
    pub const fn kind(&self) -> ContractKind {
        self.kind
    }

    /// The workspace's canonical instrument identity for this contract.
    ///
    /// Returning `brutex_core`'s own key rather than a second identity type is
    /// the point: `crates/store` and `crates/pull` already hash and compare
    /// instruments through it, and a parallel identity in this crate could
    /// disagree with theirs about what "the same contract" means.
    #[must_use]
    pub const fn key(&self) -> InstrumentKey {
        InstrumentKey {
            exchange: self.exchange,
            segment: Segment::Fno,
            underlying: self.underlying,
            kind: match self.kind {
                ContractKind::Future => Kind::Future {
                    expiry: self.expiry,
                },
                ContractKind::Option { strike, side } => Kind::Option {
                    expiry: self.expiry,
                    strike,
                    side,
                },
            },
        }
    }
}

impl fmt::Display for ContractName {
    /// Renders the canonical lake spelling, so a real name round-trips.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let month = MONTHS
            .get(usize::from(self.expiry.month()).saturating_sub(1))
            .copied()
            .unwrap_or("???");
        write!(
            f,
            "{}-{}-{:02}{}{:02}-",
            self.exchange.as_str(),
            self.underlying.as_str(),
            self.expiry.day(),
            month,
            self.expiry.year() % 100,
        )?;
        match self.kind {
            ContractKind::Future => f.write_str("FUT"),
            ContractKind::Option { strike, side } => {
                write!(f, "{}-{}", strike.raw() / PAISA_PER_RUPEE, side.as_str())
            }
        }
    }
}

/// Parses `DDMonYY`, for example `01Apr20`.
///
/// The two-digit year is read as 2000 + YY. The lake begins in 2020 and no
/// contract in it predates that, so no 19xx expiry can arise; the window this
/// convention can express is 2000..=2099 and it is stated rather than assumed.
fn parse_expiry(text: &str) -> Result<Expiry, ContractError> {
    let bad = || ContractError::BadExpiryShape {
        found: text.to_owned(),
    };
    // Fixed widths. A token like `1Apr20` is refused rather than guessed at.
    if text.len() != 7 {
        return Err(bad());
    }
    let (Some(d), Some(m), Some(y)) = (text.get(0..2), text.get(2..5), text.get(5..7)) else {
        return Err(bad());
    };
    let day: u8 = d.parse().map_err(|_| bad())?;
    let yy: u16 = y.parse().map_err(|_| bad())?;
    let month = parse_month(m)?;
    let year = 2000 + yy;

    Expiry::new(year, month, day).map_err(|_| ContractError::ImpossibleDate { day, month, year })
}

/// The three-letter month, matched case-insensitively. See the module header.
fn parse_month(text: &str) -> Result<u8, ContractError> {
    for (i, m) in MONTHS.iter().enumerate() {
        if m.eq_ignore_ascii_case(text) {
            // The index is 0..12, so this cannot truncate.
            let n = u8::try_from(i + 1).map_err(|_| ContractError::BadMonth {
                found: text.to_owned(),
            })?;
            return Ok(n);
        }
    }
    Err(ContractError::BadMonth {
        found: text.to_owned(),
    })
}

/// Parses a whole-rupee strike into paisa, by integer arithmetic.
fn parse_strike(text: &str) -> Result<Paisa, ContractError> {
    let bad = || ContractError::BadStrike {
        found: text.to_owned(),
    };
    // `u64::from_str` accepts a leading `+`, which is not a spelling the lake
    // uses; require plain digits so a novel spelling is a refusal.
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    // A LEADING ZERO IS EXACTLY AS NOVEL AS `+1000`, AND IT IS WORSE.
    //
    // The month's case tolerance above is normalisation and is safe because
    // case folding is injective over the twelve month tokens: no two contracts
    // can collapse through it. A leading zero is not injective. `010000` and
    // `10000` are two different directory names that would parse to one
    // `ContractName`, therefore one `InstrumentKey` — the workspace's
    // canonical instrument identity, which `CLAUDE.md` §3 rule 3 puts inside
    // the run-identity hash. Two directories may not become one instrument.
    //
    // It also breaks the round trip this module's header promises: `Display`
    // would render `010000` back as `10000`, so a name taken from the lake and
    // rendered again would no longer name the directory it came from.
    //
    // Measured across all 116,086 NSE and 33,199 BSE F&O directories: zero
    // strikes carry a leading zero, so this refuses no real name. It also
    // subsumes the old `rupees == 0` check — `0` and `00` start with `0` — and
    // that check is gone rather than left behind as a branch nothing reaches.
    if text.starts_with('0') {
        return Err(bad());
    }
    let rupees: u64 = text.parse().map_err(|_| bad())?;
    let paisa = i64::try_from(rupees)
        .ok()
        .and_then(|r| r.checked_mul(PAISA_PER_RUPEE))
        .ok_or(ContractError::StrikeNotRepresentable { rupees })?;
    Ok(Paisa::from_raw(paisa))
}

/// Parses the `CE` / `PE` suffix.
fn parse_side(text: &str) -> Result<OptionSide, ContractError> {
    match text {
        "CE" => Ok(OptionSide::Call),
        "PE" => Ok(OptionSide::Put),
        _ => Err(ContractError::UnknownSide {
            found: text.to_owned(),
        }),
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes"
)]
mod tests {
    use super::*;

    /// Names taken verbatim from `~/.brutex/lake/bars/*/FNO/`.
    const REAL: [&str; 6] = [
        "NSE-NIFTY-01Apr20-10000-CE",
        "NSE-BANKNIFTY-01Apr20-13900-PE",
        "NSE-BANKNIFTY-24Apr24-FUT",
        "NSE-BANKNIFTY-23Feb23-FUT",
        "BSE-SENSEX-01Apr25-68000-CE",
        "NSE-NIFTY-24Dec24-FUT",
    ];

    #[test]
    fn every_real_name_round_trips_byte_for_byte() {
        for name in REAL {
            let c = ContractName::parse(name)
                .unwrap_or_else(|e| panic!("real lake name {name} refused: {e}"));
            assert_eq!(c.to_string(), name, "round trip changed {name}");
        }
    }

    #[test]
    fn the_sample_option_parses_to_its_parts() {
        let c = ContractName::parse("NSE-NIFTY-01Apr20-10000-CE").unwrap();
        assert_eq!(c.exchange(), Exchange::Nse);
        assert_eq!(c.underlying().as_str(), "NIFTY");
        assert_eq!(c.expiry(), Expiry::new(2020, 4, 1).unwrap());
        match c.kind() {
            ContractKind::Option { strike, side } => {
                // 10000 rupees is 1,000,000 paisa, exactly.
                assert_eq!(strike.raw(), 1_000_000);
                assert_eq!(side, OptionSide::Call);
            }
            ContractKind::Future => panic!("a CE is not a future"),
        }
    }

    #[test]
    fn a_future_has_four_parts_and_no_strike() {
        let c = ContractName::parse("NSE-BANKNIFTY-24Apr24-FUT").unwrap();
        assert_eq!(c.kind(), ContractKind::Future);
        assert_eq!(c.expiry(), Expiry::new(2024, 4, 24).unwrap());
        assert_eq!(c.underlying().as_str(), "BANKNIFTY");
    }

    #[test]
    fn the_key_is_the_workspace_identity() {
        let c = ContractName::parse("NSE-NIFTY-01Apr20-10000-CE").unwrap();
        let k = c.key();
        assert_eq!(k.exchange, Exchange::Nse);
        assert_eq!(k.segment, Segment::Fno);
        assert_eq!(k.underlying.as_str(), "NIFTY");
        assert!(matches!(
            k.kind,
            Kind::Option {
                side: OptionSide::Call,
                ..
            }
        ));

        let f = ContractName::parse("NSE-BANKNIFTY-24Apr24-FUT")
            .unwrap()
            .key();
        assert!(matches!(f.kind, Kind::Future { .. }));
    }

    #[test]
    fn the_month_is_case_insensitive_but_nothing_else_is() {
        // The month tolerates case, because that is the field the lake itself
        // writes in mixed case.
        for spelling in ["01Apr20", "01APR20", "01apr20", "01aPr20"] {
            let name = format!("NSE-NIFTY-{spelling}-10000-CE");
            let c = ContractName::parse(&name)
                .unwrap_or_else(|e| panic!("{spelling} should parse: {e}"));
            assert_eq!(c.expiry(), Expiry::new(2020, 4, 1).unwrap());
            // ...and always renders back canonically.
            assert_eq!(c.to_string(), "NSE-NIFTY-01Apr20-10000-CE");
        }

        // Every other field is exact.
        assert!(matches!(
            ContractName::parse("nse-NIFTY-01Apr20-10000-CE"),
            Err(ContractError::UnknownExchange { .. })
        ));
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20-10000-ce"),
            Err(ContractError::UnknownSide { .. })
        ));
        assert!(matches!(
            ContractName::parse("NSE-BANKNIFTY-24Apr24-fut"),
            Err(ContractError::UnknownSide { .. })
        ));
    }

    #[test]
    fn a_malformed_name_is_refused_by_name() {
        // Too few parts.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20"),
            Err(ContractError::WrongPartCount { found: 3 })
        ));
        // Too many parts.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20-10000-CE-EXTRA"),
            Err(ContractError::WrongPartCount { found: 6 })
        ));
        // Not an exchange.
        assert!(matches!(
            ContractName::parse("MCX-NIFTY-01Apr20-10000-CE"),
            Err(ContractError::UnknownExchange { .. })
        ));
        // Expiry that is the wrong width.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-1Apr20-10000-CE"),
            Err(ContractError::BadExpiryShape { .. })
        ));
        // Not a month.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Zzz20-10000-CE"),
            Err(ContractError::BadMonth { .. })
        ));
        // A date that does not exist: 31 February must not become 3 March.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-31Feb21-10000-CE"),
            Err(ContractError::ImpossibleDate {
                day: 31,
                month: 2,
                year: 2021
            })
        ));
        // 29 February in a non-leap year, which is the case a naive
        // days-in-month table gets wrong.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-29Feb21-10000-CE"),
            Err(ContractError::ImpossibleDate { .. })
        ));
        // ...but a real leap day parses.
        assert!(ContractName::parse("NSE-NIFTY-29Feb24-10000-CE").is_ok());
        // Strike that is not digits.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20-10O00-CE"),
            Err(ContractError::BadStrike { .. })
        ));
        // Negative / signed strike.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20-+1000-CE"),
            Err(ContractError::BadStrike { .. })
        ));
        // Zero strike is not a real contract, however it is spelled.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20-0-CE"),
            Err(ContractError::BadStrike { .. })
        ));
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20-00-CE"),
            Err(ContractError::BadStrike { .. })
        ));
        // Empty strike.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20--CE"),
            Err(ContractError::BadStrike { .. })
        ));
        // Bad side.
        assert!(matches!(
            ContractName::parse("NSE-NIFTY-01Apr20-10000-XX"),
            Err(ContractError::UnknownSide { .. })
        ));
        // Empty underlying.
        assert!(matches!(
            ContractName::parse("NSE--01Apr20-10000-CE"),
            Err(ContractError::BadUnderlying { .. })
        ));
        // Entirely the wrong thing.
        assert!(ContractName::parse("").is_err());
        assert!(ContractName::parse("hello").is_err());
    }

    #[test]
    fn an_over_long_name_is_refused_before_anything_walks_it() {
        // The case the bench found: a 4 KiB strike. Refused on length, so no
        // digit check and no allocation of a copy of it ever happens.
        let absurd = format!("NSE-NIFTY-01Apr20-{}-CE", "9".repeat(4096));
        match ContractName::parse(&absurd) {
            Err(ContractError::TooLong { len }) => assert_eq!(len, absurd.len()),
            other => panic!("expected TooLong, got {other:?}"),
        }

        // The bound is generous: every real shape is far inside it.
        for name in REAL {
            assert!(
                name.len() <= MAX_NAME_BYTES,
                "{name} is {} bytes, over the bound",
                name.len()
            );
        }

        // Exactly at the bound is accepted as far as the length check goes —
        // it fails later, on its content, which is a different refusal.
        let at_bound = "N".repeat(MAX_NAME_BYTES);
        assert!(!matches!(
            ContractName::parse(&at_bound),
            Err(ContractError::TooLong { .. })
        ));
        // One byte over is refused on length.
        let over = "N".repeat(MAX_NAME_BYTES + 1);
        assert!(matches!(
            ContractName::parse(&over),
            Err(ContractError::TooLong { .. })
        ));
    }

    #[test]
    fn a_strike_too_large_for_paisa_is_refused_rather_than_wrapped() {
        let huge = format!("NSE-NIFTY-01Apr20-{}-CE", u64::MAX);
        assert!(matches!(
            ContractName::parse(&huge),
            Err(ContractError::StrikeNotRepresentable { .. })
        ));
        // One rupee above the i64 paisa ceiling.
        let over = format!("NSE-NIFTY-01Apr20-{}-CE", (i64::MAX / 100) + 1);
        assert!(matches!(
            ContractName::parse(&over),
            Err(ContractError::StrikeNotRepresentable { .. })
        ));
    }

    #[test]
    fn the_observed_strike_range_converts_exactly() {
        // 4600 and 69000 are the minimum and maximum strikes present in the
        // lake, measured across every option directory.
        for (rupees, paisa) in [(4600_i64, 460_000_i64), (69_000, 6_900_000)] {
            let name = format!("NSE-NIFTY-01Apr20-{rupees}-CE");
            let c = ContractName::parse(&name).unwrap();
            match c.kind() {
                ContractKind::Option { strike, .. } => assert_eq!(strike.raw(), paisa),
                ContractKind::Future => panic!("not a future"),
            }
            assert_eq!(c.to_string(), name, "round trip");
        }
    }

    #[test]
    fn every_month_token_maps_to_its_number() {
        for (i, m) in MONTHS.iter().enumerate() {
            let want = u8::try_from(i + 1).unwrap();
            assert_eq!(parse_month(m).unwrap(), want, "{m}");
        }
        assert!(parse_month("Xyz").is_err());
        assert!(parse_month("Ap").is_err());
    }

    /// **The month's case tolerance is normalisation, and the property that
    /// makes it one is injectivity.**
    ///
    /// A tolerance is safe only if no two distinct contract names can collapse
    /// onto one identity through it. Case folding over the twelve month tokens
    /// is injective — the twelve lower-cased spellings are still twelve — so
    /// every spelling of a month reaches the same expiry and no other
    /// contract's. That is why this tolerance is granted and the leading-zero
    /// strike below is not.
    #[test]
    fn the_month_case_tolerance_is_injective_and_can_never_alias_two_contracts() {
        let folded: std::collections::BTreeSet<String> =
            MONTHS.iter().map(|m| m.to_ascii_lowercase()).collect();
        assert_eq!(folded.len(), 12, "case folding must stay injective");

        // Every spelling of one month renders back to the lake's own form.
        for spelling in ["01Apr20", "01APR20", "01apr20", "01aPr20"] {
            let name = format!("NSE-NIFTY-{spelling}-10000-CE");
            let c = ContractName::parse(&name).expect("a month spelling parses");
            assert_eq!(c.to_string(), "NSE-NIFTY-01Apr20-10000-CE");
        }

        // And two different months never meet, at any casing.
        let mut seen = std::collections::BTreeSet::new();
        for m in MONTHS {
            let c = ContractName::parse(&format!("NSE-NIFTY-01{}20-10000-CE", m.to_uppercase()))
                .expect("an upper-cased month parses");
            assert!(seen.insert(c.key()), "{m} collided with another month");
        }
        assert_eq!(seen.len(), 12);
    }

    /// **A leading-zero strike is refused, because unlike the month it is NOT
    /// injective.**
    ///
    /// `010000` and `10000` are two different lake directory names. Accepting
    /// both gave them one `ContractName` and therefore one `InstrumentKey` —
    /// the workspace's canonical instrument identity, which `CLAUDE.md` §3
    /// rule 3 hashes into every run identity — and rendered `010000` back as
    /// `10000`, a path that is not the directory the name came from.
    ///
    /// `parse_strike` already refused `+1000` on the stated ground that a
    /// novel spelling is a refusal. A leading zero is exactly as novel and
    /// strictly more dangerous, and it was accepted.
    #[test]
    fn a_leading_zero_strike_is_refused_rather_than_aliased_onto_another_contract() {
        let canonical = ContractName::parse("NSE-NIFTY-01Apr20-10000-CE").expect("canonical");

        for padded in [
            "NSE-NIFTY-01Apr20-010000-CE",
            "NSE-NIFTY-01Apr20-0010000-CE",
            "NSE-NIFTY-01Apr20-04600-PE",
        ] {
            match ContractName::parse(padded) {
                Err(ContractError::BadStrike { found }) => {
                    assert!(found.starts_with('0'), "the refusal names it: {found}");
                }
                other => panic!("{padded} must be refused, got {other:?}"),
            }
        }

        // The unpadded spelling is untouched, and it is the one the lake
        // writes.
        assert_eq!(canonical.to_string(), "NSE-NIFTY-01Apr20-10000-CE");

        // Every accepted strike round-trips, which is the property the
        // tolerance broke: no accepted name renders back as a different one.
        for rupees in [1_u32, 9, 10, 4_600, 10_000, 69_000] {
            let name = format!("NSE-NIFTY-01Apr20-{rupees}-CE");
            let c = ContractName::parse(&name).expect("a plain strike parses");
            assert_eq!(c.to_string(), name);
        }
    }
}

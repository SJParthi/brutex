//! THE CALLER `crate::fno` NEVER HAD — expiries, then contracts, then bars.
//!
//! # Why this exists
//!
//! `crate::fno` has been able to build both discovery requests and read both
//! answers since it was written, and **nothing in this workspace called it**.
//! `POST /pull/fno` parsed a request in full and then answered 503. Every
//! expired future and every expired option was unreachable, on every feed, at
//! every timeframe — not because the code was missing but because no line
//! joined the two halves that existed.
//!
//! This is that line.
//!
//! # It needs no store format change, and that is the point
//!
//! Groww's historical candles are seven elements — timestamp, open, high, low,
//! close, volume, open interest — and nothing else: no implied volatility, no
//! spot (`Groww Docs/11-backtesting.md`). That is exactly what
//! `store::format::Bar` already holds. So a contract discovered here lands
//! through the SAME `crate::fetch` path a spot series does, into the same
//! record, with no new field and no new version.
//!
//! Dhan is the feed that carries implied volatility and spot, and it is
//! addressed by strike offset rather than by name — a different driver, and the
//! one that needs the wider record. Keeping the two apart is what lets this one
//! ship first. See D-0193 and D-0195.
//!
//! # The order, and why it is two calls
//!
//! `expiries` answers which dates a contract expired on for one underlying in
//! one month; `contracts` answers which contracts existed for ONE of those
//! dates. The second is fed by the first because the vendor keys it that way —
//! there is no call returning a month's contracts directly.
//!
//! # A name this build cannot read is REPORTED, never skipped
//!
//! [`Chain::unreadable`] carries every discovered name
//! [`crate::fno::read_contract`] declined. A contract dropped in silence is a
//! month that looks complete and is not, which is the only class of failure
//! that quietly corrupts a backtest years later — and it is the failure this
//! whole repository is most careful about.
//!
//! # Cost
//!
//! One request per (underlying, month) for the expiries, and one per expiry for
//! the contracts. Both are properties of the ASK: nothing here reads a census,
//! lists a directory, or grows with what the store already holds. Each parse is
//! one pass over the answer.
//!
//! # It opens no socket
//!
//! [`Discovery`] is a port, the same shape `crate::fetch::BarSource` is, so
//! every arm below is drivable from a test with no network. The adapter that
//! makes a live call is one `impl` in the crate that already owns a client.

use crate::fetch::BarRequest;
use crate::fno::{self, Ask, FnoError, Found};
use crate::session::Window;
use crate::vendor::{Feed, Granularity, HttpSpec, Listing, Transport};

/// Where a discovery answer comes from.
///
/// One method, because a discovery request is one GET whose body is JSON. The
/// implementor is responsible for the credential; this module never sees one.
pub trait Discovery {
    /// Fetches one discovery URL and returns its body.
    ///
    /// # Async, natively
    ///
    /// The real adapter is `reqwest`, which is async, and the caller is an axum
    /// handler, which is async. A blocking bridge between them would park a
    /// runtime thread for the length of a vendor round trip — and with feeds
    /// now running concurrently, several at once. Rust 1.97 takes `async fn` in
    /// a trait directly, so there is no bridge and no `async-trait` dependency.
    ///
    /// # Errors
    ///
    /// Whatever the transport refuses, in the host's own words.
    fn get(&self, url: &str) -> impl core::future::Future<Output = Result<String, String>>;
}

/// Why a chain could not be walked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// This feed publishes no contract NAME to discover.
    ///
    /// A fact about the vendor rather than a gap here, and NOT the same as "no
    /// expired data": Dhan serves five years of expired options addressed by
    /// strike offset, which is a different driver. D-0193 records the months
    /// this repository spent conflating the two.
    NoNameLookup {
        /// Which feed was asked.
        feed: &'static str,
    },
    /// The feed is a local archive; there is no vendor to ask.
    NotAnHttpFeed {
        /// Which feed was asked.
        feed: &'static str,
    },
    /// The discovery request could not be built, or its answer not read.
    Lookup(FnoError),
    /// The transport refused, and this is its reason verbatim.
    Transport {
        /// Which URL.
        url: String,
        /// Why, unparaphrased.
        why: String,
    },
}

impl core::fmt::Display for ChainError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoNameLookup { feed } => write!(
                f,
                "{feed} publishes no contract name to discover, so there is \
                 nothing to look up and nothing was sent. That is not the same \
                 as serving no expired data — a feed addressed by strike offset \
                 is pulled by a different driver."
            ),
            Self::NotAnHttpFeed { feed } => write!(
                f,
                "{feed} is a folder of files, not a vendor. Its expired \
                 contracts are whatever the archive holds and are read from \
                 disk, never discovered."
            ),
            Self::Lookup(why) => write!(f, "{why}"),
            Self::Transport { url, why } => write!(f, "{url} was not reached: {why}"),
        }
    }
}

/// One month's contracts for one underlying, and what could not be read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chain {
    /// Every expiry date the vendor named for this (underlying, month).
    pub expiries: Vec<String>,
    /// Every contract this build could read into its own identity.
    pub contracts: Vec<Found>,
    /// Every discovered name this build could NOT read, kept verbatim.
    ///
    /// Never empty-and-ignored: a caller that does not report these is
    /// reporting a month as complete while some of its contracts were dropped
    /// on the floor.
    pub unreadable: Vec<String>,
}

impl Chain {
    /// Whether every discovered name was understood.
    #[must_use]
    pub fn whole(&self) -> bool {
        self.unreadable.is_empty()
    }
}

/// The discovery this feed publishes, or why it publishes none.
fn lookup_of(feed: Feed) -> Result<HttpSpec, ChainError> {
    let descriptor = feed.descriptor();
    let Transport::Http(spec) = descriptor.transport else {
        return Err(ChainError::NotAnHttpFeed {
            feed: descriptor.display,
        });
    };
    if spec.fno.by_name().is_none() {
        return Err(ChainError::NoNameLookup {
            feed: descriptor.display,
        });
    }
    Ok(spec)
}

/// Walks one (underlying, year, month) into every contract it held.
///
/// # Errors
///
/// [`ChainError`] for a feed that cannot be asked, a request that cannot be
/// built, or a transport that refused. A name that cannot be READ is not an
/// error — it lands in [`Chain::unreadable`], because one unreadable contract
/// must not discard the month's other two hundred.
///
/// # Cost
///
/// One request for the expiries, then one per expiry. Nothing scans the store.
pub async fn month<D: Discovery>(feed: Feed, ask: &Ask, from: &D) -> Result<Chain, ChainError> {
    let spec = lookup_of(feed)?;
    let field = spec.fno.by_name().map_or("expiries", |d| d.expiries_field);
    let contracts_field = spec
        .fno
        .by_name()
        .map_or("contracts", |d| d.contracts_field);

    let url = fno::expiries_url(&spec, ask).map_err(ChainError::Lookup)?;
    let body = from.get(&url).await.map_err(|why| ChainError::Transport {
        url: url.clone(),
        why,
    })?;
    let expiries = fno::names(&body, field).map_err(ChainError::Lookup)?;

    let mut chain = Chain {
        expiries: expiries.clone(),
        ..Chain::default()
    };

    for expiry in expiries {
        // THE SECOND CALL IS KEYED ON THE FIRST'S ANSWER. `Ask::expiry` is the
        // only field that changes between the two, which is why `fno` takes one
        // `Ask` rather than two shapes.
        let keyed = Ask {
            underlying: ask.underlying.clone(),
            year: ask.year,
            month: ask.month,
            expiry,
        };
        let url = fno::contracts_url(&spec, &keyed).map_err(ChainError::Lookup)?;
        let body = from.get(&url).await.map_err(|why| ChainError::Transport {
            url: url.clone(),
            why,
        })?;
        for name in fno::names(&body, contracts_field).map_err(ChainError::Lookup)? {
            match fno::read_contract(&name) {
                Some(found) => chain.contracts.push(found),
                // REPORTED, NEVER SKIPPED. See the module header.
                None => chain.unreadable.push(name),
            }
        }
    }
    Ok(chain)
}

/// Turns one discovered contract into the bars request that fetches it.
///
/// # Why the vendor's own symbol and not this store's name
///
/// [`BarRequest::instrument_id`] is what goes ON THE WIRE, and the vendor knows
/// its own spelling — `NSE-NIFTY-30Sep25-24650-CE`. The store's name for the
/// same contract is built separately, from [`Found::contract`], at the point
/// the bars are filed. Conflating them is how one vendor's id ends up in
/// another's request.
#[must_use]
pub fn request(found: &Found, window: Window, granularity: Granularity) -> BarRequest {
    BarRequest {
        instrument_id: found.vendor_symbol.clone(),
        // A CONTRACT IS A DERIVATIVE, which is the listing class D-0170 added
        // and the reason it was added: the venue that governs its session is
        // NSE's derivatives clock, not the cash one, and those two stopped
        // agreeing on 2026-08-03.
        listing: Listing::Derivative,
        window,
        granularity,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    /// A transport that answers from memory, in the order it was given.
    struct Canned {
        answers: std::cell::RefCell<Vec<String>>,
    }

    impl Discovery for Canned {
        async fn get(&self, _url: &str) -> Result<String, String> {
            let mut left = self.answers.borrow_mut();
            if left.is_empty() {
                return Err("no answer left".to_owned());
            }
            Ok(left.remove(0))
        }
    }

    fn ask() -> Ask {
        Ask {
            underlying: "NIFTY".to_owned(),
            year: 2024,
            month: 1,
            expiry: String::new(),
        }
    }

    /// A feed with no NAME lookup refuses by name, and says what it IS.
    ///
    /// The sentence matters as much as the refusal: D-0193 exists because "no
    /// name lookup" was read as "no expired data" for months, so this refusal
    /// has to say the difference out loud.
    #[tokio::test]
    async fn a_feed_without_a_name_lookup_refuses_and_does_not_claim_it_serves_nothing() {
        let Err(why) = month(
            Feed::Dhan,
            &ask(),
            &Canned {
                answers: std::cell::RefCell::new(vec![]),
            },
        )
        .await
        else {
            panic!("Dhan publishes no contract name to discover");
        };
        assert!(matches!(why, ChainError::NoNameLookup { .. }));
        let said = why.to_string();
        assert!(said.contains("no contract name to discover"), "{said}");
        assert!(
            said.contains("not the same as serving no expired data"),
            "the refusal must not be read as a capability claim: {said}"
        );
    }

    /// An archive feed is refused as what it is: a folder, not a vendor.
    #[tokio::test]
    async fn an_archive_feed_has_no_vendor_to_ask() {
        let Err(why) = month(
            Feed::TrueData,
            &ask(),
            &Canned {
                answers: std::cell::RefCell::new(vec![]),
            },
        )
        .await
        else {
            panic!("an archive has no vendor to publish a lookup");
        };
        assert!(matches!(why, ChainError::NotAnHttpFeed { .. }));
        assert!(why.to_string().contains("folder of files"), "{why}");
    }

    /// The two calls chain, and every readable contract comes back.
    #[tokio::test]
    async fn expiries_feed_contracts_and_every_readable_name_is_kept() {
        let canned = Canned {
            answers: std::cell::RefCell::new(vec![
                r#"{"expiries":["2024-01-25"]}"#.to_owned(),
                r#"{"contracts":["NSE-NIFTY-25Jan24-21000-CE","NSE-NIFTY-25Jan24-21000-PE","NSE-NIFTY-25Jan24-FUT"]}"#.to_owned(),
            ]),
        };
        let chain = month(Feed::Groww, &ask(), &canned)
            .await
            .expect("Groww is asked by name");
        assert_eq!(chain.expiries, vec!["2024-01-25".to_owned()]);
        assert_eq!(chain.contracts.len(), 3, "two options and one future");
        assert!(chain.whole(), "every name was readable");
        assert_eq!(
            chain.contracts[0].vendor_symbol, "NSE-NIFTY-25Jan24-21000-CE",
            "the vendor's own spelling is what goes back on the wire"
        );
    }

    /// A name this build cannot read is REPORTED and does not discard the rest.
    ///
    /// The whole point of the module. A contract dropped in silence is a month
    /// that looks complete and is not.
    #[tokio::test]
    async fn an_unreadable_name_is_reported_and_the_readable_ones_still_land() {
        let canned = Canned {
            answers: std::cell::RefCell::new(vec![
                r#"{"expiries":["2024-01-25"]}"#.to_owned(),
                r#"{"contracts":["NSE-NIFTY-25Jan24-21000-CE","!! not a contract !!"]}"#.to_owned(),
            ]),
        };
        let chain = month(Feed::Groww, &ask(), &canned)
            .await
            .expect("the call itself succeeded");
        assert_eq!(chain.contracts.len(), 1, "the readable one landed");
        assert_eq!(
            chain.unreadable,
            vec!["!! not a contract !!".to_owned()],
            "and the unreadable one is carried, never dropped"
        );
        assert!(
            !chain.whole(),
            "so the caller cannot report this month as complete"
        );
    }

    /// A transport refusal stops the walk and carries the host's own words.
    #[tokio::test]
    async fn a_transport_refusal_names_the_url_and_the_reason() {
        let Err(why) = month(
            Feed::Groww,
            &ask(),
            &Canned {
                answers: std::cell::RefCell::new(vec![]),
            },
        )
        .await
        else {
            panic!("nothing answered, so the walk cannot have succeeded");
        };
        match why {
            ChainError::Transport { url, why } => {
                assert!(url.contains("expiries"), "the failing url is named: {url}");
                assert_eq!(why, "no answer left");
            }
            other => panic!("expected a transport refusal, got {other:?}"),
        }
    }
}

//! Kite Connect v3's `error_type` names, and nothing generic.
//!
//! One vendor's vocabulary. The mechanism that uses it —
//! [`crate::refusal::Disposition`], the two-axis resolution, the O(1)
//! argument — is in [`crate::refusal`], and this module is the row it reads.
//! [`KITE`] is the whole of the connection between them: one JSON field name,
//! one reader, one citation.
//!
//! # Source
//!
//! <https://kite.trade/docs/connect/v3/exceptions/>, read 19 Aug 2026, plus
//! one name that page does not carry — see [`KiteException::Permission`].

use crate::refusal::{Disposition, ErrorNames};
use core::fmt;

/// How many `error_type` names this module knows by name.
///
/// Nine from the vendor's exceptions page, plus `PermissionException` from the
/// vendor's own Python SDK.
pub const EXCEPTION_COUNT: usize = 10;

/// Kite's body-level error contract, as [`crate::vendor::HttpSpec`] carries it.
///
/// The whole of the coupling between this vendor's vocabulary and the generic
/// machinery: the field to look in, the function that reads it, and where both
/// were read from.
pub const KITE: ErrorNames = ErrorNames {
    field: "error_type",
    read,
    source: "kite.trade/docs/connect/v3/exceptions/, read 19 Aug 2026 — nine \
             names — plus PermissionException from the vendor's own SDK, \
             github.com/zerodha/pykiteconnect kiteconnect/exceptions.py",
};

/// One `error_type` value, as Kite spells it on the wire.
///
/// # Ten variants, and the last one is not on the documentation page
///
/// [`Self::Token`] through [`Self::General`] are the nine published at
/// <https://kite.trade/docs/connect/v3/exceptions/>, in the page's own order,
/// and their discriminants are that order. [`Self::Permission`] is the tenth:
/// it is **not** on that page and it is live in
/// [`pykiteconnect/exceptions.py`](https://github.com/zerodha/pykiteconnect/blob/master/kiteconnect/exceptions.py)
/// — *"Represents permission denied exceptions for certain calls. Default code
/// is 403."* — and it is what an API key without a historical-data
/// subscription is answered with.
///
/// It is carried here because leaving it out is what makes an unentitled key
/// indistinguishable from an expired session; see [`crate::refusal`]'s header.
///
/// # There is no `Unknown` variant, and that is the point
///
/// A name this module does not know must reach the operator **as the vendor
/// spelled it**. A variant would consume it and answer a classification nobody
/// made. [`Self::from_wire`] answers `None` instead and
/// [`crate::refusal::Verdict::unrecognised`] carries the raw string through.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KiteException {
    /// *"Preceded by a `403` header, this indicates the expiry or invalidation
    /// of an authenticated session."* Caused by a logout, a natural expiry, or
    /// a login to another Kite instance.
    Token = 0,
    /// *"Represents user account related errors"*.
    User = 1,
    /// *"Represents order related errors such placement failures, a corrupt
    /// fetch etc"*. This build places no order; the name is carried so an
    /// answer bearing it is named rather than collapsed.
    Order = 2,
    /// *"Represents missing required fields, bad values for parameters etc."*
    Input = 3,
    /// *"Represents insufficient funds, required for the order placement"*.
    Margin = 4,
    /// *"Represents insufficient holdings, available to place sell order for
    /// specified instrument"*.
    Holding = 5,
    /// *"Represents a network error where the API was unable to communicate
    /// with the OMS"*. Between the API and its own backend, not between this
    /// build and the API.
    Network = 6,
    /// *"Represents an internal system error where the API was unable to
    /// understand the response from the OMS"*. As [`Self::Network`]: the fault
    /// is behind the API and the request was not wrong.
    Data = 7,
    /// *"Represents an unclassified error. This should only happen rarely"*.
    General = 8,
    /// **Not on the exceptions page.** *"Represents permission denied
    /// exceptions for certain calls. Default code is 403."* — the vendor's own
    /// Python SDK. An entitlement failure, not a session failure.
    Permission = 9,
}

impl KiteException {
    /// Every name, in the discriminant's order.
    pub const ALL: [Self; EXCEPTION_COUNT] = [
        Self::Token,
        Self::User,
        Self::Order,
        Self::Input,
        Self::Margin,
        Self::Holding,
        Self::Network,
        Self::Data,
        Self::General,
        Self::Permission,
    ];

    /// The exact string the API sends in `error_type`.
    #[must_use]
    pub const fn wire(self) -> &'static str {
        match self {
            Self::Token => "TokenException",
            Self::User => "UserException",
            Self::Order => "OrderException",
            Self::Input => "InputException",
            Self::Margin => "MarginException",
            Self::Holding => "HoldingException",
            Self::Network => "NetworkException",
            Self::Data => "DataException",
            Self::General => "GeneralException",
            Self::Permission => "PermissionException",
        }
    }

    /// Whether the vendor's **exceptions page** publishes this name.
    ///
    /// False for [`Self::Permission`] alone. Kept as a fact rather than a
    /// comment because a page a reader can check and an SDK they must clone
    /// are not equally citable, and `CLAUDE.md` §3 rule 1 is about
    /// traceability rather than about being right.
    #[must_use]
    pub const fn on_documentation_page(self) -> bool {
        !matches!(self, Self::Permission)
    }

    /// The name, if this module knows it. `None` is the honest answer for a
    /// name it does not — see the type's header.
    ///
    /// # Constant time
    ///
    /// A `match` over string literals switches on the length first and then
    /// compares at most one arm, and the longest arm is 19 bytes
    /// (`PermissionException`). An `error_type` of any other length is refused
    /// without a byte being compared, so a hostile or corrupt body costs one
    /// integer test rather than a scan.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "TokenException" => Some(Self::Token),
            "UserException" => Some(Self::User),
            "OrderException" => Some(Self::Order),
            "InputException" => Some(Self::Input),
            "MarginException" => Some(Self::Margin),
            "HoldingException" => Some(Self::Holding),
            "NetworkException" => Some(Self::Network),
            "DataException" => Some(Self::Data),
            "GeneralException" => Some(Self::General),
            "PermissionException" => Some(Self::Permission),
            _ => None,
        }
    }

    /// What a caller may do about this name.
    ///
    /// # The groups, and the vendor's own words for them
    ///
    /// The exceptions page states the handling rule for each group, and this
    /// function is that rule with nothing added:
    ///
    /// * *"Handle `TokenException` specially: clear the stored session for the
    ///   user and re-initiate the login flow"* →
    ///   [`Disposition::SessionDead`]. This repository never mints
    ///   (`CLAUDE.md` §8), so "re-initiate" means the next pull reads a value
    ///   somebody else refreshed.
    /// * *"`NetworkException` and `DataException` indicate a problem between
    ///   the API and the backend OMS rather than a fault in your request; they
    ///   are candidates for a bounded retry"* →
    ///   [`Disposition::RetryBounded`].
    /// * *"`InputException`, `MarginException` and `HoldingException` are
    ///   deterministic rejections of the request as sent; retrying without
    ///   changing the request will fail again"* →
    ///   [`Disposition::RequestWrong`].
    ///
    /// [`Self::Permission`] is the one name the page does not rule on, and it
    /// gets [`Disposition::NotEntitled`] — a verdict of its own rather than a
    /// share of `SessionDead`, because the two differ in what the operator
    /// must go and do.
    ///
    /// [`Self::User`], [`Self::Order`] and [`Self::General`] are
    /// [`Disposition::ReasonGiven`]: the vendor named something, it is not one
    /// of the groups above, and inventing a retry policy for it would be
    /// inventing. `General` in particular is documented as *"unclassified"*,
    /// and re-asking an unclassified failure three times learns nothing.
    #[must_use]
    pub const fn disposition(self) -> Disposition {
        match self {
            Self::Token => Disposition::SessionDead,
            Self::Permission => Disposition::NotEntitled,
            Self::Network | Self::Data => Disposition::RetryBounded,
            Self::Input | Self::Margin | Self::Holding => Disposition::RequestWrong,
            Self::User | Self::Order | Self::General => Disposition::ReasonGiven,
        }
    }

    /// The operator-facing sentence, in the vendor's own terms.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Token => "the authenticated session expired or was invalidated",
            Self::User => "a user account error",
            Self::Order => "an order error",
            Self::Input => "a missing required field or a bad parameter value",
            Self::Margin => "insufficient funds",
            Self::Holding => "insufficient holdings",
            Self::Network => "the API could not reach its own order management system",
            Self::Data => "the API could not understand its own order management system",
            Self::General => "an unclassified error the vendor says should be rare",
            Self::Permission => "this API key is not entitled to what was asked",
        }
    }
}

impl fmt::Display for KiteException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.wire())
    }
}

/// [`KITE`]'s reader: one `error_type` string to what a caller may do.
///
/// A free function rather than a closure because [`ErrorNames::read`] is a
/// plain `fn` pointer and a `const` cannot hold a capturing closure. Kept
/// private: callers reach it through [`KITE`], which carries the field name
/// and the citation it has to travel with.
fn read(name: &str) -> Option<Disposition> {
    KiteException::from_wire(name).map(KiteException::disposition)
}

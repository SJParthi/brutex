//! Groww's `GA00x` error vocabulary — ONE VENDOR'S ROW in [`crate::refusal`].
//!
//! # What the absence of this file cost
//!
//! The descriptor carried `error_names: None` under a comment reading *"NO
//! BODY-LEVEL ERROR CONTRACT READ FOR THIS VENDOR. No page for it has been read
//! into docs/00-charter.md."* That was true when written and is not now: the
//! contract is published, the workspace already quotes a live `GA001` body in
//! two places, and this build was still classifying every Groww failure on its
//! HTTP status alone.
//!
//! Two codes are misread that way, in opposite directions:
//!
//! * `GA005` — *"User not authorised to perform this operation"*. Arrives as
//!   403, so the status says [`Disposition::SessionDead`] and an operator is
//!   told tomorrow's credential fixes an entitlement it cannot touch.
//! * `GA003` — *"Unable to serve request currently"*. Arrives as 400, so the
//!   status says [`Disposition::RequestWrong`] — a permanent verdict on a
//!   condition whose own sentence says **currently**. A retry that would have
//!   worked is never made.
//!
//! # The code is NESTED, and that is why the field alone was not enough
//!
//! ```text
//! {"status":"FAILURE","error":{"code":"GA001","message":"…","metadata":null}}
//! ```
//!
//! Kite and Dhan write their code at the root. Groww wraps it, so
//! [`crate::refusal::ErrorNames::envelope`] exists — a root-only lookup would
//! find nothing here and the whole contract would read as absent, which is the
//! worst shape available: a vendor whose errors ARE published, silently handled
//! as though they were not.
//!
//! # GA002 is missing, and that is the vendor's table, not a gap here
//!
//! The published list runs GA000, GA001, GA003 … GA007. There is no GA002. It
//! is not invented a meaning, and if the vendor ever sends one it reads as
//! unknown — which is the correct answer for a code nobody has published.

use crate::refusal::{Disposition, ErrorNames};

/// Groww's error contract, as `crate::vendor::HttpSpec::error_names` takes it.
pub const GROWW: ErrorNames = ErrorNames {
    field: "code",
    // NESTED UNDER `error`. See the module header — this is the reason the
    // envelope field exists at all.
    envelope: Some("error"),
    read,
    // NO MESSAGE READER. Not observed misfiling a code; see `kite::KITE`.
    message: None,
    source: "Groww Docs/15-BONUS-REST-introduction.md — 'Error Codes' table \
             (seven GA00x codes) and the FAILURE envelope above it, read \
             19 Aug 2026",
};

/// What one of Groww's codes means for a caller.
///
/// Returns `None` for a code the published table does not carry — deliberately
/// not a default, for the reason [`crate::dhan`] gives at length.
#[must_use]
#[allow(
    clippy::match_same_arms,
    reason = "each arm is one PUBLISHED code and carries the vendor's own \
              sentence for it. Merging the arms that happen to share a \
              disposition would collapse distinct facts into one line, hide \
              which codes are covered from anyone diffing this against the \
              vendor's table, and make a future divergence between two of them \
              an edit to a shared arm rather than a change to one row"
)]
pub fn read(code: &str) -> Option<Disposition> {
    match code {
        // "Internal error occurred." The vendor's side, and it may not recur.
        "GA000" => Some(Disposition::RetryBounded),
        // "Bad request." This build sent something wrong; the same bytes again
        // cannot help. This is the code `crate::http` already quotes live —
        // `GA001 Start time should be less than end time` — which is exactly a
        // request fault and confirms the mapping against a real body.
        "GA001" => Some(Disposition::RequestWrong),
        // "Unable to serve request currently." **CURRENTLY** is the whole
        // word: a transient refusal that the status axis reads as a permanent
        // one, because it arrives as 400.
        "GA003" => Some(Disposition::RetryBounded),
        // "Requested entity does not exist." Not a fault to retry and not an
        // entitlement — an instrument or an order this account cannot see. The
        // vendor's own sentence is what an operator needs.
        "GA004" => Some(Disposition::ReasonGiven),
        // "User not authorised to perform this operation."
        //
        // THIS WAS `NotEntitled` AND THAT IS THE DESTRUCTIVE READING.
        //
        // `NotEntitled` is the one disposition where `a_later_run_could_succeed`
        // is false: it returns immediately with "re-running later cannot fix
        // it. Nothing was retried."
        //
        // But look at what this vendor publishes — GA000 internal, GA001 bad
        // request, GA003 currently unavailable, GA004 entity missing, GA005 not
        // authorised, GA006 cannot process, GA007 duplicate order ref. **Not
        // one of the seven is a token expiry.** An expired or revoked access
        // token has nowhere else to surface: it has to arrive as GA005.
        //
        // So mapping it to `NotEntitled` turns every pull after a routine token
        // expiry into a permanent failure that declines to retry, when the
        // right instruction is the one the status axis already gave — "the
        // refreshed value is read from Parameter Store on the next pull".
        //
        // Dhan gets a split here only because Dhan PUBLISHES one: DH-901
        // against DH-902, 807 against 806. Groww publishes no split, so
        // resolving the ambiguity toward the irrecoverable answer is choosing
        // the expensive side of an unknown. `NotEntitled` is reserved for a
        // code the vendor itself distinguishes.
        "GA005" => Some(Disposition::SessionDead),
        // "Cannot process this request." A named catch-all: the vendor said
        // something and declined to say which thing, so the sentence travels
        // rather than a verdict being invented from it.
        "GA006" => Some(Disposition::ReasonGiven),
        // "Duplicate order reference id." This repository places no order, so
        // reaching this is a wiring fault rather than a market condition.
        // Mapped anyway — see `crate::dhan` on DH-906 for why.
        "GA007" => Some(Disposition::RequestWrong),

        // NOT PUBLISHED, THEREFORE UNKNOWN — including `GA002`, which the
        // vendor's own table skips.
        _ => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    /// **THE TWO THE STATUS AXIS GETS BACKWARDS.**
    ///
    /// `GA005` is a 403 that is not a dead session, and `GA003` is a 400 that
    /// is not a permanent refusal. Classified on status alone — which is what
    /// happened before this file — the first sends an operator to refresh a
    /// working token and the second throws away a retry that would have worked.
    #[test]
    fn a_dead_session_and_a_transient_are_not_what_their_statuses_say() {
        // GA005 IS A DEAD SESSION, NOT AN ENTITLEMENT. This vendor publishes
        // no token-expiry code, so GA005 is where one must arrive, and
        // `NotEntitled` would decline the retry it needs. See the arm's own
        // comment for why Dhan is split here and Groww is not.
        assert_eq!(read("GA005"), Some(Disposition::SessionDead));
        assert_eq!(read("GA003"), Some(Disposition::RetryBounded));
    }

    /// Every published code is recognised, and the count is asserted.
    #[test]
    fn every_code_in_the_published_table_is_recognised() {
        let published = [
            "GA000", "GA001", "GA003", "GA004", "GA005", "GA006", "GA007",
        ];
        assert_eq!(published.len(), 7, "the vendor's Error Codes table");
        for code in published {
            assert!(read(code).is_some(), "{code} is published and unrecognised");
        }
    }

    /// `GA002` is not in the vendor's table and is not given a meaning here.
    #[test]
    fn the_code_the_vendor_skips_is_unknown_rather_than_interpolated() {
        assert_eq!(
            read("GA002"),
            None,
            "the published list runs GA000, GA001, GA003..GA007 — a gap in a \
             vendor's numbering is not an invitation to fill it"
        );
        for unknown in ["GA008", "GA1", "", "ga001", "001"] {
            assert_eq!(read(unknown), None, "{unknown} must not be guessed at");
        }
    }

    /// The contract points at the nested field, which is the point of it.
    #[test]
    fn the_contract_descends_into_the_error_object() {
        assert_eq!(GROWW.field, "code");
        assert_eq!(
            GROWW.envelope,
            Some("error"),
            "a root-only lookup finds nothing in this vendor's body, and the \
             whole contract would read as absent"
        );
    }

    /// The nested code is actually extracted from a real body shape.
    ///
    /// The unit above asserts the declaration; this asserts it WORKS, against
    /// the envelope the vendor's own page prints.
    #[test]
    fn a_real_failure_body_yields_its_code_through_the_envelope() {
        let body = r#"{"status":"FAILURE","error":{"code":"GA005",
            "message":"User not authorised to perform this operation","metadata":null}}"#;

        let name = crate::refusal::named_error_of(body, GROWW.field, GROWW.envelope);
        assert_eq!(name.as_deref(), Some("GA005"));
        assert_eq!(
            name.as_deref().and_then(read),
            Some(Disposition::SessionDead)
        );
    }
}

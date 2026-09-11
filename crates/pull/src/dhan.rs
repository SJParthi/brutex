//! DhanHQ v2's two error vocabularies — ONE VENDOR'S ROW in [`crate::refusal`].
//!
//! # Why a vendor needs a row at all
//!
//! [`crate::refusal`]'s header makes the argument in full: the HTTP status is
//! one axis and the vendor's own name for what went wrong is another, and two
//! events that share a status can need opposite handling. This module is that
//! second axis for Dhan, exactly as [`crate::kite`] is for Kite.
//!
//! Until it existed, `error_names: None` meant `refusal::classify` fell through
//! to the status alone. The pair that costs is `DH-901` and `DH-902`, and the
//! vendor's own text is what separates them:
//!
//! * `DH-901` — *"Client ID or user generated access token is invalid or
//!   expired"*. Tomorrow's credential fixes it. [`Disposition::SessionDead`].
//! * `DH-902` — *"User has not subscribed to Data APIs or does not have access
//!   to Trading APIs"*. **Nothing this repository can do fixes it**, and it is
//!   the one disposition where `a_later_run_could_succeed` is false.
//!   [`Disposition::NotEntitled`].
//!
//! Both arrive as 401/403. Classified on status alone, both read as a dead
//! session — so an unsubscribed account was told its token would be refreshed
//! on the next pull, and every later run failed the same way with the same
//! wrong sentence. That is precisely the `TokenException`/`PermissionException`
//! situation `kite` was written for, present at a second vendor.
//!
//! # TWO tables, because the vendor publishes two
//!
//! `Dhan Docs/20-annexure.md` carries a "Trading API Error" table of ten
//! `DH-9xx` codes and a "Data API Error" table of twelve bare numbers. They
//! overlap in meaning and not in spelling: `DH-902` and `806` are both "Data
//! APIs not subscribed", `DH-901` and `807`/`808`/`809` are all a bad or
//! expired token. Reading only one table would leave the other's codes
//! unrecognised, so [`read`] takes both.
//!
//! # `None` is not a catch-all
//!
//! A code this module does not know returns `None`, and `classify` then falls
//! back to the status — which is what it did for every code before this file
//! existed. What `None` must never become is a default disposition: a name the
//! vendor invents tomorrow must read as *unknown*, not as *retryable*.

use crate::refusal::{Disposition, ErrorNames};

/// Dhan's error contract, as `crate::vendor::HttpSpec::error_names` takes it.
///
/// The field is `errorCode`, which `Dhan Docs/21-errors.md` names as part of
/// the envelope `{status, errorType, errorCode, errorMessage}`.
pub const DHAN: ErrorNames = ErrorNames {
    field: "errorCode",
    // AT THE ROOT. `Dhan Docs/21-errors.md` gives the envelope as a flat
    // `{status, errorType, errorCode, errorMessage}`.
    envelope: None,
    read,
    // THIS VENDOR MISFILES ITS OWN CODES, MEASURED. `DH-906` — the annexure's
    // "Order Error" — arrived under HTTP 400 carrying
    // `"errorMessage":"Invalid Token"`. See [`read_with_message`].
    message: Some(("errorMessage", read_with_message)),
    source: "Dhan Docs/20-annexure.md — 'Trading API Error' (ten DH-9xx codes) \
             and 'Data API Error' (twelve numeric codes), read 19 Aug 2026; \
             envelope shape from Dhan Docs/21-errors.md",
};

/// Whether the vendor's own SENTENCE names an authentication failure.
///
/// # Why the sentence is consulted at all
///
/// Measured: HTTP 400,
/// `{"errorType":"Order_Error","errorCode":"DH-906","errorMessage":"Invalid Token"}`.
/// [`read`] maps `DH-906` to [`Disposition::RequestWrong`] and is **faithful**
/// to `Dhan Docs/20-annexure.md` in doing so — *"Incorrect request for order —
/// cannot be processed"*. The vendor misused its own code, and `errorType`
/// corroborated the code rather than the message: it read `Order_Error`. The
/// message was the only field carrying the truth.
///
/// The cost was not a wrong label. `RequestWrong` means *"sending it again
/// unchanged cannot help"*, so `api::server::step` answered `Step::Answered`,
/// the credential was never re-read, and the pass loop re-asked the whole
/// instrument from chunk one against a token that was already dead.
///
/// # Why these five spellings and NOT `contains("token")`
///
/// Every one is a sentence this vendor PUBLISHES for an authentication failure
/// — `20-annexure.md`'s DH-901, 807, 808 and 809, and `21-errors.md`'s
/// `AUTHENTICATION_ERROR` — plus the one it was observed sending. The bare word
/// `token` is deliberately absent, for the reason
/// `api::autopilot::credential_fault_in_page` gives about the bare word
/// *credential*: a substring that appears in prose classifies prose.
///
/// The near misses are the point. `811` is *"Invalid Expiry Date"* and `813` is
/// *"Invalid `SecurityId`"* — both contain "invalid", neither matches any phrase
/// here, and both must keep their own dispositions.
fn message_names_a_dead_session(message: &str) -> bool {
    const AUTH_SPELLINGS: [&str; 5] = [
        "invalid token",
        "access token is invalid",
        "access token is expired",
        "invalid or expired access token",
        "authentication failed",
    ];
    let lower = message.to_ascii_lowercase();
    AUTH_SPELLINGS.iter().any(|phrase| lower.contains(phrase))
}

/// The disposition, from the code AND the sentence the vendor sent beside it.
///
/// **The code still decides**, exactly as [`read`] says. The sentence is
/// consulted only where the code's own answer is *not already* an
/// authentication verdict — so this can promote a misfiled code to
/// [`Disposition::SessionDead`] and can never demote a correct one.
///
/// That guard is what keeps this module's founding distinction intact: `DH-902`
/// and `806` are [`Disposition::NotEntitled`], the one disposition where no
/// later run can succeed, and turning either into a dead session would tell an
/// unsubscribed operator to wait for a credential refresh that fixes nothing —
/// the exact failure this file was written to close. Both return before the
/// sentence is read.
///
/// A vendor that stops misfiling its codes loses nothing: the sentence only
/// ever adds a verdict where the code supplied one that the sentence
/// contradicts.
#[must_use]
pub fn read_with_message(code: &str, message: &str) -> Option<Disposition> {
    let by_code = read(code);
    // AN AUTH VERDICT FROM THE CODE IS FINAL. Promote only; never demote.
    if matches!(
        by_code,
        Some(Disposition::SessionDead | Disposition::NotEntitled)
    ) {
        return by_code;
    }
    if message_names_a_dead_session(message) {
        return Some(Disposition::SessionDead);
    }
    by_code
}

/// What one of Dhan's codes means for a caller.
///
/// Returns `None` for a code neither published table carries — see the module
/// header on why that is deliberately not a default.
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
        // ---- Trading API errors, `Dhan Docs/20-annexure.md` ----
        //
        // "Client ID or user generated access token is invalid or expired."
        // Tomorrow's credential fixes it.
        "DH-901" => Some(Disposition::SessionDead),
        // "User has not subscribed to Data APIs or does not have access to
        // Trading APIs." Nothing here fixes it, and re-reading the credential
        // is the wrong instruction to give — this is the whole reason the
        // module exists.
        "DH-902" => Some(Disposition::NotEntitled),
        // "Errors related to User's Account — check if required segments are
        // activated." An entitlement in every practical sense: a segment this
        // account cannot trade is not a segment a retry reaches.
        "DH-903" => Some(Disposition::NotEntitled),
        // "Too many requests — try throttling API calls." The governor's own
        // signal, and the one code where backing off IS the fix.
        "DH-904" => Some(Disposition::Throttled),
        // "Missing required fields, bad values for parameters etc." This
        // build's request is wrong; sending it again unchanged cannot help.
        "DH-905" => Some(Disposition::RequestWrong),
        // "Incorrect request for order — cannot be processed." This repository
        // places no order, so reaching this is itself a wiring fault. Still
        // mapped rather than left unknown: an unmapped code reads as "the
        // vendor said something we have never seen", which would be false.
        "DH-906" => Some(Disposition::RequestWrong),
        // "Unable to fetch data due to incorrect parameters OR NO DATA
        // PRESENT." The two halves want opposite handling and the code cannot
        // tell them apart, so the vendor's sentence is surfaced rather than a
        // decision being invented from it.
        "DH-907" => Some(Disposition::ReasonGiven),
        // "Server was not able to process request — occurs rarely."
        "DH-908" => Some(Disposition::RetryBounded),
        // "API was unable to communicate with backend system."
        "DH-909" => Some(Disposition::RetryBounded),
        // "Error originating from other reasons." A named catch-all is still a
        // name: the vendor said it, and the sentence goes to the operator.
        "DH-910" => Some(Disposition::ReasonGiven),

        // ---- Data API errors, same annexure, a separate table ----
        //
        // "Internal Server Error."
        "800" => Some(Disposition::RetryBounded),
        // "Requested number of instruments exceeds limit." A bound this build
        // broke, not a state that changes.
        "804" => Some(Disposition::RequestWrong),
        // "Too many requests or connections — further requests may result in
        // blocking." The data-side spelling of DH-904, and the warning in the
        // second half is why it must reach the governor rather than a retry.
        "805" => Some(Disposition::Throttled),
        // "Data APIs not subscribed" — the data-side DH-902.
        "806" => Some(Disposition::NotEntitled),
        // "Access token is expired."
        "807" => Some(Disposition::SessionDead),
        // "Authentication Failed — Client ID or Access Token invalid."
        "808" => Some(Disposition::SessionDead),
        // "Access token is invalid."
        "809" => Some(Disposition::SessionDead),
        // "Client ID is invalid." NOT a dead session: a wrong client id is a
        // configuration fault that no token refresh corrects, and telling an
        // operator to wait for tomorrow's credential would send them away from
        // the file that is actually wrong.
        "810" => Some(Disposition::RequestWrong),
        // "Invalid Expiry Date."
        "811" => Some(Disposition::RequestWrong),
        // "Invalid Date Format."
        "812" => Some(Disposition::RequestWrong),
        // "Invalid SecurityId."
        "813" => Some(Disposition::RequestWrong),
        // "Invalid Request."
        "814" => Some(Disposition::RequestWrong),

        // A CODE NEITHER TABLE CARRIES. Not a default — see the module header.
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

    /// **THE PAIR THIS MODULE EXISTS FOR.**
    ///
    /// `DH-901` and `DH-902` both arrive as 401/403 and want opposite
    /// handling. Classified on status alone — which is what happened before
    /// this file — an account with no Data API subscription was told its token
    /// would be refreshed, and every run after it failed identically.
    #[test]
    fn a_dead_token_and_an_unsubscribed_account_are_told_apart() {
        assert_eq!(read("DH-901"), Some(Disposition::SessionDead));
        assert_eq!(read("DH-902"), Some(Disposition::NotEntitled));
        // AND THE DATA-SIDE SPELLINGS OF THE SAME TWO FACTS. The vendor
        // publishes two tables; recognising one would leave the other silent.
        assert_eq!(read("807"), Some(Disposition::SessionDead));
        assert_eq!(read("806"), Some(Disposition::NotEntitled));
    }

    /// Every published code is recognised, and the count is asserted.
    ///
    /// A count rather than a spot check: the failure this guards is a code
    /// quietly dropped when the tables are next edited, which no example test
    /// notices.
    #[test]
    fn every_code_in_both_published_tables_is_recognised() {
        let trading = [
            "DH-901", "DH-902", "DH-903", "DH-904", "DH-905", "DH-906", "DH-907", "DH-908",
            "DH-909", "DH-910",
        ];
        let data = [
            "800", "804", "805", "806", "807", "808", "809", "810", "811", "812", "813", "814",
        ];
        assert_eq!(trading.len(), 10, "the annexure's Trading API table");
        assert_eq!(data.len(), 12, "the annexure's Data API table");
        for code in trading.iter().chain(data.iter()) {
            assert!(read(code).is_some(), "{code} is published and unrecognised");
        }
    }

    /// Throttling reaches the governor from both spellings.
    #[test]
    fn both_rate_limit_codes_read_as_throttled() {
        assert_eq!(read("DH-904"), Some(Disposition::Throttled));
        assert_eq!(read("805"), Some(Disposition::Throttled));
    }

    /// An unknown code is UNKNOWN, never a default.
    ///
    /// `None` sends `classify` back to the status, which is what it did for
    /// every code before this module. What it must never do is invent a
    /// disposition for a name the vendor introduces tomorrow.
    #[test]
    fn a_code_neither_table_carries_is_unknown_rather_than_defaulted() {
        for unknown in ["DH-911", "DH-900", "815", "803", "", "901", "dh-901"] {
            assert_eq!(read(unknown), None, "{unknown} must not be guessed at");
        }
    }

    /// The contract names the field the vendor writes, and cites where from.
    #[test]
    fn the_contract_names_the_wire_field_and_its_source() {
        assert_eq!(DHAN.field, "errorCode");
        assert!(DHAN.source.contains("20-annexure.md"), "{}", DHAN.source);
    }

    /// **THE BODY THE OPERATOR ACTUALLY RECEIVED**, end to end.
    ///
    /// Quoted from a `pull.http` refusal event: HTTP 400 carrying `DH-906` —
    /// the annexure's *"Incorrect request for order"* — with
    /// `"errorMessage":"Invalid Token"`. `read` is faithful to the published
    /// table and answers `RequestWrong`, which told the run its own request was
    /// wrong and stopped it re-reading the credential.
    ///
    /// Note `errorType` reads `Order_Error`: the vendor's own type field
    /// corroborated the wrong code. The message was the only field carrying the
    /// truth, which is why the rule keys on it.
    #[test]
    fn the_body_that_carried_a_dead_token_under_an_order_code_is_read_as_a_dead_session() {
        let body =
            r#"{"errorType":"Order_Error","errorCode":"DH-906","errorMessage":"Invalid Token"}"#;
        assert_eq!(
            read("DH-906"),
            Some(Disposition::RequestWrong),
            "the code alone still reads as the annexure publishes it"
        );
        assert_eq!(
            crate::refusal::disposition_of(body, &DHAN),
            Some(Disposition::SessionDead),
            "the sentence names a dead token and the whole contract reads it"
        );
    }

    /// A REAL `DH-906` IS UNTOUCHED. The rule promotes on the sentence, so a
    /// code carrying its own published meaning keeps it.
    #[test]
    fn an_order_error_that_names_an_order_is_still_a_request_fault() {
        for sentence in [
            "Incorrect request for order - cannot be processed",
            "quantity exceeds freeze limit",
            "",
        ] {
            assert_eq!(
                read_with_message("DH-906", sentence),
                Some(Disposition::RequestWrong),
                "{sentence:?} names no authentication failure"
            );
        }
    }

    /// **THE GUARD THIS MODULE WAS WRITTEN FOR STILL HOLDS.**
    ///
    /// `DH-902` and `806` are `NotEntitled` — the one disposition where no later
    /// run can succeed. Promoting either to a dead session would tell an
    /// unsubscribed operator to wait for a credential refresh that fixes
    /// nothing, which is the exact failure this file exists to close. The
    /// sentence is never consulted for a code that already answers with an
    /// authentication verdict, so even an auth-sounding message cannot demote
    /// them.
    #[test]
    fn an_unsubscribed_account_is_never_promoted_to_a_dead_session() {
        for code in ["DH-902", "806"] {
            assert_eq!(
                read_with_message(code, "Invalid Token"),
                Some(Disposition::NotEntitled),
                "{code} is an entitlement and no sentence may change that"
            );
        }
        // And a code that IS a dead session stays one, message or not.
        for code in ["DH-901", "807", "808", "809"] {
            assert_eq!(read_with_message(code, ""), Some(Disposition::SessionDead));
        }
    }

    /// **THE NEAR MISSES**, which are why the rule is not `contains("token")`.
    ///
    /// `811` is "Invalid Expiry Date" and `813` is "Invalid `SecurityId`". Both
    /// contain "invalid", both are published `RequestWrong`, and a looser
    /// substring rule would have turned either into a credential refresh that
    /// fixes nothing.
    #[test]
    fn a_sentence_that_merely_contains_invalid_is_not_a_dead_session() {
        for (code, sentence) in [
            ("811", "Invalid Expiry Date"),
            ("813", "Invalid SecurityId"),
            ("814", "Invalid Request"),
            ("810", "Client ID is invalid"),
        ] {
            assert_eq!(
                read_with_message(code, sentence),
                read(code),
                "{code} {sentence:?} keeps the disposition its code publishes"
            );
        }
        assert!(!message_names_a_dead_session("Invalid Expiry Date"));
        assert!(!message_names_a_dead_session("token bucket exhausted"));
        assert!(message_names_a_dead_session("Invalid Token"));
        assert!(message_names_a_dead_session("Authentication Failed"));
    }

    /// A vendor declaring no message reader keeps the code-only path exactly.
    #[test]
    fn a_contract_without_a_message_reader_is_unchanged() {
        assert!(crate::kite::KITE.message.is_none());
        assert!(crate::groww::GROWW.message.is_none());
        let body = r#"{"error_type":"TokenException","message":"Invalid Token"}"#;
        assert_eq!(
            crate::refusal::disposition_of(body, &crate::kite::KITE),
            Some(Disposition::SessionDead),
            "read through its own code, as before"
        );
    }
}

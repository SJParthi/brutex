//! What a refused answer MEANS, on both of the two axes a vendor answers on.
//!
//! # The axis this crate did not have
//!
//! Every retry decision in this workspace is taken from the **HTTP status**.
//! `crates/api/src/server.rs::with_retry` reads `FetchError::VendorRefused`'s
//! `status: u16` and switches on it: 401/403 is a dead credential, 429 is the
//! governor's business, 5xx is a bounded quadratic backoff, anything else is a
//! reason about the request. That is the whole of it, plus one string search
//! for `Invalid_Authentication` — a marker one vendor writes into its body,
//! matched by `str::contains` against this build's own rendering of the error.
//!
//! A vendor's own error name is not on the status axis, and for at least one
//! vendor here the whole error contract lives there. Kite's exceptions page
//! says so in as many words:
//!
//! > error responses come with the name of the exception generated internally
//! > by the API server. You can define corresponding exceptions in your
//! > language or library, and raise them by doing a switch on the returned
//! > exception name.
//!
//! **Nothing in this workspace read that field before this module.** A grep
//! for `error_type` across `crates/` returned one hit and it was
//! `"ExpiredTokenException"` in `src/ssm.rs`, which is Amazon's word.
//!
//! # What that cost, concretely, and it is one pair
//!
//! Kite documents `TokenException` as *"Preceded by a `403` header"*. It
//! documents `PermissionException` — in its own Python SDK rather than on the
//! exceptions page — as *"Default code is 403"*, and that is what an API key
//! with no historical-data subscription is answered with.
//!
//! **Both are 403, so on the status axis they are the same event.** An
//! unentitled key is therefore reported to the operator as:
//!
//! > the access token is no longer valid mid-run (expiry, a logout, or a login
//! > to another session of the same vendor) … the refreshed value is read from
//! > Parameter Store on the next pull
//!
//! Every clause of which is false for that key. There is no expiry, no logout
//! and no other session; the next pull reads a credential that is alive and is
//! refused identically, forever, until somebody buys the subscription. A
//! failure wearing another failure's clothes is the shape `CLAUDE.md` §4 bans,
//! and this pair produced it.
//!
//! # Why this module is generic and `kite` is a row in it
//!
//! The names are one vendor's; the *shape* is not. A vendor either publishes a
//! body field carrying its own error name or it does not, and [`ErrorNames`]
//! is that fact — one field naming the JSON key, one function reading it.
//! [`crate::vendor::HttpSpec::error_names`] carries it per feed, so a vendor
//! whose error contract is read later is **a row and a `from_wire`**, not an
//! edit to any decision here.
//!
//! `None` on that field is a **recorded absence**, in the sense
//! `crate::vendor::FnoAccess::None` already means it: no error-name contract
//! this build has read, so the status decides alone — which is exactly what
//! every feed did before this module and is therefore no regression for any of
//! them.
//!
//! # Why both axes are kept, and neither is thrown away
//!
//! A recognised name binds, because the vendor instructs switching on it. But
//! a status and a name can disagree — a 403 carrying `NetworkException`, an
//! error envelope arriving under a 200 — and the disagreement is a fact about
//! the answer rather than noise to be resolved silently. So [`Verdict`]
//! carries what the losing axis would have said, in [`Verdict::contested`],
//! the way [`crate::vendor::FloorRow`] carries the floor claim that did not
//! bind. Nothing here averages and nothing here is dropped.
//!
//! # Why an unknown name is not folded into the vendor's catch-all
//!
//! Kite's nine published names are *"the complete set documented on this page,
//! not the complete set of `error_type` strings the API can return"* —
//! `PermissionException` is the standing proof, live in the vendor's own SDK
//! and absent from the page. So an unrecognised name survives verbatim in
//! [`Verdict::unrecognised`] and the **status** decides. Folding it into a
//! `GeneralException` would manufacture a classification no source made.
//!
//! # O(1)
//!
//! `CLAUDE.md` §3 rule 4. Every function here is constant-time in the size of
//! anything that grows:
//!
//! | Function | Cost | Why it does not grow |
//! |---|---|---|
//! | [`status_disposition`] | at most ten integer compares | The status set is closed by the vendor's own published table and does not grow with traffic. |
//! | [`ErrorNames::read`] | one indirect call, then one length test and at most one 16-byte compare | Each vendor's reader is a `match` over string literals, which switches on `len` first. A megabyte of hostile `error_type` fails every length bucket without a byte being compared. |
//! | [`classify`] | the sum of the two above | No allocation, no search, no map, no table walk. |
//!
//! **[`named_error_of`] is the one linear function and it is named rather than
//! hidden.** A field's offset inside a JSON body is not known before the bytes
//! are read. It is not on a per-bar path: it runs **once per refused
//! request**, against a body the transport has already bounded, and a pull
//! refusing often enough for this to matter has a larger problem than parsing.
//! `docs/06-limits.md` is where that is registered rather than claimed away.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use core::fmt;

/// What a caller may do next.
///
/// # Ordered by how many further requests it permits
///
/// The derived order is the meaning: [`Self::NotEntitled`] permits the fewest
/// further requests and [`Self::RetryBounded`] the most. A page may sort by
/// it. It is deliberately **not** used to resolve a disagreement between the
/// two axes — see [`classify`], which resolves by the vendor's own instruction
/// and keeps the loser rather than taking the stricter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Disposition {
    /// The credential is **alive and not entitled**. No further request
    /// succeeds until the operator's subscription changes. Distinct from
    /// [`Self::SessionDead`] in what has to happen next, which is the whole
    /// reason this module exists.
    NotEntitled,
    /// The session is dead. Nothing further this run; the next pull reads a
    /// credential somebody else refreshed. `CLAUDE.md` §8 forbids minting one
    /// here.
    SessionDead,
    /// The request as sent is wrong, and will be wrong when sent again.
    RequestWrong,
    /// The vendor named a reason that is not one of the groups above. Not
    /// retried, because no source says retrying it helps.
    ReasonGiven,
    /// The vendor named the arrival **rate**. Ask again, slower — and the
    /// governor takes its multiplicative decrease first.
    Throttled,
    /// A fault behind the vendor's own API. Ask again on a bounded backoff.
    RetryBounded,
}

impl Disposition {
    /// Every disposition, in the derived order.
    pub const ALL: [Self; 6] = [
        Self::NotEntitled,
        Self::SessionDead,
        Self::RequestWrong,
        Self::ReasonGiven,
        Self::Throttled,
        Self::RetryBounded,
    ];

    /// Whether this verdict permits another request for the same window.
    #[must_use]
    pub const fn permits_more_requests(self) -> bool {
        matches!(self, Self::Throttled | Self::RetryBounded)
    }

    /// Whether re-running the whole pull **later** could succeed.
    ///
    /// The distinction an operator acts on, and the one the status axis alone
    /// cannot draw: a dead session is fixed by tomorrow's credential, and an
    /// unentitled key is not fixed by anything this repository can do.
    #[must_use]
    pub const fn a_later_run_could_succeed(self) -> bool {
        !matches!(self, Self::NotEntitled)
    }

    /// Whether the governor should take its multiplicative decrease.
    ///
    /// True for [`Self::Throttled`] alone. A 5xx names no budget, so a
    /// backoff for one must not narrow the rate the vendor never complained
    /// about — the distinction `crates/api/src/server.rs` already draws and
    /// this makes readable from one place.
    #[must_use]
    pub const fn narrows_the_rate(self) -> bool {
        matches!(self, Self::Throttled)
    }

    /// The wire word. A KEY for a log line, never prose.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::NotEntitled => "not_entitled",
            Self::SessionDead => "session_dead",
            Self::RequestWrong => "request_wrong",
            Self::ReasonGiven => "reason_given",
            Self::Throttled => "throttled",
            Self::RetryBounded => "retry_bounded",
        }
    }

    /// What an operator is told, including what they must go and do.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotEntitled => {
                "this API key is not entitled to this call — the credential is alive, \
                 a later run fails identically, and only a subscription change fixes it"
            }
            Self::SessionDead => {
                "the session is dead — nothing further this run; the next pull reads a \
                 refreshed credential, which this repository never mints itself (§8)"
            }
            Self::RequestWrong => {
                "the request as sent was rejected and will be rejected again unchanged"
            }
            Self::ReasonGiven => {
                "the vendor named a reason that is not retryable by any published rule"
            }
            Self::Throttled => "the arrival rate was refused — ask again, slower",
            Self::RetryBounded => {
                "a fault behind the vendor's own API — ask again on a bounded backoff"
            }
        }
    }
}

impl fmt::Display for Disposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One vendor's body-level error-name contract.
///
/// # Why a function pointer and not a table
///
/// A `&[(&str, Disposition)]` would be walked, and a walk grows with the
/// vendor's exception count — `CLAUDE.md` §3 rule 4 forbids exactly that on a
/// path a caller reaches. A `match` over string literals does not: rustc
/// switches on the length first and compares at most one arm, so the cost is
/// bounded by the longest name the vendor publishes rather than by how many it
/// publishes. Carrying the reader as `fn` keeps that property AND keeps this
/// module free of any vendor's vocabulary.
///
/// (A vendor's optional message key and its two-argument reader travel together
/// as [`MessageReader`]: neither half means anything alone — a key with no
/// reader is a string nobody consults, and a reader with no key has nothing to
/// read — so a contract declares both or declares neither.)
///
/// # Adding a vendor is this struct and a `from_wire`
///
/// One `const` beside the vendor's own module, one `Some(&…)` in its
/// [`crate::vendor::HttpSpec`]. Nothing in [`classify`], in
/// [`status_disposition`] or in the retry ladder changes, which is what makes
/// the mechanism incremental rather than a growing `match` on the feed.
#[derive(Clone, Copy)]
pub struct ErrorNames {
    /// The JSON key the vendor writes its own error name into. Kite:
    /// `error_type`.
    pub field: &'static str,
    /// The object [`Self::field`] sits INSIDE, when the vendor nests it.
    ///
    /// # Why this is a field and not a search
    ///
    /// Vendors disagree about depth. Kite and Dhan write the code at the root
    /// (`error_type`, `errorCode`); Groww wraps it —
    /// `{"status":"FAILURE","error":{"code":"GA001", …}}` — so a root-only
    /// lookup finds nothing and the whole contract reads as absent. That is the
    /// worst shape: a vendor whose errors ARE published, silently classified on
    /// status alone as though they were not.
    ///
    /// Named rather than searched for, because a reader that hunted for any key
    /// called `code` at any depth would eventually find one belonging to
    /// something else. `None` for a vendor that writes it at the root.
    pub envelope: Option<&'static str>,
    /// That name → what a caller may do, or `None` for a name this build does
    /// not know. **`None` must not be a catch-all**: see the module header.
    pub read: fn(&str) -> Option<Disposition>,
    /// The key carrying the vendor's own SENTENCE, and a reader that may use it
    /// where the code alone is wrong.
    ///
    /// # Why a vendor's own code is not always the last word
    ///
    /// Measured: Dhan answered HTTP 400 with
    /// `{"errorType":"Order_Error","errorCode":"DH-906","errorMessage":"Invalid Token"}`.
    /// [`Self::read`] maps `DH-906` to [`Disposition::RequestWrong`] and is
    /// **faithful** to the vendor's published annexure in doing so — *"Incorrect
    /// request for order — cannot be processed"*. The vendor misfiled its own
    /// code, and `errorType` corroborated the code rather than the message: it
    /// read `Order_Error`. **The message was the only field carrying the
    /// truth.**
    ///
    /// The cost of believing the code was not a wrong label. `RequestWrong`
    /// means *"sending it again unchanged cannot help"*, so the credential was
    /// never re-read and the pass loop re-asked the whole instrument from chunk
    /// one against a token that was already dead.
    ///
    /// # `None` is a recorded absence, exactly as [`Self::envelope`]'s is
    ///
    /// `None` for a vendor whose codes have not been OBSERVED to disagree with
    /// their own messages. Not a hole — a vendor that has never been seen
    /// misfiling a code gets the byte-identical path it had before this field
    /// existed, and adding a reader on suspicion would be the invention §3 rule
    /// 1 forbids.
    pub message: Option<MessageReader>,
    /// Where the contract was read, in words an operator can go and check.
    pub source: &'static str,
}

impl fmt::Debug for ErrorNames {
    /// Hand-written because a `fn` pointer's derived `Debug` prints an
    /// address, which differs between runs and would make any snapshot of a
    /// descriptor unstable.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrorNames")
            .field("field", &self.field)
            .field("envelope", &self.envelope)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ErrorNames {
    /// Compares the two fields that are data. Function pointers are compared
    /// by address, which `clippy::fn_address_comparisons` rightly distrusts,
    /// and two contracts with the same field and the same citation are the
    /// same contract for every purpose this type is used for.
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field && self.envelope == other.envelope && self.source == other.source
    }
}

impl Eq for ErrorNames {}

impl core::hash::Hash for ErrorNames {
    /// Hashes exactly what [`PartialEq`] compares, which is the contract
    /// `Hash` and `Eq` have to keep between them. The `fn` pointer is left out
    /// of both: its address is not stable across builds, and two contracts
    /// with the same field and the same citation are the same contract.
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.field.hash(state);
        self.source.hash(state);
    }
}

/// A vendor's message key, and the reader that weighs it against the code.
///
/// The pair travels together because neither half means anything alone: a key
/// with no reader is a string nobody consults, and a reader with no key has
/// nothing to read. Named as one type so [`ErrorNames::message`] can say
/// `Option<MessageReader>` — a contract declares both or declares neither, and
/// there is no third state to represent.
pub type MessageReader = (&'static str, fn(&str, &str) -> Option<Disposition>);

/// Which of the two axes decided a [`Verdict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    /// The HTTP status decided — because no name arrived, because the feed
    /// publishes no name contract this build has read, or because the name
    /// that did arrive is one its reader does not know.
    Status,
    /// The vendor's own error name decided, which is what a vendor that
    /// publishes one instructs.
    ErrorName,
}

impl Axis {
    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::ErrorName => "error_name",
        }
    }
}

impl fmt::Display for Axis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// What the **status alone** says, for the codes Kite publishes and the one it
/// does not.
///
/// # Why an unknown status is `ReasonGiven` and not `RetryBounded`
///
/// The direction matters and the two ways to be wrong are not symmetric.
/// Treating an unstated status as retryable spends the whole ladder — 13.75 s
/// of backoff on the existing one — proving that an unstated answer stays
/// unstated. Treating it as answered costs one window, which the run records
/// and reports. The cheap error is the one taken.
///
/// # The rows
///
/// | Status | Verdict | The vendor's own description |
/// |---|---|---|
/// | `400` | `RequestWrong` | *"Missing or bad request parameters or values"* |
/// | `401` | `SessionDead` | **Not in the vendor's table.** `with_retry` already treats it as 403, and an unauthenticated answer is a backoff under no reading. |
/// | `403` | `SessionDead` | *"Session expired or invalidate. Must relogin"* — the row `PermissionException` is invisible on |
/// | `404` | `RequestWrong` | *"Request resource was not found"* |
/// | `405` | `RequestWrong` | *"Request method (GET, POST etc.) is not allowed on the requested endpoint"* |
/// | `410` | `RequestWrong` | *"The requested resource is gone permanently"* — permanently, so not a backoff |
/// | `429` | `Throttled` | *"Too many requests to the API (rate limiting)"* |
/// | `500` | `RetryBounded` | *"Something unexpected went wrong"* |
/// | `502` | `RetryBounded` | *"The backend OMS is down and the API is unable to communicate with it"* |
/// | `503` | `RetryBounded` | *"Service unavailable; the API is down"* |
/// | `504` | `RetryBounded` | *"Gateway timeout; the API is unreachable"* |
/// | anything else, `200` included | `ReasonGiven` | Unstated. An error envelope under a 200 is the answer contradicting itself, which is not a retry either. |
#[must_use]
pub const fn status_disposition(status: u16) -> Disposition {
    match status {
        429 => Disposition::Throttled,
        401 | 403 => Disposition::SessionDead,
        400 | 404 | 405 | 410 => Disposition::RequestWrong,
        500 | 502 | 503 | 504 => Disposition::RetryBounded,
        _ => Disposition::ReasonGiven,
    }
}

/// One classified refusal: what to do, who said so, and what the other axis
/// would have said.
///
/// Borrows the unrecognised name rather than owning it, so classifying costs
/// no allocation on a path that is already failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict<'a> {
    /// What a caller may do next.
    pub disposition: Disposition,
    /// Which axis decided it.
    pub decided_by: Axis,
    /// What the **losing** axis would have said, when the two disagree.
    /// `None` when they agree, or when only one axis spoke.
    ///
    /// Never resolved away — see the module header.
    pub contested: Option<Disposition>,
    /// The vendor's error name, verbatim, when one arrived that its own reader
    /// does not know.
    ///
    /// A vendor's published exception list is explicitly not closed, so this
    /// is the field that keeps a new server-side name readable instead of
    /// collapsing it into the vendor's catch-all.
    pub unrecognised: Option<&'a str>,
}

impl Verdict<'_> {
    /// Whether the two axes disagreed.
    #[must_use]
    pub const fn axes_disagreed(&self) -> bool {
        self.contested.is_some()
    }
}

/// Classify one refused answer from its status and the vendor's own error
/// name.
///
/// `names` is the feed's contract — [`crate::vendor::HttpSpec::error_names`]
/// — and `None` means this build has read no error-name contract for that
/// vendor, which is a recorded absence rather than a hole. `name` is whatever
/// arrived in the field that contract points at, which
/// [`named_error_of`] extracts.
///
/// # The rule, and it is the vendor's
///
/// A **recognised** name binds, because a vendor that publishes one instructs
/// exactly that. The status decides when no contract is declared, when no name
/// arrived, or when the name is one the vendor's own reader does not know.
///
/// Nothing is thrown away: when the losing axis would have said something
/// different it lands in [`Verdict::contested`].
///
/// # The pair this function exists for
///
/// ```
/// use pull::kite::KITE;
/// use pull::refusal::{classify, Disposition};
///
/// // Both are HTTP 403. On the status axis alone they are one event.
/// let expired = classify(Some(&KITE), 403, Some("TokenException"));
/// let unentitled = classify(Some(&KITE), 403, Some("PermissionException"));
///
/// assert_eq!(expired.disposition, Disposition::SessionDead);
/// assert_eq!(unentitled.disposition, Disposition::NotEntitled);
///
/// // And only one of them is fixed by re-running tomorrow.
/// assert!(expired.disposition.a_later_run_could_succeed());
/// assert!(!unentitled.disposition.a_later_run_could_succeed());
///
/// // The status axis said `SessionDead` for both; for the second that is kept
/// // as the claim that did not bind rather than dropped.
/// assert_eq!(unentitled.contested, Some(Disposition::SessionDead));
/// assert_eq!(expired.contested, None);
/// ```
///
/// # A feed with no declared contract is exactly what it was before
///
/// ```
/// use pull::refusal::{classify, Axis, Disposition};
///
/// // `None` — no error-name contract read for this feed. The status decides,
/// // which is what every feed did before this module existed.
/// let v = classify(None, 500, Some("NetworkException"));
/// assert_eq!(v.decided_by, Axis::Status);
/// assert_eq!(v.disposition, Disposition::RetryBounded);
/// assert_eq!(v.unrecognised, None);
/// ```
///
/// # An unknown name keeps its spelling
///
/// ```
/// use pull::kite::KITE;
/// use pull::refusal::{classify, Axis, Disposition};
///
/// let v = classify(Some(&KITE), 500, Some("QuantumFluxException"));
/// assert_eq!(v.decided_by, Axis::Status);
/// assert_eq!(v.disposition, Disposition::RetryBounded);
/// assert_eq!(v.unrecognised, Some("QuantumFluxException"));
/// ```
#[must_use]
pub fn classify<'a>(names: Option<&ErrorNames>, status: u16, name: Option<&'a str>) -> Verdict<'a> {
    let from_status = status_disposition(status);
    // Only one axis can speak, so nothing is contested and nothing is lost.
    let status_only = Verdict {
        disposition: from_status,
        decided_by: Axis::Status,
        contested: None,
        unrecognised: None,
    };
    let (Some(contract), Some(raw)) = (names, name) else {
        return status_only;
    };
    let Some(from_name) = (contract.read)(raw) else {
        // A name arrived and this vendor's reader does not know it. The status
        // decides and the name survives verbatim — NOT folded into a catch-all.
        return Verdict {
            unrecognised: Some(raw),
            ..status_only
        };
    };
    Verdict {
        disposition: from_name,
        decided_by: Axis::ErrorName,
        // Kept only when it differs. Keeping it when the two agree would
        // report a disagreement that did not happen.
        contested: (from_name != from_status).then_some(from_status),
        unrecognised: None,
    }
}

/// The vendor's error name out of an error body, without decoding the rest.
///
/// # This is the one linear function in the module, and it says so
///
/// A field's offset inside a JSON body is not known before the bytes are read,
/// so this is O(body). It runs **once per refused request**. See the module
/// header's O(1) table, which names it rather than leaving a reader to find
/// it.
///
/// # Why it answers `None` so readily
///
/// A body that is not JSON, one that is not an object, an object without the
/// field, and a field that is not a string are all *"no name arrived"* —
/// which [`classify`] already handles by falling to the status. Refusing here
/// would turn a vendor's malformed error into a second error of this build's
/// own, on a path that is already failing.
///
/// ```
/// use pull::kite::KITE;
/// use pull::refusal::named_error_of;
///
/// let body = r#"{"status":"error","message":"Error message",
///                "error_type":"GeneralException"}"#;
/// assert_eq!(named_error_of(body, KITE.field, KITE.envelope), Some("GeneralException".to_owned()));
///
/// // Not JSON, not an object, no such field, and a non-string field.
/// assert_eq!(named_error_of("<html>502</html>", "error_type", None), None);
/// assert_eq!(named_error_of("[1,2,3]", "error_type", None), None);
/// assert_eq!(named_error_of(r#"{"status":"error"}"#, "error_type", None), None);
/// assert_eq!(named_error_of(r#"{"error_type":7}"#, "error_type", None), None);
/// ```
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn named_error_of(body: &str, field: &str, envelope: Option<&str>) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    // NAMED, NOT SEARCHED. `None` reads the root, which is where Kite and Dhan
    // write it; `Some("error")` descends exactly one level, which is where
    // Groww does. A hunt for any key of that name at any depth would sooner or
    // later find one belonging to something else.
    let holder = match envelope {
        Some(name) => value.get(name)?,
        None => &value,
    };
    Some(holder.get(field)?.as_str()?.to_owned())
}

/// One refused body → what a caller may do, reading the whole contract.
///
/// # Why this exists beside [`named_error_of`]
///
/// [`ErrorNames::message`] means a contract can need TWO keys out of one body,
/// and the obvious spelling — calling [`named_error_of`] once per key — parses
/// the body twice. That function is this module's one acknowledged linear cost,
/// so doubling it on every refusal would make the cost table above wrong for a
/// reason no reader could see from the call site. **One `from_str`, two `get`s.**
///
/// A contract declaring `message: None` takes the code-only path and is
/// byte-identical to what it had before that field existed.
///
/// # Errors
///
/// Returns `None` — never a default disposition — when the body is not JSON,
/// when the envelope or the code key is absent, or when the vendor's reader does
/// not recognise the code. The caller falls back to the status, which is what it
/// did for every vendor before any contract existed.
///
/// # Cost
///
/// One JSON parse, bounded by the body the caller already holds, then a constant
/// number of key lookups. Same order as [`named_error_of`], not twice it.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn disposition_of(body: &str, contract: &ErrorNames) -> Option<Disposition> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let holder = match contract.envelope {
        Some(name) => value.get(name)?,
        None => &value,
    };
    let code = holder.get(contract.field)?.as_str()?;
    match contract.message {
        // THE SENTENCE, WHERE THE VENDOR PUBLISHES ONE AND ITS CODES HAVE BEEN
        // SEEN TO DISAGREE WITH IT. An absent or non-string message reads as
        // empty rather than as a refusal: a missing sentence must not lose the
        // code's own answer, which is right far more often than it is wrong.
        Some((key, read_both)) => {
            let sentence = holder.get(key).and_then(serde_json::Value::as_str);
            read_both(code, sentence.unwrap_or_default())
        }
        None => (contract.read)(code),
    }
}

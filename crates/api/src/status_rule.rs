//! ONE STATUS RULE, THROUGH ONE TYPED CLASSIFIER (BASE-STATUS-CODES, D-7550).
//!
//! # Why this exists
//!
//! The assurance attack of 29 Oct 2026 sent 3,286 requests and got eight
//! `5xx`, and every one of the eight was a client's input
//! (`state/assure-20261029/api-attack/90-totals.txt`, failure n=31):
//!
//! * `/sources/day.json` and `/sources/bars.json` with `symbol=nifty` or
//!   `symbol=reliance` answered `503` *"path segment symbol holds byte 0x6e in
//!   the wrong case"*, while `/bars.json` and `/gaps.json` answered the same
//!   question `400`. `sources::Refusal::store` turned every store error into a
//!   `503` and told a retired key from the rest by reading its words.
//! * One unknown 64-hex research identity was a `503` on
//!   `/boolean-candidates.json`, `/boolean-statistics.json` and
//!   `/index-stop.json`; a `200` with `"status":"missing"` on
//!   `/expression-search.json` and `/sweep-evidence.json`; a `200` with no
//!   rows, no `status` and a refusal sentence on `/frontier.json`; and a `400`
//!   on `/candidate-trades.json`, which had not been sent its required
//!   `attempt`, and a `200` `"missing"` once it was: one question, four answers.
//!
//! # The rule
//!
//! Every refusal a route answers is one [`Class`], and its status is
//! [`Class::status`] — chosen here, once, and nowhere else:
//!
//! | class | status | decided by |
//! |---|---|---|
//! | [`Class::Malformed`] | `400` | the parameter's own rule ([`Malformed`]), before any store file is opened |
//! | [`Class::NotFound`] | `404` | one probe of the path the key names, whose `io::ErrorKind` is `NotFound` ([`probe`]) |
//! | [`Class::Retired`] | `410` | the re-key's ledger (D-3331) |
//! | [`Class::OutOfRange`] | `416` | the page-window step, for a page past the found result's extent (D-7551) |
//! | [`Class::OverBound`] | `422` | the route's stated bound |
//! | [`Class::Saturated`] | `429` | the blocking pool is full |
//! | [`Class::Unreadable`] | `503` | what exists could not be read: any other `io::ErrorKind` of the probe, or the reader's refusal of what the probe found |
//!
//! The being-written `503` with `Retry-After` (R3-08) is an `Unreadable`
//! answer whose body says so; `count_route` adds the header.
//!
//! **Typed, never words.** A class comes from an `io::ErrorKind`, the store's
//! own [`store::path::PathError`], or which step refused — never from what a
//! message happens to say. **The same input gets the same status on every
//! route that takes it**, because every route asks the same function.
//!
//! # Not found is `404`, decided by ONE probe
//!
//! A well-formed identity that names nothing the store holds is `404` with one
//! body shape ([`Fault::identity_body`]) on every identity route. It is decided
//! by one `symlink_metadata` of the path the identity names — never a listing
//! of a directory — and, only when that answers `NotFound`, one `metadata` of
//! the root, which is what tells "this store holds no such result" (`404`)
//! from "the configured store root is not there" (`503`). Why `404` and not
//! the others, D-7550 says: `200` is an answer, `503` blames the server for the
//! client's question, `400` calls a well-formed identity malformed, and `410`
//! is kept for a key that once was and was retired.
//!
//! # Cost
//!
//! A parameter's rule is one pass over its bytes, at most
//! [`store::path::MAX_SEGMENT_LEN`] for a symbol and [`IDENTITY_HEX`] for an
//! identity. A probe is one `stat`, two on a `NotFound`. Neither depends on how
//! much the store holds (`docs/06-limits.md` L-STATUS-01).

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use axum::http::StatusCode;
use serde_json::{Value, json};
use store::path::{PathError, SegmentCase};

/// The decision this rule is recorded under.
pub const DECISION: &str = "D-7550";

/// The length of an identity: 32 bytes as lowercase hex.
pub const IDENTITY_HEX: usize = 64;

/// What a refusal is. Every status a refusal is answered with is
/// [`Class::status`] of one of these, so a status is chosen once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// The question is malformed: a parameter breaks its rule. Decided before
    /// any store file is opened.
    Malformed,
    /// The question is well formed and names nothing the store holds: one
    /// probe answered `NotFound` under a root that is there.
    NotFound,
    /// A key the re-key retired (D-3331): its files are kept, no route serves
    /// them.
    Retired,
    /// Well formed, and answering it would cost more than the route's bound.
    OverBound,
    /// A page past the extent the found result holds: the result is there,
    /// the page is not, as `/frontier.json` has answered it (D-7551).
    OutOfRange,
    /// The blocking pool is full; nothing was queued.
    Saturated,
    /// What exists could not be read: permission, a file where a folder
    /// should be, an I/O error, a torn or undecodable file, a writer holding
    /// it, or a store root that is not there.
    Unreadable,
}

impl Class {
    /// Every class, in the order of the table above.
    pub const ALL: [Self; 7] = [
        Self::Malformed,
        Self::NotFound,
        Self::Retired,
        Self::OutOfRange,
        Self::OverBound,
        Self::Saturated,
        Self::Unreadable,
    ];

    /// The one status this class is answered with.
    #[must_use]
    pub const fn status(self) -> StatusCode {
        match self {
            Self::Malformed => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Retired => StatusCode::GONE,
            Self::OutOfRange => StatusCode::RANGE_NOT_SATISFIABLE,
            Self::OverBound => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Saturated => StatusCode::TOO_MANY_REQUESTS,
            Self::Unreadable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// The word a body names this class with.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::NotFound => "not_found",
            Self::Retired => "retired",
            Self::OutOfRange => "out_of_range",
            Self::OverBound => "over_bound",
            Self::Saturated => "saturated",
            Self::Unreadable => "unreadable",
        }
    }
}

// ══ Parameters ══════════════════════════════════════════════════════════════

/// Which rule a parameter broke. Typed, so a page and a test name the rule
/// rather than match a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// The store's own path-segment rule, [`store::path::check_segment`]: the
    /// rule that refused `nifty` from `StorePath`, now asked before the store.
    Segment(PathError),
    /// An identity is exactly [`IDENTITY_HEX`] characters.
    IdentityLength {
        /// How many characters it was.
        len: usize,
    },
    /// An identity's character is not a hex digit at all.
    IdentityDigit {
        /// Its position, from 0.
        at: usize,
        /// The byte that is not a digit.
        byte: u8,
    },
    /// An identity's character is an upper-case hex digit. Refused, never
    /// folded: one identity has one spelling.
    IdentityCase {
        /// Its position, from 0.
        at: usize,
        /// The upper-case digit.
        byte: u8,
    },
    /// Every digit is `0`: the null value, which names no saved result. The
    /// single-stop routes refused it and the others did not, so one input had
    /// two statuses; it is malformed everywhere now.
    IdentityZero,
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Segment(why) => write!(
                f,
                "{why}; a store key segment is 1 to {} bytes of A-Z, 0-9, '-', '_' and '&', \
                 upper case, and is refused rather than folded",
                store::path::MAX_SEGMENT_LEN
            ),
            Self::IdentityLength { len } => write!(
                f,
                "it is {len} characters; an identity is exactly {IDENTITY_HEX} lowercase hex \
                 digits"
            ),
            Self::IdentityDigit { at, byte } => write!(
                f,
                "byte {byte:#04x} at {at} is not a hex digit; an identity is exactly \
                 {IDENTITY_HEX} lowercase hex digits"
            ),
            Self::IdentityCase { at, byte } => write!(
                f,
                "byte {byte:#04x} at {at} is an upper-case hex digit; an identity is exactly \
                 {IDENTITY_HEX} lowercase hex digits, refused rather than folded"
            ),
            Self::IdentityZero => f.write_str(
                "every digit is 0, the null value, which names no saved result and is never \
                 written for one",
            ),
        }
    }
}

/// One malformed parameter: its name, the value it was sent and the rule it
/// breaks. Always `400`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    /// The query parameter.
    pub parameter: &'static str,
    /// The value as decoded from the query.
    pub value: String,
    /// The rule it breaks.
    pub rule: Rule,
}

/// The most characters of a refused value a sentence quotes; a longer value
/// is quoted to here and its length named, so a refusal stays bounded
/// whatever was sent.
pub const QUOTED_CHARS: usize = 80;

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let quoted: String = self.value.chars().take(QUOTED_CHARS).collect();
        if quoted.len() == self.value.len() {
            write!(f, "{}={quoted:?} is malformed: {}", self.parameter, self.rule)
        } else {
            write!(
                f,
                "{}={quoted:?}... ({} bytes) is malformed: {}",
                self.parameter,
                self.value.len(),
                self.rule
            )
        }
    }
}

/// A store key segment named by a query parameter, checked by the store's own
/// rule before any store file is opened: `symbol=nifty` is malformed here, as
/// `StorePath` would refuse it later, and is never folded into `NIFTY`.
///
/// A byte outside ASCII is malformed whatever it folds to: a dotless `ı` or a
/// Kelvin sign is refused by its first byte, never mapped onto `I` or `K`.
///
/// # Errors
///
/// [`Malformed`] naming `parameter` and the [`PathError`] the store's rule
/// returned.
pub fn segment(parameter: &'static str, value: &str) -> Result<(), Malformed> {
    store::path::check_segment(parameter, value, SegmentCase::Upper).map_err(|why| Malformed {
        parameter,
        value: value.to_owned(),
        rule: Rule::Segment(why),
    })
}

/// `name=` of `query` as one identity, or `None` where the query does not
/// name it. Checked in this order: length, then each character from the
/// first; the first breach is the one named.
///
/// # Errors
///
/// [`Malformed`] naming `name` and the [`Rule`] it breaks.
pub fn identity(query: &str, name: &'static str) -> Result<Option<[u8; 32]>, Malformed> {
    let raw = crate::server::param(query, name);
    if raw.is_empty() {
        return Ok(None);
    }
    identity_of(&raw, name).map(Some)
}

/// `text` as one identity. See [`identity`].
///
/// # Errors
///
/// [`Malformed`] naming `name` and the [`Rule`] it breaks.
pub fn identity_of(text: &str, name: &'static str) -> Result<[u8; 32], Malformed> {
    let refuse = |rule| Malformed {
        parameter: name,
        value: text.to_owned(),
        rule,
    };
    if text.len() != IDENTITY_HEX {
        return Err(refuse(Rule::IdentityLength { len: text.len() }));
    }
    // 64 bytes are 32 pairs exactly, so every slot is written and no arm
    // here is reachable only by a length the check above already refused.
    let mut out = [0_u8; 32];
    for (index, (slot, pair)) in out.iter_mut().zip(text.as_bytes().chunks(2)).enumerate() {
        let mut value = 0_u8;
        for (offset, &byte) in pair.iter().enumerate() {
            let at = index * 2 + offset;
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => return Err(refuse(Rule::IdentityCase { at, byte })),
                _ => return Err(refuse(Rule::IdentityDigit { at, byte })),
            };
            value = (value << 4) | digit;
        }
        *slot = value;
    }
    if out == [0; 32] {
        return Err(refuse(Rule::IdentityZero));
    }
    Ok(out)
}

// ══ Faults ══════════════════════════════════════════════════════════════════

/// A refusal decided from types: its class, its sentence, and the path and
/// `io::ErrorKind` that decided it where one did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// What it is.
    pub class: Class,
    /// Why, in words a page quotes.
    pub why: String,
    /// The path the probe or the read was about: relative to the store root
    /// where it is under it, the root itself where the root is the fault.
    pub path: Option<String>,
    /// The `io::ErrorKind` that decided the class, where one did.
    pub kind: Option<ErrorKind>,
}

impl Fault {
    /// The one status of [`Self::class`].
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.class.status()
    }

    /// A malformed parameter.
    #[must_use]
    pub fn malformed(why: &Malformed) -> Self {
        Self {
            class: Class::Malformed,
            why: why.to_string(),
            path: None,
            kind: None,
        }
    }

    /// A malformed request the route words itself (a field it does not
    /// read, a selector it does not know): still [`Class::Malformed`].
    #[must_use]
    pub fn refused(why: impl Into<String>) -> Self {
        Self {
            class: Class::Malformed,
            why: why.into(),
            path: None,
            kind: None,
        }
    }

    /// What exists could not be read, with the reader's own sentence. The
    /// probe found `path`; the read that followed refused it.
    #[must_use]
    pub fn unreadable(path: Option<String>, why: impl Into<String>) -> Self {
        Self {
            class: Class::Unreadable,
            why: why.into(),
            path,
            kind: None,
        }
    }

    /// The blocking pool is full and nothing was queued.
    #[must_use]
    pub fn saturated(why: impl Into<String>) -> Self {
        Self {
            class: Class::Saturated,
            why: why.into(),
            path: None,
            kind: None,
        }
    }

    /// The one body every identity route answers a fault with: the class's
    /// word as `status`, the sentence as `refusal`, no rows, and what decided
    /// it. A page reads `status` and `refusal` the same way on every route.
    #[must_use]
    pub fn identity_body(&self, identity: Option<[u8; 32]>) -> Value {
        json!({
            "schema_version": 1,
            "status": self.class.word(),
            "identity": identity.map(crate::server::hex32),
            "refusal": self.why,
            "rows": [],
            "path": self.path,
            "error_kind": self.kind.map(|kind| format!("{kind:?}")),
            "rule": DECISION,
        })
    }
}

/// What a probe found: the entry the key names, and the path a page quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    /// The entry's full path.
    pub path: PathBuf,
    /// The same path relative to the store root, as a page quotes it.
    pub shown: String,
}

/// ONE probe of the entry `relative` names under `root`: held, not found, or
/// there and unreadable. `what` names the kind of thing asked for, as the
/// not-found sentence says it ("Boolean catalog").
///
/// * `symlink_metadata` of the entry answers: held. A symbolic link is held
///   even when it points at nothing: the entry is there, and the read that
///   follows names why it cannot be read — never a `404` for a folder that
///   exists.
/// * It answers `NotFound`: [`Class::NotFound`] ([`absent`]), whether the
///   entry, its namespace or the root itself is the missing part — nothing
///   is there, and the sentence names the root so a misconfigured one is
///   visible. (A root that is there and is not a directory never answers
///   `NotFound`: the entry below it answers `NotADirectory`, the next arm.)
/// * Any other `io::ErrorKind` (`PermissionDenied`, `NotADirectory` for a file
///   where a folder should be, an I/O error): [`Class::Unreadable`] — the
///   path exists and could not be read — naming the path, the root and the
///   kind.
///
/// # Errors
///
/// The [`Fault`] above.
pub fn probe(root: &Path, relative: &Path, what: &str) -> Result<Held, Fault> {
    let path = root.join(relative);
    let shown = relative.display().to_string();
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(Held { path, shown }),
        Err(why) if why.kind() == ErrorKind::NotFound => Err(absent(root, shown, what)),
        Err(why) => Err(Fault {
            class: Class::Unreadable,
            why: format!(
                "{what} at {shown} under the configured store root {} could not be read: {why} \
                 ({:?})",
                root.display(),
                why.kind()
            ),
            path: Some(shown),
            kind: Some(why.kind()),
        }),
    }
}

/// One probe answered `NotFound` for `shown` (relative to `root`): [`probe`]'s
/// own, or a reader's that returned a typed absence (`Ok(None)`) rather than
/// an error. [`Class::NotFound`], naming the root that was probed, so a store
/// root that is not the one the command wrote reads as exactly that: every
/// identity is "not held under <root>", never a `503` that blames the store.
#[must_use]
pub fn absent(root: &Path, shown: String, what: &str) -> Fault {
    Fault {
        class: Class::NotFound,
        why: format!(
            "{what} not held: one probe of {shown} under the configured store root {} found \
             nothing (NotFound); nothing else was searched, and the dashboard's BRUTEX_STORE must \
             name the store the command wrote",
            root.display()
        ),
        path: Some(shown),
        kind: Some(ErrorKind::NotFound),
    }
}

/// The folder under the store root that holds one kind of saved research
/// result, one entry per identity: `<root>/<namespace>/<identity hex>/`. Each
/// is the literal the `cli` reader of that kind opens (none of them is public
/// there); `status_rule_tests` binds every one to a result that reader opened,
/// so the two cannot drift apart silently.
pub mod namespace {
    /// `/boolean-candidates.json`'s catalogs.
    pub const BOOLEAN_CANDIDATES: &str = "boolean-candidates-v1";
    /// `/boolean-statistics.json`'s evidence.
    pub const BOOLEAN_STATISTICS: &str = "boolean-statistics-v1";
    /// `/boolean-admission.json`'s evidence.
    pub const BOOLEAN_ADMISSION: &str = "boolean-admission-v1";
    /// `/boolean-qualification.json`'s evidence.
    pub const BOOLEAN_QUALIFICATION: &str = "boolean-qualification-v1";
    /// `/index-stop.json`'s single-stop candidates: `cli`'s own public name.
    pub const INDEX_STOP: &str = cli::index_stop_store::NAMESPACE;
    /// `/expression-search.json`'s checkpoint namespace.
    pub const EXPRESSION_SEARCH: &str = "expression-search-v1";
    /// `/sweep-evidence.json`'s attempt evidence.
    pub const SWEEP_EVIDENCE: &str = "results/sweep-evidence-v1";
    /// `/candidate-trades.json`'s and-mask candidate catalogs.
    pub const CANDIDATE_TRADES: &str = "results/candidate-trades-v1";
    /// `/candidate-trades.json`'s expression candidate catalogs.
    pub const EXPRESSION_CANDIDATE_TRADES: &str = "results/expression-candidate-trades-v1";
}

/// [`probe`] of `identity`'s entry under `namespace`: ONE `symlink_metadata`
/// before any reader opens anything, so a well-formed identity that names
/// nothing is `404` and never a reader's `503`.
///
/// # Errors
///
/// [`probe`]'s.
pub fn probe_identity(
    root: &Path,
    namespace: &str,
    identity: [u8; 32],
    what: &str,
) -> Result<Held, Fault> {
    probe(
        root,
        &Path::new(namespace).join(crate::server::hex32(identity)),
        what,
    )
}

// ══ The one path every identity route answers through ═══════════════════════

/// One answer: status, content type, body.
pub type Reply = (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
);

/// THE ONE ORDER every identity route answers in (D-7550): the route's own
/// parse first — its `400` is answered before any admission or store file —
/// then the blocking work on the shared detail pool (`crate::detail::run`).
/// A full pool is `429` with the route's `saturated` sentence and nothing
/// queued; a task that could not be joined is `503`. `refused` is the route's
/// own refusal shape, so its pages read the same words they always read.
pub async fn serve<A, W>(
    asked: Result<A, Reply>,
    refused: fn(StatusCode, &str) -> Reply,
    saturated: &'static str,
    work: W,
) -> Reply
where
    A: Send + 'static,
    W: FnOnce(A) -> Reply + Send + 'static,
{
    let asked = match asked {
        Ok(asked) => asked,
        Err(refusal) => return refusal,
    };
    match crate::detail::run(move || work(asked)).await {
        Ok(reply) => reply,
        Err(crate::detail::RunError::Saturated) => refused(Class::Saturated.status(), saturated),
        Err(crate::detail::RunError::Join(why)) => refused(
            Class::Unreadable.status(),
            &format!("the bounded detail task could not be joined: {why}"),
        ),
    }
}

/// A found page, or the one body of its fault, with the route's content type.
/// A page over [`crate::detail::MAX_RESPONSE_BYTES`] is refused whole with the
/// route's `too_large` sentence: `503`, no prefix, as every route here bounds
/// its answer.
#[must_use]
pub fn page(
    body: Result<String, Fault>,
    identity: Option<[u8; 32]>,
    content_type: &'static str,
    refused: fn(StatusCode, &str) -> Reply,
    too_large: &str,
) -> Reply {
    let headers = [(axum::http::header::CONTENT_TYPE, content_type)];
    match body {
        Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
            (StatusCode::OK, headers, body)
        }
        Ok(_) => refused(Class::Unreadable.status(), too_large),
        Err(fault) => (
            fault.status(),
            headers,
            fault.identity_body(identity).to_string(),
        ),
    }
}

/// `root` as the route was configured with it, or the configuration's own
/// refusal as [`Class::Unreadable`]: a server with no store root cannot read
/// its store, which is no client's fault.
///
/// # Errors
///
/// The configuration's sentence, as an unreadable fault.
pub fn configured(root: Result<PathBuf, String>) -> Result<PathBuf, Fault> {
    root.map_err(|why| Fault {
        class: Class::Unreadable,
        why: format!("no store root is configured: {why}"),
        path: None,
        kind: None,
    })
}

#[cfg(test)]
#[path = "status_rule_tests.rs"]
mod tests;

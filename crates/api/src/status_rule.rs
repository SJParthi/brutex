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
//!   `/index-stop.json`, a `200` with `"status":"missing"` on `/frontier.json`,
//!   `/expression-search.json` and `/sweep-evidence.json`, and a `400` on
//!   `/candidate-trades.json`: four answers to one question.
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
    /// The blocking pool is full; nothing was queued.
    Saturated,
    /// What exists could not be read: permission, a file where a folder
    /// should be, an I/O error, a torn or undecodable file, a writer holding
    /// it, or a store root that is not there.
    Unreadable,
}

impl Class {
    /// Every class, in the order of the table above.
    pub const ALL: [Self; 6] = [
        Self::Malformed,
        Self::NotFound,
        Self::Retired,
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
        }
    }
}

/// One malformed parameter: its name and the rule it breaks. Always `400`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    /// The query parameter.
    pub parameter: &'static str,
    /// The rule it breaks.
    pub rule: Rule,
}

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is malformed: {}", self.parameter, self.rule)
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
/// * It answers `NotFound`: one `metadata` of `root`. A directory there means
///   the store holds no such entry — [`Class::NotFound`]. A root that is not
///   there, or is not a directory, is [`Class::Unreadable`]: the server cannot
///   read its own store, which no client's question caused.
/// * Any other `io::ErrorKind` (`PermissionDenied`, `NotADirectory` for a file
///   where a folder should be, an I/O error): [`Class::Unreadable`], naming
///   the path and the kind.
///
/// # Errors
///
/// The [`Fault`] above.
pub fn probe(root: &Path, relative: &Path, what: &str) -> Result<Held, Fault> {
    let path = root.join(relative);
    let shown = relative.display().to_string();
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(Held { path, shown }),
        Err(why) if why.kind() == ErrorKind::NotFound => match std::fs::metadata(root) {
            Ok(meta) if meta.is_dir() => Err(Fault {
                class: Class::NotFound,
                why: format!(
                    "{what} not held: the store holds nothing at {shown} under its configured \
                     root (one probe, NotFound); nothing else was searched"
                ),
                path: Some(shown),
                kind: Some(ErrorKind::NotFound),
            }),
            Ok(_) => Err(root_fault(root, ErrorKind::NotADirectory, "is not a directory")),
            Err(root_why) => Err(root_fault(root, root_why.kind(), &root_why.to_string())),
        },
        Err(why) => Err(Fault {
            class: Class::Unreadable,
            why: format!(
                "{what} at {shown} under the configured store root could not be read: {why} \
                 ({:?})",
                why.kind()
            ),
            path: Some(shown),
            kind: Some(why.kind()),
        }),
    }
}

/// The store root itself is the fault: `503`, naming the root and the kind.
fn root_fault(root: &Path, kind: ErrorKind, said: &str) -> Fault {
    let shown = root.display().to_string();
    Fault {
        class: Class::Unreadable,
        why: format!(
            "the configured store root {shown} cannot be read ({kind:?}: {said}); the dashboard's \
             BRUTEX_STORE must name the store the command wrote, and no other folder was searched"
        ),
        path: Some(shown),
        kind: Some(kind),
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

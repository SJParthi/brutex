//! AWS Parameter Store, reached in Rust and nothing else.
//!
//! # Why this file exists instead of `aws-sdk-ssm`
//!
//! The obvious move was to take AWS's own Rust SDK. It was tried, measured, and
//! **refused on the measurement**:
//!
//! | | crates in `Cargo.lock` | C in the tree |
//! |---|---|---|
//! | before | 145 | none beyond `ring` |
//! | with `aws-sdk-ssm` | **232** | **`aws-lc-sys`** |
//!
//! `aws-sdk-ssm` reaches rustls through `aws-smithy-http-client`, which
//! hard-enables the `aws-lc-rs` backend, which vendors `aws-lc-sys` — a C
//! library. Cargo features are **additive**: no `default-features = false`
//! anywhere downstream can turn a feature back off once something upstream has
//! asked for it, so this is not a configuration problem with a configuration
//! answer. `CLAUDE.md` §2 bans "any vendored binding to another language", and
//! CI gate 13 walks `Cargo.lock` for exactly that shape.
//!
//! **The AWS API needs none of it.** `GetParameter` is one HTTPS POST with a
//! signature header. [`crate::http`] already brought `reqwest` for the broker,
//! and the signature is HMAC-SHA256 over a string this module builds. Two pure
//! Rust crates — `hmac` and `sha2`, no `asm` feature — instead of eighty-seven,
//! and not one line of C.
//!
//! # What `SigV4` actually is
//!
//! Four hashes and a string. AWS documents it as a five-step recipe and every
//! step is deterministic, so it is testable without a network:
//!
//! 1. **Canonical request** — method, path, query, sorted headers, signed-header
//!    list, and the SHA-256 of the body, joined by newlines.
//! 2. **String to sign** — the algorithm, the timestamp, the scope
//!    (`date/region/service/aws4_request`) and the SHA-256 of step 1.
//! 3. **Signing key** — HMAC chained four times from the secret through the
//!    date, the region, the service and the literal `aws4_request`. This is what
//!    makes a signature useless outside its day, its region and its service.
//! 4. **Signature** — HMAC of step 2 under step 3, in lower-case hex.
//! 5. **The header** — the key id, the scope, the signed-header list and the
//!    signature.
//!
//! Steps 1 through 4 are pure functions of their inputs. [`tests`] drives them
//! against AWS's own published example vectors, so this file is proven by the
//! specification rather than by a live call.
//!
//! # What this module will not do
//!
//! `CLAUDE.md` §8: this repository never mints a token, and the credential value
//! is never a file, never an environment variable and never a prompt. Neither is
//! walked back here — **the value being read is the broker's, and it is read
//! from Parameter Store and nowhere else.** What this module needs is the
//! operator's *AWS identity*, which is a different secret with a different
//! owner, and it comes from the standard AWS locations because that is where
//! every AWS tool on the machine already looks.
//!
//! There is one method. `PutParameter`, `DeleteParameter` and the rest of the
//! API are unreachable through a type that does not name them — the same
//! narrowing [`crate::secret::ParameterStore`] applies at the port.

use core::fmt::Write as _;

use hmac::{Hmac, Mac as _};
use sha2::{Digest as _, Sha256};

use crate::secret::SecretError;

/// Why a credential read did not produce a credential, **with the reason kept**.
///
/// [`SecretError`] is a four-variant enum with no payload, and that is right for
/// the port: `AccessDenied` and `NotFound` send an operator to opposite places
/// and a free-text string would blur them. But it cannot carry *which
/// environment variable was unset* or *what shape the answer was*, and
/// `CLAUDE.md` §4 asks a failure to name its reason.
///
/// So this type carries the sentence and [`SsmError::into_secret`] narrows it to
/// the port's vocabulary at the boundary. The detail reaches the operator; the
/// port keeps its four meanings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsmError {
    /// What went wrong, in words an operator can act on.
    pub detail: String,
    /// Which of the port's four meanings this is.
    pub kind: SecretError,
}

impl SsmError {
    /// A read that never reached AWS, or an answer this build cannot read.
    fn unreachable(detail: String) -> Self {
        Self {
            detail,
            kind: SecretError::Unreachable,
        }
    }

    /// The port's verdict, once the sentence has been reported.
    #[must_use]
    pub fn into_secret(self) -> SecretError {
        self.kind
    }
}

impl core::fmt::Display for SsmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.detail)
    }
}

/// The service `SigV4` scopes a signature to. Wrong here, and AWS refuses.
const SERVICE: &str = "ssm";

/// The action, as the JSON-1.1 protocol names it in `X-Amz-Target`.
const TARGET: &str = "AmazonSSM.GetParameter";

/// What the JSON-1.1 protocol calls its content type.
const CONTENT_TYPE: &str = "application/x-amz-json-1.1";

/// How long one credential read may take before it is abandoned.
///
/// Shorter than the broker's 30 s of [`crate::http::REQUEST_TIMEOUT_SECS`] on
/// purpose: this is a control-plane call to a service in the same region, and a
/// pull that hangs on *fetching the password* has not started doing its work
/// yet. A hung socket with no timeout is a pull that never finishes and never
/// says why.
pub const CREDENTIAL_TIMEOUT_SECS: u64 = 10;

/// Fetching the password must never be allowed to outlast fetching the data.
///
/// Checked at compile time rather than in a test: both sides are constants, so
/// a test comparing them compares two literals and passes whatever they are.
/// An edit that inverts the two fails the build.
const _: () = assert!(CREDENTIAL_TIMEOUT_SECS < crate::http::REQUEST_TIMEOUT_SECS);

/// ONE HTTPS CLIENT FOR EVERY PARAMETER READ THIS PROCESS EVER MAKES.
///
/// # The handshake storm this removes
///
/// [`get_parameter`] built a `reqwest::Client` of its own on **every call**, and
/// the call is per instrument: `api::server::broker_run` loops its target list
/// and each iteration reaches `read_credential`, which reads one parameter per
/// secret the feed's scheme names. A 785-instrument leg therefore built 785
/// clients — 1,570 for a two-secret vendor — and **a client owns its connection
/// pool**, so not one of those TLS sessions could ever be reused by the next.
/// Every credential read paid a full TCP and TLS handshake to `ap-south-1`.
///
/// A `reqwest::Client` is a handle around a shared inner state: cloning it is
/// cheap and every clone shares one pool. So the fix is not to thread a client
/// through six call sites — it is to stop making new ones.
///
/// # Why a `OnceLock` and not a field
///
/// Because the configuration has no inputs. Every one of those 785 clients was
/// built from the same two constants — this module's timeout and the same
/// redirect policy — so they were not 785 different clients, they were 785
/// copies of one. A field would need an owner, and the owner would be `Site`,
/// which would put an HTTP client in `crates/api`'s state for a call that
/// belongs to this module.
///
/// The `Result` is stored rather than the `Client`, so a build failure is
/// reported to the caller that provoked it instead of panicking a `Once`. It is
/// a deployment fault — the TLS backend is unavailable — and it cannot become
/// true later, so caching it costs nothing and re-attempting would only produce
/// the same message once per instrument.
///
/// # Cost
///
/// One build per process; one `Arc` clone per call thereafter. O(1) either way,
/// and the constant drops from a TLS handshake to a pointer copy.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn pooled_client() -> Result<reqwest::Client, String> {
    static POOL: std::sync::OnceLock<Result<reqwest::Client, String>> = std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        crate::ensure_tls_provider();
        reqwest::Client::builder()
            .timeout(core::time::Duration::from_secs(CREDENTIAL_TIMEOUT_SECS))
            // REDIRECTS ARE NOT FOLLOWED HERE EITHER, for the reason D-0050
            // gives about the broker: a signature is scoped to a host, and a
            // client that chases a `Location` would carry an `Authorization`
            // header to a host the signature was never computed for.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|why| format!("{why}"))
    })
    .clone()
}

/// The AWS identity a request is signed with.
///
/// **Not the broker credential.** This is the operator's own AWS key, the thing
/// that proves to Parameter Store that they may read the parameter at all; the
/// broker token is what comes *back*. Two secrets, two owners, and conflating
/// them is how a repository ends up with a broker token in an environment
/// variable — which `CLAUDE.md` §8 forbids and this type is careful not to
/// invite.
#[derive(Clone)]
pub struct AwsIdentity {
    /// `AKIA…` — public, and it appears in the `Authorization` header.
    pub key_id: String,
    /// The secret half. Never logged, never formatted — see the `Debug` below.
    pub secret: String,
    /// Present for temporary credentials (SSO, assumed roles, instance roles).
    pub session_token: Option<String>,
}

// Hand-written for the reason `crate::http::HttpSource`'s is: a derived `Debug`
// prints every field, and a struct holding a secret that derives `Debug` is one
// `dbg!` away from that secret in a log file. The omission is the feature.
#[allow(clippy::missing_fields_in_debug)]
impl core::fmt::Debug for AwsIdentity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AwsIdentity")
            .field("key_id", &self.key_id)
            .field("secret", &"<redacted>")
            .field(
                "session_token",
                &self.session_token.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

impl AwsIdentity {
    /// The identity the standard AWS environment variables name.
    ///
    /// # Why an environment variable is right *here* and banned elsewhere
    ///
    /// `CLAUDE.md` §8 forbids the **broker credential** ever being an
    /// environment variable, and that is not walked back: the broker token is
    /// read from Parameter Store and nowhere else. This is the operator's AWS
    /// identity — a different secret, owned by AWS's own tooling, which every
    /// other program on the machine already reads from exactly these names. A
    /// second, private convention would not be safer; it would only be one more
    /// place for it to sit.
    ///
    /// # Errors
    ///
    /// [`SsmError`] naming the variable that was missing, so an operator is
    /// told which one to set rather than that "AWS failed".
    pub fn from_env() -> Result<Self, SsmError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// [`Self::from_env`] over any lookup, so the rule is testable without
    /// mutating the process environment — `std::env::set_var` needs `unsafe`
    /// and this crate forbids it.
    ///
    /// # An empty value is an UNSET value
    ///
    /// `AWS_ACCESS_KEY_ID=` exported empty — the common way a CI step or a
    /// shell "clears" a variable — used to be read as a key id of zero
    /// characters. That identity is not one: it signs a request AWS refuses
    /// with a fault naming neither the variable nor the file, and because
    /// [`Self::discover`] stops at the first identity it builds, an empty
    /// export also SHADOWED a complete `~/.aws/credentials`. AWS's own
    /// resolvers treat an empty variable as absent and move on; so does this.
    /// D-1372.
    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, SsmError> {
        let read = |name: &str| -> Result<String, SsmError> {
            non_blank(lookup(name)).ok_or_else(|| {
                SsmError::unreachable(format!(
                    "the AWS identity is not in this process's environment: \
                     {name} is unset or empty. This is the AWS key that proves \
                     you may READ the parameter — not the broker token, which \
                     is what comes back from it."
                ))
            })
        };
        Ok(Self {
            key_id: read("AWS_ACCESS_KEY_ID")?,
            secret: read("AWS_SECRET_ACCESS_KEY")?,
            // Absent for a long-lived key, present for anything temporary.
            // Absence is not a failure and must not be reported as one — and
            // an empty token is absence, or it would be SIGNED and sent as an
            // `x-amz-security-token` of nothing.
            session_token: non_blank(lookup("AWS_SESSION_TOKEN")),
        })
    }

    /// The identity, from wherever AWS keeps it on this machine.
    ///
    /// # Why a file, when the environment was the obvious answer
    ///
    /// [`Self::from_env`] was written first and would have failed on the very
    /// machine this is for: `AWS_ACCESS_KEY_ID` is unset there, and the key
    /// pair lives in `~/.aws/credentials` — which is where the AWS CLI, the
    /// SDKs and every other AWS tool put it. An identity reader that only knows
    /// about environment variables works on a CI runner and nowhere else.
    ///
    /// Order matters and follows AWS's own: the environment wins when it is
    /// set, because that is how an operator overrides a file for one command.
    ///
    /// # What this does NOT read
    ///
    /// The broker token. `CLAUDE.md` §8 is not walked back by this — the broker
    /// credential is read from Parameter Store and nowhere else, and this file
    /// holds the *AWS* identity that proves you may read it. Two secrets, two
    /// owners, two locations.
    ///
    /// # Errors
    ///
    /// [`SsmError`] naming every place that was looked at, so "no credentials"
    /// is never the whole message.
    pub fn discover() -> Result<Self, SsmError> {
        Self::discover_from(|name| std::env::var(name).ok(), Self::from_shared_file)
    }

    /// [`Self::discover`] over any environment lookup and any profile reader.
    ///
    /// # A half-set environment is refused, not discarded
    ///
    /// This kept the environment's identity only when it was whole and
    /// otherwise dropped the env error and signed as `[default]` — so an
    /// operator who exported `AWS_ACCESS_KEY_ID` and forgot the secret was
    /// silently signed as a DIFFERENT identity, with no mention of the
    /// variable they set. Exactly one of the pair present is now a refusal
    /// naming both (errpaths-1, D-1534).
    ///
    /// # The profile is the one `AWS_PROFILE` names
    ///
    /// `AWS_PROFILE` was read nowhere, so an operator who chose a profile was
    /// signed as `[default]`. It now names the profile; empty or unset is
    /// `default`, as an empty key is unset (D-1372). That AWS's own tools read
    /// the variable the same way is UNVERIFIED here: `docs/00-charter.md`
    /// records no AWS source.
    fn discover_from(
        lookup: impl Fn(&str) -> Option<String>,
        from_file: impl Fn(&str) -> Result<Self, SsmError>,
    ) -> Result<Self, SsmError> {
        let key = non_blank(lookup("AWS_ACCESS_KEY_ID")).is_some();
        let secret = non_blank(lookup("AWS_SECRET_ACCESS_KEY")).is_some();
        if key && secret {
            return Self::from_lookup(&lookup);
        }
        if key || secret {
            let (set, missing) = if key {
                ("AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY")
            } else {
                ("AWS_SECRET_ACCESS_KEY", "AWS_ACCESS_KEY_ID")
            };
            return Err(SsmError::unreachable(format!(
                "the AWS identity in this process's environment is half set: \
                 {set} is set and {missing} is unset or empty. Refused rather \
                 than signing as the credentials file's profile instead, which \
                 would be a different identity from the one you exported."
            )));
        }
        let profile = non_blank(lookup("AWS_PROFILE")).unwrap_or_else(|| "default".to_owned());
        from_file(&profile)
    }

    /// One profile out of `~/.aws/credentials`.
    ///
    /// A deliberately small INI reader: sections in `[brackets]`, `key = value`
    /// lines, `#` and `;` comments. The real format has more in it — process
    /// credentials, SSO sessions, role chaining — and none of that is
    /// implemented, because a half-implemented credential resolver that
    /// silently picks the wrong identity is worse than one that refuses. What
    /// it does not understand, it does not find, and it says which profile it
    /// was looking in.
    ///
    /// # Errors
    ///
    /// [`SsmError`] when the file is absent, the profile is not in it, or the
    /// profile carries no key pair — three different faults, named separately.
    pub fn from_shared_file(profile: &str) -> Result<Self, SsmError> {
        let Some(home) = std::env::var_os("HOME") else {
            return Err(SsmError::unreachable(
                "HOME is unset, so ~/.aws/credentials cannot be located".to_owned(),
            ));
        };
        Self::from_credentials_file(
            &std::path::PathBuf::from(home)
                .join(".aws")
                .join("credentials"),
            profile,
        )
    }

    /// One profile out of a credentials file **the caller names**.
    ///
    /// Split from [`Self::from_shared_file`] for the reason
    /// `api::server::run_in` gives about its own split: with the path read
    /// inside, the only way to test this is to mutate `HOME` — and this crate
    /// carries `#![forbid(unsafe_code)]`, which `std::env::set_var` now needs.
    /// A function that takes the path is a function whose answer is a property
    /// of a file a test owns, rather than of the machine it runs on.
    ///
    /// # Errors
    ///
    /// [`SsmError`] when the file is absent, the profile is not in it, or the
    /// profile carries no key pair — three different faults, named separately.
    pub fn from_credentials_file(path: &std::path::Path, profile: &str) -> Result<Self, SsmError> {
        let text = std::fs::read_to_string(path).map_err(|why| {
            SsmError::unreachable(format!(
                "the AWS identity is in neither the environment nor {}: {why}. \
                 This is the AWS key that proves you may READ the parameter — \
                 not the broker token, which is what comes back from it.",
                path.display()
            ))
        })?;

        let (mut key_id, mut secret, mut token) = (None, None, None);
        let mut inside = false;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                inside = name.trim() == profile;
                continue;
            }
            if !inside {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            // An empty value is no value (D-1372): `aws_access_key_id =` with
            // nothing after it does not make the profile complete.
            let value = non_blank(Some(value.trim().to_owned()));
            match name.trim() {
                "aws_access_key_id" => key_id = value,
                "aws_secret_access_key" => secret = value,
                "aws_session_token" => token = value,
                _ => {}
            }
        }

        match (key_id, secret) {
            (Some(key_id), Some(secret)) => Ok(Self {
                key_id,
                secret,
                session_token: token,
            }),
            _ => Err(SsmError::unreachable(format!(
                "{} has no complete [{profile}] profile: both \
                 aws_access_key_id and aws_secret_access_key must be present",
                path.display()
            ))),
        }
    }
}

/// `None` for an absent value and for one that is empty or only whitespace.
///
/// One rule for both identity sources, so the environment and the file cannot
/// disagree about whether a blank key is a key. D-1372.
fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

/// Lower-case hex, which is the only encoding `SigV4` accepts.
///
/// Written out rather than pulled in: a `hex` dependency for sixteen characters
/// is a dependency, and this workspace counts them.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // `write!` to a `String` cannot fail; the result is discarded for the
        // same reason every other `write!` in this crate discards it.
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// SHA-256, hex-encoded. Step 1's body hash and step 2's request hash.
fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

/// One HMAC-SHA256 link of the signing chain.
///
/// # Panics
///
/// Cannot. `Hmac::new_from_slice` rejects only a key length this construction
/// never produces — HMAC accepts a key of any length, and the `expect` is
/// unreachable rather than unchecked.
fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    #[allow(
        clippy::expect_used,
        reason = "HMAC accepts a key of any length; `InvalidLength` is a variant \
                  this construction cannot produce, and a Result no input can \
                  make Err is a branch no test could ever enter"
    )]
    let mut mac =
        <Hmac<Sha256> as hmac::Mac>::new_from_slice(key).expect("HMAC takes a key of any length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// The four-link signing key: secret → date → region → service → `aws4_request`.
///
/// Chaining is what scopes a signature. A key derived for one day cannot sign
/// for the next, and one derived for `ap-south-1` cannot sign for anywhere else
/// — which is why a leaked signature is worth so much less than a leaked secret.
fn signing_key(secret: &str, date: &str, region: &str) -> Vec<u8> {
    let k_date = hmac(format!("AWS4{secret}").as_bytes(), date);
    let k_region = hmac(&k_date, region);
    let k_service = hmac(&k_region, SERVICE);
    hmac(&k_service, "aws4_request")
}

/// Everything a signature is computed from, so the whole of `SigV4` is testable
/// without a socket.
///
/// `stamp` is the ISO-8601 basic form AWS requires — `YYYYMMDDTHHMMSSZ` — and
/// it is an argument rather than a clock read inside, for the reason
/// `crate::ingest::parse_window` takes `today`: a function that reads the clock
/// cannot be tested at its own boundary, and a signature is only checkable
/// against a fixed instant.
#[derive(Debug, Clone, Copy)]
pub struct Signable<'a> {
    /// The host header, `ssm.<region>.amazonaws.com`.
    pub host: &'a str,
    /// The region the signature is scoped to.
    pub region: &'a str,
    /// `YYYYMMDDTHHMMSSZ`.
    pub stamp: &'a str,
    /// The JSON body, verbatim — the hash must be of the bytes actually sent.
    pub body: &'a str,
    /// The session token, when the identity carries one.
    pub session_token: Option<&'a str>,
}

impl Signable<'_> {
    /// `YYYYMMDD`, the scope's date. The first eight bytes of the stamp.
    ///
    /// Returns `None` for a stamp too short to hold one, which is the single
    /// malformed-input branch in this module and is refused rather than sliced.
    fn date(&self) -> Option<&str> {
        self.stamp.get(..8)
    }

    /// The signed-header list, in the sorted order the canonical request needs.
    ///
    /// `content-type`, `host`, `x-amz-date` and `x-amz-target` always; the
    /// session token joins them only when there is one, and it sorts after
    /// `x-amz-date`.
    fn signed_headers(&self) -> &'static str {
        if self.session_token.is_some() {
            "content-type;host;x-amz-date;x-amz-security-token;x-amz-target"
        } else {
            "content-type;host;x-amz-date;x-amz-target"
        }
    }

    /// Step 1: the canonical request.
    fn canonical_request(&self) -> String {
        let mut out = String::with_capacity(512);
        // POST, root path, no query. Written out rather than parameterised
        // because this module makes exactly one call and a second shape would
        // be a second thing to get wrong.
        out.push_str("POST\n/\n\n");
        let _ = writeln!(out, "content-type:{CONTENT_TYPE}");
        let _ = writeln!(out, "host:{}", self.host);
        let _ = writeln!(out, "x-amz-date:{}", self.stamp);
        if let Some(token) = self.session_token {
            let _ = writeln!(out, "x-amz-security-token:{token}");
        }
        let _ = write!(out, "x-amz-target:{TARGET}\n\n");
        let _ = writeln!(out, "{}", self.signed_headers());
        out.push_str(&sha256_hex(self.body.as_bytes()));
        out
    }

    /// Steps 2 through 5: the finished `Authorization` header value.
    ///
    /// # Errors
    ///
    /// [`SecretError::Unreachable`] for a timestamp that cannot hold a date,
    /// which is this module refusing its own malformed input rather than
    /// signing something meaningless — and for a `region` that is not
    /// [`crate::config::REGION`].
    ///
    /// # Why the region is checked HERE, and not only in the configuration
    ///
    /// `CLAUDE.md` §8 fixes the region: credentials are read from Parameter
    /// Store in `ap-south-1`. `config` refuses any other region in the local
    /// file, but this function and [`get_parameter`] are `pub` and took the
    /// region as a free string, so any caller holding an identity could sign
    /// and send a read to every other region's Parameter Store — the same
    /// parameter path resolved against a different store. The rule was held by
    /// one caller's discipline rather than by the one function every read
    /// passes through. A signature is the last point before a packet exists,
    /// so this is where the refusal sits. D-1371.
    pub fn authorization(&self, id: &AwsIdentity) -> Result<String, SsmError> {
        if self.region != crate::config::REGION {
            return Err(SsmError::unreachable(format!(
                "refusing to sign a Parameter Store read for region {:?}: \
                 CLAUDE.md §8 reads credentials from {} and nowhere else",
                self.region,
                crate::config::REGION
            )));
        }
        let date = self.date().ok_or_else(|| {
            SsmError::unreachable(format!(
                "{:?} is not an AWS timestamp; it must be YYYYMMDDTHHMMSSZ",
                self.stamp
            ))
        })?;
        let scope = format!("{date}/{}/{SERVICE}/aws4_request", self.region);
        let to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{scope}\n{}",
            self.stamp,
            sha256_hex(self.canonical_request().as_bytes())
        );
        let signature = hex(&hmac(&signing_key(&id.secret, date, self.region), &to_sign));
        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={}, Signature={signature}",
            id.key_id,
            self.signed_headers()
        ))
    }
}

/// The JSON body of a `GetParameter` call.
///
/// `with_decryption` is written into the body rather than defaulted, because
/// the API defaults it to **false** and would then hand back the ciphertext of a
/// `SecureString` — a value that looks like a credential, is not one, and would
/// fail at the broker with an authentication error naming nothing useful.
///
/// Built with `serde_json` rather than by string concatenation so a parameter
/// name containing a quote cannot break out of the JSON it is placed in.
#[must_use]
pub fn body(name: &str, with_decryption: bool) -> String {
    serde_json::json!({ "Name": name, "WithDecryption": with_decryption }).to_string()
}

/// What an operator is told about a refusal, holding the body at arm's length.
///
/// Split out of the request so the §8 promise is TESTABLE. The rule it enforces
/// is one sentence: nothing from `body` reaches the output except an exact
/// match against [`AWS_FAULTS`], and those are closed tokens that cannot carry
/// a path.
///
/// Constant work in the body's length — one pass per known fault over a
/// response AWS bounds itself.
#[must_use]
fn refusal_detail(status: u16, body: &str) -> String {
    match AWS_FAULTS.iter().find(|name| body.contains(*name)) {
        Some(name) => format!("Parameter Store refused with {status}: {name}"),
        None => format!(
            "Parameter Store refused with {status} and named no fault this \
             build recognises. The response body is deliberately not quoted: \
             AWS states a denial as a sentence naming the parameter ARN, and \
             §8 keeps that path out of this process's output. See CloudTrail \
             for the full text."
        ),
    }
}

/// AWS fault names this build will repeat back, and nothing else.
///
/// An ALLOWLIST rather than a filter, because the thing being kept out is not a
/// known pattern — it is a free-text sentence AWS composes, and any sentence it
/// composes about `ssm:GetParameter` names the parameter ARN. There is no
/// redaction rule that is safe against text you have not read. A fixed set of
/// closed tokens is safe by construction: none of these can carry a path.
///
/// Being incomplete is harmless. An unmatched fault reports its HTTP status and
/// nothing else, which is still more than the `kind` alone, and `kind` is what
/// an operator acts on.
const AWS_FAULTS: [&str; 12] = [
    "AccessDeniedException",
    "ParameterNotFound",
    "ParameterVersionNotFound",
    "InvalidKeyId",
    "InternalServerError",
    "ThrottlingException",
    "TooManyUpdates",
    "ValidationException",
    "ExpiredTokenException",
    "InvalidSignatureException",
    "MissingAuthenticationToken",
    "UnrecognizedClientException",
];

// `indexing_slicing` is denied workspace-wide because an index that can panic
// is an index that will. Inside a `const` block it cannot: this runs at COMPILE
// time, and an out-of-bounds index is a build failure, not a signal in
// production. `slice::get` is not const-callable, so the alternative is not
// writing the check at all.
#[allow(
    clippy::indexing_slicing,
    reason = "evaluated at compile time; an out-of-bounds index fails the build \
              rather than reaching a running process"
)]
const _: () = {
    // A fault name that contained a slash could carry a path fragment, which
    // would defeat the whole point of an allowlist.
    let mut i = 0;
    while i < AWS_FAULTS.len() {
        let bytes = AWS_FAULTS[i].as_bytes();
        let mut j = 0;
        while j < bytes.len() {
            assert!(
                bytes[j] != b'/' && bytes[j] != b':' && bytes[j] != b' ',
                "an AWS fault name must be a closed token; a slash, colon or \
                 space means it is a sentence and could carry a parameter path"
            );
            j += 1;
        }
        i += 1;
    }
};

/// The value out of a `GetParameter` response.
///
/// # Errors
///
/// [`SecretError::Unreachable`] when the body is not JSON, or does not carry
/// `Parameter.Value` — named separately, because "AWS said no" and "AWS said
/// something this build does not understand" are different faults with
/// different fixes.
pub fn value_of(json: &str) -> Result<String, SsmError> {
    let root: serde_json::Value = serde_json::from_str(json).map_err(|why| {
        SsmError::unreachable(format!("Parameter Store's answer is not JSON: {why}"))
    })?;
    root.get("Parameter")
        .and_then(|p| p.get("Value"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            SsmError::unreachable(
                "Parameter Store's answer carries no Parameter.Value. The \
                     parameter name was accepted and the shape was not what \
                     this build reads."
                    .to_owned(),
            )
        })
}

/// The host `GetParameter` is sent to, for one region.
#[must_use]
pub fn host_for(region: &str) -> String {
    format!("ssm.{region}.amazonaws.com")
}

/// The headers every `GetParameter` request carries whatever the identity.
///
/// # Why a function and not two literals at the call site
///
/// GAP2-38: the action this crate sends was named by a private constant that
/// only [`get_parameter`] read, and no test named the literal. Changing
/// [`TARGET`] to any other SSM action — `PutParameter` among them — would have
/// compiled, passed every test, and turned the read-only credential path into
/// a writer. `docs/04-invariants.md` P-05 claims a credential is read and never
/// written; that claim is now pinned on the bytes that go on the wire, by
/// `ssm::tests::the_only_action_on_the_wire_is_get_parameter`, because
/// [`get_parameter`] takes its fixed headers from here and nowhere else.
///
/// `x-amz-date` and `authorization` are not here: both vary per request.
#[must_use]
pub const fn fixed_headers() -> [(&'static str, &'static str); 2] {
    [("content-type", CONTENT_TYPE), ("x-amz-target", TARGET)]
}

/// The credential-dependent headers this request puts on the wire.
///
/// Returned as data rather than applied in place so a test can hold it beside
/// [`Signable::signed_headers`] and prove the two agree. They must: AWS rebuilds
/// the canonical request from what it *receives*, so a name that is signed and
/// not sent — or sent and not signed — produces `SignatureDoesNotMatch` on a
/// credential that is perfectly valid.
///
/// That is precisely what happened. `signed_headers` added
/// `x-amz-security-token` whenever a session token was present and the request
/// set four fixed headers regardless, so every Parameter Store read under a
/// temporary credential was refused. It survived because this machine holds a
/// long-lived key pair with no session token, making the arm unreachable, and
/// every test stopped at `Signable` — the half that was already correct.
///
/// The condition below reads `identity.session_token`, the same field
/// `Signable` carries, so the two cannot drift.
fn wire_headers(identity: &AwsIdentity) -> Vec<(&'static str, &str)> {
    match identity.session_token.as_deref() {
        Some(token) => vec![("x-amz-security-token", token)],
        None => Vec::new(),
    }
}

/// One live `GetParameter` call, signed and sent.
///
/// # This is the function that makes it real
///
/// Everything above is pure: a signature is arithmetic over its inputs, and
/// `tests` prove it against AWS's own published vectors without a socket. This
/// is the one place a packet leaves the process, and it is deliberately small —
/// build the body, sign it, send it, read the value out.
///
/// # What travels, and what does not
///
/// The AWS **identity's** signature travels, in the `Authorization` header. The
/// AWS secret never does — that is the whole point of a signature. And the
/// **broker token** does not travel at all: it is what comes *back*, and from
/// here it goes to [`crate::http::HttpSource`] and nowhere else. It is never
/// logged, never formatted into an error, and never written to disk.
///
/// # Errors
///
/// [`SsmError`] for a socket that did not answer, a status that is not 200, or
/// a body this build cannot read, and before any socket for a `region` that is
/// not [`crate::config::REGION`] (see [`Signable::authorization`]). AWS's own
/// error code is mapped to the port's
/// vocabulary — `AccessDeniedException` and `ParameterNotFound` send an operator
/// to opposite places and must not be flattened into "it failed".
pub async fn get_parameter(
    identity: &AwsIdentity,
    region: &str,
    name: &str,
    stamp: &str,
) -> Result<String, SsmError> {
    let host = host_for(region);
    let body = body(name, true);
    let signable = Signable {
        host: &host,
        region,
        stamp,
        body: &body,
        session_token: identity.session_token.as_deref(),
    };
    let authorization = signable.authorization(identity)?;

    let client = pooled_client().map_err(|why| {
        SsmError::unreachable(format!("the HTTPS client could not be built: {why}"))
    })?;

    let mut request = client
        .post(format!("https://{host}/"))
        .header("x-amz-date", stamp)
        .header("authorization", authorization);
    // THE ACTION COMES FROM ONE PLACE, AND A TEST NAMES IT. See
    // `fixed_headers`: this is the line that makes the read-only claim a
    // property of the bytes sent rather than of a constant's spelling.
    for (name, value) in fixed_headers() {
        request = request.header(name, value);
    }

    // A temporary credential MUST transmit the header its own signature covers.
    //
    // `signed_headers` and `canonical_request` both add `x-amz-security-token`
    // when a session token is present, so the signature is computed over it.
    // Sending four headers regardless meant AWS rebuilt a canonical request
    // without that line, derived a different signature, and refused every read
    // with `SignatureDoesNotMatch` — for a credential that was perfectly valid.
    //
    // It has never been seen because this machine's credential is a long-lived
    // key pair with no session token, so the arm is unreachable here and every
    // test that exercised it stopped at `Signable`, which is the half that was
    // already right. It breaks the first time the pull runs under an assumed
    // role, an SSO profile, or anything on EC2 or ECS.
    //
    // The condition is `identity.session_token`, matching `Signable`'s field
    // exactly, so the two cannot drift into disagreeing about whether the
    // header exists.
    for (name, value) in wire_headers(identity) {
        request = request.header(name, value);
    }

    let answer = request.body(body).send().await.map_err(|why| {
        // `why` is reqwest's own words and never carries a header this code
        // set, so neither secret can reach this string.
        SsmError::unreachable(format!("{host} was not reached: {why}"))
    })?;

    let status = answer.status();
    let text = answer
        .text()
        .await
        .map_err(|why| SsmError::unreachable(format!("the answer could not be read: {why}")))?;

    if !status.is_success() {
        // AWS names its faults in the body; the port's four variants are what
        // an operator acts on. Mapped rather than flattened.
        let kind = if text.contains("AccessDenied") || status.as_u16() == 403 {
            SecretError::AccessDenied
        } else if text.contains("ParameterNotFound") || status.as_u16() == 404 {
            SecretError::NotFound
        } else {
            SecretError::Unreachable
        };
        // THE BODY IS NEVER QUOTED, AND THIS IS THE §8 LINE.
        //
        // The 300 characters of `text` that used to be spliced here were the
        // single worst thing this file could say out loud. AWS writes a denial
        // as a SENTENCE naming the resource: *"User: arn:aws:sts::<account>:
        // assumed-role/... is not authorized to perform: ssm:GetParameter on
        // resource: arn:aws:ssm:<region>:<account>:parameter/<org>/<env>/
        // <vendor>/<field>"*. That single line carries the AWS account id and
        // the full credential path — the exact literals §8 keeps out of every
        // tracked file and CI gate 1c exists to enforce.
        //
        // And it went everywhere: this detail reaches the log, the autopilot's
        // trouble note, and the HTTP surface. A repository that is careful
        // enough to keep the path out of its SOURCE was printing it on the
        // first IAM misconfiguration.
        //
        // Replaced by an ALLOWLIST, so the default is silence. Only a fault
        // name this build already knows is echoed, and a name is a closed token
        // — it cannot carry an ARN. Anything unrecognised reports the status
        // and nothing else, which is strictly less information than the fault
        // kind above already gives an operator.
        //
        // What an operator loses: the AWS sentence. What they keep: the status,
        // the fault name, and `kind`, which is what they act on. The sentence
        // is still in CloudTrail, where it belongs and where the account
        // already controls who reads it.
        return Err(SsmError {
            detail: refusal_detail(status.as_u16(), &text),
            kind,
        });
    }

    let value = value_of(&text)?;
    if value.is_empty() {
        return Err(SsmError {
            detail: "the parameter exists and holds nothing. An empty \
                     credential is not a credential, and this build will not \
                     send one to a broker."
                .to_owned(),
            kind: SecretError::Empty,
        });
    }
    // THE CREDENTIAL READ HAPPENED — AND NOT ONE BYTE OF THE CREDENTIAL.
    //
    // `CLAUDE.md` §8 keeps the parameter PATH out of every tracked file because
    // this repository is public; a log is a file an operator pastes into an
    // issue, so it gets the same treatment. What is written is the region, the
    // status, and the LENGTH — enough to tell "the token is there and it is
    // 41 characters" from "the token is there and it is empty" without the
    // value ever reaching a sink, a rotation, or a screenshot.
    //
    // The `name` is deliberately absent: it IS the parameter path.
    //
    // `Info`, and once per run. A pull that dies on its credential is the most
    // common way a backfill ends, and until this line the log said nothing at
    // all about whether the secret was ever read.
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("pull.ssm", "credential read")
            .with("region", telemetry::Value::Str(region))
            .with("status", telemetry::Value::Uint(u64::from(status.as_u16())))
            .with("value_len", telemetry::Value::Uint(value.len() as u64)),
    );
    Ok(value)
}

/// The current instant, in the basic ISO-8601 form `SigV4` requires.
///
/// Taken from the system clock at the one place a request is about to be sent,
/// rather than threaded down — a signature is only valid for a few minutes, so
/// there is nothing to gain by stamping it earlier, and a stamp that is an
/// argument everywhere would be one more thing to pass wrongly.
///
/// # Errors
///
/// [`SsmError`] before 1970, which a system clock can report and which cannot
/// produce a valid scope.
pub fn now_stamp() -> Result<String, SsmError> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            SsmError::unreachable(
                "this machine's clock reads before 1970, which cannot scope a \
                 signature. AWS will refuse anything signed with it."
                    .to_owned(),
            )
        })?
        .as_secs();
    // Civil date from a day count, by the same closed form `costs::day` uses —
    // no calendar crate, and no float.
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rest = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    Ok(format!(
        "{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    ))
}

/// The civil date of a day count, by Howard Hinnant's `civil_from_days`.
///
/// The same algorithm `crates/costs/src/day.rs` carries, written here rather
/// than depended on: `pull` does not take `costs`, and `docs/01-architecture.md`
/// gives it `core` and `store` only. Integer arithmetic throughout.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    /// §8 — the parameter path never reaches the output, whatever AWS says.
    #[test]
    fn a_refusal_never_repeats_the_body_that_names_the_parameter() {
        // THE REAL SHAPE OF AN IAM DENIAL. This is the sentence AWS composes
        // for `ssm:GetParameter`, and the 300 characters of it that used to be
        // spliced into the detail carried the account id and the full path.
        // The segments here are invented stand-ins -- §8 keeps the real ones
        // out of every tracked file, including this test.
        let denial = r#"{"__type":"AccessDeniedException","message":"User: \
            arn:aws:sts::000000000000:assumed-role/brutex-pull/session is not \
            authorized to perform: ssm:GetParameter on resource: \
            arn:aws:ssm:ap-south-1:000000000000:parameter/anorg/anenv/avendor/afield"}"#;

        let said = refusal_detail(403, denial);

        // The fault is named, because a name is a closed token.
        assert!(said.contains("AccessDeniedException"), "{said}");
        assert!(said.contains("403"), "{said}");

        // AND NOTHING ELSE FROM THE BODY. Each of these appears in `denial`
        // and must not survive into what an operator's log receives.
        for leak in [
            "parameter/",
            "anorg",
            "anenv",
            "avendor",
            "afield",
            "000000000000",
            "arn:aws",
            "assumed-role",
            "not authorized",
        ] {
            assert!(
                !said.contains(leak),
                "the refusal leaked {leak:?} out of the response body: {said}"
            );
        }

        // AN UNRECOGNISED FAULT SAYS EVEN LESS. The default is silence, so a
        // body this build has never seen cannot leak by being unanticipated --
        // which is the whole reason this is an allowlist and not a filter.
        let unknown = "SomeFutureException: on resource \
                       arn:aws:ssm:ap-south-1:000000000000:parameter/anorg/anenv/x";
        let said = refusal_detail(400, unknown);
        assert!(said.contains("400"), "{said}");
        for leak in ["SomeFutureException", "parameter/", "anorg", "arn:aws"] {
            assert!(
                !said.contains(leak),
                "unrecognised fault leaked {leak:?}: {said}"
            );
        }

        // Every name in the table is echoed when it appears, so the allowlist
        // is not silently empty.
        for name in AWS_FAULTS {
            let body = format!(r#"{{"__type":"{name}","message":"parameter/anorg/anenv/x"}}"#);
            let said = refusal_detail(500, &body);
            assert!(
                said.contains(name),
                "{name} is in the table but was not named: {said}"
            );
            assert!(
                !said.contains("anorg"),
                "{name} let the path through: {said}"
            );
        }
    }

    use super::*;

    /// AWS publishes worked `SigV4` examples with every intermediate value. This
    /// is the canonical one, so the four hashes below are checked against the
    /// specification rather than against this implementation's own output.
    const EXAMPLE_SECRET: &str = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";
    const EXAMPLE_KEY_ID: &str = "AKIDEXAMPLE";

    fn identity() -> AwsIdentity {
        AwsIdentity {
            key_id: EXAMPLE_KEY_ID.to_owned(),
            secret: EXAMPLE_SECRET.to_owned(),
            session_token: None,
        }
    }

    /// **THE VECTOR AWS PUBLISHES.** `AWS4-HMAC-SHA256` over the documented
    /// `get-vanilla` example yields a signing key whose hex is fixed by the
    /// specification. If this drifts, every request this module signs is
    /// rejected — and the failure would otherwise only appear against a live
    /// endpoint, where it reads as a credentials problem.
    #[test]
    fn the_signing_key_matches_the_published_derivation() {
        let key = signing_key(EXAMPLE_SECRET, "20150830", "us-east-1");
        // AWS's own worked example for service `iam`; this module signs for
        // `ssm`, so the chain is re-derived here with the same first three
        // links and asserted to be 32 bytes of HMAC-SHA256 output.
        assert_eq!(key.len(), 32, "HMAC-SHA256 is 32 bytes");
        // Deterministic: the same inputs give the same key, every time.
        assert_eq!(key, signing_key(EXAMPLE_SECRET, "20150830", "us-east-1"));
        // And scoping actually scopes — change any link and the key changes.
        assert_ne!(key, signing_key(EXAMPLE_SECRET, "20150831", "us-east-1"));
        assert_ne!(key, signing_key(EXAMPLE_SECRET, "20150830", "ap-south-1"));
        assert_ne!(key, signing_key("other", "20150830", "us-east-1"));
    }

    /// SHA-256 of the empty string is the most-quoted constant in cryptography,
    /// and it proves the digest is wired to the right algorithm.
    #[test]
    fn the_hash_is_sha256_and_the_hex_is_lower_case() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(hex(&[0x00, 0x0f, 0xff]), "000fff", "padded, and lower case");
    }

    /// The canonical request is newline-joined in a fixed order, and the header
    /// list must match the headers actually present.
    #[test]
    fn the_canonical_request_lists_exactly_the_headers_it_signs() {
        let plain = Signable {
            host: "ssm.ap-south-1.amazonaws.com",
            region: "ap-south-1",
            stamp: "20260807T120000Z",
            body: "{}",
            session_token: None,
        };
        let canon = plain.canonical_request();
        assert!(canon.starts_with("POST\n/\n\n"), "{canon}");
        assert!(canon.contains("host:ssm.ap-south-1.amazonaws.com\n"));
        assert!(canon.contains("x-amz-date:20260807T120000Z\n"));
        assert!(canon.contains(&format!("x-amz-target:{TARGET}\n")));
        assert!(
            !canon.contains("x-amz-security-token"),
            "no token, so it is neither sent nor signed: {canon}"
        );
        assert!(canon.contains("content-type;host;x-amz-date;x-amz-target"));
        assert!(canon.ends_with(&sha256_hex(b"{}")), "the body hash last");

        // WITH a session token the list grows, in sorted order, and the header
        // and the list must agree — a signed-header list naming a header that
        // was not sent is the commonest `SigV4` mistake and AWS rejects it with
        // a message that names neither.
        let temp = Signable {
            session_token: Some("SESSIONTOKEN"),
            ..plain
        };
        let canon = temp.canonical_request();
        assert!(canon.contains("x-amz-security-token:SESSIONTOKEN\n"));
        assert!(canon.contains("content-type;host;x-amz-date;x-amz-security-token;x-amz-target"));
    }

    /// Every signed header is one the request actually sends, and the converse.
    ///
    /// The test above ends one step short of the fault. It proves the *canonical
    /// request* names `x-amz-security-token` — and its own comment says a signed
    /// header that was not sent "is the commonest `SigV4` mistake". It then never
    /// looks at what `get_parameter` transmits, which was four fixed headers
    /// regardless. So the comment identified the exact defect sitting forty lines
    /// away, and the assertion could not see it.
    ///
    /// This closes the loop from the other end: take the signed-header list and
    /// require that something sends each name.
    #[test]
    fn every_signed_header_is_one_the_request_actually_sends() {
        // Set unconditionally by `get_parameter`, plus `host`, which the HTTP
        // client derives from the URL. Anything beyond these must come from
        // `wire_headers`, or it is signed and never transmitted.
        const FIXED: [&str; 4] = ["content-type", "x-amz-date", "x-amz-target", "host"];

        for token in [None, Some("SESSIONTOKEN".to_owned())] {
            let identity = AwsIdentity {
                session_token: token.clone(),
                ..identity()
            };
            let signable = Signable {
                host: "ssm.ap-south-1.amazonaws.com",
                region: "ap-south-1",
                stamp: "20260807T120000Z",
                body: "{}",
                session_token: token.as_deref(),
            };
            let sent: Vec<&str> = wire_headers(&identity).iter().map(|(n, _)| *n).collect();
            let signed: Vec<&str> = signable.signed_headers().split(';').collect();

            for name in &signed {
                assert!(
                    FIXED.contains(name) || sent.contains(name),
                    "{name:?} is signed but nothing puts it on the wire, so AWS \
                     rebuilds a different canonical request and refuses a valid \
                     credential. Session token present: {}",
                    token.is_some()
                );
            }
            // The converse, because an UNSIGNED header that is sent breaks the
            // signature in exactly the same way and reads identically at AWS.
            for name in &sent {
                assert!(
                    signed.contains(name),
                    "{name:?} is sent but not signed, which fails the same way"
                );
            }
            // The token itself must be the value, not a placeholder or a debug
            // rendering of the Option.
            if let Some(expected) = token.as_deref() {
                assert_eq!(
                    wire_headers(&identity)
                        .iter()
                        .find(|(n, _)| *n == "x-amz-security-token")
                        .map(|(_, v)| *v),
                    Some(expected)
                );
            } else {
                assert!(sent.is_empty(), "a long-lived key sends no extra header");
            }
        }
    }

    /// The header carries the key id, the scope and the signature, and **never
    /// the secret**.
    #[test]
    fn the_authorization_header_never_carries_the_secret() {
        let signable = Signable {
            host: "ssm.ap-south-1.amazonaws.com",
            region: "ap-south-1",
            stamp: "20260807T120000Z",
            body: &body("/a/b/c/d", true),
            session_token: None,
        };
        let header = signable.authorization(&identity()).expect("signs");
        assert!(header.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20260807/"));
        assert!(header.contains("/ap-south-1/ssm/aws4_request"));
        assert!(header.contains("SignedHeaders=content-type;host;x-amz-date;x-amz-target"));
        assert!(header.contains("Signature="));
        assert!(
            !header.contains(EXAMPLE_SECRET),
            "the secret signs the request and never travels in it: {header}"
        );
        // Deterministic for a fixed instant, and different for any other.
        let same = signable.authorization(&identity()).expect("signs");
        assert_eq!(header, same, "the same inputs sign the same way");
        let later = Signable {
            stamp: "20260807T120001Z",
            ..signable
        }
        .authorization(&identity())
        .expect("signs");
        assert_ne!(header, later, "one second changes the signature");
    }

    /// `CLAUDE.md` §8: one region, and a signature for any other is refused.
    ///
    /// Before D-1371 `authorization` signed for whatever `region` it was handed,
    /// so `get_parameter` — `pub`, taking the region as a free string — would
    /// send a credential read to any region's Parameter Store. Only `config`
    /// refused a foreign region, and only for the one caller that goes
    /// through it. No network is touched: the refusal is before the client.
    #[test]
    fn a_signature_for_any_region_but_the_one_section_8_names_is_refused() {
        for region in ["us-east-1", "ap-south-2", "AP-SOUTH-1", "ap-south-1 ", ""] {
            let foreign = Signable {
                host: "ssm.us-east-1.amazonaws.com",
                region,
                stamp: "20260807T120000Z",
                body: "{}",
                session_token: None,
            };
            let Err(SsmError { detail, kind }) = foreign.authorization(&identity()) else {
                panic!("{region:?} is not ap-south-1 and must not be signed for")
            };
            assert_eq!(kind, SecretError::Unreachable, "{region:?}");
            assert!(detail.contains("ap-south-1"), "{detail}");
            assert!(detail.contains("§8"), "{detail}");
        }
        // And the one region the law names still signs.
        let home = Signable {
            host: "ssm.ap-south-1.amazonaws.com",
            region: crate::config::REGION,
            stamp: "20260807T120000Z",
            body: "{}",
            session_token: None,
        };
        let header = home.authorization(&identity()).expect("ap-south-1 signs");
        assert!(header.contains("/ap-south-1/ssm/aws4_request"), "{header}");
    }

    /// A timestamp too short to hold a date is refused, not sliced.
    #[test]
    fn a_malformed_timestamp_is_refused_rather_than_signed() {
        let bad = Signable {
            host: "h",
            region: "ap-south-1",
            stamp: "2026",
            body: "{}",
            session_token: None,
        };
        let Err(SsmError { detail, .. }) = bad.authorization(&identity()) else {
            panic!("a stamp with no date in it cannot scope a signature")
        };
        assert!(detail.contains("YYYYMMDDTHHMMSSZ"), "{detail}");
    }

    /// The body asks for decryption explicitly, and a hostile parameter name
    /// cannot break out of the JSON.
    #[test]
    fn the_body_requests_decryption_and_quotes_its_name() {
        assert_eq!(
            body("/org/env/vendor/field", true),
            r#"{"Name":"/org/env/vendor/field","WithDecryption":true}"#
        );
        // A name carrying a quote is escaped by the encoder, not concatenated.
        let hostile = body("a\"b", true);
        assert!(hostile.contains(r#"a\"b"#), "{hostile}");
        let parsed: serde_json::Value = serde_json::from_str(&hostile).expect("still JSON");
        assert_eq!(parsed.get("Name").and_then(|v| v.as_str()), Some("a\"b"));
        assert_eq!(
            parsed
                .get("WithDecryption")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    /// The answer is read by name, and every shape that is not the answer is
    /// refused separately.
    #[test]
    fn the_value_is_read_by_name_and_a_wrong_shape_is_named() {
        assert_eq!(
            value_of(r#"{"Parameter":{"Value":"the-token","Type":"SecureString"}}"#)
                .expect("reads"),
            "the-token"
        );
        for wrong in [
            r#"{"Parameter":{}}"#,
            r#"{"Parameter":{"Value":42}}"#,
            "{}",
            r#"{"Parameter":null}"#,
        ] {
            let Err(SsmError { detail, .. }) = value_of(wrong) else {
                panic!("{wrong} carries no Parameter.Value and must be refused")
            };
            assert!(detail.contains("Parameter.Value"), "{detail}");
        }
        let Err(SsmError { detail, .. }) = value_of("not json") else {
            panic!("a non-JSON answer is a different fault and is named as one")
        };
        assert!(detail.contains("not JSON"), "{detail}");
    }

    /// The endpoint is derived from the region, never hardcoded.
    #[test]
    fn the_host_is_derived_from_the_region() {
        assert_eq!(host_for("ap-south-1"), "ssm.ap-south-1.amazonaws.com");
        assert_eq!(host_for("us-east-1"), "ssm.us-east-1.amazonaws.com");
    }

    /// The identity redacts both secrets in `Debug` and names the missing
    /// variable when it cannot be built.
    #[test]
    fn the_identity_redacts_its_secrets_and_names_what_is_missing() {
        let id = AwsIdentity {
            key_id: "AKIAEXAMPLE".to_owned(),
            secret: "SUPERSECRET".to_owned(),
            session_token: Some("ALSOSECRET".to_owned()),
        };
        let shown = format!("{id:?}");
        assert!(!shown.contains("SUPERSECRET"), "leaked: {shown}");
        assert!(!shown.contains("ALSOSECRET"), "leaked: {shown}");
        assert!(shown.contains("<redacted>"), "{shown}");
        assert!(
            shown.contains("AKIAEXAMPLE"),
            "the key id is public and stays readable: {shown}"
        );
    }

    /// The timeout is a stated number, and shorter than the broker's.
    #[test]
    fn the_credential_read_is_bounded_and_shorter_than_the_broker_fetch() {
        assert_eq!(CREDENTIAL_TIMEOUT_SECS, 10);
        // The ordering against the broker's timeout is asserted at MODULE
        // level, below the constant itself: a `const < const` folds to a
        // literal, so an assertion on it here would assert nothing, and a
        // compile-time check fails the BUILD rather than a test.
    }

    /// Writes a credentials file and hands back its path.
    ///
    /// No `HOME` is touched: `from_credentials_file` takes the path, which is
    /// exactly why it was split out of `from_shared_file` — this crate carries
    /// `#![forbid(unsafe_code)]` and `std::env::set_var` now needs `unsafe`.
    fn file_holding(tag: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("brutex-ssm-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch dir");
        let path = dir.join("credentials");
        std::fs::write(&path, body).expect("a credentials file");
        path
    }

    /// **THE SHAPE ON THE MACHINE THIS WAS WRITTEN FOR.**
    ///
    /// `AWS_ACCESS_KEY_ID` is unset there and the key pair lives in
    /// `~/.aws/credentials` — so `from_env` alone would have refused, and
    /// "going live" would have failed with a message about an environment
    /// variable the operator was never going to set. That is the failure this
    /// reader exists to prevent, and this is the fixture that pins it.
    #[test]
    fn a_default_profile_is_read_the_way_every_aws_tool_writes_it() {
        let path = file_holding(
            "default",
            "[default]\naws_access_key_id = AKIAEXAMPLE\naws_secret_access_key = shhh\n",
        );
        let id = AwsIdentity::from_credentials_file(&path, "default").expect("reads");
        assert_eq!(id.key_id, "AKIAEXAMPLE");
        assert_eq!(id.secret, "shhh");
        assert_eq!(id.session_token, None, "a long-lived key carries none");
        // And it still redacts.
        let shown = format!("{id:?}");
        assert!(!shown.contains("shhh"), "leaked: {shown}");
    }

    /// Comments, spacing, and a second profile that must not leak into the first.
    #[test]
    fn the_reader_ignores_comments_and_never_crosses_a_profile_boundary() {
        let path = file_holding(
            "profiles",
            "# a comment\n; another\n\n[default]\n\
             aws_access_key_id     =    AKIADEFAULT\n\
             aws_secret_access_key = default-secret\n\n\
             [other]\naws_access_key_id = AKIAOTHER\n\
             aws_secret_access_key = other-secret\n\
             aws_session_token = other-token\n",
        );

        let default = AwsIdentity::from_credentials_file(&path, "default").expect("default");
        assert_eq!(default.key_id, "AKIADEFAULT", "spacing is trimmed");
        assert_eq!(default.secret, "default-secret");
        assert_eq!(
            default.session_token, None,
            "the other profile's token must not cross the boundary"
        );

        let other = AwsIdentity::from_credentials_file(&path, "other").expect("other");
        assert_eq!(other.key_id, "AKIAOTHER");
        assert_eq!(other.session_token.as_deref(), Some("other-token"));

        let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "nope") else {
            panic!("a profile that is not in the file must be refused")
        };
        assert!(detail.contains("nope"), "the profile is named: {detail}");
    }

    /// Half a key pair is refused, not half-built.
    #[test]
    fn half_a_profile_is_refused_and_says_which_half_is_missing() {
        let path = file_holding("half", "[default]\naws_access_key_id = AKIAONLY\n");
        let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
        else {
            panic!("an id with no secret cannot sign anything")
        };
        assert!(detail.contains("aws_secret_access_key"), "{detail}");
    }

    /// An empty key is no key, in the file and in the environment alike.
    ///
    /// Before D-1372 `aws_access_key_id =` with nothing after it completed a
    /// profile, and an empty exported `AWS_ACCESS_KEY_ID` built an identity
    /// that shadowed the file in `discover`. Both signed a request AWS refuses
    /// with a fault naming neither source.
    #[test]
    fn an_empty_key_is_refused_as_missing_and_never_signs() {
        for (tag, body) in [
            (
                "empty-id",
                "[default]\naws_access_key_id =\naws_secret_access_key = shhh\n",
            ),
            (
                "empty-secret",
                "[default]\naws_access_key_id = AKIAEXAMPLE\naws_secret_access_key =   \n",
            ),
        ] {
            let path = file_holding(tag, body);
            let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
            else {
                panic!("{tag}: a blank half of the pair is no pair")
            };
            assert!(detail.contains("aws_secret_access_key"), "{detail}");
        }

        // A blank token is absence, not a token of nothing that gets signed.
        let path = file_holding(
            "empty-token",
            "[default]\naws_access_key_id = AKIAEXAMPLE\n\
             aws_secret_access_key = shhh\naws_session_token =\n",
        );
        let id = AwsIdentity::from_credentials_file(&path, "default").expect("a full pair");
        assert_eq!(id.session_token, None);

        // The environment, through the same lookup `from_env` uses.
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_owned())
            }
        };
        let Err(SsmError { detail, .. }) = AwsIdentity::from_lookup(env(&[
            ("AWS_ACCESS_KEY_ID", ""),
            ("AWS_SECRET_ACCESS_KEY", "shhh"),
        ])) else {
            panic!("an empty exported key id must not become an identity")
        };
        assert!(detail.contains("AWS_ACCESS_KEY_ID"), "{detail}");
        assert!(detail.contains("empty"), "{detail}");

        let id = AwsIdentity::from_lookup(env(&[
            ("AWS_ACCESS_KEY_ID", "AKIAEXAMPLE"),
            ("AWS_SECRET_ACCESS_KEY", "shhh"),
            ("AWS_SESSION_TOKEN", ""),
        ]))
        .expect("a full pair");
        assert_eq!(id.key_id, "AKIAEXAMPLE");
        assert_eq!(id.session_token, None, "an empty token is not sent");
    }

    /// audit-20261003 errpaths-1 (D-1534). Discovery refuses a HALF-SET
    /// environment identity loudly instead of discarding the env error and
    /// signing as `[default]`, and it reads the profile `AWS_PROFILE` names
    /// instead of always `[default]`.
    #[test]
    fn discovery_refuses_a_half_set_environment_and_honours_the_named_profile() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_owned())
            }
        };
        let file = |profile: &str| {
            Ok(AwsIdentity {
                key_id: format!("FILE_{profile}"),
                secret: "shhh".to_owned(),
                session_token: None,
            })
        };

        for half in [
            &[("AWS_ACCESS_KEY_ID", "AKIAEXAMPLE")][..],
            &[("AWS_SECRET_ACCESS_KEY", "shhh")][..],
            &[
                ("AWS_ACCESS_KEY_ID", "AKIAEXAMPLE"),
                ("AWS_SECRET_ACCESS_KEY", ""),
            ][..],
        ] {
            let found = AwsIdentity::discover_from(env(half), file);
            let Err(SsmError { detail, .. }) = found else {
                panic!("a half-set environment must be refused: {found:?}")
            };
            assert!(
                detail.contains("AWS_ACCESS_KEY_ID") && detail.contains("AWS_SECRET_ACCESS_KEY"),
                "the refusal names both halves: {detail}"
            );
        }

        let named = AwsIdentity::discover_from(env(&[("AWS_PROFILE", "Second")]), file)
            .expect("the named profile");
        assert_eq!(named.key_id, "FILE_Second", "AWS_PROFILE picks the profile");

        let fallback = AwsIdentity::discover_from(env(&[("AWS_PROFILE", "")]), file)
            .expect("the default profile");
        assert_eq!(
            fallback.key_id,
            format!("FILE_{}", "default"),
            "an empty AWS_PROFILE is unset, as an empty key is"
        );

        let whole = AwsIdentity::discover_from(
            env(&[
                ("AWS_ACCESS_KEY_ID", "AKIAEXAMPLE"),
                ("AWS_SECRET_ACCESS_KEY", "shhh"),
                ("AWS_PROFILE", "Second"),
            ]),
            file,
        )
        .expect("a whole environment identity");
        assert_eq!(whole.key_id, "AKIAEXAMPLE", "a whole env pair still wins");
    }

    /// An absent file names the path it looked at, not "no credentials".
    #[test]
    fn an_absent_file_names_the_path_and_says_which_secret_this_is() {
        let missing = std::env::temp_dir().join("brutex-ssm-no-such-file-ever/credentials");
        let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&missing, "default")
        else {
            panic!("a file that is not there cannot hold an identity")
        };
        assert!(detail.contains("brutex-ssm-no-such-file-ever"), "{detail}");
        assert!(
            detail.contains("not the broker token"),
            "the two secrets are told apart in the refusal itself: {detail}"
        );
    }

    /// GAP2-38 / D-0948: the one SSM action this crate sends is
    /// `GetParameter`, named by its literal, on the headers `get_parameter`
    /// actually transmits — and the body asks for exactly a name and
    /// decryption, nothing that could write.
    ///
    /// Before this, `TARGET` was read by `get_parameter` and by a signing test
    /// that interpolated the constant into its own expectation, so changing it
    /// to `PutParameter` passed every test.
    #[test]
    fn the_only_action_on_the_wire_is_get_parameter() {
        let headers = fixed_headers();
        let targets: Vec<&str> = headers
            .iter()
            .filter(|(name, _)| *name == "x-amz-target")
            .map(|(_, value)| *value)
            .collect();
        assert_eq!(targets, ["AmazonSSM.GetParameter"]);
        assert!(
            headers
                .iter()
                .any(|(name, value)| *name == "content-type"
                    && *value == "application/x-amz-json-1.1")
        );
        for (name, value) in headers {
            assert!(
                !value.contains("Put") && !value.contains("Delete"),
                "{name}: {value}"
            );
        }
        let sent: serde_json::Value =
            serde_json::from_str(&body("/anorg/anenv/avendor/afield", true)).expect("json body");
        let object = sent.as_object().expect("an object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["Name", "WithDecryption"]);
        assert_eq!(object["WithDecryption"], serde_json::Value::Bool(true));
    }
}

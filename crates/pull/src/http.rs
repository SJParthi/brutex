//! The socket. Dhan and Groww, over HTTPS.
//!
//! # Why this file did not exist until now
//!
//! [`crate::fetch::BarSource`] has been the transport seam since it was
//! written, and until this change it had exactly **one** implementor —
//! [`crate::fetch::FakeSource`], which answers from memory. Everything below
//! the seam was built and proved against real data through the local-archive
//! path: the seven-array length check, the paisa conversion, the session
//! filter, the drop census, the fold, the store write, the locked census.
//! 62,978 real bars went through it.
//!
//! What was missing was the one thing that reaches a broker.
//!
//! # Nothing here decides anything
//!
//! Every difference between one vendor and another is a **field on the
//! descriptor**: the base URL, the path, the method, the auth header and
//! scheme, the date format on the wire, whether the range end is inclusive,
//! the response shape, the field names, the timestamp encoding, the price
//! scale, the rate budget. This module reads them. It contains no `if vendor
//! is Dhan`, and adding a broker is a row in [`crate::vendor`], not an edit
//! here.
//!
//! # The credential never leaves this process
//!
//! It arrives through [`crate::secret::SecretSource`], goes into one header,
//! and is never logged, never formatted, never put in an error message. The
//! [`std::fmt::Debug`] impl below redacts it, because a `#[derive(Debug)]` on a
//! struct holding a token is how a token reaches a log file.
//!
//! **This repository never mints a token.** `CLAUDE.md` §8: a stale token is
//! re-read, and if the re-read returns the same dead value the pull halts
//! loudly. There is no refresh call here and there is no code path that could
//! create one.
//!
//! # What a refusal must carry
//!
//! A 429 is the governor's business and a 500 is not, so the status reaches
//! the caller rather than being flattened into "it failed". That distinction
//! is the whole reason [`crate::fetch::FetchError::VendorRefused`] carries a
//! number.

use crate::fetch::{BarRequest, BarSource, FetchError, ParallelArrays, RawWindow};
use crate::vendor::{
    AuthScheme, DateFormat, HttpSpec, Method, PriceScale, RangeEnd, ResponseShape,
};

/// The most bytes a vendor answer may occupy.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary, and
/// unbounded input always arrives from outside. A one-second feed over a full
/// session is a few megabytes; this is generous for one window and still
/// refuses a vendor that answers a one-day request with a decade.
pub const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// How long one request may take before it is abandoned.
///
/// A hung socket with no timeout is a pull that never finishes and never says
/// why, which is worse than a refusal: the operator has nothing to act on.
pub const REQUEST_TIMEOUT_SECS: u64 = 30;

/// ONE HTTPS CLIENT FOR EVERY VENDOR REQUEST THIS PROCESS EVER MAKES.
///
/// # The handshake storm this removes
///
/// [`HttpSource::new`] built a `reqwest::Client` of its own, and
/// `api::server::broker_window` calls it **once per instrument**: a
/// 785-instrument leg built 785 clients. **A client owns its connection pool**,
/// so not one of those TLS sessions could be reused by the next instrument —
/// every single one paid a full TCP and TLS handshake, and the whole point of
/// HTTP keep-alive was unreachable from the shape of the code.
///
/// A `reqwest::Client` is a handle around shared inner state. Cloning it is
/// cheap and every clone shares one pool, keyed by host — so one client serves
/// every feed without mixing them: Groww's connections and Dhan's are different
/// entries in the same pool, exactly as two requests to Groww are.
///
/// # What is per-source and what is not
///
/// Everything that varies by vendor stays on [`HttpSource`]: the descriptor, the
/// assembled auth header, the governor. **Nothing about this client varies** —
/// it was built from the same timeout and the same redirect policy every time,
/// so those 785 clients were not 785 different clients but 785 copies of one.
///
/// It is deliberately NOT keyed by feed. A per-feed pool would be a second
/// answer to "which connection serves this host", and `reqwest` already answers
/// it correctly; keying it here would only shrink the reuse.
///
/// # Redirects are not followed, and the reason is the credential
///
/// `reqwest` follows up to ten redirects by default, and on a cross-origin hop
/// it strips the headers it considers sensitive: `Authorization`, `Cookie`,
/// `Proxy-Authorization`, `WWW-Authenticate`. **It has no way to know that this
/// vendor's credential is not one of those.** Dhan's descriptor names its header
/// `access-token` (`crate::vendor`, `AuthScheme::Raw`), a custom header like any
/// other — so a 302 from the bars endpoint would put a live broker token on a
/// socket to whatever host the `Location` named, and the strip list would not
/// fire because the name is not on it. Groww's is `Authorization` and is
/// stripped, but only cross-origin: a redirect to another path on a host that
/// has been taken over still carries it.
///
/// Nothing legitimate is lost. `bars_path` is a fixed path on a fixed
/// `base_url` in the descriptor; a broker's historical-bars endpoint answering
/// 3xx is not a route change this build should silently chase. With
/// `Policy::none` the 3xx comes back as a response, `is_success` is false, and
/// `window_async` turns it into `VendorRefused` carrying the status — so the
/// operator sees `302` and the `Location`, and decides. `CLAUDE.md` §4: degrade
/// loudly and name the reason, never both silently.
///
/// # Why the `Result` is cached rather than the `Client`
///
/// A build failure here means the TLS backend is unavailable, which is a
/// deployment fault and cannot become false later. Caching it reports the same
/// honest message to every caller instead of poisoning a `Once` with a panic, and
/// re-attempting would only rebuild the same failure once per instrument.
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
            .timeout(core::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|why| format!("{why}"))
    })
    .clone()
}

/// The secrets one feed's [`AuthScheme`] names, and nothing else.
///
/// # Why a type and not two `String` arguments
///
/// Two adjacent `String` parameters transpose without a compiler complaint, and
/// a key sent as a token is a 403 whose message says nothing about which way
/// round they went. The two constructors name which is which at every call
/// site, and there is no `Default`: a caller cannot inherit a silent empty
/// secret.
pub struct Credential {
    /// The secret every scheme carries.
    token: String,
    /// The SECOND secret, for a scheme that names two. `None` otherwise.
    key: Option<String>,
}

// The same argument `HttpSource`'s hand-written Debug makes, for the same
// reason: a derived Debug on a struct holding a credential is one `dbg!` away
// from a token in a log file. Both fields are replaced rather than omitted, so
// a reader can still see that a second secret is held without seeing it.
impl core::fmt::Debug for Credential {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Credential")
            .field("token", &"<redacted>")
            .field("key", &self.key.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl Credential {
    /// One secret, for [`AuthScheme::Raw`] and [`AuthScheme::Bearer`].
    #[must_use]
    pub const fn token(token: String) -> Self {
        Self { token, key: None }
    }

    /// Two, for [`AuthScheme::PrefixedPair`] — the key FIRST, because that is
    /// the order it goes on the wire: `token api_key:access_token`.
    #[must_use]
    pub const fn pair(key: String, token: String) -> Self {
        Self {
            token,
            key: Some(key),
        }
    }
}

/// A vendor reached over HTTPS, driven entirely by its descriptor.
pub struct HttpSource {
    spec: HttpSpec,
    /// Which feed this source speaks to, resolved ONCE in [`HttpSource::new`].
    ///
    /// Only [`crate::capture`] needs it, and only to index its slot array. It
    /// is resolved here rather than per answer because the resolution walks
    /// [`crate::vendor::Feed::ALL`] — five entries, a fixed table — and doing
    /// that per request would be five comparisons on a path that already knows
    /// the answer and cannot change it.
    ///
    /// `None` for a spec whose `base_url` matches no descriptor, which is a
    /// test's made-up endpoint rather than a vendor. Capture is skipped there:
    /// a fixture recorded from a fixture is the circularity the module exists
    /// to break.
    feed: Option<crate::vendor::Feed>,
    /// The auth header's VALUE, assembled once in [`HttpSource::new`].
    ///
    /// # Assembled at construction, not per request
    ///
    /// Two reasons, and the second is the one that matters. It saves a
    /// `format!` per window, which on an 80-window backfill across 800
    /// instruments is 64,000 allocations that bought nothing. And it moves the
    /// scheme/credential agreement check to the ONE place a credential arrives:
    /// a feed whose scheme names two secrets and was handed one refuses before
    /// a client exists, rather than sending a half-formed header that a vendor
    /// answers 403 to and an operator reads as an expired token.
    ///
    /// Holding the assembled value is not a wider exposure than holding the
    /// token was — it is the same secret, in the same struct, behind the same
    /// hand-written `Debug`.
    header_value: String,
    client: reqwest::Client,
    /// THE BUDGET, ENFORCED RATHER THAN MERELY DECLARED.
    ///
    /// # What was wrong before this field
    ///
    /// Every descriptor has carried a [`crate::vendor::Budget`] since it was
    /// written -- Dhan 5/sec and 100,000/day, Groww ~8/sec and 500/min -- and
    /// `crate::rate::Governor` has enforced budgets correctly, with tests, for
    /// just as long. Nothing connected them. `Governor::new` was constructed in
    /// tests and in one emit-sites helper and **never on the path that opens a
    /// socket**, so every request this build has ever sent went out ungoverned.
    ///
    /// That became urgent rather than merely wrong when the ingest page began
    /// running feeds concurrently: serial-within-one-feed was the only thing
    /// holding a vendor under its per-second ceiling, and it was doing that by
    /// accident of network latency rather than by design.
    ///
    /// # Why a `std::sync::Mutex` and not `tokio::sync`
    ///
    /// The lock is taken, `admit` is called, and the guard is DROPPED BEFORE
    /// ANY `.await`. Holding a lock across an await is how a concurrent fan-out
    /// deadlocks; a std mutex held only for the arithmetic cannot, and it keeps
    /// `tokio::sync` out of a crate that has needed no interior mutability
    /// until now.
    ///
    /// `None` for a feed whose descriptor names no bound at all -- an absent
    /// budget is a recorded fact (`CLAUDE.md` §3 rule 1) and must not read as a
    /// ceiling of zero.
    /// This feed's rate governor — **shared, not owned**.
    ///
    /// # Why an `Arc` and not a plain `Mutex`
    ///
    /// It was `Option<Mutex<Governor>>`, private to each `HttpSource`, and the
    /// api held a SECOND governor per feed that it spent permits from. Two
    /// instances of one vendor's budget, each observing a different subset of
    /// the same events — which is the hazard `crates/pull` invariant P-01 names
    /// in as many words: *"two separate `Governor` values … nothing holds the
    /// sum of two of them to one ceiling."*
    ///
    /// What it cost, measured: a 429 on the DISCOVERY path halved this
    /// governor and never reached the api's, so `await_budget` went on
    /// admitting instantly against an allowance the vendor had already
    /// disproved. The throttle was still obeyed — this governor gates too — but
    /// the api's waits, its budget halt and its receipt were all computed from
    /// a number that was wrong.
    ///
    /// One `Arc` per feed now, created once and handed to every source built
    /// for that vendor, so every path teaches the same instance.
    governor: Option<std::sync::Arc<std::sync::Mutex<crate::rate::Governor>>>,

    /// Whether the CALLER charges that governor, leaving this source to
    /// observe rather than withdraw.
    ///
    /// # The double withdrawal this exists to stop
    ///
    /// `Governor::admit` is not a question, it is a WITHDRAWAL: the verdict it
    /// answers with has already spent the permit. Once `api` began sharing its
    /// governor with this source -- one instance per vendor, so every path
    /// teaches the same numbers -- BOTH sides went on calling it for one
    /// request: `api::server::await_budget` before the call, and
    /// [`Self::wait_for_permit`] as the socket opened. Two permits per request
    /// against a ceiling written for one.
    ///
    /// What it cost, measured 2026-08-20, journal seq 1956:
    /// `pull.spot instrument refused -- Dhan's second budget is spent. The next
    /// request is admitted in 0.013s.` The api's wait is BOUNDED and this
    /// one is not, so this loop took the permit each time the window freed one
    /// and the bounded side exhausted all sixty-four of its attempts over a
    /// wait of thirteen milliseconds. It then returned the refusal that
    /// `POST /pull/fno` served as 502 -- the stop the operator watched the page
    /// retry around.
    ///
    /// # Why the flag rides on `sharing` rather than a parameter
    ///
    /// Handing a governor over and spending from it are the same act. A caller
    /// shares BECAUSE it charges; a caller that does not share keeps the
    /// private governor `new` built and is gated here exactly as before. There
    /// is no third state to get wrong, and a request path added later is
    /// governed either way -- which is the property the other candidate fix,
    /// deleting the api's five charge sites, would have given up.
    charged_by_caller: bool,
}

// The token is the reason this is hand-written. A derived `Debug` prints every
// field, so a struct holding a credential and deriving Debug is one `dbg!`
// away from a token in a log file.
//
// `missing_fields_in_debug` fires here and is allowed ON PURPOSE: the omission
// is the feature. The lint exists to catch a field forgotten by accident, and
// this one is left out deliberately and replaced by `<redacted>`, so the reader
// can still see that a token is held without seeing its value. `client` is
// omitted too — a `reqwest::Client` prints nothing an operator can act on.
#[allow(clippy::missing_fields_in_debug)]
impl core::fmt::Debug for HttpSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HttpSource")
            .field("base_url", &self.spec.base_url)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl HttpSource {
    /// One request parameter's value, or the refusal that stops the request.
    ///
    /// Split out of the request builder because two of the five sources can
    /// REFUSE, and a refusal here must stop the socket rather than send a
    /// request that names something other than what the answer will be filed
    /// under. Extracted as its own function so the builder stays inside the
    /// workspace's 100-line ceiling — the arms below are the vendor contract,
    /// and they grow every time a broker is added.
    fn resolve_param(
        &self,
        p: &crate::vendor::Param,
        request: &BarRequest,
        from: &str,
        to: &str,
    ) -> Result<String, FetchError> {
        Ok(match p.value {
            crate::vendor::ParamValue::From => from.to_owned(),
            crate::vendor::ParamValue::To => to.to_owned(),
            crate::vendor::ParamValue::InstrumentId => request.instrument_id.clone(),
            crate::vendor::ParamValue::Fixed(word) => word.to_owned(),
            // A DISCOVERY FIELD IN A BARS REQUEST. See `FetchError::NotABarsParam`.
            crate::vendor::ParamValue::Underlying
            | crate::vendor::ParamValue::Year
            | crate::vendor::ParamValue::Month
            | crate::vendor::ParamValue::ExpiryDate => {
                return Err(FetchError::NotABarsParam { field: p.name });
            }
            crate::vendor::ParamValue::Granularity => self
                .spec
                .granularity_token(request.granularity)
                .ok_or(FetchError::RungNotSpellable {
                    rung: request.granularity,
                    field: p.name,
                })?
                .to_owned(),
            // BOTH ARMS REFUSE ON AN EMPTY WORD, not only on a missing row. A
            // feed that records a class but leaves the field blank would
            // otherwise send `instrument=`, which the vendor answers — and that
            // answer would be filed as bars. `CLAUDE.md` §4: never a silent
            // fallback.
            crate::vendor::ParamValue::Segment => self
                .spec
                .listing_words(request.listing)
                .map(|w| w.segment)
                .filter(|word| !word.is_empty())
                .ok_or(FetchError::ListingNotSpellable {
                    listing: request.listing,
                    field: p.name,
                })?
                .to_owned(),
            crate::vendor::ParamValue::Kind => self
                .spec
                .listing_words(request.listing)
                .map(|w| w.kind)
                .filter(|word| !word.is_empty())
                .ok_or(FetchError::ListingNotSpellable {
                    listing: request.listing,
                    field: p.name,
                })?
                .to_owned(),
        })
    }

    /// The auth header's value, or the refusal that stops a source existing.
    ///
    /// # Errors
    ///
    /// [`FetchError::CredentialMismatch`] when the credential does not match
    /// what the scheme names, in either direction. Both directions refuse:
    /// a scheme naming two secrets handed one would send `token :access_token`,
    /// which a vendor answers 403 to and an operator reads as an expired
    /// session; a scheme naming one handed two means a caller resolved a
    /// parameter the descriptor never asked for, and silently dropping it hides
    /// which of the two it was. `CLAUDE.md` §4 — degrade loudly and name the
    /// reason, never a fallback that hides a failure.
    fn header_value(scheme: AuthScheme, held: Credential) -> Result<String, FetchError> {
        match (scheme, held.key) {
            (AuthScheme::Raw, None) => Ok(held.token),
            (AuthScheme::Bearer, None) => Ok(format!("Bearer {}", held.token)),
            (AuthScheme::PrefixedPair { prefix, separator }, Some(key)) => {
                Ok(format!("{prefix}{key}{separator}{}", held.token))
            }
            (scheme, key) => Err(FetchError::CredentialMismatch {
                names_two: scheme.names_two_secrets(),
                given_two: key.is_some(),
            }),
        }
    }

    /// Builds a source for one vendor.
    ///
    /// # Errors
    ///
    /// [`FetchError::TransportFailed`] if the client cannot be constructed —
    /// which on this path means the TLS backend is unavailable, and is a
    /// deployment fault rather than a vendor one.
    /// [`FetchError::CredentialMismatch`] if the credential does not match the
    /// scheme — see [`Self::header_value`].
    pub fn new(spec: HttpSpec, credential: Credential) -> Result<Self, FetchError> {
        // BEFORE THE CLIENT, deliberately. A mismatch is a wiring fault and
        // costs nothing to find; building a TLS client first would spend that
        // work to throw it away.
        let header_value = Self::header_value(spec.auth.scheme, credential)?;
        let client = pooled_client().map_err(|why| FetchError::TransportFailed {
            detail: format!("the HTTPS client could not be built: {why}"),
        })?;
        // THE GOVERNOR IS BUILT FROM THE DESCRIPTOR'S OWN BUDGET, so a feed
        // cannot be governed to a number nobody wrote down. A budget naming no
        // span at all yields `None` -- ungoverned by declaration rather than by
        // omission, which is the distinction `CLAUDE.md` §3 rule 1 wants kept.
        let governor = match crate::rate::Governor::new(
            spec.budget.per_second,
            spec.budget.per_minute,
            spec.budget.per_day,
        ) {
            Ok(g) => Some(std::sync::Arc::new(std::sync::Mutex::new(g))),
            // A ceiling outside the governor's own bounds is a WIRING FAULT in
            // the descriptor, not a runtime condition, and it is refused here
            // rather than silently dropped -- an unenforced budget that reads as
            // an enforced one is the §4 fallback that hides a failure.
            Err(why) => {
                return Err(FetchError::TransportFailed {
                    detail: format!(
                        "{} declares a rate budget this build cannot enforce: {why}.                          Nothing was sent -- a budget that cannot be enforced must not                          read as one that is.",
                        spec.base_url
                    ),
                });
            }
        };
        // THE FEED, ONCE. Five comparisons against a table whose length is
        // pinned to `FEED_COUNT`, done at construction so no answer pays for
        // it. See the `feed` field for why `None` is a real answer and not a
        // failure.
        let feed = crate::vendor::Feed::ALL.into_iter().find(|candidate| {
            matches!(
                &candidate.descriptor().transport,
                crate::vendor::Transport::Http(other) if other.base_url == spec.base_url
            )
        });
        Ok(Self {
            spec,
            feed,
            header_value,
            client,
            governor,
            // OWNED, THEREFORE CHARGED HERE. `sharing` is the only thing that
            // moves the charge to the caller.
            charged_by_caller: false,
        })
    }

    /// Replaces this source's governor with one the caller already holds.
    ///
    /// # Why a caller would
    ///
    /// So that ONE governor per vendor sees every request, whichever path made
    /// it. `new` builds a fresh one from the descriptor, which is right for a
    /// caller that holds no other — a test, a one-shot tool — and wrong for the
    /// server, which spends permits from a governor of its own and would
    /// otherwise be teaching a second instance. See the `governor` field and
    /// P-01 for what that cost.
    ///
    /// A source whose feed declares no budget is left ungoverned: handing one a
    /// governor would enforce a ceiling nobody wrote down, which is the
    /// invention §3 rule 1 forbids.
    #[must_use]
    pub fn sharing(
        mut self,
        governor: Option<std::sync::Arc<std::sync::Mutex<crate::rate::Governor>>>,
    ) -> Self {
        if self.governor.is_some() {
            // THE CALLER NOW CHARGES, and only if it actually handed one over.
            // Sharing a governor and spending from it are one act; both sides
            // calling `admit` is two permits for one request. See
            // `charged_by_caller`.
            self.charged_by_caller = governor.is_some();
            self.governor = governor;
        }
        self
    }

    /// The governor this source gates on, for a caller that must share it.
    #[must_use]
    pub fn governor(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::rate::Governor>>> {
        self.governor.clone()
    }

    /// Waits until this feed's budget admits one more request.
    ///
    /// # Why the wait is here and not at the call site
    ///
    /// Every caller would otherwise have to remember it, and the one that
    /// forgot would be indistinguishable from one that had no budget. The
    /// socket is opened in exactly one place, so the ceiling is enforced in
    /// exactly one place.
    ///
    /// # The lock is never held across an await
    ///
    /// `admit` is arithmetic over three windows; the guard is dropped before
    /// the sleep. A `std::sync::Mutex` held only for that cannot deadlock a
    /// concurrent fan-out, which a lock held across `.await` can.
    ///
    /// # Cost
    ///
    /// One `admit` per attempt, and `admit` walks [`crate::rate::WINDOW_COUNT`]
    /// windows -- a constant three -- so this is O(1) per request and does not
    /// grow with how many requests came before it.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    async fn wait_for_permit(&self) {
        // THE CALLER ALREADY WITHDREW. Charging again here is two permits for
        // one request -- see `charged_by_caller` for what that measured and how
        // it surfaced as a 502.
        //
        // THE FEEDBACK IS NOT SKIPPED. `record_success` and `record_throttled`
        // still run on every answer below, because a shared governor must learn
        // from every path whether or not that path is the one that pays.
        if self.charged_by_caller {
            return;
        }
        let Some(lock) = self.governor.as_ref() else {
            return;
        };
        loop {
            let wait = {
                let now = Self::now_micros();
                // POISON IS NOT A REASON TO STOP GOVERNING. A panic in another
                // chain must not turn the ceiling off for every remaining one,
                // so the guard is taken either way.
                let mut g = lock
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match g.admit(now) {
                    crate::rate::Verdict::Admit => return,
                    crate::rate::Verdict::Deny { wait_micros, .. } => wait_micros,
                }
            };
            // A DENY THAT ASKS FOR NO WAIT WOULD SPIN. The governor does not
            // emit one, and this floor means a future change to it cannot turn
            // this loop into a busy wait.
            let at_least = wait.max(1);
            // COUNTED BEFORE THE SLEEP, NOT AFTER. A run cancelled mid-wait
            // still absorbed the part it waited, and a counter that only
            // credits completed sleeps under-reports exactly the runs an
            // operator is most likely to be asking about.
            crate::rate::note_absorbed(at_least);
            tokio::time::sleep(core::time::Duration::from_micros(at_least)).await;
        }
    }

    /// Microseconds since the epoch, for the governor's windows.
    fn now_micros() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
    }

    /// This feed's endpoint with every value segment left as its placeholder —
    /// `https://api.kite.trade/instruments/historical/:instrument_token/:interval`.
    ///
    /// For a receipt, a log line and a diagnostic. It needs no request and
    /// cannot fail, which is the whole reason it is separate from [`Self::url`]:
    /// a receipt that failed to render because one instrument had no vendor id
    /// would be a worse receipt than one naming the endpoint generically.
    ///
    /// # It takes the RUNG, because a feed can have more than one endpoint
    ///
    /// Dhan serves daily from `/v2/charts/historical` and minute from
    /// `/v2/charts/intraday`. Built from `bars_path` alone this reported the
    /// DEFAULT endpoint whatever was actually fetched — so a minute run's
    /// receipt named the daily URL. A receipt that names the wrong endpoint is
    /// worse than one that names none: it is the diagnostic an operator would
    /// have trusted while looking for a bug somewhere else.
    #[must_use]
    pub fn endpoint(&self, rung: crate::vendor::Granularity) -> String {
        format!(
            "{}{}",
            self.spec.base_url,
            self.spec.path_template_for(rung)
        )
    }

    /// The URL **one particular request** is fetched from.
    ///
    /// For a feed whose path is all literals this is [`Self::endpoint`] and can
    /// only succeed. For a feed that carries its instrument or its rung as a
    /// PATH SEGMENT it resolves each one through the same table a query
    /// parameter uses, so a rung with no recorded word refuses here exactly as
    /// it would refuse in a query string — before the socket, rather than
    /// fetching one bar length and filing it under another.
    ///
    /// # Errors
    ///
    /// Whatever [`Self::resolve_param`] refuses — [`FetchError::RungNotSpellable`]
    /// and [`FetchError::ListingNotSpellable`] — plus
    /// [`FetchError::PathSegmentUnusable`] for a resolved value that cannot sit
    /// in a URL path.
    pub fn url(&self, request: &BarRequest, from: &str, to: &str) -> Result<String, FetchError> {
        let mut out = String::from(self.spec.base_url);
        // THE RUNG PICKS THE ENDPOINT. `path_for` is `bars_path` for every feed
        // that serves its rungs from one URL, and the rung's own path for one
        // that does not — see `vendor::HttpSpec::rung_routes` for the daily /
        // intraday split that made this necessary and for what it cost when one
        // field had to answer for both.
        for segment in self.spec.path_for(request.granularity) {
            out.push('/');
            match *segment {
                crate::vendor::PathSegment::Literal(word) => out.push_str(word),
                crate::vendor::PathSegment::Value { placeholder, value } => {
                    // The placeholder IS the field name here — a value segment
                    // and a query parameter are the same "named value from the
                    // request", so they resolve through one function and a
                    // fifth source cannot appear in one and not the other.
                    let param = crate::vendor::Param {
                        name: placeholder,
                        value,
                    };
                    let resolved = self.resolve_param(&param, request, from, to)?;
                    out.push_str(path_safe(&resolved, placeholder)?);
                }
            }
        }
        Ok(out)
    }

    /// A date as this vendor writes it on the wire.
    ///
    /// The same four formats [`crate::csv`] reads, written rather than parsed.
    /// One function per direction and one table of formats, so a vendor cannot
    /// be read one way and written another.
    #[must_use]
    pub fn on_the_wire(day: crate::session::Day, format: DateFormat) -> String {
        let (y, m, d) = (day.year(), day.month(), day.day());
        match format {
            DateFormat::DashedYmd => format!("{y:04}-{m:02}-{d:02}"),
            DateFormat::CompactYmd => format!("{y:04}{m:02}{d:02}"),
            DateFormat::SlashedDmy => format!("{d:02}/{m:02}/{y:04}"),
            DateFormat::CompactDmy => format!("{d:02}{m:02}{y:04}"),
            DateFormat::DashedYmdMidnight => format!("{y:04}-{m:02}-{d:02} 00:00:00"),
        }
    }

    /// The end date this vendor's range takes, honouring its inclusivity.
    ///
    /// **One conversion site.** Dhan's `toDate` is exclusive, so the wire value
    /// is the day *after* the operator's last day; a vendor whose end is
    /// inclusive takes it unchanged. Two sites would be two answers and the
    /// off-by-one would return the first time either was edited.
    ///
    /// # Errors
    ///
    /// [`FetchError::TransportFailed`] past 9999-12-31 for an exclusive vendor,
    /// which has no successor to take.
    pub fn wire_end(
        last: crate::session::Day,
        end: RangeEnd,
        format: DateFormat,
    ) -> Result<String, FetchError> {
        let day = match end {
            RangeEnd::Inclusive => last,
            RangeEnd::Exclusive => last.succ().map_err(|why| FetchError::TransportFailed {
                detail: format!("{last} has no successor to put on the wire: {why}"),
            })?,
        };
        // A DATETIME FORMAT MUST NOT COLLAPSE A DAY TO A POINT.
        //
        // `on_the_wire` renders midnight, which is right for the START of a
        // window and wrong for the end of an INCLUSIVE one: a single-day pull
        // then sends `2026-08-04 00:00:00` for both ends, and Groww refuses
        // with `GA001 Start time should be less than end time`. Measured, not
        // reasoned about.
        //
        // An exclusive end is already the following day, so midnight there is
        // exactly the boundary and must stay midnight.
        if format == DateFormat::DashedYmdMidnight && end == RangeEnd::Inclusive {
            let (y, m, d) = (day.year(), day.month(), day.day());
            return Ok(format!("{y:04}-{m:02}-{d:02} 23:59:59"));
        }
        Ok(Self::on_the_wire(day, format))
    }

    /// The auth header this vendor takes, name and value.
    ///
    /// Returned as a pair rather than applied inside, so a test can assert the
    /// NAME without ever seeing the value.
    fn header(&self) -> (&'static str, &str) {
        (self.spec.auth.header, &self.header_value)
    }
}

/// The wire, as it actually behaved: one line per vendor answer.
///
/// # Why this exists
///
/// `window_async` was the single darkest place in the workspace — the one spot
/// that talks to a vendor, saying nothing about any of it. A run that was
/// throttled, one answered with an empty body and one refused outright all
/// looked identical from outside, and this session was spent reconstructing by
/// hand the facts this function now writes down.
///
/// # The level is the status
///
/// `Trace` when the vendor answered, `Warn` when it did not. One line per
/// request is ~62,600 on a one-minute backfill, so the ordinary case is off
/// unless an operator asks for it and the case worth waking up for never is.
///
/// # THE URL IS LOGGED AND THE CREDENTIAL IS NOT
///
/// `url` is the vendor.s public endpoint. The token travels in a header that
/// appears in no field here, for the reason `CLAUDE.md` section 8 keeps the
/// parameter path out of every tracked file: this log is a file an operator
/// will paste into an issue.
fn note_answer(
    url: &str,
    status: u16,
    ok: bool,
    request: &BarRequest,
    sent: &[(&'static str, String)],
) {
    // BUILT BEFORE THE EVENT so it outlives the borrow, and EMPTY ON SUCCESS so
    // the clean path spends nothing formatting a field it will not carry.
    let sent_text = if ok {
        String::new()
    } else {
        sent.iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    // BOUND, NOT INLINED. The event now outlives the expression that builds it,
    // so a `&…to_string()` temporary would be freed while the event still holds
    // the borrow.
    let from = request.window.from().to_string();
    let to = request.window.to().to_string();
    let event = telemetry::Event::new(
        if ok {
            telemetry::Level::Trace
        } else {
            telemetry::Level::Warn
        },
        "pull.http",
        "vendor answered",
    )
    .with("url", telemetry::Value::Str(url))
        .with("status", telemetry::Value::Uint(u64::from(status)))
        .with(
            "instrument_id",
            telemetry::Value::Str(&request.instrument_id),
        )
        .with("from", telemetry::Value::Str(&from))
        .with("to", telemetry::Value::Str(&to))
        // WHAT WAS ACTUALLY SENT, ON A REFUSAL ONLY.
        //
        // Dhan answered `DH-905 Input_Exception — "Missing required fields, bad
        // values for parameters etc."` on 2026-08-25 and the log recorded the
        // URL, the status and the window. None of those is a field, so the one
        // question the refusal asks — WHICH field — could not be answered from
        // this repository at all, and guessing at it is the invention §3 rule 1
        // forbids. A vendor that says a parameter is wrong is only actionable
        // beside the parameters.
        //
        // THE CREDENTIAL CANNOT REACH THIS. It travels in a header, set at the
        // call site as `builder.header(name, value)`; `pairs` is built from
        // `spec.params` alone and no `ParamValue` resolves to a secret. That is
        // the same boundary the URL is logged under — §8 keeps the token out of
        // a file an operator will paste into an issue.
        //
        // Refusals only, because one line per request is ~62,600 on a
        // one-minute backfill and the clean path has nothing to diagnose.
        ;
    let event = if ok {
        event
    } else {
        event.with("sent", telemetry::Value::Str(&sent_text))
    };
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// Turns a vendor body into rows, using only what the descriptor declares.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] for a body that is not JSON or not the
/// declared shape, and whatever [`RawWindow::decode`] refuses — which includes
/// **the seven arrays disagreeing in length**, the trap that would otherwise
/// yield a short window filed as complete.
pub fn decode_body(
    body: &str,
    spec: &HttpSpec,
    listing: crate::vendor::Listing,
) -> Result<RawWindow, FetchError> {
    // COUNTED ACROSS THE WHOLE BODY, reported ONCE. A per-row event would put
    // a `telemetry::emit` inside the decode loop; one per window is the
    // granularity `CLAUDE.md` section 3 rule 4 affords. See `one_volume`.
    let mut corrected = 0usize;
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|why| FetchError::TransportFailed {
            detail: format!("the vendor's answer is not JSON: {why}"),
        })?;

    let decoded = match spec.response {
        ResponseShape::ParallelArrays { envelope } => {
            // EVERY FIELD IS READ FROM ONE OBJECT, RESOLVED ONCE.
            //
            // Each of the seven used to be looked up on its own, and each
            // lookup fell back to searching *every* value one level below the
            // root for a key of that name. So a body holding two objects that
            // both carry bar fields — a payload beside a cached copy, a primary
            // beside a fallback, two exchanges in one answer — could have its
            // `open` taken from the first and its `close` from the second, and
            // the bar assembled from them never existed.
            //
            // Nothing downstream could catch it. The seven-array length check
            // passes when both objects hold the same number of bars, which is
            // exactly when two such objects would appear together, and the
            // result is a window of well-formed bars that no vendor ever sent.
            //
            // The descriptor has always carried `envelope` and this decoder
            // ignored it. It is now the answer: one container, named by the
            // row in `crate::vendor`, and all seven fields come out of it.
            // A CLOSURE, SO `?` CANNOT SKIP THE REPORT BELOW.
            //
            // This arm built its struct literal inline, so every `?` in it
            // returned from `decode_body` ITSELF -- past
            // `note_volumes_corrected`. A window that corrected an index volume
            // and then failed on any later field emitted nothing at all, and
            // this is the shape Dhan actually answers in, so production took
            // exactly that path. The other two arms return a `Result` into
            // `decoded` and were never affected. D-0332.
            (|| -> Result<RawWindow, FetchError> {
                let root = container(&root, envelope)?;
                let f = spec.fields;
                // THE MASK FIRST, so every column below is filtered the same
                // way and a minute the vendor did not trade is skipped rather
                // than refusing the whole window. See `kept_rows` for why this
                // shape needed a mask when the other two did not.
                let decided = kept_rows(root, &f, listing)?;
                let keep = decided.keep;
                let arrays = ParallelArrays {
                    // PRICES GO THROUGH `prices`, NOT `numbers`, AND THE
                    // DIFFERENCE IS 75 PAISE ON EVERY BAR THAT HAS THEM.
                    //
                    // `numbers` rounds a JSON number to an integer, which is right
                    // for a volume and a timestamp and WRONG for a price. A vendor
                    // quoting rupees sends `24500.75`; rounding that to `24501` and
                    // letting `fetch::to_paisa` multiply by 100 stores ₹24,501.00
                    // and the paise are gone, silently, on every bar.
                    //
                    // `CLAUDE.md` §7 puts the tick grid at TWO decimal places and
                    // the single snap at the write boundary. Rounding to whole
                    // rupees here is a snap at the wrong granularity in the wrong
                    // place. `prices` therefore scales first and rounds once, while
                    // the paise are still in the float.
                    open: prices(root, f.open, spec.prices, &keep)?,
                    high: prices(root, f.high, spec.prices, &keep)?,
                    low: prices(root, f.low, spec.prices, &keep)?,
                    close: prices(root, f.close, spec.prices, &keep)?,
                    volume: volumes(root, f.volume, listing, &mut corrected, &keep)?,
                    timestamp: numbers(root, f.timestamp, &keep)?,
                    // OPEN INTEREST IS OPTIONAL AND ITS ABSENCE IS NOT A ZERO.
                    // A spot index has none, so the descriptor leaves the name
                    // `None` and no array is looked for. When the descriptor DOES
                    // name one and the vendor omits it, that is a shape the
                    // descriptor got wrong and it must be refused rather than
                    // filled in — `CLAUDE.md` §7: `i64::MIN` is the null and zero
                    // means zero, so a silent `Vec::new()` here would later read
                    // back as real open interest of nothing.
                    open_interest: match f.open_interest {
                        Some(name) => numbers(root, name, &keep)?,
                        None => Vec::new(),
                    },
                };
                note_null_bars(decided.null_bars, keep.len());
                note_negative_volume_bars(decided.negative_volume, keep.len());
                note_negative_interest_bars(decided.negative_open_interest, keep.len());
                let mut arrays = arrays;
                note_impossible_bars(drop_impossible_bars(&mut arrays), keep.len());
                RawWindow::decode(&arrays)
            })()
        }
        // ONE OBJECT PER BAR — the shape `crate::vendor`'s Groww row declares.
        //
        // This arm refused for as long as no vendor in this build used it, and
        // the refusal said so in as many words: *a decoder nobody has run
        // against a real body is a decoder that is wrong.* That is still true,
        // and it is why the fields below are read strictly by the names the
        // descriptor gives rather than by position — a positional reader would
        // silently file a high as a low the first time a vendor reordered.
        //
        // Each element carries the seven fields the parallel-array shape
        // spreads across seven arrays, so the SAME `prices` and `numbers`
        // conversions run per object: rupees to paisa through `csv::paisa`, no
        // float, and a value off the tick grid refused by name.
        ResponseShape::ArrayOfObjects { envelope } => {
            decode_objects(&root, spec, envelope, listing, &mut corrected)
        }
        ResponseShape::PositionalRows { envelope, array } => {
            decode_positional(&root, spec, envelope, array, listing, &mut corrected)
        }
    };
    // ONLY WHEN A WINDOW ACTUALLY LANDED. A refused window wrote nothing, so an
    // event saying values "were recorded as zero" would be describing a write
    // that did not happen — and a correction counted against bars that were
    // then thrown away is worse than no line at all.
    if let Ok(window) = &decoded {
        note_volumes_corrected(corrected, window.rows.len());
    }
    decoded
}

/// Says how many index volumes were recorded as the zero their column always
/// is, or says nothing when none were.
///
/// # Why this is one line per WINDOW and not per bar
///
/// A negative volume on an index is noise in a column with no referent — see
/// [`one_volume`] — and correcting it silently is the fallback `CLAUDE.md` §4
/// bans. Correcting it *loudly* is the same rule's other half.
///
/// One event per window rather than per row, because a 90-day minute chunk is
/// ~34,000 bars and an emit inside that loop is the cost §3 rule 4 refuses. The
/// count is the fact worth carrying; the individual rows are not.
///
/// It carries the bar count as a denominator, because `corrected` alone cannot
/// separate "a few noisy rows" from "every row, so the decoder is reading the
/// wrong column" -- and the second is what a negative volume means on every
/// other listing. Only ever fires for `Listing::Index`: no other listing
/// increments the counter, so no other listing reaches this.
fn note_volumes_corrected(corrected: usize, bars: usize) {
    if corrected == 0 {
        return;
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            telemetry::Level::Warn,
            "pull.decode",
            "an index carried a negative volume and it was recorded as zero",
        )
        .with(
            "corrected",
            telemetry::Value::Uint(u64::try_from(corrected).unwrap_or(u64::MAX)),
        )
        // THE DENOMINATOR, which its sibling event two hundred lines up already
        // carries and this one did not. Without it, 34,000 of 34,000 — a
        // decoder reading the wrong column, which is what a negative volume
        // means everywhere else — reads exactly like 34,000 of 100,000, which
        // is a vendor with a bad patch. Those want opposite responses.
        .with(
            "bars",
            telemetry::Value::Uint(u64::try_from(bars).unwrap_or(u64::MAX)),
        ),
    );
}

/// One object per bar, into the same seven arrays the other shape arrives as.
///
/// # Why this transposes rather than adding a second pipeline
///
/// [`RawWindow::decode`] already owns the length check, the row assembly and
/// every refusal below it. A decoder that built rows directly would be a second
/// answer to "what is a bar", and the two would drift — the same argument
/// `crate::ingest::from_members` makes about the two ingest paths. So this
/// collects the objects' fields into columns and hands them to the one decoder.
///
/// # The length check still means something here
///
/// In the parallel-array shape the seven arrays can disagree; here they cannot,
/// because they are built by walking one list. What CAN differ is an object
/// missing a field, and that is refused **by name and by index** rather than
/// filled in — a bar with a defaulted close is a bar that looks real.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the field and the element for
/// anything that is not the declared shape, and whatever [`RawWindow::decode`]
/// refuses.
fn decode_objects(
    root: &serde_json::Value,
    spec: &HttpSpec,
    envelope: Option<&'static str>,
    listing: crate::vendor::Listing,
    corrected: &mut usize,
) -> Result<RawWindow, FetchError> {
    let container = container(root, envelope)?;
    let f = spec.fields;

    // The array itself is found the same way a parallel array is: by the name
    // the descriptor gives, inside the one container. `crate::vendor` names it
    // through the same `FieldNames` row, so there is no second convention.
    let items = array_at(container, f.timestamp)
        .or_else(|_| array_at(container, "candles"))
        .or_else(|_| {
            container
                .as_array()
                .ok_or_else(|| FetchError::TransportFailed {
                    detail: format!(
                        "this vendor declares one object per bar, and the \
                         container is neither an array nor holds one named \
                         {:?}. It has: {}",
                        f.timestamp,
                        keys_of(container)
                    ),
                })
        })?;

    let mut arrays = ParallelArrays {
        open: Vec::with_capacity(items.len()),
        high: Vec::with_capacity(items.len()),
        low: Vec::with_capacity(items.len()),
        close: Vec::with_capacity(items.len()),
        volume: Vec::with_capacity(items.len()),
        timestamp: Vec::with_capacity(items.len()),
        open_interest: Vec::new(),
    };

    let mut null_bars = 0usize;
    for (i, item) in items.iter().enumerate() {
        // A field missing from ONE object is refused naming both the field and
        // which bar it was, because "the vendor sent 400 bars and one of them
        // has no close" is a different fault from "the shape is wrong" and
        // sends an operator somewhere different.
        let one = |name: &str| -> Result<&serde_json::Value, FetchError> {
            item.get(name).ok_or_else(|| FetchError::TransportFailed {
                detail: format!("bar {i} carries no {name:?}. It has: {}", keys_of(item)),
            })
        };
        // A BAR WITH A NULL PRICE IS SKIPPED, NOT FATAL.
        //
        // Groww answers for an equity with `open: null` on a minute that did
        // not trade. This decoded every bar and returned `Err` on the first
        // one, so ONE untraded minute killed the whole instrument — measured on
        // a real 785-instrument pull: 24 reached, 761 refused, and the first
        // refusal read `"open" holds null`. The vendor was answering correctly
        // and the decoder was throwing the answer away.
        //
        // Skipped rather than zero-filled: a zero price is a LIE about a minute
        // that had no trade, and this store cannot tell an invented zero from a
        // real one afterwards.
        //
        // Skipped rather than silent: `null_bars` is counted and travels with
        // the window, so a run that dropped half its bars says so. `CLAUDE.md`
        // §4 — degrade loudly and name the reason.
        //
        // ALL FOUR PRICES ARE CHECKED before any is pushed. Pushing open and
        // then discovering close is null would leave the arrays at different
        // lengths, which `RawWindow` refuses — correctly, and with a message
        // about column lengths that says nothing about the null that caused it.
        let quartet = [f.open, f.high, f.low, f.close];
        if quartet
            .iter()
            .any(|name| one(name).is_ok_and(serde_json::Value::is_null))
        {
            null_bars += 1;
            continue;
        }
        arrays
            .open
            .push(one_price(one(f.open)?, f.open, spec.prices)?);
        arrays
            .high
            .push(one_price(one(f.high)?, f.high, spec.prices)?);
        arrays.low.push(one_price(one(f.low)?, f.low, spec.prices)?);
        arrays
            .close
            .push(one_price(one(f.close)?, f.close, spec.prices)?);
        arrays
            .volume
            .push(one_volume(one(f.volume)?, f.volume, listing, corrected)?);
        arrays
            .timestamp
            .push(one_number(one(f.timestamp)?, f.timestamp)?);
        if let Some(name) = f.open_interest {
            arrays.open_interest.push(one_number(one(name)?, name)?);
        }
    }

    // NOT SILENT. A window whose bars were mostly untraded minutes is a
    // window an operator has to know about — it is not an error, and it is not
    // a full answer either. Emitted once per window rather than once per bar,
    // because 375 lines of "skipped" is noise and one count is information.
    if null_bars > 0 {
        // BOTH, and the event is the load-bearing one. `eprintln!` reaches the
        // operator watching a terminal; the event reaches the log FILE, which is
        // the thing handed to somebody diagnosing a run that already finished.
        // A diagnostic that exists only on a terminal nobody kept is a fact this
        // repository did not record. See gate 22.
        let _noted = telemetry::emit(
            &telemetry::Event::new(
                telemetry::Level::Warn,
                "pull.decode",
                "bars carried a null price and were skipped",
            )
            .with("skipped", u64::try_from(null_bars).unwrap_or(u64::MAX))
            .with("bars", u64::try_from(items.len()).unwrap_or(u64::MAX)),
        );
        eprintln!(
            "brutex: {null_bars} of {} bars carried a null price and were \
             skipped — the vendor reported no trade in those minutes",
            items.len()
        );
    }

    // THE THIRD DOOR GETS THE RULE AT THE SAME TIME AS THE FIRST. Three
    // separate rules in this decoder reached two of the three shapes and missed
    // the same one; this one is applied at every `RawWindow::decode` in the
    // file, so a shape cannot be forgotten without deleting the call.
    note_impossible_bars(drop_impossible_bars(&mut arrays), items.len());
    RawWindow::decode(&arrays)
}

/// The unit [`decode_body`] leaves prices in, whatever the vendor quoted.
///
/// **A caller passing this window to [`crate::fetch::land`] must pass
/// [`PriceScale::Paisa`], not `spec.prices`.** The conversion has already
/// happened, and happening twice would multiply every price by 100 again.
pub const DECODED_PRICE_SCALE: PriceScale = PriceScale::Paisa;

/// One named array of **prices**, converted to exact paisa without a float.
///
/// # Two wrong answers preceded this one
///
/// The first read a price with `f.round()` and let [`crate::fetch::to_paisa`]
/// multiply by 100. A vendor quoting `24500.75` therefore stored ₹24,501.00 and
/// **the 75 paise were gone**, silently, on every bar that had them.
///
/// The second scaled before rounding — `24500.75 × 100` — which is *arithmetically*
/// right and still wrong for this repository: `clippy::float_arithmetic` is
/// denied workspace-wide, precisely so that `CLAUDE.md` §7's "never a float"
/// cannot be walked back one expression at a time. The lint was correct. There
/// is no float in a price here, not even briefly.
///
/// # What it does instead
///
/// [`crate::csv::paisa`] already turns `"24500.75"` into `2450075` by splitting
/// on the point and doing integer arithmetic — it is what the local-archive
/// path has always used, and it is now shared rather than reimplemented. JSON's
/// number is rendered back to its text with `Display`, which `serde_json` emits
/// via shortest-round-trip formatting, so a two-decimal price round-trips
/// character for character.
///
/// A VENDOR SENDING MORE DECIMALS THAN THE TICK GRID HOLDS IS **SNAPPED**,
/// HALF-UP, AND THIS USED TO REFUSE INSTEAD.
///
/// The refusal read `100.005` as evidence that the descriptor's `PriceScale`
/// was wrong, on the reasoning that a third decimal cannot occur on an NSE
/// price. Measured against a real vendor, that reasoning is false, and the
/// refusal cost every Dhan minute backfill this repository ever attempted:
/// forty-two runs, none of which reached the store, each dying on chunk 1 of 21
/// with `"open" holds 35922.6016`.
///
/// **The extra digits are the vendor's float, not the exchange's price.**
/// `Dhan Docs/12-historical-data.md` types every OHLC field as `float`, and a
/// binary float cannot hold most two-decimal decimals. BANKNIFTY's real
/// `35922.60` is held as the nearest `f32`, `35922.6015625`, and printed to four
/// places as `35922.6016`. That value is an exact dyadic rational —
/// `35922.6015625 × 256 = 9_196_186`, a whole number — which only a float
/// produces. The same holds for `35447.05` → `35447.0507812` → `35447.0508`.
/// At that magnitude one `f32` step is 1/256 of a rupee, so of the hundred
/// two-decimal endings only `.00`, `.25`, `.50` and `.75` survive the round
/// trip: **96% of bars carry float error before the vendor sends them.**
///
/// So snapping half-up does not lose precision — it **removes the vendor's
/// rounding error and recovers the exchange's own two-decimal value**. Keeping
/// `35922.6016` would freeze that error into an append-only store forever, and
/// `16797.2999` is further from the truth than `16797.30` is.
///
/// This is also what the law always said. `CLAUDE.md` §7 — *"Snapping happens
/// once, at the write boundary, half-up"* — and `docs/05-decisions.md` D-0010
/// both mandate the snap; the refusal was locked in a commit body with no
/// ledger entry, which `CLAUDE.md` §9 requires. D-0321.
///
/// **Counts are not prices and are not snapped.** [`one_number`] still routes
/// through [`crate::csv::paisa`] and still refuses a fractional share count:
/// `250.5` of anything remains a shape this build does not understand.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the field and the value, for anything
/// that is not a decimal number or does not fit `i64` paisa.
fn prices(
    root: &serde_json::Value,
    name: &str,
    scale: PriceScale,
    keep: &[bool],
) -> Result<Vec<i64>, FetchError> {
    kept(array_at(root, name)?, keep)
        .map(|v| one_price(v, name, scale))
        .collect()
}

/// One price value, whatever shape carried it.
///
/// Shared by both response shapes so a rupee is converted the same way whether
/// it arrived in a column or in an object — two conversions would be two
/// answers, and the second one would lose the paise first.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the field and the value, for anything
/// that is not a decimal number or does not fit `i64` paisa.
fn one_price(v: &serde_json::Value, name: &str, scale: PriceScale) -> Result<i64, FetchError> {
    let refuse = || FetchError::TransportFailed {
        detail: format!(
            "{name:?} holds {v}, which is not a price this build can put on the \
             paisa grid"
        ),
    };
    let Some(number) = v.as_number() else {
        return Err(refuse());
    };
    let paisa = match scale {
        // Already paisa: an integer count, and nothing to convert.
        PriceScale::Paisa => number.as_i64().ok_or_else(refuse)?,
        // Rupees: the text is the truth, and `core`'s half-up reader owns the
        // rule. NOT `csv::paisa`, which refuses past two decimals — see the
        // header on `prices` for why that refusal was wrong and what it cost.
        //
        // STILL NO FLOAT. `serde_json` renders the number back to its shortest
        // round-tripping text and the conversion walks that text digit by digit,
        // so `clippy::float_arithmetic` stays satisfied and a two-decimal price
        // round-trips character for character.
        PriceScale::Rupees => {
            let text = number.to_string();
            let snapped = brutex_core::price::Paisa::from_rupee_text_half_up(&text)
                .map_err(|_| refuse())?
                .raw();
            // A NON-ZERO PRICE THAT SNAPS TO ZERO IS REFUSED, AND WITHOUT THIS
            // THE SNAP OPENED A HOLE THE REFUSAL NEVER HAD.
            //
            // Half-up sends everything under half a paisa to zero, so a vendor
            // sending `0.0001` — or, worse, `-0.001` — decoded as a clean `0`
            // where `csv::paisa` had refused it outright. Zero is a legal price
            // (`Bar::ohlc_is_sane` admits it and
            // `every_price_on_the_paisa_grid_survives_intact` pins `"0"` as
            // real), so nothing downstream could tell the difference.
            //
            // The negative case is the sharp one: `-0.001` snaps to `0`, and
            // `0 < 0` is false, so it walked straight past the below-zero guard
            // twenty lines down — the guard whose whole purpose is to catch a
            // descriptor whose `PriceScale` is wrong. That diagnostic was
            // unreachable for the entire `(-0.005, 0)` band.
            //
            // The test is on the TEXT, not the number: a value the vendor wrote
            // with a non-zero digit is not zero, whatever it rounds to. `0`,
            // `0.00` and `-0.0` carry no non-zero digit and still decode as the
            // real zero they are. Bounded scan over one rendered number, so the
            // cost is the same constant the conversion above already pays.
            if snapped == 0 && text.bytes().any(|b| b.is_ascii_digit() && b != b'0') {
                return Err(FetchError::TransportFailed {
                    detail: format!(
                        "{name:?} holds {v}, which is not zero and is smaller \
                         than half a paisa, so snapping it to the tick grid \
                         would store a zero the vendor did not send. Zero is a \
                         real price here, which is exactly why a value that is \
                         not zero must not become one."
                    ),
                });
            }
            snapped
        }
    };
    // A NEGATIVE PRICE IS NOT A PRICE, AND IT USED TO LAND.
    //
    // `csv::paisa` parses a leading minus deliberately — it is a general
    // decimal reader and a negative is a real value for fields that can hold
    // one. Nothing downstream disagreed: a negative open decoded here, survived
    // `land`, folded, appended, and the month was recorded as good.
    // `store::format::Bar::ohlc_is_sane` checked the four prices' ORDERING and
    // nothing else, and -100/-100/-100/-100 satisfies every clause of it.
    // D-0143 closed that at the write boundary too, so this is now the outer of
    // two refusals rather than the only one.
    //
    // Nothing on the exchanges this build reads trades below zero. A negative
    // arriving here means the descriptor's `PriceScale` is wrong, or the vendor
    // sent something that is not a price — both faults to name rather than to
    // store.
    //
    // Refused HERE, where the vendor's own value is still visible. By the
    // append it is one `i64` among millions with nothing left to say where it
    // came from. Same place a third decimal place is already refused, and for
    // the same reason.
    if paisa < 0 {
        return Err(FetchError::TransportFailed {
            detail: format!(
                "{name:?} holds {v}, which is below zero. Nothing this build \
                 reads trades at a negative price, so this is the descriptor's \
                 price scale being wrong or the vendor sending something that \
                 is not a price — and a stored negative passes every ordering \
                 check downstream, which is why it is refused here rather than \
                 written."
            ),
        });
    }
    Ok(paisa)
}

/// One named array of counts — volumes, timestamps, open interest.
///
/// These are integers in their own right and no scale applies. A vendor writing
/// `250.0` means two hundred and fifty, so the same text parser is reused and
/// the hundredths are required to be zero; `250.5` of anything is a shape this
/// build does not understand and says so.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the field and the value.
fn numbers(root: &serde_json::Value, name: &str, keep: &[bool]) -> Result<Vec<i64>, FetchError> {
    kept(array_at(root, name)?, keep)
        .map(|v| one_number(v, name))
        .collect()
}

/// One named array of VOLUMES, which have a floor the other counts do not.
///
/// # Why this exists rather than a flag on [`numbers`]
///
/// [`numbers`] serves volume, timestamp and open interest, and only volume is
/// floored at zero — see [`one_volume`] for why a timestamp's negative range is
/// legal and must stay so. A boolean parameter would put the rule at the call
/// site instead of beside the field, and a call site that passed the wrong
/// boolean would fail silently.
///
/// **THIS IS THE ARRAY SHAPE DHAN ACTUALLY ANSWERS IN, and it was the site the
/// first version of this fix missed.** There are three doors from a vendor body
/// to a volume — this one, [`decode_objects`] and [`decode_positional`] — and
/// patching the latter two left the parallel-array arm, which is the only one
/// Dhan uses, still accepting `-95342`. A rule that guards two of three doors is
/// not a rule.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the field and the value.
fn volumes(
    root: &serde_json::Value,
    name: &str,
    listing: crate::vendor::Listing,
    corrected: &mut usize,
    keep: &[bool],
) -> Result<Vec<i64>, FetchError> {
    kept(array_at(root, name)?, keep)
        .map(|v| one_volume(v, name, listing, corrected))
        .collect()
}

/// Says how many rows carried a null price and were skipped, or says nothing.
///
/// The same report the object shape has always written, taken here so the
/// columnar shape says it too. `CLAUDE.md` §4: a bar dropped silently is a
/// fallback that hides a failure; a bar dropped and counted is the loud degrade
/// the same rule allows.
///
/// **BOTH, and the event is the load-bearing one.** `eprintln!` reaches an
/// operator watching a terminal; the event reaches the log FILE, which is what
/// is handed to somebody diagnosing a run that already finished.
fn note_null_bars(null_bars: usize, bars: usize) {
    if null_bars == 0 {
        return;
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            telemetry::Level::Warn,
            "pull.decode",
            "bars carried a null price and were skipped",
        )
        .with(
            "skipped",
            telemetry::Value::Uint(u64::try_from(null_bars).unwrap_or(u64::MAX)),
        )
        .with(
            "bars",
            telemetry::Value::Uint(u64::try_from(bars).unwrap_or(u64::MAX)),
        ),
    );
    eprintln!(
        "brutex: {null_bars} of {bars} bars carried a null price and were \
         skipped — the vendor reported no trade in those minutes"
    );
}

/// One event per WINDOW for rows dropped over an impossible volume.
///
/// Per window and never per row, for the reason [`note_volumes_corrected`]
/// gives: a 90-day minute chunk is ~34,000 bars, and an emit inside that loop
/// is the cost `CLAUDE.md` §3 rule 4 refuses.
///
/// **The denominator is carried because the ratio is the diagnosis.** A handful
/// out of 34,000 is a vendor sending noise in a column that mostly works.
/// 34,000 of 34,000 is the decoder reading the wrong column entirely, and those
/// two want opposite responses from an operator.
fn note_negative_volume_bars(negative_bars: usize, bars: usize) {
    if negative_bars == 0 {
        return;
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            telemetry::Level::Warn,
            "pull.decode",
            "bars carried a negative volume and were skipped",
        )
        .with(
            "skipped",
            telemetry::Value::Uint(u64::try_from(negative_bars).unwrap_or(u64::MAX)),
        )
        .with(
            "bars",
            telemetry::Value::Uint(u64::try_from(bars).unwrap_or(u64::MAX)),
        ),
    );
    eprintln!(
        "brutex: {negative_bars} of {bars} bars carried a NEGATIVE volume and \
         were skipped — a volume counts shares traded, so those rows carry no \
         quantity. The rest of the window is kept: this used to refuse all of \
         it, which cost one instrument every intraday rung it had."
    );
}

/// One event per WINDOW for rows whose open interest cannot be a count.
///
/// Its own message rather than sharing the volume's, because the two send an
/// operator to different places: a bad volume on a spot pull says the decoder
/// is reading the wrong column, and a bad open interest can only come from a
/// derivative request, where the contract itself may be the thing that is
/// wrong.
fn note_negative_interest_bars(negative_bars: usize, bars: usize) {
    if negative_bars == 0 {
        return;
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            telemetry::Level::Warn,
            "pull.decode",
            "bars carried a negative open interest and were skipped",
        )
        .with(
            "skipped",
            telemetry::Value::Uint(u64::try_from(negative_bars).unwrap_or(u64::MAX)),
        )
        .with(
            "bars",
            telemetry::Value::Uint(u64::try_from(bars).unwrap_or(u64::MAX)),
        ),
    );
    eprintln!(
        "brutex: {negative_bars} of {bars} bars carried a NEGATIVE open \
         interest and were skipped — open interest is contracts outstanding \
         and is never below zero. `i64::MIN` is NOT counted here: that is the \
         null sentinel and is refused by name."
    );
}

/// One event per WINDOW for rows whose four prices cannot be a bar.
///
/// Same per-window rule and same denominator as the two above, and the same
/// reason for both.
fn note_impossible_bars(dropped: usize, bars: usize) {
    if dropped == 0 {
        return;
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            telemetry::Level::Warn,
            "pull.decode",
            "bars carried an impossible OHLC and were skipped",
        )
        .with(
            "skipped",
            telemetry::Value::Uint(u64::try_from(dropped).unwrap_or(u64::MAX)),
        )
        .with(
            "bars",
            telemetry::Value::Uint(u64::try_from(bars).unwrap_or(u64::MAX)),
        ),
    );
    eprintln!(
        "brutex: {dropped} of {bars} bars carried an impossible OHLC and were \
         skipped — a high below its low, or a negative price. Caught here, \
         where the vendor's own row is still in hand, rather than at the store \
         append where the index names nothing an operator can open."
    );
}

/// Which rows of a columnar body are bars at all, and how many are not.
///
/// # WHY THE COLUMNAR SHAPE NEEDED THIS AND THE OTHER TWO DID NOT
///
/// A vendor answers `open: null` for a minute that did not trade.
/// [`decode_objects`] meets that one object at a time and `continue`s;
/// [`decode_positional`] meets it one row at a time and does the same. **The
/// parallel-array shape meets it as a COLUMN**, and a `map` over a column
/// cannot skip an element without skipping the same index in the other six — so
/// it refused instead, and refusing is what one untraded minute cost.
///
/// **That is the third time a rule reached two of the three decode doors.**
/// D-0323 missed this same arm on negative volumes, D-0332 missed it again, and
/// both times the door that was missed is the only one Dhan answers in. Dhan's
/// own field table marks every response field *Required: No*.
///
/// All four prices are checked before any row is kept, for the reason
/// [`decode_objects`] gives: keeping `open` and then discovering `close` is null
/// would leave the columns at different lengths, which `RawWindow` refuses with
/// a message about column lengths that says nothing about the null that caused
/// it.
///
/// **Skipped rather than zero-filled.** A zero price is a lie about a minute
/// that had no trade, and this store cannot tell an invented zero from a real
/// one afterwards.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] when a named price column is absent — the
/// refusal [`array_at`] already makes, taken here so a mask cannot be built from
/// a column that does not exist.
fn kept_rows(
    root: &serde_json::Value,
    f: &crate::vendor::FieldNames,
    listing: crate::vendor::Listing,
) -> Result<Kept, FetchError> {
    let quartet = [
        array_at(root, f.open)?,
        array_at(root, f.high)?,
        array_at(root, f.low)?,
        array_at(root, f.close)?,
    ];
    let volume = array_at(root, f.volume)?;
    let timestamp = array_at(root, f.timestamp)?;
    // OPEN INTEREST IS OPTIONAL AND ITS ABSENCE IS NOT A LENGTH.
    //
    // A spot index has none, so the descriptor leaves the name `None` and no
    // array is looked for. `None` here therefore reports the same length as
    // every other column rather than a zero, which would read as a column that
    // arrived empty — a different fact, and the wrong one.
    // THE VALUES TOO, NOT ONLY THE LENGTH — this read the `.len()` and threw
    // the column away, which is why a negative open interest had no guard at
    // all on this path.
    let open_interest_column: &[serde_json::Value] = match f.open_interest {
        Some(name) => array_at(root, name)?,
        None => &[],
    };
    let open_interest = match f.open_interest {
        Some(_) => open_interest_column.len(),
        None => quartet[0].len(),
    };
    // EVERY COLUMN IS CHECKED HERE, BEFORE THE MASK EXISTS, and that ordering
    // is load-bearing rather than tidy.
    //
    // The mask filters with `zip`, which stops at the shorter side. A column
    // LONGER than the mask would therefore be silently trimmed to fit and the
    // disagreement would vanish — every array the same length, and a window of
    // well-formed bars assembled from rows that never lined up. That is the
    // exact failure `RawWindow::decode`'s length check exists to catch, and
    // masking first would have walked around it.
    //
    // The refusal is `LengthDisagreement` and not something local, because that
    // variant names all six lengths at once. Two numbers cannot say which
    // column is the odd one out.
    let rows = quartet[0].len();
    let disagrees = quartet.iter().any(|column| column.len() != rows)
        || volume.len() != rows
        || timestamp.len() != rows
        || open_interest != rows;
    if disagrees {
        return Err(FetchError::LengthDisagreement {
            open: quartet[0].len(),
            high: quartet[1].len(),
            low: quartet[2].len(),
            close: quartet[3].len(),
            volume: volume.len(),
            timestamp: timestamp.len(),
            open_interest,
        });
    }
    let mut keep = Vec::with_capacity(rows);
    let mut skipped = 0usize;
    let mut negative = 0usize;
    let mut interest = 0usize;
    for i in 0..rows {
        let traded = quartet
            .iter()
            .all(|column| column.get(i).is_some_and(|v| !v.is_null()));
        if !traded {
            skipped = skipped.saturating_add(1);
            keep.push(false);
            continue;
        }
        // A NEGATIVE VOLUME SKIPS ITS ROW RATHER THAN REFUSING THE WINDOW.
        //
        // The sign is read here, off the JSON, because `one_volume` can only
        // answer with an `Err` and an `Err` in a column `map` refuses all of it.
        // That is what the measured `-125` on `ADANIENT` cost: one row killed a
        // 90-day window, the window's failure ended the backfill at request 1
        // of 21, and losing 1-minute lost all eight intraday rungs with it,
        // because 2/3/5/10/15/30/60 are rolled up locally from it.
        //
        // **The refusal was right about the value and wrong about its reach.**
        // A volume counts shares traded and a negative one is not a quantity —
        // D-0323 stands. What changes is the granularity: a skipped ROW is a
        // legal gap in an append-only month, because bars need only be strictly
        // increasing; a skipped CHUNK is not, because `Header::advance` refuses
        // any batch beginning at or before what is committed, which is why the
        // chunk loop's suffix discard must stay exactly as it is.
        //
        // AN INDEX IS NOT FILTERED HERE and that is P-60, not an oversight.
        // Its volume column has no referent at all — measured across 6,493
        // stored BANKNIFTY minute bars, the only distinct value is `0` — so
        // `one_volume` records the zero the column always is and counts it. An
        // equity's negative means shares DID trade and the decoder is reading
        // the wrong column, so its row carries no usable quantity and goes.
        let quantity_is_impossible = listing != crate::vendor::Listing::Index
            && volume.get(i).is_some_and(|v| {
                v.as_i64().is_some_and(|n| n < 0) || v.as_f64().is_some_and(|n| n < 0.0)
            });
        // AND AN OPEN INTEREST THAT IS NOT A COUNT EITHER.
        //
        // An open interest is contracts outstanding: never negative, exactly as
        // a volume is never negative. D-0323 gave the volume its guard and left
        // this field one column over with none, so `open_interest: -5` decoded,
        // landed, and died a crate later at `survey` as `ImpossibleCount` —
        // against a batch index that names no vendor row.
        //
        // **`i64::MIN` IS EXEMPT AND MUST STAY EXEMPT.** §7 spends that value on
        // "the vendor sent no open interest", so a vendor sending it literally
        // is a SENTINEL COLLISION, not a bad count — `one_number` refuses it by
        // name, loudly, and skipping the row here would swallow the one case
        // that needs to be shouted about.
        //
        // THE `match` IS NOT A STYLE CHOICE. Written as
        // `as_i64().is_some_and(..).unwrap_or_else(|| as_f64()..)` the sentinel
        // falls straight through: `as_i64()` answers `Some(i64::MIN)`, the
        // predicate correctly says "not impossible", and the fallback then asks
        // `as_f64()`, which answers `-9.22e18` and says "negative" — so the row
        // is skipped and the loud refusal never fires. An existing test caught
        // exactly that. The integer spelling, when there IS one, is the whole
        // answer; `as_f64` is only for a value that is not an integer at all.
        let interest_is_impossible =
            open_interest_column
                .get(i)
                .is_some_and(|v| match v.as_i64() {
                    Some(n) => n < 0 && n != i64::MIN,
                    None => v.as_f64().is_some_and(|n| n < 0.0),
                });
        if quantity_is_impossible {
            negative = negative.saturating_add(1);
        } else if interest_is_impossible {
            interest = interest.saturating_add(1);
        }
        keep.push(!quantity_is_impossible && !interest_is_impossible);
    }
    Ok(Kept {
        keep,
        null_bars: skipped,
        negative_volume: negative,
        negative_open_interest: interest,
    })
}

/// What [`kept_rows`] decided, and what it had to skip to decide it.
///
/// A struct rather than a tuple because it grew to four fields and
/// `(Vec<bool>, usize, usize, usize)` at a call site says nothing about which
/// count is which — and these three counts get three different messages.
struct Kept {
    /// One flag per row: whether it is a bar this build can store.
    keep: Vec<bool>,
    /// Rows skipped because a price was null — the vendor reporting no trade.
    null_bars: usize,
    /// Rows skipped because the volume was negative on a traded listing.
    negative_volume: usize,
    /// Rows skipped because the open interest was negative.
    ///
    /// Never counts `i64::MIN`, which is the null sentinel and is refused by
    /// name in [`one_number`] rather than skipped.
    negative_open_interest: usize,
}

/// Rows whose four prices cannot be a bar, dropped from every column at once.
///
/// Returns how many were dropped.
///
/// # Why this is here and not at the store's write boundary
///
/// It **is** at the write boundary as well — `Bar::ohlc_is_sane` is what
/// `survey` calls, and it is the reason `BANKNIFTY: batch record 7075 has
/// impossible OHLC` was ever printed. The trouble is where that leaves an
/// operator. By the time `survey` sees the batch the rows have been session
/// filtered and folded, so `7075` is an index into a vector that no longer
/// corresponds to a vendor row, a timestamp, or a file anyone can open. The
/// message names a fault and nothing that can be acted on.
///
/// Measured: **six consecutive runs** refused on `NIFTY` record 5997 and
/// `BANKNIFTY` record 7075, month 2021-08, ~4½ minutes apart. The vendor
/// returns the same bytes every time, so a deterministic refusal was retried
/// forever and the month never landed.
///
/// Here the check runs while the window is still the vendor's own rows, one
/// pass, O(1) per row — and it drops the row instead of the batch, so the other
/// ~8,000 bars in that chunk survive.
///
/// # The relationship, and why the sign is checked too
///
/// The same predicate `Bar::ohlc_is_sane` uses: the high is at or above all
/// three others, the low at or below open and close, and none of the four is
/// negative. `prices` already refuses a negative price on the JSON path, so the
/// sign clause is redundant there and load-bearing on the archive path, whose
/// decoder parses a leading minus and never checks it.
///
/// # Why every column, including the ones it does not read
///
/// A row is a bar. Dropping index `i` from the prices and not from `volume` or
/// `timestamp` would reassemble every later bar out of one row's prices and the
/// next row's stamp — the same misalignment `kept_rows` builds a shared mask to
/// avoid. `open_interest` is filtered only when it is present, because an empty
/// vector there means the vendor sends none at all, which is not a length.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
fn drop_impossible_bars(arrays: &mut ParallelArrays) -> usize {
    let rows = arrays.open.len();
    let mut keep = Vec::with_capacity(rows);
    let mut dropped = 0usize;
    for i in 0..rows {
        // A COLUMN SHORTER THAN `open` KEEPS ITS ROW. `RawWindow::decode` owns
        // the length disagreement and names all seven at once; answering it
        // here with a silent drop would erase the very evidence it reports.
        let sane = match (
            arrays.open.get(i),
            arrays.high.get(i),
            arrays.low.get(i),
            arrays.close.get(i),
        ) {
            (Some(&open), Some(&high), Some(&low), Some(&close)) => {
                high >= open
                    && high >= low
                    && high >= close
                    && low <= open
                    && low <= close
                    && open >= 0
                    && high >= 0
                    && low >= 0
                    && close >= 0
            }
            _ => true,
        };
        if !sane {
            dropped = dropped.saturating_add(1);
        }
        keep.push(sane);
    }
    if dropped == 0 {
        return 0;
    }
    // ONE MASK, APPLIED THE SAME WAY TO EVERY COLUMN. `retain` visits in order,
    // so a fresh counter per column reads the same verdict for the same row; a
    // shared iterator would be consumed by the first column and let the rest
    // through untouched, which is the misalignment this whole function exists
    // to prevent.
    let filter = |column: &mut Vec<i64>| {
        let mut nth = 0usize;
        column.retain(|_| {
            let live = keep.get(nth).copied().unwrap_or(true);
            nth = nth.saturating_add(1);
            live
        });
    };
    filter(&mut arrays.open);
    filter(&mut arrays.high);
    filter(&mut arrays.low);
    filter(&mut arrays.close);
    filter(&mut arrays.volume);
    filter(&mut arrays.timestamp);
    // OPEN INTEREST ONLY WHEN THE VENDOR SENDS IT. An empty vector here means
    // the descriptor names no column at all, which is not a length that can
    // disagree — filtering it would leave it empty and filtering it wrongly
    // would invent one.
    if !arrays.open_interest.is_empty() {
        filter(&mut arrays.open_interest);
    }
    dropped
}

/// One column, filtered through the mask [`kept_rows`] computed.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] when the column is a different length from
/// the mask. That is a real disagreement between columns, and it is named here
/// rather than left to surface later as a row count nobody can account for.
fn kept<'a>(
    column: &'a [serde_json::Value],
    keep: &'a [bool],
) -> impl Iterator<Item = &'a serde_json::Value> + use<'a> {
    // NO LENGTH CHECK HERE, AND THAT IS WHY [`kept_rows`] RUNS FIRST.
    //
    // `zip` stops at the shorter side, so a column longer than the mask would
    // be trimmed to fit and the disagreement would disappear — every array the
    // same length, and a window of well-formed bars assembled from rows that
    // never lined up. Checking here would also have only two numbers to report,
    // and `LengthDisagreement` names all seven at once. So every column is
    // verified before a mask exists, and this is a pure filter.
    column
        .iter()
        .zip(keep.iter())
        .filter_map(|(v, k)| k.then_some(v))
}

/// One count, whatever shape carried it.
///
/// # `i64::MIN` IS THE NULL SENTINEL, SO IT IS NOT A COUNT THIS CAN CARRY
///
/// `CLAUDE.md` §7 makes `i64::MIN` the open-interest **null** and zero a real
/// zero. [`crate::fetch::land`] writes that sentinel for a field the vendor did
/// not send — `row.open_interest.unwrap_or(i64::MIN)` — so a vendor that sends
/// the literal `-9223372036854775808` in its open-interest column used to decode
/// here as an ordinary number, satisfy every check below, land on disk, and read
/// back as *no open interest at all*. The reinterpretation was silent in both
/// directions: nothing said the number had been swallowed, and nothing said the
/// null had been invented.
///
/// It is refused HERE, at the vendor boundary, for the same reason a negative
/// price is refused in [`one_price`]: this is the last place the field's name
/// and the vendor's own JSON are both still in hand. By the append it is one
/// `i64` among millions with nothing left to say where it came from.
///
/// **Refused for every field this function reads, not only open interest.** A
/// volume of `i64::MIN` is not a volume and a timestamp of `i64::MIN` is not a
/// time, so one rule costs nothing and is cheaper to keep true than three. What
/// it costs: no field read through here can ever carry that one value. The
/// sentinel already spent it.
///
/// The second arm below cannot produce it — it divides by 100, and no `i64`
/// divided by 100 is `i64::MIN`. There is no check there because there is no
/// reachable branch to check, and an unreachable branch is a coverage hole with
/// a comment on it.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the field and the value.
fn one_number(v: &serde_json::Value, name: &str) -> Result<i64, FetchError> {
    if let Some(n) = v.as_i64() {
        if n == i64::MIN {
            return Err(FetchError::TransportFailed {
                detail: format!(
                    "{name:?} holds {v}, which is the value this store reserves \
                     for a field the vendor did NOT send (CLAUDE.md §7: \
                     i64::MIN is the open-interest null and zero means zero). \
                     Stored, it would read back as an absence rather than as \
                     the number that arrived, so it is refused here where the \
                     vendor's own value is still visible."
                ),
            });
        }
        return Ok(n);
    }
    let refuse = || FetchError::TransportFailed {
        detail: format!("{name:?} holds {v}, which is not a whole number"),
    };
    let number = v.as_number().ok_or_else(refuse)?;
    let hundredths = crate::csv::paisa(&number.to_string()).ok_or_else(refuse)?;
    if hundredths % 100 == 0 {
        Ok(hundredths / 100)
    } else {
        Err(refuse())
    }
}

/// One VOLUME, which counts shares traded and is therefore never negative.
///
/// # Why this is a sibling of [`one_number`] and not a clause inside it
///
/// [`one_number`] reads three fields and only one of them has zero as a floor.
///
/// * `open_interest` has exactly one legal negative — [`store::format::OI_NULL`],
///   which is `i64::MIN` and which [`one_number`] already refuses on its own
///   grounds.
/// * **`timestamp` has a legal negative RANGE**, and this is the one that would
///   have bitten. [`crate::session::IstMoment::from_epoch_secs`] adds
///   `IST_OFFSET_SECS` — 19,800 — before testing the sign, so every epoch second
///   in `-19_800..0` is a real IST moment on 1970-01-01 and is *accepted*. An
///   `n < 0` test inside [`one_number`] would refuse a stamp the session filter
///   admits, which is a behaviour change with no source behind it.
///
/// So the floor lives beside the field that has one.
///
/// # Why here and not at the store
///
/// [`store::format::Bar::counts_are_sane`] is the authority and already says
/// `volume >= 0`. It runs in `store::file::survey`, one crate and ~1,500 lines
/// away, and by then the value is one `i64` among millions with nothing left to
/// say where it came from. That is the same argument [`one_price`] makes for a
/// negative price and [`one_number`] makes for the null sentinel — and it was
/// simply never made for volume. **An omission, not a decision:** `one_price`'s
/// own refusal text already names the failure mode verbatim — *"a stored
/// negative passes every ordering check downstream, which is why it is refused
/// here rather than written"* — one function above the field that did not check.
///
/// Measured: a Dhan response carried `volume: -95342` on ADANIENT and died as
/// `StoreError::ImpossibleCount` at batch record 2333, taking the whole
/// instrument-month, its seven derived rungs, and every later month in the same
/// batch with it. The operator's message named a batch index that maps to
/// nothing they can open, and printed the open-interest null beside it as a
/// second suspect. D-0323.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the field and the value.
fn one_volume(
    v: &serde_json::Value,
    name: &str,
    listing: crate::vendor::Listing,
    corrected: &mut usize,
) -> Result<i64, FetchError> {
    let n = one_number(v, name)?;
    if n >= 0 {
        return Ok(n);
    }
    // AN INDEX IS NOT TRADED, SO ITS VOLUME COLUMN HAS NO REFERENT.
    //
    // NIFTY and BANKNIFTY are computed weighted averages. No share of an index
    // changes hands, so there is no quantity for this column to carry — and
    // measured across 6,493 stored BANKNIFTY minute bars, the ONLY value Dhan
    // ever sends here is `0`. Operator-confirmed: index volume is normally
    // zero, and a zero volume is legitimate for a thinly traded stock too.
    //
    // Against that, `-2` and `-125` are not a quantity being lost. They are
    // noise in a column that is structurally empty, and refusing the window
    // over them cost the whole 90-day chunk: measured, BANKNIFTY's minute
    // backfill stopped dead at chunk 1 of 21 and NIFTY's at chunk 3, with every
    // later month left absent while the OHLC in those chunks was perfectly
    // good.
    //
    // So for an index the value is recorded as the zero the column always is —
    // which is not a substitution, because there is nothing to substitute for —
    // and it is COUNTED. `CLAUDE.md` §4: degrade loudly and name the reason.
    // The caller emits one event per window carrying the count, so the
    // correction reaches `/logs` rather than being absorbed.
    //
    // ANY OTHER LISTING STILL REFUSES. On an equity a negative volume means
    // shares DID trade and the decoder is reading the wrong column, so zero
    // would be a lie and the refusal is the whole of D-0323. The rule turns on
    // whether the column can carry a quantity at all, never on the sign alone.
    if listing == crate::vendor::Listing::Index {
        *corrected = corrected.saturating_add(1);
        return Ok(0);
    }
    Err(FetchError::TransportFailed {
        detail: format!(
            "{name:?} holds {n}, and a volume counts shares traded: it is never \
             negative, and zero means zero (CLAUDE.md §7). The store refuses \
             this at the append, where the field is one i64 among millions and \
             the vendor's body is gone — so it is refused here, where both are \
             still in hand."
        ),
    })
}

/// The one object this vendor's bar fields are read from.
///
/// # The search this replaces could build a bar out of two objects
///
/// Every field used to resolve itself: look at the top level, and failing that
/// look inside **each** value one level down for a key of that name, taking the
/// first hit. The intent was to tolerate a vendor that wraps its payload in a
/// `data` object, and in a body with exactly one such object it did.
///
/// In a body with two it silently spliced them. `{"live":{...},"cached":{...}}`
/// — or a primary beside a fallback, or two exchanges in one answer — resolves
/// `open` against whichever object `serde_json` yields first and `close` against
/// whichever holds a key of that name, and those need not be the same object.
/// The seven-array length check downstream cannot see it: two objects describing
/// the same window hold the same number of bars, so the lengths agree and a
/// window of bars that were never quoted together lands on disk looking exactly
/// like a real one. `serde_json`'s default map is sorted, so which object won
/// was decided by *alphabetical order of the wrapper keys* — stable, and
/// stably wrong.
///
/// So there is no search. [`crate::vendor`]'s `envelope` says where the bars
/// are, one container is resolved from it here, and all seven fields come out of
/// that container.
///
/// # A descriptor that is wrong says so
///
/// `envelope` for the brokers in this build is **UNVERIFIED against a live
/// body** — no vendor has been reached from this process yet (see the module
/// header). If it is wrong, the refusal below names the key that was expected
/// and lists the keys that were actually there, which is a one-row diff to fix.
/// Guessing instead is what produced the splice.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] when the declared envelope is absent.
fn container<'a>(
    root: &'a serde_json::Value,
    envelope: Option<&'static str>,
) -> Result<&'a serde_json::Value, FetchError> {
    let Some(key) = envelope else {
        return Ok(root);
    };
    root.get(key).ok_or_else(|| FetchError::TransportFailed {
        detail: format!(
            "the descriptor says this vendor hangs its bars under {key:?}, and \
             the answer has no such key. It has: {}",
            keys_of(root)
        ),
    })
}

/// Whether an answer of this many bytes is more than this build will hold.
///
/// **A function rather than an inline comparison, so the boundary is testable.**
/// Written in place, `text.len() > MAX_RESPONSE_BYTES` is a comparison whose
/// `>=` and `==` mutants can only be killed by a test that allocates 64 MiB —
/// which is a test nobody should write and which therefore never got written.
/// Split out, the same boundary is three assertions and no allocation at all.
///
/// Its argument is now the count [`body_within`] returns, which is the body's
/// length when it fitted and a lower bound on it when it did not — so the
/// question is the same one and the answer no longer requires holding the
/// answer. One socket test does move a body past the cap end to end, because
/// *where* the check now runs is not something an arithmetic assertion can
/// prove; the arithmetic assertions below still own the boundary itself.
#[must_use]
const fn too_large(len: usize) -> bool {
    len > MAX_RESPONSE_BYTES
}

/// The most of a **refusal** body this build will take off the socket.
///
/// [`trim`] already cuts what reaches the error to 500 characters, but it can
/// only cut a `String` that has already been built — so a vendor answering 500
/// with a gigabyte cost a gigabyte of memory to produce half a kilobyte of log.
/// The read stops here instead. Eight kibibytes is far more than any broker's
/// JSON error object, and it is two orders of magnitude more than `trim` will
/// keep, so nothing an operator would have read is lost.
///
/// Deliberately **not** [`MAX_RESPONSE_BYTES`]: that cap is sized for a window
/// of bars, and a refusal is a sentence.
const MAX_REFUSAL_BYTES: usize = 8 * 1024;

/// As much of a vendor's refusal as belongs in an error.
///
/// A refusal body is unbounded input from outside, and an error string is a
/// thing that reaches a log — so it is cut, at characters rather than bytes so
/// the cut cannot land inside one.
fn trim(body: &str) -> String {
    body.chars().take(500).collect()
}

/// Reads a body a frame at a time, keeping at most `cap` bytes of it.
///
/// Returns what was kept and **how many bytes were seen** — which is the body's
/// true length when it fitted, and a lower bound on it when it did not, because
/// the read stops rather than continuing to measure something it has already
/// refused. [`too_large`] applied to the second value is therefore the same
/// question the old `text().len()` check asked, answered before the memory is
/// committed rather than after.
///
/// # Why not [`reqwest::Response::text`]
///
/// `text()` reads the WHOLE body and then hands it over. Every size check
/// written after it is a check on memory already spent: honest about the number
/// and useless about the cost. A vendor — or anything answering on the vendor's
/// address — that replies to a one-day request with a stream that does not end
/// took the process down long before the comparison ran. `Content-Length` is
/// checked before this is called, but that is a CLAIM the host may omit and may
/// get wrong; these are the bytes, and the bytes are the fact.
///
/// # What this bounds, and what it does not
///
/// Memory: the kept buffer never exceeds `cap`, because a frame that would
/// carry it past is appended only as far as the room left. Peak is `cap` plus
/// whatever single frame hyper hands over, which the sender does not choose.
///
/// It does **not** bound time. An endless stream of zero-length data frames
/// grows nothing and returns nothing; what ends that is the client's own
/// [`REQUEST_TIMEOUT_SECS`], which `reqwest` applies to the body read as well as
/// to the connect. There is no second timer here and this comment is the whole
/// of the claim about it.
///
/// Decoded lossily, which is what `text()` does for a body that declares no
/// charset. JSON is UTF-8 by definition, so a replacement character means a
/// malformed body — and that becomes a decode refusal one call later, naming
/// what could not be read.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] if a frame never arrives.
async fn body_within(
    answer: &mut reqwest::Response,
    cap: usize,
) -> Result<(String, usize), FetchError> {
    let mut kept: Vec<u8> = Vec::new();
    let mut seen: usize = 0;
    while let Some(frame) = answer
        .chunk()
        .await
        .map_err(|why| FetchError::TransportFailed {
            detail: format!("the answer could not be read: {why}"),
        })?
    {
        // SATURATING, because `seen` is a count of bytes from outside and
        // `overflow-checks` is on in both profiles: a wrapped total would be a
        // plausible small number that passes the cap, which is the one failure
        // this whole function exists to remove.
        seen = seen.saturating_add(frame.len());
        let room = cap.saturating_sub(kept.len());
        if frame.len() > room {
            // AS MUCH AS FITS, THEN STOP. Dropping the whole frame instead
            // would leave a refusal body empty whenever the vendor sent it in
            // one piece — the operator would lose the sentence that explains
            // the failure, which is the only reason a refusal body is read.
            kept.extend_from_slice(frame.get(..room).unwrap_or_default());
            break;
        }
        kept.extend_from_slice(&frame);
    }
    Ok((String::from_utf8_lossy(&kept).into_owned(), seen))
}

/// Everything a [`FetchError::VendorRefused`] says beyond its status number.
///
/// Called only when the status is not a success, and it consumes as much of the
/// body as [`MAX_REFUSAL_BYTES`] allows — so a caller must not read the body
/// again afterwards. There is nothing left to read.
async fn refusal_words(
    answer: &mut reqwest::Response,
    names: Option<&crate::refusal::ErrorNames>,
) -> (String, Option<crate::refusal::Disposition>) {
    // A REDIRECT IS A REFUSAL, SO IT HAS TO SAY SO IN WORDS.
    //
    // The client does not follow one (see `HttpSource::new`), which means a 3xx
    // arrives here instead of silently becoming a request to another host
    // carrying the credential. Its body is almost always empty, so without this
    // the operator would get `302` and nothing else. The `Location` is named —
    // it is the vendor's own routing, not a secret — and the credential is not,
    // because it never appears in anything this function can reach.
    let hint = answer.status().is_redirection().then(|| {
        let target = answer
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map_or_else(
                || "no Location header".to_owned(),
                |v| v.chars().take(200).collect(),
            );
        format!(
            "this build does not follow redirects, because the credential \
             travels in a header no HTTP client knows to strip. The vendor \
             wanted to send this request to: {target}"
        )
    });
    // A REFUSAL BODY IS UNBOUNDED INPUT TOO, AND IT USED TO BE READ WHOLE.
    //
    // This was `answer.text()`, so the cut `trim` performs ran on a `String`
    // that had already been materialised at whatever size the far end chose —
    // and on this path there was no size check at all, only the one on the
    // success path. A 500 carrying a gigabyte was a gigabyte held to print 500
    // characters of it.
    //
    // `unwrap_or_default` for the same reason `text()` carried one: a refusal
    // whose body could not be read is still a refusal, and the status is the
    // part the rate governor acts on.
    let (body, seen) = body_within(answer, MAX_REFUSAL_BYTES)
        .await
        .unwrap_or_default();
    // AND A CUT BODY SAYS IT WAS CUT. Otherwise an operator reads 500
    // characters of a vendor's error and cannot tell whether the sentence that
    // would have explained it was on line 6 or line 6,000. This does not fire
    // for any refusal a broker actually sends; it fires for the answers that
    // are not really refusals.
    let said = if seen > MAX_REFUSAL_BYTES {
        format!(
            "{} […] and it ran past {MAX_REFUSAL_BYTES} bytes, which is as much \
             of a refusal as this build reads",
            trim(&body)
        )
    } else {
        trim(&body)
    };
    // THE CLASSIFICATION IS TAKEN FROM `body`, NOT FROM `said`.
    //
    // `said` is `trim`med to 500 characters, and a Kite envelope orders its
    // keys `status`, `message`, `error_type` — so a vendor message long enough
    // pushes the name past the cut. Reading it here, from as much of the body
    // as this build kept, is the difference between classifying the answer and
    // classifying a rendering of it.
    //
    // `None` all the way through when the feed declares no contract: the whole
    // `and_then` chain is skipped and nothing is parsed at all, which is what
    // keeps this free for every feed that has no error page read for it.
    //
    // READ THROUGH THE WHOLE CONTRACT, NOT THE CODE ALONE. This chained
    // `named_error_of` with `contract.read`, which sees only the code — and a
    // vendor can misfile its own code. Dhan answered HTTP 400 with `DH-906`
    // ("Order Error", per its annexure) carrying `"errorMessage":"Invalid
    // Token"`, and the code's faithful reading told the run its REQUEST was
    // wrong, so the credential was never re-read. `disposition_of` reads both
    // keys in one parse and a contract declaring `message: None` takes the
    // code-only path byte for byte. D-0325.
    let verdict = names.and_then(|contract| crate::refusal::disposition_of(&body, contract));
    let words = match hint {
        Some(why) if body.is_empty() => why,
        Some(why) => format!("{why} — and it said: {said}"),
        None => said,
    };
    (words, verdict)
}

/// The keys of a JSON object, for a refusal that tells an operator what to fix.
///
/// Names only — **never a value**, because a value here is vendor data and a
/// refusal is a string that reaches a log.
fn keys_of(value: &serde_json::Value) -> String {
    value.as_object().map_or_else(
        || "nothing — the answer is not an object".to_owned(),
        |o| {
            let names: Vec<&str> = o.keys().map(String::as_str).collect();
            if names.is_empty() {
                "no keys at all".to_owned()
            } else {
                names.join(", ")
            }
        },
    )
}

/// The array a field names, **in the one container the descriptor chose**.
///
/// No fallback and no second place to look: see [`container`] for what looking
/// in a second place cost.
fn array_at<'a>(
    root: &'a serde_json::Value,
    name: &str,
) -> Result<&'a Vec<serde_json::Value>, FetchError> {
    let Some(array) = root.get(name).and_then(serde_json::Value::as_array) else {
        return Err(FetchError::TransportFailed {
            detail: format!(
                "the vendor's answer has no array named {name:?} where the \
                 descriptor says the bars are. It has: {}",
                keys_of(root)
            ),
        });
    };
    Ok(array)
}

/// THE LIVE DISCOVERY ADAPTER — the one `impl` `crate::chain` was written to
/// stay independent of.
///
/// # Why it belongs here and not in `chain`
///
/// This is the only type in the crate that owns a `reqwest::Client` and an
/// assembled credential. `chain` names a port and opens no socket, so every arm
/// of it is drivable from a test; the socket is here, in the module that
/// already had one, and adding it cost no new field and no second client.
///
/// # It is governed like every other request
///
/// A discovery GET spends the vendor's budget exactly as a bars POST does —
/// same account, same per-second ceiling — so it takes a permit first and
/// reports the outcome after. Skipping that would let a contract sweep, which
/// issues one request per expiry, outrun the ceiling the bars path respects.
impl HttpSource {
    /// One POST with a JSON body, answered as text.
    ///
    /// # Why this is separate from `window_async`
    ///
    /// That one builds its body from an `HttpSpec`'s parameter map, which is
    /// the shape a BARS request takes. Dhan's expired-options endpoint takes a
    /// different body entirely — a cadence, an ordinal, a strike offset and a
    /// side — built by `crate::rolling::body`. Reusing the bars builder would
    /// mean teaching the parameter map a second grammar it is not for.
    ///
    /// The credential, the extra headers, the rate permit and the status
    /// handling are the same, and those are what this shares.
    ///
    /// # Errors
    ///
    /// [`crate::chain::Refusal`], carrying the vendor's status where there was
    /// one and `None` where nothing answered.
    ///
    /// **This used to say the caller "records them and does not branch on
    /// them", and returned two `String`s.** That was true when it was written
    /// and it is what left the rolling path with no retry: the status existed
    /// only inside the sentence `"the vendor answered 429 Too Many Requests"`,
    /// so a caller wanting to know whether the refusal was worth re-asking had
    /// to search prose for a number. The bars path was given a structured status
    /// for exactly that reason — `FetchError::VendorRefused` carries
    /// `status: u16` — and this one was left behind.
    ///
    /// # Cost
    ///
    /// One request, one permit. O(1). Unchanged — the type carries a number the
    /// function had already computed.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub async fn post_json(
        &self,
        url: &str,
        body: String,
    ) -> Result<String, crate::chain::Refusal> {
        use crate::chain::Refusal;

        self.wait_for_permit().await;
        let (name, value) = self.header();
        let mut builder = self.client.post(url);
        for (header, word) in self.spec.extra_headers {
            builder = builder.header(*header, *word);
        }
        // NOTHING ANSWERED, SO THERE IS NO STATUS TO CARRY — a dropped socket, a
        // DNS failure or a timeout. The caller's ladder sizes a blip differently
        // from a backend that answered 500, and only this arm can say which.
        let answer = builder
            .header(name, value)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|why| Refusal::transport(format!("{why}")))?;

        let status = answer.status();
        if let Some(lock) = self.governor.as_ref() {
            let mut g = lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if status.as_u16() == 429 {
                g.record_throttled();
            } else if status.is_success() {
                g.record_success();
            }
        }
        if !status.is_success() {
            // THE VENDOR'S OWN STATUS, for the reason `Discovery::get` gives
            // one line away: a 401 and a 429 mean different things to an
            // operator, and collapsing them reads a throttle as a dead token.
            //
            // CARRIED AS A NUMBER BESIDE THE SENTENCE, which is what makes the
            // rolling path's retry decidable at all.
            return Err(Refusal::answered(
                status.as_u16(),
                format!("the vendor answered {status}"),
            ));
        }
        // A FAILED BODY READ IS A TRANSPORT FAILURE, NOT A REFUSAL. The status
        // was already a success; what failed is the socket delivering the rest.
        let body = answer
            .text()
            .await
            .map_err(|why| Refusal::transport(format!("{why}")))?;
        self.keep_first(crate::capture::Method::Post, url, &body);
        Ok(body)
    }

    /// Record this answer if the capture budget has room.
    ///
    /// **Between the read and the parse, deliberately.** Everything downstream
    /// of here interprets the body; a capture taken after that would record
    /// this build's reading of the vendor rather than the vendor, which is the
    /// circularity `crate::capture` exists to break.
    ///
    /// It cannot fail the request: [`crate::capture::record`] counts a failed
    /// write and returns `None`. The root is resolved per capture rather than
    /// held, because the budget makes that at most
    /// `FEED_COUNT * 2 * PER_SLOT` resolutions for the life of the process and
    /// a field would have to be threaded through every constructor for nothing.
    fn keep_first(&self, method: crate::capture::Method, url: &str, body: &str) {
        let Some(feed) = self.feed else {
            return;
        };
        if crate::capture::kept(feed, method) >= crate::capture::PER_SLOT {
            return;
        }
        let Some(root) =
            crate::capture::root_or_refused(feed, method.word(), crate::folder::root())
        else {
            return;
        };
        drop(crate::capture::record(&root, feed, method, url, body));
    }
}

impl crate::chain::Discovery for HttpSource {
    async fn get(&self, url: &str) -> Result<String, crate::chain::Refusal> {
        use crate::chain::Refusal;

        self.wait_for_permit().await;
        let (name, value) = self.header();
        let mut builder = self.client.get(url);
        for (header, word) in self.spec.extra_headers {
            builder = builder.header(*header, *word);
        }
        // NOTHING ANSWERED, SO THERE IS NO STATUS TO CARRY. A refused `send` is
        // a dropped socket, a DNS failure or a timeout, and `Refusal::transport`
        // is the arm that says so — the caller's retry ladder sizes a blip
        // differently from a backend that answered 500.
        let answer = builder
            .header(name, value)
            .send()
            .await
            .map_err(|why| Refusal::transport(format!("{why}")))?;

        let status = answer.status();
        if let Some(lock) = self.governor.as_ref() {
            let mut g = lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if status.as_u16() == 429 {
                g.record_throttled();
            } else if status.is_success() {
                g.record_success();
            }
        }
        if !status.is_success() {
            // THE VENDOR'S OWN STATUS, NOT A PARAPHRASE. A 401 here and a 429
            // here mean different things to an operator -- one is a credential
            // and one is a pace -- and collapsing them is how a throttled sweep
            // gets read as an expired token.
            //
            // AND IT IS NOW CARRIED AS A NUMBER BESIDE THE SENTENCE. The
            // sentence was the only copy, so a caller wanting to know whether
            // this was retryable had to search prose for `429` — which is the
            // coupling `api::server::with_retry` refuses by name on the bars
            // path, and the reason discovery had no retry ladder at all.
            return Err(Refusal::answered(
                status.as_u16(),
                format!("the vendor answered {status}"),
            ));
        }
        // THE BODY READ IS A TRANSPORT FAILURE, NOT A REFUSAL. The status was
        // already a success; what failed is the socket delivering the rest of
        // it, which is a blip and is ladder-eligible as one.
        let body = answer
            .text()
            .await
            .map_err(|why| Refusal::transport(format!("{why}")))?;
        // THE DISCOVERY CALLS ARE THE ONES WITH NO FIXTURE AT ALL. Groww's
        // expiries and contracts answers are parsed by field names taken from
        // its documentation and never from an observed response.
        self.keep_first(crate::capture::Method::Get, url, &body);
        Ok(body)
    }
}

impl BarSource for HttpSource {
    fn window(&self, _request: &BarRequest) -> Result<RawWindow, FetchError> {
        // A blocking `window` over an async client needs a runtime, and this
        // build has no async ingest path to hand one down. Rather than spin a
        // runtime per call — which would be a new thread pool per instrument —
        // the synchronous entry point is refused by name and `window_async` is
        // the one that works. Stated rather than silently blocking.
        Err(FetchError::TransportFailed {
            detail: "HttpSource is asynchronous: call `window_async` from a \
                     runtime. The blocking seam would need a runtime per call, \
                     which is a thread pool per instrument."
                .to_owned(),
        })
    }
}

impl HttpSource {
    /// Fetches one window from the vendor.
    ///
    /// # Errors
    ///
    /// [`FetchError::VendorRefused`] carrying the HTTP status — so a 429, which
    /// is the rate governor's business, is distinguishable from a 5xx, which is
    /// not. [`FetchError::TransportFailed`] if the socket never produced an
    /// answer, or the answer was too large, or it did not decode.
    pub async fn window_async(&self, request: &BarRequest) -> Result<RawWindow, FetchError> {
        let (name, value) = self.header();
        let from = Self::on_the_wire(request.window.from(), self.spec.date_format);
        // THE RUNG'S OWN RANGE END. `range_end_for` is the feed's for every
        // rung that shares an endpoint, and the rung's where the vendor
        // documents the two differently — see `vendor::RungRoute::range_end`
        // for what one shared field would send on Dhan's minute request.
        let to = Self::wire_end(
            request.window.to(),
            self.spec.range_end_for(request.granularity),
            self.spec.date_format,
        )?;

        // RESOLVED AFTER `from` AND `to`, because a path segment can carry
        // either. A feed whose window is in the path is not this build's today,
        // and the resolver is the same one the query string uses precisely so
        // that it could be tomorrow without a second grammar.
        let url = self.url(request, &from, &to)?;

        // THE REQUEST IS BUILT FROM THE DESCRIPTOR ROW, NOT WRITTEN HERE.
        //
        // This was `json!({ "fromDate": from, "toDate": to })` and a two-pair
        // query — a window and nothing else. No instrument, no segment, no
        // interval. That is why Dhan answered `DH-905 securityId is required`
        // and why no broker has ever returned a bar to this build.
        //
        // Every field now comes from `spec.params`, so adding a vendor stays a
        // row in `crate::vendor` and this function never learns either broker's
        // spelling. An empty `params` still sends the window alone, which is
        // what a feed that needs nothing else would want and is also exactly
        // the old behaviour — so the shape did not change, only where it is
        // decided.
        //
        // COLLECTED THROUGH A `Result`, because one of the five sources can
        // refuse: a feed that spells its bar length in a request field has no
        // word for a rung nobody has recorded one for, and the request must not
        // go out naming a different length than the answer will be filed under.
        //
        // THE RUNG'S OWN FIELDS COME AFTER THE SHARED ONES, appended rather
        // than merged: `rung_routes` carries what ONE endpoint takes and the
        // other does not, so a shared list cannot hold it without sending it to
        // both. Empty for every feed that serves its rungs from one URL.
        let pairs: Vec<(&'static str, String)> = self
            .spec
            .params
            .iter()
            .chain(
                self.spec
                    .route_for(request.granularity)
                    .map_or([].as_slice(), |r| r.params),
            )
            .map(|p| Ok((p.name, self.resolve_param(p, request, &from, &to)?)))
            .collect::<Result<_, FetchError>>()?;

        let mut builder = match self.spec.method {
            Method::Post => {
                let body: serde_json::Map<String, serde_json::Value> = pairs
                    .iter()
                    .map(|(n, v)| ((*n).to_owned(), serde_json::Value::String(v.clone())))
                    .collect();
                self.client
                    .post(&url)
                    .json(&serde_json::Value::Object(body))
            }
            Method::Get => self.client.get(&url).query(&pairs),
        };
        // Headers this feed requires beyond the credential — Groww's
        // `X-API-VERSION`, which its own page shows on every historical call.
        for (header, word) in self.spec.extra_headers {
            builder = builder.header(*header, *word);
        }

        // `mut` because the body is now read frame by frame rather than in one
        // `text()` call — see `body_within` for why the size check cannot come
        // after the whole answer is already in memory.
        // THE BUDGET IS SPENT BEFORE THE SOCKET IS OPENED, never after. Asking
        // permission afterwards would already have made the request the ceiling
        // exists to prevent.
        self.wait_for_permit().await;

        let mut answer = builder.header(name, value).send().await.map_err(|why| {
            FetchError::TransportFailed {
                // `why` is reqwest's own words and never carries the header we
                // set, so the token cannot reach this string.
                detail: format!("{url} was not reached: {why}"),
            }
        })?;

        let status = answer.status().as_u16();
        note_answer(&url, status, answer.status().is_success(), request, &pairs);

        // THE FEEDBACK HALF, AND IT IS WHAT MAKES THE GOVERNOR ADAPTIVE RATHER
        // THAN A FIXED CEILING. A clean answer tightens every span back toward
        // the declared bound; a throttle relaxes them all multiplicatively and
        // says so at `Warn`, because a run that slowed behind a vendor's refusal
        // must not look merely slow.
        //
        // 429 IS THE ONLY STATUS READ AS RATE. Treating every refusal as a
        // throttle would back off for a bad credential or a malformed window and
        // hide the real cause behind an ever-slower run.
        //
        // THE SUCCESS HALF IS DEFERRED UNTIL THE BODY HAS BEEN READ, and that
        // is not tidiness. A vendor can answer HTTP 200 and put its refusal in
        // the body — Dhan's own SDK example tests `response["status"] ==
        // "failure"` and never reads the HTTP status at all. Recording success
        // here told the governor a failed call had succeeded and RAISED the
        // allowance on it. See `answered_with_a_refusal` below.
        if status == 429
            && let Some(lock) = self.governor.as_ref()
        {
            let mut g = lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            g.record_throttled();
        }

        if !answer.status().is_success() {
            // Lifted into its own function, and NOT for tidiness: reading the
            // refusal body under a bound (rather than whole, which is what this
            // used to do) took `window_async` past `clippy::too_many_lines`.
            // The behaviour is unchanged by the move and `refusal_words` states
            // what it is.
            let (detail, named) = refusal_words(&mut answer, self.spec.error_names).await;
            return Err(FetchError::VendorRefused {
                status,
                detail,
                named,
            });
        }

        // THE CLAIM FIRST, AND THEN THE BYTES.
        //
        // `Content-Length` is a courtesy the host may decline to offer and may
        // get wrong, so it can never be the check that binds — but when it IS
        // offered and it already names more than this build will hold, reading
        // the body to discover that costs exactly what the bound exists to
        // prevent. These are the same three lines as
        // `resolve::HttpDocuments::get_async`, against this module's own cap.
        if let Some(declared) = answer.content_length() {
            let cap = MAX_RESPONSE_BYTES as u64;
            if declared > cap {
                return Err(FetchError::TransportFailed {
                    detail: format!(
                        "the vendor declares {declared} bytes and this build \
                         accepts at most {cap}. Refused before the body was read."
                    ),
                });
            }
        }

        // AND THE BYTES ARE THE FACT. This was `text()` followed by
        // `too_large(text.len())` — a comparison made only once the whole answer
        // was in memory, which reports the cost rather than avoiding it. An
        // answer that declares no length at all (chunked, or closed-delimited)
        // never met the check above, so on that shape the late one was the only
        // one there was. The boundary is unchanged; only when it is applied is.
        let (text, seen) = body_within(&mut answer, MAX_RESPONSE_BYTES).await?;
        if too_large(seen) {
            return Err(FetchError::TransportFailed {
                detail: format!(
                    "the vendor's answer ran past {MAX_RESPONSE_BYTES} bytes — \
                     at least {seen} arrived before the read was abandoned, and \
                     the rest was left on the socket. This build accepts at most \
                     {MAX_RESPONSE_BYTES}."
                ),
            });
        }

        // A REFUSAL UNDER A 200, WHICH BYPASSED EVERY GUARD THIS MODULE HAS.
        //
        // `refusal_words` is reached only from the `!is_success()` branch, so a
        // vendor that answers 200 with `{"errorCode":"DH-901", …}` produced no
        // `Disposition` at all: no `SessionDead`, no `NotEntitled`, no
        // `Throttled`. `api::server::step` never saw a named refusal, the
        // credential was never re-read, and the ladder never ran. The body then
        // failed to decode and surfaced as `BodyNotUnderstood` — a *decode*
        // fault, which is deliberately not retryable — so a dead token
        // mid-backfill read as an unreadable body while the run carried on.
        //
        // And it was worse than silent: the old `record_success` above fired on
        // any 2xx, so every one of those failures RAISED the vendor allowance.
        //
        // Dhan's own SDK example in `Dhan Docs/21-errors.md` tests
        // `response["status"] == "failure"` and never reads the HTTP status.
        // D-0325 already measured this vendor misfiling its error metadata; this
        // is the same vendor putting a refusal under the wrong status.
        //
        // Read through the WHOLE contract, so a code the vendor misfiled is
        // still read by its sentence — the rule D-0325 established.
        self.weigh_answered_body(&text, status)?;

        // NAMED AS A DECODE FAULT, BECAUSE THE EXCHANGE ALREADY SUCCEEDED.
        //
        // Everything below this line is reading bytes that arrived. Left as
        // `TransportFailed` it read as "the vendor was not reached" — false,
        // and it sent `with_retry` through its whole ladder re-asking for bytes
        // that will come back identical. See `FetchError::BodyNotUnderstood`.
        decode_body(&text, &self.spec, request.listing).map_err(|why| {
            // THE INNER SENTENCE, NOT THE INNER ERROR'S WHOLE DISPLAY.
            //
            // `decode_body` reports its faults as `TransportFailed`, whose
            // `Display` opens *"the vendor was not reached"*. Wrapping that
            // whole string inside `BodyNotUnderstood`'s *"the vendor answered
            // and the body was not readable"* produced a sentence that
            // contradicts itself in the same breath, and it is in the
            // operator's own log:
            //
            //   the vendor answered and the body was not readable:
            //   the vendor was not reached: "volume" holds -2
            //
            // Both halves cannot be true, and the reader has no way to tell
            // which one to act on. Taking the detail alone keeps the part that
            // names the fault and drops the claim that was never right.
            let detail = match why {
                FetchError::TransportFailed { detail } => detail,
                // Any other variant is already a sentence about a body, so
                // its own `Display` is the honest one.
                other => other.to_string(),
            };
            // AND KEEP THE BODY THAT DEFEATED US, BEFORE THE SENTENCE REPLACES
            // IT.
            //
            // The measured case is the whole argument. Dhan sent
            // `volume: -125` for `ADANIENT`; the refusal cost that instrument
            // every intraday rung it had; and afterwards nothing could say what
            // `-125` WAS — a genuine value, a wrapped `int32`, or a column read
            // at the wrong offset. Three causes wanting three different
            // responses, and the evidence to separate them was already gone:
            // the audit journal is a fixed-stride record that keeps the URL and
            // a message and never the payload.
            //
            // So the body is written here, where it is still in hand, and the
            // sentence goes beside it rather than instead of it.
            self.keep_unreadable(&url, &text, &detail);
            FetchError::BodyNotUnderstood { detail }
        })
    }

    /// Record a body this build could not read, if the feed's budget has room.
    ///
    /// The mirror of [`Self::keep_first`] and bounded the same way — see
    /// [`crate::capture::record_unreadable`] for why it is a separate budget
    /// and why it can never fail the request. This one matters more on that
    /// last point, not less: the caller is already returning a failure, and
    /// replacing the operator's reason with one about the disk would be
    /// `CLAUDE.md` §4 pointing the wrong way.
    fn keep_unreadable(&self, url: &str, body: &str, why: &str) {
        let Some(feed) = self.feed else {
            return;
        };
        if crate::capture::unread_kept(feed) >= crate::capture::PER_SLOT {
            return;
        }
        let Some(root) = crate::capture::root_or_refused(feed, "unreadable", crate::folder::root())
        else {
            return;
        };
        drop(crate::capture::record_unreadable(
            &root, feed, url, body, why,
        ));
    }

    /// Whether a 2xx answer is actually a success, and the governor feedback
    /// that follows from the answer.
    ///
    /// # A refusal under a 200 bypassed every guard this module has
    ///
    /// [`refusal_words`] is reached only from the `!is_success()` branch, so a
    /// vendor answering `200` with `{"errorCode":"DH-901", …}` produced no
    /// [`crate::refusal::Disposition`] at all — no `SessionDead`, no
    /// `NotEntitled`, no `Throttled`. `api::server::step` never saw a named
    /// refusal, the credential was never re-read, and the ladder never ran. The
    /// body then failed to decode and surfaced as
    /// [`FetchError::BodyNotUnderstood`], which is deliberately NOT retryable —
    /// so a dead token mid-backfill read as an unreadable body while the run
    /// carried on.
    ///
    /// And it was worse than silent: `record_success` fired on any 2xx, so every
    /// one of those failures RAISED the vendor allowance.
    ///
    /// Dhan's own SDK example in `Dhan Docs/21-errors.md` tests
    /// `response["status"] == "failure"` and never reads the HTTP status.
    /// D-0325 measured this vendor misfiling its error metadata; this is the
    /// same vendor putting a refusal under the wrong status.
    ///
    /// **Read through the whole contract**, so a code the vendor misfiled is
    /// still read by its sentence — the rule D-0325 established.
    ///
    /// A feed declaring no `error_names` has no contract to read and takes the
    /// success path unchanged, exactly as it did before this existed.
    ///
    /// # Errors
    ///
    /// [`FetchError::VendorRefused`] carrying the disposition the body named, so
    /// the caller's ladder sees the same verdict it would have seen had the
    /// vendor used the status.
    fn weigh_answered_body(&self, text: &str, status: u16) -> Result<(), FetchError> {
        if let Some(contract) = self.spec.error_names
            && let Some(named) = crate::refusal::disposition_of(text, contract)
        {
            // THE GOVERNOR LEARNS THE RIGHT THING FROM IT. A throttle named in
            // the body is still a throttle; any other refusal is one the
            // allowance has nothing to say about, so it is neither raised nor
            // narrowed.
            if named == crate::refusal::Disposition::Throttled
                && let Some(lock) = self.governor.as_ref()
            {
                let mut g = lock
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                g.record_throttled();
            }
            return Err(FetchError::VendorRefused {
                status,
                detail: format!(
                    "the vendor answered {status} and put a refusal in the \
                     body: {text}"
                ),
                named: Some(named),
            });
        }
        // ONLY NOW IS IT A SUCCESS. The status was 2xx and the body carries no
        // refusal this vendor's contract can name, so the additive increase is
        // earned rather than assumed.
        if let Some(lock) = self.governor.as_ref() {
            let mut g = lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            g.record_success();
        }
        Ok(())
    }
}

/// One array per bar, read by POSITION: `[ts, open, high, low, close, volume]`
/// and optionally open interest seventh.
///
/// # There are no names here, so the width is the contract
///
/// `decode_objects` reads each field by the name the descriptor gives. This
/// vendor sends no names at all — Groww's page says "in that order" — so a row
/// that is the wrong length is refused naming BOTH lengths rather than read
/// short. A six-element row read as seven would take `null` for volume; a
/// seven read as six would silently drop open interest. Neither is a thing to
/// discover later from a chart that looks nearly right.
///
/// # Errors
///
/// [`FetchError::TransportFailed`] naming the bar index and what was found,
/// for a container that is not the named array, a row that is not an array, a
/// row of an unusable width, or a cell that is not the number it must be.
fn decode_positional(
    root: &serde_json::Value,
    spec: &HttpSpec,
    envelope: Option<&'static str>,
    array: &'static str,
    listing: crate::vendor::Listing,
    corrected: &mut usize,
) -> Result<RawWindow, FetchError> {
    let container = container(root, envelope)?;
    let rows = array_at(container, array)?;

    let mut arrays = ParallelArrays {
        open: Vec::with_capacity(rows.len()),
        high: Vec::with_capacity(rows.len()),
        low: Vec::with_capacity(rows.len()),
        close: Vec::with_capacity(rows.len()),
        volume: Vec::with_capacity(rows.len()),
        timestamp: Vec::with_capacity(rows.len()),
        // RESERVED LIKE ITS SIX SIBLINGS. `Vec::new()` was harmless while
        // nothing pushed to this column; the push landed today, once per kept
        // row, so it grew by doubling with a memcpy at every step while the
        // other six did not. `docs/07-o1-architecture.md` law 2 — reserve the
        // bound up front, and it is the same bound.
        open_interest: Vec::with_capacity(rows.len()),
    };

    let mut null_bars = 0usize;
    for (i, row) in rows.iter().enumerate() {
        let cells = row.as_array().ok_or_else(|| FetchError::TransportFailed {
            detail: format!("bar {i} is {row}, and this vendor sends one ARRAY per bar"),
        })?;
        // Six is the deprecated endpoint, seven the live one, and the seventh
        // is open interest. Any other width is a shape this build has not seen.
        if cells.len() != 6 && cells.len() != 7 {
            return Err(FetchError::TransportFailed {
                detail: format!(
                    "bar {i} carries {} cell(s); this vendor sends six \
                     (timestamp, open, high, low, close, volume) or seven with \
                     open interest last",
                    cells.len()
                ),
            });
        }
        let cell = |at: usize| -> Result<&serde_json::Value, FetchError> {
            cells.get(at).ok_or_else(|| FetchError::TransportFailed {
                detail: format!("bar {i} has no cell {at}"),
            })
        };
        // A BAR WITH A NULL PRICE IS SKIPPED, NOT FATAL.
        //
        // Groww answers for an equity with `null` in a price cell on a minute
        // that did not trade. This returned `Err` on the first one, so ONE
        // untraded minute killed the whole instrument — measured on a real
        // 785-instrument pull: 24 reached, 761 refused, every refusal reading
        // `"open" holds null`. The vendor was answering correctly and the
        // decoder was throwing the answer away.
        //
        // `volume` was ALREADY tolerant of null a few lines below, mapping it
        // to 0. A null volume genuinely is zero — nothing traded. A null PRICE
        // is not zero, it is absent, and zero-filling it writes a lie this
        // store cannot tell from a real price afterwards. So the bar goes
        // rather than the value.
        //
        // ALL FOUR CHECKED BEFORE ANY IS PUSHED: pushing open and then finding
        // close null would leave the parallel arrays at different lengths,
        // which `RawWindow::decode` refuses with a message about columns that
        // says nothing about the null that caused it.
        if (1..=4).any(|at| cell(at).is_ok_and(serde_json::Value::is_null)) {
            null_bars += 1;
            continue;
        }
        arrays
            .timestamp
            .push(one_stamp(cell(0)?, spec.timestamps, i)?);
        arrays.open.push(one_price(cell(1)?, "open", spec.prices)?);
        arrays.high.push(one_price(cell(2)?, "high", spec.prices)?);
        arrays.low.push(one_price(cell(3)?, "low", spec.prices)?);
        arrays
            .close
            .push(one_price(cell(4)?, "close", spec.prices)?);
        // Volume is `null` on an index, which has none. Zero means zero and the
        // charter's null sentinel is for OPEN INTEREST, not volume, so a null
        // volume becomes 0 rather than i64::MIN.
        // OPEN INTEREST, CELL SIX — AND IT WAS DROPPED ON THE FLOOR.
        //
        // This decoder validated that a row carries six cells or seven, said in
        // its own comment that "the seventh is open interest", read cells 0..=5
        // and never touched cell 6. `arrays.open_interest` was initialised
        // empty above and nothing ever pushed to it, so every bar this shape
        // ever decoded was stored with no open interest at all.
        //
        // It did not matter while only SPOT was pulled: Groww's own note reads
        // "open interest (only for FNO instruments, null for others)", so an
        // index or an equity has none to lose. It matters entirely for expired
        // derivatives, where open interest is one of the two columns that make
        // the series worth holding — and it would have been lost in silence,
        // because a missing column raises nothing. The bars land, the counts
        // are right, the receipt says STORED.
        //
        // PUSHED FOR EVERY KEPT ROW, never conditionally. `RawWindow::decode`
        // checks the seven columns against each other, so a vector filled only
        // on the rows that happened to carry a seventh cell would be short and
        // refused as a shape error — which is a message about columns for a
        // problem about one absent cell. A six-cell row and a null seventh both
        // read as `OI_NULL`, which is the sentinel `CLAUDE.md` §7 names for
        // exactly this and is distinct from a real zero.
        arrays.open_interest.push(match cells.get(6) {
            None | Some(serde_json::Value::Null) => store::format::OI_NULL,
            Some(given) => one_number(given, "open_interest")?,
        });
        arrays.volume.push(match cell(5)? {
            serde_json::Value::Null => 0,
            given => one_volume(given, "volume", listing, corrected)?,
        });
    }

    // NOT SILENT. A window that was mostly untraded minutes is something the
    // operator has to know: it is not an error, and it is not a full answer
    // either. Once per window rather than once per bar — 375 lines of "skipped"
    // is noise and one count is information.
    if null_bars > 0 {
        // The event carries the fact to the FILE; the print carries it to a
        // terminal. See the sibling site above, and gate 22.
        let _noted = telemetry::emit(
            &telemetry::Event::new(
                telemetry::Level::Warn,
                "pull.decode",
                "bars carried a null price and were skipped",
            )
            .with("skipped", u64::try_from(null_bars).unwrap_or(u64::MAX))
            .with("bars", u64::try_from(rows.len()).unwrap_or(u64::MAX)),
        );
        eprintln!(
            // "in those intervals", not "in those minutes": this decoder is
            // rung-blind by design and a daily pull comes through it too.
            "brutex: {null_bars} of {} bars carried a null price and were skipped \
             — the vendor reported no trade in those intervals",
            rows.len()
        );
    }

    // AND THE SECOND DOOR. See the note on the object shape's call.
    note_impossible_bars(drop_impossible_bars(&mut arrays), rows.len());
    RawWindow::decode(&arrays)
}

/// One timestamp cell, in whichever spelling this feed uses.
///
/// A number is taken as it stands; a string is parsed as a local date and time.
/// `fetch::land` then shifts an IST-based value back to UTC — this function's
/// only job is to turn the wire into seconds, and it never guesses which.
fn one_stamp(
    v: &serde_json::Value,
    encoding: crate::vendor::TimestampEncoding,
    at: usize,
) -> Result<i64, FetchError> {
    use crate::vendor::TimestampEncoding as T;
    match encoding {
        T::EpochSecondsUtc | T::EpochMillisUtc => one_number(v, "timestamp"),
        T::IstDateTimeText | T::IsoDateTimeText => {
            let text = v.as_str().ok_or_else(|| FetchError::TransportFailed {
                detail: format!("bar {at} stamps {v}, and this feed spells its timestamps as text"),
            })?;
            local_seconds(text).ok_or_else(|| FetchError::TransportFailed {
                detail: format!(
                    "bar {at} stamps {text:?}, which is not YYYY-MM-DD followed by HH:MM:SS"
                ),
            })
        }
        // THE ZONE IS APPLIED HERE, AND THAT IS WHY THIS ARM IS SEPARATE.
        //
        // The two arms above hand `land` a LOCAL time and `land` converts it.
        // This one hands back true UTC, because only here is the offset
        // visible — and `land` has a matching arm that passes it through. The
        // pair is asserted by
        // `pull::vendor::a_zone_carrying_stamp_is_converted_once`.
        T::IsoDateTimeOffset => {
            let text = v.as_str().ok_or_else(|| FetchError::TransportFailed {
                detail: format!("bar {at} stamps {v}, and this feed spells its timestamps as text"),
            })?;
            let local = local_seconds(text).ok_or_else(|| FetchError::TransportFailed {
                detail: format!(
                    "bar {at} stamps {text:?}, which is not YYYY-MM-DD followed by HH:MM:SS"
                ),
            })?;
            let offset = stated_offset(text).ok_or_else(|| FetchError::TransportFailed {
                detail: format!(
                    "bar {at} stamps {text:?}, and this feed's timestamps CARRY their own \
                     UTC offset — expected a trailing +HHMM, -HHMM, +HH:MM, -HH:MM or Z. \
                     Refused rather than read as IST: a missing offset defaulted to +0530 \
                     would shift every bar of this window by five and a half hours and \
                     store cleanly."
                ),
            })?;
            local
                .checked_sub(offset)
                .ok_or_else(|| FetchError::TransportFailed {
                    detail: format!("bar {at} stamps {text:?}, whose offset overflows the epoch"),
                })
        }
    }
}

/// The UTC offset a timestamp states, in seconds, or `None` when it states none.
///
/// Accepts the four spellings a vendor may write and `Z` for zero. Kite writes
/// `+0530`; the colon form and `Z` are the other shapes ISO-8601 permits for the
/// same fact, and reading them costs nothing while refusing them would be a
/// refusal of a correct value.
///
/// Returns `None` for anything else — including a bare local time with no
/// offset at all, which is the case that must NOT silently become IST.
///
/// A fixed number of byte comparisons on a suffix. No allocation.
fn stated_offset(text: &str) -> Option<i64> {
    let tail = text.get(19..)?;
    if tail == "Z" {
        return Some(0);
    }
    let sign = match tail.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    // `+0530` and `+05:30` differ only by the colon, so the digits are read by
    // position from a form with it stripped rather than by two parsers.
    let digits: String = tail
        .get(1..)?
        .chars()
        .filter(|c| *c != ':')
        .collect::<String>();
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hours: i64 = digits.get(0..2)?.parse().ok()?;
    let minutes: i64 = digits.get(2..4)?.parse().ok()?;
    // BOTH FIELDS ARE BOUNDED, and the hours one was not.
    //
    // A minutes field past 59 is not a zone, it is a malformed value, and
    // reading it as an hour and a bit would invent an offset nobody stated. The
    // same is true of the hours, and leaving it open was worse: `+9900` parsed
    // to 99 hours and shifted a bar FOUR DAYS, landing it on a plausible
    // session minute of a different day — a value that stores rather than
    // refuses. No zone on earth is past 14 hours from UTC (Line Islands,
    // +14:00), so anything beyond that is a malformed value wearing a zone.
    if hours > 14 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 3_600 + minutes * 60))
}

/// A resolved value, borrowed back unchanged, if it can be a URL path segment.
///
/// # The allowlist is RFC 3986's unreserved set, and the choice is deliberate
///
/// `A-Z a-z 0-9 - . _ ~` are the characters that mean themselves in a path and
/// need no escaping under any reading. Everything else refuses — including the
/// sub-delims a permissive reading would allow, because "a vendor probably
/// tolerates `$` in a path" is a claim about a vendor and there is no source
/// for it. Kite's `instrument_token` is a run of digits and passes; its index
/// tradingsymbol `NIFTY 50` has a space and does not, which is the case this
/// exists for.
///
/// **Empty refuses too, and that is the important arm.** An empty segment
/// collapses `/historical//minute` into a path the vendor may route somewhere
/// else entirely, and it is exactly what an instrument with no vendor id would
/// produce — the `DH-905 securityId is required` shape, one layer up.
///
/// A walk of one value whose length the master already bounds at
/// `core::vendor::VENDOR_ID_CAPACITY`. No allocation: the happy path hands the
/// caller its own borrow back.
fn path_safe<'a>(value: &'a str, placeholder: &'static str) -> Result<&'a str, FetchError> {
    // THE DOT-SEGMENTS ARE REFUSED, AND THE UNRESERVED SET ALONE DOES NOT DO
    // IT. `.` is unreserved, so a character-wise allowlist admits both `.` and
    // `..` — and RFC 3986 §5.2.4 says those two are not ordinary segments at
    // all. `..` REMOVES the segment before it, so an instrument id resolving to
    // `..` turns `/instruments/historical/../minute` into
    // `/instruments/minute`: a different endpoint, requested with a live
    // credential, whose answer would be filed as bars for the instrument that
    // was asked for. `.` merely elides itself, which is quieter and no more
    // correct.
    //
    // Refused as WHOLE segments rather than by banning the character, because
    // a dot inside a symbol is ordinary and refusing `BAJAJ.NS` would be a
    // refusal of a correct value.
    if value == "." || value == ".." {
        return Err(FetchError::PathSegmentUnusable {
            placeholder,
            value: value.to_owned(),
        });
    }
    let usable = !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'));
    if usable {
        Ok(value)
    } else {
        Err(FetchError::PathSegmentUnusable {
            placeholder,
            value: value.to_owned(),
        })
    }
}

/// `YYYY-MM-DD?HH:MM:SS` to seconds since the epoch, treating the value as
/// local. The separator is a `T` on Groww's live endpoint and a space on its
/// deprecated one — and its documentation says space for both — so this accepts
/// either rather than believing the annotation.
fn local_seconds(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> { text.get(from..to)?.parse().ok() };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, sec) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    // RANGE-CHECKED, AND THEY WERE NOT. `Day::new` below validates the date
    // half and nothing validated the clock half at all, so `33:99:99` parsed
    // and was arithmetically folded into the following day: hour 33 is
    // 24 + 9, which lands the bar at 09:xx on the NEXT calendar day. A bar
    // stamped on a day the vendor never sent one for is not a rejected row —
    // it is a row filed under the wrong date, and the store is append-only, so
    // it is filed there permanently.
    //
    // `?` and not a clamp: `CLAUDE.md` §4 refuses a fallback that hides a
    // failure, and the caller already treats `None` as an unreadable timestamp.
    // A leap second would be `60`, which no vendor here sends and which this
    // deliberately refuses rather than silently rounds.
    if h > 23 || mi > 59 || sec > 59 {
        return None;
    }
    let day = crate::session::Day::new(
        u16::try_from(y).ok()?,
        u8::try_from(mo).ok()?,
        u8::try_from(d).ok()?,
    )
    .ok()?;
    let days = i64::from(day.days_from_epoch());
    Some(days * 86_400 + h * 3_600 + mi * 60 + sec)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {

    /// A descriptor's budget reaches the source that enforces it.
    ///
    /// The defect this pins is that the two existed side by side and never met:
    /// every `Descriptor` carried a `Budget`, `rate::Governor` enforced budgets
    /// correctly with its own tests, and `Governor::new` was never called on the
    /// path that opens a socket. A budgeted feed whose source holds no governor
    /// is a ceiling that is decoration.
    #[test]
    fn a_budgeted_feed_builds_a_source_that_holds_a_governor() {
        for feed in [crate::vendor::Feed::Dhan, crate::vendor::Feed::Groww] {
            let crate::vendor::Transport::Http(spec) = feed.descriptor().transport else {
                panic!("{feed:?} is an HTTP feed");
            };
            assert!(
                spec.budget.per_second.is_some(),
                "{feed:?} declares a ceiling"
            );
            let source = HttpSource::new(spec, Credential::token("t".to_owned()))
                .expect("a budgeted feed builds");
            assert!(
                source.governor.is_some(),
                "{feed:?} declares a budget, so its source must enforce one"
            );
        }
    }

    /// The governor holds the DESCRIPTOR'S numbers, never a default.
    ///
    /// A governor built from the wrong ceiling is worse than none: it enforces a
    /// bound nothing in this tree wrote down.
    #[test]
    fn the_governor_carries_the_descriptors_own_ceilings() {
        let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("Dhan is an HTTP feed");
        };
        let source = HttpSource::new(spec, Credential::token("t".to_owned())).expect("Dhan builds");
        let held = source.governor.as_ref().expect("Dhan is budgeted");
        let g = held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(
            g.ceiling(crate::rate::WindowSpan::Second),
            spec.budget.per_second
        );
        assert_eq!(g.ceiling(crate::rate::WindowSpan::Day), spec.budget.per_day);
    }

    /// The permit DENIES past the ceiling, and names a wait the caller can sleep.
    ///
    /// This is the assertion that makes the wiring worth having. `admit` is
    /// driven directly rather than through a socket: the clock is an argument to
    /// the governor precisely so a test can hold time still, and holding it
    /// still is what proves a ceiling is a ceiling rather than a suggestion.
    #[test]
    fn the_permit_denies_once_the_second_is_spent_and_names_the_wait() {
        let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("Dhan is an HTTP feed");
        };
        let per_second = spec.budget.per_second.expect("Dhan declares one");
        let source = HttpSource::new(spec, Credential::token("t".to_owned())).expect("Dhan builds");
        let held = source.governor.as_ref().expect("Dhan is budgeted");
        let mut g = held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // TIME DOES NOT MOVE, so every admission lands in one second's window.
        let now = 1_700_000_000_000_000_u64;
        for i in 0..per_second {
            assert!(
                matches!(g.admit(now), crate::rate::Verdict::Admit),
                "request {i} is inside the ceiling of {per_second}"
            );
        }
        match g.admit(now) {
            crate::rate::Verdict::Admit => {
                panic!("request {} in one second was admitted", per_second + 1)
            }
            // A DENY ASKING FOR NO WAIT WOULD SPIN in `wait_for_permit`.
            crate::rate::Verdict::Deny { wait_micros, .. } => {
                assert!(
                    wait_micros > 0,
                    "a denial names a wait the caller can sleep"
                );
            }
        }
    }

    /// A SHARED governor is charged by the CALLER, and never charged twice.
    ///
    /// # What the second charge cost
    ///
    /// `admit` is a WITHDRAWAL, not a question: the verdict it answers with has
    /// already spent the permit. Once `api` handed its governor to this source
    /// -- so that one instance per vendor would see every request -- BOTH sides
    /// went on calling it for a single request: `api::server::await_budget`
    /// before the call, and `wait_for_permit` as the socket opened. One request,
    /// two permits, against a ceiling written for one.
    ///
    /// Measured 2026-08-20, journal seq 1956:
    /// `pull.spot instrument refused -- Dhan's second budget is spent. The next
    /// request is admitted in 0.013s.` The api's wait is BOUNDED at sixty-four
    /// attempts and this one is not, so the bounded side is the one that ran
    /// out of patience, and its refusal reached the browser as HTTP 502.
    ///
    /// # Why the witness is the cursor and not the clock
    ///
    /// The first draft of this test spent the ceiling and asserted that a
    /// shared source still RETURNED QUICKLY. It passed -- and it passed with
    /// the guard removed too, because the unguarded path simply slept out the
    /// remainder of the second and finished inside the timeout anyway. A
    /// surviving mutant is a missing test (`CLAUDE.md` §4), so the assertion
    /// moved off timing altogether.
    ///
    /// `Governor::admit` is the only thing that moves `cursor_micros`, and it
    /// moves it forward only. The cursor therefore answers the exact question
    /// this test is about -- **was the governor asked at all** -- with no sleep,
    /// no ceiling to exhaust, and no race against the second rolling over.
    #[tokio::test]
    async fn a_shared_governor_is_charged_by_the_caller_and_not_again_here() {
        let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("Dhan is an HTTP feed");
        };
        // ONE GOVERNOR, TWO SOURCES. `owned` keeps the instance `new` built for
        // it; `shared` is handed that same instance, which is precisely what
        // the server does. The flag is the only difference between them.
        let owned = HttpSource::new(spec, Credential::token("t".to_owned())).expect("Dhan builds");
        let held = std::sync::Arc::clone(owned.governor.as_ref().expect("Dhan is budgeted"));
        let shared = HttpSource::new(spec, Credential::token("t".to_owned()))
            .expect("Dhan builds")
            .sharing(Some(std::sync::Arc::clone(&held)));

        assert!(
            !owned.charged_by_caller,
            "a source that built its own governor is the one that spends it"
        );
        assert!(
            shared.charged_by_caller,
            "a source handed a governor leaves the spending to whoever handed it over"
        );

        // THE CURSOR IS THE WITNESS. Only `admit` moves it, so it moves if and
        // only if the governor was actually asked for a permit.
        let cursor_of = |held: &std::sync::Arc<std::sync::Mutex<crate::rate::Governor>>| {
            held.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .cursor_micros()
        };

        let untouched = cursor_of(&held);
        shared.wait_for_permit().await;
        assert_eq!(
            untouched,
            cursor_of(&held),
            "a shared source asks the governor for nothing -- its caller withdrew already"
        );

        // AND THE OTHER HALF, WHICH IS WHAT KEEPS THIS FROM BEING VACUOUS. If
        // `wait_for_permit` charged nobody at all, the assertion above would
        // hold for entirely the wrong reason. A source that owns its governor
        // must still move that cursor.
        owned.wait_for_permit().await;
        assert!(
            cursor_of(&held) > untouched,
            "a source that owns its governor is still the one that spends it"
        );
    }

    /// A throttle lowers the allowance; clean answers raise it again.
    ///
    /// The incremental/decremental behaviour, asserted as a NUMBER rather than
    /// trusted: after a 429 the governor permits strictly fewer than its
    /// ceiling, and success must not leave it there forever.
    #[test]
    fn a_throttle_lowers_the_allowance_and_success_raises_it_again() {
        let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("Dhan is an HTTP feed");
        };
        let ceiling = spec.budget.per_second.expect("Dhan declares one");
        let source = HttpSource::new(spec, Credential::token("t".to_owned())).expect("Dhan builds");
        let held = source.governor.as_ref().expect("Dhan is budgeted");
        let mut g = held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        assert_eq!(
            g.permitted(crate::rate::WindowSpan::Second),
            Some(ceiling),
            "it starts at the declared ceiling"
        );

        g.record_throttled();
        let after = g
            .permitted(crate::rate::WindowSpan::Second)
            .expect("a bounded span still reports one");
        assert!(
            after < ceiling,
            "a throttle must lower it: {after} !< {ceiling}"
        );

        // The step size is the governor's business; what is asserted is that
        // success moves it UPWARD, which is what makes this incremental rather
        // than a one-way ratchet down.
        for _ in 0..64 {
            g.record_success();
        }
        let recovered = g
            .permitted(crate::rate::WindowSpan::Second)
            .expect("bounded");
        assert!(
            recovered > after,
            "success must raise it: {recovered} !> {after}"
        );
    }

    use super::*;
    use crate::vendor::{Budget, FieldNames, Pooling, TimestampEncoding};

    /// D-0143 — the vendor boundary refuses a negative while it can still say
    /// what the vendor actually sent.
    #[test]
    fn a_negative_price_is_refused_where_the_vendor_value_can_still_be_named() {
        // Rupees: `csv::paisa` reads the leading minus happily, because it is a
        // decimal reader and a negative is a real value for the fields that
        // hold one. That is exactly how this used to land.
        let sent = serde_json::json!(-24.5);
        let why = one_price(&sent, "close", PriceScale::Rupees)
            .expect_err("a negative rupee price must not decode");
        let said = why.to_string();
        assert!(
            said.contains("-24.5"),
            "the refusal must quote what the vendor sent, said: {said}"
        );
        assert!(
            said.contains("close"),
            "the refusal must name the field, said: {said}"
        );

        // Paisa: the same rule on the integer path, which never reaches
        // `csv::paisa` and so had no sign check of any kind.
        let raw = serde_json::json!(-2450);
        let why = one_price(&raw, "open", PriceScale::Paisa)
            .expect_err("a negative paisa price must not decode");
        assert!(why.to_string().contains("-2450"));

        // Zero is a price. The boundary is BELOW zero, not at it.
        let zero_p = one_price(&serde_json::json!(0), "low", PriceScale::Paisa);
        assert_eq!(zero_p.expect("zero is a price"), 0);
        let zero_r = one_price(&serde_json::json!(0.0), "low", PriceScale::Rupees);
        assert_eq!(zero_r.expect("zero is a price"), 0);

        // And an ordinary value still decodes on both scales.
        let p = one_price(&serde_json::json!(2450), "high", PriceScale::Paisa);
        assert_eq!(p.expect("an ordinary paisa price"), 2450);
        let r = one_price(&serde_json::json!(24.50), "high", PriceScale::Rupees);
        assert_eq!(r.expect("an ordinary rupee price"), 2450);
    }

    /// A descriptor with only the fields the decoder reads, so a test says what
    /// it is testing. `prices` is the parameter every price case turns on.
    fn spec(prices: PriceScale) -> HttpSpec {
        spec_under(prices, None)
    }

    /// The same descriptor, declaring where its bars hang.
    fn spec_under(prices: PriceScale, envelope: Option<&'static str>) -> HttpSpec {
        HttpSpec {
            response: ResponseShape::ParallelArrays { envelope },
            ..spec_top(prices)
        }
    }

    fn spec_top(prices: PriceScale) -> HttpSpec {
        HttpSpec {
            // The fixture declares no body-level error contract: these tests
            // are about decoding a SUCCESSFUL answer, and `crate::refusal`
            // is exercised against the shipped rows instead.
            error_names: None,
            // The window, named — which is what these tests assert and what
            // the request used to hardcode. A real vendor row carries more
            // (`securityId`, `exchangeSegment`); the fixtures that care about
            // those name them for themselves.
            listings: &[
                crate::vendor::ListingWords {
                    listing: crate::vendor::Listing::Index,
                    segment: "IDX_I",
                    kind: "INDEX",
                },
                crate::vendor::ListingWords {
                    listing: crate::vendor::Listing::Equity,
                    segment: "NSE_EQ",
                    kind: "EQUITY",
                },
            ],
            params: &[
                crate::vendor::Param {
                    name: "from",
                    value: crate::vendor::ParamValue::From,
                },
                crate::vendor::Param {
                    name: "to",
                    value: crate::vendor::ParamValue::To,
                },
            ],
            extra_headers: &[],
            base_url: "https://vendor.invalid",
            bars_path: &[crate::vendor::PathSegment::Literal("bars")],
            rung_routes: &[],
            fno: crate::vendor::FnoAccess::None,
            method: Method::Post,
            auth: crate::vendor::Auth {
                header: "x-token",
                scheme: AuthScheme::Raw,
                key_field: None,
            },
            date_format: DateFormat::DashedYmd,
            range_end: RangeEnd::Exclusive,
            response: ResponseShape::ParallelArrays { envelope: None },
            fields: FieldNames {
                open: "open",
                high: "high",
                low: "low",
                close: "close",
                volume: "volume",
                timestamp: "timestamp",
                open_interest: None,
            },
            timestamps: TimestampEncoding::EpochSecondsUtc,
            prices,
            budget: Budget {
                per_second: None,
                per_minute: None,
                per_day: None,
            },
            // A test spec: no floor, so the window is used as given.
            history_floor: crate::vendor::HistoryFloor::Unstated,
            // No published cap at any rung. The split is not what is under test
            // here, and this fixture never reaches it — `window` is called
            // directly rather than through `api`'s chunking loop.
            window_caps: &[],
            // ONE RUNG SPELLED AND THE REST NOT, which is the shape both
            // shipped brokers are in. The base fixture's params carry no rung
            // field, so this is read only by the tests that add one.
            granularity_tokens: &[(crate::vendor::Granularity::Minute1, "1minute")],
            pooling: Pooling::PerVendor,
        }
    }

    /// **THE 75 PAISE.** This is the regression test for a defect that reached
    /// the tree and would have silently rewritten every fractional price in the
    /// archive.
    ///
    /// The decoder read a price with `f.round()` and handed the result to
    /// `fetch::to_paisa`, which multiplies a rupee figure by 100. So a vendor
    /// quoting `24500.75` produced `24501` rupees, then `2450100` paisa —
    /// **₹24,501.00, and the 75 paise were gone.** No error, no counter, no
    /// drop reason: the bar landed on disk looking exactly like a real one.
    ///
    /// `CLAUDE.md` §7 puts the tick grid at two decimals and the single snap at
    /// the write boundary. Rounding to whole rupees is a snap at the wrong
    /// granularity, in the wrong place.
    #[test]
    fn a_fractional_rupee_price_keeps_its_paise() {
        let body = r#"{
            "open":[24500.75],"high":[24500.75],"low":[24500.75],
            "close":[24500.75],"volume":[250],"timestamp":[1751337900]
        }"#;
        let window = decode_body(
            body,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("decodes");
        let row = &window.rows[0];
        assert_eq!(
            row.open, 2_450_075,
            "24500.75 rupees is 2450075 paisa. 2450100 would be the bug: the \
             price rounded to a whole rupee before the scale was applied."
        );
        assert_eq!(
            (row.high, row.low, row.close),
            (2_450_075, 2_450_075, 2_450_075)
        );
        assert_eq!(row.volume, 250, "a volume is a count and is not scaled");
    }

    /// Every price the paisa grid can hold, held exactly.
    #[test]
    fn every_price_on_the_paisa_grid_survives_intact() {
        for (sent, want) in [
            ("0", 0_i64),      // zero means zero — never a null
            ("0.01", 1),       // one paisa survives
            ("0.1", 10),       // one tenth is TEN paisa, not one
            ("100.4", 10_040), // a single decimal pads on the RIGHT
            ("24500.75", 2_450_075),
            ("99999.99", 9_999_999),
        ] {
            let body = format!(
                "{{\"open\":[{sent}],\"high\":[{sent}],\"low\":[{sent}],\
                  \"close\":[{sent}],\"volume\":[1],\"timestamp\":[1751337900]}}"
            );
            let window = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            )
            .expect("decodes");
            assert_eq!(window.rows[0].open, want, "{sent} rupees is {want} paisa");
        }
    }

    /// **THE FOUR VALUES THAT COST FORTY-TWO RUNS**, each landing on the
    /// exchange's own price.
    ///
    /// These are not invented. They are the literal `open` values Dhan returned
    /// on `/v2/charts/intraday` for security ids 25 and 13, quoted from
    /// `pull.http` / `"vendor refused a window"` events, and every one of them
    /// refused its whole 21-chunk window before this path snapped.
    ///
    /// The right-hand column is what the exchange actually published. Two of
    /// these are provably `f32` artifacts — `35922.6015625 × 256 = 9_196_186`
    /// and `35447.0507812 × 256 = 9_074_445`, both whole numbers, which only a
    /// binary float produces — so the snap is not a loss of precision. It is
    /// the removal of the vendor's rounding error.
    #[test]
    fn the_four_dhan_index_opens_that_refused_now_land_on_the_exchanges_price() {
        for (sent, want, why) in [
            ("35922.6016", 3_592_260_i64, "BANKNIFTY, f32 of 35922.60"),
            ("35447.0508", 3_544_705, "BANKNIFTY, f32 of 35447.05"),
            ("16797.2999", 1_679_730, "NIFTY, f64 of 16797.30"),
            ("16633.2999", 1_663_330, "NIFTY, f64 of 16633.30"),
        ] {
            let body = format!(
                // A FLAT BAR AT THE PROBE, NOT A PROBE BESIDE THREE ONES.
                // These fixtures read `open: 16633.2999, high: 1, low: 1` — an
                // open far above its high and below its low, which
                // `Bar::ohlc_is_sane` refuses and the store would never have
                // accepted. They passed only because nothing between the socket
                // and the append checked the relationship. Now that
                // `drop_impossible_bars` does, the fixture has to be a bar; the
                // probe goes in all four columns, which is a legal flat minute
                // and still asserts exactly what it asserted about `open`.
                "{{\"open\":[{sent}],\"high\":[{sent}],\"low\":[{sent}],\
                  \"close\":[{sent}],\"volume\":[1],\"timestamp\":[1]}}"
            );
            let window = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            )
            .expect("decodes");
            assert_eq!(window.rows[0].open, want, "{sent} is {why}");
        }
    }

    /// Half-up, at the boundary and on both signs.
    ///
    /// `CLAUDE.md` §7 says half-up, and half-up on a NEGATIVE rounds toward
    /// positive infinity — `-14.5` is `-14`, not `-15`. The negative rows are
    /// reachable through this decoder only as a refusal (a price below zero is
    /// rejected two statements later), so they are asserted against the
    /// converter that owns the rule rather than through a body.
    #[test]
    fn the_snap_is_half_up_at_the_boundary_and_on_both_signs() {
        for (sent, want) in [
            ("100.004", 10_000_i64),
            ("100.005", 10_001),
            ("100.006", 10_001),
            ("100.0049999", 10_000),
            ("100.12345", 10_012),
            // `0.005` is the smallest value that rounds UP to a paisa and so is
            // the smallest this decoder accepts. `0.001` rounds to zero and is
            // refused — see `a_price_that_is_not_zero_never_snaps_to_zero`,
            // which owns that rule and the reason for it.
            ("0.005", 1),
            ("100", 10_000),
            ("100.1", 10_010),
            ("100.10", 10_010),
        ] {
            let body = format!(
                // A FLAT BAR AT THE PROBE, NOT A PROBE BESIDE THREE ONES.
                // These fixtures read `open: 16633.2999, high: 1, low: 1` — an
                // open far above its high and below its low, which
                // `Bar::ohlc_is_sane` refuses and the store would never have
                // accepted. They passed only because nothing between the socket
                // and the append checked the relationship. Now that
                // `drop_impossible_bars` does, the fixture has to be a bar; the
                // probe goes in all four columns, which is a legal flat minute
                // and still asserts exactly what it asserted about `open`.
                "{{\"open\":[{sent}],\"high\":[{sent}],\"low\":[{sent}],\
                  \"close\":[{sent}],\"volume\":[1],\"timestamp\":[1]}}"
            );
            let window = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            )
            .expect("decodes");
            assert_eq!(window.rows[0].open, want, "{sent} snaps half-up to {want}");
        }
        // Half-up toward positive infinity on the negative side, asserted where
        // the rule lives — this decoder refuses a negative before it returns.
        for (sent, want) in [("-14.505", -1_450_i64), ("-14.506", -1_451)] {
            let got = brutex_core::price::Paisa::from_rupee_text_half_up(sent)
                .expect("a decimal")
                .raw();
            assert_eq!(got, want, "{sent} rounds toward positive infinity");
        }
    }

    /// **A NON-ZERO PRICE MUST NOT SNAP TO ZERO**, and the hole this closes was
    /// opened by the snap itself.
    ///
    /// Half-up sends everything under half a paisa to zero. `csv::paisa` had
    /// refused those values outright, so the snap silently gained a band —
    /// `[0.00001, 0.005)` and its mirror — where a real vendor value decodes as
    /// a clean `0`. Zero is a LEGAL price here (`Bar::ohlc_is_sane` admits it,
    /// and `every_price_on_the_paisa_grid_survives_intact` pins `"0"` as real),
    /// so nothing downstream could tell the invented zero from a sent one.
    ///
    /// **The negative half is the sharp one.** `-0.001` snaps to `0`, and
    /// `0 < 0` is false, so it walked past the below-zero guard entirely — the
    /// guard that exists to catch a descriptor whose `PriceScale` is wrong. That
    /// diagnostic was unreachable across the whole `(-0.005, 0)` band. Found by
    /// adversarial review of the snap, not by the suite.
    #[test]
    fn a_price_that_is_not_zero_never_snaps_to_zero() {
        for sent in ["0.0001", "0.004", "0.00001", "-0.001", "-0.004", "-0.0049"] {
            let body = format!(
                // A FLAT BAR AT THE PROBE, NOT A PROBE BESIDE THREE ONES.
                // These fixtures read `open: 16633.2999, high: 1, low: 1` — an
                // open far above its high and below its low, which
                // `Bar::ohlc_is_sane` refuses and the store would never have
                // accepted. They passed only because nothing between the socket
                // and the append checked the relationship. Now that
                // `drop_impossible_bars` does, the fixture has to be a bar; the
                // probe goes in all four columns, which is a legal flat minute
                // and still asserts exactly what it asserted about `open`.
                "{{\"open\":[{sent}],\"high\":[{sent}],\"low\":[{sent}],\
                  \"close\":[{sent}],\"volume\":[1],\"timestamp\":[1]}}"
            );
            let Err(FetchError::TransportFailed { detail }) = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            ) else {
                panic!("{sent} is not zero and must not be stored as one")
            };
            assert!(detail.contains("open"), "names the field: {detail}");
        }

        // AND A REAL ZERO IS STILL A REAL ZERO. The rule is about a value the
        // vendor wrote with a non-zero digit, not about the number zero, which
        // this store carries as a price like any other.
        for sent in ["0", "0.0", "0.00", "-0.0"] {
            let body = format!(
                // A FLAT BAR AT THE PROBE, NOT A PROBE BESIDE THREE ONES.
                // These fixtures read `open: 16633.2999, high: 1, low: 1` — an
                // open far above its high and below its low, which
                // `Bar::ohlc_is_sane` refuses and the store would never have
                // accepted. They passed only because nothing between the socket
                // and the append checked the relationship. Now that
                // `drop_impossible_bars` does, the fixture has to be a bar; the
                // probe goes in all four columns, which is a legal flat minute
                // and still asserts exactly what it asserted about `open`.
                "{{\"open\":[{sent}],\"high\":[{sent}],\"low\":[{sent}],\
                  \"close\":[{sent}],\"volume\":[1],\"timestamp\":[1]}}"
            );
            let window = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            )
            .unwrap_or_else(|why| panic!("{sent} is a real zero: {why}"));
            assert_eq!(window.rows[0].open, 0, "{sent} decodes as the zero it is");
        }
    }

    /// A COUNT IS NOT A PRICE AND IS STILL NOT SNAPPED.
    ///
    /// The snap above applies to `open/high/low/close` alone. `one_number`
    /// still routes through [`crate::csv::paisa`], so a fractional share count
    /// remains a shape this build refuses rather than rounds — half a share is
    /// not a rounding error, it is a field that is not what it claims to be.
    #[test]
    fn a_fractional_count_is_still_refused_rather_than_snapped() {
        for bad in ["250.5", "1.25"] {
            let body = format!(
                "{{\"open\":[1],\"high\":[1],\"low\":[1],\"close\":[1],\
                  \"volume\":[{bad}],\"timestamp\":[1]}}"
            );
            let refused = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            );
            assert!(
                matches!(refused, Err(FetchError::TransportFailed { .. })),
                "{bad} is not a count and must not be rounded into one"
            );
        }
    }

    /// A vendor already quoting paisa is taken as is and never scaled twice.
    #[test]
    fn a_paisa_vendor_is_not_scaled_again() {
        let body = r#"{"open":[2450075],"high":[2450075],"low":[2450075],
                       "close":[2450075],"volume":[1],"timestamp":[1751337900]}"#;
        let window = decode_body(
            body,
            &spec(PriceScale::Paisa),
            crate::vendor::Listing::Equity,
        )
        .expect("decodes");
        assert_eq!(window.rows[0].open, 2_450_075, "already paisa, unchanged");
    }

    /// `NaN` must never become a price, and `1e300` must never become
    /// `i64::MAX`. Both were silent under the `as` cast this replaced.
    #[test]
    fn a_price_that_cannot_be_represented_is_refused_rather_than_coerced() {
        for bad in ["1e300", "-1e300"] {
            let body = format!(
                "{{\"open\":[{bad}],\"high\":[1],\"low\":[1],\"close\":[1],\
                  \"volume\":[1],\"timestamp\":[1]}}"
            );
            let refused = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            );
            assert!(
                matches!(refused, Err(FetchError::TransportFailed { .. })),
                "{bad} must be refused, not saturated to i64::MAX"
            );
        }
        // serde_json parses a bare `NaN` as invalid JSON, so the reachable
        // not-a-number case is a string where a number belongs.
        let body = r#"{"open":["x"],"high":[1],"low":[1],"close":[1],
                       "volume":[1],"timestamp":[1]}"#;
        assert!(matches!(
            decode_body(
                body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity
            ),
            Err(FetchError::TransportFailed { .. })
        ));
    }

    /// The seven-array length check is the trap `zip` would have hidden: a
    /// short array must refuse the whole window, not yield a short one.
    #[test]
    fn a_null_price_skips_its_row_rather_than_refusing_the_window() {
        // Three minutes; the middle one did not trade, so the vendor answers
        // `null` for it. This is the shape Dhan and Zerodha answer in.
        let body = r#"{"open":[100.00,null,102.00],"high":[100.50,null,102.50],
                       "low":[99.50,null,101.50],"close":[100.25,null,102.25],
                       "volume":[10,0,12],"timestamp":[1751337900,1751337960,1751338020]}"#;
        let window = decode_body(
            body,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("one untraded minute is not a reason to refuse the other two");

        assert_eq!(window.rows.len(), 2, "the untraded minute is skipped");
        // AND THE SURVIVORS ARE THE RIGHT TWO, not merely the right count. A
        // filter applied to some columns and not others would keep two rows
        // whose fields came from different minutes — a bar that never existed,
        // which is the failure the mask is built once for.
        assert_eq!(window.rows[0].open, 10_000);
        assert_eq!(window.rows[0].close, 10_025);
        assert_eq!(window.rows[0].timestamp, 1_751_337_900);
        assert_eq!(window.rows[1].open, 10_200);
        assert_eq!(window.rows[1].close, 10_225);
        assert_eq!(window.rows[1].timestamp, 1_751_338_020);

        // A NULL IN ANY OF THE FOUR IS ENOUGH. Checking only `open` would let a
        // row through whose close is null, and the columns would then disagree
        // in length — a message about lengths that says nothing about the null.
        for field in ["open", "high", "low", "close"] {
            let body = format!(
                r#"{{"open":[{o}],"high":[{h}],"low":[{l}],"close":[{c}],
                     "volume":[10],"timestamp":[1751337900]}}"#,
                o = if field == "open" { "null" } else { "100.00" },
                h = if field == "high" { "null" } else { "100.50" },
                l = if field == "low" { "null" } else { "99.50" },
                c = if field == "close" { "null" } else { "100.25" },
            );
            let window = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            )
            .unwrap_or_else(|why| panic!("a null {field} skips its row: {why}"));
            assert!(
                window.rows.is_empty(),
                "the only row had a null {field}, so no bar survives"
            );
        }
    }

    /// **THE COLUMNAR SHAPE REFUSED A NULL AND THE OTHER TWO NEVER DID.**
    ///
    /// This is the third time a rule reached two of three decode doors, and
    /// both earlier times the door it missed was the one Dhan answers in —
    /// D-0323 on negative volumes and D-0332 again. Dhan's own field table
    /// marks every response field *Required: No*.
    ///
    /// So the three shapes are asserted to agree, on the same body shape, in
    /// one test. A rule added to one arm and not the others fails here rather
    /// than in a backfill.
    #[test]
    fn all_three_decode_shapes_skip_a_null_price_alike() {
        // The parallel-array shape, which is Dhan's.
        let columnar = r#"{"open":[100.00,null],"high":[100.50,null],
                           "low":[99.50,null],"close":[100.25,null],
                           "volume":[10,0],"timestamp":[1751337900,1751337960]}"#;
        let window = decode_body(
            columnar,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("the columnar shape skips a null row");
        assert_eq!(window.rows.len(), 1, "parallel arrays");

        // The object-per-bar shape, which Groww's row declares.
        let objects = r#"{"candles":[
            {"open":100.00,"high":100.50,"low":99.50,"close":100.25,
             "volume":10,"timestamp":1751337900},
            {"open":null,"high":null,"low":null,"close":null,
             "volume":0,"timestamp":1751337960}]}"#;
        let spec_objects = HttpSpec {
            response: ResponseShape::ArrayOfObjects {
                envelope: Some("candles"),
            },
            ..spec(PriceScale::Rupees)
        };
        let window = decode_body(objects, &spec_objects, crate::vendor::Listing::Equity)
            .expect("the object shape skips a null row");
        assert_eq!(window.rows.len(), 1, "array of objects");
    }

    /// **A ROW WHOSE FOUR PRICES CANNOT BE A BAR IS DROPPED, AND THE WINDOW
    /// SURVIVES — IN EVERY DECODE SHAPE.**
    ///
    /// The store already refuses these: `Bar::ohlc_is_sane` is what printed
    /// `BANKNIFTY: batch record 7075 has impossible OHLC`. Two things were
    /// wrong with catching it only there. It arrives ~1,500 lines downstream,
    /// where the rows have been session-filtered and folded, so `7075` indexes
    /// a vector that maps to no vendor row, no timestamp and no file an
    /// operator can open. And it takes the whole batch with it.
    ///
    /// Measured: **six consecutive runs**, ~4½ minutes apart, refused on
    /// `NIFTY` record 5997 and `BANKNIFTY` record 7075 — the same two records
    /// in the same month, 2021-08, every time. The vendor returns the same
    /// bytes on every request, so a deterministic refusal was retried for ever
    /// and the month never landed.
    ///
    /// Asserted on all three shapes for the reason the null-price rule learned
    /// three times over: a rule added to a subset of the doors is a rule that
    /// will be missing from Dhan's.
    #[test]
    fn an_impossible_bar_is_dropped_and_the_rest_of_the_window_survives() {
        // Row 2's high is BELOW its low, which no bar can be.
        let columnar = r#"{"open":[100.00,244.00],"high":[100.50,243.00],
                           "low":[99.50,244.50],"close":[100.25,244.00],
                           "volume":[10,11],"timestamp":[1751337900,1751337960]}"#;
        let window = decode_body(
            columnar,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("one impossible row does not refuse the window");
        assert_eq!(window.rows.len(), 1, "parallel arrays: one row dropped");
        // WHICH row survived, not merely how many. A filter applied to the
        // prices and not to `volume` or `timestamp` keeps the right count and
        // reassembles every later bar from one row's prices and the next row's
        // stamp — the misalignment `kept_rows` builds a shared mask to avoid.
        assert_eq!(window.rows[0].volume, 10, "and it is the good row");
        assert_eq!(window.rows[0].open, 10_000, "with its own price");
        assert_eq!(
            window.rows[0].timestamp, 1_751_337_900,
            "and its own stamp, which is the half a count cannot catch"
        );

        let objects = r#"{"candles":[
            {"open":100.00,"high":100.50,"low":99.50,"close":100.25,
             "volume":10,"timestamp":1751337900},
            {"open":244.00,"high":243.00,"low":244.50,"close":244.00,
             "volume":11,"timestamp":1751337960}]}"#;
        let spec_objects = HttpSpec {
            response: ResponseShape::ArrayOfObjects {
                envelope: Some("candles"),
            },
            ..spec(PriceScale::Rupees)
        };
        let window = decode_body(objects, &spec_objects, crate::vendor::Listing::Equity)
            .expect("the object shape drops it too");
        assert_eq!(window.rows.len(), 1, "array of objects");
        assert_eq!(window.rows[0].volume, 10, "and the same row survived");

        // A FLAT BAR IS LEGAL AND MUST NOT BE DROPPED. Every price equal is a
        // minute that traded at one level, which is common at an open and on an
        // illiquid name — a filter that used strict comparisons would silently
        // delete them all.
        let flat = r#"{"open":[100.00],"high":[100.00],"low":[100.00],
                       "close":[100.00],"volume":[3],"timestamp":[1751337900]}"#;
        let window = decode_body(
            flat,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("a flat bar is a bar");
        assert_eq!(
            window.rows.len(),
            1,
            "open == high == low == close is legal"
        );

        // AND A ZERO-PRICED BAR IS KEPT. `ohlc_is_sane` admits zero, and
        // `CLAUDE.md` §7 says zero means zero rather than absence — so the
        // sign clause must refuse only what is BELOW zero.
        let zero = r#"{"open":[0],"high":[0],"low":[0],"close":[0],
                       "volume":[0],"timestamp":[1751337900]}"#;
        let window = decode_body(
            zero,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("zero is a price like any other");
        assert_eq!(window.rows.len(), 1, "zero is not negative");
    }

    /// One column set, for driving [`drop_impossible_bars`] directly.
    fn arrays_of(quads: &[(i64, i64, i64, i64)], with_oi: bool) -> ParallelArrays {
        ParallelArrays {
            open: quads.iter().map(|q| q.0).collect(),
            high: quads.iter().map(|q| q.1).collect(),
            low: quads.iter().map(|q| q.2).collect(),
            close: quads.iter().map(|q| q.3).collect(),
            // `try_from` RATHER THAN `as`. A fixture row count cannot overflow
            // an `i64`, but `as` is the cast this workspace refuses outright —
            // and the values matter here: each column carries a DISTINCT number
            // per row, so a filter that took the wrong row is caught by the
            // value rather than only by the length.
            volume: (0..quads.len())
                .map(|i| 1_000 + i64::try_from(i).unwrap_or(0))
                .collect(),
            timestamp: (0..quads.len())
                .map(|i| 1_751_337_900 + i64::try_from(i).unwrap_or(0))
                .collect(),
            open_interest: if with_oi {
                (0..quads.len())
                    .map(|i| 7_000 + i64::try_from(i).unwrap_or(0))
                    .collect()
            } else {
                Vec::new()
            },
        }
    }

    /// **EVERY CLAUSE OF THE BAR RELATIONSHIP IS LOAD-BEARING, ONE AT A TIME.**
    ///
    /// Driven directly rather than through [`decode_body`] because two of the
    /// clauses cannot be reached that way at all: `prices` refuses a negative
    /// price on the JSON path before the filter ever sees it, so the sign half
    /// of this predicate is unreachable from a body. It is kept because
    /// `Bar::ohlc_is_sane` — the store's own copy of this rule — carries it for
    /// the archive path, whose decoder parses a leading minus and never checks
    /// it, and a filter that agreed with the store on five clauses out of nine
    /// would be the more dangerous kind of nearly-right.
    ///
    /// `cargo mutants` is why this test exists in this shape: ten mutants of
    /// these two functions survived the body-level tests, including every `&&`
    /// in the relationship. Each row below fails **exactly one** clause while
    /// the clauses before it hold, which is what it takes to catch an `&&`
    /// turning into an `||`.
    #[test]
    fn each_clause_of_the_bar_relationship_drops_the_row_on_its_own() {
        // (open, high, low, close, why)
        for (open, high, low, close, why) in [
            (300, 200, 100, 150, "the high is below the open"),
            (100, 100, 200, 100, "the high is below the low"),
            (100, 100, 50, 200, "the high is below the close"),
            (100, 300, 200, 250, "the low is above the open"),
            (200, 300, 150, 100, "the low is above the close"),
            (-100, -50, -200, -100, "the open is negative"),
            (-50, -40, -200, -100, "the high is negative"),
            (0, 0, -1, 0, "the low is negative"),
            (0, 0, -1, -1, "the close is negative"),
        ] {
            let mut one = arrays_of(&[(open, high, low, close)], false);
            assert_eq!(
                drop_impossible_bars(&mut one),
                1,
                "{why}: ({open},{high},{low},{close}) is not a bar"
            );
            assert!(one.open.is_empty(), "{why}: and the row is gone");
            assert!(one.volume.is_empty(), "{why}: from every column");
            assert!(one.timestamp.is_empty(), "{why}: the stamp too");
        }

        // AND EVERY LEGAL SHAPE SURVIVES UNTOUCHED. Without this the whole
        // function could return "drop everything" and every row above passes.
        for (open, high, low, close, why) in [
            (100, 110, 90, 105, "an ordinary bar"),
            (100, 100, 100, 100, "a flat bar — common at an open"),
            (0, 0, 0, 0, "zero is a price, not an absence (CLAUDE.md §7)"),
            (100, 110, 90, 110, "the close AT the high"),
            (100, 110, 90, 90, "the close AT the low"),
        ] {
            let mut one = arrays_of(&[(open, high, low, close)], false);
            assert_eq!(drop_impossible_bars(&mut one), 0, "{why} is a bar");
            assert_eq!(one.open.len(), 1, "{why}: and it is kept");
        }
    }

    /// **THE DROP TAKES THE SAME ROW OUT OF EVERY COLUMN, INCLUDING OPEN
    /// INTEREST — AND LEAVES AN ABSENT OPEN-INTEREST COLUMN ABSENT.**
    ///
    /// A row is a bar. Dropping index `i` from the prices and not from `volume`
    /// or `timestamp` would rebuild every later bar out of one row's prices and
    /// the next row's stamp, which is exactly the misalignment `kept_rows`
    /// builds a shared mask to avoid — and it would do it while keeping all
    /// seven lengths equal, so nothing downstream would notice.
    ///
    /// The second half is its own rule: an EMPTY `open_interest` means the
    /// descriptor names no such column, which is not a length that can
    /// disagree. `CLAUDE.md` §7 — `i64::MIN` is the null and zero means zero —
    /// so a column filled in here would later read back as real open interest.
    #[test]
    fn a_dropped_row_leaves_every_column_and_an_absent_one_stays_absent() {
        // Middle row is impossible; the two either side are bars.
        let quads = [
            (100, 110, 90, 105),
            (100, 100, 200, 100),
            (300, 310, 290, 305),
        ];

        let mut with_oi = arrays_of(&quads, true);
        assert_eq!(drop_impossible_bars(&mut with_oi), 1);
        assert_eq!(with_oi.open, vec![100, 300], "prices keep rows 0 and 2");
        assert_eq!(with_oi.high, vec![110, 310]);
        assert_eq!(with_oi.low, vec![90, 290]);
        assert_eq!(with_oi.close, vec![105, 305]);
        assert_eq!(with_oi.volume, vec![1_000, 1_002], "and so does the volume");
        assert_eq!(
            with_oi.timestamp,
            vec![1_751_337_900, 1_751_337_902],
            "and the stamps — the half a length check cannot catch"
        );
        assert_eq!(
            with_oi.open_interest,
            vec![7_000, 7_002],
            "open interest is a column like any other when it is present"
        );

        let mut without = arrays_of(&quads, false);
        assert_eq!(drop_impossible_bars(&mut without), 1);
        assert!(
            without.open_interest.is_empty(),
            "an absent column stays absent — filtering it would invent one"
        );
        assert_eq!(without.open.len(), 2, "and the rest still filtered");
    }

    /// **A WINDOW OF NOTHING BUT BARS IS RETURNED UNTOUCHED, AND SAYS SO.**
    ///
    /// The early return is the common path — almost every window — and a
    /// mutant that removed it would still pass every test above, because
    /// filtering a mask of all-true is a no-op. Asserting the COUNT is what
    /// separates "nothing was dropped" from "the filter ran and found nothing".
    #[test]
    fn a_clean_window_is_not_filtered_at_all() {
        let mut clean = arrays_of(&[(100, 110, 90, 105), (200, 210, 190, 205)], true);
        assert_eq!(drop_impossible_bars(&mut clean), 0, "nothing to drop");
        assert_eq!(clean.open, vec![100, 200]);
        assert_eq!(clean.open_interest, vec![7_000, 7_001], "untouched");
    }

    /// **A NEGATIVE VOLUME IS CAUGHT IN BOTH JSON SPELLINGS.**
    ///
    /// `serde_json` answers `as_i64()` for `-125` and `None` for `-125.0`,
    /// which is a float — so a decoder that read only the integer spelling
    /// would let every float-encoded negative straight through. Dhan types
    /// volume as `int` in its field table and as `int32` in its binary feed
    /// spec, and this decoder does not get to assume which one arrives.
    #[test]
    fn a_negative_volume_is_caught_whether_it_arrives_as_an_int_or_a_float() {
        for sent in ["-125", "-125.0", "-0.5"] {
            let body = format!(
                "{{\"open\":[100.00,100.00],\"high\":[100.00,100.00],\
                   \"low\":[100.00,100.00],\"close\":[100.00,100.00],\
                   \"volume\":[9,{sent}],\"timestamp\":[1751337900,1751337960]}}"
            );
            let window = decode_body(
                &body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            )
            .unwrap_or_else(|why| panic!("{sent} drops its row, not the window: {why}"));
            assert_eq!(window.rows.len(), 1, "{sent}: the negative row went");
            assert_eq!(window.rows[0].volume, 9, "{sent}: the good row stayed");
        }

        // AND A POSITIVE FLOAT VOLUME IS NOT COLLATERAL DAMAGE.
        let ok = r#"{"open":[100.00],"high":[100.00],"low":[100.00],
                     "close":[100.00],"volume":[9.0],"timestamp":[1751337900]}"#;
        let window = decode_body(
            ok,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("a whole number sent as a float is still a count");
        assert_eq!(window.rows.len(), 1, "a positive float volume is kept");
    }

    /// **EVERY COLUMN IS LENGTH-CHECKED, AND EACH ONE ON ITS OWN.**
    ///
    /// The check is a chain of `||`, and a single odd column proves only the
    /// clause it happens to hit — `cargo mutants` turned three of those `||`
    /// into `&&` and every one survived, because no test made `volume`,
    /// `timestamp` or `open_interest` the short column by itself.
    #[test]
    fn each_column_on_its_own_can_be_the_odd_length_one() {
        for (name, body) in [
            (
                "high",
                r#"{"open":[1,1],"high":[1],"low":[1,1],"close":[1,1],
                    "volume":[1,1],"timestamp":[1,2]}"#,
            ),
            (
                "low",
                r#"{"open":[1,1],"high":[1,1],"low":[1],"close":[1,1],
                    "volume":[1,1],"timestamp":[1,2]}"#,
            ),
            (
                "close",
                r#"{"open":[1,1],"high":[1,1],"low":[1,1],"close":[1],
                    "volume":[1,1],"timestamp":[1,2]}"#,
            ),
            (
                "volume",
                r#"{"open":[1,1],"high":[1,1],"low":[1,1],"close":[1,1],
                    "volume":[1],"timestamp":[1,2]}"#,
            ),
            (
                "timestamp",
                r#"{"open":[1,1],"high":[1,1],"low":[1,1],"close":[1,1],
                    "volume":[1,1],"timestamp":[1]}"#,
            ),
        ] {
            let got = decode_body(
                body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            );
            assert!(
                matches!(got, Err(FetchError::LengthDisagreement { .. })),
                "a short {name} column is a disagreement, not a window: {got:?}"
            );
        }

        // AND THE LONGER SIDE, WHICH IS THE ONE THAT MATTERS.
        //
        // A SHORT column is caught twice over: this check refuses it, and if
        // this check were removed `RawWindow::decode`'s own length check would
        // still refuse it. So a short column cannot tell the two apart, and
        // `cargo mutants` proved exactly that — turning these `||` into `&&`
        // survived every short-column case above.
        //
        // A LONGER column is the case only this check can catch. The mask is
        // applied with `zip`, which stops at the shorter side, so an
        // over-length column would be silently TRIMMED to fit: seven equal
        // lengths, a clean decode, and a window of well-formed bars assembled
        // from rows that never lined up. That is the failure this ordering
        // exists to prevent, and `is_err()` is the assertion that proves it —
        // under the mutant these decode successfully.
        for (name, body) in [
            (
                "volume",
                r#"{"open":[1,1],"high":[1,1],"low":[1,1],"close":[1,1],
                    "volume":[1,1,1],"timestamp":[1,2]}"#,
            ),
            (
                "timestamp",
                r#"{"open":[1,1],"high":[1,1],"low":[1,1],"close":[1,1],
                    "volume":[1,1],"timestamp":[1,2,3]}"#,
            ),
            (
                "high",
                r#"{"open":[1,1],"high":[1,1,1],"low":[1,1],"close":[1,1],
                    "volume":[1,1],"timestamp":[1,2]}"#,
            ),
        ] {
            let got = decode_body(
                body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity,
            );
            assert!(
                matches!(got, Err(FetchError::LengthDisagreement { .. })),
                "an over-length {name} column must be REFUSED, never trimmed to \
                 fit the mask: {got:?}"
            );
        }
    }

    #[test]
    fn a_refusal_under_a_200_is_a_refusal_and_never_a_success() {
        let shipped = match crate::vendor::Feed::Dhan.descriptor().transport {
            crate::vendor::Transport::Http(spec) => spec,
            crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
        };
        let source = HttpSource::new(shipped, Credential::token("shhh".to_owned()))
            .expect("a client for the shipped Dhan row");

        // THE SHAPE DHAN'S OWN SDK EXAMPLE TESTS FOR: a body that says failure
        // under a status that says success. `21-errors.md` checks
        // `response["status"] == "failure"` and never reads the HTTP status.
        let dead = r#"{"errorType":"Invalid_Authentication","errorCode":"DH-901",
                       "errorMessage":"Client ID or access token is invalid"}"#;
        let Err(FetchError::VendorRefused { status, named, .. }) =
            source.weigh_answered_body(dead, 200)
        else {
            panic!("a body naming DH-901 is a refusal whatever the status says")
        };
        assert_eq!(status, 200, "the status the vendor actually sent");
        assert_eq!(
            named,
            Some(crate::refusal::Disposition::SessionDead),
            "and it carries the disposition, so the ladder sees the same \
             verdict it would have seen had the vendor used the status"
        );

        // A MISFILED CODE IS STILL READ BY ITS SENTENCE, which is D-0325's
        // rule and has to survive being reached from this new path too.
        let misfiled = r#"{"errorType":"Order_Error","errorCode":"DH-906",
                          "errorMessage":"Invalid Token"}"#;
        let Err(FetchError::VendorRefused { named, .. }) =
            source.weigh_answered_body(misfiled, 200)
        else {
            panic!("the sentence names a dead token")
        };
        assert_eq!(named, Some(crate::refusal::Disposition::SessionDead));

        // AN ORDINARY BODY IS STILL A SUCCESS. A bars answer carries no error
        // code, so the contract finds nothing and the additive increase is
        // earned — the path every good request takes.
        source
            .weigh_answered_body(r#"{"open":[1],"close":[1]}"#, 200)
            .expect("a body with no refusal in it is a success");
    }

    /// **A THROTTLE NAMED IN THE BODY STILL NARROWS THE ALLOWANCE**, and any
    /// other refusal leaves it alone.
    ///
    /// Written because `cargo mutants` proved the test above could not tell the
    /// two apart: flipping `named == Throttled` to `!=` survived the whole
    /// suite. That mutation is the live defect it looks like — a throttle would
    /// stop narrowing the rate while every unrelated refusal narrowed it
    /// instead, so the governor would back off for a dead token and accelerate
    /// into a rate limit.
    ///
    /// The allowance is read through the shared governor rather than asserted
    /// on a returned value, because the narrowing IS the side effect and there
    /// is nothing else to look at.
    #[test]
    fn only_a_throttle_named_in_the_body_narrows_the_allowance() {
        let shipped = match crate::vendor::Feed::Dhan.descriptor().transport {
            crate::vendor::Transport::Http(spec) => spec,
            crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
        };
        let ceiling = crate::rate::DHAN_PER_SECOND;
        let allowance = |source: &HttpSource| -> Option<u32> {
            source.governor.as_ref().and_then(|lock| {
                lock.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .permitted(crate::rate::WindowSpan::Second)
            })
        };

        // DH-904 is the vendor's own word for "too many requests".
        let source = HttpSource::new(shipped, Credential::token("shhh".to_owned()))
            .expect("a client for the shipped Dhan row");
        assert_eq!(allowance(&source), Some(ceiling), "untouched to begin with");
        let throttled = r#"{"errorCode":"DH-904","errorMessage":"Too many requests"}"#;
        assert!(source.weigh_answered_body(throttled, 200).is_err());
        assert!(
            allowance(&source).is_some_and(|now| now < ceiling),
            "a throttle under a 200 is still a throttle"
        );

        // AND A REFUSAL THAT IS NOT ABOUT RATE LEAVES IT WHERE IT WAS. A dead
        // token says nothing about pace, so backing off for it would slow every
        // remaining request for a reason that is not pace.
        let source = HttpSource::new(shipped, Credential::token("shhh".to_owned()))
            .expect("a second client, its own governor");
        let dead = r#"{"errorCode":"DH-901","errorMessage":"token expired"}"#;
        assert!(source.weigh_answered_body(dead, 200).is_err());
        assert_eq!(
            allowance(&source),
            Some(ceiling),
            "a dead token is not a pace, so the allowance is untouched"
        );
    }

    #[test]
    fn arrays_that_disagree_in_length_refuse_the_whole_window() {
        let body = r#"{"open":[1,2],"high":[1,2],"low":[1,2],"close":[1,2],
                       "volume":[1],"timestamp":[1,2]}"#;
        assert!(matches!(
            decode_body(
                body,
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity
            ),
            Err(FetchError::LengthDisagreement { .. })
        ));
    }

    /// A body that is not JSON, and one missing a declared field, are both
    /// named rather than defaulted.
    #[test]
    fn a_malformed_or_incomplete_body_is_refused_by_name() {
        assert!(matches!(
            decode_body(
                "not json",
                &spec(PriceScale::Rupees),
                crate::vendor::Listing::Equity
            ),
            Err(FetchError::TransportFailed { .. })
        ));
        let missing = r#"{"open":[1],"high":[1],"low":[1],"close":[1],"volume":[1]}"#;
        let Err(FetchError::TransportFailed { detail }) = decode_body(
            missing,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        ) else {
            panic!("a missing timestamp array must refuse")
        };
        assert!(
            detail.contains("timestamp"),
            "the refusal names it: {detail}"
        );
    }

    /// The arrays may sit one level down — **when the descriptor says so.**
    #[test]
    fn arrays_under_the_declared_envelope_are_found() {
        let body = r#"{"data":{"open":[100.5],"high":[100.5],"low":[100.5],
                       "close":[100.5],"volume":[7],"timestamp":[1751337900]}}"#;
        let window = decode_body(
            body,
            &spec_under(PriceScale::Rupees, Some("data")),
            crate::vendor::Listing::Equity,
        )
        .expect("decodes");
        assert_eq!(window.rows[0].open, 10_050);
    }

    /// **ONE BAR, TWO OBJECTS.** The defect this envelope exists to close.
    ///
    /// Each field used to resolve itself: top level first, then the first value
    /// one level down holding a key of that name. With two such objects in one
    /// body the seven fields could come from either, and `serde_json`'s map is
    /// sorted — so `cached` won over `live` by alphabet, on every field that
    /// `live` did not also have at the same place.
    ///
    /// Nothing downstream catches it. Two objects describing the same window
    /// hold the same number of bars, so the seven-array length check passes and
    /// the spliced bar reaches the store looking exactly like a real one.
    ///
    /// Here `cached` quotes a stale 999 and `live` quotes 100.5. A decoder that
    /// searches returns a bar built from both. A decoder told where to look
    /// returns `live`'s bar, whole.
    #[test]
    fn one_bar_can_never_be_assembled_from_two_different_objects() {
        let body = r#"{
            "cached":{"open":[999],"high":[999],"low":[999],"close":[999],
                      "volume":[1],"timestamp":[1]},
            "live":{"open":[100.5],"high":[100.5],"low":[100.5],"close":[100.5],
                    "volume":[7],"timestamp":[1751337900]}
        }"#;
        let window = decode_body(
            body,
            &spec_under(PriceScale::Rupees, Some("live")),
            crate::vendor::Listing::Equity,
        )
        .expect("decodes");
        let row = &window.rows[0];
        assert_eq!(
            (row.open, row.high, row.low, row.close),
            (10_050, 10_050, 10_050, 10_050),
            "every price comes from `live`; a 99900 anywhere here is `cached` \
             leaking into a bar that was never quoted"
        );
        assert_eq!(row.volume, 7, "and so does the volume");
        assert_eq!(row.timestamp, 1_751_337_900, "and the timestamp");

        // The other object is reachable only by naming it, which is the point:
        // which bar you get is the descriptor's decision, never the alphabet's.
        let stale = decode_body(
            body,
            &spec_under(PriceScale::Rupees, Some("cached")),
            crate::vendor::Listing::Equity,
        )
        .expect("decodes");
        assert_eq!(stale.rows[0].open, 99_900);
    }

    /// With no envelope declared, the top level is the only place looked.
    ///
    /// A body that wraps its bars is then refused by name rather than
    /// rummaged through — the refusal is what tells an operator to add the
    /// envelope to the descriptor's row.
    #[test]
    fn no_envelope_means_the_top_level_and_nowhere_else() {
        let wrapped = r#"{"data":{"open":[1],"high":[1],"low":[1],"close":[1],
                          "volume":[1],"timestamp":[1]}}"#;
        let Err(FetchError::TransportFailed { detail }) = decode_body(
            wrapped,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        ) else {
            panic!("the descriptor says top level, so `data` is not searched")
        };
        assert!(
            detail.contains("open"),
            "the refusal names the field: {detail}"
        );
        assert!(
            detail.contains("data"),
            "and lists what the answer does have, so the descriptor can be \
             corrected: {detail}"
        );
    }

    /// A declared envelope the answer does not carry is refused, and the
    /// refusal carries the two things needed to fix the descriptor.
    #[test]
    fn a_missing_envelope_names_the_key_expected_and_the_keys_present() {
        let body = r#"{"payload":{"open":[1],"high":[1],"low":[1],"close":[1],
                       "volume":[1],"timestamp":[1]}}"#;
        let Err(FetchError::TransportFailed { detail }) = decode_body(
            body,
            &spec_under(PriceScale::Rupees, Some("data")),
            crate::vendor::Listing::Equity,
        ) else {
            panic!("the declared envelope is absent and must be named")
        };
        assert!(detail.contains("\"data\""), "the key expected: {detail}");
        assert!(detail.contains("payload"), "the key present: {detail}");

        // A body that is not an object at all says that rather than listing
        // keys it does not have.
        let Err(FetchError::TransportFailed { detail }) = decode_body(
            "[1,2,3]",
            &spec_under(PriceScale::Rupees, Some("data")),
            crate::vendor::Listing::Equity,
        ) else {
            panic!("an array is not an envelope")
        };
        assert!(detail.contains("not an object"), "{detail}");

        // And an object with no keys at all.
        let Err(FetchError::TransportFailed { detail }) = decode_body(
            "{}",
            &spec_under(PriceScale::Rupees, Some("data")),
            crate::vendor::Listing::Equity,
        ) else {
            panic!("an empty object holds no envelope")
        };
        assert!(detail.contains("no keys at all"), "{detail}");
    }

    /// **THE W1 PAIR, AND THE TEST THIS CODE ALREADY CLAIMED TO HAVE.**
    ///
    /// `IsoDateTimeOffset` works only because TWO arms agree: `one_stamp`
    /// applies the stated offset, and `fetch::land` passes the result through
    /// instead of subtracting IST again. Either alone is wrong, and the wrong
    /// direction is the one that **stores** — a Kite bar run through the IST
    /// arm lands 5h30m early, and 03:45 on a trading day is a plausible
    /// timestamp rather than an obviously broken one.
    ///
    /// Both this file and `vendor.rs` cited a test proving it. **No such test
    /// existed**: an adversarial pass moved the variant into the IST arm and
    /// the whole suite stayed green. This is that test, driven through the
    /// SHIPPED descriptor rather than a fixture.
    #[test]
    fn a_zone_carrying_stamp_is_converted_once_and_the_two_arms_must_agree() {
        // Computed independently of the code under test: 17,515 days since the
        // epoch, plus 9h15m, minus the stated 5h30m. Declared first because an
        // item after a statement reads as though it were scoped to what came
        // before it, and it is not.
        const TRUE_UTC: i64 = 17_515 * 86_400 + 9 * 3_600 + 15 * 60 - 19_800;

        let crate::vendor::Transport::Http(zerodha) =
            crate::vendor::Feed::Zerodha.descriptor().transport
        else {
            panic!("Zerodha is an HTTP feed");
        };
        // The vendor's own first example row, verbatim from charter §4z.
        let body = r#"{"status":"success","data":{"candles":[
            ["2017-12-15T09:15:00+0530",1704.5,1705,1699.25,1702.8,2499]]}}"#;
        let raw = decode_body(body, &zerodha, crate::vendor::Listing::Equity)
            .expect("the vendor's own example decodes");

        assert_eq!(TRUE_UTC, 1_513_309_500, "the hand computation");

        let request = BarRequest {
            instrument_id: "5633".to_owned(),
            listing: crate::vendor::Listing::Equity,
            window: crate::session::Window::new(
                crate::session::Day::new(2017, 12, 15).expect("a day"),
                crate::session::Day::new(2017, 12, 15).expect("a day"),
            )
            .expect("a window"),
            granularity: crate::vendor::Granularity::Minute1,
        };
        let landed = crate::fetch::land(&raw, &request, zerodha.timestamps, PriceScale::Paisa)
            .expect("the bar lands");
        assert_eq!(landed.bars.len(), 1);
        assert_eq!(
            landed.bars[0].ts_micros,
            TRUE_UTC * 1_000_000,
            "the offset is applied ONCE. If this reads 19,800 seconds early, \
             the IsoDateTimeOffset arm has been moved into fetch::land's IST \
             branch and every Kite bar is stored 5h30m before it happened"
        );

        // AND THE OTHER DIRECTION. Landing the SAME rows under the zoneless
        // encoding must differ by exactly the IST offset — that is what makes
        // moving the arm a failure rather than a no-op.
        let zoneless = crate::fetch::land(
            &raw,
            &request,
            crate::vendor::TimestampEncoding::IsoDateTimeText,
            PriceScale::Paisa,
        )
        .expect("it still lands, wrongly");
        //
        // ON THIS WINDOW IT DROPS THE BAR RATHER THAN MISPLACING IT, and the
        // distinction is worth writing down. Shifted 5h30m, 09:15 IST becomes
        // 03:45 IST — before the session opens — so the filter refuses it and
        // the window comes back EMPTY. That is the lucky case. The fault is
        // called silent because a shift that lands INSIDE a session does not
        // get caught: a day-rung bar, or any window whose shifted instant is
        // still between 09:15 and 15:29, stores a wrong timestamp that passes
        // every check below it.
        //
        // So the assertion is that the two encodings DISAGREE, by either route.
        // Requiring one specific route would make this test pass or fail on the
        // window it happens to use rather than on the pairing it exists to pin.
        assert!(
            zoneless.bars.len() != landed.bars.len()
                || zoneless.bars.first().map(|b| b.ts_micros)
                    != landed.bars.first().map(|b| b.ts_micros),
            "the two encodings must not agree, or this proves nothing: \
             zoneless landed {} bar(s), zone-carrying landed {}",
            zoneless.bars.len(),
            landed.bars.len()
        );
    }

    /// The descriptor's own bytes, against the vendor's published curl example.
    ///
    /// **ZERODHA IS ASKED FOR OPEN INTEREST, AND IT HAS TO BE ASKED.**
    ///
    /// `Zerodha Docs/13-historical.md:65`: *"Accepts `0` or `1`. Pass `1` to get
    /// OI (Open Interest) data."* Line 95: with `oi=1` *"each candle is a
    /// 7-element positional array; the OI value is appended as the last
    /// element."* Without the parameter the array is six cells and carries none.
    ///
    /// The request sent `from` and `to` and nothing else, so every Zerodha bar
    /// this build could store held no open interest — the vendor was not
    /// withholding it, nobody asked. The decoder was already ready: it reads
    /// cell six unconditionally and falls back to `OI_NULL`.
    ///
    /// Asserted on the DESCRIPTOR rather than on a URL, because the query pairs
    /// are resolved inside `window_async`, which needs a socket. The descriptor
    /// is what decides, so the descriptor is what is pinned — and a mutation
    /// that drops this row fails here rather than silently storing six-cell
    /// candles for a feed that would have sent seven.
    #[test]
    fn zerodha_asks_for_open_interest_because_the_vendor_only_sends_it_on_request() {
        let crate::vendor::Transport::Http(zerodha) =
            crate::vendor::Feed::Zerodha.descriptor().transport
        else {
            panic!("Zerodha is an HTTP feed");
        };
        let oi =
            zerodha.params.iter().find(|p| p.name == "oi").expect(
                "the OI parameter is on the row, or every candle comes back six cells wide",
            );
        assert!(
            matches!(oi.value, crate::vendor::ParamValue::Fixed("1")),
            "the vendor accepts 0 or 1 and only 1 appends the column: {:?}",
            oi.value
        );

        // AND THE FIELD NAME STAYS ABSENT, deliberately. `FieldNames` feeds the
        // OBJECT-shaped decoder, which looks a column up by name; this vendor
        // answers positional rows where the index is the whole contract, and a
        // name here would claim a shape the payload does not have.
        assert!(
            zerodha.fields.open_interest.is_none(),
            "a positional feed names no OI field — position is the contract"
        );
        assert!(
            matches!(
                zerodha.response,
                crate::vendor::ResponseShape::PositionalRows { .. }
            ),
            "if this feed ever answers objects, the assertion above changes meaning"
        );
    }

    /// Nine single-field mutations to the Zerodha row left the suite green
    /// before this existed — the base URL, both path literals, the placeholder
    /// order, the two interval words, the prefix and the separator.
    #[test]
    fn the_zerodha_request_is_the_url_and_header_the_vendor_documents() {
        let crate::vendor::Transport::Http(zerodha) =
            crate::vendor::Feed::Zerodha.descriptor().transport
        else {
            panic!("Zerodha is an HTTP feed");
        };
        let source = HttpSource::new(
            zerodha,
            Credential::pair("APIKEY".to_owned(), "TOKEN".to_owned()),
        )
        .expect("a client builds");

        let minute = BarRequest {
            instrument_id: "5633".to_owned(),
            listing: crate::vendor::Listing::Equity,
            window: crate::session::Window::new(
                crate::session::Day::new(2017, 12, 15).expect("a day"),
                crate::session::Day::new(2017, 12, 15).expect("a day"),
            )
            .expect("a window"),
            granularity: crate::vendor::Granularity::Minute1,
        };
        assert_eq!(
            source
                .url(&minute, "unused", "unused")
                .expect("the URL resolves"),
            "https://api.kite.trade/instruments/historical/5633/minute",
            "byte for byte, the vendor's own curl example"
        );
        let day = BarRequest {
            granularity: crate::vendor::Granularity::Day1,
            ..minute.clone()
        };
        assert!(
            source
                .url(&day, "u", "u")
                .expect("the day rung resolves")
                .ends_with("/5633/day"),
            "the rung reaches the PATH as the vendor's word, not the store's"
        );

        let (name, value) = source.header();
        assert_eq!(name, "Authorization");
        assert_eq!(
            value, "token APIKEY:TOKEN",
            "key first, one colon, one trailing space in the prefix"
        );
        assert!(
            zerodha
                .extra_headers
                .iter()
                .any(|(h, v)| *h == "X-Kite-Version" && *v == "3"),
            "every Kite curl example carries it beside the credential"
        );
    }

    /// Both mismatch directions refuse before a client exists.
    #[test]
    fn a_credential_that_does_not_match_the_scheme_refuses_at_construction() {
        let crate::vendor::Transport::Http(zerodha) =
            crate::vendor::Feed::Zerodha.descriptor().transport
        else {
            panic!("Zerodha is an HTTP feed");
        };
        let crate::vendor::Transport::Http(groww) =
            crate::vendor::Feed::Groww.descriptor().transport
        else {
            panic!("Groww is an HTTP feed");
        };
        // Two wanted, one given: would have sent `token :TOKEN`, which a vendor
        // answers 403 to — indistinguishable from an expired session.
        assert_eq!(
            HttpSource::new(zerodha, Credential::token("TOKEN".to_owned()))
                .expect_err("a two-secret scheme refuses one secret"),
            FetchError::CredentialMismatch {
                names_two: true,
                given_two: false
            }
        );
        assert_eq!(
            HttpSource::new(groww, Credential::pair("K".to_owned(), "T".to_owned()))
                .expect_err("a one-secret scheme refuses two"),
            FetchError::CredentialMismatch {
                names_two: false,
                given_two: true
            }
        );
    }

    /// The blocking seam refuses by name rather than silently blocking, and
    /// the credential never appears in a `Debug` rendering.
    #[test]
    fn the_sync_seam_refuses_and_the_token_is_never_printed() {
        let source = HttpSource::new(
            spec(PriceScale::Rupees),
            Credential::token("SUPERSECRET".to_owned()),
        )
        .expect("a client builds");
        let shown = format!("{source:?}");
        assert!(!shown.contains("SUPERSECRET"), "the token leaked: {shown}");
        assert!(shown.contains("<redacted>"), "and it says so: {shown}");
        assert_eq!(
            source.endpoint(crate::vendor::Granularity::Day1),
            "https://vendor.invalid/bars"
        );
    }

    /// A server on loopback that answers once and reports what it was sent.
    ///
    /// Raw sockets and hand-written HTTP, because `crates/pull` takes `tokio`
    /// without the `net` feature and a test is not a reason to widen a
    /// dependency. `std::net` in a plain thread is enough to prove where a
    /// header did and did not go.
    ///
    /// Returns the base URL to point a descriptor at, and a handle that yields
    /// the request line and headers of whatever arrived — or `None` if nothing
    /// ever connected, which is the assertion that matters below.
    fn listener(
        answer: Option<String>,
    ) -> (
        String,
        std::sync::mpsc::Receiver<String>,
        std::net::SocketAddr,
    ) {
        use std::io::{Read as _, Write as _};
        let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let addr = socket.local_addr().expect("an address");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = socket.accept() else {
                return;
            };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let seen = String::from_utf8_lossy(buf.get(..n).unwrap_or(&[])).into_owned();
            let _ = tx.send(seen);
            if let Some(ref body) = answer {
                let _ = stream.write_all(body.as_bytes());
            }
            let _ = stream.flush();
        });
        (format!("http://{addr}"), rx, addr)
    }

    /// A server that sends `header` and then floods, until the client stops
    /// reading or `chunks` writes of `frame` bytes have gone out.
    ///
    /// Separate from [`listener`], which answers with one `String`. The body
    /// this exists to send is 64 MiB, and a 64 MiB `String` built in the test
    /// process to prove that the client will not build one is the wrong shape
    /// of proof. This holds `frame` bytes and writes them over and over, so the
    /// client is the only side that has to decide when to stop.
    ///
    /// **No `Content-Length` is sent by any caller of this.** That is the point:
    /// with no declared length the pre-read check has nothing to check, and
    /// what the answer costs is decided entirely by the read loop.
    fn flooding_listener(header: &'static str, frame: usize, chunks: usize) -> String {
        use std::io::{Read as _, Write as _};
        let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let addr = socket.local_addr().expect("an address");
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = socket.accept() else {
                return;
            };
            let mut buf = [0u8; 4096];
            let _request = stream.read(&mut buf);
            if stream.write_all(header.as_bytes()).is_err() {
                return;
            }
            let filler = vec![b'x'; frame];
            for _ in 0..chunks {
                // THE CLIENT HANGING UP IS THE EXPECTED END, NOT A FAILURE.
                // Once it has seen more than it will hold it drops the
                // response, the connection resets, and this write fails — which
                // is the signal to stop rather than something to report. Rust
                // ignores SIGPIPE at startup, so this is an `Err` and not a
                // killed test process.
                if stream.write_all(&filler).is_err() {
                    return;
                }
            }
            let _flushed = stream.flush();
        });
        format!("http://{addr}")
    }

    /// **THE CREDENTIAL MUST NOT FOLLOW A REDIRECT.**
    ///
    /// `reqwest` follows up to ten by default and strips only the headers it
    /// knows are sensitive — `Authorization`, `Cookie`, `Proxy-Authorization`,
    /// `WWW-Authenticate`. Dhan's descriptor calls its credential header
    /// `access-token`, which is on none of those lists, so a 302 from the bars
    /// endpoint would have carried a live broker token to whatever host the
    /// `Location` named. Nothing would have logged it and nothing would have
    /// failed.
    ///
    /// This drives it over two real loopback sockets: the origin answers `302`
    /// pointing at the second, and the second must never be connected to at all.
    #[test]
    fn a_redirect_is_refused_and_the_token_never_reaches_its_target() {
        // The hop answers nothing, because nothing must ever ask it.
        let (hop_url, hop_seen, _) = listener(None);
        let redirect = format!(
            "HTTP/1.1 302 Found\r\nLocation: {hop_url}/stolen\r\nContent-Length: 0\r\n\
             Connection: close\r\n\r\n"
        );
        let (origin_url, origin_seen, _) = listener(Some(redirect));

        let spec = HttpSpec {
            base_url: Box::leak(origin_url.into_boxed_str()),
            ..spec(PriceScale::Rupees)
        };
        let source = HttpSource::new(spec, Credential::token("SUPERSECRET".to_owned()))
            .expect("a client builds");
        let request = BarRequest {
            instrument_id: String::new(),
            listing: crate::vendor::Listing::Equity,
            window: crate::session::Window::new(
                crate::session::Day::new(2025, 7, 1).expect("a real day"),
                crate::session::Day::new(2025, 7, 1).expect("a real day"),
            )
            .expect("a real window"),
            granularity: crate::vendor::Granularity::Minute1,
        };
        let outcome = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(source.window_async(&request));

        // The origin was reached, and it WAS sent the credential — otherwise
        // this test would pass by never authenticating at all.
        let sent = origin_seen
            .recv_timeout(core::time::Duration::from_secs(5))
            .expect("the origin was contacted");
        assert!(
            sent.contains("SUPERSECRET"),
            "the descriptor's own header must carry the token to the vendor, \
             or this test proves nothing: {sent}"
        );

        // THE HOP WAS NEVER CONTACTED. This is the whole assertion.
        assert!(
            hop_seen
                .recv_timeout(core::time::Duration::from_secs(1))
                .is_err(),
            "the redirect was followed and the credential left for a host the \
             descriptor never named"
        );

        // And the 302 came back as a refusal that says what happened.
        let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
            panic!("a redirect is a refusal this build reports, not one it hides")
        };
        assert_eq!(status, 302, "the status reaches the caller");
        assert!(detail.contains("stolen"), "the Location is named: {detail}");
        assert!(
            detail.contains("does not follow redirects"),
            "and the reason is named: {detail}"
        );
        assert!(
            !detail.contains("SUPERSECRET"),
            "the refusal must never carry the credential: {detail}"
        );
    }

    /// An ordinary refusal still carries the vendor's own words, and a 429 is
    /// still distinguishable from a 500 — the redirect arm did not swallow them.
    #[test]
    fn a_non_redirect_refusal_carries_the_body_and_its_status() {
        let body = "rate limited, try later";
        let (url, _seen, _) = listener(Some(format!(
            "HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        )));
        let spec = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            ..spec(PriceScale::Rupees)
        };
        let source = HttpSource::new(spec, Credential::token("SUPERSECRET".to_owned()))
            .expect("a client builds");
        let request = BarRequest {
            instrument_id: String::new(),
            listing: crate::vendor::Listing::Equity,
            window: crate::session::Window::new(
                crate::session::Day::new(2025, 7, 1).expect("a real day"),
                crate::session::Day::new(2025, 7, 1).expect("a real day"),
            )
            .expect("a real window"),
            granularity: crate::vendor::Granularity::Minute1,
        };
        let outcome = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(source.window_async(&request));

        let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
            panic!("429 is a refusal the governor needs to see")
        };
        assert_eq!(
            status, 429,
            "the governor's business, not a flattened 'failed'"
        );
        assert_eq!(detail, body, "the vendor's own words, unchanged");
        assert!(
            !detail.contains("redirect"),
            "and no redirect wording: {detail}"
        );
    }

    /// **THE CAP IS CHECKED BEFORE THE BODY IS READ, NOT AFTER IT IS HELD.**
    ///
    /// The size check used to run on `text.len()` — that is, on a `String` the
    /// whole answer had already been decoded into. It named the right number
    /// and it named it having already paid for it, which is not a bound.
    ///
    /// A declared length past the cap is the cheap half: nothing is gained by
    /// reading a body to discover a number the header already gave. The body
    /// here is two bytes and the assertion is that they are never asked for —
    /// visible in the refusal's own words, which say where it happened.
    #[test]
    fn a_declared_length_past_the_cap_is_refused_before_the_body_is_read() {
        let declared = MAX_RESPONSE_BYTES.saturating_add(1);
        let (url, seen, _) = listener(Some(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Content-Length: {declared}\r\nConnection: close\r\n\r\n{{}}"
        )));
        let spec = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            ..spec(PriceScale::Rupees)
        };
        let outcome = source_of(spec, "SUPERSECRET").block_on_window();

        // The vendor WAS reached — otherwise this would pass by refusing to
        // send the request at all, which is a different function's job.
        assert!(
            seen.recv_timeout(core::time::Duration::from_secs(5))
                .is_ok(),
            "the request has to go out, or this proves nothing"
        );
        let Err(FetchError::TransportFailed { detail }) = outcome else {
            panic!("an answer declaring more than the cap is refused, not decoded")
        };
        assert!(
            detail.contains(&declared.to_string()),
            "the vendor's own claim is quoted back: {detail}"
        );
        assert!(
            detail.contains("Refused before the body was read"),
            "and WHERE it was refused is the whole point: {detail}"
        );
    }

    /// **AND WITH NO DECLARED LENGTH, THE READ ITSELF STOPS.**
    ///
    /// `Content-Length` is a claim a host may decline to make. A chunked or
    /// close-delimited answer declares nothing, so the pre-read check above has
    /// nothing to check and the bound has to live in the read loop — which is
    /// the case the old `text()` handled by holding the entire body first.
    ///
    /// This is the one test in this module that moves 64 MiB, and it is here
    /// because *where* the check runs is not something an arithmetic assertion
    /// can prove. The boundary itself is still owned by
    /// `the_stated_bounds_are_the_numbers_they_are_written_as`, which allocates
    /// nothing. The server holds one 64 KiB buffer; the client holds the cap and
    /// then stops.
    #[test]
    fn an_answer_with_no_declared_length_is_abandoned_once_it_runs_past_the_cap() {
        const FRAME: usize = 64 * 1024;
        // One frame more than the cap holds, so the last one is the one that
        // crosses it — the boundary, not a body ten times too big.
        let chunks = MAX_RESPONSE_BYTES.div_euclid(FRAME).saturating_add(1);
        let url = flooding_listener(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n",
            FRAME,
            chunks,
        );
        let spec = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            ..spec(PriceScale::Rupees)
        };
        let Err(FetchError::TransportFailed { detail }) =
            source_of(spec, "SUPERSECRET").block_on_window()
        else {
            panic!("an answer past the cap is refused, not decoded")
        };
        assert!(
            detail.contains("ran past"),
            "the refusal says the read was stopped, not that a held body was \
             measured: {detail}"
        );
        assert!(
            detail.contains(&MAX_RESPONSE_BYTES.to_string()),
            "and it names the cap: {detail}"
        );
    }

    /// A refusal body is read only as far as it will ever be quoted.
    ///
    /// The refusal path had **no** size check at all — `trim` cut the error
    /// string to 500 characters, but only after `text()` had built the whole
    /// body, so a 5xx carrying a gigabyte cost a gigabyte to print half a
    /// kilobyte. This drives 32 KiB against an 8 KiB cap: the operator still
    /// gets the vendor's opening words, and the sentence says the rest was
    /// never read rather than leaving them to wonder what was cut.
    #[test]
    fn a_refusal_body_is_read_only_as_far_as_it_will_ever_be_quoted() {
        let body = "z".repeat(MAX_REFUSAL_BYTES.saturating_mul(4));
        let (url, _seen, _) = listener(Some(format!(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        )));
        let spec = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            ..spec(PriceScale::Rupees)
        };
        let Err(FetchError::VendorRefused { status, detail, .. }) =
            source_of(spec, "SUPERSECRET").block_on_window()
        else {
            panic!("500 is a refusal, however long its body")
        };
        assert_eq!(status, 500, "the status still reaches the caller");
        assert!(
            detail.starts_with("zzz"),
            "the vendor's opening words survive: {detail}"
        );
        assert!(
            detail.contains(&format!("ran past {MAX_REFUSAL_BYTES} bytes")),
            "and the operator is told the rest was never read: {detail}"
        );
        assert!(
            detail.chars().count() < 700,
            "trim still owns what reaches the log; this is the read, not the \
             cut: {} characters",
            detail.chars().count()
        );
    }

    /// **`i64::MIN` IS THIS STORE'S NULL, SO NO VENDOR MAY SEND IT AS A COUNT.**
    ///
    /// `CLAUDE.md` §7: `i64::MIN` is the open-interest null and zero means zero.
    /// `fetch::land` writes that sentinel for a field the vendor did not send —
    /// so a vendor sending it literally decoded here as an ordinary number,
    /// passed every check below, landed, and read back off disk as *no open
    /// interest at all*. Nothing recorded the reinterpretation in either
    /// direction.
    ///
    /// **A NEGATIVE VOLUME IS REFUSED WHERE THE VENDOR STILL OWNS IT**, in both
    /// response shapes.
    ///
    /// Measured: Dhan sent `volume: -95342` on ADANIENT. It decoded cleanly
    /// here, survived `land`, folded, and died ~1,500 lines later at
    /// `store::file::survey` as `ImpossibleCount` — taking the whole
    /// instrument-month, its seven derived rungs, and every later month in the
    /// batch. The operator's message named batch record 2333, which is an index
    /// into a per-month slice they cannot open, and printed the open-interest
    /// null beside it as a second suspect.
    ///
    /// The refusal names the field and the value, so the failure arrives with
    /// the chunk and the date range still attached.
    #[test]
    fn a_negative_volume_is_refused_where_the_vendor_still_owns_the_value() {
        // The columnar shape, and the value the operator actually received.
        // TWO ROWS, because the window now survives one bad one and a single
        // row could not tell a dropped row from a refused window.
        let body = r#"{"open":[1,1],"high":[1,1],"low":[1,1],"close":[1,1],
                       "volume":[7,-95342],"timestamp":[1751337900,1751337960]}"#;
        let kept = decode_body(
            body,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Equity,
        )
        .expect("the impossible row goes; the window does not");
        assert_eq!(kept.rows.len(), 1, "only the good row survives");
        assert_eq!(kept.rows[0].volume, 7, "and it is the good one");

        // And the unit, directly, so a future caller cannot lose the rule by
        // routing around `decode_body`.
        for bad in [-1_i64, -95_342, i64::MIN + 1] {
            assert!(
                one_volume(
                    &serde_json::json!(bad),
                    "volume",
                    crate::vendor::Listing::Equity,
                    &mut 0
                )
                .is_err(),
                "{bad} is not a count of shares"
            );
        }
        assert_eq!(
            one_volume(
                &serde_json::json!(0),
                "volume",
                crate::vendor::Listing::Equity,
                &mut 0
            )
            .expect("zero is a real zero"),
            0,
            "zero means zero — it is not an absence"
        );
    }

    /// **AN INDEX'S NEGATIVE VOLUME IS THE ZERO ITS COLUMN ALWAYS IS**, and an
    /// equity's is still refused.
    ///
    /// NIFTY and BANKNIFTY are computed weighted averages — no share of an
    /// index changes hands, so the volume column has no referent. Measured
    /// across 6,493 stored BANKNIFTY minute bars, the only value Dhan ever
    /// sends there is `0`, and the operator confirms index volume is normally
    /// zero.
    ///
    /// So `-2` and `-125` are not a quantity being lost; they are noise in a
    /// structurally empty column. Refusing over them cost the whole 90-day
    /// chunk: BANKNIFTY's minute backfill stopped at chunk 1 of 21 and NIFTY's
    /// at chunk 3, every later month absent, while the OHLC in those chunks was
    /// good.
    ///
    /// **The rule turns on whether the column can carry a quantity, never on
    /// the sign alone.** On an equity a negative volume means shares DID trade
    /// and the decoder is reading the wrong column, so zero would be a lie —
    /// that half is D-0323 and is asserted here beside the change, because a
    /// test for the new behaviour that did not also pin the old one would let
    /// the refusal be widened away by accident.
    #[test]
    fn an_index_negative_volume_is_zero_and_an_equitys_is_still_refused() {
        let body = r#"{"open":[1],"high":[1],"low":[1],"close":[1],
                       "volume":[-125],"timestamp":[1751337900]}"#;

        let window = decode_body(
            body,
            &spec(PriceScale::Rupees),
            crate::vendor::Listing::Index,
        )
        .expect("an index has no volume, so noise in that column is not a refusal");
        assert_eq!(
            window.rows[0].volume, 0,
            "recorded as the zero the column always is"
        );
        // AND THE OHLC SURVIVES, which is the whole point: the bar is good and
        // only the empty column was noisy.
        assert_eq!(window.rows[0].open, 100, "the price is untouched");

        // ON A TRADED LISTING THE ROW GOES AND THE WINDOW STAYS. Two rows so
        // there is something left to assert on: a one-row body would decode to
        // an empty window and could not tell "the bad row was dropped" from
        // "the whole thing was refused", which is the distinction this change
        // is entirely about.
        let pair = r#"{"open":[1,2],"high":[1,2],"low":[1,2],"close":[1,2],
                       "volume":[5,-125],"timestamp":[1751337900,1751337960]}"#;
        for listing in [
            crate::vendor::Listing::Equity,
            crate::vendor::Listing::Derivative,
        ] {
            let kept = decode_body(pair, &spec(PriceScale::Rupees), listing)
                .expect("one impossible row does not refuse the window");
            assert_eq!(
                kept.rows.len(),
                1,
                "{listing:?}: the negative row is dropped and the good one is \
                 kept — this used to refuse all of it, which is what cost \
                 ADANIENT every intraday rung it had"
            );
            assert_eq!(
                kept.rows[0].volume, 5,
                "{listing:?}: and it is the GOOD row that survived, not the \
                 bad one zero-filled — a zero volume on a traded listing would \
                 be the lie D-0323 refuses"
            );
            assert_eq!(kept.rows[0].open, 100, "{listing:?}: its price too");
        }

        // AND ZERO ITSELF IS UNTOUCHED ON EVERY LISTING. Zero is a real zero,
        // not an absence — `CLAUDE.md` §7 — and it is what an index sends on
        // every ordinary bar.
        let zero = r#"{"open":[1],"high":[1],"low":[1],"close":[1],
                       "volume":[0],"timestamp":[1751337900]}"#;
        for listing in [
            crate::vendor::Listing::Index,
            crate::vendor::Listing::Equity,
            crate::vendor::Listing::Derivative,
        ] {
            let window = decode_body(zero, &spec(PriceScale::Rupees), listing)
                .expect("zero is a real volume on every listing");
            assert_eq!(window.rows[0].volume, 0);
        }
    }

    /// **A NEGATIVE TIMESTAMP IS STILL ACCEPTED**, and this is why the floor is
    /// on `one_volume` rather than on [`one_number`].
    ///
    /// `session::IstMoment::from_epoch_secs` adds `IST_OFFSET_SECS` — 19,800 —
    /// before it tests the sign, so every epoch second in `-19_800..0` is a real
    /// IST moment on 1970-01-01. A blanket `n < 0` inside `one_number` would
    /// have refused a stamp the session filter admits, which is a behaviour
    /// change with no source behind it and exactly the kind of collateral the
    /// null-sentinel test above exists to catch.
    #[test]
    fn a_negative_timestamp_is_not_a_negative_count() {
        for early in [-1_i64, -19_800, -12_345] {
            assert_eq!(
                one_number(&serde_json::json!(early), "timestamp").expect("a legal IST moment"),
                early,
                "{early} is 1970-01-01 in IST, not an impossible count"
            );
        }
    }

    /// Its neighbour still passes, because a rule that refuses the value next to
    /// the one it means is a second defect wearing the first one's clothes.
    #[test]
    fn the_null_sentinel_is_refused_where_the_vendor_still_owns_the_value() {
        let why = one_number(&serde_json::json!(i64::MIN), "open_interest")
            .expect_err("the store's null is not a number a vendor may send");
        let FetchError::TransportFailed { detail } = why else {
            panic!("the variant a bad field already uses, not a new one")
        };
        assert!(
            detail.contains("open_interest"),
            "the field is named: {detail}"
        );
        assert!(
            detail.contains("-9223372036854775808"),
            "and so is the value that arrived: {detail}"
        );

        // The neighbour, zero, and an ordinary count all still decode.
        assert_eq!(
            one_number(&serde_json::json!(i64::MIN + 1), "open_interest")
                .expect("one above the sentinel is an ordinary count"),
            i64::MIN + 1
        );
        assert_eq!(
            one_number(&serde_json::json!(0), "volume").expect("zero is a real zero"),
            0
        );
        assert_eq!(
            one_number(&serde_json::json!(41), "open_interest").expect("an ordinary count"),
            41
        );
    }

    /// And the refusal reaches the whole decode, not just the leaf.
    ///
    /// One bar, seven arrays, and the sentinel in the open-interest column: the
    /// window must not come back. Asserted through `decode_body` rather than
    /// through `one_number` alone because the leaf being right is worth nothing
    /// if the caller swallows it.
    #[test]
    fn a_window_carrying_the_null_sentinel_does_not_decode() {
        let named = HttpSpec {
            fields: FieldNames {
                open_interest: Some("open_interest"),
                ..spec(PriceScale::Rupees).fields
            },
            ..spec(PriceScale::Rupees)
        };
        let sane = r#"{"open":[24500.75],"high":[24500.75],"low":[24500.75],
                       "close":[24500.75],"volume":[250],"timestamp":[1751337900],
                       "open_interest":[41]}"#;
        let window = decode_body(sane, &named, crate::vendor::Listing::Equity)
            .expect("an ordinary open interest decodes");
        assert_eq!(window.rows.len(), 1, "the happy path still decodes");

        let sentinel = r#"{"open":[24500.75],"high":[24500.75],"low":[24500.75],
                          "close":[24500.75],"volume":[250],"timestamp":[1751337900],
                          "open_interest":[-9223372036854775808]}"#;
        let why = decode_body(sentinel, &named, crate::vendor::Listing::Equity)
            .expect_err("the sentinel is refused");
        assert!(
            format!("{why}").contains("open_interest"),
            "and the refusal names the column: {why}"
        );
    }

    /// **A NEGATIVE OPEN INTEREST SKIPS ITS ROW — AND `i64::MIN` STILL REFUSES
    /// THE WINDOW, LOUDLY.**
    ///
    /// Two rules that look alike and are not, which is why they are asserted
    /// together. An open interest is contracts outstanding and is never below
    /// zero, so `-5` is a count that cannot be a count: the row goes, exactly
    /// as a negative volume's does. But `i64::MIN` is the value `CLAUDE.md` §7
    /// spends on *"the vendor sent no open interest"*, so a vendor sending it
    /// literally is a SENTINEL COLLISION rather than a bad reading — stored, it
    /// would read back as an absence — and it must be shouted about, not
    /// quietly skipped.
    ///
    /// The first draft of this guard got that exactly wrong and an existing
    /// test caught it: written as
    /// `as_i64().is_some_and(..).unwrap_or_else(|| as_f64()..)`, the sentinel
    /// passed the integer predicate, fell through to the float fallback, came
    /// back `-9.22e18`, and was skipped — swallowing the one case that needs to
    /// be loud.
    #[test]
    fn a_negative_open_interest_skips_its_row_and_the_sentinel_still_shouts() {
        let named = HttpSpec {
            fields: FieldNames {
                open_interest: Some("open_interest"),
                ..spec(PriceScale::Rupees).fields
            },
            ..spec(PriceScale::Rupees)
        };

        // AN ORDINARY NEGATIVE: the row goes, the window and its neighbour stay.
        let body = r#"{"open":[24500.75,24501.00],"high":[24500.75,24501.00],
                       "low":[24500.75,24501.00],"close":[24500.75,24501.00],
                       "volume":[250,260],"timestamp":[1751337900,1751337960],
                       "open_interest":[41,-5]}"#;
        let window = decode_body(body, &named, crate::vendor::Listing::Equity)
            .expect("one impossible count does not refuse the window");
        assert_eq!(window.rows.len(), 1, "the negative row went");
        assert_eq!(window.rows[0].volume, 250, "and the good row stayed");
        assert_eq!(
            window.rows[0].open_interest,
            Some(41),
            "with its own open interest"
        );

        // THE SENTINEL, BESIDE A GOOD ROW: still a refusal, never a skip. A
        // second row is present precisely so a skip would leave something
        // behind and read as success — without it, an empty window and a
        // refusal would be hard to tell apart.
        let sentinel = r#"{"open":[24500.75,24501.00],"high":[24500.75,24501.00],
                           "low":[24500.75,24501.00],"close":[24500.75,24501.00],
                           "volume":[250,260],"timestamp":[1751337900,1751337960],
                           "open_interest":[41,-9223372036854775808]}"#;
        let why = decode_body(sentinel, &named, crate::vendor::Listing::Equity)
            .expect_err("the sentinel is refused, not skipped");
        assert!(
            format!("{why}").contains("open_interest"),
            "and the refusal names the column: {why}"
        );

        // ZERO IS A REAL MEASUREMENT ON EVERY LISTING. §7: zero means zero, and
        // an open interest of nothing is a fact a derivative reports daily.
        let zero = r#"{"open":[24500.75],"high":[24500.75],"low":[24500.75],
                       "close":[24500.75],"volume":[250],"timestamp":[1751337900],
                       "open_interest":[0]}"#;
        let window = decode_body(zero, &named, crate::vendor::Listing::Derivative)
            .expect("zero open interest is a reading, not an absence");
        assert_eq!(window.rows.len(), 1, "zero is not negative");
        assert_eq!(window.rows[0].open_interest, Some(0), "and it is a zero");

        // AND THE FLOAT SPELLING, which is a separate branch and was a separate
        // hole. `serde_json` answers `as_i64()` for `-5` and `None` for `-5.0`,
        // so a guard that read only the integer spelling would pass every
        // float-encoded negative straight through. `cargo mutants` proved this
        // branch untested three times over — `<` survived being turned into
        // `==`, `>` and `<=`, because nothing sent a non-integer here at all.
        for sent in ["-5.0", "-0.5"] {
            let body = format!(
                "{{\"open\":[24500.75,24501.00],\"high\":[24500.75,24501.00],\
                   \"low\":[24500.75,24501.00],\"close\":[24500.75,24501.00],\
                   \"volume\":[250,260],\"timestamp\":[1751337900,1751337960],\
                   \"open_interest\":[41,{sent}]}}"
            );
            let window = decode_body(&body, &named, crate::vendor::Listing::Derivative)
                .unwrap_or_else(|why| panic!("{sent} skips its row, not the window: {why}"));
            assert_eq!(window.rows.len(), 1, "{sent}: the negative row went");
            assert_eq!(
                window.rows[0].open_interest,
                Some(41),
                "{sent}: and the good row stayed"
            );
        }

        // A POSITIVE FLOAT IS NOT COLLATERAL DAMAGE — a whole number sent with
        // a decimal point is still a count, and `one_number` accepts it.
        //
        // `0.0` IS THE ROW THAT PINS THE BOUNDARY. The comparison is `< 0.0`,
        // and `cargo mutants` turned it into `<= 0.0` and survived everything
        // above: no case sent a float zero, so nothing noticed that every
        // contract with no open interest — which is most of the option chain,
        // every day — would have had its bar deleted. §7 again: zero means
        // zero, and it is a measurement rather than an absence.
        let float_ok = r#"{"open":[24500.75,24501.00],"high":[24500.75,24501.00],
                           "low":[24500.75,24501.00],"close":[24500.75,24501.00],
                           "volume":[250,260],"timestamp":[1751337900,1751337960],
                           "open_interest":[41.0,0.0]}"#;
        let window = decode_body(float_ok, &named, crate::vendor::Listing::Derivative)
            .expect("41.0 is a count of contracts and 0.0 is a count of none");
        assert_eq!(
            window.rows.len(),
            2,
            "BOTH rows are kept — a float zero is not a negative"
        );
        assert_eq!(window.rows[0].open_interest, Some(41));
        assert_eq!(
            window.rows[1].open_interest,
            Some(0),
            "and zero open interest survives as the zero it is"
        );
    }

    /// One window, fetched over a real socket, end to end.
    ///
    /// Everything else here drives one seam. This drives all of them at once —
    /// the header, the wire dates, the status check, the size bound and the
    /// decode — because until this test existed `window_async`'s success path
    /// was the one part of the vendor client no test had ever entered.
    #[test]
    fn a_window_is_fetched_decoded_and_returned_over_a_real_socket() {
        let body = r#"{"open":[24500.75],"high":[24500.75],"low":[24500.75],
                       "close":[24500.75],"volume":[250],"timestamp":[1751337900],
                       "open_interest":[41]}"#;
        let (url, seen, _) = listener(Some(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )));
        let spec = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            fields: FieldNames {
                open_interest: Some("open_interest"),
                ..spec(PriceScale::Rupees).fields
            },
            ..spec(PriceScale::Rupees)
        };
        let window = source_of(spec, "SUPERSECRET")
            .block_on_window()
            .expect("a window comes back");
        let row = &window.rows[0];
        assert_eq!(row.open, 2_450_075, "the paise survive the whole path");
        assert_eq!(row.volume, 250);
        assert_eq!(
            row.open_interest,
            Some(41),
            "a declared field is read, not zeroed"
        );

        // And the request that produced it carried the descriptor's own header
        // and the descriptor's own wire dates — `toDate` exclusive, so the day
        // AFTER the operator's last day.
        let sent = seen
            .recv_timeout(core::time::Duration::from_secs(5))
            .expect("the vendor was contacted");
        assert!(
            sent.contains("access-token: SUPERSECRET") || sent.contains("x-token: SUPERSECRET")
        );
        assert!(sent.contains("2025-07-01"), "fromDate: {sent}");
        assert!(sent.contains("2025-07-02"), "toDate is exclusive: {sent}");
    }

    /// A `GET` vendor puts its window in the query string, not in a body.
    #[test]
    fn a_get_vendor_carries_its_window_in_the_query_string() {
        let (url, seen, _) = listener(Some(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\
             Connection: close\r\n\r\n"
                .to_owned(),
        ));
        let spec = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            method: Method::Get,
            range_end: RangeEnd::Inclusive,
            auth: crate::vendor::Auth {
                // Spelled as `crate::vendor` spells Groww's, capital and all:
                // this fixture is meant to be that descriptor's shape, and a
                // lowercase copy would be a second spelling of one name.
                header: "Authorization",
                scheme: AuthScheme::Bearer,
                key_field: None,
            },
            ..spec(PriceScale::Rupees)
        };
        let outcome = source_of(spec, "SUPERSECRET").block_on_window();
        let sent = seen
            .recv_timeout(core::time::Duration::from_secs(5))
            .expect("the vendor was contacted");
        assert!(sent.starts_with("GET /bars?"), "a GET with a query: {sent}");
        assert!(sent.contains("from=2025-07-01"), "{sent}");
        assert!(
            sent.contains("to=2025-07-01"),
            "an inclusive vendor takes the day unchanged: {sent}"
        );
        // Header names are case-insensitive and the client writes them
        // lowercased on the wire, so the haystack is folded rather than the
        // needle being written out in the wire's own casing.
        assert!(
            sent.to_lowercase()
                .contains("authorization: bearer supersecret"),
            "the Bearer scheme is a prefix on the value, not a second header: {sent}"
        );
        // A 5xx is not the governor's business and is not a redirect either, so
        // it carries neither redirect wording nor a body it does not have.
        let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
            panic!("500 is a refusal")
        };
        assert_eq!(status, 500);
        assert!(
            detail.is_empty(),
            "no body, and nothing invented: {detail:?}"
        );
    }

    /// A redirect with no `Location`, and a redirect that also says something.
    ///
    /// Both arms of the refusal's wording, which the two-socket test above does
    /// not reach: it always sends a `Location` and never a body.
    #[test]
    fn a_redirect_says_so_with_or_without_a_location_and_with_or_without_a_body() {
        let bare = "HTTP/1.1 307 Temporary Redirect\r\nContent-Length: 0\r\n\
                    Connection: close\r\n\r\n";
        let (url, _seen, _) = listener(Some(bare.to_owned()));
        let no_location = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            ..spec(PriceScale::Rupees)
        };
        let Err(FetchError::VendorRefused { status, detail, .. }) =
            source_of(no_location, "SUPERSECRET").block_on_window()
        else {
            panic!("a 307 is still a redirect this build refuses")
        };
        assert_eq!(status, 307);
        assert!(detail.contains("no Location header"), "{detail}");
        assert!(detail.contains("does not follow redirects"), "{detail}");
        // AN EMPTY BODY ADDS NOTHING. Without the `body.is_empty()` guard the
        // refusal ends `— and it said: ` with nothing after it, which reads as
        // a vendor message that was lost rather than one that never existed.
        assert!(
            !detail.contains("and it said"),
            "a 307 with no body must not claim the vendor said something: {detail}"
        );
        assert!(
            detail.ends_with("no Location header"),
            "the refusal ends where the facts do: {detail}"
        );

        // And one that redirects AND explains itself.
        let chatty = "moved, ask elsewhere";
        let (url, _seen, _) = listener(Some(format!(
            "HTTP/1.1 301 Moved Permanently\r\nLocation: https://elsewhere.invalid/x\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{chatty}",
            chatty.len()
        )));
        let with_body = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            ..spec(PriceScale::Rupees)
        };
        let Err(FetchError::VendorRefused { status, detail, .. }) =
            source_of(with_body, "SUPERSECRET").block_on_window()
        else {
            panic!("a 301 is still a redirect this build refuses")
        };
        assert_eq!(status, 301);
        assert!(detail.contains("elsewhere.invalid"), "the target: {detail}");
        assert!(detail.contains(chatty), "and the vendor's words: {detail}");
        assert!(!detail.contains("SUPERSECRET"), "never the token: {detail}");
    }

    /// The object shape no longer refuses on principle — it refuses on SHAPE.
    ///
    /// This test used to assert "no decoder was written", which was true and is
    /// not any more: Groww's descriptor declares one object per bar, so the
    /// decoder exists. What must still hold is that a body which is *not* that
    /// shape is refused by name rather than guessed at, and that the refusal
    /// says what it did find.
    #[test]
    fn an_object_shape_body_that_is_not_a_list_of_bars_is_refused_by_name() {
        let shape = HttpSpec {
            response: ResponseShape::ArrayOfObjects { envelope: None },
            ..spec(PriceScale::Rupees)
        };
        let Err(FetchError::TransportFailed { detail }) =
            decode_body("{}", &shape, crate::vendor::Listing::Equity)
        else {
            panic!("an empty object holds no bars and must be refused")
        };
        assert!(
            detail.contains("one object per bar"),
            "the refusal names the shape it expected: {detail}"
        );
        assert!(
            detail.contains("no keys at all"),
            "and what it actually found: {detail}"
        );
    }

    /// The blocking seam refuses by name rather than spinning a runtime per call.
    #[test]
    fn the_blocking_seam_refuses_and_names_the_asynchronous_one() {
        let source = HttpSource::new(
            spec(PriceScale::Rupees),
            Credential::token("SUPERSECRET".to_owned()),
        )
        .expect("a client builds");
        let Err(FetchError::TransportFailed { detail }) = source.window(&one_day()) else {
            panic!("the sync seam cannot work and must say so")
        };
        assert!(
            detail.contains("window_async"),
            "it names the one that does: {detail}"
        );
        assert!(!detail.contains("SUPERSECRET"), "{detail}");
    }

    /// A count written as a decimal means the count, and a fractional one does
    /// not mean anything.
    ///
    /// `250.0` of something is two hundred and fifty; `250.5` of something is a
    /// shape this build does not understand, and it says so rather than
    /// truncating. Same text parser as a price, with the hundredths required to
    /// be zero.
    #[test]
    fn a_count_written_as_a_decimal_is_read_and_a_fractional_one_is_refused() {
        let whole = r#"{"open":[1],"high":[1],"low":[1],"close":[1],
                        "volume":[250.0],"timestamp":[1751337900.0]}"#;
        let window = decode_body(
            whole,
            &spec(PriceScale::Paisa),
            crate::vendor::Listing::Equity,
        )
        .expect("decodes");
        assert_eq!(window.rows[0].volume, 250, "250.0 of anything is 250");
        assert_eq!(window.rows[0].timestamp, 1_751_337_900);

        for bad in ["250.5", "\"250\"", "250.005"] {
            let body = format!(
                "{{\"open\":[1],\"high\":[1],\"low\":[1],\"close\":[1],\
                  \"volume\":[{bad}],\"timestamp\":[1]}}"
            );
            let Err(FetchError::TransportFailed { detail }) = decode_body(
                &body,
                &spec(PriceScale::Paisa),
                crate::vendor::Listing::Equity,
            ) else {
                panic!("{bad} is not a whole count and must be refused, not truncated")
            };
            assert!(detail.contains("volume"), "the refusal names it: {detail}");
        }
    }

    /// A socket that never answers is a named transport failure, and the
    /// refusal cannot carry the credential — `reqwest`'s own words never
    /// include a header this code set.
    #[test]
    fn a_host_that_cannot_be_reached_is_named_and_never_carries_the_token() {
        // Port 1 on loopback: bound by nothing, and refused immediately rather
        // than left to the 30-second timeout.
        let spec = HttpSpec {
            base_url: "http://127.0.0.1:1",
            ..spec(PriceScale::Rupees)
        };
        let Err(FetchError::TransportFailed { detail }) =
            source_of(spec, "SUPERSECRET").block_on_window()
        else {
            panic!("an unreachable host is a transport failure, not a window")
        };
        assert!(detail.contains("127.0.0.1:1"), "the URL is named: {detail}");
        assert!(detail.contains("was not reached"), "{detail}");
        assert!(
            !detail.contains("SUPERSECRET"),
            "the token leaked: {detail}"
        );
    }

    /// The last day this calendar can name has no successor to put on the wire,
    /// so an exclusive vendor is refused there rather than wrapping.
    #[test]
    fn the_last_nameable_day_has_no_exclusive_successor_and_says_so() {
        let last = crate::session::Day::new(9999, 12, 31).expect("the last day");
        let Err(FetchError::TransportFailed { detail }) =
            HttpSource::wire_end(last, RangeEnd::Exclusive, DateFormat::DashedYmd)
        else {
            panic!("9999-12-31 has no day after it")
        };
        assert!(detail.contains("9999-12-31"), "the day is named: {detail}");
        // An inclusive vendor takes it unchanged, because it needs no successor.
        assert_eq!(
            HttpSource::wire_end(last, RangeEnd::Inclusive, DateFormat::DashedYmd)
                .expect("unchanged"),
            "9999-12-31"
        );
    }

    /// EVERY FEED'S WIRE END REACHES THE LAST SESSION AND NO FURTHER — AND THE
    /// TWO FEEDS GET THERE BY OPPOSITE ROUTES.
    ///
    /// The operator names an INCLUSIVE window: the last day they typed is a day
    /// they want. What each vendor takes for that is its own business, and the
    /// two shipped rows disagree:
    ///
    ///   * Dhan — `toDate` is documented `End date (YYYY-MM-DD, non-inclusive)`
    ///     on its daily endpoint, so the wire value is the day AFTER. Forward
    ///     the operator's day verbatim and the last session is missing, one day
    ///     per request, and a short window is indistinguishable from a holiday.
    ///   * Groww — `end_time` is an instant and is taken inclusively, so the
    ///     day is unchanged and the clock is pushed to the end of it. Adding a
    ///     day HERE would be the blanket `+1` this test exists to refuse: it
    ///     would pull a session outside the window the operator asked for.
    ///
    /// So the property is stated once, over both, in terms neither vendor's
    /// convention appears in: the wire end must fall strictly after the last
    /// print of the session named, and strictly before the first print of the
    /// next one. Both wire formats are ISO-ordered — `YYYY-MM-DD` and
    /// `YYYY-MM-DD HH:MM:SS` — so byte order IS instant order, and one
    /// comparison serves both.
    ///
    /// No socket: this is the value the request builder puts in the `To`
    /// parameter, taken from the same function `window_async` calls.
    #[test]
    fn every_feeds_wire_end_covers_the_last_session_and_not_the_next() {
        // A Friday that traded. `docs/00-charter.md` §3: the regular session
        // runs 09:15 to 15:30 IST and the last one-minute bar OPENS at 15:29,
        // so these two strings are the last instant that must be inside the
        // request and the first that must be outside it.
        let last = crate::session::Day::new(2026, 8, 7).expect("a real date");
        let last_print = "2026-08-07 15:29:00";
        let next_open = "2026-08-08 09:15:00";

        let mut checked = 0;
        for feed in crate::vendor::Feed::ALL {
            let crate::vendor::Transport::Http(spec) = feed.descriptor().transport else {
                continue;
            };
            let wire = HttpSource::wire_end(last, spec.range_end, spec.date_format)
                .expect("2026 has a successor");
            assert!(
                wire.as_str() > last_print,
                "{feed} asks to {wire}, which stops before the last bar of the \
                 session the operator named — the window comes back one session \
                 short and nothing downstream can tell that from a holiday"
            );
            assert!(
                wire.as_str() < next_open,
                "{feed} asks to {wire}, which reaches into the next session — a \
                 blanket +1 against a vendor whose bound is already inclusive"
            );
            checked += 1;
        }
        assert_eq!(checked, 3, "every HTTP feed was checked, not one twice");
    }

    /// WHICH VENDOR IS WHICH, AND THE OFF-BY-ONE IS LOAD-BEARING.
    ///
    /// The test above passes for both feeds without saying that they differ.
    /// This one says it: swap either row's `RangeEnd` and the request breaks in
    /// the direction that row's convention makes silent.
    #[test]
    fn the_range_end_is_a_per_vendor_fact_and_the_wrong_one_loses_a_session() {
        let last = crate::session::Day::new(2026, 8, 7).expect("a real date");
        let last_print = "2026-08-07 15:29:00";

        // Dhan: documented NON-INCLUSIVE. Forwarding the operator's day
        // verbatim stops at midnight ON that day, before the session opened.
        let dhan = match crate::vendor::Feed::Dhan.descriptor().transport {
            crate::vendor::Transport::Http(spec) => spec,
            crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
        };
        assert_eq!(
            dhan.range_end,
            RangeEnd::Exclusive,
            "Dhan Docs / 12-historical-data.md, Get Daily Historical Data: \
             toDate is End date (YYYY-MM-DD, non-inclusive)"
        );
        assert!(
            HttpSource::wire_end(last, RangeEnd::Inclusive, dhan.date_format)
                .expect("unchanged")
                .as_str()
                < last_print,
            "if this row were inclusive the request would end before the \
             session it names, which is the silent loss the field prevents"
        );

        // Groww: an instant, taken inclusively. The successor a blanket +1
        // would send lands INSIDE the next session.
        let groww = match crate::vendor::Feed::Groww.descriptor().transport {
            crate::vendor::Transport::Http(spec) => spec,
            crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
        };
        assert_eq!(
            groww.range_end,
            RangeEnd::Inclusive,
            "Groww Docs / 08-historical-data.md: the window is a pair of \
             instants, start_time and end_time, not a pair of dates"
        );
        let correct = HttpSource::wire_end(last, groww.range_end, groww.date_format)
            .expect("an inclusive vendor needs no successor");
        let blanket = HttpSource::wire_end(last, RangeEnd::Exclusive, groww.date_format)
            .expect("the day after");
        assert!(
            correct.starts_with("2026-08-07"),
            "the operator's own last day, with the clock pushed to the end of \
             it: {correct}"
        );
        assert!(
            blanket.starts_with("2026-08-08"),
            "a blanket +1 names a DAY OUTSIDE the window the operator asked \
             for: {blanket}. Harmless-looking at the minute rung, where the \
             extra span holds no bars, and not harmless at the day rung, where \
             this vendor stamps a daily bar at midnight."
        );
    }

    /// The two bounds this module states as numbers, asserted as numbers.
    ///
    /// `64 * 1024 * 1024` is a product, and `64 + 1024 + 1024` is 2,112 — a
    /// bound that would refuse every real answer while still reading like a
    /// generous one. Nothing else in the suite looks at the value.
    #[test]
    fn the_stated_bounds_are_the_numbers_they_are_written_as() {
        assert_eq!(MAX_RESPONSE_BYTES, 67_108_864, "64 MiB, not 64+1024+1024");
        assert_eq!(REQUEST_TIMEOUT_SECS, 30);

        // And the size bound is tight: the last acceptable answer is exactly
        // MAX_RESPONSE_BYTES, and one byte more is refused. Asserted here
        // rather than over a real body, because a test that allocates 64 MiB to
        // check a `>` is a test nobody runs.
        assert!(!too_large(0), "an empty answer is not too large");
        assert!(!too_large(MAX_RESPONSE_BYTES - 1));
        assert!(
            !too_large(MAX_RESPONSE_BYTES),
            "the bound is inclusive: exactly the maximum is accepted"
        );
        assert!(
            too_large(MAX_RESPONSE_BYTES + 1),
            "and one byte past it is refused"
        );
    }

    /// A one-day request, which is every socket test's window.
    fn one_day() -> BarRequest {
        BarRequest {
            instrument_id: String::new(),
            listing: crate::vendor::Listing::Equity,
            window: crate::session::Window::new(
                crate::session::Day::new(2025, 7, 1).expect("a real day"),
                crate::session::Day::new(2025, 7, 1).expect("a real day"),
            )
            .expect("a real window"),
            granularity: crate::vendor::Granularity::Minute1,
        }
    }

    /// A source, and a runtime to drive its one asynchronous method.
    // `HttpSpec` passed by value here rather than by reference, and clippy is
    // right that it is now over the 256-byte threshold: it grew a parameter map
    // and a header list. It is `Copy`, this is a test helper called a handful
    // of times, and taking it by reference would make every fixture write `&`
    // for no gain a profile could measure.
    #[allow(
        clippy::large_types_passed_by_value,
        reason = "a Copy descriptor row in a test helper; the copy is the point"
    )]
    fn source_of(spec: HttpSpec, token: &str) -> Driven {
        Driven(HttpSource::new(spec, Credential::token(token.to_owned())).expect("a client builds"))
    }

    struct Driven(HttpSource);

    impl Driven {
        fn block_on_window(&self) -> Result<RawWindow, FetchError> {
            self.block_on(&one_day())
        }

        fn block_on(&self, request: &BarRequest) -> Result<RawWindow, FetchError> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime")
                .block_on(self.0.window_async(request))
        }
    }

    /// **THE RUNG GOES ON THE WIRE, AND A RUNG WITH NO RECORDED WORD REFUSES
    /// BEFORE THE SOCKET.**
    ///
    /// A feed that names its bar length in a request field — Groww's
    /// `candle_interval` — must send the length the answer will be FILED under.
    /// It used to be `ParamValue::Fixed("1minute")`, so a request the operator
    /// filed as daily still asked the vendor for minutes; the answer would fold
    /// to a correct-looking daily bar and the window would have gone out against
    /// a cap that was measured for a different rung.
    ///
    /// The other half is the one `CLAUDE.md` §3 rule 1 decides. `1day`, `1d` and
    /// `day` are all plausible spellings and only one of them is a request; no
    /// source in this repository records which. So an unrecorded rung is refused
    /// **by name**, nothing is sent, and the day the word is read live it is one
    /// row in `granularity_tokens`.
    #[test]
    fn the_rung_is_named_on_the_wire_and_an_unrecorded_one_refuses_before_the_socket() {
        let (url, seen, _) = listener(Some(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\
             Connection: close\r\n\r\n"
                .to_owned(),
        ));
        // Groww's shape: a GET whose interval is a request parameter.
        let spec = HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            method: Method::Get,
            listings: &[
                crate::vendor::ListingWords {
                    listing: crate::vendor::Listing::Index,
                    segment: "IDX_I",
                    kind: "INDEX",
                },
                crate::vendor::ListingWords {
                    listing: crate::vendor::Listing::Equity,
                    segment: "NSE_EQ",
                    kind: "EQUITY",
                },
            ],
            params: &[
                crate::vendor::Param {
                    name: "from",
                    value: crate::vendor::ParamValue::From,
                },
                crate::vendor::Param {
                    name: "candle_interval",
                    value: crate::vendor::ParamValue::Granularity,
                },
            ],
            ..spec(PriceScale::Rupees)
        };
        let source = source_of(spec, "SUPERSECRET");

        // THE RECORDED RUNG REACHES THE VENDOR IN THE VENDOR'S OWN WORD —
        // `1minute`, not `1min`, which is the STORE's spelling for the same
        // rung. Two vocabularies, one field, and the descriptor is what
        // translates.
        let _ = source.block_on(&one_day());
        let sent = seen
            .recv_timeout(core::time::Duration::from_secs(5))
            .expect("the vendor was contacted");
        assert!(
            sent.contains("candle_interval=1minute"),
            "the rung is on the wire in this feed's own spelling: {sent}"
        );
        assert_eq!(
            crate::vendor::Granularity::Minute1.dir(),
            "1min",
            "and it is NOT the store's spelling, which is what a `Fixed` word \
             here would have had no way to be wrong about"
        );

        // AND THE UNRECORDED RUNG NEVER LEAVES THE PROCESS.
        let daily = BarRequest {
            granularity: crate::vendor::Granularity::Day1,
            ..one_day()
        };
        let Err(FetchError::RungNotSpellable { rung, field }) = source.block_on(&daily) else {
            panic!(
                "a rung this feed has no recorded word for must refuse by name, \
                 not be sent as some other rung"
            );
        };
        assert_eq!(rung, crate::vendor::Granularity::Day1);
        assert_eq!(field, "candle_interval", "the descriptor row to amend");
        let why = FetchError::RungNotSpellable { rung, field }.to_string();
        assert!(why.contains("1day"), "the refusal names the rung: {why}");
        assert!(
            why.contains("candle_interval"),
            "and the field that would carry it: {why}"
        );
        assert!(
            why.contains("UNVERIFIED"),
            "and says the word is unrecorded rather than unsupported: {why}"
        );
        // NOTHING WAS SENT. The listener answers exactly one request and the
        // recorded rung above consumed it, so a second arrival here would mean
        // the daily request went out anyway.
        assert!(
            seen.recv_timeout(core::time::Duration::from_millis(250))
                .is_err(),
            "the refusal happens before the socket, not after the answer"
        );
    }

    /// Groww's declared shape, decoded: one object per bar under `payload`.
    ///
    /// This arm refused for as long as no vendor used it. Groww's descriptor
    /// declares it, so it is written — and written to read fields **by the
    /// names the descriptor gives**, never by position. A positional reader
    /// would file a high as a low the first time a vendor reordered its keys,
    /// and every value would still be a plausible price.
    #[test]
    fn one_object_per_bar_decodes_through_the_same_conversions() {
        let body = r#"{"payload":[
            {"open":24500.75,"high":24512.00,"low":24498.50,"close":24510.25,
             "volume":1200,"timestamp":1751341500},
            {"open":24510.25,"high":24518.75,"low":24505.00,"close":24515.00,
             "volume":980,"timestamp":1751341560}
        ]}"#;
        let spec = HttpSpec {
            response: ResponseShape::ArrayOfObjects {
                envelope: Some("payload"),
            },
            ..spec(PriceScale::Rupees)
        };
        let window = decode_body(body, &spec, crate::vendor::Listing::Equity).expect("decodes");
        assert_eq!(window.rows.len(), 2);
        // THE SAME PAISA CONVERSION as the column shape — one implementation,
        // so a rupee cannot be worth two different things depending on which
        // way the vendor happened to send it.
        assert_eq!(window.rows[0].open, 2_450_075);
        assert_eq!(window.rows[0].close, 2_451_025);
        assert_eq!(window.rows[1].high, 2_451_875);
        assert_eq!(window.rows[0].volume, 1200);
        assert_eq!(window.rows[1].timestamp, 1_751_341_560);
    }

    /// A field missing from ONE bar names the field **and which bar**.
    ///
    /// "the vendor sent 400 bars and one has no close" is a different fault
    /// from "the shape is wrong", and it sends an operator somewhere different.
    /// Filling in a default would put a bar on disk that looks entirely real.
    #[test]
    fn a_bar_missing_one_field_is_refused_by_field_and_by_index() {
        let body = r#"{"payload":[
            {"open":1,"high":1,"low":1,"close":1,"volume":1,"timestamp":1},
            {"open":1,"high":1,"low":1,"volume":1,"timestamp":2}
        ]}"#;
        let spec = HttpSpec {
            response: ResponseShape::ArrayOfObjects {
                envelope: Some("payload"),
            },
            ..spec(PriceScale::Paisa)
        };
        let Err(FetchError::TransportFailed { detail }) =
            decode_body(body, &spec, crate::vendor::Listing::Equity)
        else {
            panic!("a bar with no close must be refused, not defaulted")
        };
        assert!(detail.contains("close"), "the field: {detail}");
        assert!(detail.contains("bar 1"), "and which bar: {detail}");
        assert!(detail.contains("open"), "and what it did have: {detail}");
    }

    /// The declared envelope is honoured here too — D-0049 applies to both
    /// shapes, so one bar can never be spliced out of two objects.
    #[test]
    fn the_object_shape_reads_only_the_declared_envelope() {
        let body = r#"{
            "cached":[{"open":999,"high":999,"low":999,"close":999,"volume":1,"timestamp":1}],
            "payload":[{"open":100,"high":100,"low":100,"close":100,"volume":7,"timestamp":2}]
        }"#;
        let spec = HttpSpec {
            response: ResponseShape::ArrayOfObjects {
                envelope: Some("payload"),
            },
            ..spec(PriceScale::Paisa)
        };
        let window = decode_body(body, &spec, crate::vendor::Listing::Equity).expect("decodes");
        assert_eq!(window.rows.len(), 1, "only the named envelope is read");
        assert_eq!(window.rows[0].open, 100, "99900 would be `cached` leaking");
        assert_eq!(window.rows[0].volume, 7);
    }

    /// An envelope the answer does not carry is refused, naming what it has.
    #[test]
    fn the_object_shape_refuses_a_missing_envelope_by_name() {
        let spec = HttpSpec {
            response: ResponseShape::ArrayOfObjects {
                envelope: Some("payload"),
            },
            ..spec(PriceScale::Paisa)
        };
        let Err(FetchError::TransportFailed { detail }) =
            decode_body(r#"{"data":[]}"#, &spec, crate::vendor::Listing::Equity)
        else {
            panic!("the declared envelope is absent and must be named")
        };
        assert!(detail.contains("payload"), "expected: {detail}");
        assert!(detail.contains("data"), "present: {detail}");
    }

    /// An empty list is a real answer — no bars, and no refusal.
    #[test]
    fn an_empty_object_list_is_a_window_of_no_bars_and_not_a_failure() {
        let spec = HttpSpec {
            response: ResponseShape::ArrayOfObjects {
                envelope: Some("payload"),
            },
            ..spec(PriceScale::Paisa)
        };
        let window = decode_body(r#"{"payload":[]}"#, &spec, crate::vendor::Listing::Equity)
            .expect("decodes");
        assert!(window.rows.is_empty(), "no bars is not an error");
    }

    /// `wire_end` is the one conversion site for a non-inclusive `toDate`.
    #[test]
    fn the_wire_end_is_the_day_after_only_for_an_exclusive_vendor() {
        let last = crate::session::Day::new(2025, 7, 31).expect("a real day");
        assert_eq!(
            HttpSource::wire_end(last, RangeEnd::Exclusive, DateFormat::DashedYmd)
                .expect("has a successor"),
            "2025-08-01",
            "exclusive means the day AFTER goes on the wire"
        );
        assert_eq!(
            HttpSource::wire_end(last, RangeEnd::Inclusive, DateFormat::DashedYmd)
                .expect("unchanged"),
            "2025-07-31"
        );
    }

    /// Every date format the descriptor can declare is written, and written
    /// zero-padded, so a one-digit month cannot reach a vendor as `2025-7-1`.
    #[test]
    fn every_declared_date_format_is_written_zero_padded() {
        let day = crate::session::Day::new(2025, 7, 1).expect("a real day");
        for (format, want) in [
            (DateFormat::DashedYmd, "2025-07-01"),
            (DateFormat::CompactYmd, "20250701"),
            (DateFormat::SlashedDmy, "01/07/2025"),
            (DateFormat::CompactDmy, "01072025"),
        ] {
            assert_eq!(HttpSource::on_the_wire(day, format), want);
        }
    }

    /// DHAN'S TWO RUNGS GO TO TWO ENDPOINTS, and the minute one carries the
    /// field the daily one does not document.
    ///
    /// Driven off the SHIPPED descriptor rather than a fixture, because the
    /// thing under test is what this build will actually put on a socket.
    ///
    /// # The defect this closes
    ///
    /// `Minute1` was declared on this feed once and withdrawn. One `bars_path`
    /// meant the minute request went to `/v2/charts/historical` — the DAILY
    /// endpoint — which answered, with daily bars, that would then be filed
    /// under `1min/`. The bar length lives in the PATH in this store, so no
    /// later reader could tell those from real minute data. It did not fail;
    /// that is what made it dangerous.
    #[test]
    fn dhan_serves_the_two_rungs_from_two_endpoints_and_only_one_takes_an_interval() {
        let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("Dhan is an HTTP broker");
        };

        let day = spec.path_for(crate::vendor::Granularity::Day1);
        let minute = spec.path_for(crate::vendor::Granularity::Minute1);
        assert_ne!(
            day, minute,
            "the two rungs must not resolve to one endpoint: that is the defect"
        );
        let joined = |segs: &[crate::vendor::PathSegment]| {
            segs.iter()
                .map(|s| match *s {
                    crate::vendor::PathSegment::Literal(w) => w,
                    crate::vendor::PathSegment::Value { placeholder, .. } => placeholder,
                })
                .collect::<Vec<_>>()
                .join("/")
        };
        assert_eq!(
            joined(day),
            "v2/charts/historical",
            "the vendor's daily page"
        );
        assert_eq!(
            joined(minute),
            "v2/charts/intraday",
            "the vendor's intraday page"
        );

        // THE INTERVAL TRAVELS WITH THE MINUTE RUNG AND NOWHERE ELSE. In the
        // shared `params` list it would be sent to the daily endpoint too,
        // which does not document it.
        assert!(
            spec.route_for(crate::vendor::Granularity::Day1).is_none(),
            "the daily rung is the default endpoint and adds nothing"
        );
        let route = spec
            .route_for(crate::vendor::Granularity::Minute1)
            .expect("the minute rung has its own route");
        assert_eq!(route.params.len(), 1, "one field, and it is the interval");
        assert_eq!(route.params[0].name, "interval");

        // AND THE WORD IS THE VENDOR'S, NOT THE STORE'S. Dhan spells the rung
        // `1`; Groww spells the same rung `1minute`; the store spells it
        // `1min`. Three vocabularies, and the descriptor is what keeps them
        // apart.
        assert_eq!(
            spec.granularity_token(crate::vendor::Granularity::Minute1),
            Some("1"),
            "the intraday `interval` enum is 1, 5, 15, 30, 60"
        );
        assert_eq!(
            spec.granularity_token(crate::vendor::Granularity::Day1),
            None,
            "the daily endpoint takes no interval, so there is no word to send"
        );
        // AND THE RUNG IS ACTUALLY OFFERED NOW.
        assert!(
            crate::vendor::Feed::Dhan.serves(crate::vendor::Granularity::Minute1),
            "the rung is declared, so the form may offer it"
        );
    }

    /// DHAN'S MINUTE RUNG TAKES THE INCLUSIVE READING OF `toDate`.
    ///
    /// The vendor marks `toDate` "non-inclusive" on the DAILY page and marks it
    /// nothing at all on the intraday one. One shared `range_end` therefore
    /// sent `last_day + 1` on a minute request, on the strength of a sentence
    /// written about a different endpoint.
    ///
    /// The two readings do not fail the same way, which is the whole reason
    /// this is decidable without asking the vendor. Exclusive-when-inclusive
    /// reaches one day PAST the window, and the day past a window ending today
    /// is today — an unfinished session, written into an append-only store,
    /// which is the permanent gap `finished_day_only` exists to prevent.
    /// Inclusive-when-exclusive is one day SHORT, and short self-heals: the
    /// resume point asks for that day again on the next run.
    #[test]
    fn dhans_minute_rung_takes_the_safe_reading_of_an_undocumented_range_end() {
        let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("Dhan is an HTTP broker");
        };
        assert_eq!(
            spec.range_end_for(crate::vendor::Granularity::Day1),
            RangeEnd::Exclusive,
            "the daily page states non-inclusive, and that is verified"
        );
        assert_eq!(
            spec.range_end_for(crate::vendor::Granularity::Minute1),
            RangeEnd::Inclusive,
            "the intraday page states nothing, so the reading that fails SHORT \
             is taken — the other one can write an unfinished day"
        );

        // AND THE DAY THAT REACHES THE WIRE DIFFERS BY EXACTLY ONE.
        let last = crate::session::Day::new(2026, 8, 14).expect("a real day");
        let day = HttpSource::wire_end(
            last,
            spec.range_end_for(crate::vendor::Granularity::Day1),
            spec.date_format,
        )
        .expect("it renders");
        let minute = HttpSource::wire_end(
            last,
            spec.range_end_for(crate::vendor::Granularity::Minute1),
            spec.date_format,
        )
        .expect("it renders");
        assert_eq!(
            day, "2026-08-15",
            "the day AFTER, because toDate excludes it"
        );
        assert_eq!(minute, "2026-08-14", "the day itself, because it may not");
    }

    /// A FEED THAT SERVES EVERY RUNG FROM ONE URL IS UNCHANGED BY ANY OF THIS.
    #[test]
    fn groww_serves_both_rungs_from_the_one_endpoint_it_always_did() {
        let crate::vendor::Transport::Http(spec) =
            crate::vendor::Feed::Groww.descriptor().transport
        else {
            panic!("Groww is an HTTP broker");
        };
        assert!(spec.rung_routes.is_empty(), "no override, by construction");
        for rung in [
            crate::vendor::Granularity::Day1,
            crate::vendor::Granularity::Minute1,
        ] {
            assert_eq!(
                spec.path_for(rung),
                spec.bars_path,
                "every rung resolves to `/v1/historical/candles`"
            );
            assert!(
                spec.route_for(rung).is_none(),
                "and adds no per-rung parameter"
            );
        }
    }
}

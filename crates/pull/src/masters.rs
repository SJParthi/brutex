//! Where an instrument master comes from, and what it takes to replace one.
//!
//! # Why this exists
//!
//! `core::vendor::master_file` has always named the file each feed's master is
//! stored under, and `crates/api` has always parsed those files at startup.
//! **Nothing has ever fetched one.** The masters arrived on the operator's disk
//! by hand, so a stale master was invisible: the parse succeeded, the universe
//! resolved, and every symbol that had been renamed or delisted since the file
//! was written resolved to whatever the old row said.
//!
//! `Vendor::Zerodha`'s own comment records the vendor's instruction — *"The
//! vendor regenerates it once a day and asks that it be stored rather than
//! re-fetched"* — which is an argument for caching it, and no argument at all
//! for never refreshing it.
//!
//! # Two kinds of source, and the difference is not cosmetic
//!
//! Dhan and Groww publish their masters as **plain CDN files with no
//! authentication**. Zerodha's dump
//! is behind the same token every bar request spends:
//!
//! ```text
//! curl "https://api.kite.trade/instruments" \
//!   -H "X-Kite-Version: 3" \
//!   -H "Authorization: token api_key:access_token"
//! ```
//!
//! So a refresh of "the masters" is not one operation. Two of the three cost
//! nothing and can run unattended; the third spends a shared credential and is
//! an act an operator takes deliberately. [`Source::needs_token`] is that
//! distinction, carried in the data rather than in a comment, so a caller
//! cannot run the whole set unattended without noticing.
//!
//! # This module does not fetch
//!
//! It describes the sources and lands the bytes. The transport is
//! [`crate::chain::Discovery`], which the caller supplies — already governed by
//! `api`'s rate limiter for the credentialed feed, and trivially faked in tests.
//! A module that owned its own HTTP client would be a second transport with a
//! second retry policy and a second idea of what a timeout is.

use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;

/// The smallest body that can be a real master, in bytes.
///
/// # A truncated download must never replace a good file
///
/// A CDN that answers `200` with an error page, a connection cut mid-body, a
/// proxy that returns its own splash — all of these produce a short `String`
/// that writes cleanly over a working master. The next parse then yields a
/// universe missing most of its instruments, and every symbol that vanished
/// resolves to nothing with no error anywhere: `CLAUDE.md` §4's fallback that
/// hides a failure, wearing a successful HTTP status.
///
/// Measured against the real files: `dhan_scrip.csv` and `groww_instruments.csv`
/// are megabytes. A thousand bytes is far below any of them and far above any
/// error page worth mistaking for one, so it separates the two without needing
/// to know what a given vendor's error page looks like.
pub const MIN_BODY_BYTES: usize = 1_024;

/// What a source answers with, and therefore what has to happen before it is
/// landed.
///
/// # Why a source needs this at all
///
/// Three of the four masters are published as the CSV the engine reads, so
/// landing them is a write. NSE's index list is published as **JSON** — a map
/// of category to index names — while `api::indexmap::Published::read` parses
/// `index_name,category` rows.
///
/// That mismatch shipped once as a URL pointed at the wrong shape, and the
/// first version of this module then removed the source rather than convert it.
/// Both were wrong: the endpoint carries exactly the two fields the engine
/// wants, in a different container. **A transformation is not a guess** — the
/// mapping from `{category: [names]}` to `name,category` is total and
/// checkable, and [`nse_index_csv`] refuses rather than emits when the document
/// is not that shape. D-0310.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Already the CSV the engine reads. Landed as received.
    Csv,
    /// NSE's category-to-names JSON, converted before landing.
    NseIndexJson,
}

/// One instrument master, and what it costs to fetch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    /// What the host answers with, and therefore what landing has to do first.
    pub shape: Shape,
    /// The feed this master belongs to, or [`None`] for the exchange's own list.
    ///
    /// NSE's index list is not a feed's master — it is the reference the feeds
    /// are checked AGAINST, which is why `api::indexmap` exists. It has no
    /// `Vendor` because it is not one.
    pub vendor: Option<Vendor>,
    /// The filename it is stored under, inside the masters directory.
    pub file: &'static str,
    /// Where it is published.
    pub url: &'static str,
    /// Other URLs serving the identical file, tried in order after [`Self::url`].
    ///
    /// # Empty for three of the four, and that is the honest state
    ///
    /// `CLAUDE.md` §3 rule 1 forbids inventing a vendor fact, and a mirror is
    /// exactly that kind of fact: an address that either serves byte-identical
    /// content or silently serves something else. None of Dhan, Groww or
    /// Zerodha documents a second address for its master, so none is written
    /// here. The **mechanism** ships anyway, because the alternative is
    /// discovering on the day a CDN fails that adding a mirror is a code change
    /// rather than a data change.
    ///
    /// A mirror added later must be one the vendor published, cited where every
    /// other vendor fact in this repository is cited.
    pub mirrors: &'static [&'static str],
    /// A URL to GET first, to establish whatever session the real URL needs.
    ///
    /// # UNVERIFIED, and shipped anyway — deliberately
    ///
    /// `www.nseindia.com` is reported to gate its API behind cookies its own
    /// homepage sets, and to answer `401`/`403` to a request that presents
    /// none. This repository **cannot confirm that**: `crates/pull/src/
    /// calendar.rs` records that `nseindia.com` was unreachable from the
    /// environment this was built in — *"the documentation fetch timed out and
    /// the browser refused the host"* — so no request from here has ever seen
    /// either answer.
    ///
    /// So the claim is marked `UNVERIFIED` per §3 rule 1 and the *handling* is
    /// written to be correct either way. A prime that was unnecessary costs one
    /// cheap GET against a host already being asked for a file. A prime that
    /// was necessary and absent costs the whole index catalogue. And a prime
    /// that FAILS is recorded as [`Got::PrimeRefused`] and the real request goes
    /// out regardless, so the operator reads the host's own answer rather than
    /// this module's guess about it.
    ///
    /// [`None`] for every source that needs no session.
    pub prime: Option<&'static str>,
    /// Whether fetching it spends the shared vendor credential.
    ///
    /// **The whole reason this field exists**: two of the three sources are
    /// public CDN files that cost nothing, and one is behind the same token
    /// every bar request spends. A caller that refreshes the set unattended
    /// must be able to tell them apart without reading a comment.
    pub needs_token: bool,
}

impl Source {
    /// Every address this file is published at, primary first.
    ///
    /// One allocation-free iterator rather than a `Vec`, because [`fetch`]
    /// walks it once and a mirror list is at most a handful long.
    pub fn every_url(&self) -> impl Iterator<Item = &'static str> + use<> {
        core::iter::once(self.url).chain(self.mirrors.iter().copied())
    }
}

/// NSE's own index list — the reference every feed is checked against.
pub const NSE_INDICES_FILE: &str = "nse_indices.csv";

/// The largest body that will be read into memory, in bytes. 256 MiB.
///
/// # The other end of [`MIN_BODY_BYTES`], and it is not symmetric
///
/// A too-SHORT body is a truncated download that would overwrite a good file.
/// A too-LONG body is a different failure entirely: a redirect that landed on
/// something enormous, a host streaming an unbounded response, a proxy looping.
/// `reqwest`'s `text()` reads to the end with no ceiling, so without this the
/// process would grow until the allocator refused — and the operator would see
/// a kill, not a refusal.
///
/// Measured against what these files actually are: the largest master in this
/// set is Zerodha's full dump, tens of megabytes. A quarter of a gigabyte is an
/// order of magnitude above any of them and far below the 48 GB this machine
/// has, so it separates "a big CSV" from "something has gone wrong" without
/// needing to track how each vendor's file grows.
pub const MAX_BODY_BYTES: usize = 256 * 1_024 * 1_024;

/// Every master this engine reads, with where it comes from.
///
/// # The two swept feeds are here and the archives are not
///
/// `CLAUDE.md` §1 puts exactly two instruments on the engine surface and
/// `Vendor::master_file`'s own comment records that an ARCHIVE ships no master:
/// *"The folder of CSVs IS the listing: every file in it is an instrument,
/// named by its own filename."* `TrueData` and `Gdfl` are therefore absent by
/// the same rule that gives them a filename which `master_paths` looks for and
/// will not find.
pub const SOURCES: [Source; 4] = [
    Source {
        shape: Shape::Csv,
        vendor: Some(Vendor::Dhan),
        file: "dhan_scrip.csv",
        url: "https://images.dhan.co/api-data/api-scrip-master.csv",
        mirrors: &[],
        prime: None,
        needs_token: false,
    },
    Source {
        shape: Shape::Csv,
        vendor: Some(Vendor::Groww),
        file: "groww_instruments.csv",
        url: "https://growwapi-assets.groww.in/instruments/instrument.csv",
        mirrors: &[],
        prime: None,
        needs_token: false,
    },
    Source {
        shape: Shape::Csv,
        vendor: Some(Vendor::Zerodha),
        file: "zerodha_instruments.csv",
        url: "https://api.kite.trade/instruments",
        mirrors: &[],
        // NO PRIME, AND NOT BECAUSE ONE WOULD BE HARMLESS. This source travels
        // with the shared credential, and a prime is an extra request carrying
        // that header to an extra address. `may_fetch` exists to keep the
        // credential on the one host that issued it; a prime here would walk
        // straight around it.
        prime: None,
        needs_token: true,
    },
    Source {
        // THE EXCHANGE'S OWN LIST, CONVERTED RATHER THAN GUESSED AT. The
        // endpoint answers `{category: [index names]}` and the engine reads
        // `index_name,category`; those are the same two fields in a different
        // container, and `nse_index_csv` refuses rather than emits if the
        // document is not that shape.
        shape: Shape::NseIndexJson,
        vendor: None,
        file: NSE_INDICES_FILE,
        url: "https://www.nseindia.com/api/equity-master",
        mirrors: &[],
        // THE HOMEPAGE, GET FIRST. See `Source::prime` for why this is marked
        // UNVERIFIED and shipped regardless, and for what happens when the
        // prime itself refuses.
        prime: Some("https://www.nseindia.com/"),
        needs_token: false,
    },
];

/// NSE's index JSON as the `index_name,category` rows the engine reads.
///
/// # Why a conversion and not a second parser
///
/// `api::indexmap::Published::read` is the authority on the catalogue's shape
/// and its own test writes `index_name,category\nNIFTY PRIVATE BANK,sectoral`.
/// Teaching it JSON would give the catalogue two readers that must agree
/// forever; converting at the boundary leaves it one.
///
/// # It refuses rather than emits, in four ways
///
/// A document that is not an object, an object whose values are not arrays of
/// strings, a conversion that yields **no rows**, and any name or category
/// carrying a comma or a newline — which would produce a CSV whose columns do
/// not line up with its header and which `Published::read` would then
/// mis-split. Every one is a refusal naming the cause, because the alternative
/// is a catalogue that parses into the wrong names.
///
/// # Errors
///
/// A sentence for each of the four.
pub fn nse_index_csv(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(body).map_err(|why| format!("the index list is not JSON — {why}"))?;
    let Some(groups) = parsed.as_object() else {
        return Err(
            "the index list is JSON but not an object of category to names, so \
             there is no category to attach to any index"
                .to_owned(),
        );
    };

    let mut out = String::from("index_name,category\n");
    let mut rows = 0_usize;
    for (category, names) in groups {
        let Some(list) = names.as_array() else {
            continue;
        };
        for name in list {
            let Some(name) = name.as_str() else {
                continue;
            };
            // A COMMA IN A FIELD IS A COLUMN THIS FILE DOES NOT HAVE. Quoting
            // it would be the other choice and a worse one: `Published::read`
            // splits on commas and does not unquote, so a quoted field would
            // arrive with its quotes in the name.
            if name.contains([',', '\n']) || category.contains([',', '\n']) {
                return Err(format!(
                    "`{name}` in category `{category}` carries a comma or a \
                     newline, which would produce a row whose columns do not \
                     line up with the header"
                ));
            }
            out.push_str(name);
            out.push(',');
            out.push_str(category);
            out.push('\n');
            rows = rows.saturating_add(1);
        }
    }

    if rows == 0 {
        return Err(
            "the index list parsed as JSON and yielded no index at all, so the \
             document is not the category-to-names map this conversion reads"
                .to_owned(),
        );
    }
    Ok(out)
}

/// Which kind of transport a fetch is going out on.
///
/// # A credential must never travel to a host that did not issue it
///
/// `HttpSource`'s `Discovery::get` attaches the vendor's auth header to
/// **every** URL it is handed — `let (name, value) = self.header();` then
/// `builder.header(name, value)`. That is right for the vendor's own endpoints
/// and catastrophic anywhere else: fetching Dhan's public CDN through a
/// Zerodha-configured source sends the Zerodha token to `images.dhan.co`.
///
/// The masters are the one place in this workspace where public URLs and a
/// credentialed URL sit in one list and get iterated together, so the mistake is
/// one `for` loop away. [`may_fetch`] makes the pairing a checked fact rather
/// than a convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    /// Sends no credential. The only kind a third-party host may be given.
    Public,
    /// Carries the vendor's auth header on every request it makes.
    Credentialed,
}

/// Whether this source may be fetched over this transport.
///
/// # Errors
///
/// A sentence naming the host, in both directions:
///
/// * a public source over a credentialed transport — **a credential leak**, and
///   the reason this function exists;
/// * a credentialed source over a public one — a guaranteed `401`, refused here
///   with the cause rather than at the vendor with a status.
pub fn may_fetch(source: &Source, via: Transport) -> Result<(), String> {
    match (source.needs_token, via) {
        (false, Transport::Credentialed) => Err(format!(
            "`{}` is a public file and this transport attaches a vendor \
             credential to every request. Sending it would hand that credential \
             to a host that did not issue it. Fetch public sources over \
             Transport::Public.",
            source.url
        )),
        (true, Transport::Public) => Err(format!(
            "`{}` is behind the vendor's token and this transport sends none, \
             so the vendor would answer 401. Fetch it over \
             Transport::Credentialed.",
            source.url
        )),
        (false, Transport::Public) | (true, Transport::Credentialed) => Ok(()),
    }
}

/// A transport that sends no credential, for the hosts that issued none.
///
/// # Why this exists rather than reusing `HttpSource`
///
/// [`Transport`] explains the hazard; this is the other half of the fix.
/// `HttpSource` is built around one vendor's [`crate::vendor::HttpSpec`] — its
/// auth header, its date format, its response shape — and its `Discovery::get`
/// attaches that header to any URL. There is no configuration of it that means
/// *"a plain GET to somebody else's CDN"*, and constructing one with an empty
/// credential to fake that would be a credential-shaped hole waiting for the
/// next person who fills it in.
///
/// So this is deliberately the smallest thing that can be a transport: a client
/// with a timeout, no auth header and no governor. **No governor is not an
/// oversight** — a rate budget is per FEED and spends against a vendor's quota,
/// and none of these hosts issues one. A once-a-day CDN file is not a quota to
/// protect. The retry ladder is not here either, and that is also deliberate:
/// [`fetch`] holds it, one layer up, where it is testable against a fake.
///
/// # Redirects are FOLLOWED here, and the vendor transport still refuses them
///
/// This paragraph used to say the opposite, and matching `pooled_client`'s
/// `redirect::Policy::none()` was the stated reason. That reasoning imported a
/// rule from a place where it earns its keep to a place where it does not:
/// `pooled_client` declines redirects because **it carries a credential**, and
/// a redirect walks that credential to a host that never issued it.
///
/// This client carries no credential at all — no auth header, by construction,
/// which is the whole reason the type exists. What refusing redirects bought
/// here was not safety; it was a CDN that moves a file behind a `301` reading
/// as a permanent failure on a file sitting one hop away. Bounded at five, so a
/// redirect loop terminates.
///
/// The host confinement [`may_fetch`] provides is unaffected: it is about which
/// TRANSPORT may carry which source, and a credential-free client following a
/// hop leaks nothing to leak.
///
/// # Not `Clone`, because a jar must not be
///
/// [`Jar`] holds a session established by [`Source::prime`]. Cloning the
/// transport would fork it, and the copy that made the real request would be
/// presenting cookies the copy that was primed had been given — or not
/// presenting them at all, silently, which is the failure this whole module is
/// written against.
#[derive(Debug)]
pub struct PublicFetch {
    client: reqwest::Client,
    jar: Jar,
}

/// Cookies kept per host, written by hand rather than by a dependency.
///
/// # Why this is thirty lines here and not one feature flag
///
/// `reqwest`'s `cookies` feature pulls `cookie_store` → `cookie` → `time`, and
/// **`cargo deny check` refused it**: `time 0.3.45` carries a RUSTSEC
/// stack-exhaustion advisory. `CLAUDE.md` §9 makes a green `cargo deny` part of
/// done, so that alone settled it. Dropping the feature removed all three
/// packages — `time` appears nowhere in `Cargo.lock` now, which is why the
/// lockfile carries no change from any of this.
///
/// **One argument that was made for this and does not hold, recorded because
/// the ledger should not carry it either.** `cookie`'s `build.rs` runs `rustc`
/// through `version_check`, and §2 forbids *"any `build.rs` that invokes an
/// external process"* without exception. That reading is correct and it is
/// **not a distinction that favours this jar**: `serde`, `libc`, `proc-macro2`,
/// `quote`, `httparse`, `ahash` and `generic-array` all probe `rustc` from
/// their build scripts and all are in this tree today. §2's dependency-level
/// ban is not held by the workspace and is not holdable — `docs/06-limits.md`
/// §96 records that rather than leaving it as a rule invoked selectively.
///
/// What is left, and is enough: the advisory above, zero added dependencies,
/// and one job instead of a general mechanism — carry whatever
/// [`Source::prime`] was given back to the same host on the next request.
///
/// # The scoping is the security property, not a nicety
///
/// Keyed **by host**. A cookie set by `www.nseindia.com` is presented to
/// `www.nseindia.com` and to nothing else, so a session from one source cannot
/// ride along to another. `PublicFetch` carries no credential of any kind, so
/// this is the only thing on this transport that is worth confining — and it is
/// confined. `a_cookie_never_leaves_the_host_that_set_it` is the proof.
///
/// # What it deliberately does not implement
///
/// No `Domain`, `Path`, `Secure`, `Max-Age` or `Expires` handling, and no
/// eviction. Those matter to a browser visiting arbitrary sites; this visits
/// four fixed URLs within one refresh and the process does not outlive it. A
/// half-implemented `Domain` attribute would be worse than none, because it
/// would look like scoping while widening it.
type Jar =
    std::sync::Mutex<std::collections::HashMap<String, std::collections::BTreeMap<String, String>>>;

impl PublicFetch {
    /// A transport for public files.
    ///
    /// # Errors
    ///
    /// Whatever the client builder said.
    pub fn new() -> Result<Self, String> {
        crate::ensure_tls_provider();
        reqwest::Client::builder()
            // A USER AGENT, BECAUSE ONE OF THESE HOSTS REQUIRES IT.
            //
            // `www.nseindia.com`'s API refuses a request that does not present
            // a browser agent — the answer is an HTML block page or a 403, not
            // the JSON. Without this the index list would refuse on its opener
            // check every single time, which is correct behaviour reporting a
            // cause that is not the real one.
            //
            // It names this client honestly rather than impersonating a browser
            // build: the requirement is that the field is present and
            // browser-shaped, not that it lies about what is calling.
            .user_agent("Mozilla/5.0 (compatible; brutex/1.0; +instrument-master-refresh)")
            // WHAT A BROWSER SENDS, because the host that needs the agent
            // checks these too. An `Accept` of `*/*` is what a script sends;
            // this names the two shapes these four sources actually answer.
            .default_headers(browser_headers())
            // A MINUTE, because these are whole-file downloads and the
            // largest is 19 MB. The vendor transport's own timeout bounds a
            // bar window, which is a different size of answer.
            .timeout(core::time::Duration::from_mins(1))
            // TEN SECONDS TO GET CONNECTED, SEPARATELY. Without this a host
            // that accepts the socket and never speaks burns the whole minute
            // before the ladder learns anything; DNS and TLS either work in ten
            // seconds or are not going to.
            .connect_timeout(core::time::Duration::from_secs(10))
            // REDIRECTS FOLLOWED, BOUNDED AT FIVE — and safe here for one
            // reason that must stay true: **this client carries no credential.**
            // It sets no `Authorization` header and is used only for the public
            // sources, so a redirect cannot walk a secret to a host that did
            // not issue it. The credentialed transport is `http::HttpSource`,
            // which declines redirects for exactly that reason.
            //
            // Refusing them here is not the safe default it looks like: a CDN
            // moving a file behind a `301` is ordinary, and `Policy::none()`
            // turns that into a permanent-looking failure on a file that is
            // sitting right there one hop away.
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map(|client| Self {
                client,
                jar: Jar::default(),
            })
            .map_err(|why| format!("a public transport could not be built — {why}"))
    }

    /// The `Cookie` header value for this host, if anything has been set for it.
    fn cookies_for(&self, host: &str) -> Option<String> {
        let jar = self
            .jar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let held = jar.get(host)?;
        if held.is_empty() {
            return None;
        }
        // A `BTreeMap` RATHER THAN A `HashMap`, so the header is byte-identical
        // for the same set of cookies on every run. `CLAUDE.md` §3 rule 5 is
        // idempotence, and a header whose field order changes per process is a
        // request that differs run to run for no reason anyone chose.
        Some(
            held.iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    /// Records whatever the host set, under that host.
    ///
    /// # Why this takes strings rather than a `Response`
    ///
    /// A `reqwest::Response` cannot be constructed in a test without a socket,
    /// and this is where the actual decisions live — what counts as a pair,
    /// what a repeat name does, what a malformed header does. Behind a
    /// `Response` all three would be reasoning nothing could check; taking the
    /// header values makes the parse ordinary testable code and leaves the
    /// caller with one expression.
    fn remember_pairs(&self, host: &str, raw_headers: &[&str]) {
        let mut jar = self
            .jar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let held = jar.entry(host.to_owned()).or_default();
        for raw in raw_headers {
            // EVERYTHING BEFORE THE FIRST `;` IS THE PAIR; the rest are
            // attributes this jar deliberately does not implement. See `Jar`.
            let pair = raw.split(';').next().unwrap_or_default().trim();
            let Some((name, value)) = pair.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            held.insert(name.to_owned(), value.trim().to_owned());
        }
    }

    /// The host a URL names, or `None` if it names none.
    fn host_of(url: &str) -> Option<String> {
        reqwest::Url::parse(url)
            .ok()
            .and_then(|parsed| parsed.host_str().map(str::to_owned))
    }
}

impl crate::chain::Discovery for PublicFetch {
    async fn get(&self, url: &str) -> Result<String, crate::chain::Refusal> {
        use crate::chain::Refusal;

        let host = Self::host_of(url);

        let mut asking = self.client.get(url);
        // WHATEVER THE PRIME WAS GIVEN, HANDED BACK TO THE SAME HOST. Without
        // this the prime is an HTTP request that accomplishes nothing while
        // looking like it did, which is `CLAUDE.md` §4's shape exactly.
        if let Some(cookies) = host.as_deref().and_then(|h| self.cookies_for(h)) {
            asking = asking.header(reqwest::header::COOKIE, cookies);
        }

        let mut answer = asking
            .send()
            .await
            .map_err(|why| Refusal::transport(format!("{url} — {why}")))?;
        let status = answer.status();

        // BEFORE THE STATUS IS JUDGED, because a host that gates on a session
        // sets the cookie on the very response that refuses — that is what
        // makes a re-prime worth attempting at all.
        if let Some(ref host) = host {
            let set: Vec<&str> = answer
                .headers()
                .get_all(reqwest::header::SET_COOKIE)
                .iter()
                .filter_map(|value| value.to_str().ok())
                .collect();
            self.remember_pairs(host, &set);
        }
        if !status.is_success() {
            // THE HOST'S OWN STATUS, NOT A PARAPHRASE, for the reason
            // `HttpSource` states: a 404 and a 503 mean different things to an
            // operator — one is a URL that moved and one is a host to try again
            // — and collapsing them turns a permanent break into a retry.
            return Err(Refusal::answered(
                status.as_u16(),
                format!("{url} answered {status}"),
            ));
        }
        // THE DECLARED LENGTH FIRST, WHERE THERE IS ONE. A host that says up
        // front it is about to send half a gigabyte is refused without
        // downloading half a gigabyte to find out.
        if answer
            .content_length()
            .is_some_and(|declared| declared > MAX_BODY_BYTES as u64)
        {
            return Err(Refusal::answered(
                status.as_u16(),
                format!("{url} declared more than the {MAX_BODY_BYTES}-byte ceiling"),
            ));
        }

        // AND THEN CHUNK BY CHUNK, BECAUSE THE DECLARATION IS OPTIONAL AND
        // OPTIONALLY TRUE. A chunked response carries no `Content-Length` at
        // all, and one that carries a false one is not obliged to honour it.
        // `Response::text` reads to the end with no ceiling of any kind, so
        // this is the only place the ceiling can actually bind.
        let mut held: Vec<u8> = Vec::new();
        loop {
            let chunk = answer
                .chunk()
                .await
                .map_err(|why| Refusal::transport(format!("{url} body — {why}")))?;
            let Some(bytes) = chunk else { break };
            if held.len().saturating_add(bytes.len()) > MAX_BODY_BYTES {
                return Err(Refusal::answered(
                    status.as_u16(),
                    format!("{url} sent more than the {MAX_BODY_BYTES}-byte ceiling"),
                ));
            }
            held.extend_from_slice(&bytes);
        }

        // NOT `from_utf8_lossy`. A master with a replacement character where a
        // symbol used to be parses cleanly and resolves the wrong instrument —
        // a silent corruption, which is worse than a refusal an operator reads.
        String::from_utf8(held)
            .map_err(|why| Refusal::transport(format!("{url} body is not UTF-8 — {why}")))
    }
}

/// The headers a browser sends, which one of these hosts checks for.
///
/// # Why these three and not a copy of a real browser's set
///
/// A host that filters scripted traffic looks at whether the request is shaped
/// like a browser's, and the fields it can cheaply check are the agent, what
/// the caller will accept, and in what language. Sending a full impersonation
/// of a specific Chrome build would be a claim about what this program is; this
/// is the smaller, true version — a real client that accepts what these four
/// sources actually serve.
///
/// `Accept` names both shapes deliberately: three sources answer CSV and one
/// answers JSON, and a single client asks for both because a single client
/// fetches both.
fn browser_headers() -> reqwest::header::HeaderMap {
    use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/csv, text/plain, */*"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers
}

/// What happened to one master.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Landed {
    /// Written, with the byte count and whether it differed from what was there.
    Written {
        /// Bytes written.
        bytes: usize,
        /// Whether the file on disk changed.
        ///
        /// A vendor that regenerates once a day answers the same bytes for the
        /// rest of it, and reporting that as a change would make every refresh
        /// look like new data.
        changed: bool,
    },
    /// Refused, with the reason an operator reads.
    Refused(String),
}

impl Landed {
    /// Whether this landed at all.
    #[must_use]
    pub const fn is_written(&self) -> bool {
        matches!(*self, Self::Written { .. })
    }
}

/// The path one source is stored at inside `dir`.
#[must_use]
pub fn path_of(dir: &Path, source: &Source) -> PathBuf {
    dir.join(source.file)
}

/// Writes one fetched body, refusing anything that cannot be a master.
///
/// # Why the write is not direct
///
/// A `write` that truncates and then fails part-way leaves a master shorter
/// than it was, and the parse that follows succeeds on the fragment. The body
/// goes to a sibling temporary file and is renamed over the target, so the
/// master is either the old bytes or the new ones and never a prefix of either
/// — `rename` within one directory is atomic on every filesystem this runs on.
///
/// # Errors
///
/// Returns [`Landed::Refused`] rather than a `Result`: a refusal here is one
/// source's outcome inside a set, and a caller refreshing four of them needs
/// the other three to continue. The reason is a sentence, because it is read by
/// an operator and not matched on by a retry.
#[must_use]
pub fn land(dir: &Path, source: &Source, body: &str) -> Landed {
    // CONVERTED FIRST, THEN CHECKED. The guards below describe the file the
    // engine reads, not the document the host answered with — running them on
    // NSE's JSON would refuse it for opening with `{`, which is true of every
    // correct response it will ever give.
    let converted = match source.shape {
        Shape::Csv => std::borrow::Cow::Borrowed(body),
        Shape::NseIndexJson => match nse_index_csv(body) {
            Ok(csv) => std::borrow::Cow::Owned(csv),
            Err(why) => {
                return Landed::Refused(format!("`{}` — {why}", source.url));
            }
        },
    };
    let body: &str = &converted;

    if body.len() < MIN_BODY_BYTES {
        return Landed::Refused(format!(
            "`{}` answered {} byte(s), under the {MIN_BODY_BYTES} a master must \
             exceed. An error page, a cut connection and a proxy splash all \
             arrive as a short 200, and writing one over a working master \
             would leave a universe missing most of its instruments with no \
             error anywhere.",
            source.url,
            body.len()
        ));
    }
    // A CSV DOES NOT OPEN WITH `{` OR `<`, AND THE COMMA TEST ALONE MISSES
    // BOTH.
    //
    // MEASURED, on this module's own first version: it pointed NSE's index list
    // at `https://www.nseindia.com/api/equity-master`, which answers **JSON**.
    // `{"Broad Market Indices":["NIFTY 50",…]}` contains commas, so it cleared
    // the byte floor AND the comma test, landed cleanly over the operator's
    // catalogue, and would have failed at `indexmap::Published::read` — one
    // layer away from the cause, with a file on disk that looks like a master.
    //
    // The first non-space byte is the cheapest thing that separates them, and it
    // costs one comparison rather than a parse.
    let opener = body.trim_start().as_bytes().first().copied();
    if opener == Some(b'{') || opener == Some(b'[') || opener == Some(b'<') {
        return Landed::Refused(format!(
            "`{}` answered {} bytes opening with `{}`, which is JSON or markup \
             and not the CSV a master is. A body like this passes a comma test \
             — JSON is full of commas — so it is refused on its first byte \
             instead.",
            source.url,
            body.len(),
            char::from(opener.unwrap_or(b'?'))
        ));
    }

    // A MASTER IS A CSV AND A CSV HAS A HEADER ROW. An HTML error page long
    // enough to clear the byte floor still fails this, and the check costs one
    // comparison on the first line rather than a parse of the whole file.
    let first = body.lines().next().unwrap_or_default();
    if !first.contains(',') {
        return Landed::Refused(format!(
            "`{}` answered {} bytes whose first line carries no comma, so it is \
             not the CSV a master is. First line: {}",
            source.url,
            body.len(),
            first.chars().take(80).collect::<String>()
        ));
    }

    let target = path_of(dir, source);
    let changed = std::fs::read_to_string(&target).map_or(true, |held| held != body);
    if let Err(why) = std::fs::create_dir_all(dir) {
        return Landed::Refused(format!(
            "the masters directory {} cannot be created — {why}",
            dir.display()
        ));
    }
    // NAMED FOR THE TARGET, so two sources refreshing at once cannot collide on
    // one temporary file and hand each other's bytes to the rename.
    let temporary = dir.join(format!(".{}.partial", source.file));
    if let Err(why) = std::fs::write(&temporary, body) {
        return Landed::Refused(format!(
            "{} could not be written — {why}",
            temporary.display()
        ));
    }
    if let Err(why) = std::fs::rename(&temporary, &target) {
        // THE PARTIAL IS REMOVED ON A FAILED RENAME. Leaving it would grow one
        // stale file per failed refresh in a directory the operator reads.
        let _ignored = std::fs::remove_file(&temporary);
        return Landed::Refused(format!(
            "{} could not replace {} — {why}",
            temporary.display(),
            target.display()
        ));
    }
    Landed::Written {
        bytes: body.len(),
        changed,
    }
}

/// The sources a caller may fetch without spending the shared credential.
pub fn free_sources() -> impl Iterator<Item = &'static Source> {
    SOURCES.iter().filter(|source| !source.needs_token)
}

/// The sources that spend it.
pub fn token_sources() -> impl Iterator<Item = &'static Source> {
    SOURCES.iter().filter(|source| source.needs_token)
}

// ======================= asking again, and knowing when not to =======================

/// Whether a refusal is worth asking again, and if so how.
///
/// # Why a status has to become a decision somewhere
///
/// A master refresh that gives up on the first refusal is not a downloader; it
/// is a coin toss against a CDN. A refresh that retries *everything* is worse —
/// it asks a dead credential sixty-four times and hammers a host that has
/// already said no. The difference between those two is entirely in the number
/// the host answered with, which is why [`crate::chain::Refusal`] carries it as
/// a `u16` rather than inside a sentence.
///
/// [`Reprime`](Self::Reprime) is the third arm and it exists for one observed
/// host shape: an endpoint that gates on a session cookie answers `401` or
/// `403` to a first request and the *same* request succeeds once a session
/// exists. By status alone that is permanent; by request it is not, because the
/// second request is not the same request. Collapsing it into
/// [`Never`](Self::Never) is what makes a cookie-gated host look permanently
/// forbidden, and collapsing it into [`Again`](Self::Again) is what makes a
/// genuinely dead credential get asked five times.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Transient. The identical request, later, may well answer.
    Again,
    /// The host is gating on a session. Refresh it and ask exactly once more.
    Reprime,
    /// Settled. Asking again spends a request to learn what is already known.
    Never,
}

/// What to do about the status a host answered, or about answering nothing.
///
/// # Every arm, and the reason it is where it is
///
/// | Answer | Verdict | Because |
/// |---|---|---|
/// | nothing at all | `Again` | DNS, a reset socket, a TLS handshake, a timeout — none is a decision the host made |
/// | `408` `425` `429` | `Again` | the host said *not now*, in three different words |
/// | `500` `502` `503` `504` `507` `508` `509` | `Again` | the host is broken or out of room, and may not be in a minute |
/// | `520`…`599` | `Again` | the CDN vendor range; a proxy failing in front of a host that may be fine |
/// | `501` `505` | `Never` | the host will not support this method or version in a minute either |
/// | `401` `403` | `Reprime` | either a dead credential or a missing session, and the two are indistinguishable by status |
/// | any other `4xx` | `Never` | `404`, `410`, `414` — the URL is wrong, and it will still be wrong |
/// | `2xx` | `Never` | it answered; whatever refused it did so on the BODY, and the body will be the same |
/// | `3xx` | `Never` | the redirect policy declined it; re-asking gets the identical hop |
///
/// # `2xx` reaches here, which is not obvious
///
/// [`PublicFetch`] refuses a `200` whose body exceeds [`MAX_BODY_BYTES`]. That
/// is a successful HTTP exchange and a refused fetch, and re-asking would
/// download the same oversized body again.
#[must_use]
pub const fn verdict_of(status: Option<u16>) -> Verdict {
    let Some(code) = status else {
        // NOTHING WAS ANSWERED. `Refusal::transport` is constructed for a
        // dropped socket, a DNS failure and a timeout, and none of those is
        // the host declining — it is the road, and roads clear.
        return Verdict::Again;
    };
    match code {
        401 | 403 => Verdict::Reprime,
        // THE TWO PERMANENT 5xx, NAMED BEFORE THE RANGE THAT WOULD SWALLOW
        // THEM. `501 Not Implemented` and `505 HTTP Version Not Supported` are
        // statements about what the host can ever do, not about today, and the
        // arm below would otherwise read them as a host having a bad minute.
        501 | 505 => Verdict::Never,
        // NOT NOW (`408` `425` `429`), and BROKEN OR OUT OF ROOM (`5xx`, plus
        // the `520`…`599` range CDNs use for a proxy failing in front of a host
        // that may itself be fine). One arm because they earn one answer.
        408 | 425 | 429 | 500..=509 | 520..=599 => Verdict::Again,
        _ => Verdict::Never,
    }
}

/// The wait before the second attempt, in milliseconds.
pub const FIRST_BACKOFF_MS: u64 = 500;

/// The longest single wait, in milliseconds. Thirty-two seconds.
///
/// Four sources at five attempts each cannot then spend more than a bounded
/// couple of minutes before the refresh reports, which is the property that
/// matters: a caller waiting on `POST /masters/refresh` gets an answer.
pub const MAX_BACKOFF_MS: u64 = 32_000;

/// How many times one URL is asked before the next URL is tried.
pub const ATTEMPTS_PER_URL: u32 = 5;

/// The wait after this many consecutive refusals: 500 ms, 1 s, 2 s, 4 s, …
///
/// # Deterministic, and deliberately not jittered
///
/// `CLAUDE.md` §3 rule 5 is idempotence, and jitter would make the attempt
/// ledger below un-assertable — a test could check that a wait happened but
/// never that the schedule is the one documented here. Jitter earns its keep
/// against a thundering herd of many clients; this is one operator refreshing
/// four files in sequence, and there is no herd.
///
/// # `checked_shl` is the wrong tool here, and it was the first version
///
/// `u64::checked_shl` bounds the **shift amount** and says nothing about the
/// **result**: `500u64.checked_shl(63)` is `Some(0)`, because the shift is
/// perfectly legal and every set bit has simply fallen off the top. A guard
/// reading `Some(ms) if ms <= MAX_BACKOFF_MS` then accepts `0` as a valid
/// backoff, and a ladder that waits zero milliseconds between five attempts is
/// not backing off at all — it is hammering a host that has just asked it not
/// to, at full speed, with a comment claiming otherwise.
///
/// `checked_pow` genuinely returns `None` on overflow and `checked_mul`
/// genuinely returns `None` when the product does not fit, so between them the
/// value is bounded rather than the operation being legal.
/// `the_backoff_schedule_is_the_one_documented` asserts the two steps where the
/// difference shows.
#[must_use]
pub const fn backoff_ms(step: u32) -> u64 {
    let Some(factor) = 2u64.checked_pow(step) else {
        return MAX_BACKOFF_MS;
    };
    match FIRST_BACKOFF_MS.checked_mul(factor) {
        Some(ms) if ms <= MAX_BACKOFF_MS => ms,
        _ => MAX_BACKOFF_MS,
    }
}

/// Waiting, as a port, so a retry ladder is testable in zero wall-clock time.
///
/// # Why this is a trait and not `tokio::time::sleep`
///
/// A ladder that sleeps for real can be tested for its ANSWER and never for its
/// SCHEDULE: proving that the third attempt waits two seconds would cost two
/// seconds of every future test run, so nobody writes that test and the
/// schedule goes unchecked. Behind a port the fake records what it was asked to
/// wait and returns immediately, and the schedule becomes an ordinary equality
/// assertion — see `the_backoff_schedule_is_the_one_documented`.
///
/// It is the same seam [`crate::chain::Discovery`] draws for the socket, for
/// the same reason.
pub trait Pause {
    /// Waits the given number of milliseconds.
    fn pause(&self, ms: u64) -> impl core::future::Future<Output = ()>;
}

/// The real clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct Clock;

impl Pause for Clock {
    async fn pause(&self, ms: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
}

/// What one step of a fetch did.
///
/// # Why every step is recorded, including the ones that worked
///
/// `POST /masters/refresh` answers a page, and an operator reading "it failed"
/// has been told nothing they can act on. The three questions they actually
/// have are *which URL*, *how many times*, and *what did the host say* — and
/// all three are lost the moment a ladder collapses its history into a final
/// `Result`. This is that history, and it is a `Vec` of at most
/// `ATTEMPTS_PER_URL` × urls + two primes, which is bounded by the source
/// table rather than by anything the network does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    /// The URL this step went to.
    pub url: String,
    /// Which attempt at that URL this was, from 1.
    pub number: u32,
    /// What was waited **before** this step, in milliseconds.
    pub waited_ms: u64,
    /// What came back.
    pub got: Got,
}

/// The outcome of one step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Got {
    /// A session was established at the priming URL.
    Primed,
    /// The priming URL itself refused.
    ///
    /// **Not fatal, and recorded rather than swallowed.** A prime is an attempt
    /// to satisfy a requirement this repository cannot verify from where it was
    /// built — see [`Source::prime`]. If it fails, the real request still goes
    /// out and reports the host's own answer, which is more informative than a
    /// guess about why the prime mattered. `CLAUDE.md` §4 bans a fallback that
    /// HIDES a failure; this one is a row.
    PrimeRefused {
        /// The host's words.
        detail: String,
    },
    /// A body came back.
    Body {
        /// How many bytes of it.
        bytes: usize,
    },
    /// The host refused, and what was decided about asking again.
    Refused {
        /// The status, or [`None`] if nothing was answered at all.
        status: Option<u16>,
        /// The host's words, unparaphrased.
        detail: String,
        /// What [`verdict_of`] made of it.
        verdict: Verdict,
    },
}

/// A body, or every reason there is not one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fetched {
    /// The body, if any URL answered one.
    pub body: Option<String>,
    /// Every step, in order.
    pub attempts: Vec<Attempt>,
}

impl Fetched {
    /// The host's last word, for a caller rendering one line.
    #[must_use]
    pub fn last_refusal(&self) -> Option<&str> {
        self.attempts.iter().rev().find_map(|a| match &a.got {
            Got::Refused { detail, .. } | Got::PrimeRefused { detail } => Some(detail.as_str()),
            Got::Body { .. } | Got::Primed => None,
        })
    }

    /// How long the ladder spent waiting, in milliseconds.
    #[must_use]
    pub fn waited_ms(&self) -> u64 {
        self.attempts.iter().map(|a| a.waited_ms).sum()
    }
}

/// Asks every URL a source publishes, retrying what is worth retrying.
///
/// # The order, and why the URL loop is outside the attempt loop
///
/// A source with a mirror should exhaust the primary's retries *before* moving
/// on, not alternate between them: alternating turns one host's transient blip
/// into two hosts' worth of requests and arrives no sooner. So the outer loop
/// is URLs, the inner loop is attempts, and a [`Verdict::Never`] breaks
/// straight out of the inner one — because five attempts at a `404` is five
/// requests spent learning what the first one said.
///
/// # It opens no socket
///
/// Both moving parts are ports. `from` is the transport, `clock` is the wait,
/// and the whole ladder — every branch, every backoff, the prime, the mirror —
/// runs in a test with neither a network nor a second of wall clock.
pub async fn fetch<D: crate::chain::Discovery, P: Pause>(
    from: &D,
    clock: &P,
    source: &Source,
) -> Fetched {
    let mut attempts = Vec::new();

    for url in source.every_url() {
        // ONE RE-PRIME PER URL, AND IT IS SPENT DELIBERATELY. A host that
        // answers 401 twice with a fresh session between them is not gating on
        // a session, and the second answer is the real one.
        let mut reprimed = false;
        let mut waited = 0;

        prime_once(from, source, &mut attempts).await;

        for number in 1..=ATTEMPTS_PER_URL {
            if waited > 0 {
                clock.pause(waited).await;
            }

            let refusal = match from.get(url).await {
                Ok(body) => {
                    attempts.push(Attempt {
                        url: url.to_owned(),
                        number,
                        waited_ms: waited,
                        got: Got::Body { bytes: body.len() },
                    });
                    return Fetched {
                        body: Some(body),
                        attempts,
                    };
                }
                Err(refusal) => refusal,
            };

            let verdict = verdict_of(refusal.status);
            attempts.push(Attempt {
                url: url.to_owned(),
                number,
                waited_ms: waited,
                got: Got::Refused {
                    status: refusal.status,
                    detail: refusal.detail,
                    verdict,
                },
            });

            match verdict {
                Verdict::Never => break,
                Verdict::Reprime => {
                    // NOTHING TO REFRESH, OR ALREADY REFRESHED. Either way the
                    // status means what it says on its face, which is no.
                    if source.prime.is_none() || reprimed {
                        break;
                    }
                    reprimed = true;
                    prime_now(from, source, &mut attempts).await;
                    // Deliberately no wait: the session is what changed, and a
                    // backoff here would be waiting for something else.
                }
                Verdict::Again => waited = backoff_ms(number.saturating_sub(1)),
            }
        }
    }

    Fetched {
        body: None,
        attempts,
    }
}

/// Establishes a session, if this source declares one is needed.
async fn prime_once<D: crate::chain::Discovery>(
    from: &D,
    source: &Source,
    attempts: &mut Vec<Attempt>,
) {
    if source.prime.is_some() {
        prime_now(from, source, attempts).await;
    }
}

/// Establishes a session. The caller has already decided one is wanted.
async fn prime_now<D: crate::chain::Discovery>(
    from: &D,
    source: &Source,
    attempts: &mut Vec<Attempt>,
) {
    let Some(prime) = source.prime else {
        return;
    };
    let got = match from.get(prime).await {
        Ok(_) => Got::Primed,
        Err(refusal) => Got::PrimeRefused {
            detail: refusal.detail,
        },
    };
    attempts.push(Attempt {
        url: prime.to_owned(),
        number: 1,
        waited_ms: 0,
        got,
    });
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{
        Landed, MIN_BODY_BYTES, NSE_INDICES_FILE, SOURCES, Transport, free_sources, land,
        may_fetch, path_of, token_sources,
    };
    use brutex_core::vendor::Vendor;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("brutex-masters-{name}-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    fn a_master() -> String {
        let mut body = String::from("symbol,name,isin\n");
        while body.len() <= MIN_BODY_BYTES {
            body.push_str("NIFTY,NIFTY 50,INE000000000\n");
        }
        body
    }

    #[test]
    fn every_source_names_a_file_the_engine_actually_reads() {
        // A SOURCE WRITING TO A NAME NOTHING PARSES IS A DOWNLOAD THAT CHANGES
        // NOTHING. `core::vendor::master_file` is the authority on where each
        // feed's master is read from, so the table is checked against it rather
        // than against a second list written from memory.
        for source in SOURCES {
            match source.vendor {
                Some(vendor) => assert_eq!(
                    source.file,
                    vendor.master_file(),
                    "{vendor:?} is fetched into a file it is not read from"
                ),
                None => assert_eq!(
                    source.file, NSE_INDICES_FILE,
                    "the only sourceless master is NSE's own index list"
                ),
            }
        }
    }

    #[test]
    fn the_two_swept_feeds_are_covered_and_the_archives_are_not() {
        // CLAUDE.md section 1 sweeps NSE-NIFTY and NSE-BANKNIFTY, and
        // `master_file`'s own comment records that an ARCHIVE ships no master.
        let covered: Vec<Option<Vendor>> = SOURCES.iter().map(|s| s.vendor).collect();
        for vendor in [Vendor::Dhan, Vendor::Groww, Vendor::Zerodha] {
            assert!(
                covered.contains(&Some(vendor)),
                "{vendor:?} publishes a master and none is fetched"
            );
        }
        for archive in [Vendor::TrueData, Vendor::Gdfl] {
            assert!(
                !covered.contains(&Some(archive)),
                "{archive:?} is an archive and ships no master to fetch"
            );
        }
    }

    #[test]
    fn only_zerodha_spends_the_credential() {
        // THE DISTINCTION THAT DECIDES WHETHER A REFRESH MAY RUN UNATTENDED.
        // Dhan and Groww publish plain CDN files and NSE publishes its own
        // list; Zerodha's dump carries the same token every bar request spends.
        let paid: Vec<&str> = token_sources().map(|s| s.file).collect();
        assert_eq!(paid, vec![Vendor::Zerodha.master_file()]);
        assert_eq!(free_sources().count(), SOURCES.len() - 1);
    }

    #[test]
    fn every_url_is_https_and_distinct() {
        // A MASTER FETCHED OVER PLAIN HTTP IS ONE ANY NETWORK CAN REWRITE, and
        // the universe every run resolves against is what it would rewrite.
        let mut seen = std::collections::BTreeSet::new();
        for source in SOURCES {
            assert!(
                source.url.starts_with("https://"),
                "{} is not fetched over https",
                source.url
            );
            assert!(seen.insert(source.url), "{} is listed twice", source.url);
        }
    }

    #[test]
    fn a_short_body_is_refused_rather_than_written_over_a_good_master() {
        // THE REPRODUCED CASE: a CDN answering 200 with an error page. It
        // writes cleanly, the next parse succeeds on the fragment, and every
        // instrument that vanished resolves to nothing with no error anywhere.
        let dir = scratch("short-body");
        let source = &SOURCES[0];
        let good = a_master();
        assert!(land(&dir, source, &good).is_written());

        let outcome = land(&dir, source, "<html>error</html>");
        let Landed::Refused(why) = outcome else {
            panic!("a short body must be refused");
        };
        assert!(why.contains("byte(s)"), "{why}");

        // AND THE GOOD MASTER IS STILL THERE, which is the whole point.
        let held = std::fs::read_to_string(path_of(&dir, source)).expect("still readable");
        assert_eq!(held, good, "a refused body must not touch the master");
    }

    #[test]
    fn a_long_body_that_is_not_csv_is_refused() {
        // An error page long enough to clear the byte floor is still not a
        // master, and one comparison on the first line separates them.
        let dir = scratch("not-csv");
        let mut html = String::from("<!doctype html><html><body>\n");
        while html.len() <= MIN_BODY_BYTES {
            html.push_str("<p>service unavailable</p>\n");
        }
        let Landed::Refused(why) = land(&dir, &SOURCES[0], &html) else {
            panic!("a long HTML page is not a CSV");
        };
        assert!(why.contains("JSON or markup"), "{why}");
        assert!(!path_of(&dir, &SOURCES[0]).exists(), "nothing was written");
    }

    #[test]
    fn a_json_body_is_refused_even_though_json_is_full_of_commas() {
        // THE DEFECT THIS GUARD EXISTS FOR, and it shipped for one commit.
        // NSE's index list was pointed at an endpoint answering JSON. The
        // engine reads `index_name,category` CSV rows -- and JSON contains
        // commas, so the body cleared the byte floor AND the comma test, landed
        // cleanly over the operator's catalogue, and would have failed at the
        // parse one layer away from the cause.
        let dir = scratch("json-body");
        let mut json = String::from(r#"{"Broad Market Indices":["NIFTY 50","NIFTY NEXT 50""#);
        while json.len() <= MIN_BODY_BYTES {
            json.push_str(r#","NIFTY 100""#);
        }
        json.push_str("]}");

        // IT PASSES THE TWO CHECKS THAT WERE THERE BEFORE, which is the point.
        assert!(
            json.len() > MIN_BODY_BYTES,
            "long enough to clear the floor"
        );
        assert!(
            json.lines().next().unwrap_or_default().contains(','),
            "and full of commas, so the comma test cannot catch it"
        );

        let Landed::Refused(why) = land(&dir, &SOURCES[0], &json) else {
            panic!("JSON is not the CSV a master is");
        };
        assert!(why.contains("JSON or markup"), "{why}");
        assert!(!path_of(&dir, &SOURCES[0]).exists(), "nothing was written");
    }

    #[test]
    fn the_index_json_becomes_the_rows_the_catalogue_reads() {
        // THE SHAPE `api::indexmap::Published::read` PARSES, taken from its own
        // test: `index_name,category` with a header row. The endpoint answers
        // the same two fields in a different container, so this is a
        // conversion and not a guess.
        let json = r#"{
            "Broad Market Indices": ["NIFTY 50", "NIFTY NEXT 50"],
            "Sectoral Indices": ["NIFTY BANK"]
        }"#;
        let csv = super::nse_index_csv(json).expect("the shape converts");

        assert!(
            csv.starts_with("index_name,category\n"),
            "the header the reader expects: {csv}"
        );
        assert!(csv.contains("NIFTY 50,Broad Market Indices\n"), "{csv}");
        assert!(csv.contains("NIFTY BANK,Sectoral Indices\n"), "{csv}");
        assert_eq!(csv.lines().count(), 4, "header plus three indices: {csv}");
    }

    #[test]
    fn an_index_json_that_is_not_the_expected_shape_refuses_rather_than_emits() {
        // FOUR WAYS, EACH NAMED. The alternative to refusing is a catalogue
        // that parses into the wrong names, which is worse than one that does
        // not parse at all.
        for (body, expect) in [
            ("not json at all", "not JSON"),
            (r#"["NIFTY 50"]"#, "not an object"),
            (r#"{"Broad Market Indices": "NIFTY 50"}"#, "no index at all"),
            (r"{}", "no index at all"),
        ] {
            let why = super::nse_index_csv(body).expect_err("an unexpected shape must refuse");
            assert!(
                why.contains(expect),
                "expected {expect:?} in the refusal, got: {why}"
            );
        }
    }

    #[test]
    fn a_comma_in_an_index_name_refuses_rather_than_breaking_the_columns() {
        // `Published::read` SPLITS ON COMMAS AND DOES NOT UNQUOTE, so a quoted
        // field would arrive with its quotes inside the name. The only honest
        // options are refuse or corrupt, and this refuses.
        let json = r#"{"Broad Market Indices": ["NIFTY 50, LARGE CAP"]}"#;
        let why = super::nse_index_csv(json).expect_err("a comma must refuse");
        assert!(why.contains("columns do not line up"), "{why}");

        let in_category = r#"{"Broad, Market": ["NIFTY 50"]}"#;
        let why = super::nse_index_csv(in_category).expect_err("in the category too");
        assert!(why.contains("columns do not line up"), "{why}");
    }

    #[test]
    fn the_index_source_lands_through_the_conversion() {
        // END TO END THROUGH `land`: the JSON body the host answers must reach
        // disk as the CSV the engine reads. The guards run on the CONVERTED
        // body -- running them on the JSON would refuse it for opening with
        // `{`, which is true of every correct response it will ever give.
        let dir = scratch("index-lands");
        let source = SOURCES
            .iter()
            .find(|s| matches!(s.shape, super::Shape::NseIndexJson))
            .expect("the index source");

        let names: Vec<String> = (0..80).map(|n| format!(r#""NIFTY INDEX {n}""#)).collect();
        let json = format!(
            r#"{{"Broad Market Indices":["NIFTY 50",{}]}}"#,
            names.join(",")
        );

        assert!(
            land(&dir, source, &json).is_written(),
            "the conversion lands"
        );
        let held = std::fs::read_to_string(path_of(&dir, source)).expect("readable");
        assert!(held.starts_with("index_name,category\n"), "{held}");
        assert!(held.contains("NIFTY 50,Broad Market Indices\n"), "{held}");
        assert!(!held.contains('{'), "no JSON reached the file: {held}");
    }

    #[test]
    fn the_index_source_is_public_and_named_for_the_file_the_engine_reads() {
        // THE FILE `api::indexmap` READS, FETCHED RATHER THAN WAITED FOR. It
        // carries no `Vendor` because it is not a feed's master -- it is the
        // reference the feeds are checked AGAINST -- and it needs no token
        // because the exchange publishes it openly.
        let source = SOURCES
            .iter()
            .find(|s| s.file == NSE_INDICES_FILE)
            .expect("the exchange's own list is fetched, not waited for");

        assert_eq!(source.vendor, None, "the exchange is not a feed");
        assert!(
            !source.needs_token,
            "no credential is spent on a public list"
        );
        assert_eq!(source.shape, super::Shape::NseIndexJson, "it answers JSON");
        assert!(
            source.url.contains("nseindia.com"),
            "it comes from the exchange itself: {}",
            source.url
        );
    }

    #[test]
    fn an_unchanged_master_lands_and_says_it_did_not_change() {
        // A vendor that regenerates once a day answers the same bytes for the
        // rest of it. Reporting that as new data would make every refresh look
        // like a change and teach an operator to ignore the field.
        let dir = scratch("unchanged");
        let source = &SOURCES[1];
        let body = a_master();

        let Landed::Written { changed, bytes } = land(&dir, source, &body) else {
            panic!("the first write lands");
        };
        assert!(changed, "the first write is a change");
        assert_eq!(bytes, body.len());

        let Landed::Written { changed, .. } = land(&dir, source, &body) else {
            panic!("the second write lands");
        };
        assert!(!changed, "identical bytes are not a change");

        let mut moved = body.clone();
        moved.push_str("BANKNIFTY,BANK NIFTY,INE000000001\n");
        let Landed::Written { changed, .. } = land(&dir, source, &moved) else {
            panic!("the third write lands");
        };
        assert!(changed, "different bytes are a change");
    }

    #[test]
    fn no_partial_file_survives_a_landing() {
        // A `.partial` left behind grows one stale file per refresh in a
        // directory the operator reads, and a `.partial` left where the rename
        // failed is a master-sized file nothing will ever parse.
        let dir = scratch("no-partial");
        let source = &SOURCES[2];
        assert!(land(&dir, source, &a_master()).is_written());

        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .expect("readable")
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .filter(|name| name.contains("partial"))
            .collect();
        assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
    }

    #[test]
    fn a_credential_never_travels_to_a_host_that_did_not_issue_it() {
        // THE DEFECT THIS MAKES UNREPRESENTABLE. `HttpSource::get` attaches the
        // vendor's auth header to EVERY url it is handed. The masters are the
        // one list in this workspace where public urls and a credentialed url
        // are iterated together, so handing Dhan's CDN to a Zerodha-configured
        // source -- one `for` loop away -- would put the Zerodha token on
        // images.dhan.co.
        for source in SOURCES.iter().filter(|s| !s.needs_token) {
            let why = may_fetch(source, Transport::Credentialed)
                .expect_err("a public url must refuse a credentialed transport");
            assert!(why.contains("did not issue it"), "{why}");
            assert!(why.contains(source.url), "the host must be named: {why}");
            may_fetch(source, Transport::Public).expect("public over public is the pairing");
        }
    }

    #[test]
    fn a_credentialed_source_over_a_public_transport_is_refused_with_the_cause() {
        // The other direction is not a leak, it is a guaranteed 401 -- and
        // refusing here names the cause, where the vendor would only give a
        // status an operator has to interpret.
        for source in SOURCES.iter().filter(|s| s.needs_token) {
            let why = may_fetch(source, Transport::Public)
                .expect_err("a token url must refuse a transport that sends none");
            assert!(why.contains("401"), "{why}");
            may_fetch(source, Transport::Credentialed).expect("the correct pairing");
        }
    }

    #[test]
    fn every_source_has_exactly_one_transport_that_may_carry_it() {
        // NO SOURCE IS FETCHABLE BOTH WAYS, and none is fetchable neither way.
        // A source that accepted both would let the loop pick either; a source
        // that accepted neither could never be refreshed at all.
        for source in SOURCES {
            let ok: Vec<Transport> = [Transport::Public, Transport::Credentialed]
                .into_iter()
                .filter(|via| may_fetch(&source, *via).is_ok())
                .collect();
            assert_eq!(
                ok.len(),
                1,
                "{} may be fetched over {ok:?}, and exactly one is correct",
                source.file
            );
        }
    }

    #[test]
    fn two_sources_do_not_share_a_temporary_file() {
        // NAMED FOR THE TARGET. One shared `.partial` would let two concurrent
        // refreshes hand each other's bytes to the rename, and each master
        // would be internally valid and belong to the other feed.
        // No directory needed: the invariant is about NAMES, not files.
        let mut names = std::collections::BTreeSet::new();
        for source in SOURCES {
            assert!(
                names.insert(format!(".{}.partial", source.file)),
                "{} shares a temporary with another source",
                source.file
            );
        }
        assert_eq!(names.len(), SOURCES.len());
    }

    // ==================== the retry ladder ====================

    use super::{
        ATTEMPTS_PER_URL, Attempt, Clock, FIRST_BACKOFF_MS, Fetched, Got, MAX_BACKOFF_MS,
        MAX_BODY_BYTES, Pause, PublicFetch, Shape, Source, Verdict, backoff_ms, fetch, verdict_of,
    };
    use crate::chain::{Discovery, Refusal};
    use std::cell::RefCell;

    /// A transport that answers a script, and remembers what it was asked.
    ///
    /// Deliberately NOT keyed by URL: the order of the questions is half of
    /// what these tests are checking, and a map would throw it away.
    struct Scripted {
        answers: RefCell<Vec<Result<String, Refusal>>>,
        asked: RefCell<Vec<String>>,
    }

    impl Scripted {
        fn of(answers: Vec<Result<String, Refusal>>) -> Self {
            Self {
                answers: RefCell::new(answers),
                asked: RefCell::new(Vec::new()),
            }
        }

        fn refusing(status: u16, times: usize) -> Self {
            Self::of(
                (0..times)
                    .map(|_| Err(Refusal::answered(status, format!("answered {status}"))))
                    .collect(),
            )
        }

        fn asked(&self) -> Vec<String> {
            self.asked.borrow().clone()
        }
    }

    impl Discovery for Scripted {
        async fn get(&self, url: &str) -> Result<String, Refusal> {
            self.asked.borrow_mut().push(url.to_owned());
            if self.answers.borrow().is_empty() {
                // THE SCRIPT RAN OUT, which in these tests means the ladder
                // asked more times than the test expected. Saying so beats
                // looping forever or answering a default.
                return Err(Refusal::transport("the script ran out".to_owned()));
            }
            self.answers.borrow_mut().remove(0)
        }
    }

    /// A clock that records what it was asked to wait, and waits nothing.
    #[derive(Default)]
    struct Recorded {
        waits: RefCell<Vec<u64>>,
    }

    impl Recorded {
        fn waits(&self) -> Vec<u64> {
            self.waits.borrow().clone()
        }
    }

    impl Pause for Recorded {
        async fn pause(&self, ms: u64) {
            self.waits.borrow_mut().push(ms);
        }
    }

    fn plain(url: &'static str) -> Source {
        Source {
            shape: Shape::Csv,
            vendor: None,
            file: "scratch.csv",
            url,
            mirrors: &[],
            prime: None,
            needs_token: false,
        }
    }

    fn block_on<F: core::future::Future>(future: F) -> F::Output {
        // ONE CURRENT-THREAD RUNTIME PER TEST. The ladder is async because the
        // transport is; nothing in these tests actually waits, because
        // `Recorded` returns immediately.
        tokio::runtime::Builder::new_current_thread()
            // TIMERS ON, because `Clock` is one of the things under test.
            // Without this `tokio::time::sleep` panics rather than sleeping,
            // and the production `Pause` would be the one implementation no
            // test had ever entered.
            .enable_time()
            .build()
            .expect("a test runtime")
            .block_on(future)
    }

    #[test]
    fn every_status_lands_in_the_verdict_its_row_names() {
        // THE WHOLE TABLE, ROW BY ROW. A classifier is exactly as good as its
        // boundaries, so the ends of every range are here and not just a
        // representative from the middle.
        for status in [None, Some(408), Some(425), Some(429)] {
            assert_eq!(verdict_of(status), Verdict::Again, "{status:?}");
        }
        for status in [500, 502, 503, 504, 506, 507, 508, 509, 520, 522, 599] {
            assert_eq!(verdict_of(Some(status)), Verdict::Again, "{status}");
        }
        for status in [401, 403] {
            assert_eq!(verdict_of(Some(status)), Verdict::Reprime, "{status}");
        }
        for status in [
            // THE TWO PERMANENT 5xx, which sit INSIDE a range that is otherwise
            // retryable -- the exact place an ordinary `500..=599` arm would be
            // wrong, and therefore the exact place worth an assertion.
            501, 505, //
            200, 204, 301, 302, 307, 308, 400, 402, 404, 405, 410, 414, 418, 451, 510, 600,
        ] {
            assert_eq!(verdict_of(Some(status)), Verdict::Never, "{status}");
        }
    }

    #[test]
    fn the_backoff_schedule_is_the_one_documented() {
        // 500 ms, 1 s, 2 s, 4 s ... capped, and the cap is REACHED rather than
        // approached: without `checked_shl` the shift is undefined long before
        // step 64, and the last row is what proves it saturates instead.
        assert_eq!(backoff_ms(0), FIRST_BACKOFF_MS);
        assert_eq!(backoff_ms(1), 1_000);
        assert_eq!(backoff_ms(2), 2_000);
        assert_eq!(backoff_ms(3), 4_000);
        assert_eq!(backoff_ms(6), MAX_BACKOFF_MS);
        assert_eq!(backoff_ms(7), MAX_BACKOFF_MS, "capped, not doubled past it");
        assert_eq!(backoff_ms(63), MAX_BACKOFF_MS);
        assert_eq!(backoff_ms(64), MAX_BACKOFF_MS, "a shift that would be UB");
        assert_eq!(backoff_ms(u32::MAX), MAX_BACKOFF_MS);
    }

    #[test]
    fn a_body_on_the_first_ask_costs_no_wait_at_all() {
        let source = plain("https://example.invalid/a.csv");
        let from = Scripted::of(vec![Ok(a_master())]);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_some(), "it answered");
        assert_eq!(clock.waits(), Vec::<u64>::new(), "nothing was waited");
        assert_eq!(got.attempts.len(), 1, "one step: {:?}", got.attempts);
        assert_eq!(got.waited_ms(), 0);
    }

    #[test]
    fn a_transient_refusal_is_re_asked_on_the_documented_schedule() {
        // FOUR 503s AND THEN A BODY. The waits prove the ladder backed off
        // rather than hammering, and the count proves it did not give up.
        let source = plain("https://example.invalid/a.csv");
        let mut answers: Vec<Result<String, Refusal>> = (0..4)
            .map(|_| Err(Refusal::answered(503, "answered 503".to_owned())))
            .collect();
        answers.push(Ok(a_master()));
        let from = Scripted::of(answers);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_some(), "the fifth ask answered");
        assert_eq!(clock.waits(), vec![500, 1_000, 2_000, 4_000]);
        assert_eq!(got.waited_ms(), 7_500, "the ledger totals its own waits");
        assert_eq!(from.asked().len(), 5);
    }

    #[test]
    fn a_settled_refusal_is_not_re_asked_even_once() {
        // A 404 IS AN ANSWER. Five attempts at it would be four requests spent
        // learning what the first one said, which is the failure mode the
        // whole classifier exists to prevent.
        let source = plain("https://example.invalid/gone.csv");
        let from = Scripted::refusing(404, 9);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_none());
        assert_eq!(from.asked().len(), 1, "asked once: {:?}", from.asked());
        assert_eq!(clock.waits(), Vec::<u64>::new(), "and waited not at all");
        assert_eq!(got.last_refusal(), Some("answered 404"));
    }

    #[test]
    fn a_transient_refusal_gives_up_after_the_documented_number_of_asks() {
        // IT MUST TERMINATE. A ladder with no ceiling against a permanently
        // sick host is an unbounded loop inside an HTTP handler.
        let source = plain("https://example.invalid/sick.csv");
        let from = Scripted::refusing(503, 50);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_none());
        assert_eq!(from.asked().len(), ATTEMPTS_PER_URL as usize);
        assert_eq!(got.attempts.len(), ATTEMPTS_PER_URL as usize);
    }

    #[test]
    fn nothing_answered_at_all_is_retried_because_a_road_is_not_a_decision() {
        // A DROPPED SOCKET IS NOT THE HOST SAYING NO. `Refusal::transport`
        // carries no status, and treating that as settled would give up on
        // every blip, every DNS hiccup and every laptop lid.
        let source = plain("https://example.invalid/a.csv");
        let from = Scripted::of(vec![
            Err(Refusal::transport("connection reset".to_owned())),
            Err(Refusal::transport("dns failure".to_owned())),
            Ok(a_master()),
        ]);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_some());
        assert_eq!(clock.waits(), vec![500, 1_000]);
    }

    #[test]
    fn a_mirror_is_tried_only_after_the_primary_is_exhausted() {
        // NOT ALTERNATING. Five asks at the primary, then the mirror -- an
        // interleaved walk doubles the requests one blip costs and arrives no
        // sooner.
        static MIRRORS: &[&str] = &["https://mirror.invalid/a.csv"];
        let mut source = plain("https://primary.invalid/a.csv");
        source.mirrors = MIRRORS;

        let mut answers: Vec<Result<String, Refusal>> = (0..ATTEMPTS_PER_URL)
            .map(|_| Err(Refusal::answered(503, "answered 503".to_owned())))
            .collect();
        answers.push(Ok(a_master()));
        let from = Scripted::of(answers);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_some(), "the mirror answered");
        let asked = from.asked();
        assert_eq!(asked.len(), ATTEMPTS_PER_URL as usize + 1);
        for url in asked.iter().take(ATTEMPTS_PER_URL as usize) {
            assert_eq!(url, "https://primary.invalid/a.csv");
        }
        assert_eq!(
            asked.get(ATTEMPTS_PER_URL as usize).map(String::as_str),
            Some("https://mirror.invalid/a.csv")
        );
    }

    #[test]
    fn a_settled_refusal_moves_straight_to_the_mirror() {
        // A 404 AT THE PRIMARY ENDS THE PRIMARY, not the fetch. The file may
        // well be at the other address, and the whole point of a mirror is the
        // case where the first one is gone.
        static MIRRORS: &[&str] = &["https://mirror.invalid/a.csv"];
        let mut source = plain("https://primary.invalid/a.csv");
        source.mirrors = MIRRORS;

        let from = Scripted::of(vec![
            Err(Refusal::answered(404, "answered 404".to_owned())),
            Ok(a_master()),
        ]);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_some());
        assert_eq!(from.asked().len(), 2, "one at each: {:?}", from.asked());
    }

    #[test]
    fn a_session_gated_host_is_primed_before_the_first_ask() {
        // THE PRIME GOES FIRST, unconditionally, for a source that declares
        // one. Priming only after a 403 would spend a guaranteed-doomed request
        // on every single refresh.
        static PRIME: &str = "https://gated.invalid/";
        let mut source = plain("https://gated.invalid/api/list");
        source.prime = Some(PRIME);

        let from = Scripted::of(vec![Ok("<html>".to_owned()), Ok(a_master())]);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_some());
        assert_eq!(from.asked(), vec![PRIME, "https://gated.invalid/api/list"]);
        assert!(
            matches!(got.attempts.first().map(|a| &a.got), Some(Got::Primed)),
            "the prime is a row: {:?}",
            got.attempts
        );
    }

    #[test]
    fn a_forbidden_answer_reprimes_exactly_once_and_then_stops() {
        // 403 IS AMBIGUOUS AND THE LADDER RESOLVES IT BY EXPERIMENT: refresh
        // the session and ask once more. Twice would be asking a genuinely dead
        // credential over and over, which is the auto-retry that hides a
        // permanent fault.
        static PRIME: &str = "https://gated.invalid/";
        let mut source = plain("https://gated.invalid/api/list");
        source.prime = Some(PRIME);

        let from = Scripted::of(vec![
            Ok("<html>".to_owned()),                                // prime
            Err(Refusal::answered(403, "answered 403".to_owned())), // ask 1
            Ok("<html>".to_owned()),                                // re-prime
            Err(Refusal::answered(403, "answered 403".to_owned())), // ask 2
            Ok(a_master()),                                         // never reached
        ]);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_none(), "twice forbidden is forbidden");
        assert_eq!(from.asked().len(), 4, "{:?}", from.asked());
        assert_eq!(
            clock.waits(),
            Vec::<u64>::new(),
            "a re-prime is not a backoff"
        );
    }

    #[test]
    fn a_reprime_that_works_gets_the_file() {
        static PRIME: &str = "https://gated.invalid/";
        let mut source = plain("https://gated.invalid/api/list");
        source.prime = Some(PRIME);

        let from = Scripted::of(vec![
            Ok("<html>".to_owned()),
            Err(Refusal::answered(403, "answered 403".to_owned())),
            Ok("<html>".to_owned()),
            Ok(a_master()),
        ]);
        let clock = Recorded::default();

        assert!(block_on(fetch(&from, &clock, &source)).body.is_some());
    }

    #[test]
    fn a_forbidden_answer_with_nothing_to_reprime_stops_immediately() {
        // WITHOUT A PRIME THERE IS NO EXPERIMENT TO RUN. A 401 from a
        // credentialed host means the credential, and re-asking it four more
        // times is `CLAUDE.md` §4's auto-retry against an unchanged dead value.
        let source = plain("https://api.invalid/instruments");
        let from = Scripted::refusing(401, 9);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_none());
        assert_eq!(from.asked().len(), 1, "{:?}", from.asked());
    }

    #[test]
    fn a_prime_that_refuses_is_recorded_and_the_real_ask_still_goes_out() {
        // §4 BANS A FALLBACK THAT HIDES A FAILURE, not one that reports it.
        // The prime is unverified (see `Source::prime`), so a prime that fails
        // may mean nothing at all -- and the host's own answer to the real
        // request is more informative than this module's guess about it.
        static PRIME: &str = "https://gated.invalid/";
        let mut source = plain("https://gated.invalid/api/list");
        source.prime = Some(PRIME);

        let from = Scripted::of(vec![
            Err(Refusal::answered(
                503,
                "the homepage answered 503".to_owned(),
            )),
            Ok(a_master()),
        ]);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        assert!(got.body.is_some(), "the real ask still happened");
        assert!(
            matches!(
                got.attempts.first().map(|a| &a.got),
                Some(Got::PrimeRefused { .. })
            ),
            "and the failed prime is a ROW, not a silence: {:?}",
            got.attempts
        );
    }

    #[test]
    fn the_ledger_names_the_url_the_number_and_the_verdict() {
        // AN OPERATOR READING "IT FAILED" HAS BEEN TOLD NOTHING. Which URL,
        // how many times, and what the host said are the three questions, and
        // all three are lost if a ladder collapses into a bare `Result`.
        let source = plain("https://example.invalid/a.csv");
        let from = Scripted::refusing(503, 9);
        let clock = Recorded::default();

        let got = block_on(fetch(&from, &clock, &source));

        for (index, step) in got.attempts.iter().enumerate() {
            assert_eq!(step.url, "https://example.invalid/a.csv");
            assert_eq!(step.number as usize, index + 1, "numbered from one");
            match &step.got {
                Got::Refused {
                    status, verdict, ..
                } => {
                    assert_eq!(*status, Some(503));
                    assert_eq!(*verdict, Verdict::Again);
                }
                other => panic!("expected a refusal, got {other:?}"),
            }
        }
        assert_eq!(got.last_refusal(), Some("answered 503"));
    }

    #[test]
    fn an_empty_ledger_has_no_last_word_and_no_waits() {
        // THE `None` ARMS OF BOTH ACCESSORS, which a happy-path test never
        // reaches -- and an accessor whose empty case is unexercised is where
        // an `unwrap` hides.
        let empty = Fetched {
            body: None,
            attempts: Vec::new(),
        };
        assert_eq!(empty.last_refusal(), None);
        assert_eq!(empty.waited_ms(), 0);

        let only_ok = Fetched {
            body: Some("x".to_owned()),
            attempts: vec![Attempt {
                url: "u".to_owned(),
                number: 1,
                waited_ms: 3,
                got: Got::Body { bytes: 1 },
            }],
        };
        assert_eq!(only_ok.last_refusal(), None, "a body is not a refusal");
        assert_eq!(only_ok.waited_ms(), 3);
    }

    #[test]
    fn every_url_is_the_primary_then_the_mirrors_in_order() {
        static MIRRORS: &[&str] = &["https://b.invalid/", "https://c.invalid/"];
        let mut source = plain("https://a.invalid/");
        source.mirrors = MIRRORS;

        assert_eq!(
            source.every_url().collect::<Vec<_>>(),
            vec![
                "https://a.invalid/",
                "https://b.invalid/",
                "https://c.invalid/"
            ]
        );
        assert_eq!(
            plain("https://a.invalid/").every_url().collect::<Vec<_>>(),
            vec!["https://a.invalid/"],
            "a source with no mirror still yields itself"
        );
    }

    #[test]
    fn the_real_clock_waits_and_is_reachable() {
        // THE PRODUCTION `Pause`, exercised at one millisecond so it costs
        // nothing. Without this the only implementation that ever runs in
        // anger is the one no test has ever entered.
        block_on(async {
            let before = std::time::Instant::now();
            Clock.pause(1).await;
            assert!(before.elapsed() >= std::time::Duration::from_millis(1));
        });
    }

    #[test]
    fn the_body_ceiling_is_far_above_any_real_master_and_far_below_this_machine() {
        // A CEILING IS ONLY USEFUL IF IT SEPARATES THE TWO CASES. Zerodha's
        // full dump is tens of megabytes; this machine has 48 GB.
        const {
            assert!(
                MAX_BODY_BYTES > 100 * 1_024 * 1_024,
                "above any real master"
            );
            assert!(MAX_BODY_BYTES < 1_024 * 1_024 * 1_024, "below one gigabyte");
            assert!(
                MAX_BODY_BYTES > MIN_BODY_BYTES,
                "the floor and the ceiling are the right way round"
            );
        }
    }

    #[test]
    fn the_credentialed_source_declares_no_prime() {
        // A PRIME IS AN EXTRA ADDRESS THE CREDENTIAL WOULD TRAVEL TO. Whatever
        // else changes in the table, the source that spends the shared token
        // must keep its request count at one host.
        for source in SOURCES {
            if source.needs_token {
                assert_eq!(
                    source.prime, None,
                    "{:?} spends a credential and declares a second host",
                    source.vendor
                );
            }
        }
    }

    #[test]
    fn a_cookie_never_leaves_the_host_that_set_it() {
        // THE SECURITY PROPERTY OF THE HAND-WRITTEN JAR, and the only reason it
        // is keyed by host rather than being one string. A session established
        // at the exchange must not ride along to a vendor CDN.
        let fetch = PublicFetch::new().expect("a public transport");
        fetch.remember_pairs("www.nseindia.com", &["nsit=abc; Path=/; HttpOnly"]);

        assert_eq!(
            fetch.cookies_for("www.nseindia.com").as_deref(),
            Some("nsit=abc"),
            "the host that set it gets it back"
        );
        assert_eq!(
            fetch.cookies_for("images.dhan.co"),
            None,
            "and nobody else does"
        );
    }

    #[test]
    fn the_cookie_header_is_byte_identical_for_the_same_set() {
        // §3 RULE 5 IS IDEMPOTENCE. A `HashMap` would order the fields by hash
        // seed, so the same session would produce a different request line from
        // one process to the next -- a difference nobody chose and nobody could
        // reproduce.
        let fetch = PublicFetch::new().expect("a public transport");
        fetch.remember_pairs(
            "host.invalid",
            &["zeta=1; Path=/", "alpha=2", "middle=3; Secure"],
        );
        assert_eq!(
            fetch.cookies_for("host.invalid").as_deref(),
            Some("alpha=2; middle=3; zeta=1"),
            "sorted by name, not by arrival"
        );
    }

    #[test]
    fn a_later_value_replaces_an_earlier_one_rather_than_duplicating_it() {
        // A HOST ROTATING A SESSION SENDS THE SAME NAME AGAIN. Appending would
        // present both, and a host reading the first would get the stale one.
        let fetch = PublicFetch::new().expect("a public transport");
        fetch.remember_pairs("host.invalid", &["sid=old"]);
        fetch.remember_pairs("host.invalid", &["sid=new"]);
        assert_eq!(
            fetch.cookies_for("host.invalid").as_deref(),
            Some("sid=new")
        );
    }

    #[test]
    fn a_malformed_set_cookie_is_skipped_rather_than_stored_as_nonsense() {
        // FOUR SHAPES THAT ARE NOT A PAIR. None of them should become a cookie,
        // and none of them should stop the ones beside it being stored.
        let fetch = PublicFetch::new().expect("a public transport");
        fetch.remember_pairs(
            "host.invalid",
            &["novalue", "=orphan", "  ; Path=/", "good=yes"],
        );
        assert_eq!(
            fetch.cookies_for("host.invalid").as_deref(),
            Some("good=yes"),
            "the one well-formed pair, and nothing else"
        );
    }

    #[test]
    fn a_host_that_set_nothing_sends_no_cookie_header_at_all() {
        // AN EMPTY HEADER IS NOT THE SAME AS NO HEADER, and a `Cookie:` with an
        // empty value is a request shape no browser produces.
        let fetch = PublicFetch::new().expect("a public transport");
        assert_eq!(fetch.cookies_for("never-seen.invalid"), None);
        fetch.remember_pairs("empty.invalid", &["not-a-pair"]);
        assert_eq!(
            fetch.cookies_for("empty.invalid"),
            None,
            "an entry with nothing in it is still nothing to send"
        );
    }

    #[test]
    fn a_url_with_no_host_is_carried_without_a_jar_lookup() {
        // `host_of` IS FALLIBLE AND ITS `None` ARM IS REACHABLE: a source URL
        // is `&'static str` and nothing type-checks it as a URL. The fetch must
        // still be attempted -- the host's own answer is more useful than this
        // module refusing on a parse it did for its own bookkeeping.
        assert_eq!(PublicFetch::host_of("not a url at all"), None);
        assert_eq!(PublicFetch::host_of("file:///tmp/x.csv"), None);
        assert_eq!(
            PublicFetch::host_of("https://images.dhan.co/api-data/x.csv").as_deref(),
            Some("images.dhan.co")
        );
    }

    #[test]
    fn a_json_body_that_will_not_convert_refuses_at_the_landing_and_names_the_url() {
        // THE CONVERSION'S `Err` ARM, REACHED THROUGH `land`. `nse_index_csv`
        // refusing is tested directly; this is the different question of what
        // the operator is handed when it does -- and the answer must name the
        // URL, because "it refused" against four sources identifies none.
        let dir = scratch("json-will-not-convert");
        let source = SOURCES
            .iter()
            .find(|s| s.shape == super::Shape::NseIndexJson)
            .expect("the index source");

        let landed = land(&dir, source, "<html>a block page</html>");
        let Landed::Refused(ref why) = landed else {
            unreachable!("an unconvertible body cannot be written")
        };
        assert!(why.contains(source.url), "names the url: {why}");
        assert!(why.contains("not JSON"), "and the cause: {why}");
        assert!(
            !path_of(&dir, source).exists(),
            "and nothing reached the disk"
        );
    }

    #[test]
    fn a_body_whose_first_line_has_no_comma_is_refused_with_that_line_quoted() {
        // AN ERROR PAGE LONG ENOUGH TO CLEAR THE BYTE FLOOR and not starting
        // with a JSON or HTML opener -- a plain-text splash, a proxy notice, a
        // maintenance message. It is none of the shapes the other guards catch,
        // and it would otherwise overwrite a working master.
        let dir = scratch("no-comma");
        let source = &SOURCES[0];

        let mut body = String::from("SERVICE TEMPORARILY UNAVAILABLE\n");
        while body.len() <= MIN_BODY_BYTES {
            body.push_str("please try again later\n");
        }

        let landed = land(&dir, source, &body);
        let Landed::Refused(ref why) = landed else {
            unreachable!("a body with no columns cannot be a master")
        };
        assert!(why.contains("no comma"), "{why}");
        assert!(
            why.contains("SERVICE TEMPORARILY UNAVAILABLE"),
            "the first line is quoted back so an operator can see it: {why}"
        );
        assert!(!path_of(&dir, source).exists(), "nothing reached the disk");
    }

    #[test]
    fn an_index_document_with_a_non_string_name_skips_it_rather_than_refusing() {
        // A CATEGORY WHOSE LIST HOLDS A NUMBER, AND ONE WHOSE VALUE IS NOT A
        // LIST AT ALL. Neither is a name, and neither is a reason to discard
        // the names that ARE there -- `Published::read` wants every index NSE
        // publishes, and refusing the document over one malformed element
        // would cost all of them.
        let json = r#"{
            "Broad Market Indices": ["NIFTY 50", 42, null, "NIFTY NEXT 50"],
            "Not A List": "NIFTY BANK"
        }"#;
        let csv = super::nse_index_csv(json).expect("the good names still convert");

        assert!(csv.contains("NIFTY 50,Broad Market Indices\n"), "{csv}");
        assert!(
            csv.contains("NIFTY NEXT 50,Broad Market Indices\n"),
            "{csv}"
        );
        assert!(!csv.contains("42"), "a number is not an index name: {csv}");
        assert!(
            !csv.contains("NIFTY BANK"),
            "a category whose value is not a list contributes nothing: {csv}"
        );
        assert_eq!(csv.lines().count(), 3, "header plus two names: {csv}");
    }
}

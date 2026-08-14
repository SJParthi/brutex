//! One pass over the exchange's directory, and the dated snapshot it produces.
//!
//! # The two rules this module exists to hold
//!
//! **Every file in one pass is the same age.** The pass carries one day, and a
//! resolution is stamped with it rather than with the moment each document
//! happened to arrive. Comparing an index list fetched now against a master
//! fetched yesterday is a difference report about *time*, and it reads exactly
//! like a coverage report — which is the worse of the two to be wrong about.
//!
//! **A snapshot is published whole or not at all.** Four of five index files
//! fetching cleanly is not a smaller universe, it is an unknown one, and
//! letting it replace a complete predecessor loses the only good answer there
//! was. [`Snapshot::admits_publication`] refuses it and names what failed.
//!
//! # No socket, and this is the module where that is a design decision
//!
//! [`DocumentSource`] is the seam, exactly as [`crate::fetch::BarSource`] is
//! for vendor bars and [`crate::secret::SecretSource`] is for credentials.
//! Everything above it — the crawl order, the failure collection, the
//! whole-or-nothing rule, the digest — is transport-blind and is proved against
//! [`FakeDocuments`]. `CLAUDE.md` requires this crate to build and test with no
//! live vendor call, and a driver that can only be exercised against the
//! network is one nobody exercises.
//!
//! # Idempotence
//!
//! Same inputs on the same day produce the same bytes. [`Snapshot::to_wire`]
//! writes in a fixed order and carries no instant finer than the day, so a
//! re-run is free and a diff between two snapshots means something. That is
//! `CLAUDE.md` §3 rule 5, and it is what makes the append-only store safe to
//! write a resolution into every day.

use core::future::Future;

use sha2::{Digest, Sha256};
use std::fmt::Write as _;

use crate::nse::{self, Category};
use crate::session::Day;
use crate::universe::{IndexResolution, JoinKey, VendorInstrument};

/// Where a document's bytes come from. The only thing a transport must satisfy.
///
/// Returning the body as a `String` rather than bytes is deliberate: every
/// document this module reads is text the exchange serves as text, and a
/// transport that cannot produce valid UTF-8 for one has already found
/// something worth refusing.
pub trait DocumentSource {
    /// Fetches one document.
    ///
    /// # Errors
    ///
    /// Whatever the transport refuses, in its own words. It is a `String`
    /// because this trait is the boundary: below it live sockets and TLS and
    /// redirect policy, and none of that belongs in a type this crate matches
    /// on.
    fn get(&self, url: &str) -> impl Future<Output = Result<String, String>>;
}

/// One index that did not resolve, and why.
///
/// **Collected rather than propagated.** A pass over 148 indices that dies on
/// the first network blip is a pass that never finishes — the same
/// per-instrument isolation `broker_run` applies to a backfill. The failure is
/// recorded against its own index and the crawl continues, and the
/// whole-or-nothing rule is applied once at the end where it can see everything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexFailure {
    /// The page or file that failed, as the crawl addressed it.
    pub at: String,
    /// Why, in the words of whatever refused.
    pub why: String,
}

/// What one pass produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The day this pass ran. **The whole pass shares it** — see the module
    /// header on why one age matters more than an exact instant.
    pub day: Day,
    /// The feed every resolution here was joined against.
    pub feed: String,
    /// Which key that feed's join used, and therefore how much a match proves.
    pub key: JoinKey,
    /// One resolution per index that resolved, in crawl order.
    pub resolutions: Vec<IndexResolution>,
    /// Every index that did not, in crawl order.
    pub failures: Vec<IndexFailure>,
}

impl Snapshot {
    /// Whether every index the crawl found also resolved.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.failures.is_empty()
    }

    /// Whether every resolution's partition sums.
    ///
    /// Asked of the snapshot as a whole so a caller reading one back from disk
    /// can re-verify the one thing that is checkable without the source files.
    #[must_use]
    pub fn is_sound(&self) -> bool {
        self.resolutions.iter().all(IndexResolution::is_sound)
    }

    /// How many published names this snapshot covers, across every index.
    ///
    /// Names are counted **per index**, so an equity in both the NIFTY 50 and
    /// the NIFTY 500 counts twice. That is the right number for the question
    /// this feeds — [`crate::universe::admits_replacement`], which compares two
    /// passes of the same shape — and it is the wrong number for "how many
    /// distinct instruments exist", which nothing here claims to answer.
    #[must_use]
    pub fn published(&self) -> usize {
        self.resolutions
            .iter()
            .map(IndexResolution::published)
            .sum()
    }

    /// Whether this snapshot may replace `previous`, or must halt and say why.
    ///
    /// # Errors
    ///
    /// A sentence naming what is wrong, for three separate reasons — and they
    /// are separate because they send an operator to three different places:
    ///
    /// 1. **Incomplete.** Some index did not resolve. Publishing a partial pass
    ///    over a complete one destroys the only good answer there was, and the
    ///    partial one is indistinguishable from a universe that shrank.
    /// 2. **Unsound.** A partition did not sum, which is a defect in the join
    ///    rather than in the data, and no amount of re-fetching fixes it.
    /// 3. **Collapsed.** More names vanished than a rebalance explains — see
    ///    [`crate::universe::admits_replacement`].
    ///
    /// A `None` previous is the first pass, and the first pass has nothing to
    /// collapse against. It still has to be complete and sound.
    pub fn admits_publication(&self, previous: Option<&Self>) -> Result<(), String> {
        if !self.is_complete() {
            let named: Vec<&str> = self
                .failures
                .iter()
                .take(3)
                .map(|f| f.at.as_str())
                .collect();
            return Err(format!(
                "{} of {} index file(s) did not resolve — {} and so on. Nothing \
                 was published: a partial pass is not a smaller universe, it is \
                 an unknown one, and letting it replace a complete predecessor \
                 loses the only good answer there was.",
                self.failures.len(),
                self.failures.len().saturating_add(self.resolutions.len()),
                named.join(", ")
            ));
        }
        if !self.is_sound() {
            return Err(
                "a resolution's buckets do not add up to the names it published. \
                 That is a defect in the JOIN and not in the data, so re-fetching \
                 will not close it. Nothing was published."
                    .to_owned(),
            );
        }
        match previous {
            None => Ok(()),
            Some(last) => crate::universe::admits_replacement(last.published(), self.published()),
        }
    }

    /// This snapshot as bytes, in a fixed order, carrying no instant finer than
    /// the day.
    ///
    /// # Why the shape is this austere
    ///
    /// It is what [`Self::digest`] hashes, so every byte in it is a byte two
    /// runs must agree on. A field that moved — a fetch instant, a duration, a
    /// map iterated in hash order — would make every re-run differ and every
    /// diff meaningless, which is the failure `CLAUDE.md` §3 rule 5 names.
    ///
    /// Rows are written in the exchange's own file order and indices in crawl
    /// order. Neither is sorted: both orders are facts about the source, and
    /// re-ordering them would be this repository inventing a sequence.
    #[must_use]
    pub fn to_wire(&self) -> String {
        let mut out = String::new();
        // The day, the feed and the key first — the three facts that qualify
        // every number below them.
        let _ = writeln!(out, "day\t{}", self.day);
        let _ = writeln!(out, "feed\t{}", self.feed);
        let _ = writeln!(out, "key\t{}", self.key);
        for resolution in &self.resolutions {
            let _ = writeln!(
                out,
                "index\t{}\t{}\t{}",
                resolution.index,
                resolution.published(),
                resolution.extra
            );
            for row in &resolution.rows {
                let _ = writeln!(
                    out,
                    "row\t{}\t{}\t{}\t{}",
                    row.symbol,
                    row.isin,
                    row.verdict,
                    row.vendor_id.as_deref().unwrap_or("")
                );
            }
        }
        out
    }

    /// A content digest over [`Self::to_wire`].
    ///
    /// Two passes that saw the same files on the same day produce the same
    /// digest, so "did anything move" is one comparison rather than a walk. The
    /// failures are deliberately **not** hashed: a snapshot with failures never
    /// reaches publication, so a digest over them would only ever distinguish
    /// two things neither of which is stored.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.to_wire().as_bytes());
        hasher.finalize().into()
    }
}

/// Crawl the exchange's directory and resolve every index it lists.
///
/// # The order is the order, and it is not an optimisation
///
/// Category page → index page → constituent file. Each step reads a URL the
/// previous step published; nothing is composed. [`crate::nse`]'s header
/// carries the argument, and it is the reason this function has no parameter
/// for a filename pattern.
///
/// # Failures do not abort the pass
///
/// An index whose page is unreachable, whose link is missing, or whose file
/// does not parse is recorded in [`Snapshot::failures`] and the crawl moves on.
/// The whole-or-nothing rule is applied once, afterwards, by
/// [`Snapshot::admits_publication`] — where it can see every failure rather
/// than only the first.
///
/// # Cost
///
/// One request per category, one per index, one per constituent file: 4 + 2n
/// for n indices, which is 300 for the 148 the exchange published on 14 Aug
/// 2026. The join inside is [`crate::universe::resolve`]'s, which indexes the
/// master **once** for the whole pass rather than per index.
pub async fn crawl<S: DocumentSource>(
    source: &S,
    host: &str,
    day: Day,
    feed: &str,
    key: JoinKey,
    master: &[VendorInstrument<'_>],
) -> Snapshot {
    let mut resolutions = Vec::new();
    let mut failures = Vec::new();

    for category in Category::ALL {
        let listing_url = format!("{host}{}", category.path());
        let listing = match source.get(&listing_url).await {
            Ok(body) => body,
            Err(why) => {
                failures.push(IndexFailure {
                    at: listing_url,
                    why,
                });
                continue;
            }
        };
        // A LISTING THAT YIELDS NOTHING IS A FAILURE, NOT AN EMPTY CATEGORY.
        //
        // The exchange publishes no empty category, so zero links means the
        // page stopped emitting them as markup or the body is not the page at
        // all. Reading it as "this family has no indices" is how a universe
        // silently loses 34 sets.
        let links = nse::index_links(&listing, category);
        if links.is_empty() {
            failures.push(IndexFailure {
                at: listing_url,
                why: format!(
                    "this listing yielded no index links. The exchange publishes \
                     none empty — it listed {} on 14 Aug 2026 — so zero here is \
                     the page having changed shape, not the family having \
                     emptied.",
                    category.counted_on_14_aug_2026()
                ),
            });
            continue;
        }

        for link in links {
            let page_url = format!("{host}{}", link.path);
            let page = match source.get(&page_url).await {
                Ok(body) => body,
                Err(why) => {
                    failures.push(IndexFailure { at: page_url, why });
                    continue;
                }
            };
            // READ, NEVER COMPOSED. See `nse::constituent_link`.
            let csv_url = match nse::constituent_link(&page, &link.path) {
                Ok(url) => url,
                Err(why) => {
                    failures.push(IndexFailure {
                        at: page_url,
                        why: why.to_string(),
                    });
                    continue;
                }
            };
            let body = match source.get(&csv_url).await {
                Ok(body) => body,
                Err(why) => {
                    failures.push(IndexFailure { at: csv_url, why });
                    continue;
                }
            };
            match nse::constituents(&body) {
                Err(why) => failures.push(IndexFailure {
                    at: csv_url,
                    why: why.to_string(),
                }),
                Ok(published) => resolutions.push(crate::universe::resolve(
                    &link.path, feed, key, &published, master,
                )),
            }
        }
    }

    Snapshot {
        day,
        feed: feed.to_owned(),
        key,
        resolutions,
        failures,
    }
}

/// How long one document fetch may take before it is abandoned.
///
/// A hung socket with no timeout is a pass that never finishes and never says
/// why, which is worse than a refusal: the operator has nothing to act on. The
/// same argument [`crate::http::REQUEST_TIMEOUT_SECS`] makes, and the same
/// number, because these documents are smaller than a bar window and there is
/// no reason for a second figure.
pub const DOCUMENT_TIMEOUT_SECS: u64 = 30;

/// The real transport: HTTPS, bounded, following no redirect.
///
/// # This is the only thing in the pipeline that opens a socket
///
/// Every other stage — the crawl order, the parse, the join, the buckets, the
/// snapshot — is a pure function proved against [`FakeDocuments`]. That is not
/// an accident of testing convenience: it is what lets the whole resolution be
/// exercised, and changed, without depending on a third party's uptime or on
/// today's index membership.
///
/// # Redirects are not followed, and here the reason is not a credential
///
/// [`crate::http::HttpSource`] refuses them because a cross-origin hop can
/// carry a broker token. Nothing here carries one — these are public exchange
/// documents and this client sends no credential at all. The reason is the
/// second half of that argument: a constituent file answering 3xx is a route
/// change, and silently chasing it means parsing whatever the new location
/// serves as though it were the index's membership. The 3xx comes back as a
/// refusal naming the status instead.
#[derive(Debug)]
pub struct HttpDocuments {
    client: reqwest::Client,
}

impl HttpDocuments {
    /// Builds the client.
    ///
    /// # Errors
    ///
    /// A sentence when the client cannot be constructed, which on this path
    /// means the TLS backend is unavailable — a deployment fault rather than an
    /// exchange one, and worth distinguishing.
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(core::time::Duration::from_secs(DOCUMENT_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|why| format!("the HTTPS client could not be built: {why}"))?;
        Ok(Self { client })
    }

    /// Fetches one document, bounded.
    ///
    /// # Errors
    ///
    /// A sentence naming the status for a non-success answer, the byte count
    /// for one past [`crate::nse::MAX_DOCUMENT_BYTES`], and the transport's own
    /// words when no answer arrived at all. Each is a different fault and each
    /// sends an operator somewhere different, so none of them is flattened into
    /// "it failed".
    ///
    /// **The size is checked against the declared length before the body is
    /// read**, and against the body afterwards. The first is a courtesy the
    /// host may decline to offer; the second is the one that binds, because a
    /// `Content-Length` is a claim and the bytes are the fact.
    pub async fn get_async(&self, url: &str) -> Result<String, String> {
        let answer = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|why| format!("{url} was not reached: {why}"))?;

        let status = answer.status();
        if !status.is_success() {
            // A 3xx is a route change, and it is named as one rather than left
            // to read as a server error — see this type's own documentation.
            let hint = if status.is_redirection() {
                answer
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .map_or_else(String::new, |to| format!(" It points at {to}."))
            } else {
                String::new()
            };
            return Err(format!(
                "{url} answered {}.{hint} Nothing was read: this pass does not \
                 follow a redirect, because a constituent file answering 3xx is \
                 a route change and parsing whatever the new location serves \
                 would file it as the index's membership.",
                status.as_u16()
            ));
        }

        if let Some(declared) = answer.content_length() {
            let cap = crate::nse::MAX_DOCUMENT_BYTES as u64;
            if declared > cap {
                return Err(format!(
                    "{url} declares {declared} bytes and this build accepts at \
                     most {cap}. Refused before the body was read."
                ));
            }
        }

        let body = answer
            .text()
            .await
            .map_err(|why| format!("{url} answered, and the body could not be read: {why}"))?;

        // THE BYTES ARE THE FACT. `Content-Length` is a claim the host makes and
        // may omit or get wrong; this is the check that actually binds.
        if body.len() > crate::nse::MAX_DOCUMENT_BYTES {
            return Err(format!(
                "{url} answered {} bytes and this build accepts at most {}",
                body.len(),
                crate::nse::MAX_DOCUMENT_BYTES
            ));
        }
        Ok(body)
    }
}

impl DocumentSource for HttpDocuments {
    fn get(&self, url: &str) -> impl Future<Output = Result<String, String>> {
        self.get_async(url)
    }
}

/// A source backed by a fixed map, for tests and for nothing else.
///
/// Every test in this module runs against one. The alternative — proving the
/// crawl against the live exchange — makes the suite depend on a third party's
/// uptime and on today's index membership, and a test whose expected value
/// changes on a rebalance is a test nobody trusts.
#[derive(Debug, Default, Clone)]
pub struct FakeDocuments {
    /// `(url, body)` — or `(url, Err)` for a route that refuses.
    pub answers: Vec<(String, Result<String, String>)>,
}

impl FakeDocuments {
    /// Records a body for a URL, **replacing** any answer already held for it.
    #[must_use]
    pub fn with(self, url: &str, body: &str) -> Self {
        self.answering(url, Ok(body.to_owned()))
    }

    /// Records a refusal for a URL, replacing any answer already held for it.
    #[must_use]
    pub fn refusing(self, url: &str, why: &str) -> Self {
        self.answering(url, Err(why.to_owned()))
    }

    /// REPLACES RATHER THAN APPENDS, and the first draft of this appended.
    ///
    /// With `push` and a `find` that takes the first match, a test building on
    /// a complete fixture and overriding one route got the ORIGINAL answer
    /// back — silently, because nothing distinguishes "the fixture I set" from
    /// "an earlier one that shadowed it". Seven tests passed for the wrong
    /// reason before this was fixed.
    ///
    /// It is the same defect shape this module refuses in the data it reads: a
    /// second row claiming a key it already holds must not be resolved by
    /// whichever came first. A test fixture is allowed to have last-wins
    /// semantics — it is a builder, and later calls are a caller's later
    /// intent — but it must actually have them rather than appear to.
    #[must_use]
    fn answering(mut self, url: &str, answer: Result<String, String>) -> Self {
        self.answers.retain(|(at, _)| at != url);
        self.answers.push((url.to_owned(), answer));
        self
    }
}

impl DocumentSource for FakeDocuments {
    async fn get(&self, url: &str) -> Result<String, String> {
        self.answers.iter().find(|(at, _)| at == url).map_or_else(
            || Err(format!("no fixture for {url}")),
            |(_, answer)| answer.clone(),
        )
    }
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
    use super::*;

    const HOST: &str = "https://exchange.invalid";

    fn csv() -> String {
        "Company Name,Industry,Symbol,Series,ISIN Code\n\
         A Ltd.,Fin,AAA,EQ,INE000A01001\n\
         B Ltd.,Fin,BBB,EQ,INE000B01002\n"
            .to_owned()
    }

    fn master() -> Vec<VendorInstrument<'static>> {
        vec![
            VendorInstrument {
                vendor_id: "1",
                trading_symbol: "AAA",
                isin: "INE000A01001",
            },
            VendorInstrument {
                vendor_id: "2",
                trading_symbol: "BBB",
                isin: "INE000B01002",
            },
        ]
    }

    /// Every category answers, one index each, all resolving.
    fn complete() -> FakeDocuments {
        let mut docs = FakeDocuments::default();
        for category in Category::ALL {
            let listing = format!("{HOST}{}", category.path());
            let index_path = format!("{}/one", category.path());
            let page = format!("{HOST}{index_path}");
            let file = format!("{HOST}/IndexConstituent/ind_one.csv");
            docs = docs
                .with(&listing, &format!(r#"<a href="{index_path}">an index</a>"#))
                .with(&page, &format!(r#"<a href="{file}">Download</a>"#))
                .with(&file, &csv());
        }
        docs
    }

    fn day() -> Day {
        Day::new(2026, 8, 14).expect("a real day")
    }

    #[tokio::test]
    async fn a_clean_pass_resolves_every_index_and_is_publishable() {
        let snap = crawl(&complete(), HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert_eq!(snap.resolutions.len(), 4, "one index per category");
        assert!(snap.is_complete());
        assert!(snap.is_sound());
        assert_eq!(snap.published(), 8, "two names in each of four indices");
        snap.admits_publication(None)
            .expect("a complete, sound first pass publishes");
    }

    /// **Same inputs, same day, same bytes.** The property that makes a re-run
    /// free and a diff meaningful.
    #[tokio::test]
    async fn two_passes_over_the_same_documents_are_byte_identical() {
        let once = crawl(&complete(), HOST, day(), "groww", JoinKey::Isin, &master()).await;
        let twice = crawl(&complete(), HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert_eq!(once.to_wire(), twice.to_wire());
        assert_eq!(once.digest(), twice.digest());
        assert!(
            !once.to_wire().is_empty(),
            "an empty wire would make this test pass for free"
        );
    }

    #[tokio::test]
    async fn a_changed_membership_changes_the_digest() {
        let base = crawl(&complete(), HOST, day(), "groww", JoinKey::Isin, &master()).await;
        // One name leaves every index.
        let smaller = complete().with(
            &format!("{HOST}/IndexConstituent/ind_one.csv"),
            "Company Name,Industry,Symbol,Series,ISIN Code\n\
             A Ltd.,Fin,AAA,EQ,INE000A01001\n",
        );
        let moved = crawl(&smaller, HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert_ne!(
            base.digest(),
            moved.digest(),
            "a membership change must be visible as a digest change"
        );
    }

    /// **Whole or not at all.** Four of five is an unknown universe.
    #[tokio::test]
    async fn a_partial_pass_never_replaces_a_complete_one() {
        let broken = complete().refusing(
            &format!("{HOST}{}", Category::Sectoral.path()),
            "503 from the exchange",
        );
        let snap = crawl(&broken, HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert!(!snap.is_complete());
        assert_eq!(snap.resolutions.len(), 3, "the other three still resolved");
        let why = snap
            .admits_publication(None)
            .expect_err("a partial pass is not publishable");
        assert!(why.contains("unknown one"), "{why}");
        assert!(
            why.contains("sectoral-indices"),
            "it names what failed: {why}"
        );
    }

    /// A listing that yields nothing is the page changing shape, never a family
    /// that emptied.
    #[tokio::test]
    async fn an_empty_listing_is_a_failure_and_not_a_category_with_no_indices() {
        let blank = complete().with(
            &format!("{HOST}{}", Category::Strategy.path()),
            "<html></html>",
        );
        let snap = crawl(&blank, HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert!(!snap.is_complete());
        let failure = snap
            .failures
            .iter()
            .find(|f| f.at.contains("strategy"))
            .expect("the empty listing failed");
        assert!(
            failure.why.contains("48"),
            "it cites what it counted: {failure:?}"
        );
        assert!(failure.why.contains("changed shape"));
    }

    #[tokio::test]
    async fn one_unreachable_index_does_not_abort_the_other_three() {
        let one_gone = complete().refusing(
            &format!("{HOST}{}/one", Category::Thematic.path()),
            "connection reset",
        );
        let snap = crawl(&one_gone, HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert_eq!(snap.resolutions.len(), 3);
        assert_eq!(snap.failures.len(), 1);
        assert_eq!(snap.failures[0].why, "connection reset");
    }

    /// An interstitial served where a CSV was asked for.
    #[tokio::test]
    async fn html_in_place_of_a_constituent_file_fails_that_index_by_name() {
        let interstitial = complete().with(
            &format!("{HOST}/IndexConstituent/ind_one.csv"),
            "<!doctype html><title>Access Denied</title>",
        );
        let snap = crawl(
            &interstitial,
            HOST,
            day(),
            "groww",
            JoinKey::Isin,
            &master(),
        )
        .await;
        assert!(snap.resolutions.is_empty(), "every index used that file");
        assert_eq!(snap.failures.len(), 4);
        assert!(
            snap.failures[0].why.contains("indistinguishable"),
            "the refusal explains what zero rows would have meant: {:?}",
            snap.failures[0]
        );
    }

    #[tokio::test]
    async fn an_index_page_with_no_link_fails_rather_than_composing_a_filename() {
        let no_link = complete().with(
            &format!("{HOST}{}/one", Category::BroadBased.path()),
            "<a href=\"/somewhere/else\">not the file</a>",
        );
        let snap = crawl(&no_link, HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert_eq!(snap.failures.len(), 1);
        assert!(snap.failures[0].why.contains("ind_niftybanklist"));
    }

    #[tokio::test]
    async fn a_collapse_between_two_passes_halts_publication() {
        let full = crawl(&complete(), HOST, day(), "groww", JoinKey::Isin, &master()).await;
        // Every index down to one name: 8 published becomes 4, which is half.
        let halved = complete().with(
            &format!("{HOST}/IndexConstituent/ind_one.csv"),
            "Company Name,Industry,Symbol,Series,ISIN Code\n\
             A Ltd.,Fin,AAA,EQ,INE000A01001\n",
        );
        let shrunk = crawl(&halved, HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert_eq!(shrunk.published(), 4);
        let why = shrunk
            .admits_publication(Some(&full))
            .expect_err("half the universe is not a rebalance");
        assert!(why.contains("reports success"), "{why}");
        // And it publishes fine when there is no predecessor to collapse from.
        shrunk
            .admits_publication(None)
            .expect("a first pass has nothing to collapse against");
    }

    #[tokio::test]
    async fn the_wire_carries_the_day_the_feed_and_the_key_before_any_count() {
        let snap = crawl(&complete(), HOST, day(), "groww", JoinKey::Isin, &master()).await;
        let wire = snap.to_wire();
        let mut lines = wire.lines();
        assert_eq!(lines.next(), Some("day\t2026-08-14"));
        assert_eq!(lines.next(), Some("feed\tgroww"));
        assert_eq!(lines.next(), Some("key\tisin"));
        assert!(
            wire.contains("\tINE000A01001\tmatched\t1"),
            "a row carries its ISIN, its verdict and the id that reaches a \
             request: {wire}"
        );
    }

    /// The weaker key travels into the snapshot rather than being forgotten.
    #[tokio::test]
    async fn a_symbol_keyed_pass_records_that_its_join_proves_less() {
        let snap = crawl(
            &complete(),
            HOST,
            day(),
            "kite",
            JoinKey::TradingSymbol,
            &master(),
        )
        .await;
        assert!(!snap.key.is_identity());
        assert!(snap.to_wire().contains("key\tsymbol"));
        assert!(snap.is_sound());
    }

    // -- the real transport, over a loopback socket ------------------------
    //
    // Raw sockets and hand-written HTTP, the same device `crate::http`'s tests
    // use and for the same reason: this crate takes tokio WITHOUT the `net`
    // feature, and a test is not a reason to widen a dependency.

    /// A one-shot loopback server that answers `answer` and hangs up.
    fn listener(answer: &str) -> String {
        use std::io::{Read as _, Write as _};
        let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let addr = socket.local_addr().expect("an address");
        let answer = answer.to_owned();
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = socket.accept() else {
                return;
            };
            let mut buf = [0u8; 4096];
            let _read = stream.read(&mut buf).unwrap_or(0);
            let _wrote = stream.write_all(answer.as_bytes());
            let _flushed = stream.flush();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn the_real_transport_reads_a_body_it_is_given() {
        let body = csv();
        let url = listener(&format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ));
        let http = HttpDocuments::new().expect("a client builds");
        let got = http.get_async(&url).await.expect("a 200 with a body");
        assert_eq!(got, body);
        // And it flows through the trait the crawl actually calls.
        assert_eq!(
            DocumentSource::get(&http, &url).await.unwrap_or_default(),
            "",
            "the socket answered once; a second request reaches a closed port \
             and refuses rather than hanging"
        );
    }

    /// **A 3xx IS A ROUTE CHANGE AND IS NEVER CHASED.** Nothing here carries a
    /// credential — the reason is that parsing whatever the new location serves
    /// would file it as the index's membership.
    #[tokio::test]
    async fn the_real_transport_refuses_a_redirect_and_names_where_it_pointed() {
        let url = listener(
            "HTTP/1.1 302 Found\r\nLocation: http://elsewhere.invalid/other\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let http = HttpDocuments::new().expect("a client builds");
        let why = http
            .get_async(&url)
            .await
            .expect_err("a redirect is refused");
        assert!(why.contains("302"), "{why}");
        assert!(
            why.contains("elsewhere.invalid"),
            "it names the target: {why}"
        );
        assert!(why.contains("route change"), "{why}");
    }

    #[tokio::test]
    async fn the_real_transport_names_a_refusal_status_without_calling_it_a_redirect() {
        let url =
            listener("HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let http = HttpDocuments::new().expect("a client builds");
        let why = http
            .get_async(&url)
            .await
            .expect_err("403 is not a document");
        assert!(why.contains("403"), "{why}");
        assert!(
            !why.contains("It points at"),
            "a 403 has no Location and must not pretend to: {why}"
        );
    }

    /// A declared length past the bound refuses **before** the body is read.
    #[tokio::test]
    async fn a_declared_length_past_the_bound_refuses_before_the_body_is_read() {
        let url = listener(&format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            crate::nse::MAX_DOCUMENT_BYTES + 1
        ));
        let http = HttpDocuments::new().expect("a client builds");
        let why = http.get_async(&url).await.expect_err("past the bound");
        assert!(why.contains("declares"), "{why}");
        assert!(why.contains("before the body was read"), "{why}");
    }

    #[tokio::test]
    async fn a_socket_that_never_answers_refuses_rather_than_hanging() {
        // A port nothing is listening on: the connection is refused at once.
        let http = HttpDocuments::new().expect("a client builds");
        let why = http
            .get_async("http://127.0.0.1:1/never")
            .await
            .expect_err("nothing is listening");
        assert!(why.contains("was not reached"), "{why}");
    }

    #[tokio::test]
    async fn a_source_with_no_fixture_refuses_by_name_rather_than_answering_empty() {
        let empty = FakeDocuments::default();
        let snap = crawl(&empty, HOST, day(), "groww", JoinKey::Isin, &master()).await;
        assert!(snap.resolutions.is_empty());
        assert_eq!(snap.failures.len(), 4, "one per category");
        assert!(snap.failures[0].why.contains("no fixture"));
        assert!(snap.admits_publication(None).is_err());
    }
}

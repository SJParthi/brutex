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
//! authentication**. NSE publishes its index list the same way. Zerodha's dump
//! is behind the same token every bar request spends:
//!
//! ```text
//! curl "https://api.kite.trade/instruments" \
//!   -H "X-Kite-Version: 3" \
//!   -H "Authorization: token api_key:access_token"
//! ```
//!
//! So a refresh of "the masters" is not one operation. Three of the four cost
//! nothing and can run unattended; the fourth spends a shared credential and is
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

/// One instrument master, and what it costs to fetch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
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
    /// Whether fetching it spends the shared vendor credential.
    ///
    /// **The whole reason this field exists**: three of the four sources are
    /// public CDN files that cost nothing, and one is behind the same token
    /// every bar request spends. A caller that refreshes the set unattended
    /// must be able to tell them apart without reading a comment.
    pub needs_token: bool,
}

/// NSE's own index list — the reference every feed is checked against.
pub const NSE_INDICES_FILE: &str = "nse_indices.csv";

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
        vendor: Some(Vendor::Dhan),
        file: "dhan_scrip.csv",
        url: "https://images.dhan.co/api-data/api-scrip-master.csv",
        needs_token: false,
    },
    Source {
        vendor: Some(Vendor::Groww),
        file: "groww_instruments.csv",
        url: "https://growwapi-assets.groww.in/instruments/instrument.csv",
        needs_token: false,
    },
    Source {
        vendor: Some(Vendor::Zerodha),
        file: "zerodha_instruments.csv",
        url: "https://api.kite.trade/instruments",
        needs_token: true,
    },
    Source {
        vendor: None,
        file: NSE_INDICES_FILE,
        url: "https://www.nseindia.com/api/equity-master",
        needs_token: false,
    },
];

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
#[must_use]
pub fn free_sources() -> impl Iterator<Item = &'static Source> {
    SOURCES.iter().filter(|source| !source.needs_token)
}

/// The sources that spend it.
#[must_use]
pub fn token_sources() -> impl Iterator<Item = &'static Source> {
    SOURCES.iter().filter(|source| source.needs_token)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{
        Landed, MIN_BODY_BYTES, NSE_INDICES_FILE, SOURCES, Source, free_sources, land, path_of,
        token_sources,
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
        assert!(why.contains("no comma"), "{why}");
        assert!(!path_of(&dir, &SOURCES[0]).exists(), "nothing was written");
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
    fn two_sources_do_not_share_a_temporary_file() {
        // NAMED FOR THE TARGET. One shared `.partial` would let two concurrent
        // refreshes hand each other's bytes to the rename, and each master
        // would be internally valid and belong to the other feed.
        let dir = scratch("distinct-temporaries");
        let mut names = std::collections::BTreeSet::new();
        for source in SOURCES {
            assert!(
                names.insert(format!(".{}.partial", source.file)),
                "{} shares a temporary with another source",
                source.file
            );
        }
        assert_eq!(names.len(), SOURCES.len());
        let _unused = dir;
    }
}

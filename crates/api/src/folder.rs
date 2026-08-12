//! HOW FAR A FOLDER FEED REACHES, over the wire — read off the disk, on
//! demand, and never guessed.
//!
//! # The operator's rule, 12 Aug 2026, verbatim
//!
//! > "for truedata and gdfl alone, one and only, we will pull the data
//! > entirely from csv files from the precise folder — because we will buy
//! > those data from them as csv files and we will put that into the specified
//! > folder, only from there it should be read."
//! >
//! > "except these two alone only, for all other vendors or brokers feeds it
//! > should be always REST."
//!
//! [`pull::vendor::SourceKind`] is that rule as a type and
//! [`pull::folder`] is the half of it that touches a disk. This module is the
//! half that answers a browser, and it exists because **nothing did**.
//!
//! # Why this is its own route and not a field on `/feeds.json`
//!
//! `/feeds.json` renders on every page load, and its own body says an
//! `O(files)` probe there would be the cost `/store` exists to avoid. A
//! folder's reach cannot be answered without walking the folder —
//! [`pull::folder::read_reach`] is `O(members)` and says so — so putting it on
//! the per-render path would make every page load pay for a bulk import's
//! worth of `read_dir`.
//!
//! So `/feeds.json` carries only what is free: the KIND and the VERB, both
//! `const fn` on [`pull::vendor::SourceKind`]. The walk lives here, behind a
//! route the page calls once, when an operator actually chooses one of the two
//! folder vendors. `docs/06-limits.md` records the cost.
//!
//! # An empty folder is an ANSWER; a missing folder is a HALT
//!
//! This is the distinction the whole module is shaped around, and the one that
//! did not exist before it. A folder that is there and empty and a folder that
//! is not there at all both used to produce no bars and no reason, and an
//! operator could not tell "I have not bought that month yet" from "the path is
//! wrong" — so both got dressed as *no data yet*, which is the fallback
//! `CLAUDE.md` §4 bans.
//!
//! | | HTTP | body |
//! |---|---|---|
//! | folder read, no members | **200** | `"reach":{"state":"empty"}` — a real answer |
//! | folder read, blank members | **200** | `"state":"blank"` with the file count |
//! | folder read, rows found | **200** | `"state":"days"` with both ends |
//! | the question is wrong | **400** | unknown feed, or a REST feed, by name |
//! | this machine cannot answer | **409** | **and it names the PATH** |
//!
//! 409 rather than 404 for the halt: the folder is not a resource this server
//! owns and could create, it is a place the operator puts files. The request
//! was well formed and the machine's state is what refuses it, which is what
//! that code is for. Every 409 body carries `path`, because the path is the one
//! thing an operator needs next and the one thing a run that produced no bars
//! never used to say.

use std::fmt::Write as _;

use pull::csv::Columns;
use pull::vendor::{Feed, SourceKind, Transport};

use crate::{ingest, render, server};

/// `GET /folder.json?feed=<wire>[&segment=<INDEX|CASH|FNO>]`
///
/// # Errors
///
/// Never returns `Err`; every refusal is a status code and a JSON body naming
/// what refused and, when there is one, the path.
pub async fn folder_json(
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let json = "application/json; charset=utf-8";
    let head = [(axum::http::header::CONTENT_TYPE, json)];
    let query = uri.query().unwrap_or("");
    let asked = server::param(query, "feed");

    // AN UNKNOWN FEED REFUSES BY NAME rather than defaulting. `parse_feed`
    // answers Dhan for an EMPTY string, which is a broker and is refused one
    // step below as the wrong kind — so an absent parameter still lands on a
    // sentence that names a kind rather than on a folder invented for it.
    let Some(feed) = ingest::parse_feed(&asked) else {
        let known: Vec<&str> = Feed::ALL.into_iter().map(Feed::wire).collect();
        return (
            axum::http::StatusCode::BAD_REQUEST,
            head,
            refused(&format!(
                "this build reads no feed called that. The feeds it knows are {}.",
                known.join(", ")
            )),
        );
    };

    // A REST FEED HAS NO FOLDER, AND IS TOLD SO IN ITS OWN WORDS.
    //
    // Answered here rather than by handing `read_reach` a broker and relaying
    // its refusal, because the useful half of the sentence is what the feed IS
    // — the verb its bars actually arrive under — and that is free.
    let kind = feed.source_kind();
    if !matches!(kind, SourceKind::Folder) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            head,
            format!(
                r#"{{"feed":{},"kind":{},"verb":{},"refused":{}}}"#,
                render::json_string(feed.wire()),
                render::json_string(kind_wire(kind)),
                render::json_string(kind.verb()),
                render::json_string(&format!(
                    "{} is a {}, so it has no folder — its bars are {} over the network, \
                     and how far back it answers is on /feeds.json as a history floor.",
                    feed.display(),
                    kind.label(),
                    kind.verb_past()
                )),
            ),
        );
    }

    let shape = match shape_of(feed, &server::param(query, "segment")) {
        Ok(shape) => shape,
        Err(why) => return (axum::http::StatusCode::BAD_REQUEST, head, refused(&why)),
    };

    // THE ROOT IS RESOLVED EXACTLY AS THE STORE AND THE MASTERS ARE.
    //
    // `pull::folder::root` reads `BRUTEX_ARCHIVES`, else `$HOME/.brutex/…`,
    // in one place — the same two-step `server::store_dir_from` performs for
    // `BRUTEX_STORE`. No literal path appears here or there.
    let root = match pull::folder::root() {
        Ok(root) => root,
        // NO HOME AND NO VARIABLE IS A HALT WITH NO PATH TO NAME, which is the
        // one case that cannot carry `path` — so it carries the variables an
        // operator can set instead, which is the equivalent next step.
        Err(why) => {
            return (
                axum::http::StatusCode::CONFLICT,
                head,
                refused(&why.to_string()),
            );
        }
    };
    let (status, out) = answer(feed, shape, &root);
    (status, head, out)
}

/// Everything [`folder_json`] does once a root is known.
///
/// Split from it for the reason `pull::folder::root_from` is split from `root`
/// and `server::store_dir_from` from `store_dir`: every outcome has to be
/// testable, and a test cannot arrange the environment — `set_var` is `unsafe`
/// under edition 2024 and mutating process-wide state would race every other
/// test in the binary. Taking the root as an argument is what makes the empty
/// folder, the missing folder and the unreadable folder three assertions
/// rather than three sentences in a doc comment.
fn answer(feed: Feed, shape: Columns, root: &std::path::Path) -> (axum::http::StatusCode, String) {
    let kind = feed.source_kind();
    let dir = match pull::folder::folder_of(root, feed) {
        Ok(dir) => dir,
        Err(why) => {
            return (axum::http::StatusCode::CONFLICT, refused(&why.to_string()));
        }
    };
    let path = dir.display().to_string();

    match pull::folder::read_reach(&dir, feed, shape) {
        Ok(reach) => (axum::http::StatusCode::OK, body(feed, kind, &path, reach)),
        // MISSING, UNREADABLE, OR HOLDING SOMETHING THAT WILL NOT DECODE.
        // Loud, with the path, and never as an empty reach — `read_reach` has
        // already put the same sentence on the rolling log at `Error`.
        Err(why) => (
            axum::http::StatusCode::CONFLICT,
            format!(
                r#"{{"feed":{},"kind":{},"verb":{},"path":{},"refused":{}}}"#,
                render::json_string(feed.wire()),
                render::json_string(kind_wire(kind)),
                render::json_string(kind.verb()),
                render::json_string(&path),
                render::json_string(&why.to_string()),
            ),
        ),
    }
}

/// A refusal with no path to name.
fn refused(why: &str) -> String {
    format!(r#"{{"refused":{}}}"#, render::json_string(why))
}

/// The wire spelling of a [`SourceKind`], which is what the page keys on.
///
/// Lower case and stable. [`SourceKind::label`] is the human sentence and
/// [`SourceKind::verb`] is the word in a sentence; neither is a key, and using
/// one as a key would make a rewording a breaking change.
const fn kind_wire(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Rest => "rest",
        SourceKind::Folder => "folder",
    }
}

/// Which column shape to decode this feed's members with.
///
/// Taken from [`pull::vendor::ColumnLayout::shape`], so the decoder is handed
/// the shape the descriptor MEASURED rather than one chosen by a `match` here
/// — which is what `crates/api/src/server.rs` used to do, hardcoded to GDFL's
/// ten columns for every archive feed.
///
/// # Errors
///
/// A sentence naming the segments this feed has a measured layout for, when
/// the one asked for has none, or when the feed declares several and the
/// request named none. A feed with exactly one layout needs no parameter:
/// there is nothing to disambiguate.
fn shape_of(feed: Feed, asked: &str) -> Result<Columns, String> {
    let Transport::LocalArchive(spec) = feed.descriptor().transport else {
        // Unreachable through `folder_json`, which refuses a REST feed above.
        // `.expect()`-free and stated rather than `unreachable!()`, which
        // would be a region the coverage floor can never enter.
        return Err(format!("{} declares no archive", feed.display()));
    };
    let measured = || {
        spec.layouts
            .iter()
            .map(|l| l.segment.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if asked.is_empty() {
        return match spec.layouts {
            [only] => Ok(only.shape),
            // Zero layouts, or more than one and no way to choose. Both refuse
            // by name rather than taking the first: a reach read with the
            // wrong shape is a precise and wrong answer, which is the exact
            // failure `pull::folder` refuses a guessed root to avoid.
            _ => Err(format!(
                "{} has {} measured column layouts, so a segment has to be named. \
                 Measured: {}.",
                feed.display(),
                spec.layouts.len(),
                measured()
            )),
        };
    }
    let Ok(segment) = brutex_core::instrument::Segment::parse(asked) else {
        return Err(format!(
            "{asked:?} is not a segment. They are INDEX, CASH and FNO."
        ));
    };
    spec.layout(segment).map(|l| l.shape).ok_or_else(|| {
        format!(
            "no column layout was ever measured for {} {}, so its files cannot be \
             decoded and its reach cannot be read. Measured: {}.",
            feed.display(),
            segment.as_str(),
            measured()
        )
    })
}

/// A reach that was read, as JSON.
///
/// `state` is the discriminant and the two ends are `null` on the arms that
/// have none — never absent. A missing key and a null key read the same in
/// most browsers' code and mean different things here: `"state":"empty"` with
/// `"earliest":null` is a folder that answers for NOTHING, which is an answer,
/// and a caller that saw only a missing key would render it as an unknown
/// bound. That is the same distinction `pull::folder::Reach::window` makes.
fn body(feed: Feed, kind: SourceKind, path: &str, reach: pull::folder::Reach) -> String {
    let (state, earliest, latest) = match reach {
        pull::folder::Reach::Empty => ("empty", None, None),
        pull::folder::Reach::Blank { .. } => ("blank", None, None),
        pull::folder::Reach::Days {
            earliest, latest, ..
        } => ("days", Some(earliest.to_string()), Some(latest.to_string())),
    };
    let day = |d: Option<String>| d.map_or_else(|| "null".to_owned(), |d| render::json_string(&d));
    let mut out = String::new();
    let _ = write!(
        out,
        r#"{{"feed":{},"kind":{},"verb":{},"path":{},"reach":{{"state":{},"earliest":{},"latest":{},"files":{},"rows":{}}}}}"#,
        render::json_string(feed.wire()),
        render::json_string(kind_wire(kind)),
        render::json_string(kind.verb()),
        render::json_string(path),
        render::json_string(state),
        day(earliest),
        day(latest),
        reach.files(),
        reach.rows(),
    );
    out
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
    use super::{answer, body, kind_wire, shape_of};
    use pull::csv::Columns;
    use pull::vendor::{Feed, SourceKind};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Distinguishes two scratch trees taken in the same process.
    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// A temporary tree that removes itself. The same shape
    /// `pull::folder`'s own tests use, for the same reason.
    struct Scratch {
        root: std::path::PathBuf,
    }

    impl Scratch {
        fn new() -> Self {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let mut root = std::env::temp_dir();
            root.push(format!("brutex-api-folder-{}-{serial}", std::process::id()));
            std::fs::create_dir_all(&root).expect("a scratch root");
            Self { root }
        }

        /// The feed's own subfolder under the root, created, holding `members`.
        fn feed_dir(&self, feed: Feed, members: &[(&str, &str)]) -> &Self {
            let dir = self.root.join(feed.wire());
            std::fs::create_dir_all(&dir).expect("a feed folder");
            for (name, bodyy) in members {
                std::fs::write(dir.join(format!("{name}.csv")), bodyy).expect("a member");
            }
            self
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _best_effort = std::fs::remove_dir_all(&self.root);
        }
    }

    /// `TrueData`'s index shape: `YYYYMMDD,HH:MM:SS,price,volume,open_interest`.
    const TWO_DAYS: &str = "20221003,09:15:01,38445.65,0,0\n20221004,15:29:59,38600.00,0,0\n";

    /// THE ANSWERABLE RANGE IS THE FILES PRESENT — read, never declared, and
    /// never a history floor.
    #[test]
    fn the_range_is_read_off_the_files_and_carries_the_honest_verb() {
        let scratch = Scratch::new();
        scratch.feed_dir(Feed::TrueData, &[("NIFTY", TWO_DAYS)]);
        let (status, out) = answer(Feed::TrueData, Columns::TrueDataIndex, &scratch.root);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(out.contains(r#""state":"days""#), "{out}");
        assert!(out.contains(r#""earliest":"2022-10-03""#), "{out}");
        assert!(out.contains(r#""latest":"2022-10-04""#), "{out}");
        assert!(out.contains(r#""rows":2"#), "{out}");
        // THE VERB, AND IT IS NOT `pull`.
        assert!(out.contains(r#""verb":"read""#), "{out}");
        assert!(out.contains(r#""kind":"folder""#), "{out}");
        assert!(!out.contains("pull"), "a folder is never pulled: {out}");
        // AND THE PATH IS ON THE ANSWER, not only on the refusals.
        assert!(out.contains(Feed::TrueData.wire()), "{out}");
    }

    /// AN EMPTY FOLDER IS AN ANSWER, STATED WITH THE PATH — never "no data
    /// yet", and never a 404.
    #[test]
    fn an_empty_folder_answers_empty_with_the_path_and_never_a_halt() {
        let scratch = Scratch::new();
        scratch.feed_dir(Feed::TrueData, &[]);
        let (status, out) = answer(Feed::TrueData, Columns::TrueDataIndex, &scratch.root);
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "an empty folder is a real reach, not a failure: {out}"
        );
        assert!(out.contains(r#""state":"empty""#), "{out}");
        assert!(out.contains(r#""earliest":null"#), "{out}");
        assert!(out.contains(r#""files":0"#), "{out}");
        let dir = scratch.root.join(Feed::TrueData.wire());
        assert!(
            out.contains(&dir.display().to_string()),
            "the path is what an operator needs next: {out}"
        );
    }

    /// A FOLDER OF BLANK FILES IS NOT AN EMPTY FOLDER. Bought and delivered
    /// and blank is a different thing to do about than not bought at all.
    #[test]
    fn blank_members_are_told_apart_from_an_empty_folder() {
        let scratch = Scratch::new();
        scratch.feed_dir(Feed::TrueData, &[("NIFTY", "")]);
        let (status, out) = answer(Feed::TrueData, Columns::TrueDataIndex, &scratch.root);
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(out.contains(r#""state":"blank""#), "{out}");
        assert!(out.contains(r#""files":1"#), "{out}");
        assert!(out.contains(r#""rows":0"#), "{out}");
    }

    /// A MISSING FOLDER HALTS LOUDLY AND NAMES THE PATH. Never an empty reach
    /// wearing the same words as a folder that is really there and really
    /// empty — `CLAUDE.md` §4.
    #[test]
    fn a_missing_folder_halts_loudly_and_names_the_path() {
        let scratch = Scratch::new();
        // Nothing is created: the feed's subfolder does not exist.
        let (status, out) = answer(Feed::TrueData, Columns::TrueDataIndex, &scratch.root);
        assert_eq!(
            status,
            axum::http::StatusCode::CONFLICT,
            "a missing folder is a halt, not an answer: {out}"
        );
        assert!(out.contains(r#""refused""#), "{out}");
        let dir = scratch.root.join(Feed::TrueData.wire());
        assert!(
            out.contains(&dir.display().to_string()),
            "the halt must name the path: {out}"
        );
        assert!(
            !out.contains(r#""state""#),
            "a halt must not carry a reach: {out}"
        );
    }

    /// AN UNREADABLE FOLDER HALTS THE SAME WAY, and for the same reason: a
    /// permission that denies the walk is not an empty month.
    #[test]
    #[cfg(unix)]
    fn an_unreadable_folder_halts_loudly_and_names_the_path() {
        use std::os::unix::fs::PermissionsExt as _;
        let scratch = Scratch::new();
        scratch.feed_dir(Feed::TrueData, &[("NIFTY", TWO_DAYS)]);
        let dir = scratch.root.join(Feed::TrueData.wire());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000))
            .expect("a folder with no permissions");
        let (status, out) = answer(Feed::TrueData, Columns::TrueDataIndex, &scratch.root);
        // Restore before asserting, so a failure still cleans up.
        let _restored = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755));
        assert_eq!(status, axum::http::StatusCode::CONFLICT, "{out}");
        assert!(
            out.contains(&dir.display().to_string()),
            "the halt must name the path: {out}"
        );
    }

    /// A REST FEED HAS NO FOLDER AND IS REFUSED BY NAME — with its own verb,
    /// so the sentence cannot be read as "the folder is missing".
    #[test]
    fn a_rest_feed_is_refused_by_name_and_never_given_a_path() {
        let (status, out) = answer(
            Feed::Dhan,
            Columns::TrueDataIndex,
            std::path::Path::new("/x"),
        );
        assert_eq!(status, axum::http::StatusCode::CONFLICT);
        assert!(out.contains("REST"), "{out}");
        assert!(
            !out.contains("/x"),
            "a path invented for a broker would come back as a missing folder: {out}"
        );
    }

    /// THE TWO KINDS, AND THE WIRE WORD FOR EACH.
    #[test]
    fn the_wire_word_for_each_kind_is_stable_and_lower_case() {
        assert_eq!(kind_wire(SourceKind::Rest), "rest");
        assert_eq!(kind_wire(SourceKind::Folder), "folder");
        assert_eq!(SourceKind::Folder.verb(), "read");
        assert_eq!(SourceKind::Rest.verb(), "pull");
    }

    /// THE SHAPE COMES FROM THE DESCRIPTOR. A feed with exactly one measured
    /// layout needs no segment parameter; there is nothing to disambiguate.
    #[test]
    fn the_shape_is_the_descriptors_and_a_single_layout_needs_no_segment() {
        assert_eq!(shape_of(Feed::TrueData, ""), Ok(Columns::TrueDataIndex));
        assert_eq!(shape_of(Feed::Gdfl, ""), Ok(Columns::Gdfl));
        // And named explicitly, the same answer.
        assert_eq!(
            shape_of(Feed::TrueData, "INDEX"),
            Ok(Columns::TrueDataIndex)
        );
        assert_eq!(shape_of(Feed::Gdfl, "FNO"), Ok(Columns::Gdfl));
    }

    /// AN UNMEASURED SEGMENT REFUSES BY NAME rather than borrowing the other
    /// vendor's shape. GDFL publishes no index layout, and reading its files
    /// as five columns would take every field from the wrong offset.
    #[test]
    fn an_unmeasured_segment_refuses_by_name_and_lists_what_was_measured() {
        let why = shape_of(Feed::Gdfl, "INDEX").expect_err("no index layout was ever measured");
        assert!(why.contains("INDEX"), "{why}");
        assert!(why.contains("FNO"), "it must say what IS measured: {why}");
        let why = shape_of(Feed::TrueData, "FNO").expect_err("no FNO layout was ever measured");
        assert!(why.contains("INDEX"), "{why}");
    }

    /// A SEGMENT THAT IS NOT ONE refuses with the three that are.
    #[test]
    fn a_word_that_is_not_a_segment_refuses_with_the_three_that_are() {
        let why = shape_of(Feed::TrueData, "OPTIONS").expect_err("not a segment");
        assert!(
            why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
            "{why}"
        );
    }

    /// A REST FEED HAS NO ARCHIVE SPEC, so it has no shape to offer. Reached
    /// only by calling `shape_of` directly — `folder_json` refuses a broker one
    /// step earlier — and it still refuses rather than panicking.
    #[test]
    fn a_rest_feed_declares_no_archive_and_so_offers_no_shape() {
        let why = shape_of(Feed::Dhan, "").expect_err("a broker has no archive");
        assert!(why.contains("no archive"), "{why}");
    }

    /// EVERY ARM OF THE REACH RENDERS BOTH ENDS — `null` on the arms that have
    /// none, never absent. A missing key and a null key read the same in a
    /// browser and mean opposite things.
    #[test]
    fn every_reach_arm_renders_both_ends_rather_than_omitting_them() {
        for reach in [
            pull::folder::Reach::Empty,
            pull::folder::Reach::Blank { files: 2 },
        ] {
            let out = body(Feed::TrueData, SourceKind::Folder, "/p", reach);
            assert!(out.contains(r#""earliest":null"#), "{out}");
            assert!(out.contains(r#""latest":null"#), "{out}");
        }
    }
}

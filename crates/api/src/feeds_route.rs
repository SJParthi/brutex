//! `/feeds.json` -- every feed this build reads, its kind, readiness,
//! granularity floor and per-rung history floors.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The only edits are visibility
//! qualifiers (`pub(super)`) on items another module now has to reach.
//! No route answers differently because of the move.

use super::{Loaded, floor_oldest};
use crate::{ingest, render};
use std::fmt::Write as _;

/// One history claim as JSON object FIELDS — no braces, so the binding claim
/// can be flattened into its row and a contested one nested under a key.
///
/// # Four kinds, four different words
///
/// `rolling`, `fixed`, `none` and `unknown`. The last two are the pair a
/// caller must never collapse: `none` is the vendor stating it holds
/// everything, `unknown` is nobody having stated anything. Emitting `null` for
/// both and letting the reader guess is exactly the fallback `CLAUDE.md` §4
/// bans — so the day is `null` in both cases and the WORD carries the fact.
///
/// `oldest` is the resolved day and is `null` whenever there is none to
/// resolve, including when the clock could not be read: a `rolling` kind with
/// a null day says "it moves and this answer could not place it", which is
/// still not "no floor".
fn claim_fields(
    claim: Option<pull::vendor::FloorClaim>,
    today: Option<pull::session::Day>,
) -> String {
    use pull::vendor::HistoryFloor;

    // NO CLAIM IS A CLAIM OF NOTHING, not a claim of everything. An unrecorded
    // rung answers `unknown` with a null source.
    let (floor, source, standing) = match claim {
        Some(made) => (made.floor, Some(made.source), Some(made.standing)),
        None => (HistoryFloor::Unstated, None, None),
    };
    let (kind, unit, count) = match floor {
        HistoryFloor::Fixed { .. } => ("fixed", None, None),
        HistoryFloor::Rolling { years } => ("rolling", Some("y"), Some(u64::from(years))),
        HistoryFloor::RollingMonths { months } => ("rolling", Some("m"), Some(u64::from(months))),
        HistoryFloor::Unbounded => ("none", None, None),
        HistoryFloor::Unstated => ("unknown", None, None),
    };
    // The stated date of a fixed floor, rendered from the three numbers the
    // descriptor holds rather than through a calendar — `oldest` below is the
    // one that goes through the clamp, and it is the one a caller compares.
    let from = match floor {
        HistoryFloor::Fixed { year, month, day } => {
            render::json_string(&format!("{year:04}-{month:02}-{day:02}"))
        }
        _ => "null".to_owned(),
    };
    let oldest = match today.and_then(|day| floor_oldest(floor, day)) {
        Some(day) => render::json_string(&day.to_string()),
        None => "null".to_owned(),
    };
    let unit = match unit {
        Some(word) => render::json_string(word),
        None => "null".to_owned(),
    };
    let count = match count {
        Some(n) => n.to_string(),
        None => "null".to_owned(),
    };
    let source = match source {
        Some(text) => render::json_string(text),
        None => "null".to_owned(),
    };
    // WHAT KIND OF SOURCE SAID IT, as the wire's own word.
    //
    // Emitted beside `source` rather than parsed out of it. The page has to
    // say WHICH of two disagreeing claims binds — it drew that from a second,
    // hand-maintained table in the browser until D-0131 deleted it — and
    // deciding it by matching prose against the string "the operator" would be
    // the same second copy wearing a regular expression.
    //
    // `null` only where there is no claim at all, which `kind":"unknown"`
    // already says. A claim always has a standing; that is what makes the
    // binding rule checkable.
    let standing = match standing {
        Some(who) => render::json_string(who.word()),
        None => "null".to_owned(),
    };
    format!(
        r#""kind":{},"unit":{unit},"n":{count},"from":{from},"oldest":{oldest},"source":{source},"standing":{standing}"#,
        render::json_string(kind),
    )
}

/// The wire word for what one record at a feed's finest rung IS.
///
/// Its own function, and not an arm inlined into `finest_fields`, for one
/// reason: `FinestKind::Tick` is constructed by NO descriptor in this build and
/// the `const` block under `DESCRIPTORS` makes a row that claims one a build
/// failure. An arm nothing reaches is an uncovered region, and a region that
/// can never run is the thing `CLAUDE.md` §9's 100% floor has no way to
/// forgive. Lifted out, all three arms are reachable from a test that names
/// them — see `finest_kind_words_are_three_and_the_tick_word_is_one_of_them`.
///
/// The word is `snapshot` rather than `conflated_snapshot` because it is a KEY,
/// and `label` beside it carries the sentence.
const fn finest_kind_word(kind: pull::vendor::FinestKind) -> &'static str {
    match kind {
        pull::vendor::FinestKind::Tick => "tick",
        pull::vendor::FinestKind::ConflatedSnapshot => "snapshot",
        pull::vendor::FinestKind::Bar => "bar",
    }
}

/// One feed's GRANULARITY FLOOR, as a JSON object.
///
/// The finest rung this vendor can **ever** serve, what one record at that rung
/// is, why nothing finer exists in the vendor's own terms, and where those
/// words were read — `pull::vendor::GranularityFloor`, whole, with nothing
/// paraphrased on the way out.
///
/// # Why the whole thing crosses, rather than the number
///
/// Because the number is the half that cannot be trusted alone. Two of the four
/// feeds bottom out at one second and NEITHER serves a tick: what sits on that
/// second is a conflated snapshot of the best bid, the best ask and the best
/// last price, and every print between two snapshots was discarded before the
/// file was written and cannot be recovered by any reader. A page handed `1s`
/// and nothing else is free to write *tick* beside it — a claim about the data
/// that the data does not support, made in a label, which is exactly the
/// substitution `CLAUDE.md` §4 bans.
///
/// So `kind` travels as a word, and `tick_stream` and `conflated` travel beside
/// it as `FinestKind::is_tick_stream` and `FinestKind::is_conflated` — the two
/// questions that decide what may be printed next to a number, answered here by
/// the enum rather than by a reader matching on a string and getting it subtly
/// different. `because` and `source` travel verbatim: `CLAUDE.md` §3 rule 1
/// makes a vendor claim with no source indistinguishable from one somebody
/// typed, and a browser that received the claim without the citation would have
/// nothing to show an operator who asks how it is known.
///
/// One field read, three `const fn` calls and no allocation beyond the strings
/// the descriptor already holds. No filesystem call — this route renders on
/// every page load, which is why the folder's REACH is `/folder.json` and not
/// here.
fn finest_fields(feed: pull::vendor::Feed) -> String {
    let floor = feed.descriptor().granularity_floor;
    format!(
        r#"{{"rung":{},"kind":{},"label":{},"tick_stream":{},"conflated":{},"because":{},"source":{}}}"#,
        render::json_string(floor.finest.dir()),
        render::json_string(finest_kind_word(floor.kind)),
        render::json_string(floor.kind.label()),
        floor.kind.is_tick_stream(),
        floor.kind.is_conflated(),
        render::json_string(floor.because),
        render::json_string(floor.source),
    )
}

/// Every feed this build can read, for the browser's feed selector.
///
/// Built from `DESCRIPTORS`, so a fifth feed appears in the UI the day its row
/// exists and nothing in the front end names a vendor.
///
/// # `kind` — REST or a folder, and it is emitted ONCE
///
/// This route used to carry the same two-valued fact TWICE: a `transport` word
/// (`broker` / `archive`) matched off `Transport`'s two arms right here, and a
/// `kind` word (`rest` / `folder`) read from `pull::vendor::SourceKind`. Two
/// spellings of one split, produced by two independent `match`es in one
/// function, is the defect this endpoint exists to remove rather than an
/// example of it — and the browser had begun cross-deriving them, reading
/// `kind` with a fallback that guessed it from `transport`.
///
/// `transport` is GONE from the wire. `kind`, `kind_label` and `verb` are all
/// `SourceKind`'s, all `const fn`, and the enum is reached exactly once per feed
/// through `Feed::source_kind`. Everything that turns on the split — whether a
/// credential is required, whether a quota exists, whether a history floor is
/// the right question, and whether the honest verb is *pull* or *read* — is
/// asked of that one value.
///
/// # `finest` — how FINE this vendor can ever answer, and who says so
///
/// A second floor, and not the first one. `history` below answers *how far
/// back*; this answers *how fine*, and only one of the two refusals is
/// permanent. See `finest_fields`, which also says why the tick-versus-conflated
/// distinction crosses the wire as a field instead of being left to a reader.
///
/// # `history` — how far back each rung answers, and who says so
///
/// # `history` — how far back each rung answers, and who says so
///
/// The floor is a property of the feed AND of the rung: Groww's own interval
/// table gives its day row "Full history" and its one-minute row "Last 3
/// months". Until `pull::vendor::Descriptor::history` existed there was no
/// field to read, so the /ingest page held that table itself — a vendor fact
/// living in a browser, versioned separately from the vendor row it describes,
/// which is the copy that goes stale. It is emitted here so the page can stop
/// carrying literals.
///
/// One entry per rung the feed serves, PLUS any rung it records a floor for
/// without serving it — `served` says which, and a floor recorded for a
/// withdrawn rung is a fact worth keeping rather than a capability being
/// advertised.
pub(super) async fn feeds_json(
    axum::extract::State(_site): axum::extract::State<Loaded>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    let mut out = String::from("[");
    // ONE CLOCK READ FOR THE WHOLE ANSWER, so two feeds' rolling floors are
    // resolved against the same day. Reading it per row could straddle
    // midnight and emit two answers that disagree by a day for no reason a
    // reader could see. `None` when the clock is unusable, which every rolling
    // floor below then reports as a null day beside a `rolling` kind — never
    // as an absent floor.
    let today = ingest::ist_day(std::time::SystemTime::now()).ok();
    for (n, feed) in pull::vendor::Feed::ALL.into_iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        // WHERE THIS FEED'S BYTES COME FROM, READ ONCE.
        //
        // Every question below that used to `match` on `Transport`'s two arms
        // separately — the emitted word, and whether an empty store means "not
        // pulled yet" or "not bought" — is asked of this one value. Three
        // independent matches on one enum in one function is three chances to
        // disagree, and the third of them is what decided whether a feed was
        // offered at all.
        let kind = feed.source_kind();

        // THE HISTORY FLOORS, ONE ROW PER RUNG.
        //
        // Walked over the whole ladder rather than over the descriptor's
        // table, so a rung a feed SERVES with no recorded floor still appears
        // — as `unknown`, which is the honest answer and the one the page
        // renders as "claims nothing". A rung that is neither served nor
        // recorded is not emitted at all: there is nothing to say about it.
        let mut history = String::from("[");
        let mut rungs = 0usize;
        for rung in pull::vendor::Granularity::ALL {
            let row = feed.descriptor().history_row(rung);
            let served = feed.serves(rung);
            if !served && row.is_none() {
                continue;
            }
            if rungs > 0 {
                history.push(',');
            }
            rungs += 1;
            let contested = match row.and_then(|r| r.contested) {
                Some(other) => format!("{{{}}}", claim_fields(Some(other), today)),
                None => "null".to_owned(),
            };
            let _ = write!(
                history,
                r#"{{"rung":{},"served":{served},{},"binds_because":{},"contested":{contested}}}"#,
                render::json_string(rung.dir()),
                claim_fields(row.map(|r| r.binding), today),
                render::json_string(row.map_or("", |r| r.binds_because)),
            );
        }
        history.push(']');

        // CAN THIS FEED ACTUALLY SERVE?
        //
        // A feed listed but unusable is worse than one absent: it invites a
        // pull that cannot work and answers with a failure the operator has to
        // decode. TrueData and GDFL are the live case — samples were supplied,
        // the real data has not been bought, and offering them implies a
        // capability that does not exist.
        //
        // The signal is the CENSUS, which is already loaded and is a header
        // read rather than a directory walk — this is a per-render path and an
        // O(files) probe here would be the cost `/store` exists to avoid.
        //
        // NOT HIDDEN, DISABLED, WITH THE REASON. §4 forbids the silent version:
        // a feed that vanishes teaches the operator nothing, and one that says
        // "no data held, and no credential configured" tells them exactly which
        // of the two things to do.
        // THE CENSUS READ IS GONE, and its absence is the fix rather than a
        // tidy-up. It counted what had been INGESTED and was then used to
        // answer whether a feed could BE ingested — the circularity that made
        // both archive feeds unreachable. Readiness below asks the kind and the
        // store prefix; nothing here needs a counter.
        // A BROKER IS READY WITH AN EMPTY STORE. Its credential is what proves
        // entitlement and the first pull is what fills the store; an empty
        // census means "nothing pulled yet", not "not owned". Using the census
        // as the entitlement signal for a broker is circular — it could never
        // be selected to do the pull that would make it selectable.
        //
        // An ARCHIVE is different: there is no credential, so the only evidence
        // of ownership is data read from files the operator bought.
        //
        // ASKED AS "DOES A CREDENTIAL PROVE ENTITLEMENT HERE", which is
        // `SourceKind::needs_credential` and is the actual reason, rather than
        // as a second `match` on the transport that happens to land the same
        // way. The two were separate and could have drifted; the predicate is
        // now the same value `kind`, `kind_label` and `verb` are read from.
        // THE PREDICATE IS THE FEED'S KIND AND ITS STORE PREFIX — NOT ITS CENSUS.
        //
        // `a966a6f` moved archive readiness onto the folder and put the call in
        // the `Some(_)` arm of a match on `held`. That was DEAD CODE, and the
        // audit caught it: `held` is `Some` only when the census is
        // `Census::Held` (`census.rs` returns `None` for `Absent` and
        // `Unreadable`), so a never-ingested TrueData or GDFL produced
        // `held == None`, fell to the `None` arm, and was refused. The fix sat
        // behind the very condition it was written to remove and the
        // circularity survived it — manifest needs an ingest, an ingest needs
        // ready, ready needed the manifest.
        //
        // AND THE ARM IT FELL TO LIED. It said "<feed> has no store prefix",
        // while `Feed::store_vendor` returns `Some(Vendor::TrueData)` and
        // `Some(Vendor::Gdfl)`. The operator was told to buy the archive he had
        // already bought and to give the feed a prefix it already had — two
        // remedies that are both no-ops, so following the page could never
        // clear the state.
        //
        // `held` is therefore not consulted at all. It answered a question
        // about INGEST HISTORY and readiness is a question about ENTITLEMENT.
        let (ready, why) = if kind.needs_credential() {
            // A broker's credential is the entitlement, so it is ready with an
            // empty store: the first pull is what fills it.
            (true, String::new())
        } else if feed.store_vendor().is_none() {
            // The one case the old `None` arm was actually written for, and now
            // the only case that can reach this sentence.
            (
                false,
                format!(
                    "{} has no store prefix, so nothing it pulled could be filed",
                    feed.display()
                ),
            )
        } else {
            // An archive has no credential, so the evidence is the files the
            // operator bought — whatever the census has or has not recorded.
            archive_ready(feed)
        };

        // THE KIND, ITS LABEL AND THE VERB — ALL THREE FROM ONE VALUE, ALL
        // THREE FREE.
        //
        // `kind` is `pull::vendor::SourceKind`, the operator's rule of 12 Aug
        // 2026 as a type. `kind_label` is that type's own words, so a page and
        // a refusal cannot call the same feed two different things — the page
        // used to print `transport`, whose words (`broker`, `archive`) were
        // this file's and nothing else's. `verb` is the honest word for getting
        // data out of it: a folder is READ, never pulled. All three are
        // `const fn`, so none costs a filesystem call and all three can sit on
        // this per-render path.
        //
        // The REACH of a folder feed is NOT here: it cannot be answered
        // without walking the folder, and this route renders on every page
        // load. `/folder.json` answers it on demand — see `crate::folder`.
        // WHICH EXPIRED-F&O SHAPE THIS FEED ANSWERS IN, and it is not a
        // yes/no.
        //
        // Three states, because there are three: a feed with no derivative
        // history at all, one addressed by CONTRACT NAME through a discovery
        // walk, and one addressed by STRIKE OFFSET with no names to discover.
        // The two that serve need entirely different request builders, so a
        // page that knew only "serves F&O" would still send the wrong one.
        //
        // It is here because the browser had no way to ask. Measured on
        // 2026-08-19: the ingest page fired an expired-F&O request at every
        // ticked feed, and the ones at Dhan came back 502 -- correctly, since
        // Dhan publishes no name to discover -- after paying for a credential
        // read and a socket each. A page cannot decline what it cannot see.
        let fno = match feed.descriptor().transport {
            pull::vendor::Transport::Http(spec) => {
                if spec.fno.by_name().is_some() {
                    "by_name"
                } else if spec.fno.by_offset().is_some() {
                    "by_strike_offset"
                } else {
                    "none"
                }
            }
            // AN ARCHIVE HAS NO DISCOVERY ENDPOINT and never will: its expired
            // contracts are files on the operator's disk, not an answer to a
            // GET. Reported as its own word rather than folded into `none`,
            // because "this vendor does not serve it" and "this is not a
            // vendor" send an operator to different places.
            pull::vendor::Transport::LocalArchive(_) => "local_folder",
        };
        let _ = write!(
            out,
            r#"{{"wire":{},"display":{},"kind":{},"kind_label":{},"verb":{},"ready":{ready},"why":{},"fno":{},"finest":{finest},"history":{history}}}"#,
            render::json_string(feed.wire()),
            render::json_string(feed.display()),
            render::json_string(match kind {
                pull::vendor::SourceKind::Rest => "rest",
                pull::vendor::SourceKind::Folder => "folder",
            }),
            render::json_string(kind.label()),
            render::json_string(kind.verb()),
            render::json_string(&why),
            render::json_string(fno),
            finest = finest_fields(feed),
        );
    }
    out.push(']');
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        out,
    )
}

/// Whether an ARCHIVE feed has data the operator actually bought, and why not.
///
/// # Why the folder and not the census
///
/// A broker proves entitlement with a credential, so it is ready with an empty
/// store — `SourceKind::needs_credential` answers that and this function is
/// never reached for one. An archive has no credential, so the only evidence is
/// the files themselves. Asking the census instead made readiness circular: the
/// census fills from an ingest, the ingest needs readiness, so a freshly bought
/// archive could never be selected to do the pull that would make it
/// selectable.
///
/// # Three refusals, never one
///
/// `CLAUDE.md` §4 forbids collapsing distinct failures into one silence, and
/// these three are the operator's whole diagnostic path — they tell him whether
/// to set a path, fix a permission, or copy his files in:
///
/// * the root or the feed's folder cannot be resolved at all;
/// * the folder is not there yet;
/// * the folder is there and holds nothing.
///
/// # Cost
///
/// O(1). `read_dir` opens the directory and `next()` pulls ONE entry — the
/// count of files in it is never walked, so a folder holding 100,000 CSVs
/// answers in the same time as one holding a single file. That matters because
/// this runs on the `/feeds.json` render path, where the comment above the
/// readiness match already warns that an O(files) probe is the cost `/store`
/// exists to avoid.
///
/// UNVERIFIED as a measured figure. Nothing here was timed against a folder of
/// 100,000 CSVs, and the sentence above about one answering as fast as a folder
/// of one is what the STRUCTURE implies — one `read_dir` and one `next()`,
/// never a `count()` and never a `collect()` — not a reading anybody took.
/// `CLAUDE.md` §3 rule 6, and the same admission `docs/06-limits.md` §65 makes
/// about the walk this probe exists to avoid.
fn archive_ready(feed: pull::vendor::Feed) -> (bool, String) {
    let root = match pull::folder::root() {
        Ok(root) => root,
        Err(why) => {
            return (
                false,
                format!("the archive root could not be resolved: {why}"),
            );
        }
    };
    let dir = match pull::folder::folder_of(&root, feed) {
        Ok(dir) => dir,
        Err(why) => {
            return (
                false,
                format!(
                    "{}'s archive folder could not be named: {why}",
                    feed.display()
                ),
            );
        }
    };
    let shown = dir.display();
    match std::fs::read_dir(&dir) {
        // NOT THERE YET is not the same fact as EMPTY, and the operator acts
        // differently on each: one is a path to create or configure, the other
        // is files to copy.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => (
            false,
            format!("no folder at {shown} yet — the bought CSVs go there"),
        ),
        Err(why) => (false, format!("{shown} could not be read: {why}")),
        Ok(mut entries) => match entries.next() {
            Some(_) => (true, String::new()),
            None => (
                false,
                format!(
                    "{shown} is there and holds nothing — the bought CSVs have not been copied in"
                ),
            ),
        },
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use crate::server::Site;
    use crate::server::tests::{masters, store_root};
    use pull::session::Day;

    /// THE HISTORY FLOORS LEAVE THIS PROCESS, PER RUNG, WITH THEIR SOURCES.
    ///
    /// The /ingest page held this table in the browser because there was no
    /// field to read. Every claim below is now emitted, so a stale copy in a
    /// front end is a copy that can be deleted rather than a copy that has to
    /// be kept in step with `crates/pull` by hand.
    #[tokio::test]
    async fn feeds_json_carries_a_floor_per_rung_with_the_source_that_made_it() {
        let dir = masters("feeds-floors", None, None);
        let site = Loaded::new(Site::load(&dir, &store_root("feeds-floors")));
        let (_, body) = feeds_json(axum::extract::State(site)).await;

        // ONE VENDOR, TWO RUNGS, AND THE DISAGREEMENT IS PER RUNG — the whole
        // reason the field is keyed on the rung.
        //
        // What BINDS is the operator's fixed January 2020 at both rungs; he
        // stated it against the vendor and not against a rung, and D-0131
        // records that applying one figure to both is UNVERIFIED rather than
        // split on a guess.
        for rung in ["1min", "1day"] {
            assert!(
                body.contains(&format!(
                    r#"{{"rung":"{rung}","served":true,"kind":"fixed","unit":null,"n":null,"from":"2020-01-01""#
                )),
                "Groww's {rung} rung binds at the operator's fixed 2020: {body}"
            );
        }
        // What the VENDOR claims still differs between the two, and both are
        // carried as `contested` — deleting them would leave a number no
        // reader could argue with.
        assert!(
            body.contains(
                r#""contested":{"kind":"rolling","unit":"m","n":3,"from":null,"oldest":"#
            ),
            "the vendor's published quarter is kept beside the minute rung: {body}"
        );

        // WHO SAID IT, AS A WORD RATHER THAN A PROSE MATCH. The page decides
        // which of two claims binds from this field; before D-0131 it decided
        // it from a second table in the browser.
        assert!(
            body.contains(r#""standing":"operator""#)
                && body.contains(r#""standing":"vendor_doc""#),
            "both standings reach the wire: {body}"
        );
        // A rung nobody has said anything about has no standing to report, and
        // that is the one place the field is null.
        assert!(
            body.contains(r#""kind":"unknown","unit":null,"n":null,"from":null,"oldest":null,"source":null,"standing":null"#),
            "an unrecorded rung reports no standing rather than a default: {body}"
        );

        // A ROLLING FLOOR IS RESOLVED, NOT STORED. The day is computed from
        // today's clock through the same clamp a pull uses, so it is a
        // different string tomorrow — which is why the assertion is on its
        // shape and on the two facts around it rather than on a literal.
        let oldest = body
            .split(r#""kind":"rolling","unit":"y","n":5,"from":null,"oldest":""#)
            .nth(1)
            .expect("the five-year floor resolved to a day")
            .get(..10)
            .expect("a rendered day is ten characters")
            .to_owned();
        assert!(
            oldest.starts_with("20") && oldest.contains('-'),
            "a resolved day, not a stored one: {oldest}"
        );
        assert!(
            !body.contains(r#""oldest":"2021-08-11""#) || oldest != "2021-08-11",
            "nothing here pins a rolling floor to a literal"
        );

        // THE FOUR KINDS ARE FOUR WORDS, and the two that name no day are not
        // the same word. An archive claims NOTHING; a vendor that states it
        // holds everything says so.
        assert!(
            body.contains(r#"{"rung":"1s","served":true,"kind":"unknown","unit":null,"n":null,"from":null,"oldest":null,"source":null"#),
            "an archive's reach is unknown, not unlimited: {body}"
        );
        assert!(
            body.contains(r#""contested":{"kind":"none""#),
            "the vendor claim that lost is carried, by name: {body}"
        );

        // THE RUNG IS SERVED NOW, AND ITS FLOOR IS THE SAME FACT IT ALWAYS WAS.
        //
        // This asserted `"served":false` — Dhan's minute rung was withdrawn
        // until the intraday endpoint existed. It exists, so the flag flips and
        // the five-year floor beside it does not move: the floor was read from
        // the vendor's own intraday page ("for last 5 years") the whole time,
        // which is exactly why `served` and the floor are separate fields. What
        // this build ASKS FOR and what the vendor OFFERS are two facts, and a
        // rung going from withdrawn to served changes only the first.
        assert!(
            body.contains(r#"{"rung":"1min","served":true,"kind":"rolling","unit":"y","n":5"#),
            "the minute rung is served and carries the floor its own page states: {body}"
        );

        // Every entry carries every key, so a reader never has to tell an
        // absent field from a null one.
        for key in [
            r#""kind":"#,
            r#""unit":"#,
            r#""n":"#,
            r#""from":"#,
            r#""oldest":"#,
            r#""source":"#,
            r#""binds_because":"#,
            r#""contested":"#,
        ] {
            assert!(body.contains(key), "every entry carries {key}: {body}");
        }
        // THE SOURCE KIND IS ON THE WIRE ONCE. `transport` is gone: it was a
        // second spelling (`broker` / `archive`) of the split `kind` already
        // carries, produced by its own `match` in the same function, and the
        // browser had started guessing one from the other. `kind`, `kind_label`
        // and `verb` are all `SourceKind`'s, read from one value per feed.
        assert!(body.starts_with(
            r#"[{"wire":"dhan","display":"Dhan","kind":"rest","kind_label":"REST API","verb":"pull","ready":true,"why":""#
        ), "{body}");
        assert!(
            !body.contains(r#""transport":"#),
            "the second spelling of the source kind is gone, not merely unused: {body}"
        );
        // A BROKER IS PULLED; A FOLDER IS READ. The two archive vendors carry
        // the other word, and no row carries both.
        for (wire, kind, verb) in [
            ("truedata", "folder", "read"),
            ("gdfl", "folder", "read"),
            ("groww", "rest", "pull"),
        ] {
            let row = body
                .split("{\"wire\":")
                .find(|part| part.starts_with(&format!("\"{wire}\"")))
                .unwrap_or_else(|| panic!("a row for {wire}: {body}"));
            assert!(
                row.contains(&format!(r#""kind":"{kind}""#)),
                "{wire} is a {kind}: {row}"
            );
            assert!(
                row.contains(&format!(r#""verb":"{verb}""#)),
                "{wire} is {verb}: {row}"
            );
        }
        // AND THE LABEL IS THE TYPE'S OWN WORDS. The page prints this; it used
        // to print `transport`, whose vocabulary lived in this file alone.
        for (wire, label) in [("dhan", "REST API"), ("gdfl", "folder of files")] {
            let row = body
                .split("{\"wire\":")
                .find(|part| part.starts_with(&format!("\"{wire}\"")))
                .unwrap_or_else(|| panic!("a row for {wire}: {body}"));
            assert!(
                row.contains(&format!(r#""kind_label":"{label}""#)),
                "{wire} is labelled by SourceKind: {row}"
            );
        }
    }

    /// THE GRANULARITY FLOOR LEAVES THIS PROCESS WHOLE, AND A SNAPSHOT IS
    /// NEVER CALLED A TICK.
    ///
    /// The /ingest page held a transcription of all four `GranularityFloor`
    /// consts because there was no field to read. Every part of the fact is
    /// emitted here — the rung, what one record at it IS, the two booleans that
    /// decide what may be printed beside a number, and the vendor's own words
    /// with the place they were read — so the copy in the browser is one that
    /// can be deleted rather than one that has to be kept in step by hand.
    #[tokio::test]
    async fn feeds_json_carries_the_granularity_floor_and_never_calls_a_snapshot_a_tick() {
        let dir = masters("feeds-finest", None, None);
        let site = Loaded::new(Site::load(&dir, &store_root("feeds-finest")));
        let (_, body) = feeds_json(axum::extract::State(site)).await;

        // TWO SHAPES, FOUR ROWS. The brokers bottom out at a minute and one
        // record there is a BAR; the archives bottom out at a second and one
        // record there is a CONFLATED SNAPSHOT, which is a different object at
        // the same number.
        for (wire, rung, kind, conflated) in [
            ("dhan", "1min", "bar", false),
            ("groww", "1min", "bar", false),
            ("truedata", "1s", "snapshot", true),
            ("gdfl", "1s", "snapshot", true),
        ] {
            let row = body
                .split("{\"wire\":")
                .find(|part| part.starts_with(&format!("\"{wire}\"")))
                .unwrap_or_else(|| panic!("a row for {wire}: {body}"));
            assert!(
                row.contains(&format!(
                    r#""finest":{{"rung":"{rung}","kind":"{kind}","label":""#
                )),
                "{wire}'s floor is {rung} and one record is a {kind}: {row}"
            );
            assert!(
                row.contains(&format!(r#""tick_stream":false,"conflated":{conflated},"#)),
                "{wire}: the two booleans that decide what may be written beside \
                 the number: {row}"
            );
        }

        // NO ROW CLAIMS A TICK STREAM, and the wire says so in a field rather
        // than by omission. `FinestKind::Tick` is constructed by no descriptor
        // in this build; a reader must be able to see that stated.
        assert!(
            !body.contains(r#""tick_stream":true"#),
            "no feed in this build serves a tick stream: {body}"
        );

        // THE VENDOR'S WORDS TRAVEL VERBATIM, NOT PARAPHRASED. Compared against
        // the descriptor itself, so a reworded const is a failing test here
        // rather than a browser quietly showing yesterday's reason.
        for feed in pull::vendor::Feed::ALL {
            let floor = feed.descriptor().granularity_floor;
            assert!(
                body.contains(render::json_string(floor.because).trim_matches('"')),
                "{} carries its own reason: {body}",
                feed.wire()
            );
            assert!(
                body.contains(render::json_string(floor.source).trim_matches('"')),
                "{} carries where that was read — CLAUDE.md section 3 rule 1: {body}",
                feed.wire()
            );
        }
    }

    /// THREE KINDS, THREE WORDS, AND THE UNREACHED ONE IS NAMED HERE.
    ///
    /// `FinestKind::Tick` is constructed by no descriptor and the `const` block
    /// under `DESCRIPTORS` keeps it that way. Its arm is therefore unreachable
    /// through `feeds_json`, which is why the word lives in its own `const fn`
    /// — a region that can never run is a coverage hole `CLAUDE.md` section 9
    /// has no way to forgive, and this test is what runs it.
    #[test]
    fn finest_kind_words_are_three_and_the_tick_word_is_one_of_them() {
        assert_eq!(finest_kind_word(pull::vendor::FinestKind::Tick), "tick");
        assert_eq!(
            finest_kind_word(pull::vendor::FinestKind::ConflatedSnapshot),
            "snapshot",
            "and it is NOT the tick word — the whole point of the variant"
        );
        assert_eq!(finest_kind_word(pull::vendor::FinestKind::Bar), "bar");
    }

    /// A FLOOR THAT NAMES NO DAY RESOLVES TO NO DAY — both of them, and for
    /// two different reasons that the resolver is deliberately blind to.
    #[test]
    fn the_two_floors_that_name_no_day_resolve_to_none() {
        let today = Day::new(2026, 8, 12).expect("a real date");
        assert_eq!(
            floor_oldest(pull::vendor::HistoryFloor::Unbounded, today),
            None,
            "the vendor says it holds everything, so there is no oldest day"
        );
        assert_eq!(
            floor_oldest(pull::vendor::HistoryFloor::Unstated, today),
            None,
            "and nobody has said anything, which is not the same fact"
        );
        // A fixed floor resolves to itself, and a rolling one to a day before
        // today — through the clamp, so this is the day a pull would be given.
        assert_eq!(
            floor_oldest(
                pull::vendor::HistoryFloor::Fixed {
                    year: 2020,
                    month: 1,
                    day: 1
                },
                today
            ),
            Some(Day::new(2020, 1, 1).expect("a real date"))
        );
        assert_eq!(
            floor_oldest(
                pull::vendor::HistoryFloor::RollingMonths { months: 3 },
                today
            ),
            Some(Day::new(2026, 5, 12).expect("a real date")),
            "three calendar months, not ninety days"
        );
        // A CLAIM NOBODY MADE IS `unknown`, WITH A NULL SOURCE. This is the
        // arm a rung with no row takes, and it must not read as "no limit".
        let fields = claim_fields(None, Some(today));
        assert!(
            fields.contains(r#""kind":"unknown""#) && fields.contains(r#""source":null"#),
            "{fields}"
        );
        // And a floor whose date is not a date resolves to nothing rather than
        // to a day it invented. 30 February is the case.
        assert_eq!(
            floor_oldest(
                pull::vendor::HistoryFloor::Fixed {
                    year: 2020,
                    month: 2,
                    day: 30
                },
                today
            ),
            None
        );
    }
}

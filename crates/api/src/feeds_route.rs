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

use super::{Loaded, census_now, floor_oldest};
use crate::{census, ingest, render};
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
///
/// # What is offered, what is held, and on what basis (D-3320 to D-3338)
///
/// Every field below is read off `pull::vendor::Descriptor` or the census, and
/// this route spells only the wire words:
///
/// * `offered`, and for an excluded feed `excluded_by`, `excluded_on` and
///   `excluded_because` — the operator's allow-list (D-3338). An excluded feed
///   is also `ready: false`, so a picker that predates `offered` greys it.
/// * `rungs` (served) and `derived_rungs` (folded from the feed's minutes;
///   none for GDFL, D-2802).
/// * `archive` — the span the store holds from a bought archive, measured —
///   and `window` — the operator's own span, with the words a month outside it
///   carries.
/// * `basis` — per segment and rung: price, volume, `adjusted_as_of`, close,
///   whether the day bar is a fold, and whether open interest applies
///   (D-3322, D-3323, D-3324); `[]` with `basis_unverified` for a feed nobody
///   measured.
/// * `closing_auction` — what the store holds for a CAS-eligible share from
///   2026-08-03 (D-3320).
/// * `store` — the census's state and counters, from which a folder feed's
///   readiness is derived rather than from a folder probe.
///
/// O(1) in the store: one cached census read for the whole answer and a
/// walk bounded by the closed enums (five feeds, three segments, eleven
/// rungs). Measured on a shadow of the operator's store, 9 Oct 2026, 200
/// sequential GETs: p50 0.69 ms, p99 1.37 ms (the base build: 0.66 / 1.26).
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
pub(super) async fn feeds_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    let mut out = String::from("[");
    // THE STORE, READ ONCE FOR THE WHOLE ANSWER (D-3338). A folder feed is
    // ready when its census holds bars — the store is what /db reads, and a
    // folder probe answered a different question for a feed imported from
    // somewhere else (DB-051). A cache hit is a refcount bump; a miss is the
    // header read every census reader already pays. No directory is walked.
    let (censuses, _entries) = census_now(&site);
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

        let (ready, why) = readiness(feed, kind, census_row(&censuses, feed));

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
        // WHAT IS OFFERED, WHAT IS HELD, AND ON WHAT BASIS (D-3322, D-3323,
        // D-3324, D-3338). Every word below is read off the descriptor; this
        // route spells the wire words and invents no fact.
        let (served, derived) = rungs_of(feed);
        let _ = write!(
            out,
            r#"{{"wire":{},"display":{},"kind":{},"kind_label":{},"verb":{},"ready":{ready},"why":{},{offer},"rungs":{rungs},"derived_rungs":{derived_rungs},"archive":{archive},"window":{window},"basis":{basis},"basis_unverified":{unverified},"close_source":{close_source},"closing_auction":{auction},"store":{store},"fno":{},"finest":{finest},"history":{history}}}"#,
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
            offer = offer_fields(feed.descriptor().offer),
            rungs = words_array(served.iter().copied()),
            derived_rungs = words_array(derived.iter().map(|tf| tf.as_str())),
            archive = span_json(feed.descriptor().archive, None),
            window = span_json(feed.descriptor().window, Some(OUTSIDE_WINDOW)),
            basis = basis_json(feed, &served, &derived),
            unverified = match feed.descriptor().basis {
                pull::vendor::Basis::Unverified(why) => render::json_string(why),
                pull::vendor::Basis::Measured { .. } => "null".to_owned(),
            },
            close_source = match feed.descriptor().basis {
                pull::vendor::Basis::Measured { close_source, .. } => {
                    render::json_string(close_source)
                }
                pull::vendor::Basis::Unverified(_) => "null".to_owned(),
            },
            auction = auction_json(feed.descriptor().closing_auction),
            store = store_json(census_row(&censuses, feed)),
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

/// Whether a feed can serve the operator, and why not — `ready` and `why` on
/// `/feeds.json`.
///
/// `row` is the feed's census row from the one store read the route makes;
/// only a folder feed reads it (D-3338). O(1): a few field reads and no
/// filesystem call.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
fn readiness(
    feed: pull::vendor::Feed,
    kind: pull::vendor::SourceKind,
    row: Option<&census::VendorCensus>,
) -> (bool, String) {
    // CAN THIS FEED ACTUALLY SERVE THE OPERATOR?
    //
    // NOT HIDDEN, DISABLED, WITH THE REASON. §4 forbids the silent version: a
    // feed that vanishes teaches the operator nothing, and one that says why
    // it is not ready tells him which thing to do. Four answers, in order:
    //
    // * excluded by the operator — whatever its entitlement (D-3338);
    // * a broker — its credential proves entitlement, so it is ready with an
    //   empty store: the first pull is what fills it, and asking the census
    //   would make it unselectable for the very pull that fills it;
    // * a feed with no store prefix — nothing it read could be filed;
    // * a folder feed — ready when its STORE holds bars (`held_ready`).
    //
    // `SourceKind::needs_credential` is asked rather than a second `match` on
    // the transport, so this and `kind`, `kind_label` and `verb` are read from
    // one value and cannot drift.
    //
    // EXCLUDED BY THE OPERATOR, BEFORE ANYTHING ELSE (D-3338). Dhan and Groww
    // hold a credential and so read `ready: true`, which every picker that
    // predates `offered` took as "offer it" (DB-051, DB-G24). An excluded feed
    // is not ready for the operator, whatever its entitlement, and the reason
    // names who excluded it, when and where it is recorded.
    if let pull::vendor::Offer::Excluded { on, source, .. } = feed.descriptor().offer {
        return (
            false,
            format!(
                "{} is excluded by the operator since {on}: {source}",
                feed.display()
            ),
        );
    }
    if kind.needs_credential() {
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
        // AN ARCHIVE HAS NO CREDENTIAL, AND ITS EVIDENCE IS NOW THE STORE.
        //
        // This read the folder at `~/.brutex/vendor-data/<feed>` and called
        // GDFL "not ready — no folder … yet" while its store held about 3.1
        // billion bars, because `cli gdfl-import` reads the operator's own
        // data root (D-2804) and that folder was never part of it (DB-051,
        // D-3338). The circularity the folder probe was written to break —
        // a census that fills from an ingest which needed readiness — is
        // gone with it: nothing in this server ingests an archive on the
        // strength of this field, and `/folder.json` still answers a
        // folder's reach on demand.
        held_ready(feed, row)
    }
}

/// The label D-3338 gives a month outside the operator's window, so no page
/// spells it a second way.
const OUTSIDE_WINDOW: &str = "outside your window";

/// This feed's census row in one read of the store, or `None` where the feed
/// has no store prefix or the read yielded no row for it.
///
/// [`census::read_all`] yields one row per `Vendor::ALL` entry in that order,
/// so the row at the vendor's index is its own; the search behind it runs only
/// for a row out of that order and is bounded by the vendor count.
fn census_row(
    censuses: &[census::VendorCensus],
    feed: pull::vendor::Feed,
) -> Option<&census::VendorCensus> {
    let vendor = feed.store_vendor()?;
    censuses
        .get(vendor as usize)
        .filter(|row| row.vendor == vendor)
        .or_else(|| censuses.iter().find(|row| row.vendor == vendor))
}

/// Whether a FOLDER feed's store holds bars, and why not (D-3338).
///
/// # Three refusals, never one
///
/// `CLAUDE.md` §4 forbids collapsing distinct failures into one silence:
///
/// * no census row at all for the feed;
/// * a census that is absent or unreadable — its own note says which, where;
/// * a census that loaded and holds no bars.
///
/// # Cost
///
/// O(1): the census is already read by the caller, and this is two field
/// reads. No filesystem call.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
fn held_ready(feed: pull::vendor::Feed, row: Option<&census::VendorCensus>) -> (bool, String) {
    let Some(row) = row else {
        return (
            false,
            format!(
                "{} has no census row, so nothing says what its store holds",
                feed.display()
            ),
        );
    };
    match row.counters() {
        Some((_, bars, _)) if bars > 0 => (true, String::new()),
        Some(_) => (
            false,
            format!(
                "{}'s census at {} loaded and holds no bars yet: nothing has been imported for it",
                feed.display(),
                row.path.display()
            ),
        ),
        None => (false, row.note()),
    }
}

/// What the store holds for a feed, as one JSON object: the census's state,
/// its month files and bars, and its own note when it did not load.
///
/// The three counters are one field read each (`docs/04-invariants.md`
/// M-series); nothing is walked.
fn store_json(row: Option<&census::VendorCensus>) -> String {
    let Some(row) = row else {
        return r#"{"state":"no_prefix","month_files":null,"bars":null,"note":null}"#.to_owned();
    };
    let state = match row.state {
        census::Census::Held { .. } => "held",
        census::Census::Absent => "absent",
        census::Census::Unreadable { .. } => "unreadable",
    };
    match row.counters() {
        Some((month_files, bars, _)) => format!(
            r#"{{"state":"{state}","month_files":{month_files},"bars":{bars},"note":{}}}"#,
            match row.degraded() {
                Some(why) => render::json_string(&why),
                None => "null".to_owned(),
            }
        ),
        None => format!(
            r#"{{"state":"{state}","month_files":null,"bars":null,"note":{}}}"#,
            render::json_string(&row.note())
        ),
    }
}

/// `offered`, and who excluded a feed, when and why — as JSON object FIELDS.
///
/// The three exclusion fields are `null` on an offered feed and all present
/// on an excluded one, so a reader never tells an absent key from a null.
fn offer_fields(offer: pull::vendor::Offer) -> String {
    match offer {
        pull::vendor::Offer::Offered => {
            r#""offered":true,"excluded_by":null,"excluded_on":null,"excluded_because":null"#
                .to_owned()
        }
        pull::vendor::Offer::Excluded { by, on, source } => format!(
            r#""offered":false,"excluded_by":{},"excluded_on":{},"excluded_because":{}"#,
            render::json_string(by.word()),
            render::json_string(&on.to_string()),
            render::json_string(source),
        ),
    }
}

/// The wire word for who states a span.
const fn span_standing_word(standing: pull::vendor::SpanStanding) -> &'static str {
    match standing {
        pull::vendor::SpanStanding::Measured => "measured",
        pull::vendor::SpanStanding::Operator => "operator",
    }
}

/// A dated span as one JSON object, or `null`. `label_outside` is the words
/// a page prints on a month outside it, where the span has one.
fn span_json(span: Option<pull::vendor::Span>, label_outside: Option<&str>) -> String {
    let Some(span) = span else {
        return "null".to_owned();
    };
    let label = match label_outside {
        Some(words) => format!(r#","label_outside":{}"#, render::json_string(words)),
        None => String::new(),
    };
    format!(
        r#"{{"from":{},"to":{},"standing":{},"source":{}{label}}}"#,
        render::json_string(&span.from.to_string()),
        render::json_string(&span.to.to_string()),
        render::json_string(span_standing_word(span.standing)),
        render::json_string(span.source),
    )
}

/// A JSON array of strings.
fn words_array<'a>(words: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::from("[");
    for (n, word) in words.enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&render::json_string(word));
    }
    out.push(']');
    out
}

/// The rungs a feed's store holds: the ones it SERVES, as the descriptor
/// declares them, and the ones derived from its minutes.
///
/// # The derived set is `pull::ingest`'s rule, restated and pinned
///
/// `pull::ingest::derived_from` is private; its count is public. A rung is
/// derived from one-minute bars when it is longer than a minute, a whole
/// multiple of it, and not the day — D-0077's day is served, never derived —
/// and only for a feed `pull::ingest::derives_for` admits (never GDFL, D-2802).
/// `the_derived_rungs_are_the_ingest_rule` holds the count against
/// `pull::ingest::derived_count`, so the two cannot drift silently.
pub(super) fn rungs_of(
    feed: pull::vendor::Feed,
) -> (Vec<&'static str>, Vec<store::path::Timeframe>) {
    let served: Vec<&'static str> = pull::vendor::Granularity::ALL
        .into_iter()
        .filter(|rung| feed.serves(*rung))
        .map(pull::vendor::Granularity::dir)
        .collect();
    let folds_minutes = feed.store_vendor().is_some_and(pull::ingest::derives_for)
        && feed.serves(pull::vendor::Granularity::Minute1);
    let derived = if folds_minutes {
        let source = store::path::Timeframe::MINUTE_1.secs();
        store::path::Timeframe::KNOWN
            .iter()
            .copied()
            .filter(|target| {
                target.secs() > source
                    && target.secs().is_multiple_of(source)
                    && target.secs() != store::path::Timeframe::DAY_1.secs()
            })
            .collect()
    } else {
        Vec::new()
    };
    (served, derived)
}

/// The wire word for what a feed's store holds for the closing auction.
const fn auction_word(handling: pull::vendor::AuctionHandling) -> &'static str {
    match handling {
        pull::vendor::AuctionHandling::MinutesEndBeforeAuction => "minutes_end_before_auction",
        pull::vendor::AuctionHandling::PrintKeptApart => "print_kept_apart",
    }
}

/// A feed's closing-auction note as one JSON object, or `null` for a feed
/// nobody measured (D-3320). The day and the 15:15 are the descriptor's, which
/// holds them from the eligible share's session table; nothing here spells
/// either.
fn auction_json(note: Option<pull::vendor::AuctionNote>) -> String {
    let Some(note) = note else {
        return "null".to_owned();
    };
    let close = note.continuous_close_minute;
    format!(
        r#"{{"from":{},"eligible_continuous_close":"{:02}:{:02}","handling":{},"label":{},"source":{}}}"#,
        render::json_string(&note.from.to_string()),
        close / 60,
        close % 60,
        render::json_string(auction_word(note.handling)),
        render::json_string(note.handling.label()),
        render::json_string(note.source),
    )
}

/// The wire word for a price basis (D-3323).
pub(super) const fn price_word(price: pull::vendor::PriceBasis) -> &'static str {
    match price {
        pull::vendor::PriceBasis::Raw => "raw",
        pull::vendor::PriceBasis::AsPublished => "as_published",
        pull::vendor::PriceBasis::VendorAdjusted => "vendor_adjusted",
    }
}

/// The wire word for a volume basis (D-3323).
pub(super) const fn volume_word(volume: pull::vendor::VolumeBasis) -> &'static str {
    match volume {
        pull::vendor::VolumeBasis::Raw => "raw",
        pull::vendor::VolumeBasis::VendorOwn => "vendor_own",
        pull::vendor::VolumeBasis::NotTraded => "none",
    }
}

/// The wire word for what a close is (D-3324).
const fn close_word(close: pull::vendor::CloseSemantics) -> &'static str {
    match close {
        pull::vendor::CloseSemantics::Official => "official",
        pull::vendor::CloseSemantics::LastPrint => "last_print",
    }
}

/// Every (segment, rung) a feed holds, with its price, volume and close basis
/// and whether open interest applies — D-3323's `basis` array, with D-3324's
/// `close` on each row. `[]` for a feed nobody measured, whose
/// `basis_unverified` says so.
///
/// At most three segments × eleven rungs, so the answer is bounded by the
/// closed enums, never by the store.
fn basis_json(
    feed: pull::vendor::Feed,
    served: &[&'static str],
    derived: &[store::path::Timeframe],
) -> String {
    let basis = feed.descriptor().basis;
    let pull::vendor::Basis::Measured { segments, .. } = basis else {
        return "[]".to_owned();
    };
    let day = pull::vendor::Granularity::Day1.dir();
    let rungs = served
        .iter()
        .map(|rung| (*rung, false))
        .chain(derived.iter().map(|tf| (tf.as_str(), true)));
    let mut out = String::from("[");
    let mut n = 0usize;
    for (rung, is_derived) in rungs {
        let daily = rung == day;
        let close = basis.close_of(daily);
        let day_is_fold = match basis {
            pull::vendor::Basis::Measured { day: Some(day), .. } if daily => {
                if day.is_fold {
                    "true"
                } else {
                    "false"
                }
            }
            _ => "null",
        };
        for row in segments {
            if n > 0 {
                out.push(',');
            }
            n += 1;
            let _ = write!(
                out,
                r#"{{"segment":{},"rung":{},"derived":{is_derived},"price":{},"price_label":{},"volume":{},"volume_label":{},"adjusted_as_of":{},"close":{},"close_label":{},"day_is_fold":{day_is_fold},"oi":{},"source":{}}}"#,
                render::json_string(row.segment.as_str()),
                render::json_string(rung),
                render::json_string(price_word(row.price)),
                render::json_string(row.price.label()),
                render::json_string(volume_word(row.volume)),
                render::json_string(row.volume.label()),
                if row.price.adjusted_as_of_pull_date() {
                    r#""pull_date""#
                } else {
                    "null"
                },
                close.map_or_else(|| "null".to_owned(), |c| render::json_string(close_word(c))),
                close.map_or_else(|| "null".to_owned(), |c| render::json_string(c.label())),
                render::json_string(if pull::vendor::open_interest_applies(row.segment) {
                    "carried"
                } else {
                    "not_applicable"
                }),
                render::json_string(row.source),
            );
        }
    }
    out.push(']');
    out
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
        // DHAN IS A REST FEED AND IS NOT READY: the operator excluded it
        // (D-3338), and the reason is the first thing its row says after the
        // kind. It held `"ready":true` on its credential alone (DB-G24).
        assert!(body.starts_with(
            r#"[{"wire":"dhan","display":"Dhan","kind":"rest","kind_label":"REST API","verb":"pull","ready":false,"why":"Dhan is excluded by the operator since 2026-10-06: "#
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

    // -- what is offered, what is held, and on what basis --------------------

    /// The route's answer, parsed: one object per feed, in `Feed::ALL` order.
    async fn feeds_of(name: &str, root: &std::path::Path) -> Vec<serde_json::Value> {
        let dir = masters(name, None, None);
        let site = Loaded::new(Site::load(&dir, root));
        let (_, body) = feeds_json(axum::extract::State(site)).await;
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("feeds JSON");
        parsed.as_array().expect("an array of feeds").clone()
    }

    /// The row for `wire`.
    fn row<'a>(rows: &'a [serde_json::Value], wire: &str) -> &'a serde_json::Value {
        rows.iter()
            .find(|row| row["wire"] == wire)
            .unwrap_or_else(|| panic!("a row for {wire}: {rows:?}"))
    }

    /// A store whose GDFL census holds `entries` one-second NIFTY months of
    /// `rows` bars each; `None` writes nothing, so the census is absent.
    fn gdfl_store(name: &str, entries: Option<(u32, u64)>) -> std::path::PathBuf {
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        let root = store_root(name);
        let Some((months, rows)) = entries else {
            return root;
        };
        let mut manifest = Manifest::open(brutex_core::vendor::Vendor::Gdfl, &[], &[])
            .expect("a genesis manifest");
        for m in 0..months {
            manifest
                .record(Entry {
                    key: EntryKey {
                        contract: None,
                        exchange: brutex_core::instrument::Exchange::Nse,
                        segment: brutex_core::instrument::Segment::Index,
                        symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
                        timeframe: store::path::Timeframe::SECOND_1,
                        month: store::path::YearMonth::new(
                            2026,
                            1 + u8::try_from(m).expect("small"),
                        )
                        .expect("a month"),
                    },
                    rows,
                    first_ts_micros: 1_767_240_900_000_000,
                    last_ts_micros: 1_767_263_399_000_000,
                })
                .expect("records");
        }
        let path = manifest_path(&root, brutex_core::vendor::Vendor::Gdfl);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&path, manifest.image()).expect("writes");
        root
    }

    /// **ZERODHA AND GDFL ARE OFFERED; DHAN, GROWW AND TRUEDATA ARE EXCLUDED BY
    /// THE OPERATOR, NOT READY, AND SAY SO** (D-3338; DB-051, DB-G24).
    ///
    /// Before: dhan and groww read `ready: true` on their credential, and every
    /// picker offered all five.
    #[tokio::test]
    async fn only_zerodha_and_gdfl_are_offered_and_the_rest_are_excluded_by_the_operator() {
        let rows = feeds_of("feeds-offer", &gdfl_store("feeds-offer", None)).await;
        assert_eq!(rows.len(), pull::vendor::Feed::ALL.len());
        for wire in ["zerodha", "gdfl"] {
            let feed = row(&rows, wire);
            assert_eq!(feed["offered"], true, "{feed}");
            for key in ["excluded_by", "excluded_on", "excluded_because"] {
                assert!(
                    feed[key].is_null(),
                    "{wire}: {key} is null, not absent: {feed}"
                );
            }
        }
        assert_eq!(
            row(&rows, "zerodha")["ready"],
            true,
            "a broker with a credential"
        );
        for (wire, display) in [
            ("dhan", "Dhan"),
            ("groww", "Groww"),
            ("truedata", "TrueData"),
        ] {
            let feed = row(&rows, wire);
            assert_eq!(feed["offered"], false, "{feed}");
            assert_eq!(feed["excluded_by"], "operator", "{feed}");
            assert_eq!(feed["excluded_on"], "2026-10-06", "{feed}");
            assert!(
                feed["excluded_because"]
                    .as_str()
                    .is_some_and(|why| why.contains("D-3338")),
                "{feed}"
            );
            assert_eq!(
                feed["ready"], false,
                "{wire}: excluded is not ready: {feed}"
            );
            assert!(
                feed["why"]
                    .as_str()
                    .is_some_and(|why| why.starts_with(&format!(
                        "{display} is excluded by the operator since 2026-10-06"
                    ))),
                "{feed}"
            );
        }
        assert_eq!(
            rows.iter().filter(|feed| feed["offered"] == true).count(),
            2,
            "exactly two feeds are offered"
        );
    }

    /// **GDFL'S READINESS IS ITS STORE, NOT A FOLDER PROBE** (D-3338; DB-051,
    /// DB-G68).
    ///
    /// GDFL read "ingest not ready — no folder at ~/.brutex/vendor-data/gdfl
    /// yet" while its store held about 3.1 billion bars imported by `cli
    /// gdfl-import`. Every census state answers here, each in its own words.
    #[tokio::test]
    async fn gdfl_is_ready_when_its_store_holds_bars_and_says_which_census_state_it_is_not() {
        // HELD, WITH BARS: ready, and the counters cross the wire.
        let rows = feeds_of("feeds-held", &gdfl_store("feeds-held", Some((2, 22_500)))).await;
        let gdfl = row(&rows, "gdfl");
        assert_eq!(gdfl["ready"], true, "{gdfl}");
        assert_eq!(gdfl["why"], "", "{gdfl}");
        assert_eq!(gdfl["store"]["state"], "held", "{gdfl}");
        assert_eq!(gdfl["store"]["month_files"], 2, "{gdfl}");
        assert_eq!(gdfl["store"]["bars"], 45_000, "{gdfl}");
        assert!(
            gdfl["store"]["note"].is_null(),
            "a census that loaded whole: {gdfl}"
        );

        // HELD, EMPTY: a census that loaded and holds nothing is not ready.
        let rows = feeds_of("feeds-genesis", &gdfl_store("feeds-genesis", Some((0, 0)))).await;
        let gdfl = row(&rows, "gdfl");
        assert_eq!(gdfl["ready"], false, "{gdfl}");
        assert!(
            gdfl["why"]
                .as_str()
                .is_some_and(|why| why.contains("holds no bars yet")),
            "{gdfl}"
        );
        assert_eq!(gdfl["store"]["state"], "held");
        assert_eq!(gdfl["store"]["bars"], 0);

        // ABSENT: the census's own note, which names the path it looked at.
        let rows = feeds_of("feeds-absent", &gdfl_store("feeds-absent", None)).await;
        let gdfl = row(&rows, "gdfl");
        assert_eq!(gdfl["ready"], false, "{gdfl}");
        assert_eq!(gdfl["store"]["state"], "absent", "{gdfl}");
        assert!(
            gdfl["store"]["bars"].is_null(),
            "an absent census is not zero: {gdfl}"
        );
        let why = gdfl["why"].as_str().expect("a reason");
        assert!(
            !why.is_empty() && !why.contains("vendor-data"),
            "no folder probe: {why}"
        );
        assert_eq!(
            gdfl["store"]["note"], gdfl["why"],
            "one note, two places: {gdfl}"
        );

        // UNREADABLE: a manifest of zeros is refused, and the refusal is named.
        let root = store_root("feeds-unreadable");
        let path = pull::manifest::manifest_path(&root, brutex_core::vendor::Vendor::Gdfl);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&path, vec![0u8; 4096]).expect("writes");
        let rows = feeds_of("feeds-unreadable", &root).await;
        let gdfl = row(&rows, "gdfl");
        assert_eq!(gdfl["ready"], false, "{gdfl}");
        assert_eq!(gdfl["store"]["state"], "unreadable", "{gdfl}");
        assert!(
            gdfl["why"]
                .as_str()
                .is_some_and(|why| why.contains("UNREADABLE")),
            "{gdfl}"
        );
    }

    /// **GDFL IS ONE SECOND OVER ITS MEASURED ARCHIVE, 2018-09-03 TO
    /// 2026-09-24, RAW, AND ITS OPEN INTEREST APPLIES TO CONTRACTS ONLY**
    /// (D-2802, D-3322, D-3323, D-3324, D-3338; DB-051, DB-G24, DB-G68).
    #[tokio::test]
    async fn gdfl_is_one_second_over_its_measured_archive_and_raw() {
        let rows = feeds_of("feeds-basis", &gdfl_store("feeds-basis", None)).await;

        // GDFL: one second, over 2018-09-03..2026-09-24, measured.
        let gdfl = row(&rows, "gdfl");
        assert_eq!(gdfl["rungs"], serde_json::json!(["1s"]), "{gdfl}");
        assert_eq!(gdfl["derived_rungs"], serde_json::json!([]), "{gdfl}");
        assert_eq!(gdfl["archive"]["from"], "2018-09-03");
        assert_eq!(gdfl["archive"]["to"], "2026-09-24");
        assert_eq!(gdfl["archive"]["standing"], "measured");
        assert!(gdfl["window"].is_null());
        let history: Vec<&str> = gdfl["history"]
            .as_array()
            .expect("history")
            .iter()
            .filter_map(|rung| rung["rung"].as_str())
            .collect();
        assert_eq!(history, ["1s"], "no 1min or 1day row is advertised: {gdfl}");
        let basis = gdfl["basis"].as_array().expect("a basis array");
        assert_eq!(basis.len(), 3, "three segments at one rung: {gdfl}");
        for (segment, price, volume, oi) in [
            ("INDEX", "as_published", "none", "not_applicable"),
            ("CASH", "raw", "raw", "not_applicable"),
            ("FNO", "raw", "raw", "carried"),
        ] {
            let at = basis
                .iter()
                .find(|row| row["segment"] == segment)
                .unwrap_or_else(|| panic!("{segment}: {gdfl}"));
            assert_eq!(at["rung"], "1s");
            assert_eq!(at["derived"], false);
            assert_eq!(at["price"], price, "{at}");
            assert_eq!(at["volume"], volume, "{at}");
            assert!(at["adjusted_as_of"].is_null(), "{at}");
            assert_eq!(at["close"], "last_print", "{at}");
            assert_eq!(at["close_label"], "last print", "{at}");
            assert!(at["day_is_fold"].is_null(), "{at}");
            assert_eq!(at["oi"], oi, "{at}");
            assert!(
                at["source"].as_str().is_some_and(|s| s.contains("D-3323")),
                "{at}"
            );
        }
        assert!(gdfl["basis_unverified"].is_null());
        assert!(
            gdfl["close_source"]
                .as_str()
                .is_some_and(|s| s.contains("D-3324"))
        );
    }

    /// **ZERODHA SERVES 1MIN AND 1DAY AND THE RUNGS DERIVED FROM ITS MINUTES,
    /// OVER THE OPERATOR'S WINDOW, ITS SHARES BACK-ADJUSTED AS OF THE PULL
    /// DATE, ITS DAY CLOSE OFFICIAL** (D-3322, D-3323, D-3324, D-3338; DB-017,
    /// DB-033, DB-035, DB-036, DB-G24, DB-G43, DB-G58, DB-G75, DB-G100).
    #[tokio::test]
    async fn zerodha_says_its_window_its_adjusted_shares_and_its_official_day_close() {
        let rows = feeds_of("feeds-zerodha", &gdfl_store("feeds-zerodha", None)).await;
        // ZERODHA: 1min and 1day served, the derived rungs, the operator's
        // window, and its shares back-adjusted as of the pull date.
        let zerodha = row(&rows, "zerodha");
        assert_eq!(
            zerodha["rungs"],
            serde_json::json!(["1min", "1day"]),
            "{zerodha}"
        );
        assert!(zerodha["archive"].is_null());
        assert_eq!(zerodha["window"]["from"], "2018-01-01");
        assert_eq!(zerodha["window"]["to"], "2026-09-30");
        assert_eq!(zerodha["window"]["standing"], "operator");
        assert_eq!(zerodha["window"]["label_outside"], OUTSIDE_WINDOW);
        let basis = zerodha["basis"].as_array().expect("a basis array");
        let cell = |segment: &str, rung: &str| {
            basis
                .iter()
                .find(|row| row["segment"] == segment && row["rung"] == rung)
                .unwrap_or_else(|| panic!("{segment} {rung}: {zerodha}"))
        };
        let day = cell("CASH", "1day");
        assert_eq!(day["price"], "vendor_adjusted", "{day}");
        assert!(
            day["price_label"]
                .as_str()
                .is_some_and(|s| s.contains("pull date"))
        );
        assert_eq!(day["volume"], "vendor_own", "{day}");
        assert_eq!(day["adjusted_as_of"], "pull_date", "{day}");
        assert_eq!(day["close"], "official", "{day}");
        assert_eq!(day["close_label"], "official close", "{day}");
        assert_eq!(day["day_is_fold"], false, "{day}");
        assert_eq!(day["oi"], "not_applicable", "{day}");
        let minute = cell("CASH", "1min");
        assert_eq!(minute["close"], "last_print", "{minute}");
        assert!(minute["day_is_fold"].is_null(), "{minute}");
        assert_eq!(minute["adjusted_as_of"], "pull_date", "{minute}");
        let index = cell("INDEX", "1day");
        assert_eq!(
            (&index["price"], &index["volume"], &index["oi"]),
            (
                &serde_json::json!("as_published"),
                &serde_json::json!("none"),
                &serde_json::json!("not_applicable")
            ),
            "{index}"
        );
        assert!(index["adjusted_as_of"].is_null());
        assert!(
            basis.iter().all(|row| row["segment"] != "FNO"),
            "Zerodha holds no contract: {zerodha}"
        );
        let derived: Vec<&serde_json::Value> =
            basis.iter().filter(|row| row["derived"] == true).collect();
        assert!(
            !derived.is_empty(),
            "the derived rungs carry a basis too: {zerodha}"
        );
        assert!(
            derived.iter().all(|row| row["close"] == "last_print"),
            "a derived rung closes on its last print: {zerodha}"
        );
    }

    /// **EACH OFFERED FEED SAYS WHAT IT HOLDS FOR THE CLOSING AUCTION, AND THE
    /// THREE UNMEASURED FEEDS SAY THEY WERE NEVER MEASURED** (D-3320, D-3323;
    /// DB-034, DB-G42).
    #[tokio::test]
    async fn the_closing_auction_is_described_per_feed_and_an_unmeasured_basis_says_so() {
        let rows = feeds_of("feeds-auction", &gdfl_store("feeds-auction", None)).await;
        let (zerodha, gdfl) = (row(&rows, "zerodha"), row(&rows, "gdfl"));
        // THE CLOSING AUCTION, PER FEED (D-3320): from 2026-08-03, an eligible
        // share's continuous trading ends at 15:15, and each feed says what
        // its store holds after it.
        for (feed, handling) in [
            (zerodha, "minutes_end_before_auction"),
            (gdfl, "print_kept_apart"),
        ] {
            let auction = &feed["closing_auction"];
            assert_eq!(auction["from"], "2026-08-03", "{feed}");
            assert_eq!(auction["eligible_continuous_close"], "15:15", "{feed}");
            assert_eq!(auction["handling"], handling, "{feed}");
            assert!(auction["label"].as_str().is_some_and(|l| !l.is_empty()));
            assert!(
                auction["source"]
                    .as_str()
                    .is_some_and(|s| s.contains("D-3320"))
            );
        }
        assert!(
            zerodha["closing_auction"]["label"]
                .as_str()
                .is_some_and(|l| l.contains("15:14")),
            "{zerodha}"
        );

        // THE OTHER THREE: never measured, and the wire says so.
        for wire in ["dhan", "groww", "truedata"] {
            let feed = row(&rows, wire);
            assert!(feed["closing_auction"].is_null(), "{feed}");
            assert_eq!(feed["basis"], serde_json::json!([]), "{feed}");
            assert!(
                feed["basis_unverified"]
                    .as_str()
                    .is_some_and(|why| why.starts_with("UNVERIFIED")),
                "{feed}"
            );
            assert!(feed["close_source"].is_null(), "{feed}");
            assert!(
                feed["archive"].is_null() && feed["window"].is_null(),
                "{feed}"
            );
        }
    }

    /// **THE DERIVED RUNGS ARE THE INGEST RULE**: `rungs_of` restates
    /// `pull::ingest`'s private `derived_from`, so its count is held against
    /// the public `derived_count`, and GDFL derives nothing (D-2802).
    #[test]
    fn the_derived_rungs_are_the_ingest_rule() {
        let (served, derived) = rungs_of(pull::vendor::Feed::Zerodha);
        assert_eq!(served, ["1min", "1day"]);
        assert_eq!(
            derived.len(),
            pull::ingest::derived_count(store::path::Timeframe::MINUTE_1)
        );
        assert!(derived.iter().all(|tf| {
            tf.secs() > 60 && tf.secs() % 60 == 0 && *tf != store::path::Timeframe::DAY_1
        }));
        let (served, derived) = rungs_of(pull::vendor::Feed::Gdfl);
        assert_eq!(served, ["1s"]);
        assert!(derived.is_empty(), "GDFL is folded into nothing");
        // TrueData serves a minute and is not GDFL, so it would derive; its
        // basis is unmeasured, which `basis_json` answers with `[]`.
        let (_, derived) = rungs_of(pull::vendor::Feed::TrueData);
        assert!(!derived.is_empty());
        assert_eq!(
            basis_json(pull::vendor::Feed::TrueData, &[], &derived),
            "[]"
        );
    }

    /// The arms the route cannot reach with the shipped five feeds: a feed
    /// with no census row, a census row out of `Vendor::ALL` order, and a
    /// store answer with no prefix.
    #[test]
    fn a_missing_or_reordered_census_row_is_answered_in_words() {
        use brutex_core::vendor::Vendor;
        let absent = |vendor| census::VendorCensus {
            vendor,
            path: std::path::PathBuf::from("/NOWHERE"),
            state: census::Census::Absent,
        };
        // OUT OF ORDER: the indexed probe misses, and the bounded search finds it.
        let reordered = [absent(Vendor::Gdfl), absent(Vendor::Dhan)];
        assert_eq!(
            census_row(&reordered, pull::vendor::Feed::Dhan).map(|row| row.vendor),
            Some(Vendor::Dhan)
        );
        assert!(census_row(&reordered, pull::vendor::Feed::Zerodha).is_none());
        // NO ROW: not ready, and the words say nothing is known.
        let (ready, why) = held_ready(pull::vendor::Feed::Gdfl, None);
        assert!(!ready && why.contains("has no census row"), "{why}");
        assert_eq!(
            store_json(None),
            r#"{"state":"no_prefix","month_files":null,"bars":null,"note":null}"#
        );
        // A SPAN IS NULL WHERE NONE IS HELD.
        assert_eq!(span_json(None, Some(OUTSIDE_WINDOW)), "null");
        assert_eq!(words_array(std::iter::empty()), "[]");
        assert_eq!(words_array(["1s", "1min"].into_iter()), r#"["1s","1min"]"#);
    }
}

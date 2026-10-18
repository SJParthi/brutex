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
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    // THE OPERATOR'S SETTINGS, READ ONCE (R3-05): one `stat` when nothing
    // was saved since the last request, so a saved window is served on the
    // next request with no restart. ON THE BLOCKING POOL, NOT THIS WORKER
    // (repair 3): the read is file I/O, and an async handler that waits on
    // it holds a runtime worker every other route needs.
    // THE CENSUS READ AND THE HELD-SPAN WALK ON THE BLOCKING POOL TOO (R3-05
    // repair 4, the second agent's follow-up). A cold held-span walk is
    // O(census keys) — 0.66 to 2 s on the operator's store — and it ran
    // inline on a runtime worker, so the first `/feeds.json` after a start
    // stalled unrelated routes. A task that did not finish is a 503 that
    // says so, never an empty list.
    //
    // IN THE NAMED LIST POOL, BOTH READS IN ONE HOP (R3-08, NEW-04): Tokio's
    // own blocking pool has no name, no bound and no busy answer; the list
    // pool has all three, and is apart from every page read's pool.
    let root = site.store_root.clone();
    let answer = crate::detail::run_list(move || {
        let settings = crate::server::settings_route::snapshot(&root);
        feeds_body(&site, &settings)
    })
    .await;
    let (status, body) = match answer {
        Ok(body) => (axum::http::StatusCode::OK, body),
        Err(crate::detail::RunError::Saturated) => {
            // 429; the server adds `Retry-After` to every 429 (WP-81).
            crate::detail::admission_refused(
                "/feeds.json",
                crate::detail::MAX_LIST_CONCURRENT,
                &crate::detail::RunError::Saturated,
            )
        }
        Err(why) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            format!(
                r#"{{"refused":{}}}"#,
                render::json_string(&format!(
                    "the /feeds.json answer did not finish on the list pool: {why:?}"
                ))
            ),
        ),
    };
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        body,
    )
}

/// The body of `/feeds.json`: every feed's row, from one census read, one
/// clock read and one settings snapshot. Synchronous file reads and the
/// held-span walk: called on the blocking pool only.
fn feeds_body(site: &Loaded, settings: &crate::server::settings_route::Snapshot) -> String {
    let mut out = String::from("[");
    // THE STORE, READ ONCE FOR THE WHOLE ANSWER (D-3338). A folder feed is
    // ready when its census holds bars — the store is what /db reads, and a
    // folder probe answered a different question for a feed imported from
    // somewhere else (DB-051). A cache hit is a refcount bump; a miss is the
    // header read every census reader already pays. No directory is walked.
    let (censuses, _entries) = census_now(site);
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
            r#"{{"wire":{},"display":{},"kind":{},"kind_label":{},"verb":{},"ready":{ready},"why":{},{offer},"rungs":{rungs},"derived_rungs":{derived_rungs},"archive":{archive},"held":{held},"window":{window},"basis":{basis},"basis_unverified":{unverified},"close_source":{close_source},"closing_auction":{auction},"store":{store},"fno":{},"finest":{finest},"history":{history}}}"#,
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
            archive = archive_json(
                feed,
                census_row(&censuses, feed),
                &site.store_root,
                today.map(|day| i64::from(day.days_from_epoch())),
            ),
            held = held_json(census_row(&censuses, feed), kind),
            window = window_json(feed, settings),
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
    out
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

// -- what the store holds, measured from its census (R3-05) ------------------

/// The words a day past the held span carries: the store has not imported
/// it. Never "outside the vendor archive" and never a vendor hole — the
/// vendor may well have delivered it (SC-CLOSE-2, CMP-04).
pub(crate) const NOT_IMPORTED: &str = "not imported";

/// The words a day past a PULLED feed's held span carries: its store has
/// not pulled it (a broker feed is never imported).
pub(crate) const NOT_PULLED: &str = "not pulled";

/// A span of IST epoch days, both ends inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DaySpan {
    /// The first day.
    pub(crate) first: i64,
    /// The last day.
    pub(crate) last: i64,
}

impl DaySpan {
    /// The span one census entry covers: its first and last bar's IST days.
    const fn of_entry(entry: &pull::manifest::Entry) -> Self {
        Self {
            first: ist_epoch_day(entry.first_ts_micros),
            last: ist_epoch_day(entry.last_ts_micros),
        }
    }

    /// The smallest span holding both.
    const fn widen(self, other: Self) -> Self {
        Self {
            first: if other.first < self.first {
                other.first
            } else {
                self.first
            },
            last: if other.last > self.last {
                other.last
            } else {
                self.last
            },
        }
    }
}

/// The IST epoch day of a bar's timestamp.
const fn ist_epoch_day(ts_micros: i64) -> i64 {
    (ts_micros.div_euclid(1_000_000) + 19_800).div_euclid(86_400)
}

/// `epoch_day` as `YYYY-MM-DD`.
fn day_text(epoch_day: i64) -> String {
    u32::try_from(epoch_day)
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
        .map_or_else(|| epoch_day.to_string(), |day| day.to_string())
}

/// What one census part lists, per segment, at one generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartSpan {
    /// The generation it was measured at.
    generation: u64,
    /// The span per segment, in `pull::manifest::SEGMENTS` order.
    segments: [Option<DaySpan>; 3],
}

/// The slot of `segment` in [`PartSpan::segments`].
const fn segment_slot(segment: brutex_core::instrument::Segment) -> usize {
    match segment {
        brutex_core::instrument::Segment::Index => 0,
        brutex_core::instrument::Segment::Cash => 1,
        brutex_core::instrument::Segment::Fno => 2,
    }
}

impl PartSpan {
    const EMPTY: [Option<DaySpan>; 3] = [None; 3];

    /// Fold one key's newest entry in.
    fn fold(&mut self, part: &census::Part, key: &pull::manifest::EntryKey) {
        let Some(held) = part.held(key) else {
            return;
        };
        let one = DaySpan::of_entry(&held.entry);
        if let Some(slot) = self.segments.get_mut(segment_slot(key.segment)) {
            *slot = Some(slot.map_or(one, |span| span.widen(one)));
        }
    }

    /// Every key of `part`, walked: O(keys).
    fn walk(part: &census::Part) -> Self {
        let mut span = Self {
            generation: part.header().generation,
            segments: Self::EMPTY,
        };
        for key in part.held_keys() {
            span.fold(part, key);
        }
        span
    }
}

/// The held span of each census part file, and how many times that file's
/// keys were walked whole — kept so a test can prove a hit and an append walk
/// nothing.
type Memory = std::collections::HashMap<std::path::PathBuf, (PartSpan, u64)>;

/// The process's one [`Memory`].
fn memory() -> std::sync::MutexGuard<'static, Memory> {
    static MEMORY: std::sync::OnceLock<std::sync::Mutex<Memory>> = std::sync::OnceLock::new();
    MEMORY
        .get_or_init(|| std::sync::Mutex::new(Memory::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How many times the census part at `path` has been walked whole.
#[cfg(test)]
pub(crate) fn walks_of(path: &std::path::Path) -> u64 {
    memory().get(path).map_or(0, |(_, walks)| *walks)
}

/// The held span of one census part, remembered per part file and brought
/// up to date by folding in only the keys appended since.
///
/// # Cost
///
/// * the same generation as last time: one map probe, O(1);
/// * a newer generation the part can name its appended keys for
///   (`census::Part::changed_since`): one probe per appended key —
///   O(appended), the import's own work, never the history's;
/// * otherwise (the first request, or a part re-read whole): one walk of
///   its keys, O(keys), named for `docs/06-limits.md`. A part re-read whole
///   cost the census reader the same O(keys) on that request already.
///
/// A span only widens as entries are appended, because the store is
/// append-only (`CLAUDE.md` §3 rule 8): a month once held stays held at its
/// generation. A part re-read whole is walked whole.
///
/// Proof: `api::server::feeds_route::tests::held_span_growth_bench` (a hit
/// and a one-key append at 1x, 10x and 100x keys, the walk counted).
fn part_span(part: &census::Part) -> PartSpan {
    let generation = part.header().generation;
    // A PART WITH NO FILE (built in memory) has no identity to remember.
    if part.path().as_os_str().is_empty() {
        return PartSpan::walk(part);
    }
    let mut memory = memory();
    let kept = memory.get(part.path()).copied();
    let kept = match kept {
        Some((span, _)) if span.generation == generation => return span,
        Some((span, walks)) if span.generation < generation => {
            if let Some(since) = part.changed_since(span.generation) {
                let mut span = span;
                span.generation = generation;
                for (_, key) in since.iter() {
                    span.fold(part, key);
                }
                (span, walks)
            } else {
                (PartSpan::walk(part), walks.saturating_add(1))
            }
        }
        Some((_, walks)) => (PartSpan::walk(part), walks.saturating_add(1)),
        None => (PartSpan::walk(part), 1),
    };
    memory.insert(part.path().to_path_buf(), kept);
    kept.0
}

/// What a feed's store holds: per segment, and over all of them; `Err` is
/// why nothing can be said.
pub(crate) fn held_spans(
    row: Option<&census::VendorCensus>,
) -> Result<(u64, [Option<DaySpan>; 3]), String> {
    let row = row.ok_or("no census row for this feed")?;
    let manifest = match &row.state {
        census::Census::Held { manifest } => manifest,
        census::Census::Absent | census::Census::Unreadable { .. } => return Err(row.note()),
    };
    let mut segments = PartSpan::EMPTY;
    for part in manifest.parts() {
        let span = part_span(part);
        for (into, from) in segments.iter_mut().zip(span.segments) {
            *into = match (*into, from) {
                (Some(a), Some(b)) => Some(a.widen(b)),
                (a, b) => a.or(b),
            };
        }
    }
    Ok((manifest.generation(), segments))
}

/// What the GDFL import journal says about its reach, read once per journal
/// change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct JournalReach {
    /// Per kind, in `ImportKind::ALL` order: the furthest day whose import
    /// CLOSED — `done` (the journal reader reads a `recounted` line as
    /// `DayOutcome::Done`), `incomplete` or `refused`. A day begun
    /// and never closed is not reached: its import did not finish, so its
    /// minutes are not imported rather than lost.
    pub(crate) closed: [Option<pull::session::Day>; 3],
    /// The first and last day any import WROTE bars on ([`wrote`]: `done`,
    /// which includes a `recounted` line, done with refusals, or
    /// `incomplete`), whatever its kind: the files of
    /// those days were delivered, because an import read them.
    pub(crate) written: Option<(pull::session::Day, pull::session::Day)>,
}

/// `kind`'s place in `pull::gdfl_import::ImportKind::ALL`, by an exhaustive
/// match rather than a search of the table (gate 11 rule 6): a new kind
/// fails to compile here, and `every_import_kind_has_its_own_slot` holds the
/// match equal to the table's order.
const fn kind_slot(kind: pull::gdfl_import::ImportKind) -> usize {
    use pull::gdfl_import::ImportKind;
    match kind {
        ImportKind::Indices => 0,
        ImportKind::Stocks => 1,
        ImportKind::Options => 2,
    }
}

/// Whether a closing line about a day says the import wrote bars from its
/// files.
const fn wrote(outcome: crate::server::import_journal::DayOutcome) -> bool {
    use crate::server::import_journal::DayOutcome;
    matches!(
        outcome,
        DayOutcome::Done | DayOutcome::DoneWithRefusals | DayOutcome::Incomplete
    )
}

/// [`JournalReach`] of the journal at `root`, remembered per store against
/// the journal's length and line count.
///
/// NOT `Parsed::last`: that is the kind's NEWEST LINE, and a re-run of an
/// old day makes it go backwards — on the operator's journal the newest
/// indices line names 2025-10-21 while indices are journaled through
/// 2026-09-24, which would have called eleven months of held days "not
/// imported".
///
/// Cost: while the journal does not grow, one map probe. After it grows,
/// one walk of its rows newest first until every kind's closed day and the
/// last written day are found, and one oldest first until the first written
/// day is: O(rows after the furthest day of the kind that lags most) —
/// three rows when the kinds stand on one day, as the operator's do; at most
/// the journal's rows (12,196 lines on 13 Oct 2026), once per journal
/// change. Named for `docs/06-limits.md` (L-R305-3).
pub(crate) fn journal_reach(
    root: &std::path::Path,
    snapshot: &crate::server::import_journal::Snapshot,
) -> JournalReach {
    type Kept = std::collections::HashMap<std::path::PathBuf, ((u64, u64), JournalReach)>;
    static KEPT: std::sync::OnceLock<std::sync::Mutex<Kept>> = std::sync::OnceLock::new();
    let parsed = &snapshot.parsed;
    // KEYED BY LENGTH AND LINES, not lines alone: a journal replaced by one
    // of the same line count is a different journal.
    let stamp = (snapshot.bytes, parsed.lines);
    let mut kept = KEPT
        .get_or_init(|| std::sync::Mutex::new(Kept::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((at, reach)) = kept.get(root)
        && *at == stamp
    {
        return *reach;
    }
    let mut reach = JournalReach::default();
    if let Some((first, last)) = parsed.span() {
        let mut last_written = None;
        for row in parsed.range(first, last).rev() {
            if row.outcome == crate::server::import_journal::DayOutcome::Open {
                continue;
            }
            if last_written.is_none() && wrote(row.outcome) {
                last_written = Some(row.day);
            }
            if let Some(slot) = reach.closed.get_mut(kind_slot(row.kind))
                && slot.is_none()
            {
                *slot = Some(row.day);
            }
            if last_written.is_some() && reach.closed.iter().all(Option::is_some) {
                break;
            }
        }
        if let Some(last_written) = last_written {
            let first_written = parsed
                .range(first, last)
                .find(|row| wrote(row.outcome))
                .map_or(last_written, |row| row.day);
            reach.written = Some((first_written, last_written));
        }
    }
    kept.insert(root.to_path_buf(), (stamp, reach));
    reach
}

/// How far a feed's IMPORT has reached for one segment, and the days inside
/// that reach whose import began and never closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportReach {
    /// The furthest day whose import of the segment's kind closed, as IST
    /// epoch days; `None` when nothing can be said (see `from`).
    pub(crate) to: Option<i64>,
    /// Where `to` came from, or why there is none.
    pub(crate) from: String,
    /// Days of the asked range, at or before `to`, whose whole-tree import
    /// (`*`) was begun and never closed and was never closed before: their
    /// bars were not written, so their minutes are not imported, not lost.
    /// Ascending.
    pub(crate) open: Vec<i64>,
    /// Today, IST epoch days, read once when the reach was built; `None`
    /// when the clock could not be read (then no day is `not_yet`).
    pub(crate) today: Option<i64>,
}

impl ImportReach {
    /// Whether `day` (IST epoch days) is after today: a session that has
    /// not happened yet owes nothing, so a minute the calendar names there
    /// is neither lost nor "not imported" (R3-05 repair 3, the second
    /// agent's follow-up: `/gaps.json` for 2026-10 counted 16..31 October as
    /// 7,500 not-imported minutes on 15 October). Every feed, pulled or
    /// imported. O(1): one comparison with the day read once
    /// when the reach was built. UNVERIFIED: no test times it (GF-1, gate 12;
    /// `docs/06-limits.md` L-GF1-33).
    pub(crate) fn not_yet(&self, day: i64) -> bool {
        self.today.is_some_and(|today| day > today)
    }

    /// Whether `day` (IST epoch days) has not been imported: on or before
    /// today, and past the reach or an import of it that never closed.
    /// O(log open).
    pub(crate) fn not_imported(&self, day: i64) -> bool {
        !self.not_yet(day)
            && (self.to.is_some_and(|last| day > last) || self.open.binary_search(&day).is_ok())
    }
}

/// How far a feed's IMPORT has reached for `segment` (`INDEX`, `CASH` or
/// `FNO`): the furthest day whose import of the segment's kind (indices,
/// stocks, options) CLOSED in the GDFL import journal, read through the
/// journal reader `/imports.json` and `/dayflags.json` use (R3-05, CMP-04),
/// and the days of `days` (IST epoch days, both ends inclusive) before it
/// whose import began and never closed. `from` says where the answer came
/// from, or why there is none.
///
/// `to: None` — and so nothing is renamed `not imported` — for a feed that
/// keeps no import journal (a pulled feed: Zerodha's months are pulled, not
/// imported), a segment the journal has no closed line for, or a journal
/// that cannot be read. The census's held span is NOT used here: a series
/// whose own bars stop early, inside the import's reach, is the vendor's to
/// explain, and only the journal knows the reach.
///
/// Cost: one `metadata` call and the journal's new bytes
/// (`crate::server::import_journal::snapshot`), then [`journal_reach`]: O(1)
/// while the journal does not move; plus O(log n + rows in `days`) for the
/// open days — a month's rows, at most 31 days of three kinds.
/// UNVERIFIED: no test times the reach at two journal sizes (GF-1, gate 12;
/// `docs/06-limits.md` L-GF1-33); the bound is L-R305-3's.
pub(crate) fn imported_to(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    segment: &str,
    days: (i64, i64),
) -> ImportReach {
    let today = ingest::today_ist()
        .ok()
        .map(|day| i64::from(day.days_from_epoch()));
    let none = |from: String| ImportReach {
        to: None,
        from,
        open: Vec::new(),
        today,
    };
    if vendor != brutex_core::vendor::Vendor::Gdfl {
        return none(format!(
            "{vendor:?} keeps no import journal: its months are pulled, not imported"
        ));
    }
    let Ok(segment) = brutex_core::instrument::Segment::parse(segment) else {
        return none(format!("{segment:?} is not a segment"));
    };
    let slot = segment_slot(segment);
    let snapshot = match crate::server::import_journal::snapshot(root) {
        Ok(snapshot) => snapshot,
        Err(why) => return none(format!("the GDFL import journal cannot be read: {why}")),
    };
    let reach = journal_reach(root, &snapshot);
    let (Some(kind), Some(day)) = (
        pull::gdfl_import::ImportKind::ALL.get(slot),
        reach.closed.get(slot).copied().flatten(),
    ) else {
        return none(
            "the GDFL import journal names no closed day for this segment's kind".to_owned(),
        );
    };
    let civil = |epoch: i64| {
        u32::try_from(epoch)
            .ok()
            .and_then(|days| pull::session::Day::from_days(days).ok())
    };
    let mut from = format!(
        "the GDFL import journal: the furthest day whose {} import closed",
        kind.as_str()
    );
    // A CLOSED DAY AFTER TODAY IS NOT EVIDENCE OF AN IMPORT (R3-05 repair 4,
    // the second agent's follow-up 3), as `archive_json` already rules for
    // the archive's span. Before: a `done` line for 2099-01-01 made the
    // reach 2099, so every unimported day up to today was a lost vendor
    // hole. The reach is the furthest closed day of the kind AT OR BEFORE
    // today, and the passed-over line is named in `from`. Only a journal
    // naming a future day walks: O(rows after that closed day), once per
    // request (`docs/06-limits.md` L-R305-3).
    let day = match today {
        Some(now) if i64::from(day.days_from_epoch()) > now => {
            let bounded = civil(now).and_then(|now| {
                let (first, _) = snapshot.parsed.span()?;
                snapshot
                    .parsed
                    .range(first, now)
                    .rev()
                    .find(|row| {
                        row.kind == *kind
                            && row.outcome != crate::server::import_journal::DayOutcome::Open
                    })
                    .map(|row| row.day)
            });
            let Some(bounded) = bounded else {
                return none(format!(
                    "the GDFL import journal's furthest closed {} day, {day}, is after today \
                     and is not evidence of an import, and it names no closed day at or \
                     before today",
                    kind.as_str()
                ));
            };
            from = format!(
                "{from} at or before today; a closed line for {day}, after today, is not \
                 evidence of an import and was passed over"
            );
            bounded
        }
        _ => day,
    };
    let to = i64::from(day.days_from_epoch());
    let mut open = Vec::new();
    // THE OPEN-DAY SCAN COVERS ONLY THE ASKED DAYS AT OR BEFORE THE REACH. A
    // month that begins past the reach has none to scan, and is never handed
    // to the journal as a reversed span (R3-05 repair 4: `month=2026-11` on
    // 15 October 2026 asked for `1 Nov ..= 30 Sep` and, under
    // `panic = "abort"`, took the API down; `Parsed::range` now also answers
    // a reversed span with nothing).
    let last = days.1.min(to);
    if days.0 <= last
        && let (Some(first), Some(last)) = (civil(days.0), civil(last))
    {
        for row in snapshot.parsed.range(first, last) {
            if row.kind == *kind
                && row.filter == "*"
                && row.outcome == crate::server::import_journal::DayOutcome::Open
                && !row.previous.is_some_and(wrote)
            {
                open.push(i64::from(row.day.days_from_epoch()));
            }
        }
    }
    open.dedup();
    ImportReach {
        to: Some(to),
        from,
        open,
        today,
    }
}

/// The whole span of `segments`, or `None` when none holds anything.
fn whole(segments: &[Option<DaySpan>; 3]) -> Option<DaySpan> {
    segments.iter().flatten().copied().reduce(DaySpan::widen)
}

/// `held` on `/feeds.json`: the first and last day the feed's store holds,
/// over all and per segment, measured from its census — or `null` beside a
/// `why`.
fn held_json(row: Option<&census::VendorCensus>, kind: pull::vendor::SourceKind) -> String {
    let (generation, segments) = match held_spans(row) {
        Ok(found) => found,
        Err(why) => return format!(r#"{{"why":{}}}"#, render::json_string(&why)),
    };
    let Some(all) = whole(&segments) else {
        return format!(
            r#"{{"why":{}}}"#,
            render::json_string("the census loaded and lists no month: nothing is imported")
        );
    };
    let mut per = String::from("{");
    for (n, (segment, span)) in pull::manifest::SEGMENTS.iter().zip(segments).enumerate() {
        if n > 0 {
            per.push(',');
        }
        let _ = match span {
            Some(span) => write!(
                per,
                r#"{}:{{"from":{},"to":{}}}"#,
                render::json_string(segment.as_str()),
                render::json_string(&day_text(span.first)),
                render::json_string(&day_text(span.last)),
            ),
            None => write!(per, r"{}:null", render::json_string(segment.as_str())),
        };
    }
    per.push('}');
    format!(
        r#"{{"from":{},"to":{},"standing":"measured","source":{},"label_outside":{},"segments":{per}}}"#,
        render::json_string(&day_text(all.first)),
        render::json_string(&day_text(all.last)),
        render::json_string(&format!(
            "the store's census at generation {generation}: the first and last bar of every \
             month it lists"
        )),
        render::json_string(match kind {
            // A PULLED FEED IS NOT IMPORTED: a day outside what its store
            // holds has not been pulled.
            pull::vendor::SourceKind::Rest => NOT_PULLED,
            pull::vendor::SourceKind::Folder => NOT_IMPORTED,
        }),
    )
}

/// One piece of evidence about how far a bought archive reaches.
struct Evidence {
    /// `descriptor`, `census` or `journal`.
    by: &'static str,
    /// The span it shows, or why it shows none.
    span: Result<DaySpan, String>,
    /// Where it was read.
    source: String,
}

impl Evidence {
    fn json(&self) -> String {
        match &self.span {
            Ok(span) => format!(
                r#"{{"by":{},"from":{},"to":{},"source":{}}}"#,
                render::json_string(self.by),
                render::json_string(&day_text(span.first)),
                render::json_string(&day_text(span.last)),
                render::json_string(&self.source),
            ),
            Err(why) => format!(
                r#"{{"by":{},"why":{},"source":{}}}"#,
                render::json_string(self.by),
                render::json_string(why),
                render::json_string(&self.source),
            ),
        }
    }
}

/// `evidence` unless its last day is after `today` (IST epoch days); then
/// the same evidence with no span and a `why` naming both days. A day that
/// has not happened is not evidence that a file was delivered.
fn not_after_today(evidence: Evidence, today: Option<i64>) -> Evidence {
    match (&evidence.span, today) {
        (Ok(span), Some(today)) if span.last > today => Evidence {
            span: Err(format!(
                "names {} as its last day, after today {} (IST): a day that has not happened \
                 shows no delivery, so this evidence sets neither end of the archive",
                day_text(span.last),
                day_text(today)
            )),
            ..evidence
        },
        _ => evidence,
    }
}

/// Where the journal's archive evidence is read, said on its row.
const JOURNAL_SOURCE: &str = "the GDFL import journal: the first and last day an import wrote \
                              bars on, from that day's delivered files";

/// The import journal's evidence for `/feeds.json`'s archive: its written
/// span bounded at today ([`written_at_or_before`]), or why it shows none.
fn journal_evidence(root: &std::path::Path, today: Option<i64>) -> Evidence {
    let journal_source = JOURNAL_SOURCE;
    match crate::server::import_journal::snapshot(root) {
        Ok(snapshot) => {
            let written = journal_reach(root, &snapshot).written;
            match written_at_or_before(&snapshot.parsed, written, today) {
                Some(Ok((first, last, passed_over))) => Evidence {
                    by: "journal",
                    span: Ok(DaySpan {
                        first: i64::from(first.days_from_epoch()),
                        last: i64::from(last.days_from_epoch()),
                    }),
                    source: passed_over.map_or_else(
                        || journal_source.to_owned(),
                        |future| {
                            format!(
                                "{journal_source}, at or before today; a line that wrote bars \
                                 on {future}, after today, is not evidence of a delivery and \
                                 was passed over"
                            )
                        },
                    ),
                },
                Some(Err(why)) => Evidence {
                    by: "journal",
                    span: Err(why),
                    source: journal_source.to_owned(),
                },
                None => Evidence {
                    by: "journal",
                    span: Err(if snapshot.present {
                        "the journal names no day an import wrote bars on".to_owned()
                    } else {
                        "no import has run against this store: the journal is absent".to_owned()
                    }),
                    source: journal_source.to_owned(),
                },
            }
        }
        Err(why) => Evidence {
            by: "journal",
            span: Err(format!("the GDFL import journal cannot be read: {why}")),
            source: journal_source.to_owned(),
        },
    }
}

/// The first written day, the furthest written day at or before today, and
/// the passed-over last day when it was after today.
type WrittenSpan = (
    pull::session::Day,
    pull::session::Day,
    Option<pull::session::Day>,
);

/// The import journal's written span (`JournalReach::written`) bounded at
/// `today` (IST epoch days): the first written day and the FURTHEST WRITTEN
/// DAY AT OR BEFORE TODAY, with the passed-over last day when it was after
/// today; `Err` when every written day is after today; `None` when nothing
/// was written.
///
/// THE SAME END `/gaps.json` NAMES (R3-05 repair 5, the second agent's
/// follow-up 1). Before, one line for a day after today made the whole
/// journal evidence a `why`, so `/feeds.json`'s archive dropped every day the
/// journal really shows while `/gaps.json`'s `imported_to` passed over the
/// same line and kept the furthest day at or before today, and `/db`'s day
/// flags and `/gaps.json` named two ends for one journal.
///
/// Cost: none while the last written day is not after today; otherwise one
/// newest-first walk over the rows after today until a written row at or
/// before today, O(rows after that day), as `imported_to` (L-R305-3).
fn written_at_or_before(
    parsed: &crate::server::import_journal::Parsed,
    written: Option<(pull::session::Day, pull::session::Day)>,
    today: Option<i64>,
) -> Option<Result<WrittenSpan, String>> {
    let (first, last) = written?;
    let epoch = |day: pull::session::Day| i64::from(day.days_from_epoch());
    let Some(now) = today.filter(|&now| epoch(last) > now) else {
        return Some(Ok((first, last, None)));
    };
    if epoch(first) > now {
        return Some(Err(format!(
            "names no day at or before today {} (IST) that an import wrote bars on: its \
             written days run {first} to {last}, and a day that has not happened shows no \
             delivery, so this evidence sets neither end of the archive",
            day_text(now)
        )));
    }
    // `first` IS WRITTEN AND NOT AFTER TODAY, so the walk always finds a
    // day; it never passes below `first`.
    let bounded = parsed
        .range(first, last)
        .rev()
        .find(|row| wrote(row.outcome) && epoch(row.day) <= now)
        .map_or(first, |row| row.day);
    Some(Ok((first, bounded, Some(last))))
}

/// `archive` on `/feeds.json` for a feed bought as an archive: how far the
/// VENDOR'S delivered archive reaches, as far as this server can show it
/// without walking a folder — never what the store has imported (that is
/// `held`, served beside it).
///
/// Three pieces of evidence, each named in `evidence` with its span or why
/// it has none, and the widest wins at each end:
///
/// * `descriptor` — the dated measurement `pull::vendor` records
///   (`Descriptor::archive`), labelled as such;
/// * `journal` — the first and last day the GDFL import journal says an
///   import wrote bars on: the import read those days' delivered files;
/// * `census` — the store's held span: a day the store holds was delivered.
///
/// `from_binds: true`: the measured first day is where the vendor's archive
/// begins, and no file before it exists. `to_binds: false`: a measurement is
/// dated, and a later day may already be delivered and waiting (how often
/// the vendor delivers is UNVERIFIED: no line of `docs/00-charter.md` gives
/// GDFL a delivery cadence, so none is claimed) — past `to` a day is `not imported`, never
/// "outside the archive", and no form may cap there (FM-083, SC-CLOSE-2).
/// No delivered-folder inventory is read on this path (`inventory` says
/// so): `/folder.json` walks a folder on demand.
///
/// Never `{"why"}` while the descriptor records a span: an empty or
/// unreadable census leaves the descriptor's dated span in force, and the
/// census's row in `evidence` says why it adds nothing. `null` for a feed
/// that is not an archive.
///
/// NO EVIDENCE REACHES PAST TODAY (IST, `today`): a journal or census day
/// after today is a day that has not happened, so it shows no delivery.
/// That piece of evidence then sets NEITHER end and says why in its row —
/// never clamped to today, which would hide the bad line — and the other
/// evidence decides (R3-05 repair 3, the second agent's follow-up: one
/// `done` line for 2099-01-01 made `archive.to` 2099-01-01). `today: None`
/// (the clock could not be read) bounds nothing, and `to_source` still names
/// the evidence that set the end.
///
/// Cost: [`held_spans`] (a cached probe) and [`journal_reach`] (one
/// `metadata` call and a cached probe while the journal does not move).
fn archive_json(
    feed: pull::vendor::Feed,
    row: Option<&census::VendorCensus>,
    root: &std::path::Path,
    today: Option<i64>,
) -> String {
    let Some(measured) = feed.descriptor().archive else {
        return "null".to_owned();
    };
    let descriptor = Evidence {
        by: "descriptor",
        span: Ok(DaySpan {
            first: i64::from(measured.from.days_from_epoch()),
            last: i64::from(measured.to.days_from_epoch()),
        }),
        source: format!("the descriptor's dated measurement: {}", measured.source),
    };
    let census = match held_spans(row) {
        Ok((generation, segments)) => Evidence {
            by: "census",
            span: whole(&segments).ok_or_else(|| "the census loaded and lists no month".to_owned()),
            source: format!(
                "the store's census at generation {generation}: a day the store holds was \
                 delivered"
            ),
        },
        Err(why) => Evidence {
            by: "census",
            span: Err(why),
            source: "the store's census".to_owned(),
        },
    };
    let journal = journal_evidence(root, today);
    let journal = not_after_today(journal, today);
    let census = not_after_today(census, today);
    let all = [&descriptor, &journal, &census];
    // THE WIDEST AT EACH END, AND WHICH EVIDENCE SET IT. The descriptor
    // always has a span, so both folds find one; ties keep the earlier
    // evidence, the descriptor first.
    // Each end keeps the evidence that set it, never a name to look up
    // again: the source is that evidence's own (gate 11 rule 6).
    let mut from = (i64::MAX, &descriptor);
    let mut to = (i64::MIN, &descriptor);
    for evidence in all {
        if let Ok(span) = &evidence.span {
            if span.first < from.0 {
                from = (span.first, evidence);
            }
            if span.last > to.0 {
                to = (span.last, evidence);
            }
        }
    }
    let evidence: Vec<String> = all.iter().map(|evidence| evidence.json()).collect();
    format!(
        r#"{{"from":{},"to":{},"standing":"measured","from_binds":true,"to_binds":false,"source":{},"from_source":{},"to_source":{},"label_outside":{},"inventory":{},"evidence":[{}]}}"#,
        render::json_string(&day_text(from.0)),
        render::json_string(&day_text(to.0)),
        render::json_string(&format!(
            "the widest of the descriptor's dated measurement, the import journal and the \
             store's census; the last day is set by the {} and is a lower bound: a later day \
             may be delivered and not imported (the vendor's delivery cadence is UNVERIFIED)",
            to.1.by
        )),
        render::json_string(&from.1.source),
        render::json_string(&to.1.source),
        render::json_string(NOT_IMPORTED),
        render::json_string(
            "not read: no delivered-folder inventory is read on this path; /folder.json walks \
             a folder on demand"
        ),
        evidence.join(","),
    )
}

/// `window` on `/feeds.json`: the operator's window from the settings in
/// force (R3-05), for a feed this build gives a window; `null` otherwise.
///
/// LOUD WHEN THE SETTINGS ARE DEGRADED (`CLAUDE.md` §4): while a settings
/// generation cannot be read or the directory cannot be listed, an end that
/// would come from the environment or the built-in default is NOT served —
/// a saved window may differ, and serving the default would hide that — so
/// the answer is `why` and `degraded` with no `from` or `to`. An end read
/// from a generation that did read is served beside `degraded`. A window
/// whose first day is after its last (a generation an older build saved) is
/// refused the same way rather than served inverted.
fn window_json(
    feed: pull::vendor::Feed,
    settings: &crate::server::settings_route::Snapshot,
) -> String {
    if feed.descriptor().window.is_none() {
        return "null".to_owned();
    }
    let from = crate::server::settings_route::effective(settings, store::settings::Key::WindowFrom);
    let to = crate::server::settings_route::effective(settings, store::settings::Key::WindowTo);
    let degraded = crate::server::settings_route::degraded(settings);
    let degraded_json = degraded
        .as_deref()
        .map_or_else(|| "null".to_owned(), render::json_string);
    let refuse = |why: String| {
        format!(
            r#"{{"why":{},"degraded":{degraded_json}}}"#,
            render::json_string(&why)
        )
    };
    if let Some(fault) = &degraded
        && (from.origin != "generation" || to.origin != "generation")
    {
        return refuse(format!(
            "the operator's settings cannot be read ({fault}), so no window is served: the \
             built-in default would hide a saved window"
        ));
    }
    let (Some(first), Some(last)) = (from.value, to.value) else {
        return refuse(format!(
            "the operator's window has no first or last day in force (from: {}; to: {})",
            from.source, to.source
        ));
    };
    if first > last {
        return refuse(format!(
            "the window in force begins {first} ({}) after it ends {last} ({}): refused, not \
             served inverted",
            from.source, to.source
        ));
    }
    let source = if from.source == to.source {
        from.source
    } else {
        format!("from: {}; to: {}", from.source, to.source)
    };
    format!(
        r#"{{"from":{},"to":{},"standing":"operator","source":{},"label_outside":{},"degraded":{degraded_json}}}"#,
        render::json_string(&first),
        render::json_string(&last),
        render::json_string(&source),
        render::json_string(OUTSIDE_WINDOW),
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
        let (_, _, body) = feeds_json(axum::extract::State(site)).await;

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
        let (_, _, body) = feeds_json(axum::extract::State(site)).await;

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
    /// **EVERY IMPORT KIND HAS ITS OWN SLOT, IN THE TABLE'S ORDER**: the
    /// exhaustive match that replaced a search of `ImportKind::ALL`.
    #[test]
    fn every_import_kind_has_its_own_slot() {
        for (at, kind) in pull::gdfl_import::ImportKind::ALL.into_iter().enumerate() {
            assert_eq!(super::kind_slot(kind), at, "{kind:?}");
        }
    }

    /// **THE `/feeds.json` BODY IS BUILT ON THE BLOCKING POOL, NEVER ON A
    /// RUNTIME WORKER** (R3-05 repair 4, the second agent's follow-up). The
    /// census read and the cold held-span walk (0.66 to 2 s on the operator's
    /// store) ran inline in the async handler and stalled unrelated routes
    /// after a start. The handler now awaits only the settings snapshot and
    /// the blocking task; the body is a plain function.
    #[test]
    fn the_feeds_body_is_built_on_the_blocking_pool_never_a_worker() {
        let source = include_str!("feeds_route.rs");
        let handler = source
            .split("pub(super) async fn feeds_json(")
            .nth(1)
            .and_then(|rest| rest.split("\nfn feeds_body(").next())
            .expect("the handler, then the body");
        // In the named list pool since R3-08 (NEW-04), the settings read
        // and the body in one hop.
        assert!(handler.contains("crate::detail::run_list(move || {"));
        assert!(handler.contains("feeds_body(&site, &settings)"));
        assert!(
            !handler.contains("tokio::task::spawn_blocking("),
            "an unnamed, unbounded blocking task"
        );
        assert!(
            !handler.contains("census_now("),
            "no census read in the handler"
        );
        assert!(
            !handler.contains("held_json("),
            "no held-span walk in the handler"
        );
        assert!(
            handler.contains("StatusCode::SERVICE_UNAVAILABLE"),
            "a task that did not finish is said"
        );
    }

    async fn feeds_of(name: &str, root: &std::path::Path) -> Vec<serde_json::Value> {
        let dir = masters(name, None, None);
        let site = Loaded::new(Site::load(&dir, root));
        let (_, _, body) = feeds_json(axum::extract::State(site)).await;
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

    /// **GDFL IS ONE SECOND OVER ITS MEASURED ARCHIVE, RAW, AND ITS OPEN INTEREST APPLIES TO CONTRACTS ONLY**
    /// (D-2802, D-3322, D-3323, D-3324, D-3338; DB-051, DB-G24, DB-G68).
    #[tokio::test]
    async fn gdfl_is_one_second_over_its_measured_archive_and_raw() {
        let rows = feeds_of("feeds-basis", &gdfl_store("feeds-basis", None)).await;

        // GDFL: one second, over what its census measures.
        let gdfl = row(&rows, "gdfl");
        assert_eq!(gdfl["rungs"], serde_json::json!(["1s"]), "{gdfl}");
        assert_eq!(gdfl["derived_rungs"], serde_json::json!([]), "{gdfl}");
        // AN EMPTY STORE STILL HAS AN ARCHIVE (R3-05 repair 2): the
        // descriptor's dated measurement, labelled as such, its last day a
        // lower bound; the census says why it adds nothing.
        assert_eq!(gdfl["archive"]["from"], "2018-09-03", "{gdfl}");
        assert_eq!(gdfl["archive"]["to"], "2026-09-24", "{gdfl}");
        assert_eq!(gdfl["archive"]["standing"], "measured", "{gdfl}");
        assert_eq!(gdfl["archive"]["to_binds"], false, "{gdfl}");
        assert!(gdfl["archive"]["why"].is_null(), "{gdfl}");
        assert!(gdfl["held"]["why"].is_string(), "{gdfl}");
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
        // NO CENSUS ROW: the held span says why rather than inventing one.
        assert_eq!(
            held_json(None, pull::vendor::SourceKind::Folder),
            r#"{"why":"no census row for this feed"}"#
        );
        assert_eq!(words_array(std::iter::empty()), "[]");
        assert_eq!(words_array(["1s", "1min"].into_iter()), r#"["1s","1min"]"#);
    }

    /// The micros of `hh:mm:ss` IST on `day`.
    fn ist_micros(day: (u16, u8, u8), hh: i64, mm: i64, ss: i64) -> i64 {
        let days = i64::from(
            Day::new(day.0, day.1, day.2)
                .expect("a civil day")
                .days_from_epoch(),
        );
        (days * 86_400 + hh * 3_600 + mm * 60 + ss - 19_800) * 1_000_000
    }

    /// A GDFL census holding one NIFTY spot month whose last bar is on
    /// `spot_last` and one NIFTY option month whose last bar is 2020-09-16.
    fn census_with(
        root: &std::path::Path,
        manifest: Option<pull::manifest::Manifest>,
        spot_last: (u16, u8, u8),
        spot_rows: u64,
    ) -> pull::manifest::Manifest {
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        let mut manifest = manifest.unwrap_or_else(|| {
            let mut fresh = Manifest::open(brutex_core::vendor::Vendor::Gdfl, &[], &[])
                .expect("a genesis manifest");
            fresh
                .record(Entry {
                    key: EntryKey {
                        contract: Some(
                            brutex_core::instrument::Contract::parse("2020-09-24-11000-CE")
                                .expect("a contract"),
                        ),
                        exchange: brutex_core::instrument::Exchange::Nse,
                        segment: brutex_core::instrument::Segment::Fno,
                        symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
                        timeframe: store::path::Timeframe::SECOND_1,
                        month: store::path::YearMonth::new(2020, 9).expect("a month"),
                    },
                    rows: 7,
                    first_ts_micros: ist_micros((2020, 9, 1), 9, 15, 0),
                    last_ts_micros: ist_micros((2020, 9, 16), 15, 29, 59),
                })
                .expect("records the option month");
            fresh
        });
        manifest
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: brutex_core::instrument::Exchange::Nse,
                    segment: brutex_core::instrument::Segment::Index,
                    symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
                    timeframe: store::path::Timeframe::SECOND_1,
                    month: store::path::YearMonth::new(2026, 9).expect("a month"),
                },
                rows: spot_rows,
                first_ts_micros: ist_micros((2026, 9, 1), 9, 15, 0),
                last_ts_micros: ist_micros(spot_last, 15, 29, 59),
            })
            .expect("records the spot month");
        let path = manifest_path(root, brutex_core::vendor::Vendor::Gdfl);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        // A NEW FILE BESIDE, RENAMED OVER: the census reader sees one whole
        // image or the other, as a real commit leaves it.
        let next = path.with_extension("next");
        std::fs::write(&next, manifest.image()).expect("writes");
        std::fs::rename(&next, &path).expect("renames");
        manifest
    }

    /// **IMPORTING ONE DAY PAST THE OLD END MOVES THE SPAN, WITH NO REBUILD
    /// AND NO RESTART** (R3-05, SC-CLOSE-2).
    ///
    /// Before: `archive.to` was `pull::vendor`'s typed `2026-09-24` whatever
    /// the store held, so a page called 25-30 Sep "outside the vendor's
    /// archive" though the files were delivered and only the import was
    /// behind. Now the span is the census's, per segment, and a day past it
    /// is `not imported`.
    #[tokio::test]
    async fn importing_one_day_past_the_old_end_moves_the_span() {
        let root = store_root("feeds-held-moves");
        let manifest = census_with(&root, None, (2026, 9, 24), 100);
        let dir = masters("feeds-held-moves", None, None);
        let site = Loaded::new(Site::load(&dir, &root));
        let ask = |site: &Loaded| {
            let site = site.clone();
            async move {
                let (_, _, body) = feeds_json(axum::extract::State(site)).await;
                let rows: Vec<serde_json::Value> = serde_json::from_str(&body).expect("feeds JSON");
                row(&rows, "gdfl").clone()
            }
        };

        let census_file = pull::manifest::manifest_path(&root, brutex_core::vendor::Vendor::Gdfl);
        let before = ask(&site).await;
        // THE ARCHIVE'S FIRST DAY IS THE DESCRIPTOR'S MEASURED ONE: the
        // store's first held day (2020-09-01) is later, and is `held`'s.
        assert_eq!(before["archive"]["from"], "2018-09-03", "{before}");
        assert_eq!(before["held"]["from"], "2020-09-01", "{before}");
        assert_eq!(before["archive"]["to"], "2026-09-24", "{before}");
        assert_eq!(before["archive"]["standing"], "measured");
        assert_eq!(before["archive"]["label_outside"], NOT_IMPORTED);
        assert_eq!(before["held"]["segments"]["INDEX"]["to"], "2026-09-24");
        assert_eq!(before["held"]["segments"]["FNO"]["to"], "2020-09-16");
        assert!(before["held"]["segments"]["CASH"].is_null(), "{before}");
        // THE SAME ANSWER TWICE: nothing appended, nothing moves, and the
        // second answer walked nothing — one walk, on the first request.
        assert_eq!(ask(&site).await["held"], before["held"]);
        assert_eq!(walks_of(&census_file), 1, "a hit walks nothing");

        // ONE DAY IS IMPORTED PAST THE OLD END.
        census_with(&root, Some(manifest), (2026, 9, 25), 101);
        let after = ask(&site).await;
        assert_eq!(after["archive"]["to"], "2026-09-25", "{after}");
        assert_eq!(after["held"]["segments"]["INDEX"]["to"], "2026-09-25");
        // THE OPTIONS' SPAN IS ITS OWN: the spot import does not move it.
        assert_eq!(after["held"]["segments"]["FNO"]["to"], "2020-09-16");
        assert_ne!(
            after["held"]["source"], before["held"]["source"],
            "a new generation"
        );
    }

    /// **THE ARCHIVE SPAN FOLLOWS THE SOURCE'S EVIDENCE, NOT A CONSTANT AND
    /// NOT THE IMPORT** (FM-083, HUMAN-037, SC-CLOSE-2; R3-05 repair 2).
    ///
    /// `archive` is how far the VENDOR'S delivered archive reaches: the
    /// widest of the descriptor's dated measurement, the days the import
    /// journal says an import wrote bars on, and the store's held span — each
    /// end naming the evidence that set it, the last day never binding. The
    /// store's own span is `held`, beside it.
    ///
    /// Before this repair: `archive` WAS the held span, so a store imported
    /// to 2026-09-10 told /ingest the vendor's archive ended there and capped
    /// the form on it, and an empty census served `{"why"}`, so /ingest
    /// stopped treating GDFL as an imported feed. Before R3-05: the typed
    /// 2026-09-24 in every case, whatever the journal or the store showed.
    #[tokio::test]
    async fn the_archive_span_follows_the_inventory_not_a_constant() {
        let typed = pull::vendor::Feed::Gdfl
            .descriptor()
            .archive
            .expect("GDFL's descriptor records a measured archive");
        let typed_from = typed.from.to_string();
        let typed_to = typed.to.to_string();
        let root = store_root("feeds-archive-inventory");
        let dir = masters("feeds-archive-inventory", None, None);
        let site = Loaded::new(Site::load(&dir, &root));
        let ask = |site: &Loaded| {
            let site = site.clone();
            async move {
                let (_, _, body) = feeds_json(axum::extract::State(site)).await;
                let rows: Vec<serde_json::Value> = serde_json::from_str(&body).expect("feeds JSON");
                row(&rows, "gdfl").clone()
            }
        };
        let by = |answer: &serde_json::Value, who: &str| {
            answer["archive"]["evidence"]
                .as_array()
                .expect("evidence")
                .iter()
                .find(|row| row["by"] == who)
                .cloned()
                .unwrap_or_else(|| panic!("no {who} evidence: {answer}"))
        };

        // NOTHING IMPORTED, NO JOURNAL: the descriptor's dated span, named as
        // such, and each other piece of evidence says why it adds nothing.
        let empty = ask(&site).await;
        assert_eq!(empty["archive"]["from"], typed_from.as_str(), "{empty}");
        assert_eq!(empty["archive"]["to"], typed_to.as_str(), "{empty}");
        assert_eq!(empty["archive"]["from_binds"], true, "{empty}");
        assert_eq!(empty["archive"]["to_binds"], false, "{empty}");
        assert!(
            empty["archive"]["to_source"]
                .as_str()
                .is_some_and(|s| s.starts_with("the descriptor's dated measurement")),
            "{empty}"
        );
        assert!(by(&empty, "census")["why"].is_string(), "{empty}");
        assert!(by(&empty, "journal")["why"].is_string(), "{empty}");
        assert!(empty["held"]["why"].is_string(), "{empty}");

        // THE STORE HOLDS LESS THAN WAS DELIVERED: the archive is not cut
        // back to it; the store's last day is `held`'s.
        let manifest = census_with(&root, None, (2026, 9, 10), 50);
        let short = ask(&site).await;
        assert_eq!(short["archive"]["to"], typed_to.as_str(), "{short}");
        assert_eq!(short["held"]["to"], "2026-09-10", "{short}");
        assert_eq!(by(&short, "census")["to"], "2026-09-10", "{short}");

        // THE IMPORT WROTE A DAY PAST THE MEASUREMENT: the journal moves the
        // archive's last day. A day begun and never closed is no evidence.
        let journal = pull::gdfl_import::journal_path(&root);
        std::fs::create_dir_all(journal.parent().expect("imports/")).expect("mkdir");
        std::fs::write(
            &journal,
            "begin indices 2026-09-28 *\n\
             done indices 2026-09-28 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
             begin indices 2026-09-29 *\n",
        )
        .expect("the journal");
        let journaled = ask(&site).await;
        assert_eq!(journaled["archive"]["to"], "2026-09-28", "{journaled}");
        assert!(
            journaled["archive"]["to_source"]
                .as_str()
                .is_some_and(|s| s.starts_with("the GDFL import journal")),
            "{journaled}"
        );
        assert_eq!(journaled["held"]["to"], "2026-09-10", "{journaled}");

        // THE STORE HOLDS A LATER DAY: delivered, so the archive reaches it,
        // and the census names itself as the evidence.
        census_with(&root, Some(manifest), (2026, 9, 30), 150);
        let past = ask(&site).await;
        assert_eq!(past["archive"]["to"], "2026-09-30", "{past}");
        assert_eq!(past["archive"]["from"], typed_from.as_str(), "{past}");
        assert!(
            past["archive"]["to_source"]
                .as_str()
                .is_some_and(|source| source.starts_with("the store's census at generation")),
            "the span names the evidence it was read from: {past}"
        );
        assert_eq!(past["archive"]["label_outside"], NOT_IMPORTED);
    }

    /// **A DAY AFTER TODAY IS NOT EVIDENCE OF DELIVERY** (R3-05 repair 3,
    /// the second agent's follow-up). Before: one journal `done` line for
    /// 2099-01-01 made `archive.to` 2099-01-01, with the journal named as
    /// its source. Now the journal's row says why it sets neither end, the
    /// descriptor decides, and with no clock (`today: None`) nothing is
    /// bounded and the journal's day still names itself.
    #[test]
    fn a_journal_day_after_today_is_not_evidence_of_delivery() {
        let typed = pull::vendor::Feed::Gdfl
            .descriptor()
            .archive
            .expect("GDFL's descriptor records a measured archive");
        let root = store_root("feeds-archive-after-today");
        let journal = pull::gdfl_import::journal_path(&root);
        std::fs::create_dir_all(journal.parent().expect("imports/")).expect("mkdir");
        std::fs::write(
            &journal,
            "begin indices 2099-01-01 *\n\
             done indices 2099-01-01 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
        )
        .expect("the journal");
        let today = i64::from(
            Day::new(2026, 10, 15)
                .expect("a real date")
                .days_from_epoch(),
        );
        let answer: serde_json::Value = serde_json::from_str(&archive_json(
            pull::vendor::Feed::Gdfl,
            None,
            &root,
            Some(today),
        ))
        .expect("archive JSON");
        assert_eq!(answer["to"], typed.to.to_string().as_str(), "{answer}");
        assert!(
            answer["to_source"]
                .as_str()
                .is_some_and(|s| s.starts_with("the descriptor's dated measurement")),
            "{answer}"
        );
        let journal_row = answer["evidence"]
            .as_array()
            .expect("evidence")
            .iter()
            .find(|row| row["by"] == "journal")
            .cloned()
            .expect("the journal's row");
        assert!(journal_row["to"].is_null(), "{answer}");
        let why = journal_row["why"].as_str().expect("the journal says why");
        assert!(
            why.contains("2099-01-01") && why.contains("2026-10-15"),
            "{why}"
        );
        // NO CLOCK, NO BOUND: the journal's day is served as before.
        let unbounded: serde_json::Value =
            serde_json::from_str(&archive_json(pull::vendor::Feed::Gdfl, None, &root, None))
                .expect("archive JSON");
        assert_eq!(unbounded["to"], "2099-01-01", "{unbounded}");
    }

    /// **A JOURNAL LINE AFTER TODAY IS PASSED OVER, AND THE FURTHEST WRITTEN
    /// DAY AT OR BEFORE TODAY STILL COUNTS** — the end `/gaps.json`'s
    /// `imported_to` names (R3-05 repair 5, the second agent's follow-up 1).
    /// Before: one `done` line for 2099-01-01 made the whole journal row a
    /// `why`, and the 2026-09-24 the journal really shows was dropped.
    #[test]
    fn a_journal_line_after_today_is_passed_over_and_the_day_before_it_counts() {
        let root = store_root("feeds-archive-passed-over");
        let journal = pull::gdfl_import::journal_path(&root);
        std::fs::create_dir_all(journal.parent().expect("imports/")).expect("mkdir");
        std::fs::write(
            &journal,
            "begin indices 2026-09-23 *\n\
             done indices 2026-09-23 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
             begin indices 2026-09-24 *\n\
             done indices 2026-09-24 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
             begin indices 2099-01-01 *\n\
             done indices 2099-01-01 * files=2 seconds=20 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
        )
        .expect("the journal");
        let today = i64::from(
            Day::new(2026, 10, 15)
                .expect("a real date")
                .days_from_epoch(),
        );
        let answer: serde_json::Value = serde_json::from_str(&archive_json(
            pull::vendor::Feed::Gdfl,
            None,
            &root,
            Some(today),
        ))
        .expect("archive JSON");
        assert_ne!(answer["to"], "2099-01-01", "{answer}");
        let journal_row = answer["evidence"]
            .as_array()
            .expect("evidence")
            .iter()
            .find(|row| row["by"] == "journal")
            .cloned()
            .expect("the journal's row");
        assert_eq!(journal_row["from"], "2026-09-23", "{answer}");
        assert_eq!(journal_row["to"], "2026-09-24", "{answer}");
        let source = journal_row["source"].as_str().expect("a source");
        assert!(
            source.contains("2099-01-01") && source.contains("passed over"),
            "{source}"
        );
        // ON OR BEFORE TODAY NOTHING IS PASSED OVER: the plain source.
        let on_the_day = i64::from(Day::new(2099, 1, 1).expect("a day").days_from_epoch());
        let whole: serde_json::Value = serde_json::from_str(&archive_json(
            pull::vendor::Feed::Gdfl,
            None,
            &root,
            Some(on_the_day),
        ))
        .expect("archive JSON");
        let row = whole["evidence"]
            .as_array()
            .expect("evidence")
            .iter()
            .find(|row| row["by"] == "journal")
            .cloned()
            .expect("the journal's row");
        assert_eq!(row["to"], "2099-01-01", "{whole}");
        assert!(
            !row["source"]
                .as_str()
                .is_some_and(|s| s.contains("passed over")),
            "{whole}"
        );
    }

    /// **THE WINDOW IS REFUSED LOUDLY WHEN THE SETTINGS CANNOT BE READ, AND
    /// NEVER SERVED INVERTED** (R3-05 repair 2, the second agent's blocking
    /// 2 and 3).
    ///
    /// Before: with the settings directory unreadable, `/feeds.json` served
    /// the built-in 2018-01-01..2026-09-30 as the window in force with no
    /// word, while `/settings.json` said `degraded`; and a generation that
    /// set `window.from` alone past the default `window.to` was served with
    /// its first day after its last.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_window_is_refused_loudly_when_degraded_or_inverted() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = store_root("feeds-window-degraded");
        let (status, body) = crate::server::settings_route::change(
            &root,
            r#"{"expected":0,"changed_by":"test","reason":"widen","set":{"window.to":"2026-10-31"}}"#,
        );
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        let settings_dir = store::settings::dir(&root);
        std::fs::set_permissions(&settings_dir, std::fs::Permissions::from_mode(0o000))
            .expect("chmod 000");
        let readable_anyway = std::fs::read_dir(&settings_dir).is_ok();
        let rows = feeds_of("feeds-window-degraded", &root).await;
        std::fs::set_permissions(&settings_dir, std::fs::Permissions::from_mode(0o755))
            .expect("chmod back");
        assert!(!readable_anyway, "this host reads a 000 directory (root?)");
        let window = &row(&rows, "zerodha")["window"];
        assert!(window["from"].is_null(), "{window}");
        assert!(window["to"].is_null(), "{window}");
        assert!(
            window["why"]
                .as_str()
                .is_some_and(|why| why.contains("cannot be read")),
            "{window}"
        );
        assert!(window["degraded"].is_string(), "{window}");

        // AN INVERTED GENERATION, as a build without the whole-window check
        // could save it: `window.from` alone, past the default `window.to`.
        let root = store_root("feeds-window-inverted");
        let mut reader = store::settings::Reader::new(&root);
        store::settings::save(
            &mut reader,
            &store::settings::Change {
                expected: 0,
                set: vec![(store::settings::Key::WindowFrom, "2026-10-01".to_owned())],
                unset: Vec::new(),
                changed_by: "an older build".to_owned(),
                reason: "narrow to October".to_owned(),
                at_micros: 1_700_000_000_000_000,
            },
            |_| Ok(()),
        )
        .expect("the store's own check passes one end");
        let rows = feeds_of("feeds-window-inverted", &root).await;
        let window = &row(&rows, "zerodha")["window"];
        assert!(window["from"].is_null(), "{window}");
        assert!(
            window["why"]
                .as_str()
                .is_some_and(|why| why.contains("not served inverted")),
            "{window}"
        );
        assert!(window["degraded"].is_null(), "{window}");
    }

    /// **THE WINDOW IS THE OPERATOR'S SETTING**: saved, it is served on the
    /// next request with no restart, and the built-in default is named while
    /// nothing is saved (R3-05).
    #[tokio::test]
    async fn the_window_on_feeds_json_follows_the_saved_setting() {
        let root = store_root("feeds-window-setting");
        let rows = feeds_of("feeds-window-setting", &root).await;
        let zerodha = row(&rows, "zerodha");
        assert_eq!(zerodha["window"]["to"], "2026-09-30", "{zerodha}");
        assert!(
            zerodha["window"]["source"]
                .as_str()
                .is_some_and(|s| s.starts_with("built-in default")),
            "{zerodha}"
        );
        let (status, body) = crate::server::settings_route::change(
            &root,
            r#"{"expected":0,"changed_by":"test","reason":"widen","set":{"window.to":"2026-10-31"}}"#,
        );
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        let rows = feeds_of("feeds-window-setting", &root).await;
        let zerodha = row(&rows, "zerodha");
        assert_eq!(zerodha["window"]["from"], "2018-01-01", "{zerodha}");
        assert_eq!(zerodha["window"]["to"], "2026-10-31", "{zerodha}");
        let source = zerodha["window"]["source"].as_str().expect("a source");
        assert!(source.starts_with("from: built-in default"), "{source}");
        assert!(source.ends_with("; to: settings generation 1"), "{source}");
    }

    /// The p50, p99 and max of `samples`, nanoseconds.
    fn quantiles(mut samples: Vec<u128>) -> (u128, u128, u128) {
        samples.sort_unstable();
        let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
        (at(50), at(99), *samples.last().expect("samples"))
    }

    /// **GROWTH BENCH OF THE HELD SPAN: 1x, 10x, 100x KEYS** (R3-05).
    ///
    /// A census of 1,000, 10,000 and 100,000 option months. For each: the
    /// first measure (a walk, O(keys)), 200 repeats at the same generation (a
    /// hit, O(1)), and 200 one-key appends each read as a tail (a fold of the
    /// appended key, O(appended)). The ratio gate: a hit and an append at 100x
    /// cost no more than 4x their 1x figure, each the quietest of three
    /// rounds of 200 (one round's p99 is three samples on a shared host);
    /// the walk is allowed to grow and is the cost named for
    /// `docs/06-limits.md`. Proof of `part_span`'s claim: this test,
    /// `api::server::feeds_route::tests::held_span_growth_bench`.
    #[test]
    #[ignore = "a growth bench: cargo test --release -p api held_span_growth -- --ignored --nocapture"]
    fn held_span_growth_bench() {
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        let entry = |n: u32| Entry {
            key: EntryKey {
                contract: Some(
                    brutex_core::instrument::Contract::parse(&format!(
                        "2020-09-24-{}-CE",
                        10_000 + n
                    ))
                    .expect("a contract"),
                ),
                exchange: brutex_core::instrument::Exchange::Nse,
                segment: brutex_core::instrument::Segment::Fno,
                symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
                timeframe: store::path::Timeframe::SECOND_1,
                month: store::path::YearMonth::new(2020, 9).expect("a month"),
            },
            rows: 1,
            first_ts_micros: ist_micros((2020, 9, 1), 9, 15, 0),
            last_ts_micros: ist_micros(
                (2020, 9, 1 + u8::try_from(n % 20).expect("small")),
                15,
                29,
                59,
            ),
        };
        let mut hits_p99 = Vec::new();
        let mut appends_p99 = Vec::new();
        for keys in [1_000_u32, 10_000, 100_000] {
            let root = store_root(&format!("held-bench-{keys}"));
            let path = manifest_path(&root, brutex_core::vendor::Vendor::Gdfl);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            let mut manifest =
                Manifest::open(brutex_core::vendor::Vendor::Gdfl, &[], &[]).expect("genesis");
            for n in 0..keys {
                manifest.record(entry(n)).expect("records");
            }
            std::fs::write(&path, manifest.image()).expect("writes");
            let mut census = census::read_vendor(&root, brutex_core::vendor::Vendor::Gdfl);
            let started = std::time::Instant::now();
            let (_, spans) = held_spans(Some(&census)).expect("a span");
            let walk = started.elapsed().as_nanos();
            assert_eq!(
                spans[2].map(|s| day_text(s.last)),
                Some("2020-09-20".to_owned())
            );
            // THREE ROUNDS PER SCALE, THE GATE ON THE QUIETEST: on a shared
            // host one round's p99 is the third-largest of 200 samples, and
            // two descheduled samples move it 60x (14 Oct, load 337 on 14
            // cores: 1x 2,750 ns, 100x 166,416 ns, max 1.1 ms). A path that
            // grew with the census would be slow in EVERY round, so the
            // smallest round p99 still fails it; every round is printed.
            let mut hit_rounds = Vec::new();
            let mut append_rounds = Vec::new();
            for round in 0..3_u32 {
                let hits: Vec<u128> = (0..200)
                    .map(|_| {
                        let at = std::time::Instant::now();
                        let _ = held_spans(Some(&census)).expect("a span");
                        at.elapsed().as_nanos()
                    })
                    .collect();
                let mut appends = Vec::new();
                for n in 0..200 {
                    manifest
                        .record(entry(keys + round * 200 + n))
                        .expect("records");
                    std::fs::write(&path, manifest.image()).expect("writes");
                    census = census::read_vendor_after(
                        &root,
                        brutex_core::vendor::Vendor::Gdfl,
                        Some(&census),
                    );
                    let at = std::time::Instant::now();
                    let _ = held_spans(Some(&census)).expect("a span");
                    appends.push(at.elapsed().as_nanos());
                }
                let (hp50, hp99, hmax) = quantiles(hits);
                let (ap50, ap99, amax) = quantiles(appends);
                println!(
                    "held span, {keys} keys, round {round}: walk {walk} ns; hit p50 {hp50} p99 {hp99} \
                     max {hmax} ns (n=200); one-key append p50 {ap50} p99 {ap99} max {amax} ns (n=200)"
                );
                hit_rounds.push(hp99);
                append_rounds.push(ap99);
            }
            // ONE WALK PER SCALE, on the first measure: no hit and no
            // one-key append walked the census again.
            let walks = walks_of(&path);
            println!("held span, {keys} keys: walks {walks}");
            assert_eq!(
                walks, 1,
                "{keys} keys: a hit or an append walked the census"
            );
            hits_p99.push(hit_rounds.into_iter().min().expect("three rounds"));
            appends_p99.push(append_rounds.into_iter().min().expect("three rounds"));
        }
        assert!(hits_p99[2] <= hits_p99[0].max(1_000) * 4, "{hits_p99:?}");
        assert!(
            appends_p99[2] <= appends_p99[0].max(1_000) * 4,
            "{appends_p99:?}"
        );
    }
}

//! `/store.json` -- one feed's census on the wire, its validator, and the
//! three census headers `/store.json` and `/instruments.json` stamp.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The only edits are visibility
//! qualifiers (`pub(super)`) on items another module now has to reach.
//! No route answers differently because of the move.

use super::{
    Loaded, Site, Unknown, hex32, month_before, month_change, no_such_feed_json, note_header,
    param, store_wire, vendor_census,
};
use crate::{census, ingest, render};
use brutex_core::vendor::Vendor;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

/// The state of the selected feed's census, as one stable word.
///
/// `held` · `absent` · `unreadable` — [`census::Census::name`]'s words, which
/// are `/audit.json`'s words. A feed with no census row in this build reads
/// `absent`, exactly as `audit_json::store_block` renders it, so the two
/// surfaces cannot drift into two vocabularies for one fact.
pub const CENSUS_STATE_HEADER: &str = "x-brutex-census-state";

/// The sentence [`census::VendorCensus::note`] produces for that state.
pub const CENSUS_NOTE_HEADER: &str = "x-brutex-census-note";

/// What loading the census had to step over, or empty when it stepped over
/// nothing. `/audit.json` writes `null` here; a header has no `null`, and an
/// **always-present, sometimes-empty** value keeps "nothing was stepped over"
/// and "this build does not say" apart — an absent header is the second.
pub const CENSUS_DEGRADED_HEADER: &str = "x-brutex-census-degraded";

/// The census's commit counter ([`census::VendorCensus::generation`]), or
/// empty when no census loaded. D-3327 item 2.
///
/// For a partitioned census it is the SUM of the parts' generations: a value
/// that rises whenever any part moves and never goes back, so a page can
/// tell "the store moved" with one comparison. It counts nothing (each part
/// carries the legacy file's generation as its base, so the sum counts that
/// base once per part; DB-B67), and it is never a `since` for
/// `/census/facets.json`, which answers in ONE part's generation
/// ([`CENSUS_PART_GENERATIONS_HEADER`], [`CENSUS_PART_GENERATION_HEADER`]).
pub(crate) const CENSUS_GENERATION_HEADER: &str = "x-brutex-census-generation";

/// Each partition's own generation, `INDEX=2097116,CASH=2114847,FNO=2841866`,
/// or empty for a single file (whose one generation is
/// [`CENSUS_GENERATION_HEADER`]). The same spelling as
/// [`CENSUS_EPOCH_HEADER`], on `/store.json`, its `HEAD` and
/// `/census/facets.json`, so one census is spelled one way on every route
/// (WP-56; DB-B56, DB-B67; D-3327 item 3).
pub(crate) const CENSUS_PART_GENERATIONS_HEADER: &str = "x-brutex-census-part-generations";

/// On `/census/facets.json` only: the generation of the part that answered,
/// which is the body's `generation` and the only value its `?since=` takes
/// (WP-56; DB-B56). A client that keyed its delta on
/// [`CENSUS_GENERATION_HEADER`] (the vendor-wide sum) was refused every time.
pub(crate) const CENSUS_PART_GENERATION_HEADER: &str = "x-brutex-census-part-generation";

/// On an answer to `/store.json?...&since=` only, a delta or its refusal: the
/// `since` it answers. Absent on a whole body, which is how a reader tells a
/// delta from the whole census an older API sends for the same request
/// before it reads either (WP-56; D-3327 item 3).
pub(crate) const CENSUS_SINCE_HEADER: &str = "x-brutex-census-since";

/// The entries the tightest part may hold, from its own header. D-3327.
pub(crate) const CENSUS_CAPACITY_HEADER: &str = "x-brutex-census-capacity";

/// The entries the tightest part has left. D-3327, DB-G07.
pub(crate) const CENSUS_HEADROOM_HEADER: &str = "x-brutex-census-headroom";

/// Each partition's current epoch, `INDEX=000001,CASH=000001,FNO=000003`
/// (`none` for a partition with no epoch yet), or empty for a single file.
/// D-3327, D-3316.
pub(crate) const CENSUS_EPOCH_HEADER: &str = "x-brutex-census-epoch";

/// Days the import journal closed census-refused and has not recounted
/// (D-3317), or empty where the feed keeps no import journal. D-3327.
pub(crate) const CENSUS_BEHIND_DAYS_HEADER: &str = "x-brutex-census-behind-days";

/// The first such day, `YYYY-MM-DD`, or empty. D-3327.
pub(crate) const CENSUS_BEHIND_FROM_HEADER: &str = "x-brutex-census-behind-from";

/// The sentence naming how a migrated vendor's legacy single file changed
/// since migration, or empty (DB-B70, WP-81). Non-empty makes the state word
/// `behind` and leads `x-brutex-census-reason`.
pub(crate) const CENSUS_LEGACY_DRIFT_HEADER: &str = "x-brutex-census-legacy-drift";

/// Which census format answered: `v4-partitioned` for a migrated vendor,
/// `v3-single` for one still in its single file, or empty when no census
/// loaded (DB-B68, WP-81). A reader — or a deploy script comparing two
/// builds — can tell from one header which census a number came from.
pub(crate) const CENSUS_FORMAT_HEADER: &str = "x-brutex-census-format";

/// Why the state is `capped` or `behind`, in a sentence; empty when it is
/// neither.
///
/// # Why not in `x-brutex-census-degraded`
///
/// That header means what LOADING stepped over: a damaged generation, so the
/// rows are an OLDER census than the one committed. Every reader treats a
/// non-empty value so, and refuses to draw the census at all
/// (`web/src/lib/store-census.js`, `degradedRefusal`). A census that is
/// behind its import or near its capacity is the current one, complete for
/// every month it counts; refusing to draw it would hide the months it does
/// hold to report the ones it does not. So that fact has its own header and
/// its own state word, and is never folded into "damaged" (DB-G07, DB-G11).
pub(crate) const CENSUS_REASON_HEADER: &str = "x-brutex-census-reason";

/// The three census facts, stamped on a JSON response that cannot carry them
/// in its body.
///
/// # Why headers and not a field
///
/// `/store.json` answers a JSON **array** and `/db` reads it as one —
/// `rows = Array.isArray(j) ? j : []`. Wrapping the rows in an object to make
/// room for a status would turn every existing reader into a reader of `[]`,
/// which is the very outcome being fixed. A header is additive: an array stays
/// an array, and a reader that wants the reason asks for it by name.
///
/// # What a page must read
///
/// `r.headers.get('x-brutex-census-state')` — `unreadable` means the array is
/// EMPTY BECAUSE THE COUNTER IS DAMAGED, not because the store is. The sentence
/// to show is `x-brutex-census-note`. The response also carries a non-200 for
/// that state, so a reader that only checks `r.ok` stops calling it empty.
/// D-0124.
#[cfg(test)]
pub(super) fn census_headers(census: Option<&census::VendorCensus>) -> axum::http::HeaderMap {
    census_headers_with(census, None)
}

/// [`census_headers`], with what the import journal says about the census
/// when the caller asked it ([`behind`]). D-3327 item 2.
///
/// The state word is [`census::VendorCensus::state_word`]: `held`, `absent`
/// and `unreadable` as before, and `capped` or `behind` for a census that
/// loaded and cannot be called complete. Every new header is ALWAYS present,
/// empty where it does not apply, so "this build says nothing" (no header)
/// and "nothing to say" (empty) stay apart, as `CENSUS_DEGRADED_HEADER`
/// already keeps them. The behind headers are absent only when the caller did
/// not ask the journal.
pub(super) fn census_headers_with(
    census: Option<&census::VendorCensus>,
    behind: Option<&Behind>,
) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    let behind_days = behind.map_or(0, Behind::state_days);
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_STATE_HEADER),
        // A feed with no row is `absent`, which is what
        // `audit_json::store_block` already answers for the same input.
        axum::http::HeaderValue::from_static(
            census.map_or("absent", |census| census.state_word(behind_days)),
        ),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_NOTE_HEADER),
        note_header(&census.map_or_else(
            || String::from("no census row for this feed in this build"),
            census::VendorCensus::note,
        )),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_DEGRADED_HEADER),
        note_header(
            &census
                .and_then(census::VendorCensus::degraded)
                .unwrap_or_default(),
        ),
    );
    let room = census.and_then(census::VendorCensus::room);
    let number = |value: Option<u64>| value.map_or_else(String::new, |n| n.to_string());
    for (name, value) in [
        (
            CENSUS_GENERATION_HEADER,
            number(census.and_then(census::VendorCensus::generation)),
        ),
        (
            CENSUS_CAPACITY_HEADER,
            number(room.map(|room| room.capacity)),
        ),
        (
            CENSUS_HEADROOM_HEADER,
            number(room.map(|room| room.headroom)),
        ),
        (CENSUS_EPOCH_HEADER, census.map_or_else(String::new, epochs)),
        (
            CENSUS_FORMAT_HEADER,
            census.map_or_else(String::new, format_word),
        ),
        (
            CENSUS_PART_GENERATIONS_HEADER,
            census.map_or_else(String::new, part_generations),
        ),
        (CENSUS_REASON_HEADER, reason(room.as_ref(), behind)),
    ] {
        headers.insert(
            axum::http::HeaderName::from_static(name),
            note_header(&value),
        );
    }
    if let Some(behind) = behind {
        headers.insert(
            axum::http::HeaderName::from_static(CENSUS_LEGACY_DRIFT_HEADER),
            note_header(behind.legacy.as_deref().unwrap_or_default()),
        );
        headers.insert(
            axum::http::HeaderName::from_static(CENSUS_BEHIND_DAYS_HEADER),
            note_header(&if behind.kept {
                behind.days.to_string()
            } else {
                String::new()
            }),
        );
        headers.insert(
            axum::http::HeaderName::from_static(CENSUS_BEHIND_FROM_HEADER),
            note_header(&behind.from.map_or_else(String::new, |day| day.to_string())),
        );
    }
    headers
}

/// [`CENSUS_FORMAT_HEADER`]'s word for a census that loaded, or empty.
fn format_word(census: &census::VendorCensus) -> String {
    match census.state {
        census::Census::Held { ref manifest } if manifest.is_partitioned() => {
            "v4-partitioned".to_owned()
        }
        census::Census::Held { .. } => "v3-single".to_owned(),
        census::Census::Absent | census::Census::Unreadable { .. } => String::new(),
    }
}

/// Each partition's current epoch, as [`CENSUS_EPOCH_HEADER`] spells it.
fn epochs(census: &census::VendorCensus) -> String {
    let census::Census::Held { ref manifest } = census.state else {
        return String::new();
    };
    if !manifest.is_partitioned() {
        return String::new();
    }
    let mut out = String::new();
    for (n, part) in manifest.parts().iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{}={}",
            part.segment().map_or("", |segment| segment.as_str()),
            part.epoch()
                .map_or_else(|| "none".to_owned(), |epoch| format!("{epoch:06}"))
        );
    }
    out
}

/// Each partition's own generation, as [`CENSUS_PART_GENERATIONS_HEADER`]
/// spells it: one field read per part.
fn part_generations(census: &census::VendorCensus) -> String {
    let census::Census::Held { ref manifest } = census.state else {
        return String::new();
    };
    if !manifest.is_partitioned() {
        return String::new();
    }
    let mut out = String::new();
    for (n, part) in manifest.parts().iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{}={}",
            part.segment().map_or("", |segment| segment.as_str()),
            part.header().generation
        );
    }
    out
}

/// [`CENSUS_REASON_HEADER`]'s sentence: behind first, then capped, then a
/// journal that could not be read.
fn reason(room: Option<&census::Room>, behind: Option<&Behind>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(legacy) = behind.and_then(|behind| behind.legacy.as_ref()) {
        parts.push(format!(
            "legacy drift: {legacy}; `cli census-recount` counts them from the bar files \
             (DB-B70)"
        ));
    }
    if let Some(behind) = behind {
        if behind.days > 0 {
            parts.push(format!(
                "behind: the import journal closed {} day(s) census-refused from {} and has \
                 not recounted them, so their bars are on disk and not in this census; \
                 `cli census-recount` counts them from the bar files (D-3316, D-3317)",
                behind.days,
                behind.from.map_or_else(String::new, |day| day.to_string())
            ));
        }
        if let Some(why) = &behind.unread {
            parts.push(format!(
                "the import journal could not be read ({why}), so whether this census is \
                 behind its import is not known"
            ));
        }
    }
    if let Some(capped) = room.and_then(census::Room::capped_reason) {
        parts.push(capped);
    }
    parts.join("; ")
}

/// How far a feed's census is behind the bars its import wrote. D-3327 item
/// 2, DB-G03, DB-G11, DB-G60, DB-G78.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Behind {
    /// Distinct days the import journal closed census-refused and has not
    /// recounted.
    pub(crate) days: u64,
    /// The first of them.
    pub(crate) from: Option<pull::session::Day>,
    /// Whether this feed keeps an import journal at all; `false` writes the
    /// behind-days header empty rather than `0`.
    pub(crate) kept: bool,
    /// Why the journal could not be read, when it could not.
    pub(crate) unread: Option<String>,
    /// The migrated census's legacy single file has changed since migration
    /// (DB-B70, WP-81): a writer built before version 4 appended months no
    /// partition counts. The sentence names the receipt's length and the
    /// current one; `None` when the vendor is not migrated or nothing moved.
    pub(crate) legacy: Option<String>,
}

impl Behind {
    /// The day count the state word is decided by: a census whose legacy file
    /// drifted is `behind` even with no journal day behind, because months
    /// are on disk that it does not count (DB-B70).
    pub(crate) fn state_days(&self) -> u64 {
        if self.legacy.is_some() {
            self.days.max(1)
        } else {
            self.days
        }
    }
}

/// What [`behind`] last answered, and the journal and log reads it was
/// derived from: the same `Arc`s again are the same answer.
#[derive(Debug, Default)]
pub(super) struct BehindCache(Mutex<Option<BehindKept>>);

/// One kept [`behind`] answer: the journal read, the log read, the answer.
type BehindKept = (
    Arc<super::import_journal::Parsed>,
    Arc<crate::logs::ImportRefusals>,
    Behind,
);

/// How far `feed`'s census is behind its import (D-3327 item 2).
///
/// # Read from the import journal, never from the bar tree
///
/// A day the census refused is closed `incomplete` (or `refused`) by the
/// importer with `stage=census`, and before D-3317 with no stage at all, the
/// importer's own `refused` event naming the census instead; the stage is
/// settled exactly as `/imports.json` settles it (`imports::stage_of`). A day
/// later closed `recounted` (or `done`) is not behind. Counting the bar
/// files the census does not hold would be the directory walk the census
/// exists to avoid (`docs/07-o1-architecture.md` law 3).
///
/// # Cost
///
/// The journal and the importer's logs are each read through their own tail
/// (`import_journal::snapshot`, `logs::import_refusals`): a `stat`, and the
/// new bytes. The log read is every directory a GDFL run can write, merged
/// (`logs/cli`, `logs/gdfl/<kind>`, `logs/gdfl/store`; D-4862 round 6), and
/// its `Arc` stands for the tuple of the per-directory reads: the same
/// pointer again means none of them moved. The count is a walk of the journal's days (2,000 on the
/// operator's store) made once per change of either, and kept against them;
/// a request with neither changed is two pointer comparisons.
pub(crate) fn behind(site: &Site, feed: Vendor) -> Behind {
    // A PRE-VERSION-4 WRITER IS VISIBLE HERE, EVERY REQUEST (DB-B70, WP-81).
    // The legacy single file stays beside a migrated vendor's partitions, and
    // a binary built before version 4 (an old `cli`, an old api's pull) still
    // appends to it; the version-4 census never reads it, so those months
    // were invisible to every page. One small read of the receipt and one
    // `lstat`, outside the journal cache below because neither read moves
    // the journal.
    let legacy = pull::ingest::census_store::legacy_drift(&site.store_root, feed);
    Behind {
        legacy,
        ..journal_behind(site, feed)
    }
}

/// [`behind`]'s import-journal half.
fn journal_behind(site: &Site, feed: Vendor) -> Behind {
    // ONLY GDFL'S IMPORT KEEPS A JOURNAL (`pull::gdfl_import::journal_path`).
    // A broker pull records its runs in the audit journal, which says nothing
    // about a census refusal, so every other feed is not measured here and
    // says so with an empty header rather than a zero.
    if feed != Vendor::Gdfl {
        return Behind::default();
    }
    let snapshot = match super::import_journal::snapshot(&site.store_root) {
        Ok(snapshot) => snapshot,
        Err(why) => {
            return Behind {
                kept: true,
                unread: Some(why),
                ..Behind::default()
            };
        }
    };
    let evidence = crate::logs::import_refusals(&site.store_root);
    let cache = &site.census_wire.behind;
    if let Ok(held) = cache.0.lock()
        && let Some((parsed, logged, answer)) = held.as_ref()
        && Arc::ptr_eq(parsed, &snapshot.parsed)
        && Arc::ptr_eq(logged, &evidence)
    {
        return answer.clone();
    }
    let answer = behind_in(&snapshot.parsed, &evidence);
    if let Ok(mut held) = cache.0.lock() {
        *held = Some((Arc::clone(&snapshot.parsed), evidence, answer.clone()));
    }
    answer
}

/// [`behind`]'s count over one journal read and one log read.
fn behind_in(
    parsed: &super::import_journal::Parsed,
    evidence: &crate::logs::ImportRefusals,
) -> Behind {
    let mut out = Behind {
        kept: true,
        ..Behind::default()
    };
    let Some((first, last)) = parsed.span() else {
        return out;
    };
    let census_stage = pull::gdfl_import::Stage::Census.as_str();
    let mut counted: Option<pull::session::Day> = None;
    for row in parsed.range(first, last) {
        if row.outcome.is_done() {
            continue;
        }
        let logged = evidence.of(row.kind.as_str(), &row.day.to_string());
        let refused_by_census =
            super::imports::stage_of(row, &logged).is_some_and(|(stage, _)| stage == census_stage);
        if !refused_by_census || counted == Some(row.day) {
            continue;
        }
        counted = Some(row.day);
        out.days = out.days.saturating_add(1);
        out.from = Some(out.from.map_or(row.day, |from| from.min(row.day)));
    }
    out
}

/// Whether the selected feed's counter refused to load.
///
/// The one state where a body of `[]` and a body of every row this store holds
/// are produced by the same code path, and the reason a status code moves.
pub(super) fn census_is_unreadable(census: Option<&census::VendorCensus>) -> bool {
    matches!(
        census.map(|c| &c.state),
        Some(census::Census::Unreadable { .. })
    )
}

/// The census row for one key — the counters and the closes in **one probe**.
///
/// `pull::manifest::Manifest::held` is one hash probe into a table the load
/// walk built once, so this reads `rows` and both closes for the cost the old
/// `rows_for` paid for `rows` alone. Proven by
/// `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file`.
pub(super) fn held_row(
    census: &census::VendorCensus,
    key: &pull::manifest::EntryKey,
) -> Option<pull::manifest::Held> {
    match census.state {
        census::Census::Held { ref manifest } => manifest.held(key),
        census::Census::Absent | census::Census::Unreadable { .. } => None,
    }
}

/// One row of `/store.json`, appended to `out`.
///
/// Written as `"chg_bps": <n>, "chg_why": null` **or** `"chg_bps": null,
/// "chg_why": "<code>"` — both keys always present, exactly one of them a
/// value. A key that is sometimes absent makes "unknown" and "the field does
/// not exist in this build" the same thing on the wire, and the browser cannot
/// tell them apart.
fn write_change(out: &mut String, key: &str, change: Result<i64, Unknown>) {
    match change {
        Ok(bps) => {
            let _ = write!(out, r#","{key}_bps":{bps},"{key}_why":null"#);
        }
        Err(why) => {
            let _ = write!(
                out,
                r#","{key}_bps":null,"{key}_why":{}"#,
                render::json_string(why.code())
            );
        }
    }
}

/// The body of `/store.json` for one feed.
///
/// Split out of the handler so every arm is reachable from a test without a
/// socket — the same argument this module's own header makes about `main.rs`.
///
/// # Cost
///
/// **Two hash probes per row and no syscall**: this month's row, and the same
/// series at [`month_before`]. Crossing a month crosses a manifest *entry*, not
/// a file — no `open`, no `stat`, no `pread`, because the census is already
/// resident. Proven by
/// `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file`.
///
/// What is **not** constant is the caller: `census_now` re-reads and re-walks
/// every manifest per request, which is O(entries) and was before this existed.
/// `docs/06-limits.md` §41 records it rather than this comment hiding it.
pub(super) fn store_body(
    censuses: &[census::VendorCensus],
    entries: &[(census::Series, store::path::YearMonth)],
    feed: Vendor,
) -> String {
    // ONE LOOKUP FOR THE WHOLE RESPONSE, not one per row. `Vendor::ALL` is four
    // long so the old per-row `find` was cheap, but it was O(vendors) inside an
    // O(entries) loop for a value that cannot change between iterations — and
    // a feed with no census at all is now one comparison rather than one per
    // entry that could never match.
    let Some(census) = censuses.iter().find(|c| c.vendor == feed) else {
        return String::from("[]");
    };
    // THE ANSWER SAYS WHO PRODUCED IT. Until this field existed no byte of a
    // `/store.json` body named the feed, so a reader that got the wrong one had
    // no way to notice — which is half of why substituting Dhan for an unknown
    // feed went unseen for as long as it did. The other half is now a `400`;
    // see `store_json`.
    //
    // ENCODED ONCE, not once per row. `Vendor::as_str` is a fixed token, and
    // allocating its quoted form again for each of the 43,422 entries
    // `docs/06-limits.md` §34 projects is a per-row cost with no reader.
    let feed_json = render::json_string(feed.as_str());
    let mut out = String::from("[");
    let mut n = 0usize;
    for (series, month) in entries {
        let Some(row) = held_row(census, &series.at(*month)) else {
            continue;
        };
        if n > 0 {
            out.push(',');
        }
        n += 1;
        // THE ROW'S OWN RUNG, NOT A LITERAL.
        //
        // This wrote `"timeframe":"1min"` on every row, unconditionally, while
        // `Series` has carried the real `Timeframe` since D-0055 and the store
        // has filed under `1day/` and `1min/` accordingly. The consequences all
        // landed on the reader, and all of them looked like data faults:
        //
        //   * `/db`'s TF column was a sort control over one repeated value —
        //     20,516 identical cells that could never differ.
        //   * `/db`'s completeness divided DAILY bars by a 375-bar MINUTE
        //     session, so a complete 1day month read as ~99.7% missing.
        //   * the Markets chart asked for `…/1min/2021-08.bin` for an
        //     instrument stored under `1day/`, and answered "That read failed"
        //     about a file that was never supposed to exist.
        //
        // The store was right the whole time. `Timeframe::as_str` is the SAME
        // word the path segment uses — `1min`, `1day` — so a reader that keys
        // off this string and a reader that keys off the directory cannot
        // disagree again.
        // THE WINDOW THIS MONTH ACTUALLY COVERS, not just how many bars it
        // holds. The census has carried `first_ts_micros` and `last_ts_micros`
        // per entry since it was written; nothing put them on the wire, so a
        // reader could say "21 bars" and not "09 Aug 2021 to 31 Aug 2021".
        // Deriving them from the month name would be a guess — a month holding
        // one bar covers one day, and which day is a fact only the entry has.
        //
        // Microseconds, unconverted. The browser owns the IST rendering, the
        // same way it owns the decimal point on basis points: one encoding on
        // the wire, one place that formats it. `CLAUDE.md` §7's reasoning
        // applied to time rather than money.
        let _ = write!(
            out,
            r#"{{"feed":{feed_json},"instrument":{},"month":{},"timeframe":{},"rows":{},"first_ts":{},"last_ts":{}"#,
            render::json_string(&series.to_string()),
            render::json_string(&month.to_string()),
            render::json_string(series.timeframe.as_str()),
            row.entry.rows,
            row.entry.first_ts_micros,
            row.entry.last_ts_micros
        );
        write_change(
            &mut out,
            "chg",
            month_change(series.segment, series.symbol.as_str(), Some(row.closes)),
        );
        // THE PREVIOUS MONTH IS A PROBE, NOT A SCAN, AND NOT AN ASSUMPTION. A
        // month the census does not hold is `no_earlier_month` — never zero,
        // and never this month's number repeated.
        let before = month_before(*month).and_then(|at| held_row(census, &series.at(at)));
        write_change(
            &mut out,
            "prev_chg",
            month_change(
                series.segment,
                series.symbol.as_str(),
                before.map(|held| held.closes),
            ),
        );
        out.push('}');
    }
    out.push(']');
    out
}

/// One census body's validator, for conditional requests.
///
/// # Why this endpoint of all of them
///
/// `/store.json` is the whole census and it GROWS WITH THE STORE. Measured on a
/// running binary mid-pull: 329 KB, then 377,735 bytes minutes later, for ONE
/// feed. The console polls it every five seconds while a run is in flight —
/// `watchStore(5000)` — so an unchanged store was still costing a full transfer
/// and a full `JSON.parse` on the reader's main thread, every tick, for the
/// length of a run.
///
/// A poll that changes nothing should cost nothing. With a validator it costs a
/// 304 and an empty body.
///
/// # Why the digest and not a counter
///
/// A generation counter would be cheaper to compute and WRONG: it advances when
/// the store is re-read, not when the answer changes, so a re-read that found
/// the same bytes would still force a full transfer. The digest is over the
/// bytes actually being sent, so the validator means what a validator is
/// supposed to mean — this is the same answer you already have.
///
/// `brutex_core::blake3` is the repository's own primitive, the one `CLAUDE.md`
/// §3 rule 3 already uses for run identity. No new dependency.
///
/// # Cost
///
/// O(body) once per immutable census snapshot, vendor and wire format. The
/// wire cache retains both bytes and this digest, so unchanged GETs reuse the
/// encoding and conditional GETs need no new body pass. HEAD omits both.
pub(super) fn census_etag(body: &str) -> String {
    // `hex32` AND NOT A SECOND LOOP. This file already renders 32 bytes as hex
    // at its foot, under a test that pins the width — writing the same fold
    // again here is the duplicate-spelling this repository keeps removing, and
    // it would drift the first time either was touched.
    format!("\"{}\"", hex32(brutex_core::blake3::hash(body.as_bytes())))
}

/// Whether the reader already holds this exact answer.
///
/// `If-None-Match` is a COMMA-SEPARATED LIST and a reader may legitimately send
/// several, so this compares each in turn rather than the header as one string.
/// `*` is deliberately not honoured: it means "if any representation exists",
/// which for an endpoint that always has one would answer 304 forever.
fn none_match_hit(headers: &axum::http::HeaderMap, etag: &str) -> bool {
    headers
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|sent| sent.split(',').any(|one| one.trim() == etag))
}

pub(super) async fn store_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    asked_with: axum::http::HeaderMap,
    uri: axum::http::Uri,
    method: axum::http::Method,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    let head = method == axum::http::Method::HEAD;
    let result =
        // THE CENSUS POOL, NOT THE DETAIL POOL (DB-B51): the audit journal of
        // every audited route takes `detail::run`'s four slots, and ten `/db`
        // tabs opening at once starved both.
        match crate::detail::run_census(move || store_response(&site, &asked_with, &uri, &method))
            .await
        {
            Ok(response) => response,
            Err(why) => (
                if matches!(why, crate::detail::RunError::Saturated) {
                    axum::http::StatusCode::TOO_MANY_REQUESTS
                } else {
                    axum::http::StatusCode::SERVICE_UNAVAILABLE
                },
                census_response_headers(matches!(why, crate::detail::RunError::Saturated)),
                serde_json::json!({"error":format!("census read unavailable: {why:?}")})
                    .to_string()
                    .into(),
            ),
        };
    let mut response = result.into_response();
    if head {
        // An exact empty body makes Axum invent Content-Length: 0, which would
        // misdescribe the GET representation. Unknown size permits omission.
        *response.body_mut() =
            axum::body::Body::from_stream(axum::body::Body::empty().into_data_stream());
        response
            .headers_mut()
            .remove(axum::http::header::CONTENT_LENGTH);
    }
    response
}

fn census_response_headers(retry: bool) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    if retry {
        headers.insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("1"),
        );
    }
    headers
}

pub(super) fn store_response(
    site: &Site,
    asked_with: &axum::http::HeaderMap,
    uri: &axum::http::Uri,
    method: &axum::http::Method,
) -> (
    axum::http::StatusCode,
    axum::http::HeaderMap,
    axum::body::Bytes,
) {
    let query = uri.query().unwrap_or("");
    // REFUSED BY NAME, NEVER SUBSTITUTED. The `.unwrap_or(Vendor::Dhan)` that
    // stood here was unreachable for an ABSENT feed — `parse_vendor` answers
    // the empty string with Dhan itself — so the only input it ever caught was
    // a feed somebody NAMED and this build does not read. See the doc block
    // above for what answering it as Dhan cost.
    let asked = param(query, "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        // The census headers are deliberately NOT stamped: they describe a
        // feed's counter, and there is no feed here to have one. Only the
        // content type, so a browser parses the refusal as what it is.
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
        );
        return (
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            no_such_feed_json(&asked).into(),
        );
    };

    // FRESH, NOT THE STARTUP SNAPSHOT. See `census_now`.
    let format = match param(query, "encoding").as_str() {
        "" => store_wire::Format::Expanded,
        store_wire::ENCODING => store_wire::Format::Compact,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                census_response_headers(false),
                serde_json::json!({"error":"unrecognised census encoding"})
                    .to_string()
                    .into(),
            );
        }
    };
    // THIS FEED'S OWN CELL, NOTHING ELSE (D-3327 item 1). Another feed's
    // commit moves another stamp; this read is a hit while it does.
    let (_stamps, now) = vendor_census(site, feed);
    let census = now.census();
    // 503, FOR THE REASON `health` GIVES: a monitor reads the status code and
    // nothing else. The body stays an array so the readers that only want rows
    // are untouched.
    let code = if census_is_unreadable(census) {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    let behind = behind(site, feed);
    let mut headers = census_headers_with(census, Some(&behind));

    // HEAD is a header read, not a full census encoding whose body Axum drops.
    // Representation headers that require generating a body are omitted.
    if method == axum::http::Method::HEAD {
        return (code, headers, axum::body::Bytes::new());
    }
    // A REFRESH ASKS ONLY FOR WHAT CHANGED (WP-56; D-3327 item 3; DB-060):
    // `since=` names the generation the reader holds, and the answer is the
    // rows appended after it on this same wire. An unreadable census keeps
    // its 503 below; a delta is never cut from one.
    let since = param(query, "since");
    if !since.is_empty() && code == axum::http::StatusCode::OK {
        let (code, body) = delta_response(census, feed, format, &since, &param(query, "epoch"));
        // SAID IN A HEADER TOO, so a reader knows before it reads the body
        // whether this is a delta or the whole census an API older than this
        // one answers the same request with.
        headers.insert(
            axum::http::HeaderName::from_static(CENSUS_SINCE_HEADER),
            note_header(&since),
        );
        return (code, headers, body.into());
    }
    let row = census.map(std::slice::from_ref).unwrap_or_default();
    let slot = now.slot();
    let encoded = match site.census_wire.get(slot, feed, format, || {
        let entries = slot.entries();
        match format {
            store_wire::Format::Expanded => Ok(store_body(row, &entries, feed)),
            store_wire::Format::Compact => store_wire::compact(row, &entries, feed),
        }
    }) {
        Ok(encoded) => encoded,
        Err(why) => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                headers,
                serde_json::json!({"error":why}).to_string().into(),
            );
        }
    };
    let etag = &encoded.etag;
    // `from_str` cannot fail on this value — it is quotes and hex — but the
    // header map takes a `Result` and a silently dropped validator would mean a
    // reader never gets a 304 and never knows why. Refusing to guess: if it
    // could not be built the response simply carries no validator, which is the
    // pre-existing behaviour rather than a new failure.
    if let Ok(value) = axum::http::HeaderValue::from_str(etag) {
        headers.insert(axum::http::header::ETAG, value);
    }

    // A MATCH IS A 304 AND NO BODY — but ONLY on a 200.
    //
    // `census_is_unreadable` answers 503 with an empty array, and two different
    // damaged reads produce the same bytes and therefore the same tag. Serving
    // 304 for that would tell a reader "nothing changed" about a state whose
    // whole point is that the counter is broken — and `census_headers` carries
    // the reason in `x-brutex-census-note`, which a 304 body would still deliver
    // but a caching reader might not re-read. A failure keeps its status.
    if code == axum::http::StatusCode::OK && none_match_hit(asked_with, etag) {
        return (
            axum::http::StatusCode::NOT_MODIFIED,
            headers,
            axum::body::Bytes::new(),
        );
    }

    (code, headers, encoded.body)
}

/* ======================================================================
THE DELTA: `/store.json?...&since=<generation>` (WP-56; D-3327 item 3;
DB-022, DB-060, DB-G59, DB-G70)
----------------------------------------------------------------------
A page that holds the census refreshed it by downloading it again: 33 MB
and an O(keys log keys) sort and encode on this side for each commit an
import made, every five seconds while `/ingest` watched one. A refresh
now names the generation it holds and is answered with the rows appended
after it, on the SAME wire (`store_wire::compact_delta`), so one decoder
reads a whole census and a delta.

WHAT A GENERATION NAMES. A manifest is an append-only log, so (file,
generation) names a prefix of it. For a single file the file is the
vendor's one manifest. For a partitioned census it is each segment's
current epoch file (D-3316), so `since` carries one generation per part
in the spelling of `x-brutex-census-part-generations`, and `epoch` the
epochs the reader read them from, in the spelling of
`x-brutex-census-epoch`. A different epoch is a different log: refused,
and the reader loads the census whole.

WHAT IS REFUSED, AND NEVER GUESSED: a `since` this census's shape does
not take (409 `since_shape`), an epoch that moved (409 `epoch_changed`),
a generation past a part's newest (409 `since_in_future`), and one the
census no longer holds the appends after (410 `since_too_old`: it was
read whole since). A reader that is refused reads the census whole.

WHICH ROWS. Every key appended after the generation, and for each the
same series' NEXT month when held: a row carries its previous month's
change (`prev_chg_bps`), so a month whose closes moved changes the row
after it too. In the order the whole body lists them.

COST. `Part::changed_since` is a binary search per tail level; the answer
is two probes per row. Nothing here walks the census or sorts it.
====================================================================== */

/// What a `since=` named: one generation, or one per partition.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Since {
    /// A single-file census's one generation.
    One(u64),
    /// `INDEX=<g>,CASH=<g>,FNO=<g>`: each partition's own.
    Parts(Vec<(brutex_core::instrument::Segment, u64)>),
}

/// Reads a `since`, or `None` for anything that is neither spelling. A
/// generation is digits only (`u64::from_str` would take a sign).
fn parse_since(text: &str) -> Option<Since> {
    let generation = |word: &str| -> Option<u64> {
        (!word.is_empty() && word.bytes().all(|b| b.is_ascii_digit()))
            .then(|| word.parse().ok())
            .flatten()
    };
    if let Some(one) = generation(text) {
        return Some(Since::One(one));
    }
    let mut parts: Vec<(brutex_core::instrument::Segment, u64)> = Vec::new();
    for pair in text.split(',') {
        let (segment, word) = pair.split_once('=')?;
        let segment = brutex_core::instrument::Segment::parse(segment).ok()?;
        if parts.iter().any(|(held, _)| *held == segment) {
            return None;
        }
        parts.push((segment, generation(word)?));
    }
    Some(Since::Parts(parts))
}

/// `{"error": <code>, "why": <sentence>, ...extra}`: a delta's refusal.
fn delta_refusal(code: &str, why: &str, extra: &[(&str, String)]) -> String {
    let mut out = format!(
        r#"{{"error":{},"why":{}"#,
        render::json_string(code),
        render::json_string(why)
    );
    for (name, value) in extra {
        let _ = write!(out, ",{}:{value}", render::json_string(name));
    }
    out.push('}');
    out
}

/// The month after `month`, or `None` past the last the store addresses.
///
/// Which WIRE ROW a changed month also changes (the next month's row carries
/// this month's change as `prev_chg_bps`); no bar of a later month is read to
/// compute anything about an earlier one.
fn month_after(month: store::path::YearMonth) -> Option<store::path::YearMonth> {
    let (year, ordinal) = if month.month() < 12 {
        (month.year(), month.month() + 1)
    } else {
        (month.year().checked_add(1)?, 1)
    };
    store::path::YearMonth::new(year, ordinal).ok()
}

/// A delta's refusal: its status and its body.
type Refused = (axum::http::StatusCode, String);

/// EACH PART AND THE GENERATION ASKED OF IT, or the shape is refused: a
/// census migrated to partitions (or back) since the reader read it is not
/// the log the reader's generation names.
fn asked_parts<'a>(
    manifest: &'a census::Manifests,
    since: &Since,
) -> Result<Vec<(&'a census::Part, u64)>, Refused> {
    let conflict = axum::http::StatusCode::CONFLICT;
    let parts = manifest.parts();
    Ok(match since {
        Since::One(generation) if !manifest.is_partitioned() => {
            parts.iter().map(|part| (part, *generation)).collect()
        }
        Since::Parts(named)
            if manifest.is_partitioned()
                && named.len() == parts.len()
                && parts.iter().all(|part| {
                    named
                        .iter()
                        .any(|(segment, _)| Some(*segment) == part.segment())
                }) =>
        {
            parts
                .iter()
                .filter_map(|part| {
                    named
                        .iter()
                        .find(|(segment, _)| Some(*segment) == part.segment())
                        .map(|(_, generation)| (part, *generation))
                })
                .collect()
        }
        Since::One(_) | Since::Parts(_) => {
            return Err((
                conflict,
                delta_refusal(
                    "since_shape",
                    if manifest.is_partitioned() {
                        "this census is partitioned: a since names every part's generation, \
                         as x-brutex-census-part-generations spells them; read the census whole"
                    } else {
                        "this census is a single file: a since is its one generation, \
                         x-brutex-census-generation; read the census whole"
                    },
                    &[],
                ),
            ));
        }
    })
}

/// The rows a delta since `since` carries, in the whole body's order, or the
/// refusal.
fn delta_entries(
    manifest: &census::Manifests,
    since: &Since,
) -> Result<Vec<(census::Series, store::path::YearMonth)>, Refused> {
    let conflict = axum::http::StatusCode::CONFLICT;
    let asked = asked_parts(manifest, since)?;
    let mut seen: std::collections::HashSet<pull::manifest::EntryKey> =
        std::collections::HashSet::new();
    for (part, generation) in asked {
        let name = part.segment().map_or("", |segment| segment.as_str());
        let newest = part.header().generation;
        let extra = |floor: Option<u64>| {
            let mut extra = vec![
                ("part", render::json_string(name)),
                ("since", generation.to_string()),
                ("generation", newest.to_string()),
            ];
            if let Some(floor) = floor {
                extra.push(("floor", floor.to_string()));
            }
            extra
        };
        if generation > newest {
            return Err((
                conflict,
                delta_refusal(
                    "since_in_future",
                    "that generation is past this part's newest, so it is not one this \
                     census published; read the census whole",
                    &extra(None),
                ),
            ));
        }
        let Some(changes) = part.changed_since(generation) else {
            return Err((
                axum::http::StatusCode::GONE,
                delta_refusal(
                    "since_too_old",
                    "the census no longer holds the appends after that generation (it was \
                     read whole since); read the census whole",
                    &extra(Some(part.since_floor())),
                ),
            ));
        };
        for (_, key) in changes.iter() {
            if !seen.insert(*key) {
                continue;
            }
            // THE NEXT MONTH'S ROW MOVES WITH THIS ONE: it carries this
            // month's change as its `prev_chg_bps`.
            if let Some(next) = month_after(key.month) {
                let next = pull::manifest::EntryKey {
                    month: next,
                    ..*key
                };
                if manifest.held(&next).is_some() {
                    let _ = seen.insert(next);
                }
            }
        }
    }
    let mut out: Vec<(census::Series, store::path::YearMonth)> = seen
        .iter()
        .map(|key| (census::Series::of(key), key.month))
        .collect();
    // THE WHOLE BODY'S ORDER (`census::held_entries`): newest month first,
    // then the series' own. A reader places a new row among the rows it holds
    // by this order, so the delta is sent in it.
    out.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(out)
}

/// The delta of `census` since `since` (read from `epoch`): the status and
/// the body `/store.json?feed=<its vendor>&encoding=census-tuples-v1&since=`
/// answers with. What the route calls, and what gate 8 times
/// (`benches/ratio/census.rs`, `deltas_are_flat`).
#[must_use]
pub fn census_delta(census: &census::VendorCensus, since: &str, epoch: &str) -> (u16, String) {
    let (code, body) = delta_response(
        Some(census),
        census.vendor,
        store_wire::Format::Compact,
        since,
        epoch,
    );
    (code.as_u16(), body)
}

/// The delta's answer: its status and body. See the block above.
fn delta_response(
    census: Option<&census::VendorCensus>,
    feed: Vendor,
    format: store_wire::Format,
    since: &str,
    epoch: &str,
) -> (axum::http::StatusCode, String) {
    let bad = axum::http::StatusCode::BAD_REQUEST;
    if format != store_wire::Format::Compact {
        return (
            bad,
            delta_refusal(
                "bad_request",
                "since needs encoding=census-tuples-v1: a delta is the tuples wire",
                &[],
            ),
        );
    }
    let Some(asked) = parse_since(since) else {
        return (
            bad,
            delta_refusal(
                "bad_request",
                "since is one generation, or SEGMENT=generation for every part, comma \
                 separated, as x-brutex-census-part-generations spells them",
                &[("since", render::json_string(since))],
            ),
        );
    };
    let held = census.and_then(|census| match census.state {
        census::Census::Held { ref manifest } => Some((census, manifest)),
        census::Census::Absent | census::Census::Unreadable { .. } => None,
    });
    let Some((census, manifest)) = held else {
        return (
            axum::http::StatusCode::CONFLICT,
            delta_refusal(
                "since_shape",
                "this feed holds no census now, so no generation is held; read it whole",
                &[],
            ),
        );
    };
    // THE EPOCHS THE READER READ ITS GENERATIONS FROM. A partitioned since
    // must name them: a generation alone does not name a file (D-3316).
    let now = epochs(census);
    if manifest.is_partitioned() && epoch.is_empty() {
        return (
            bad,
            delta_refusal(
                "bad_request",
                "a since over a partitioned census needs epoch=, the x-brutex-census-epoch \
                 the generations were read with",
                &[],
            ),
        );
    }
    if epoch != now {
        return (
            axum::http::StatusCode::CONFLICT,
            delta_refusal(
                "epoch_changed",
                "the census is in another epoch than the one that since was read from (a \
                 compaction or a recount wrote a new one); read the census whole",
                &[
                    ("epoch", render::json_string(epoch)),
                    ("now", render::json_string(&now)),
                ],
            ),
        );
    }
    let entries = match delta_entries(manifest, &asked) {
        Ok(entries) => entries,
        Err(refused) => return refused,
    };
    match store_wire::compact_delta(
        std::slice::from_ref(census),
        &entries,
        feed,
        since,
        (manifest.keys(), manifest.total_rows()),
    ) {
        Ok(body) => (axum::http::StatusCode::OK, body),
        Err(why) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error":why}).to_string(),
        ),
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
    use crate::server::percentage_tests::{census_of, month, priced, series, series_at};
    use brutex_core::instrument::Segment;
    use pull::manifest::Closes;
    use store::path::Timeframe;

    /// The one row `store_body` produced, so an assertion names a field rather
    /// than a substring of the whole array.
    fn only_row(body: &str) -> String {
        let inner = body
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .expect("a JSON array");
        assert!(
            !inner.contains("},{"),
            "this helper is for a single-row body: {body}"
        );
        inner.to_owned()
    }

    /// **A STORE ROW NAMES THE RUNG IT IS STORED AT, NEVER A FIXED WORD.**
    ///
    /// `store_body` wrote `"timeframe":"1min"` as a literal on every row while
    /// `Series` carried the real `Timeframe`. Every test in this module built
    /// its series at `MINUTE_1`, so all 321 of them agreed with the literal and
    /// none of them could tell the difference — the writer was reverted to the
    /// literal and the suite stayed green.
    ///
    /// This test builds a `DAY_1` series specifically so it cannot: it asserts
    /// the row says `1day` AND that it does not say `1min`, which is what makes
    /// reverting the writer a failure rather than a wash.
    #[test]
    fn a_store_row_names_the_rung_it_is_stored_at_and_never_a_fixed_word() {
        let daily = series_at(Segment::Index, "NIFTY", Timeframe::DAY_1);
        let at = month(2026, 7);
        let census = census_of(&[(daily, at, 23, priced(10_000_000, 10_125_000))]);
        let body = only_row(&store_body(&[census], &[(daily, at)], Vendor::Groww));
        assert!(
            body.contains(r#""timeframe":"1day""#),
            "a 1day series must say 1day: {body}"
        );
        assert!(
            !body.contains(r#""timeframe":"1min""#),
            "a 1day series must not say 1min: {body}"
        );

        // The other rung, from the same writer, so the assertion above is
        // reading the field and not a word that happens to be everywhere.
        let minute = series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1);
        let census = census_of(&[(minute, at, 8_250, priced(10_000_000, 10_125_000))]);
        let body = only_row(&store_body(&[census], &[(minute, at)], Vendor::Groww));
        assert!(
            body.contains(r#""timeframe":"1min""#),
            "a 1min series must say 1min: {body}"
        );
    }

    /// **A MONTH WITH NO CLOSE RECORDED IS UNKNOWN, AND NEVER ZERO.**
    ///
    /// Every version-1 census entry reads back this way, and so does every
    /// month this build committed before it read a bar file — D-0067. On the
    /// wire it is `null` and a code, so the browser cannot render it as a
    /// number by accident.
    #[test]
    fn a_month_with_no_close_recorded_is_unknown_and_never_zero() {
        assert_eq!(
            month_change(Segment::Index, "NIFTY", Some(Closes::UNKNOWN)),
            Err(Unknown::NotRecorded)
        );
        let nifty = series(Segment::Index, "NIFTY");
        let at = month(2026, 7);
        let body = store_body(
            &[census_of(&[(nifty, at, 8_250, Closes::UNKNOWN)])],
            &[(nifty, at)],
            Vendor::Groww,
        );
        let row = only_row(&body);
        assert!(
            row.contains(r#""chg_bps":null,"chg_why":"not_recorded""#),
            "unknown on the wire is null and a reason: {row}"
        );
        assert!(
            !row.contains(r#""chg_bps":0"#),
            "and it is never a zero: {row}"
        );
    }

    /// **AN INSTRUMENT'S FIRST MONTH HAS A CHANGE AND NO PREVIOUS CHANGE.**
    ///
    /// This is the property that decided the reading. Because `Δ %` is the
    /// month's own first close to its own last close, it is defined for every
    /// held month including the first — and only the "previous" column has a
    /// missing-neighbour case. That case is `no_earlier_month`, spelled out,
    /// and it is not `0.00%`.
    #[test]
    fn an_instruments_first_month_has_a_change_and_no_previous_change() {
        let nifty = series(Segment::Index, "NIFTY");
        let first = month(2026, 6);
        let second = month(2026, 7);
        let census = census_of(&[
            (nifty, first, 8_250, priced(10_000, 10_125)),
            (nifty, second, 8_250, priced(10_000, 10_500)),
        ]);
        let body = store_body(
            &census_slice(&census),
            &[(nifty, second), (nifty, first)],
            Vendor::Groww,
        );

        assert!(
            body.contains(r#""chg_bps":500,"chg_why":null,"prev_chg_bps":125,"prev_chg_why":null"#),
            "July renders its own +5.00% and June's +1.25% beside it: {body}"
        );
        assert!(
            body.contains(r#""chg_bps":125,"chg_why":null,"prev_chg_bps":null,"prev_chg_why":"no_earlier_month""#),
            "June has its own number and NO previous — not a zero: {body}"
        );
        assert!(
            !body.contains(r#""prev_chg_bps":0"#),
            "a missing neighbour is never rendered as no change: {body}"
        );
    }

    /// One census as the slice `store_body` takes.
    fn census_slice(census: &census::VendorCensus) -> [census::VendorCensus; 1] {
        [census.clone()]
    }

    /// **A ROW AND ITS NEIGHBOUR COST TWO PROBES AND NO FILE.**
    ///
    /// Two things are asserted, and each is the falsifiable half of a claim the
    /// doc comments make.
    ///
    /// **No file.** The census is built in memory and its `path` names a
    /// directory that does not exist. Every number below therefore came out of
    /// the manifest. A change that reached for a bar file to price a month
    /// would fail here rather than pass slowly, which is the difference between
    /// this design and the 17.5 GB route D-0067 measured and rejected.
    ///
    /// **One month back, never a search.** June and August are held and July is
    /// not. August's previous change is `no_earlier_month` — it does NOT walk
    /// back to June. That is what makes the lookup a probe: a scan would find
    /// June, print a number spanning two months, and label it as one.
    #[test]
    fn a_row_and_its_neighbour_cost_two_probes_and_no_file() {
        let nifty = series(Segment::Index, "NIFTY");
        let june = month(2026, 6);
        let august = month(2026, 8);
        let census = census_of(&[
            (nifty, june, 8_250, priced(10_000, 10_125)),
            (nifty, august, 8_250, priced(10_000, 10_200)),
        ]);
        assert!(
            !census.path.exists(),
            "the fixture must not have a file behind it, or it proves nothing"
        );
        let body = store_body(
            &census_slice(&census),
            &[(nifty, august), (nifty, june)],
            Vendor::Groww,
        );
        assert!(
            body.contains(r#""chg_bps":200,"chg_why":null,"prev_chg_bps":null,"prev_chg_why":"no_earlier_month""#),
            "August reads July, finds nothing, and stops — it does not reach June: {body}"
        );
        assert!(
            !body.contains(r#""chg_bps":200,"chg_why":null,"prev_chg_bps":125"#),
            "reaching past the gap would print a June number on an August row: {body}"
        );
    }

    /// **EVERY ROW CARRIES A NUMBER OR A NAMED REASON, AND NEVER BOTH.**
    ///
    /// The wire shape is total: both keys of each pair are present on every
    /// row, and exactly one of them is non-`null`. A key that is sometimes
    /// absent makes "unknown" and "this build has no such field" the same thing
    /// in the browser, and a row carrying both would be a number with an
    /// excuse — which is the shape `CLAUDE.md` §4 bans.
    #[test]
    fn every_row_carries_a_number_or_a_named_reason_and_never_both() {
        let nifty = series(Segment::Index, "NIFTY");
        let reliance = series(Segment::Cash, "RELIANCE");
        let june = month(2026, 6);
        let july = month(2026, 7);
        let census = census_of(&[
            (nifty, june, 8_250, priced(10_000, 10_125)),
            (nifty, july, 8_250, Closes::UNKNOWN),
            (reliance, june, 8_250, priced(295_050, 310_025)),
            (reliance, july, 8_250, priced(310_025, 62_005)),
        ]);
        let body = store_body(
            &census_slice(&census),
            &[
                (nifty, july),
                (reliance, july),
                (nifty, june),
                (reliance, june),
            ],
            Vendor::Groww,
        );

        // Four rows, four of each key, and no key ever missing.
        for field in [
            "\"chg_bps\":",
            "\"chg_why\":",
            "\"prev_chg_bps\":",
            "\"prev_chg_why\":",
        ] {
            assert_eq!(
                body.matches(field).count(),
                4,
                "{field} appears on every row exactly once: {body}"
            );
        }
        // A number and a reason never co-occur: every `_bps` that is a number
        // is followed by a `_why` of null, and vice versa.
        assert_eq!(
            body.matches("_bps\":null").count() + body.matches("_why\":null").count(),
            8,
            "one of each pair is null on every row, and only one: {body}"
        );

        assert!(
            body.contains(
                r#""chg_bps":null,"chg_why":"not_recorded","prev_chg_bps":125,"prev_chg_why":null"#
            ),
            "an unpriced index month is unknown and its priced neighbour is not: {body}"
        );
        // THE 80% FALL D-0018 IS ABOUT. ₹3,100.25 to ₹620.05 is a 1:5 split
        // read as an -8,000 bp crash, and it is exactly the number this page
        // must never print.
        assert!(
            body.contains(r#""chg_bps":null,"chg_why":"corporate_action_unverified","prev_chg_bps":null,"prev_chg_why":"corporate_action_unverified""#),
            "the equity is refused in both columns, by name: {body}"
        );
        assert!(
            !body.contains("-8000"),
            "the fabricated crash never reaches the wire: {body}"
        );
    }

    /// **A FEED WITH NO CENSUS IS AN EMPTY ARRAY, NOT A ROW OF ZEROES.**
    ///
    /// The page renders "nothing stored for this feed" off an empty array and
    /// names where the rows actually are. It has no way to distinguish a zeroed
    /// row from a real one, so there must never be a zeroed row.
    #[test]
    fn a_feed_with_no_census_is_an_empty_array() {
        let nifty = series(Segment::Index, "NIFTY");
        let at = month(2026, 7);
        let census = census_of(&[(nifty, at, 8_250, priced(10_000, 10_125))]);
        assert_eq!(
            store_body(&census_slice(&census), &[(nifty, at)], Vendor::Dhan),
            "[]",
            "a feed this store has never held answers with nothing"
        );
        assert_eq!(
            store_body(&[], &[(nifty, at)], Vendor::Groww),
            "[]",
            "and so does a store with no censuses at all"
        );
        // A MONTH THE CENSUS DOES NOT HOLD IS SKIPPED, not emitted empty.
        assert_eq!(
            store_body(
                &census_slice(&census),
                &[(nifty, month(2026, 5))],
                Vendor::Groww
            ),
            "[]"
        );
    }

    /// One GDFL row of NIFTY 1-second bars, 2019-11, at the census's next
    /// generation: what the header tests below write. Invented counts.
    fn gdfl_nifty(manifest: &mut pull::manifest::Manifest, rows: u64) {
        let _append = manifest
            .record_held(pull::manifest::Held::new(
                pull::manifest::Entry {
                    key: series_at(Segment::Index, "NIFTY", Timeframe::SECOND_1)
                        .at(month(2019, 11)),
                    rows,
                    first_ts_micros: 1_572_580_800_000_000,
                    last_ts_micros: 1_572_603_000_000_000,
                },
                Closes::UNKNOWN,
            ))
            .expect("records");
    }

    /// `/store.json`'s answer to `method` for `query`, over `site`.
    fn ask(
        site: &Site,
        method: &axum::http::Method,
        query: &str,
    ) -> (axum::http::StatusCode, axum::http::HeaderMap) {
        let uri: axum::http::Uri = format!("/store.json?{query}").parse().expect("a uri");
        let (code, headers, _body) =
            store_response(site, &axum::http::HeaderMap::new(), &uri, method);
        (code, headers)
    }

    fn header<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> &'a str {
        headers
            .get(name)
            .unwrap_or_else(|| panic!("{name} is always present: {headers:?}"))
            .to_str()
            .expect("ascii")
    }

    /// **A CENSUS BEHIND ITS IMPORT SAYS `behind` ON `/store.json` AND ITS
    /// HEAD, WITH HOW MANY DAYS AND THE FIRST, AND A RECOUNT CLEARS IT DAY BY
    /// DAY** (D-3327 item 2; DB-G03, DB-G07, DB-G11, DB-G60, DB-G78).
    ///
    /// The operator's census refused every options day from 2019-11-14 while
    /// these headers said `held` with an empty degraded header. Here the
    /// journal closes two options days census-refused (the D-3317 shape, with
    /// `stage=census`), one day `done` between them and a day refused at the
    /// vendor, which is not a census day. The state is `behind`, two days,
    /// from the first; `x-brutex-census-degraded` stays empty because the
    /// census is not damaged. A `recounted` line for the first day leaves one
    /// day, from the second. Zerodha keeps no import journal, so its
    /// behind-days header is empty, never `0`. Every new header is present on
    /// HEAD as on GET.
    #[test]
    fn a_census_behind_its_import_says_so_and_a_recount_clears_it() {
        let dir = crate::server::tests::agreeing("store-behind");
        let site = crate::server::tests::site("store-behind", &dir);
        let mut manifest =
            pull::manifest::Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
        gdfl_nifty(&mut manifest, 375);
        gdfl_nifty(&mut manifest, 750);
        std::fs::write(
            pull::manifest::manifest_path(&site.store_root, Vendor::Gdfl),
            manifest.image(),
        )
        .expect("a census on disk");
        let journal = pull::gdfl_import::journal_path(&site.store_root);
        std::fs::create_dir_all(journal.parent().expect("imports/")).expect("mkdir");
        std::fs::write(
            &journal,
            "\
begin options 2019-11-13 *
done options 2019-11-13 * definition=2 files=4 seconds=40 failures=0 late=0 late_unresolved=0 max_back_s=0
begin options 2019-11-15 *
incomplete options 2019-11-15 * definition=2 files=9 seconds=99 failures=1 late=0 late_unresolved=0 max_back_s=0 stage=census reason=too_many_entries run=1791300000
begin options 2019-11-18 *
done options 2019-11-18 * definition=2 files=4 seconds=40 failures=0 late=0 late_unresolved=0 max_back_s=0
begin options 2019-11-19 *
incomplete options 2019-11-19 * definition=2 files=9 seconds=99 failures=1 late=0 late_unresolved=0 max_back_s=0 stage=census reason=too_many_entries run=1791300000
begin indices 2022-03-07 *
refused indices 2022-03-07 * definition=2 stage=vendor reason=price_refused failures=2 run=1791300000
",
        )
        .expect("a journal");

        for method in [axum::http::Method::HEAD, axum::http::Method::GET] {
            let (code, headers) = ask(&site, &method, "feed=gdfl&encoding=census-tuples-v1");
            assert_eq!(code, axum::http::StatusCode::OK, "{method}");
            assert_eq!(header(&headers, CENSUS_STATE_HEADER), "behind", "{method}");
            assert_eq!(header(&headers, CENSUS_DEGRADED_HEADER), "", "not damaged");
            assert_eq!(header(&headers, CENSUS_BEHIND_DAYS_HEADER), "2", "{method}");
            assert_eq!(header(&headers, CENSUS_BEHIND_FROM_HEADER), "2019-11-15");
            assert_eq!(header(&headers, CENSUS_GENERATION_HEADER), "2");
            assert_eq!(
                header(&headers, CENSUS_CAPACITY_HEADER),
                pull::manifest::MAX_ENTRIES.to_string(),
                "a single file's capacity is the bound it was written under"
            );
            assert_eq!(
                header(&headers, CENSUS_HEADROOM_HEADER),
                (pull::manifest::MAX_ENTRIES - 2).to_string()
            );
            assert_eq!(header(&headers, CENSUS_EPOCH_HEADER), "", "a single file");
            assert_eq!(
                header(&headers, CENSUS_PART_GENERATIONS_HEADER),
                "",
                "a single file's one generation is the vendor-wide one"
            );
            let why = header(&headers, CENSUS_REASON_HEADER);
            assert!(
                why.starts_with("behind: ") && why.contains("2019-11-15"),
                "{why}"
            );
        }

        // A RECOUNT OF THE FIRST DAY, appended as `cli census-recount` writes it.
        let mut text = std::fs::read_to_string(&journal).expect("the journal");
        text.push_str("recounted options 2019-11-15 * definition=2 months=3 run=1791300001\n");
        std::fs::write(&journal, text).expect("appended");
        let (_, headers) = ask(&site, &axum::http::Method::HEAD, "feed=gdfl");
        assert_eq!(header(&headers, CENSUS_STATE_HEADER), "behind");
        assert_eq!(header(&headers, CENSUS_BEHIND_DAYS_HEADER), "1");
        assert_eq!(header(&headers, CENSUS_BEHIND_FROM_HEADER), "2019-11-19");

        // AND THE LAST: the census is held, the journal still measured.
        let mut text = std::fs::read_to_string(&journal).expect("the journal");
        text.push_str("recounted options 2019-11-19 * definition=2 months=3 run=1791300002\n");
        std::fs::write(&journal, text).expect("appended");
        let (_, headers) = ask(&site, &axum::http::Method::HEAD, "feed=gdfl");
        assert_eq!(header(&headers, CENSUS_STATE_HEADER), "held");
        assert_eq!(header(&headers, CENSUS_BEHIND_DAYS_HEADER), "0");
        assert_eq!(header(&headers, CENSUS_BEHIND_FROM_HEADER), "");
        assert_eq!(header(&headers, CENSUS_REASON_HEADER), "");

        // ZERODHA KEEPS NO IMPORT JOURNAL: not measured, said empty.
        let (_, headers) = ask(&site, &axum::http::Method::HEAD, "feed=zerodha");
        assert_eq!(header(&headers, CENSUS_STATE_HEADER), "absent");
        assert_eq!(header(&headers, CENSUS_BEHIND_DAYS_HEADER), "");
        assert_eq!(header(&headers, CENSUS_GENERATION_HEADER), "");
        assert_eq!(header(&headers, CENSUS_CAPACITY_HEADER), "");
    }

    /// **A MIGRATED CENSUS NEAR ITS OWN CAPACITY SAYS `capped`, WITH THE
    /// CAPACITY ITS HEADER DECLARES, ITS HEADROOM AND EVERY EPOCH** (D-3327
    /// item 2, D-3316; DB-G07, DB-G08, DB-G78).
    ///
    /// The capacity is the FNO epoch's own, not `MAX_ENTRIES`; the headroom
    /// is below `CAPPED_HEADROOM`; the degraded header stays empty, because a
    /// census near its capacity is the current one and complete for every
    /// month it counts (refusing to draw it would hide them).
    #[test]
    fn a_migrated_census_near_its_capacity_says_capped_with_its_own_numbers() {
        let dir = crate::server::tests::agreeing("store-capped");
        let site = crate::server::tests::site("store-capped", &dir);
        let capacity = census::CAPPED_HEADROOM + 1;
        let mut fno =
            pull::manifest::Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, capacity)
                .expect("a partition");
        let key = census::Series {
            contract: Some(
                brutex_core::instrument::Contract::parse("2019-11-28-1200000-CE")
                    .expect("a contract"),
            ),
            ..series_at(Segment::Fno, "NIFTY", Timeframe::SECOND_1)
        }
        .at(month(2019, 11));
        for rows in [10, 20] {
            let _append = fno
                .record_held(pull::manifest::Held::new(
                    pull::manifest::Entry {
                        key,
                        rows,
                        first_ts_micros: 1_572_580_800_000_000,
                        last_ts_micros: 1_572_580_800_000_000 + i64::try_from(rows).expect("small"),
                    },
                    Closes::UNKNOWN,
                ))
                .expect("records");
        }
        std::fs::create_dir_all(pull::manifest::segment_dir(
            &site.store_root,
            Vendor::Gdfl,
            Segment::Fno,
        ))
        .expect("mkdir");
        std::fs::write(
            pull::manifest::epoch_path(&site.store_root, Vendor::Gdfl, Segment::Fno, 1),
            fno.image(),
        )
        .expect("an epoch on disk");

        let (code, headers) = ask(&site, &axum::http::Method::HEAD, "feed=gdfl");
        assert_eq!(code, axum::http::StatusCode::OK);
        assert_eq!(header(&headers, CENSUS_STATE_HEADER), "capped");
        assert_eq!(header(&headers, CENSUS_DEGRADED_HEADER), "", "not damaged");
        assert_eq!(
            header(&headers, CENSUS_CAPACITY_HEADER),
            capacity.to_string()
        );
        assert_eq!(
            header(&headers, CENSUS_HEADROOM_HEADER),
            (capacity - 2).to_string()
        );
        assert_eq!(
            header(&headers, CENSUS_EPOCH_HEADER),
            "INDEX=none,CASH=none,FNO=000001"
        );
        // EACH PART'S OWN GENERATION, spelled as the epochs are, and the
        // vendor-wide generation their sum (WP-56; DB-B56, DB-B67): the value
        // a facets `since` takes is one of these, never the sum.
        let parts = header(&headers, CENSUS_PART_GENERATIONS_HEADER);
        assert!(
            parts.starts_with("INDEX=")
                && parts.contains(",CASH=")
                && parts.ends_with(&format!(",FNO={}", fno.header().generation)),
            "{parts}"
        );
        let sum: u64 = parts
            .split(',')
            .map(|part| {
                part.split_once('=')
                    .expect("SEGMENT=generation")
                    .1
                    .parse::<u64>()
                    .expect("a generation")
            })
            .sum();
        assert_eq!(header(&headers, CENSUS_GENERATION_HEADER), sum.to_string());
        assert_eq!(
            header(&headers, CENSUS_BEHIND_DAYS_HEADER),
            "0",
            "no journal yet"
        );
        let why = header(&headers, CENSUS_REASON_HEADER);
        assert!(why.starts_with("capped: ") && why.contains("FNO"), "{why}");
    }

    /* ------------------------------------------------------------------
    THE DELTA (WP-56; D-3327 item 3; DB-022, DB-060). Every price and
    count below is invented for the fixture.
    ------------------------------------------------------------------ */

    /// A GDFL census on disk in one shape, appended to in place the way the
    /// writer appends (each entry at its offset, then the header slot), and
    /// read as the per-vendor cache reads it.
    struct Disk {
        root: std::path::PathBuf,
        files: Vec<(std::path::PathBuf, pull::manifest::Manifest)>,
    }

    impl Disk {
        fn root_of(name: &str) -> std::path::PathBuf {
            let root = crate::scratch::path(&format!("store-delta-{name}"));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("manifest")).expect("mkdir");
            root
        }

        /// One `manifest/gdfl.man`.
        fn single(name: &str, rows: &[pull::manifest::Held]) -> Self {
            let root = Self::root_of(name);
            let mut manifest =
                pull::manifest::Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
            for row in rows {
                let _append = manifest.record_held(*row).expect("records");
            }
            let path = pull::manifest::manifest_path(&root, Vendor::Gdfl);
            std::fs::write(&path, manifest.image()).expect("written");
            Self {
                root,
                files: vec![(path, manifest)],
            }
        }

        /// One epoch-1 partition per segment named, each holding its rows.
        fn partitioned(name: &str, parts: &[(Segment, &[pull::manifest::Held])]) -> Self {
            let root = Self::root_of(name);
            let mut files = Vec::new();
            for (segment, rows) in parts {
                let mut manifest =
                    pull::manifest::Manifest::partition_genesis(Vendor::Gdfl, *segment, 4_096)
                        .expect("a partition");
                for row in *rows {
                    let _append = manifest.record_held(*row).expect("records");
                }
                std::fs::create_dir_all(pull::manifest::segment_dir(&root, Vendor::Gdfl, *segment))
                    .expect("mkdir");
                let path = pull::manifest::epoch_path(&root, Vendor::Gdfl, *segment, 1);
                std::fs::write(&path, manifest.image()).expect("an epoch on disk");
                files.push((path, manifest));
            }
            Self { root, files }
        }

        /// Appends `rows` to the file that holds their segment.
        fn append(&mut self, rows: &[pull::manifest::Held]) {
            use std::os::unix::fs::FileExt as _;
            for row in rows {
                let (path, manifest) = self
                    .files
                    .iter_mut()
                    .find(|(_, manifest)| {
                        manifest
                            .segment()
                            .is_none_or(|segment| segment == row.entry.key.segment)
                    })
                    .expect("a file for the segment");
                let file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(path)
                    .expect("open");
                let append = manifest.record_held(*row).expect("records");
                file.write_all_at(&append.bytes, append.offset)
                    .expect("entry");
                file.write_all_at(&append.commit.bytes, append.commit.offset)
                    .expect("slot");
            }
        }

        fn read(&self, previous: Option<&census::VendorCensus>) -> census::VendorCensus {
            census::read_vendor_after(&self.root, Vendor::Gdfl, previous)
        }
    }

    impl Drop for Disk {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// One held row of `series` in `at`: `rows` bars, closing `first` to
    /// `last` paisa.
    fn held_at(
        series: census::Series,
        at: store::path::YearMonth,
        rows: u64,
        (first, last): (i64, i64),
    ) -> pull::manifest::Held {
        let (from, _) = at.ist_bounds_micros();
        pull::manifest::Held::new(
            pull::manifest::Entry {
                key: series.at(at),
                rows,
                first_ts_micros: from,
                last_ts_micros: from + i64::try_from(rows).expect("small"),
            },
            priced(first, last),
        )
    }

    fn contract(symbol: &str, contract: &str) -> census::Series {
        census::Series {
            contract: Some(brutex_core::instrument::Contract::parse(contract).expect("a contract")),
            ..series_at(Segment::Fno, symbol, Timeframe::SECOND_1)
        }
    }

    /// The whole body of `census`, as `/store.json` encodes it.
    fn whole(census: &census::VendorCensus) -> serde_json::Value {
        let one = std::slice::from_ref(census);
        serde_json::from_str(
            &store_wire::compact(one, &census::held_entries(one), Vendor::Gdfl).expect("encodes"),
        )
        .expect("json")
    }

    /// One wire row: its key, `(instrument, month, rung)`, and the tuple's
    /// other seven fields.
    type WireRow = ((String, String, String), Vec<serde_json::Value>);

    /// A wire body's rows by key, in wire order.
    fn wire_rows(body: &serde_json::Value) -> Vec<WireRow> {
        let word = |list: &str, index: &serde_json::Value| {
            body[list][usize::try_from(index.as_u64().expect("an index")).expect("small")]
                .as_str()
                .expect("a word")
                .to_owned()
        };
        body["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|row| {
                let row = row.as_array().expect("a tuple");
                let reason = |index: &serde_json::Value| {
                    if index.is_null() {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(word("reasons", index))
                    }
                };
                (
                    (
                        word("instruments", &row[0]),
                        word("months", &row[1]),
                        word("timeframes", &row[2]),
                    ),
                    vec![
                        row[3].clone(),
                        row[4].clone(),
                        row[5].clone(),
                        row[6].clone(),
                        reason(&row[7]),
                        row[8].clone(),
                        reason(&row[9]),
                    ],
                )
            })
            .collect()
    }

    fn delta_of(census: &census::VendorCensus, since: &str, epoch: &str) -> serde_json::Value {
        let (code, body) = delta_response(
            Some(census),
            Vendor::Gdfl,
            store_wire::Format::Compact,
            since,
            epoch,
        );
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        serde_json::from_str(&body).expect("json")
    }

    fn refused(
        census: Option<&census::VendorCensus>,
        format: store_wire::Format,
        since: &str,
        epoch: &str,
    ) -> (axum::http::StatusCode, serde_json::Value) {
        let (code, body) = delta_response(census, Vendor::Gdfl, format, since, epoch);
        (code, serde_json::from_str(&body).expect("json"))
    }

    /// **THE ORDER A WHOLE BODY LISTS ITS ROWS IN, PINNED BY EXAMPLE** (WP-56).
    ///
    /// Newest month first; inside a month, the series' own order: a spot
    /// series before every contract, then by exchange, segment, symbol and
    /// rung; contracts by their own text, then exchange, segment, symbol,
    /// rung. A reader that merges a delta places a new row among the rows it
    /// holds by this order (`web/src/lib/store-census-wire.js`,
    /// `censusOrder`), and `web/tests/store-census-wire.test.js` holds the
    /// SAME list: the two sides are pinned to one example.
    #[test]
    fn the_whole_body_lists_newest_month_first_then_the_series_own_order() {
        let march = month(2019, 3);
        let april = month(2019, 4);
        let bse = census::Series {
            exchange: brutex_core::instrument::Exchange::Bse,
            ..series_at(Segment::Index, "SENSEX", Timeframe::DAY_1)
        };
        let rows = [
            held_at(contract("NIFTY", "2019-04-25-1150000-PE"), march, 1, (5, 6)),
            held_at(contract("NIFTY", "2019-04-25-1150000-CE"), march, 1, (5, 6)),
            held_at(contract("NIFTY", "2019-04-25-FUT"), march, 1, (5, 6)),
            held_at(contract("NIFTY", "2019-04-25-950000-CE"), march, 1, (5, 6)),
            held_at(
                contract("BANKNIFTY", "2019-04-25-1150000-CE"),
                march,
                1,
                (5, 6),
            ),
            held_at(
                contract("BANKNIFTY", "2019-03-28-2900000-CE"),
                march,
                1,
                (5, 6),
            ),
            held_at(
                series_at(Segment::Cash, "RELIANCE", Timeframe::SECOND_1),
                march,
                1,
                (5, 6),
            ),
            held_at(
                series_at(Segment::Cash, "BAJAJ-AUTO", Timeframe::SECOND_1),
                march,
                1,
                (5, 6),
            ),
            held_at(
                series_at(Segment::Index, "NIFTY", Timeframe::DAY_1),
                march,
                1,
                (5, 6),
            ),
            held_at(
                series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1),
                march,
                1,
                (5, 6),
            ),
            held_at(
                series_at(Segment::Index, "NIFTY", Timeframe::SECOND_1),
                march,
                1,
                (5, 6),
            ),
            held_at(
                series_at(Segment::Index, "BANKNIFTY", Timeframe::MINUTE_1),
                march,
                1,
                (5, 6),
            ),
            held_at(bse, march, 1, (5, 6)),
            held_at(
                series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1),
                april,
                1,
                (5, 6),
            ),
            held_at(contract("NIFTY", "2019-04-25-1150000-CE"), april, 1, (5, 6)),
        ];
        let disk = Disk::single("order", &rows);
        let listed: Vec<String> = wire_rows(&whole(&disk.read(None)))
            .into_iter()
            .map(|((instrument, month, rung), _)| format!("{month} {instrument} {rung}"))
            .collect();
        assert_eq!(
            listed,
            [
                "2019-04 NSE-INDEX-NIFTY 1min",
                "2019-04 NSE-FNO-NIFTY-2019-04-25-1150000-CE 1s",
                "2019-03 NSE-INDEX-BANKNIFTY 1min",
                "2019-03 NSE-INDEX-NIFTY 1s",
                "2019-03 NSE-INDEX-NIFTY 1min",
                "2019-03 NSE-INDEX-NIFTY 1day",
                "2019-03 NSE-CASH-BAJAJ-AUTO 1s",
                "2019-03 NSE-CASH-RELIANCE 1s",
                "2019-03 BSE-INDEX-SENSEX 1day",
                "2019-03 NSE-FNO-BANKNIFTY-2019-03-28-2900000-CE 1s",
                "2019-03 NSE-FNO-BANKNIFTY-2019-04-25-1150000-CE 1s",
                "2019-03 NSE-FNO-NIFTY-2019-04-25-1150000-CE 1s",
                "2019-03 NSE-FNO-NIFTY-2019-04-25-1150000-PE 1s",
                "2019-03 NSE-FNO-NIFTY-2019-04-25-950000-CE 1s",
                "2019-03 NSE-FNO-NIFTY-2019-04-25-FUT 1s",
            ]
        );
    }

    /// **A DELTA IS THE ROWS APPENDED SINCE, AND MERGING IT INTO THE CENSUS
    /// THE READER HELD GIVES THE WHOLE BODY AT THE NEW GENERATION** (WP-56;
    /// D-3327 item 3; DB-022, DB-060).
    ///
    /// Four appends after the read: more bars in a month already held (its
    /// change moves, AND the next month's `prev_chg_bps`, so that row is in
    /// the delta though nothing was appended to it), a new month of a held
    /// series, a new contract in a held month and a new series. The contract
    /// nothing touched is not in the delta. Merged by key into the body read
    /// before, the rows are exactly the whole body read after; the delta's
    /// rows keep the whole body's order; and the counters beside them are the
    /// census's own. Before WP-56 `since` was not read: the answer was the
    /// whole body every time.
    #[test]
    fn a_delta_carries_what_was_appended_and_merges_to_the_whole_body() {
        let nifty = series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1);
        let ce = contract("NIFTY", "2019-03-28-1150000-CE");
        let (feb, march, april) = (month(2019, 2), month(2019, 3), month(2019, 4));
        let mut disk = Disk::single(
            "merge",
            &[
                held_at(nifty, feb, 10, (10_000, 11_000)),
                held_at(nifty, march, 20, (11_000, 12_100)),
                held_at(ce, march, 30, (700, 770)),
            ],
        );
        let before = disk.read(None);
        let generation = before.generation().expect("held");
        let read_before = whole(&before);

        disk.append(&[
            held_at(nifty, feb, 12, (10_000, 10_500)),
            held_at(nifty, april, 5, (12_100, 12_705)),
            held_at(
                contract("NIFTY", "2019-03-28-1160000-PE"),
                march,
                7,
                (300, 330),
            ),
            held_at(
                series_at(Segment::Index, "BANKNIFTY", Timeframe::DAY_1),
                march,
                3,
                (2_900_000, 2_929_000),
            ),
        ]);
        let after = disk.read(Some(&before));
        let read_after = whole(&after);

        let delta = delta_of(&after, &generation.to_string(), "");
        assert_eq!(delta["since"], generation.to_string());
        assert_eq!(delta["schema"], 1);
        assert_eq!(delta["encoding"], store_wire::ENCODING);
        assert_eq!(delta["feed"], "gdfl");
        let sent = wire_rows(&delta);
        let keys: Vec<String> = sent
            .iter()
            .map(|((instrument, month, rung), _)| format!("{month} {instrument} {rung}"))
            .collect();
        assert_eq!(
            keys,
            [
                "2019-04 NSE-INDEX-NIFTY 1min",
                "2019-03 NSE-INDEX-BANKNIFTY 1day",
                "2019-03 NSE-INDEX-NIFTY 1min",
                "2019-03 NSE-FNO-NIFTY-2019-03-28-1160000-PE 1s",
                "2019-02 NSE-INDEX-NIFTY 1min",
            ],
            "what was appended, the row after a changed month, and nothing else"
        );

        // MERGED BY KEY, IT IS THE WHOLE BODY.
        let mut merged: std::collections::BTreeMap<_, _> =
            wire_rows(&read_before).into_iter().collect();
        for (key, row) in sent {
            let _ = merged.insert(key, row);
        }
        let now = wire_rows(&read_after);
        assert_eq!(merged, now.iter().cloned().collect());
        // March's own change did not move; the change of the month before it
        // did, from +10.00% to +5.00%.
        let march_row = &merged[&(
            "NSE-INDEX-NIFTY".to_owned(),
            "2019-03".to_owned(),
            "1min".to_owned(),
        )];
        assert_eq!(march_row[3], 1_000);
        assert_eq!(march_row[5], 500, "its previous month's new change");

        // THE DELTA'S ROWS KEEP THE WHOLE BODY'S ORDER.
        let order: Vec<&(String, String, String)> = now.iter().map(|(key, _)| key).collect();
        let mut at = 0;
        for key in &keys {
            let found = order[at..]
                .iter()
                .position(|(instrument, month, rung)| {
                    format!("{month} {instrument} {rung}") == *key
                })
                .unwrap_or_else(|| panic!("{key} is out of the whole body's order"));
            at += found + 1;
        }

        // THE COUNTERS A READER CHECKS ITS MERGE AGAINST.
        assert_eq!(delta["held"]["months"], now.len());
        let bars: u64 = now
            .iter()
            .map(|(_, row)| row[0].as_u64().expect("rows"))
            .sum();
        assert_eq!(delta["held"]["rows"], bars);

        // ALREADY CURRENT: no row, the same counters.
        let newest = after.generation().expect("held");
        let current = delta_of(&after, &newest.to_string(), "");
        assert_eq!(current["rows"], serde_json::json!([]));
        assert_eq!(current["held"]["months"], now.len());
    }

    /// **A DELTA THAT CANNOT BE CUT IS REFUSED BY NAME, NEVER ANSWERED IN
    /// PART** (WP-56; DB-B56's rule, on this route too).
    #[test]
    fn a_delta_that_cannot_be_cut_is_refused_by_name() {
        let nifty = series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1);
        let mut disk = Disk::single("refused", &[held_at(nifty, month(2019, 2), 10, (5, 6))]);
        let before = disk.read(None);
        let generation = before.generation().expect("held");
        disk.append(&[held_at(nifty, month(2019, 3), 10, (6, 7))]);
        let after = disk.read(Some(&before));
        let compact = store_wire::Format::Compact;

        // BEFORE THE LAST WHOLE READ: the appends after it are not held.
        let (code, body) = refused(Some(&after), compact, &(generation - 1).to_string(), "");
        assert_eq!(code, axum::http::StatusCode::GONE, "{body}");
        assert_eq!(body["error"], "since_too_old");
        assert_eq!(body["floor"], generation);
        // PAST THE NEWEST: never published, so not "too old".
        for future in [generation + 2, 99_999_999_999] {
            let (code, body) = refused(Some(&after), compact, &future.to_string(), "");
            assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
            assert_eq!(body["error"], "since_in_future");
            assert_eq!(body["generation"], generation + 1);
        }
        // A SHAPE THIS CENSUS DOES NOT TAKE.
        let (code, body) = refused(Some(&after), compact, "INDEX=1,CASH=0,FNO=0", "");
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "since_shape");
        // AN EPOCH NAMED FOR A SINGLE FILE, WHICH HAS NONE.
        let (code, body) = refused(Some(&after), compact, &generation.to_string(), "FNO=000001");
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "epoch_changed");
        // NOT A GENERATION, AND NOT THE TUPLES WIRE.
        for bad in [
            "soon",
            "-1",
            "+1",
            "1.5",
            "INDEX=",
            "INDEX=1,INDEX=2",
            "NOPE=1",
            "1,2",
        ] {
            let (code, body) = refused(Some(&after), compact, bad, "");
            assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{bad}: {body}");
            assert_eq!(body["error"], "bad_request", "{bad}");
        }
        let (code, body) = refused(
            Some(&after),
            store_wire::Format::Expanded,
            &generation.to_string(),
            "",
        );
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{body}");
        // NO CENSUS NOW.
        let (code, body) = refused(None, compact, &generation.to_string(), "");
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "since_shape");
    }

    /// **OVER A PARTITIONED CENSUS A DELTA NAMES EVERY PART'S GENERATION AND
    /// THE EPOCHS THEY WERE READ FROM** (WP-56; D-3316, DB-B56, DB-B67).
    ///
    /// The vendor-wide generation is the parts' sum and names no log; each
    /// part's own does, in its own epoch. So `since` is the
    /// `x-brutex-census-part-generations` the reader read, `epoch` its
    /// `x-brutex-census-epoch`, and a different epoch, a missing part or the
    /// sum is refused.
    #[test]
    fn a_partitioned_delta_names_every_part_and_its_epoch() {
        let nifty = series_at(Segment::Index, "NIFTY", Timeframe::SECOND_1);
        let ce = contract("NIFTY", "2019-03-28-1150000-CE");
        let march = month(2019, 3);
        let mut disk = Disk::partitioned(
            "parts",
            &[
                (Segment::Index, &[held_at(nifty, march, 10, (5, 6))]),
                (Segment::Fno, &[held_at(ce, march, 20, (7, 8))]),
            ],
        );
        let before = disk.read(None);
        let headers = census_headers_with(Some(&before), None);
        let since = header(&headers, CENSUS_PART_GENERATIONS_HEADER).to_owned();
        let epoch = header(&headers, CENSUS_EPOCH_HEADER).to_owned();
        assert_eq!(epoch, "INDEX=000001,CASH=none,FNO=000001");
        let sum = header(&headers, CENSUS_GENERATION_HEADER).to_owned();

        disk.append(&[held_at(
            contract("NIFTY", "2019-03-28-1160000-CE"),
            march,
            4,
            (9, 10),
        )]);
        let after = disk.read(Some(&before));
        let delta = delta_of(&after, &since, &epoch);
        let sent = wire_rows(&delta);
        assert_eq!(sent.len(), 1, "{delta}");
        assert_eq!(sent[0].0.0, "NSE-FNO-NIFTY-2019-03-28-1160000-CE");
        assert_eq!(delta["held"]["months"], 3);
        assert_eq!(delta["since"], since);

        let compact = store_wire::Format::Compact;
        // THE SUM IS NOT A SINCE (DB-B56, DB-B67).
        let (code, body) = refused(Some(&after), compact, &sum, &epoch);
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "since_shape");
        // A PART LEFT OUT.
        let (code, body) = refused(Some(&after), compact, "INDEX=1,FNO=1", &epoch);
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "since_shape");
        // NO EPOCH NAMED, AND ANOTHER EPOCH.
        let (code, body) = refused(Some(&after), compact, &since, "");
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{body}");
        let (code, body) = refused(
            Some(&after),
            compact,
            &since,
            "INDEX=000001,CASH=none,FNO=000002",
        );
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "epoch_changed");
        assert_eq!(body["now"], epoch);
        // ONE PART PAST ITS NEWEST: refused naming the part.
        let future = since.replace("INDEX=", "INDEX=9");
        let (code, body) = refused(Some(&after), compact, &future, &epoch);
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "since_in_future");
        assert_eq!(body["part"], "INDEX");
    }

    /// **THE ROUTE READS `since`: A CURRENT READER IS ANSWERED NO ROWS, WITH
    /// EVERY CENSUS HEADER, AND A HEAD IS STILL A HEAD** (WP-56).
    #[test]
    fn the_route_answers_a_since_with_the_census_headers() {
        let dir = crate::server::tests::agreeing("store-since");
        let site = crate::server::tests::site("store-since", &dir);
        let mut manifest =
            pull::manifest::Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
        let nifty = series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1);
        let _append = manifest
            .record_held(held_at(nifty, month(2019, 3), 10, (5, 6)))
            .expect("records");
        std::fs::write(
            pull::manifest::manifest_path(&site.store_root, Vendor::Gdfl),
            manifest.image(),
        )
        .expect("written");
        let get = |query: &str| {
            let uri: axum::http::Uri = format!("/store.json?{query}").parse().expect("a uri");
            let (code, headers, body) = store_response(
                &site,
                &axum::http::HeaderMap::new(),
                &uri,
                &axum::http::Method::GET,
            );
            (
                code,
                headers,
                serde_json::from_slice::<serde_json::Value>(&body).expect("json"),
            )
        };
        let (code, headers, body) = get("feed=gdfl&encoding=census-tuples-v1&since=1");
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert_eq!(body["rows"], serde_json::json!([]));
        assert_eq!(body["since"], "1");
        assert_eq!(body["held"]["months"], 1);
        assert_eq!(body["held"]["rows"], 10);
        assert_eq!(header(&headers, CENSUS_GENERATION_HEADER), "1");
        assert_eq!(header(&headers, CENSUS_STATE_HEADER), "held");
        assert!(
            headers.get(axum::http::header::ETAG).is_none(),
            "a delta is not the body"
        );
        assert_eq!(header(&headers, CENSUS_SINCE_HEADER), "1");
        let (_, whole_headers) = ask(
            &site,
            &axum::http::Method::GET,
            "feed=gdfl&encoding=census-tuples-v1",
        );
        assert!(
            whole_headers.get(CENSUS_SINCE_HEADER).is_none(),
            "a whole body carries no since: that is how a reader tells the two apart"
        );

        let (code, _, body) = get("feed=gdfl&encoding=census-tuples-v1&since=2");
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "since_in_future");
        let (code, _, body) = get("feed=gdfl&since=1");
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{body}");

        // A HEAD NEVER CUTS A DELTA: it is the header read it always was.
        let (code, headers) = ask(&site, &axum::http::Method::HEAD, "feed=gdfl&since=1");
        assert_eq!(code, axum::http::StatusCode::OK);
        assert_eq!(header(&headers, CENSUS_GENERATION_HEADER), "1");
    }

    #[test]
    fn a_census_validator_answers_for_the_bytes_and_not_for_the_read() {
        let one = census_etag("[]");
        let same = census_etag("[]");
        let other = census_etag(r#"[{"instrument":"BANKNIFTY"}]"#);

        assert_eq!(
            one, same,
            "the same bytes are the same answer — a validator that changed on a \
             re-read would force a full transfer for an unchanged census, which \
             is the whole cost this exists to remove"
        );
        assert_ne!(one, other, "different bytes are a different answer");
        assert_eq!(one.len(), 66, "a quoted 32-byte digest: {one}");
        assert!(
            one.starts_with('"') && one.ends_with('"'),
            "an ETag is a quoted-string by RFC 9110: {one}"
        );
    }

    #[test]
    fn a_reader_holding_the_answer_is_recognised_and_a_wildcard_is_not() {
        let etag = census_etag("[]");
        let with = |value: &str| {
            let mut map = axum::http::HeaderMap::new();
            map.insert(
                axum::http::header::IF_NONE_MATCH,
                axum::http::HeaderValue::from_str(value).expect("a header value"),
            );
            map
        };

        assert!(none_match_hit(&with(&etag), &etag), "the exact tag");
        assert!(
            none_match_hit(&with(&format!("\"deadbeef\", {etag}")), &etag),
            "If-None-Match is a LIST and a reader may legitimately send several"
        );
        assert!(
            !none_match_hit(&with("\"deadbeef\""), &etag),
            "a tag for another answer is not this answer"
        );
        assert!(
            !none_match_hit(&with("*"), &etag),
            "`*` means \"if any representation exists\", which for an endpoint \
             that always has one would answer 304 for ever"
        );
        assert!(
            !none_match_hit(&axum::http::HeaderMap::new(), &etag),
            "a reader holding nothing gets the body"
        );
    }
}

//! `/instruments.json` and `/universes.json` -- the selected feed's tracked
//! instruments with their stored bar counts, and what each spot target
//! resolves to for that feed.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The edits are visibility qualifiers
//! (`pub(super)`) on items another module now has to reach, and one probe:
//! `bars_by_symbol` asks [`census::VendorCensus::rows_of`] where it asked
//! `rows_for`, keeping only the `Held` arm -- exactly the `Some` arm it kept
//! before, so the sums are the same. That gives the WP-01 interface a
//! production caller. Intra-doc links to `server.rs` items that are no longer
//! in scope here carry their path. No route answers differently because of the
//! move.

use super::{
    Loaded, MasterState, census_headers, census_is_unreadable, census_now, no_such_feed_json,
    note_header, param, universe_label, universes_json,
};
use crate::{census, ingest, render};
use std::fmt::Write as _;

/// The browser's copy of the bounded universe, for the type-ahead.
///
/// # Why the whole set, and why once
///
/// The engine surface is bounded: ~750 NIFTY Total Market equities plus ~35 NSE
/// indices. That is small enough to hand the browser the entire searchable set
/// in one response, which is what makes a keystroke a Map probe instead of a
/// request. A request per character is ~800 requests to type one symbol and
/// makes keystroke latency a function of the network.
///
/// Hand-written JSON rather than a serialiser, because adding one would be a
/// dependency for six fields whose shapes are all known here, and every value
/// below is escaped through [`render::json_string`] rather than trusted.
///
/// # Two silent answers this route used to give, and what now separates them
///
/// Both were `200` with no status anywhere on the wire, while `Read::notes` and
/// [`Read::status`](super::Read::status) — which `/health` reads and answers
/// `503` from — sat one field away and were discarded here.
///
/// * **The counter would not load.** `rows_for` answers `None` for an
///   unreadable census, so `bars_of` summed to `0` for every row and the page
///   drew the same em dash a genuinely un-pulled instrument shows. The universe
///   was intact; the count was not a measurement.
/// * **The selected feed's master would not decode.** No row then carries that
///   feed's id, the filter admits nothing, and the body is `[]` — indistinguishable
///   from a feed that lists nothing. [`Read::master`](super::Read::master) is
///   the per-feed answer the whole-read boolean could not give.
///
/// Both now answer `503` and stamp the reason: [`UNIVERSE_STATUS_HEADER`],
/// [`MASTER_STATE_HEADER`], [`MASTER_NOTE_HEADER`] and the three census headers
/// [`census_headers`] writes. **The body shape does not move** — it is a JSON
/// array in every state, exactly as it was. D-0124.
pub(super) async fn instruments_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    // THE LIST IS THE SELECTED FEED'S, NOT THE MERGE OF ALL OF THEM.
    //
    // This returned the merged universe whatever feed was chosen, so switching
    // the picker changed nothing. That is wrong twice over: the two brokers do
    // NOT list the same instruments — `GIFTNIFTY` is Dhan-only — so the page
    // offered symbols the selected feed cannot be asked for, and `held` was
    // computed feed-agnostically, so a row read HELD while the chosen feed held
    // nothing for it.
    //
    // An instrument belongs to a feed's list when that feed's master gave it an
    // id. `ids` is indexed by the vendor's own discriminant, so the test is one
    // array index per row and not a lookup.
    // REFUSED BY NAME, NEVER SUBSTITUTED — the same rule `masters_json` states
    // twelve hundred lines below, and this route was the last of twelve
    // `parse_vendor` call sites in this crate still breaking it.
    //
    // `ingest::parse_vendor` answers the EMPTY string with Dhan itself, so this
    // `unwrap_or` was unreachable for an absent `?feed=` and caught exactly one
    // input: a feed somebody NAMED and this build does not read. Everything
    // downstream then became Dhan's answer under HTTP 200 — the row filter
    // (`ids.get(feed as usize)`), the per-symbol bar counts, and the census and
    // master headers. A wrong answer wearing the shape of a right one, which
    // this file elsewhere calls the worst class of defect in the repository.
    let asked_feed = param(uri.query().unwrap_or(""), "feed");
    let Some(feed) = ingest::parse_vendor(&asked_feed) else {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
        );
        return (
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            no_such_feed_json(&asked_feed),
        );
    };
    // SORTED, GROUPED, AND HELD-FIRST. `by_key` is a HashMap, so iterating it
    // gave the browser TATACOMM, GMRAIRPORT, PINELABS — an order stable per
    // process and meaningless to a human. Nothing was findable by scrolling and
    // 35 indices were scattered through 750 equities.
    //
    // Three keys: instruments this store HOLDS come first because those are the
    // ones that can be charted; indices before equities because they are the
    // engine's own surface and there are 35 against 750; then alphabetical.
    //
    // O(n log n) once per request over a bounded ~800, replacing an unbounded
    // amount of human scanning.
    //
    // ONE GUARD FOR THE WHOLE STATEMENT. The rows below BORROW from the parsed
    // universe, so the read guard has to outlive them — a temporary taken
    // inline is dropped at the semicolon with the borrows still live, which is
    // the compiler catching the reload's one real hazard rather than a nuisance.
    // NAMED `universe` AND NOT `held`, because `held` is REBOUND thirteen lines
    // down and a shadow does not drop.
    //
    // This guard was called `held`; `let held = bars_by_symbol(..)` below then
    // shadowed the name while the guard itself stayed alive to the end of the
    // function -- correctly, since `listing` borrows through it. Two later
    // statements read `site.universe()` again, and with the name taken there was
    // no way to see that those were a SECOND `read()` on a lock this thread
    // already held.
    //
    // That is a deadlock, not a slow path. `Site::reparse` takes `.write()` on
    // the same lock and is reachable from `POST /masters/refresh`, and a writer
    // waiting between the two acquisitions blocks the second one against a guard
    // only this thread can release. The crate already states the rule twenty-odd
    // lines from the writer: *"holding both across the same statement is the
    // deadlock this shape invites."*
    //
    // The fix is to acquire once and reuse, which the borrow already required.
    let universe = site.universe();
    let mut listing: Vec<_> = universe
        .read
        .merged
        .by_key
        .iter()
        .filter(|(_, entry)| {
            crate::catalog::tracked(entry.universe)
                && entry.ids.get(feed as usize).copied().flatten().is_some()
        })
        .collect();
    // HOW MANY BARS, not whether any. "HELD" was store jargon that leaked onto
    // the screen: it told an operator a boolean when the question they actually
    // have is "how much of this do I have". A count answers both — zero IS the
    // boolean, and 1,125 is what the boolean threw away.
    // FRESH, so a row's bar count moves as a backfill lands. See `census_now`.
    let (censuses, entries) = census_now(&site);
    // ONE PASS OVER THE CENSUS, NOT ONE PER INSTRUMENT. See `bars_by_symbol`.
    let held = bars_by_symbol(censuses.iter().find(|c| c.vendor == feed), entries.iter());
    let bars_of = |key: &brutex_core::instrument::InstrumentKey| -> u64 {
        held.get(&(key.exchange, key.segment, key.underlying))
            .copied()
            .unwrap_or(0)
    };
    listing.sort_unstable_by_key(|(key, _)| {
        (
            bars_of(key) == 0,
            key.kind != brutex_core::instrument::Kind::Index,
            key.underlying,
        )
    });

    let mut out = String::from("[");
    for (n, (key, entry)) in listing.into_iter().enumerate() {
        let bars = bars_of(key);
        // THE TRACKED UNIVERSE ONLY — the same predicate the page uses.
        //
        // This iterated the whole master and shipped 2,780 listings while the
        // page beside it said 785. A type-ahead that suggests instruments the
        // operator's own universe excludes is worse than none: it offers a
        // symbol, the click lands on a page that does not list it, and nothing
        // says why. The bound is ~800 by decision, and this is one of the
        // places that must honour it rather than one of the places that
        // quietly does not.
        if n > 0 {
            out.push(',');
        }
        let canonical = key.to_string();
        let _ = write!(
            out,
            // EXCHANGE AND SEGMENT TRAVEL WITH THE ROW, because the store is
            // keyed on them: the path IS the index, so a chart request names
            // every part rather than sending a synthetic id the server would
            // have to resolve back into one.
            // `universe` and `held` travel with the row so the browser can
            // GROUP without asking again: F&O vs NIFTY Total Market vs index is
            // the distinction the operator reads, and it is a bitset here.
            //
            // TWO UNIVERSE FIELDS, AND BOTH ARE LOAD-BEARING.
            //   `universe`  — the frozen one. `index`, `fno`, `ntm`, those
            //                 joined by `+`, or `other`, and never anything
            //                 else. The pages in production split it on `+`;
            //                 widening it would change a field they already
            //                 read. See `universe_label`.
            //   `universes` — the whole set, one token per bit, including the
            //                 four NIFTY tiers D-0089 appended. A new reader
            //                 uses this and needs no split; an old reader never
            //                 sees it. See `universe_tokens`.
            // The second is not a rename of the first and the first is not
            // deprecated by the second: an instrument's membership is a set,
            // and the string can only ever be a lossy view of it.
            r#"{{"symbol":{},"key":{},"kind":{},"exchange":{},"segment":{},"universe":{},"universes":{},"bars":{bars},"href":{}}}"#,
            render::json_string(key.underlying.as_str()),
            render::json_string(&canonical),
            render::json_string(&format!("{:?}", key.kind)),
            render::json_string(key.exchange.as_str()),
            render::json_string(key.segment.as_str()),
            render::json_string(&universe_label(entry.universe)),
            universes_json(entry.universe),
            render::json_string(&format!(
                "/instruments?q={}",
                render::query_value(&canonical)
            )),
        );
    }
    out.push(']');

    // THE READ'S OWN VERDICT, ON THE WIRE. `read.status()` and `read.master`
    // were computed at startup and consulted by `/health` and the HTML pages
    // and by nothing on this route, which is the whole defect: the numbers
    // below are only measurements when the master decoded and the counter
    // loaded, and until now nothing said which of those held.
    let census = censuses.iter().find(|c| c.vendor == feed);
    // THE GUARD TAKEN AT THE TOP, not a second `read()` on a lock this thread
    // already holds. Both of these read `site.universe()` before -- see the
    // comment on that binding for why that deadlocks against `reparse`.
    let (master, master_note) = universe.read.master(feed);
    let mut headers = census_headers(census);
    headers.insert(
        axum::http::HeaderName::from_static(UNIVERSE_STATUS_HEADER),
        note_header(universe.read.status()),
    );
    headers.insert(
        axum::http::HeaderName::from_static(MASTER_STATE_HEADER),
        axum::http::HeaderValue::from_static(master.name()),
    );
    headers.insert(
        axum::http::HeaderName::from_static(MASTER_NOTE_HEADER),
        note_header(&master_note),
    );
    // 503 FOR THE TWO STATES WHERE EVERY NUMBER BELOW IS ABSENT RATHER THAN
    // MEASURED, and 200 for everything else — including a merge disagreement,
    // which makes `status()` say `DEGRADED` while the list and the counts are
    // both real. Refusing the type-ahead for a routine ISIN conflict would take
    // the console down for a fact the header already carries.
    let code = if census_is_unreadable(census) || master == MasterState::Unavailable {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    (code, headers, out)
}

/// Bars held per symbol, folded out of the census in ONE pass.
///
/// # What this replaces, and what it cost
///
/// The closure this replaces scanned the whole entry vector for every symbol:
///
/// ```text
/// entries.iter().filter(|(series, _)| series.symbol == sym)
/// ```
///
/// and it was the `sort_unstable_by_key` key as well as the emitted field, so
/// the scan ran once per *comparison* — about 22.6 scans per row at n=785, plus
/// one per emitted row: ~18,565 full passes over the census for one request.
/// The cost was the PRODUCT of the tracked universe and the census, on a
/// response whose size never moves. Measured, in an optimised build with no
/// disk I/O, at universe 785: 750 entries 15.5 ms, 9,000 entries 165.9 ms,
/// 43,422 entries 819.2 ms — for 25 KB of JSON. `docs/06-limits.md` §34
/// projects a 93,776-row census, and `/instruments.json` is on the load path of
/// every page in the front end (`web/src/lib/index.svelte.js`).
///
/// Now: one pass over the entries, a hash probe per symbol, `CLAUDE.md` §3
/// rule 4. The per-request cost is O(entries + universe) and no longer their
/// product. `api::server::instrument_bar_counts_are_one_pass_over_the_census`
/// holds the pass count at one for two universe sizes, so a future edit that
/// reintroduces the inner scan fails by cost and not by taste.
///
/// THE NAME WAS WRONG AND THE TEST BENEATH IT WAS EMPTY, and the two failures
/// were one. It read `api::unit::…`, and there is no `unit` module anywhere in
/// this crate — the test is in `server.rs`'s own `mod tests` — while the test
/// itself folded an entries vector of length ZERO and asserted `0 == 0`. Gate
/// 12 was satisfied by both, because it resolves the crate and the final
/// segment and asks whether a proof is NAMED; its own log says it cannot ask
/// whether the claim is true. The module segment is `server` rather than
/// `server::tests` for that resolver's sake: it matches on `crate::mod::fn` and
/// a fourth segment makes it look for an `fn tests`, which is the form every
/// other citation in this file already avoids.
///
/// The iterator is taken generically for that test: it counts what it yields.
///
/// # What one count is
///
/// The ONE-MINUTE SPOT bars of one listing: keyed by exchange, segment and
/// symbol, with every contract series and every other rung left out. It was
/// keyed by the symbol alone, so the NIFTY index row summed its 1-minute and
/// 1-day bars and every stored NIFTY option, an index whose only data was
/// options showed as held and charted nothing, and `NSE-CASH-X` and
/// `BSE-CASH-X` showed one number (Z1-slice14-F1, D-1762).
fn bars_by_symbol<'a>(
    census: Option<&census::VendorCensus>,
    entries: impl IntoIterator<Item = &'a (census::Series, store::path::YearMonth)>,
) -> std::collections::HashMap<ListingKey, u64> {
    // PRE-SIZED FROM THE ENTRIES, not from the census. Gate 11 rule 3 refuses a
    // map that grows by rehashing, and this function exists to remove a cost —
    // it replaced an O(entries x universe) nested scan, so a map that rehashes
    // its way up hands part of that saving straight back. The ceiling is one
    // symbol per entry: the loop below inserts at most once per entry and
    // usually far fewer, since a symbol repeats across months. `size_hint().0`
    // is a LOWER bound by contract, which is what a capacity wants — never an
    // over-allocation on an iterator that cannot say how long it is.
    let entries = entries.into_iter();
    let mut held = std::collections::HashMap::with_capacity(entries.size_hint().0);
    let Some(census) = census else {
        // NOT AN EMPTY MAP OF ZEROES — an empty map, which every caller reads
        // as "no count for this symbol". The distinction is the one D-0124
        // makes on the wire: an unreadable census is not a store of zeroes.
        return held;
    };
    for (series, month) in entries {
        if series.contract.is_some() || series.timeframe != store::path::Timeframe::MINUTE_1 {
            continue;
        }
        if let super::census_cache::RowsOf::Held(rows) = census.rows_of(&series.at(*month)) {
            *held
                .entry((series.exchange, series.segment, series.symbol))
                .or_insert(0) += rows;
        }
    }
    held
}

/// The part of an instrument key that names its spot series in the store.
type ListingKey = (
    brutex_core::instrument::Exchange,
    brutex_core::instrument::Segment,
    brutex_core::symbol::Symbol,
);

/// What each spot target resolves to FOR ONE FEED — the counts, and every name
/// that did not resolve.
///
/// # The route the `/ingest` universe menu was missing
///
/// That menu drew four NIFTY tiers as `no target` and could not do otherwise:
/// nothing on the wire said what a tier resolves to for the selected feed, so a
/// page could either disable the row or synthesise a request the server cannot
/// honour. `/instruments.json` carries the membership — the `universes` array
/// has named `n500`, `n200`, `n100` and `n50` since D-0089 — but membership is
/// not reachability. A NIFTY 50 constituent that Groww's master has no row for
/// is in the tier and cannot be fetched from that feed, and folding
/// `/instruments.json` rows counts it or does not depending on which of two
/// filters a browser happens to apply.
///
/// This answers the question directly, once per feed: the slug to POST as
/// `target=`, the number that belongs on the control, and the buckets behind
/// the shortfall. D-0120.
///
/// # Cost
///
/// One [`coverage::Coverage::of`](crate::coverage::Coverage::of) per target —
/// seven array indices — plus the string. Nothing walks the universe:
/// [`Read::coverage`](super::Read::coverage) was built at load
/// (D-0039/D-0042), and the only thing that grows with the answer is the
/// `unresolved` list, which is the whole reason the operator asked.
///
/// A feed this build cannot read is REFUSED BY NAME rather than answered as
/// Dhan. `/instruments.json` defaults an unknown feed and that is a shipped
/// behaviour this does not copy: a page that mistypes a feed here would
/// otherwise enable four controls against a set the other broker reaches.
pub(super) async fn universe_reach_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let json = "application/json; charset=utf-8";
    let asked = param(uri.query().unwrap_or(""), "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        // THE SENTENCE IS NOT WRITTEN HERE ANY MORE. It moved to
        // [`no_such_feed_json`] when `/store.json` and `/store` stopped
        // answering an unknown feed as Dhan and needed the same words.
        return (
            axum::http::StatusCode::BAD_REQUEST,
            [(axum::http::header::CONTENT_TYPE, json)],
            no_such_feed_json(&asked),
        );
    };
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, json)],
        site.universe().read.coverage.json(feed),
    )
}

/// [`Read::status`](super::Read::status) — `ok` or `DEGRADED`, the same word
/// `/health` answers with.
pub const UNIVERSE_STATUS_HEADER: &str = "x-brutex-universe-status";

/// [`MasterState::name`] for the feed the request named.
pub const MASTER_STATE_HEADER: &str = "x-brutex-master-state";

/// The sentence behind [`MASTER_STATE_HEADER`].
pub const MASTER_NOTE_HEADER: &str = "x-brutex-master-note";

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
    use crate::server::tests::{
        DHAN_HEAD, agreeing, body_of, corrupt_census, get, masters, month_of, served_over, site,
        store_root,
    };
    use crate::server::{LEGACY_UNIVERSE_TOKENS, Site, UNIVERSE_TOKENS, universe_tokens};
    use brutex_core::vendor::Vendor;
    use std::path::PathBuf;

    /// **The bar count per instrument is ONE pass over the census, whatever the
    /// universe is.**
    ///
    /// `/instruments.json` folded the census inside a closure that was both the
    /// sort key and the emitted field, so the scan ran once per comparison —
    /// ~22.6 per row at n=785 — and the request cost the PRODUCT of the universe
    /// and the census for a response whose size never moves. Measured in an
    /// optimised build with no disk I/O: 750 entries 15.5 ms, 43,422 entries
    /// 819.2 ms, response flat at ~25 KB.
    ///
    /// The assertion is the pass count and not a duration, because a timing
    /// assertion on a shared machine is a flake. The iterator counts what it
    /// yields, so a future edit that puts the scan back inside the per-symbol
    /// path fails here by COST.
    ///
    /// The proof of the claim above is this test itself,
    /// `api::server::instrument_bar_counts_are_one_pass_over_the_census`. Named
    /// rather than left implicit because gate 12 reads a cost claim in any doc
    /// block, including a test's own, and cannot tell that the assertion is six
    /// lines below the sentence. The two DURATIONS quoted -- 15.5 ms and 819.2 ms
    /// -- are measurements and are not asserted anywhere, which is what the
    /// paragraph above says and why the pass count is the thing pinned.
    ///
    /// # What this asserted before, and why it asserted nothing
    ///
    /// `entries` was `Vec::new()`. The load-bearing line was
    /// `assert_eq!(visits.get(), entries.len(), ...)` with BOTH SIDES ZERO —
    /// `0 == 0`, under a message reading "one visit per entry, and the universe
    /// is not a factor in it". Nothing about a per-symbol scan can fail against
    /// an empty vector, so the O(entries x universe) regression the doc block
    /// on `bars_by_symbol` cites this test against would have walked straight
    /// through it. `CLAUDE.md` §4's "a test that asserts nothing" is the row it
    /// was sitting on.
    ///
    /// It runs at TWO sizes now, and the sizes differ in the UNIVERSE — three
    /// symbols and ten, over the same four months each — because the whole
    /// claim is that the universe is not a factor. A reintroduced inner scan
    /// yields `entries.len() * symbols` and misses at both.
    #[test]
    fn instrument_bar_counts_are_one_pass_over_the_census() {
        // Two universes, four months each. The month count is held fixed so the
        // only thing moving between the passes is the number of distinct
        // symbols -- which is the factor the claim says is absent.
        let small = ["NIFTY", "BANKNIFTY", "RELIANCE"];
        let large = [
            "NIFTY",
            "BANKNIFTY",
            "RELIANCE",
            "TCS",
            "INFY",
            "HDFCBANK",
            "ITC",
            "SBIN",
            "WIPRO",
            "ONGC",
        ];
        for symbols in [small.as_slice(), large.as_slice()] {
            let entries = one_pass_entries(symbols, 4);
            assert_eq!(
                entries.len(),
                symbols.len().saturating_mul(4),
                "the fixture must not be empty, which is the defect this replaces"
            );
            let census = one_pass_census(&entries);
            let visits = std::cell::Cell::new(0_usize);
            let counted = || {
                entries.iter().inspect(|_| {
                    visits.set(visits.get() + 1);
                })
            };

            // THE COST. One visit per entry, at both universe sizes. A scan
            // reintroduced inside the per-symbol path reads
            // `entries.len() * symbols` here -- 36 for the small pass and 400
            // for the large one -- and fails by COST rather than by taste.
            let held = bars_by_symbol(Some(&census), counted());
            assert_eq!(
                visits.get(),
                entries.len(),
                "one visit per entry, and the universe is not a factor in it"
            );

            // AND THE FOLD IS RIGHT, not merely cheap. A pass count on its own
            // is satisfied by a function that walks the entries and counts
            // nothing, so the map is checked too: one key per symbol, and each
            // key carrying every month that symbol holds.
            assert_eq!(
                held.len(),
                symbols.len(),
                "one key per symbol, and no key for a symbol nobody holds"
            );
            for name in symbols {
                let symbol = brutex_core::symbol::Symbol::new(name).expect("a symbol");
                let key = (
                    brutex_core::instrument::Exchange::Nse,
                    brutex_core::instrument::Segment::Index,
                    symbol,
                );
                assert_eq!(
                    held.get(&key),
                    Some(&(4 * ONE_PASS_ROWS)),
                    "{name} holds four months of {ONE_PASS_ROWS} rows"
                );
            }

            // NO CENSUS IS AN EMPTY MAP, not a map of zeroes: the caller reads a
            // missing key as "no count", which is the distinction D-0124 put on
            // the wire. It does not walk the entries at all -- which is the one
            // assertion here that the old empty fixture could still make.
            visits.set(0);
            let none = bars_by_symbol(None, counted());
            assert!(none.is_empty(), "an unreadable census counts nothing");
            assert_eq!(visits.get(), 0, "and it does not even walk the entries");

            // AN ABSENT MANIFEST OVER THE SAME ENTRIES. The entries are walked
            // -- once each, still -- and every probe misses, so the map is
            // empty rather than a row of zeroes. With the old fixture this arm
            // never entered the loop body at all.
            visits.set(0);
            let absent = census::read_vendor(
                &crate::scratch::path("bars-by-symbol"),
                brutex_core::vendor::Vendor::Dhan,
            );
            let nothing = bars_by_symbol(Some(&absent), counted());
            assert!(nothing.is_empty(), "an absent manifest holds nothing");
            assert_eq!(
                visits.get(),
                entries.len(),
                "and it is still one visit per entry when every probe misses"
            );
        }
    }

    /// One count is one listing's one-minute SPOT bars: an option on the same
    /// symbol, the same symbol's daily rung, and the same symbol on another
    /// exchange each stay out of it (Z1-slice14-F1, D-1762).
    #[test]
    fn an_instrument_counts_only_its_own_one_minute_spot_bars() {
        use brutex_core::instrument::{Contract, Exchange, Segment};
        let symbol = brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol");
        let spot = census::Series {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol,
            timeframe: store::path::Timeframe::MINUTE_1,
        };
        let option = census::Series {
            contract: Some(Contract::parse("2026-01-29-2500000-CE").expect("a contract")),
            segment: Segment::Fno,
            ..spot
        };
        let daily = census::Series {
            timeframe: store::path::Timeframe::DAY_1,
            ..spot
        };
        let bse = census::Series {
            exchange: Exchange::Bse,
            ..spot
        };
        let at = month_of(2026, 1);
        let entries = [(spot, at), (option, at), (daily, at), (bse, at)];
        let census = one_pass_census(&entries);
        let held = bars_by_symbol(Some(&census), entries.iter());
        assert_eq!(
            held.get(&(Exchange::Nse, Segment::Index, symbol)),
            Some(&ONE_PASS_ROWS)
        );
        assert_eq!(
            held.get(&(Exchange::Bse, Segment::Index, symbol)),
            Some(&ONE_PASS_ROWS)
        );
        assert_eq!(held.get(&(Exchange::Nse, Segment::Fno, symbol)), None);
        assert_eq!(held.len(), 2, "{held:?}");
    }

    /// The rows every fixture month in [`one_pass_census`] is recorded with.
    ///
    /// A regular NSE session, so a wrong fold reads as a wrong number of
    /// sessions rather than as an anonymous integer.
    const ONE_PASS_ROWS: u64 = 375;

    /// `symbols x months` index entries, oldest month first.
    ///
    /// Real [`census::Series`] values and real months: a fixture of the right
    /// SHAPE is what the test this feeds did not have.
    fn one_pass_entries(
        symbols: &[&str],
        months: u8,
    ) -> Vec<(census::Series, store::path::YearMonth)> {
        let mut out = Vec::with_capacity(symbols.len().saturating_mul(usize::from(months)));
        for name in symbols {
            for m in 1..=months {
                out.push((
                    census::Series {
                        contract: None,
                        exchange: brutex_core::instrument::Exchange::Nse,
                        segment: brutex_core::instrument::Segment::Index,
                        symbol: brutex_core::symbol::Symbol::new(name).expect("a symbol"),
                        timeframe: store::path::Timeframe::MINUTE_1,
                    },
                    month_of(2026, m),
                ));
            }
        }
        out
    }

    /// A census holding [`ONE_PASS_ROWS`] for every entry it is handed.
    ///
    /// In memory, at a path nothing ever writes -- the same fixture discipline
    /// `percentage_tests::census_of` states: every number the test reads came
    /// out of the manifest, and a change that reached for a bar file would fail
    /// rather than pass slowly.
    fn one_pass_census(
        entries: &[(census::Series, store::path::YearMonth)],
    ) -> census::VendorCensus {
        let mut manifest =
            pull::manifest::Manifest::open(Vendor::Dhan, &[], &[]).expect("a genesis census");
        for &(series, at) in entries {
            manifest
                .record_held(pull::manifest::Held::new(
                    pull::manifest::Entry {
                        key: series.at(at),
                        rows: ONE_PASS_ROWS,
                        first_ts_micros: 1_751_350_800_000_000,
                        last_ts_micros: 1_751_363_940_000_000,
                    },
                    pull::manifest::Closes::UNKNOWN,
                ))
                .expect("records");
        }
        census::VendorCensus {
            vendor: Vendor::Dhan,
            path: PathBuf::from("/nonexistent/bars-by-symbol/dhan.man"),
            state: census::Census::Held {
                manifest: Box::new(manifest),
            },
        }
    }

    /// **`/instruments.json` REFUSES A FEED IT DOES NOT READ, RATHER THAN
    /// ANSWERING AS DHAN.**
    ///
    /// The last of twelve `parse_vendor` call sites in this crate that
    /// substituted. `ingest::parse_vendor` answers the EMPTY string with Dhan
    /// itself, so the `.unwrap_or(Vendor::Dhan)` that stood here was
    /// unreachable for an absent `?feed=` and caught exactly one input: a feed
    /// somebody NAMED and this build does not read.
    ///
    /// Everything downstream then became Dhan's answer under **HTTP 200** — the
    /// row filter, the per-symbol bar counts, and the census and master
    /// headers. `masters_json` removed the identical line and said why: being
    /// served one vendor's copy after asking for another's is a wrong answer
    /// wearing the shape of a right one.
    ///
    /// Both arms are asserted because only the pair proves the rule: an absent
    /// feed must still work, or "refuse the unknown" would have quietly broken
    /// every page that omits the parameter.
    #[tokio::test]
    async fn instruments_json_refuses_a_named_feed_it_cannot_read() {
        let dir = masters("feedrefuse", None, None);
        let site = Site::serving(&dir, &store_root("feedrefuse"));
        let loaded = std::sync::Arc::new(site);

        let (status, _headers, body) = instruments_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/instruments.json?feed=notafeed".parse().expect("a uri"),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "a feed this build cannot read is refused, never answered as \
             another vendor's list: {body}"
        );
        assert!(
            body.contains("notafeed"),
            "and the refusal names what was asked for: {body}"
        );

        // AN ABSENT FEED IS NOT AN UNKNOWN ONE. `parse_vendor` answers the
        // empty string with the descriptor's default, so every page that omits
        // the parameter must still be served — without this half, "refuse the
        // unknown" would have broken all of them.
        let (status, _headers, body) = instruments_json(
            axum::extract::State(loaded),
            "/instruments.json".parse().expect("a uri"),
        )
        .await;
        assert_ne!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "an absent feed resolves to the descriptor's default and must NOT \
             take the unknown-feed arm: {body}"
        );
        assert!(
            !body.contains("refused"),
            "and it is not answered with the no-such-feed body: {body}"
        );
        // NOT asserted as 200: this fixture store holds no census, so the route
        // answers 503 for a reason that has nothing to do with the feed. What
        // matters here is only that it did not take the refusal arm — pinning
        // the success code would be testing the fixture, not the rule.
    }

    /// **`/instruments.json` used to answer both of its failures with `200`
    /// and nothing else.**
    ///
    /// Two states, one route, and neither reached the caller:
    ///
    /// * the counter would not load, so `rows_for` answered `None` for every
    ///   key and every row read `"bars":0` — the same em dash an un-pulled
    ///   instrument shows;
    /// * the selected feed's master would not decode, so no row carried that
    ///   feed's id, the filter admitted nothing, and the body was `[]` —
    ///   identical to a feed that lists nothing.
    ///
    /// `Read::notes` and `Read::status` were one field away, and `/health`
    /// answers `503` off exactly them. Both are on the wire now, and the second
    /// case gets the per-feed answer a whole-read boolean could never give:
    /// `?feed=dhan` on the very same site is `200` and complete.
    #[tokio::test]
    async fn instruments_json_says_when_its_zeroes_are_not_measurements() {
        // (a) THE COUNTER IS DAMAGED. The universe is intact, so the LIST is
        // real and only the counts are absent — 503, and the census headers say
        // which half is not to be believed.
        let dir = agreeing("instrcensus");
        let root = corrupt_census("instrcensus");
        let built = Loaded::new(Site::load(&dir, &root));
        served_over("instrcensus", built, |addr| async move {
            let answer = get(addr, "/instruments.json?feed=groww").await;
            assert!(answer.starts_with("HTTP/1.1 503"), "{answer}");
            assert!(
                answer.contains("x-brutex-census-state: unreadable"),
                "{answer}"
            );
            assert!(
                answer.contains("x-brutex-census-note:") && answer.contains("UNREADABLE"),
                "the reason, not just a code: {answer}"
            );
            assert!(
                answer.contains("x-brutex-master-state: read"),
                "the master decoded — it is the counter that did not: {answer}"
            );
            // THE ZEROES ARE STILL THERE AND STILL AN ARRAY. Nothing about the
            // body moved; what moved is that the response now says the zeroes
            // are absent rather than measured.
            let body = body_of(&answer);
            assert!(body.starts_with('['), "still an array: {body}");
            assert!(body.contains(r#""bars":0"#), "{body}");
        })
        .await;

        // (b) THE SELECTED FEED'S MASTER NEVER DECODED. The body is `[]` and
        // that is not this feed listing nothing.
        let half = masters(
            "instrmaster",
            None,
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n"
            )),
        );
        let built = Loaded::new(Site::load(&half, &store_root("instrmaster")));
        served_over("instrmaster", built, |addr| async move {
            let missing = get(addr, "/instruments.json?feed=groww").await;
            assert_eq!(body_of(&missing), "[]", "{missing}");
            assert!(
                missing.starts_with("HTTP/1.1 503"),
                "an empty list from a failed read is not an empty universe: {missing}"
            );
            assert!(
                missing.contains("x-brutex-master-state: UNAVAILABLE"),
                "{missing}"
            );
            assert!(
                missing.contains("groww_instruments.csv"),
                "the note names the file that was not read: {missing}"
            );
            assert!(
                missing.contains("x-brutex-universe-status: DEGRADED"),
                "the same word /health answers with: {missing}"
            );

            // AND THE OTHER FEED ON THE SAME SITE IS FINE. This is what a
            // whole-read boolean cannot say, and it is why `Read::master`
            // exists: one master failed, one did not, and the route answers per
            // feed.
            let ok = get(addr, "/instruments.json?feed=dhan").await;
            assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
            assert!(ok.contains("x-brutex-master-state: read"), "{ok}");
            assert!(
                body_of(&ok).contains(r#""symbol":"NIFTY""#),
                "and it lists what it read: {ok}"
            );
        })
        .await;
    }

    /// The type-ahead index is the SAME universe the page shows.
    ///
    /// It shipped `merged.by_key` whole — 2,780 listings — while the page
    /// beside it said 785. A suggestion the operator's own universe excludes is
    /// worse than no suggestion: the click lands on a page that does not list
    /// it and nothing says why. `CLAUDE.md` bounds the surface; a second
    /// endpoint that does not honour the bound is the bound not holding.
    #[tokio::test]
    async fn the_typeahead_index_is_the_tracked_universe_and_not_the_master() {
        let dir = agreeing("tajson");
        let site = site("tajson", &dir);
        let loaded: Loaded = std::sync::Arc::new(site);

        let (_code, _headers, json) = instruments_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            // The feed is part of the question now: the two brokers do not
            // list the same instruments, so the index is per feed.
            "/instruments.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;
        let site = &*loaded;

        let shipped = json.matches(r#""symbol":"#).count();
        let tracked = site
            .universe()
            .read
            .merged
            .by_key
            .values()
            .filter(|e| crate::catalog::tracked(e.universe))
            .count();

        assert_eq!(
            shipped, tracked,
            "the index must carry exactly the tracked universe — not the whole \
             master, which is what shipped 2,780 rows behind a page saying 785"
        );
        assert!(
            shipped <= site.universe().read.merged.by_key.len(),
            "and it cannot exceed the master it is drawn from"
        );

        // The escaper is exercised on real data rather than asserted about:
        // `&` is a legal symbol byte (M&M, M&MFIN) and a raw control character
        // would make the whole document unparseable, losing the type-ahead
        // rather than one row.
        assert!(
            !json.contains('\n') && !json.contains('\t'),
            "no raw control characters reach the document"
        );
        assert!(
            json.starts_with('[') && json.ends_with(']'),
            "and it is an array"
        );
    }

    /// The wire says every universe a row is in, and the old field never moves.
    ///
    /// D-0089 landed NIFTY 50 / 100 / 200 / 500 as `Universe` bits 3–6 and
    /// wired none of it: `/instruments.json` still answered one word out of
    /// `index|fno|ntm|other`, so four rows on `/` and `/db` shipped DISABLED
    /// with "no endpoint carries this membership" written on them. The data was
    /// there and the API could not say it.
    ///
    /// Two fields now, and this test is the reason both exist. `universes`
    /// carries the set. `universe` is frozen at the three words a running page
    /// already splits on `+` — widening it would have been the quiet kind of
    /// break, because the string would still parse and would still group, and
    /// only a reader comparing or bookmarking the whole value would notice.
    ///
    /// Pinned on a value with a NEW bit and no old one — a bare `NIFTY_50` —
    /// because that is the only shape that can tell "the compatibility field is
    /// pinned to three" apart from "the new bits happen to travel with `ntm`",
    /// which every real constituent does.
    #[tokio::test]
    async fn the_wire_carries_every_membership_and_the_frozen_field_never_widens() {
        use brutex_core::universe::Universe;

        // Nothing at all: the array is empty, the word is `other`. The two
        // spellings of "no membership" are deliberate and are not equal.
        assert_eq!(universe_label(Universe::NONE), "other");
        assert!(universe_tokens(Universe::NONE).is_empty());
        assert_eq!(universes_json(Universe::NONE), "[]");

        // Everything: the frozen field is STILL exactly three words.
        let every = UNIVERSE_TOKENS
            .iter()
            .fold(Universe::NONE, |acc, (bit, _)| acc.union(*bit));
        assert_eq!(
            universe_label(every),
            "index+fno+ntm",
            "the compatibility field is pinned to the three universes that \
             existed before D-0089, whatever else the row is in"
        );
        assert_eq!(
            universe_tokens(every),
            ["index", "fno", "ntm", "n500", "n200", "n100", "n50"],
            "and the array carries all seven, in bit order"
        );

        // A new bit ALONE cannot reach the frozen field.
        assert_eq!(universe_label(Universe::NIFTY_50), "other");
        assert_eq!(universe_tokens(Universe::NIFTY_50), ["n50"]);
        assert_eq!(universes_json(Universe::NIFTY_50), r#"["n50"]"#);

        // And the two fields cannot disagree about how a bit is spelled,
        // because both read `UNIVERSE_TOKENS`.
        for (bit, token) in UNIVERSE_TOKENS.into_iter().take(LEGACY_UNIVERSE_TOKENS) {
            assert_eq!(universe_label(bit), token, "one spelling per bit");
        }

        // Now the endpoint, on real membership rather than synthesised bits.
        let dir = agreeing("unijson");
        let site = site("unijson", &dir);
        let loaded: Loaded = std::sync::Arc::new(site);
        let (_code, _headers, json) = instruments_json(
            axum::extract::State(loaded),
            "/instruments.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;

        // RELIANCE is in all six equity lists — that is what the exchange's own
        // constituent files say, and it is why the four tiers were worth
        // wiring: `fno+ntm` was the whole answer this endpoint could give.
        assert!(
            json.contains(
                r#""universe":"fno+ntm","universes":["fno","ntm","n500","n200","n100","n50"]"#
            ),
            "RELIANCE must carry its four tiers in the array and none of them \
             in the frozen word: {json}"
        );
        // NIFTY is an index and an F&O underlying, and a constituent of
        // nothing: an index is not a share. Both fields say so.
        assert!(
            json.contains(r#""universe":"index+fno","universes":["index","fno"]"#),
            "an index names no tier: {json}"
        );
        // Every row carries both. `"universe":` is not a substring of
        // `"universes":` — the byte after `universe` differs — so these two
        // counts are independent.
        let frozen = json.matches(r#""universe":"#).count();
        let full = json.matches(r#""universes":"#).count();
        assert_eq!(frozen, json.matches(r#""symbol":"#).count());
        assert_eq!(full, frozen, "neither field is optional");
    }

    #[tokio::test]
    async fn universes_json_says_what_each_target_resolves_to_for_the_named_feed() {
        // THE ROUTE THE `/ingest` UNIVERSE MENU HAD NO ANSWER FROM. Its four
        // NIFTY rows drew `no target` because nothing on the wire turned a
        // tier into a request for the selected feed. D-0120.
        let dir = agreeing("reachjson");
        let loaded: Loaded = std::sync::Arc::new(site("reachjson", &dir));

        let (status, _headers, json) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(
            json.starts_with(r#"{"feed":"groww","mastered":true,"#),
            "{json}"
        );

        // ALL SEVEN ROWS, EACH WITH THE SLUG A POST WOULD CARRY. The four
        // tiers are the point: a page that reads this can enable them.
        for target in ingest::SpotTarget::ALL {
            let slug = target.slug();
            assert!(
                json.contains(&format!(r#""target":"{slug}""#)),
                "{slug} must be requestable from this answer: {json}"
            );
        }
        // RELIANCE is in every equity list this fixture's master carries, so
        // each tier resolves to exactly one Groww id and the rest of the
        // published list is named as unreachable rather than dropped.
        assert!(
            json.contains(r#""target":"n50","label":"NIFTY 50 equities""#),
            "{json}"
        );
        assert!(
            json.contains(
                r#""counted_from":"join","published":50,"matched":1,"lacks":49,"ambiguous":0,"malformed":0,"no_nse_isin":0"#
            ),
            "the count on the control is the matched bucket and the shortfall is \
             beside it, not behind it: {json}"
        );
        // AND THE BUCKET IS THE RIGHT ONE, WHICH D-0125 CHANGED. The join is
        // keyed on NSE's OWN ISIN now, so every NIFTY 50 name HAS an identity
        // whether or not a master mentions it — the forty-nine this fixture's
        // master does not carry are `lacks`, each naming the ISIN the exchange
        // printed, and `malformed` is empty because no published name here is
        // unusable. The two are still not interchangeable: `lacks` is a row to
        // go and ask Groww about, `malformed` would be a transcription to fix
        // in this repository, and `no_nse_isin` is a cell NSE left empty.
        assert!(
            json.contains(
                r#""symbol":"3MINDIA","bucket":"lacks","why":"this feed's master carries no row for ISIN INE470A01017""#
            ),
            "{json}"
        );
        assert!(
            json.contains(r#""counted_from":"master","published":null,"matched":1,"lacks":0"#),
            "and the two targets no published list defines say where THEIR count \
             came from rather than reporting a denominator nobody published: {json}"
        );

        // THE FEED DECIDES THE ANSWER. Dhan's master in this fixture carries
        // the same two instruments, so the counts match and the ids do not —
        // what must differ is the feed the document is about.
        let (status, _headers, other) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=dhan".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(other.starts_with(r#"{"feed":"dhan","#), "{other}");

        // AN ARCHIVE FEED IS NOT AN EMPTY ONE. A folder of CSVs is its own
        // listing and publishes no master, so every count is null.
        let (status, _headers, archive) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=truedata"
                .parse()
                .expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(
            archive.contains(r#""mastered":false"#)
                && archive.contains(r#""counted_from":"no master""#),
            "{archive}"
        );

        // AND AN UNKNOWN FEED IS REFUSED BY NAME. `/instruments.json` answers
        // an unrecognised feed as Dhan; copying that here would silently
        // enable four controls against the other broker's reach.
        //
        // `zerodha` WAS THIS EXAMPLE until 14 Aug 2026, when it became a feed
        // this build names. A test for an unknown feed has to name one that
        // stays unknown, or it stops testing the refusal and starts testing
        // whichever vendor was added last.
        let (status, _headers, refused) = universe_reach_json(
            axum::extract::State(loaded),
            "/universes.json?feed=upstox".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            refused.contains(r#""feed":"upstox""#)
                && refused.contains("this build reads no feed called that")
                && refused.contains("groww")
                && refused.contains("dhan"),
            "the refusal names what was asked and what is known: {refused}"
        );
    }
}

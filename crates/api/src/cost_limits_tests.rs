#![cfg(test)]
//! `docs/06-limits.md`'s D-1446 section, held to the source it describes.
//!
//! Each finding the section documents rather than changes gets one test. The
//! test checks two things: that the section's bullet for the finding names the
//! function that pays the cost and the bound it pays, and that the source
//! still has the shape that makes the bound true. A change that removes the
//! cost fails here until the bullet is updated, and a bullet that drops its
//! function or bound fails here too. W1-api5-0 through W1-api5-11, D-1446.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a document or a function that is not where these tests look is the failure"
)]

const LIMITS: &str = include_str!("../../../docs/06-limits.md");
const SERVER: &str = include_str!("server.rs");
// THE HANDLERS WP-01 MOVED OUT OF `server.rs`, read where they now live. The
// costs and the shapes these tests hold are unchanged; only the file is.
const CENSUS_CACHE: &str = include_str!("census_cache.rs");
const INSTRUMENTS_ROUTE: &str = include_str!("instruments_route.rs");
const CALENDAR_ROUTE: &str = include_str!("calendar_route.rs");
const BARS_ROUTE: &str = include_str!("bars_route.rs");
const BARS: &str = include_str!("bars.rs");
const CENSUS: &str = include_str!("census.rs");
const VERIFY: &str = include_str!("verify.rs");
const INGEST: &str = include_str!("../../pull/src/ingest.rs");
const MANIFEST: &str = include_str!("../../pull/src/manifest.rs");
// THE CENSUS READ D-3316 MOVED BEHIND `census_store::Target::load` (WP-10):
// the single file through `read_census` as before, a migrated vendor's
// partition through `read_epoch`. The cost this file holds is unchanged.
const CENSUS_STORE: &str = include_str!("../../pull/src/census_store.rs");

/// The D-1446 section, from its heading to the next `## ` heading or the end.
fn section() -> &'static str {
    let from = LIMITS
        .split_once("## API request and pull costs that still grow with the store — D-1446")
        .expect("docs/06-limits.md has the D-1446 section")
        .1;
    from.split_once("\n## ").map_or(from, |(own, _)| own)
}

/// The bullet that opens with `* **{id} — `, to the next bullet or heading,
/// with every run of whitespace made one space.
fn bullet(id: &str) -> String {
    let head = format!("* **{id} — ");
    let from = section()
        .split_once(head.as_str())
        .unwrap_or_else(|| panic!("the D-1446 section has a bullet for {id}"))
        .1;
    let end = ["\n* ", "\n### ", "\n## "]
        .iter()
        .filter_map(|stop| from.find(stop))
        .min()
        .unwrap_or(from.len());
    from.get(..end)
        .unwrap_or(from)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A top-level item's text in `source`, from `{head}` to its closing brace in
/// the first column.
fn item<'a>(source: &'a str, head: &str) -> &'a str {
    let from = source
        .split_once(&format!("\n{head}"))
        .unwrap_or_else(|| panic!("`{head}` is in the source"))
        .1;
    from.split_once("\n}\n")
        .unwrap_or_else(|| panic!("`{head}` ends"))
        .0
}

/// Every word in `words` is in the bullet for `id`.
fn names(id: &str, words: &[&str]) {
    let text = bullet(id);
    for word in words {
        assert!(
            text.contains(word),
            "{id}'s bullet does not name `{word}`: {text}"
        );
    }
}

#[test]
fn w1_api5_0_the_observation_is_taken_once_per_instrument_and_its_rest_is_named() {
    names(
        "W1-api5-0",
        &[
            "`ingestion_observations`",
            "once per instrument",
            "O(manifest bytes + E log E)",
            "O(E_v log E_v)",
            "What remains:",
            "one_instrument_lands_every_body_on_one_index_observation",
        ],
    );
    let land = item(SERVER, "async fn land_spot(");
    let (before, inside) = land
        .split_once("for bodies in landed.bodies.chunks(1)")
        .expect("land_spot walks its bodies one at a time");
    assert!(
        before.contains("ingestion_observations(landed, site)"),
        "the observation is taken before the first body: {land}"
    );
    assert!(
        !inside.contains("ingestion_observations") && inside.contains("land_bodies_observed("),
        "and no body takes its own: {land}"
    );
    assert!(
        !item(SERVER, "fn land_bodies_observed(").contains("ingestion_observations("),
        "the per-body path is handed the observation"
    );
}

#[test]
fn w1_api5_1_a_body_reads_the_whole_census_and_the_bullet_says_so() {
    names(
        "W1-api5-1",
        &[
            "`pull::ingest::from_window`",
            "once per body",
            "`read_census`",
            "O(manifest bytes + E_v)",
            "`pull::ingest::record_held`",
            "`roll_one`",
        ],
    );
    let body = item(SERVER, "fn land_bodies_observed(");
    let walk = body
        .split_once("for (chunk, body) in bodies")
        .expect("one from_window per body")
        .1;
    assert!(walk.contains("pull::ingest::from_window("), "{body}");
    assert!(
        item(INGEST, "fn from_members_inner(").contains("target.load()"),
        "`fn from_members_inner(` reads the whole census on every call"
    );
    assert!(
        item(INGEST, "fn record_all(").contains("census_store::record(store_root, vendor, held)"),
        "`fn record_all(` goes through the census cycle"
    );
    assert!(
        item(CENSUS_STORE, "pub fn cycle(").contains("target.load()"),
        "and every cycle reads the whole census"
    );
    let load = CENSUS_STORE
        .split_once("pub fn load(&self)")
        .and_then(|(_, rest)| rest.split_once("\n    }\n"))
        .map_or("", |(body, _)| body);
    assert!(
        load.contains("read_census(&path, self.vendor)")
            && load.contains("read_epoch(&path, self.vendor, segment)"),
        "`Target::load` reads the single file or the partition whole: {load}"
    );
    assert!(
        item(INGEST, "fn read_census(").contains("fs::read(path)"),
        "and the read is the whole file"
    );
    assert!(
        item(SERVER, "async fn roll_one(").contains("pull::ingest::record_held("),
        "`roll_one` records through the whole-census read"
    );
}

/// W1-api5-2, AS D-3327 LEFT IT. The bullet still names the cost it was
/// written for: every census miss read every vendor's whole manifest and sorted
/// every entry. D-3327 (WP-30) removed it: each vendor is read through its own
/// cell on its own stamp, a moved manifest is extended by its appended entries
/// (`census::read_vendor_after`), and the merged entry list is sorted only when
/// a caller reads it. The source assertions below pin that shape; the bullet
/// in `docs/06-limits.md` is not this package's to rewrite (WP-99 rewrites it
/// under D-3327), so the words are still held to it until it is.
#[test]
fn w1_api5_2_a_census_miss_reads_every_manifest_and_a_pull_makes_every_request_miss() {
    names(
        "W1-api5-2",
        &[
            "`census_now`",
            "five `stat` calls",
            "O(manifest bytes + E log E)",
            "every request",
            "`census::MAX_MANIFEST_BYTES`",
        ],
    );
    // R3-09 MOVED the all-vendor read into `census_now_kept`, which both the
    // production path and the test seam `census_now_stamping` call; the read
    // there hands back which rows were served from a kept read.
    let miss = item(CENSUS_CACHE, "fn census_now_kept(");
    assert!(
        miss.contains("let (mut censuses, kept_under) = read(&site.store_root);")
            && item(CENSUS_CACHE, "pub(super) fn census_now_stamping(")
                .contains("census_now_kept(site, stamp,"),
        "{miss}"
    );
    assert!(
        miss.contains("HeldEntries::of(&censuses)")
            && !miss.contains("census::held_entries(&censuses)"),
        "the entries are no longer sorted on every miss: {miss}"
    );
    assert!(
        item(CENSUS_CACHE, "pub(crate) fn census_now_stamped(")
            .contains("vendor_slot_fresh(site, vendor, vendor_stamp(root, vendor), true)"),
        "each vendor is read through its own cell"
    );
    // R3-09: the miss is rebuilt by `rebuild` (a cold vendor's first reader,
    // or the background re-read), through `fresh_read`, which extends the
    // vendor's previous read.
    assert!(
        item(CENSUS_CACHE, "fn vendor_slot_fresh(").contains("rebuild(&site.store_root, cell,")
            && item(CENSUS_CACHE, "fn rebuild(").contains("let census = fresh_read(")
            && item(CENSUS_CACHE, "fn fresh_read(")
                .contains("census::read_vendor_after(root, vendor, previous)"),
        "and a miss extends the vendor's previous read"
    );
    assert!(
        item(CENSUS, "pub fn held_entries(").contains("sort_unstable"),
        "the merged list is still a sort when a caller reads it"
    );
}

/// W1-api5-3, REWRITTEN TO WHAT THE CODE DOES (AF-05, p99-11, D-4972). The
/// bullet named a walk this route made on every request: the universe, every
/// census entry (`bars_by_symbol`, O(E)), and a sort. D-3327 (WP-30) builds
/// the body once per (feed census read, universe parse) on the store-read
/// pool, from the facet index's one node per instrument, and keeps it; the
/// pin held the old words until the bullet was rewritten. It now names the
/// kept body, the facet index, that the walk is gone, and the flat warm
/// latency AF-05 measured across an 8.4x difference in E.
#[test]
fn w1_api5_3_the_instrument_list_is_kept_per_census_read_and_counted_from_the_facet_index() {
    names(
        "W1-api5-3",
        &[
            "`instruments_json`",
            "`site.census_wire.kept`",
            "`held_by_listing`",
            "`facets::index`",
            "`bars_by_symbol`",
            "is gone",
            "O(T log T)",
            "E = 2,098,428",
            "n = 200",
        ],
    );
    let route = item(INSTRUMENTS_ROUTE, "pub(super) async fn instruments_json(");
    assert!(
        route.contains("crate::detail::run_store_read(move || instruments_answer(&site, feed))"),
        "off the async workers: {route}"
    );
    let answer = item(INSTRUMENTS_ROUTE, "pub(super) fn instruments_answer(");
    assert!(
        answer.contains("site.census_wire.kept(") && answer.contains("universe.generation"),
        "kept per census read and universe parse: {answer}"
    );
    assert!(
        item(INSTRUMENTS_ROUTE, "fn held_by_listing(").contains("super::facets::index("),
        "counted from the facet index, not a walk of every entry"
    );
    assert!(
        !INSTRUMENTS_ROUTE.contains("fn bars_by_symbol"),
        "the per-request walk is gone"
    );
}

/// W1-api5-4, REWRITTEN TO WHAT THE CODE DOES (AF-05, p99-11, D-4972). The
/// bullet said every non-`ts` sort and every `extremes=1` "still reads every
/// bar in its month range". Measured, a value sort and the extremes read no
/// row an index run orders (`scan_rows` 0 over 44 million one-second rows);
/// the time-of-day sort still reads every kept row and is refused by name
/// past `MAX_SCAN_BARS`; a month without a `.srt` is scanned (L-WP81-8,
/// L-SC-3). The bullet now says each of those, and the source below still
/// has the shape that makes each true.
#[test]
fn w1_api5_4_a_value_sort_reads_from_the_index_and_a_time_of_day_sort_still_reads_its_range() {
    names(
        "W1-api5-4",
        &[
            "`bars::window`",
            "`page_of`",
            "O(limit log limit)",
            "`sort::page`",
            "`scan_rows` 0",
            "44,032,544",
            "`sort=tod`",
            "every kept row",
            "`MAX_SCAN_BARS` = 24,000,000",
            "`scan_budget_exceeded`",
            "L-WP81-8",
            "L-SC-3",
            "the_page_orders_only_itself_wherever_the_offset_lands",
        ],
    );
    let window = item(BARS, "pub fn window_cancellable(");
    // D-3326 (WP-31): a non-time sort streamed every kept row once through
    // `scan`, holding at most `offset + limit` of them, and still ordered only
    // the page. D-3349 (WP-39): a VALUE sort is served from the months' sort
    // indexes (`sort::page`), reading only rows no index run orders, under the
    // same caps; `scan` remains the time-of-day sort's, which still reads
    // every kept row. AF-05 rewrote the bullet with those measurements.
    let scan = item(BARS, "fn scan(");
    assert!(
        window.contains("sort::page(")
            && window.contains("scan(")
            // EVERY KEPT ROW, through the one stretch reader, which names a
            // month that will not read once (WP-39 repair 1).
            && scan.contains("plan.read_stretch(")
            && scan.contains("(run.start, run.end),"),
        "a value sort is indexed and a time-of-day sort still reads: {window}\n{scan}"
    );
    assert!(
        scan.contains("page_of(kept, offset, limit, order)"),
        "{scan}"
    );
    assert!(
        scan.contains("kept = page_of(std::mem::take(&mut kept), 0, depth, order);"),
        "the rows kept while streaming are bounded by the page's depth: {scan}"
    );
    assert!(
        !window.contains("all.sort_by(order)") && !scan.contains("want - 1"),
        "nothing outside the page is ordered: {window}"
    );
    assert!(BARS.contains("pub const MAX_WINDOW_MONTHS: usize = 240;"));
    assert!(BARS.contains("pub const MAX_SCAN_BARS: u64 = 24_000_000;"));
}

#[test]
fn w1_api5_5_the_calendar_route_sorts_its_feed_per_request_and_its_doc_says_so() {
    names(
        "W1-api5-5",
        &[
            "`calendar_json`",
            "`held_entries`",
            "O(E_v log E_v)",
            "`agree`",
            "corrected",
        ],
    );
    // D-3336 (WP-35) REMOVED THE PER-REQUEST SORT: a request is answered from
    // the answer kept under the feed's manifest stamp, and only a miss walks
    // the census, once over its keys with no sort and no copy (`held_series`).
    // The D-1446 bullet above still prices the old cost; WP-99 rewrites it.
    let route = item(CALENDAR_ROUTE, "pub(super) fn calendar_json_reading(");
    assert!(
        !route.contains("census::held_entries("),
        "no request collects and sorts the feed's entries: {route}"
    );
    assert!(
        route.contains("site.calendars.answer(&key, at)"),
        "a request asks the kept answer first: {route}"
    );
    let walk = item(CALENDAR_ROUTE, "fn held_series(");
    assert!(
        walk.contains("manifest.held_keys()") && !walk.contains("held_entries"),
        "a miss walks the census's keys once: {walk}"
    );
    assert!(
        SERVER.contains(
            "/// request, and the exchange branch adds three key `String`s and a deep clone"
        ),
        "the route's own doc names the per-request sort"
    );
}

#[test]
fn w1_api5_6_a_filtered_store_page_walks_and_copies_every_entry() {
    names(
        "W1-api5-6",
        &["`store_html`", "`census::filtered`", "O(E)", "O(page)"],
    );
    let filtered = item(CENSUS, "pub fn filtered<'a>(");
    assert!(filtered.contains("Cow::Borrowed(entries)"), "{filtered}");
    assert!(filtered.contains("Cow::Owned("), "{filtered}");
    assert!(filtered.contains(".filter(|&&(series, month)| filter.keeps(&series, month))"));
    assert!(
        item(SERVER, "pub fn store_html(").contains("census::filtered(&entries, &filter)"),
        "the page filters the whole entry list"
    );
}

#[test]
fn w1_api5_7_a_scrub_walks_the_append_log_and_opens_a_file_per_entry() {
    names(
        "W1-api5-7",
        &[
            "`verify_json`",
            "`Manifest::newest`",
            "O(log length)",
            "O(E_v) file opens",
            "corrected",
        ],
    );
    assert!(VERIFY.contains("for entry in manifest.newest() {"));
    let newest = MANIFEST
        .split_once("    pub fn newest(&self) -> Vec<Entry> {")
        .expect("Manifest::newest")
        .1
        .split_once("\n    }\n")
        .expect("its body ends")
        .0;
    assert!(
        newest.contains("for held in self.log.iter().rev()"),
        "{newest}"
    );
    assert!(
        SERVER.contains("/// `O(log length)` in memory plus `O(E_v)` file opens."),
        "the route's own doc names the log walk"
    );
}

/// W1-api5-8, CHANGED BY AF-05 (p99-8, D-4972, narrowing D-1446). A window
/// past the last bar read the whole month to answer `[]`; it now reads the
/// last record once and answers from it, and only a tail that is not the
/// header's last bar keeps the full read. The bullet names both costs and
/// the test that counts them; the source keeps the decision and the
/// fallback.
#[test]
fn w1_api5_8_a_window_past_the_last_bar_reads_one_record() {
    names(
        "W1-api5-8",
        &[
            "`bars_json`",
            "`past_the_last_bar`",
            "one record",
            "`n_valid`",
            "zero-filled records",
            "O(n_valid) reads",
            "a_window_past_the_last_bar_reads_one_record",
            "a_zero_filled_tail_still_takes_the_full_read",
            "n = 200",
        ],
    );
    // THE BODY MOVED ONTO THE BLOCKING POOL (D-3326, DB-066): the handler
    // admits it and `month_answer` reads the month.
    let route = item(BARS_ROUTE, "fn month_answer(");
    let (before, after) = route
        .split_once("past_the_last_bar(")
        .expect("the past-the-end decision is taken in `month_answer`");
    assert!(
        before.contains("file.first_at_or_after(at)")
            && after.contains("file.read_record(")
            && after.contains(r#"return Ok((axum::http::StatusCode::OK, "[]".to_owned()));"#),
        "decided after the lookup, from one record read, answering `[]`: {route}"
    );
    assert!(
        after.contains("let begins = landed.filter(|&index| index < held).unwrap_or(0);"),
        "every other landing past the end still falls back to the whole month: {route}"
    );
    let decision = item(BARS_ROUTE, "fn past_the_last_bar(");
    assert!(
        decision.contains("held.checked_sub(1).filter(|_| landed >= held)")
            && decision.contains("stamp != 0 && stamp == header_last_ts && stamp < from"),
        "only the header's own last bar, older than `from`, answers `[]`: {decision}"
    );
}

#[test]
fn w1_api5_9_the_index_map_rereads_its_catalogue_and_walks_the_universe() {
    names(
        "W1-api5-9",
        &[
            "`indexmap_json`",
            "`indexmap::Published::read`",
            "O(file bytes + U)",
        ],
    );
    // The read moved onto the blocking pool with its walk (D-1508); the cost
    // stated is unchanged, so it is checked where it now runs.
    let handler = item(SERVER, "async fn indexmap_json(");
    assert!(
        handler.contains("run_store_read(move || indexmap_reading(&site, feed))"),
        "{handler}"
    );
    let route = item(SERVER, "fn indexmap_reading(");
    assert!(
        route.contains("crate::indexmap::Published::read(&path)"),
        "{route}"
    );
    assert!(route.contains(".by_key\n        .iter()"), "{route}");
}

#[test]
fn w1_api5_11_spot_targets_and_resolve_walk_the_universe_whatever_they_return() {
    names(
        "W1-api5-11",
        &[
            "`spot_targets`",
            "`resolved_master_rows`",
            "O(U)",
            "O(U log U)",
            "`Swept`",
        ],
    );
    assert!(item(SERVER, "fn spot_targets(").contains(".by_key"));
    let resolve = item(SERVER, "fn resolved_master_rows(");
    assert!(resolve.contains(".by_key"), "{resolve}");
    assert!(resolve.contains("rows.sort_unstable();"), "{resolve}");
}

#[test]
fn section_34_is_corrected_where_it_went_stale() {
    let corrections = section()
        .split_once("### Corrections to earlier sections")
        .expect("the D-1446 section corrects §34")
        .1
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for words in [
        "§34 says the broker path \"is not reachable today\"",
        "D-0136",
        "`append_locked`",
    ] {
        assert!(corrections.contains(words), "{words}: {corrections}");
    }
    assert!(
        INGEST.contains("fn append_locked("),
        "the positional installer the correction names exists"
    );
    assert!(
        SERVER.contains("// D-0136.") && SERVER.contains("THE TARGET GUARD IS GONE"),
        "and the guard §34 relied on is gone"
    );
}

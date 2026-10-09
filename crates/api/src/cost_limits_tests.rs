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
const BARS: &str = include_str!("bars.rs");
const CENSUS: &str = include_str!("census.rs");
const VERIFY: &str = include_str!("verify.rs");
const INGEST: &str = include_str!("../../pull/src/ingest.rs");
const MANIFEST: &str = include_str!("../../pull/src/manifest.rs");

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
    for head in ["fn from_members_inner(", "fn record_all("] {
        assert!(
            item(INGEST, head).contains("read_census(&census_path, vendor)"),
            "`{head}` reads the whole census on every call"
        );
    }
    assert!(
        item(INGEST, "fn read_census(").contains("fs::read(path)"),
        "and the read is the whole file"
    );
    assert!(
        item(SERVER, "async fn roll_one(").contains("pull::ingest::record_held("),
        "`roll_one` records through the whole-census read"
    );
}

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
    let miss = item(SERVER, "fn census_now_stamping(");
    assert!(
        miss.contains("let mut censuses = read(&site.store_root);"),
        "{miss}"
    );
    assert!(miss.contains("census::held_entries(&censuses)"), "{miss}");
    assert!(
        item(CENSUS, "pub fn held_entries(").contains("sort_unstable"),
        "the entries are sorted on every miss"
    );
}

#[test]
fn w1_api5_3_the_instrument_list_walks_the_universe_and_every_census_entry() {
    names(
        "W1-api5-3",
        &[
            "`instruments_json`",
            "O(U)",
            "`bars_by_symbol`",
            "O(E)",
            "O(T log T)",
            "Since D-2285",
            "O(answer bytes)",
        ],
    );
    assert!(
        item(SERVER, "async fn instruments_json(")
            .contains(".of_census(&censuses, universe.generation, feed,"),
        "the route serves the kept answer"
    );
    let route = item(SERVER, "fn instruments_answer(");
    assert!(
        route.contains(".by_key\n        .iter()\n        .filter("),
        "{route}"
    );
    assert!(route.contains("bars_by_symbol(censuses.iter()"), "{route}");
    assert!(route.contains("entries.iter()"), "{route}");
    assert!(route.contains("listing.sort_unstable_by_key("), "{route}");
    assert!(
        item(SERVER, "fn bars_by_symbol<'a>(").contains("for (series, month) in entries"),
        "every entry is walked"
    );
}

#[test]
fn w1_api5_4_the_window_still_reads_its_range_and_orders_only_the_page() {
    names(
        "W1-api5-4",
        &[
            "`bars::window`",
            "`page_of`",
            "O(limit log limit)",
            "O(n) reads and O(n) memory",
            "`MAX_WINDOW_MONTHS` = 240",
            "the_page_orders_only_itself_wherever_the_offset_lands",
            "Since D-4439",
            "`month_fold`",
            "`MAX_SCAN_WINDOW_RECORDS` = 1,048,576",
            "p99",
            "a_ts_window_with_extremes_reads_each_month_once_until_it_moves",
        ],
    );
    let window = item(BARS, "pub fn window(");
    // The read moved into `read_in_time`, which carries the lookback across
    // months (D-1762); the window still calls it for every non-seek request.
    let read = item(BARS, "fn read_in_time(");
    assert!(
        window.contains("read_in_time(") && read.contains("slots(file, 0, held)"),
        "every bar is read: {window}\n{read}"
    );
    assert!(
        window.contains("page_of(all, offset, limit, order)"),
        "{window}"
    );
    assert!(
        window.contains("let fold = month_fold(file, before);"),
        "a ts window takes its extremes from the kept month folds: {window}"
    );
    assert!(window.contains("scan_admitted(total)?;"), "{window}");
    assert!(BARS.contains("pub const MAX_SCAN_WINDOW_RECORDS: u64 = 1 << 20;"));
    assert!(
        !window.contains("all.sort_by(order)") && !window.contains("want - 1"),
        "nothing outside the page is ordered: {window}"
    );
    assert!(BARS.contains("pub const MAX_WINDOW_MONTHS: usize = 240;"));
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
            "Since D-2286",
        ],
    );
    assert!(
        item(SERVER, "fn calendar_json_reading(")
            .contains(".of_census(&fresh, 0, (feed, symbol.clone(), stamp),"),
        "the route serves the kept answer"
    );
    let route = item(SERVER, "fn calendar_answer(");
    assert_eq!(
        route
            .matches("census::held_entries(std::slice::from_ref(")
            .count(),
        2,
        "both branches collect and sort the feed's entries: {route}"
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
        &[
            "`store_html`",
            "`census::filtered`",
            "O(E)",
            "O(page)",
            "Since D-2289",
            "`STORE_FILTERS_KEPT` = 8",
            "SUBSTRING",
        ],
    );
    assert!(SERVER.contains("const STORE_FILTERS_KEPT: usize = 8;"));
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
fn w1_api5_7_a_scrub_opens_one_page_and_walks_the_log_once_per_snapshot() {
    names(
        "W1-api5-7",
        &[
            "`verify_json`",
            "`Manifest::newest`",
            "O(log length)",
            "Since D-4435",
            "`MAX_VERIFY_PAGE` = 1,024",
            "`next_offset`",
            "`verify_memo`",
            "p99",
            "scrub_route_opens_no_more_than_a_page_of_a_larger_counter",
        ],
    );
    let vendor = item(VERIFY, "pub fn vendor(");
    assert!(vendor.contains("for entry in page {"), "{vendor}");
    assert!(
        vendor.contains("if limit == 0 || limit > MAX_VERIFY_PAGE {"),
        "{vendor}"
    );
    assert!(!vendor.contains(".newest()"), "the page is cut, not walked");
    assert!(VERIFY.contains("pub const MAX_VERIFY_PAGE: u64 = 1_024;"));
    assert!(
        item(VERIFY, "pub fn newest(").contains("Some(Arc::new(manifest.newest()))"),
        "the log walk is the memo's build"
    );
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
    // The scrub runs on the store-read pool, never on the handler's own task
    // (W1-api6-0, D-2281).
    let handler = item(SERVER, "async fn verify_json(");
    assert!(
        handler.contains("verify_reading(&site, feed, &asked, offset, limit)"),
        "{handler}"
    );
    assert!(handler.contains("run_store_read(move || {"), "{handler}");
    assert!(!handler.contains("crate::verify::vendor("), "{handler}");
    assert!(!handler.contains("census_now("), "{handler}");
    let reading = item(SERVER, "fn verify_reading(");
    assert!(
        reading.contains("site.verify_memo.of_census(&censuses, 0, feed, || {"),
        "{reading}"
    );
    assert!(reading.contains("crate::verify::vendor("), "{reading}");
}

#[test]
fn w1_api5_8_a_window_past_the_last_bar_reads_the_month_only_when_unsealed() {
    names(
        "W1-api5-8",
        &[
            "`bars_json`",
            "`n_valid`",
            "O(n_valid) reads",
            "zero-filled records",
            "Since D-2280 only in a month born without `FLAG_CHECKSUMS`",
            "`ceil(log2(n_valid + 1))` reads",
            "Since D-4432",
            "`last_ts_micros`",
            "one record read",
            "bars_json_past_the_headers_last_stamp_reads_one_record",
        ],
    );
    let route = item(SERVER, "async fn bars_json(");
    assert!(
        route.contains(
            "if from_micros.is_some_and(|at| past_the_last_bar(&file, at)) {\n        \
             return (axum::http::StatusCode::OK, json(), \"[]\".to_owned());"
        ),
        "past the header's last stamp, one record read answers: {route}"
    );
    let past = item(SERVER, "fn past_the_last_bar(");
    assert!(
        past.contains("if from <= header.last_ts_micros {"),
        "{past}"
    );
    assert!(
        past.contains("file.read_record(last).is_ok_and(|bar| {"),
        "{past}"
    );
    assert!(
        route.contains(
            "if landed == Some(held) && file.header().checksums_present() {\n        \
             return (axum::http::StatusCode::OK, json(), \"[]\".to_owned());"
        ),
        "a sealed landing past the end answers from the bisection: {route}"
    );
    assert!(
        route.contains("let begins = landed.filter(|&index| index < held).unwrap_or(0);"),
        "an unsealed landing past the end still falls back to the whole month: {route}"
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
            "Since D-2287",
        ],
    );
    // The read moved onto the blocking pool with its walk (D-1508); the cost
    // stated is unchanged, so it is checked where it now runs.
    let handler = item(SERVER, "async fn indexmap_json(");
    assert!(
        handler.contains("run_store_read(move || indexmap_reading(&site, feed))"),
        "{handler}"
    );
    assert!(
        item(SERVER, "fn indexmap_reading_at(")
            .contains("site.indexmap_memo.get(&stamp, generation, feed,"),
        "the read serves the kept answer"
    );
    let route = item(SERVER, "fn indexmap_answer(");
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
            "Since D-2288",
            "O(|target|)",
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

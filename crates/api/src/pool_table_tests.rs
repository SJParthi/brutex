#![cfg(test)]
//! Every route and the pool it runs in; a page read that meets a writer; a
//! read whose reader leaves (R3-08: NEW-04, NEW-21; RA-009, RA-020, RA-060,
//! RA-088, RA-117, RA-130).
//!
//! # The table
//!
//! [`ROUTES`] names every route `route_table` registers and where its work
//! runs: in a named blocking pool (with the handler, or the helper it hands
//! the work to, and the call that takes the pool), in memory on the async
//! worker, or as a write a POST starts. No route reads a file on the async
//! worker any more: the last one, `/feeds.json`, moved into the list pool
//! with `/settings.json` (R3-08), so the debt row this table had is gone and
//! a route that reads a file must name a pool.
//! [`every_store_route_names_its_pool`] reads the route table out of
//! `server.rs` and fails for a route the table does not name, for a
//! row whose route is gone, and for a pooled row whose handler no longer
//! makes the call it names. A route added later names its pool here in its
//! own change.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a failed fixture, socket or source read must fail the test by name"
)]

use super::*;
use crate::detail::Named;

/// Where one route's work runs.
#[derive(Debug, Clone, Copy)]
enum Runs {
    /// In a named pool. `file` and `function` hold the call `takes`, which
    /// is the call that takes that pool: the handler itself, or the helper
    /// it hands its blocking work to.
    Pool {
        named: Named,
        file: &'static str,
        function: &'static str,
        takes: &'static str,
    },
    /// Touches no file: memory, a constant, a page shell. Why.
    Memory(&'static str),
    /// Starts or steers a write (a POST): where its blocking work goes.
    Write(&'static str),
}

/// A pooled row.
const fn pool(
    named: Named,
    file: &'static str,
    function: &'static str,
    takes: &'static str,
) -> Runs {
    Runs::Pool {
        named,
        file,
        function,
        takes,
    }
}

const RUN: &str = "crate::detail::run(";
const STORE_READ: &str = "crate::detail::run_store_read(";

/// Every registered route and where it runs.
const ROUTES: &[(&str, Runs)] = &[
    (
        "/dashboard",
        Runs::Memory("renders the universe loaded at start"),
    ),
    ("/instruments", Runs::Memory("a page shell")),
    (
        "/instruments.json",
        pool(
            Named::StoreRead,
            "instruments_route.rs",
            "instruments_json",
            STORE_READ,
        ),
    ),
    (
        "/feeds.json",
        pool(
            Named::List,
            "feeds_route.rs",
            "feeds_json",
            "crate::detail::run_list(",
        ),
    ),
    (
        "/settings.json",
        pool(
            Named::List,
            "settings_route.rs",
            "settings_json",
            "crate::detail::run_list(",
        ),
    ),
    (
        "/settings",
        Runs::Write("saves one new settings generation on its own blocking task"),
    ),
    (
        "/universes.json",
        Runs::Memory("the universe loaded at start"),
    ),
    (
        "/calendar.json",
        pool(
            Named::Calendar,
            "calendar_route.rs",
            "lead",
            "crate::detail::run_calendar(",
        ),
    ),
    ("/vocab.json", Runs::Memory("the compiled vocabulary table")),
    (
        "/indexmap.json",
        pool(Named::StoreRead, "server.rs", "indexmap_json", STORE_READ),
    ),
    (
        "/folder.json",
        pool(Named::StoreRead, "folder.rs", "folder_json", STORE_READ),
    ),
    (
        "/bars.json",
        pool(
            Named::StoreRead,
            "bars_route.rs",
            "bars_json",
            "cancellable(false,",
        ),
    ),
    (
        "/gaps.json",
        pool(
            Named::Calendar,
            "gaps_route.rs",
            "audit_span",
            "crate::detail::cancellable_in(crate::detail::Named::Calendar,",
        ),
    ),
    (
        "/bars/window.json",
        pool(
            Named::Scan,
            "bars_route.rs",
            "bars_window_json",
            "cancellable(heavy, work)",
        ),
    ),
    (
        "/spot.json",
        pool(
            Named::StoreRead,
            "spot.rs",
            "spot_json",
            "super::bars_route::cancellable(false,",
        ),
    ),
    (
        "/greeks.json",
        pool(
            Named::StoreRead,
            "optiongreeks.rs",
            "greeks_json",
            "super::bars_route::cancellable(false,",
        ),
    ),
    (
        "/census/facets.json",
        pool(Named::StoreRead, "facets.rs", "facets_json", STORE_READ),
    ),
    // THE TWO CENSUS ROUTES R3-09 ADDED, named on the merge of feat/db-fix
    // (this test refused both until they were): the warm table is memory, the
    // roll-up reads the census in the store-read pool.
    (
        "/census/warm.json",
        Runs::Memory("the census warm rows kept in memory (R3-09): one lock, no file"),
    ),
    (
        "/census/rollup.json",
        pool(
            Named::StoreRead,
            "census_rollup.rs",
            "rollup_json",
            STORE_READ,
        ),
    ),
    (
        "/chain.json",
        pool(Named::StoreRead, "chain.rs", "chain_json", STORE_READ),
    ),
    (
        "/imports.json",
        pool(Named::StoreRead, "imports.rs", "imports_json", STORE_READ),
    ),
    (
        "/disk.json",
        pool(
            Named::OpsRead,
            "disk_route.rs",
            "disk_json",
            "crate::detail::run_ops_read(",
        ),
    ),
    (
        "/scrub.json",
        pool(
            Named::OpsRead,
            "scrub_route.rs",
            "scrub_json",
            "crate::detail::run_ops_read(",
        ),
    ),
    (
        "/ingest/refusals.json",
        pool(
            Named::StoreRead,
            "ingest_refusals.rs",
            "refusals_json",
            STORE_READ,
        ),
    ),
    (
        "/dayflags.json",
        pool(Named::StoreRead, "dayflags.rs", "dayflags_json", STORE_READ),
    ),
    ("/aliases.json", Runs::Memory("the compiled alias table")),
    (
        "/sources.json",
        pool(Named::StoreRead, "sources.rs", "sources_json", STORE_READ),
    ),
    (
        "/sources/bars.json",
        pool(
            Named::StoreRead,
            "sources.rs",
            "sources_bars_json",
            STORE_READ,
        ),
    ),
    (
        "/sources/day.json",
        pool(
            Named::StoreRead,
            "sources.rs",
            "sources_day_json",
            STORE_READ,
        ),
    ),
    (
        "/provenance.json",
        pool(
            Named::StoreRead,
            "provenance.rs",
            "provenance_json",
            STORE_READ,
        ),
    ),
    (
        "/store.json",
        pool(
            Named::Census,
            "store_route.rs",
            "store_json",
            "crate::detail::run_census(",
        ),
    ),
    (
        "/verify.json",
        pool(
            Named::Verify,
            "verify.rs",
            "vendor_off_worker",
            "crate::detail::run_verify(",
        ),
    ),
    (
        "/pools.json",
        Runs::Memory("the pools' own counters; it must answer while every pool is full"),
    ),
    (
        "/speed.json",
        Runs::Memory("the process's own per-route timings (R3-14), from memory"),
    ),
    (
        "/page-report",
        Runs::Write("hands one rationed page report to the telemetry sink (R3-14)"),
    ),
    (
        "/masters.js",
        Runs::Memory("the masters script rendered at start"),
    ),
    (
        "/typeahead.js",
        Runs::Memory("the typeahead script rendered at start"),
    ),
    ("/pull", Runs::Memory("a page shell")),
    (
        "/pull/spot",
        Runs::Write("starts a pull run on its own thread"),
    ),
    (
        "/pull/repair",
        Runs::Write("starts a month repair on the pull run's thread"),
    ),
    (
        "/pull/repair.json",
        pool(
            Named::StoreRead,
            "repair_route.rs",
            "repair_plan_json",
            STORE_READ,
        ),
    ),
    (
        "/pull/fno",
        Runs::Write("starts a pull run on its own thread"),
    ),
    (
        "/pull/run",
        Runs::Write("starts a pull run on its own thread"),
    ),
    (
        "/pull/recovery",
        pool(Named::StoreRead, "recovery.rs", "journal_view", STORE_READ),
    ),
    (
        "/pull/recovery.json",
        pool(Named::StoreRead, "recovery.rs", "journal_view", STORE_READ),
    ),
    (
        "/pull/run.json",
        Runs::Memory("the run slot's progress in memory"),
    ),
    (
        "/pull/run/stop",
        Runs::Write("sets the run's stop flag in memory (RA-020: owner IA-10a for the journal)"),
    ),
    (
        "/autopilot.json",
        Runs::Memory("the autopilot's state in memory"),
    ),
    (
        "/universe/resolve",
        Runs::Write("resolves a typed universe against masters in memory"),
    ),
    (
        "/autopilot/pause",
        Runs::Write("flips the autopilot's state in memory"),
    ),
    (
        "/autopilot/resume",
        Runs::Write("flips the autopilot's state in memory"),
    ),
    (
        "/autopilot/control",
        Runs::Write("flips the autopilot's state in memory"),
    ),
    (
        "/ingest/status.json",
        Runs::Memory("the ingest queue's state in memory"),
    ),
    (
        "/ingest/queue",
        Runs::Write("queues an import; the import runs on the ingest worker threads"),
    ),
    (
        "/audit.json",
        pool(Named::StoreRead, "audit_json.rs", "audit_json", STORE_READ),
    ),
    ("/audit/page", Runs::Memory("a page shell")),
    ("/store", Runs::Memory("a page shell")),
    ("/bars", Runs::Memory("a page shell")),
    (
        "/logs",
        pool(
            Named::LogRead,
            "logs.rs",
            "logs_page",
            "crate::detail::run_log_read(",
        ),
    ),
    (
        "/logs.json",
        pool(
            Named::LogRead,
            "logs.rs",
            "logs_json",
            "crate::detail::run_log_read(",
        ),
    ),
    (
        "/trades.json",
        pool(Named::Detail, "trades.rs", "trades_json", RUN),
    ),
    (
        "/frontier.json",
        pool(Named::Detail, "frontierjson.rs", "frontier_json", RUN),
    ),
    (
        "/sweep-evidence.json",
        pool(Named::Detail, "sweepevidence.rs", "evidence_json", RUN),
    ),
    (
        "/candidate-trades.json",
        pool(Named::Detail, "candidatejson.rs", "candidate_json", RUN),
    ),
    (
        "/expression-search.json",
        pool(
            Named::Detail,
            "expressionsearchjson.rs",
            "expression_search_json",
            RUN,
        ),
    ),
    (
        "/boolean-candidates.json",
        pool(Named::Detail, "booleanjson.rs", "boolean_json", RUN),
    ),
    (
        "/boolean-statistics.json",
        pool(Named::Detail, "booleanevidencejson.rs", "serve", RUN),
    ),
    (
        "/boolean-admission.json",
        pool(Named::Detail, "booleanevidencejson.rs", "serve", RUN),
    ),
    (
        "/boolean-qualification.json",
        pool(Named::Detail, "booleanevidencejson.rs", "serve", RUN),
    ),
    (
        "/boolean-campaign.json",
        pool(Named::Detail, "booleancampaignjson.rs", "serve", RUN),
    ),
    (
        "/boolean-qualified-campaign.json",
        pool(Named::Detail, "booleancampaignjson.rs", "serve", RUN),
    ),
    (
        "/boolean-qualified-search.json",
        pool(Named::Detail, "booleansearchjson.rs", "search_json", RUN),
    ),
    (
        "/boolean-oos.json",
        pool(Named::Detail, "booleanoosjson.rs", "later_json", RUN),
    ),
    (
        "/selection-v6.json",
        pool(
            Named::Detail,
            "selectionv6json.rs",
            "selection_v6_json",
            RUN,
        ),
    ),
    (
        "/live.json",
        pool(Named::Detail, "livejson.rs", "live_json", RUN),
    ),
    (
        "/backtest.json",
        pool(Named::Detail, "backtest.rs", "backtest_json", RUN),
    ),
    (
        "/backtest/run",
        Runs::Write("journals and launches a sweep through the detail pool (`detail::run`)"),
    ),
    (
        "/backtest/descend",
        Runs::Write("journals and launches a descent through the detail pool (`detail::run`)"),
    ),
    (
        "/engine/command",
        Runs::Write("journals and launches a command through the detail pool (`detail::run`)"),
    ),
    (
        "/engine/boolean-launch.json",
        pool(Named::Detail, "booleanlaunch.rs", "metadata", RUN),
    ),
    (
        "/index-stop.json",
        pool(Named::Detail, "indexstopjson.rs", "index_stop_json", RUN),
    ),
    (
        "/index-stop-candles.json",
        pool(
            Named::Detail,
            "indexstopcandlesjson.rs",
            "index_stop_candles_json",
            RUN,
        ),
    ),
    (
        "/engine/index-stop-launch.json",
        pool(Named::Detail, "indexstoplaunch.rs", "metadata", RUN),
    ),
    (
        "/index-stop-qualification.json",
        pool(
            Named::Detail,
            "indexstopqualificationjson.rs",
            "index_stop_qualification_json",
            RUN,
        ),
    ),
    (
        "/index-stop-ranking.json",
        pool(
            Named::Detail,
            "indexstoprankingjson.rs",
            "index_stop_ranking_json",
            RUN,
        ),
    ),
    (
        "/index-stop-vix.json",
        pool(
            Named::Detail,
            "indexstopvixjson.rs",
            "index_stop_vix_json",
            RUN,
        ),
    ),
    (
        "/engine/top.json",
        pool(Named::Detail, "topjson.rs", "top_json", RUN),
    ),
    ("/masters", Runs::Memory("a page shell")),
    (
        "/masters/refresh",
        Runs::Write("starts a masters refresh on its own thread"),
    ),
    (
        "/masters/status.json",
        Runs::Memory("the refresh's state in memory"),
    ),
    (
        "/backtest/run.json",
        pool(Named::Detail, "sweeprun.rs", "run_json", RUN),
    ),
    (
        "/backtest/audit.json",
        pool(Named::Detail, "operation_audit.rs", "audit_json", RUN),
    ),
    ("/health", Runs::Memory("the universe loaded at start")),
];

/// This crate's source file, read at test time.
fn source(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|why| panic!("{} could not be read: {why}", path.display()))
}

/// The body of `fn <name>` in `text`, from its signature to its closing
/// brace, or `None` when no such function is there.
///
/// Braces inside string, raw-string and character literals and inside line
/// comments are not counted: a handler that writes `"{{{}"` into a body
/// (the being-written answer does) would otherwise end its own body early or
/// never.
fn body_of<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = [format!("fn {name}("), format!("fn {name}<")]
        .iter()
        .filter_map(|needle| text.find(needle.as_str()))
        .min()?;
    let open = start + text[start..].find('{')?;
    let bytes = text.as_bytes();
    let at = |index: usize| bytes.get(index).copied().unwrap_or(0);
    let mut depth = 0_usize;
    let mut index = open;
    while index < bytes.len() {
        match at(index) {
            b'/' if at(index + 1) == b'/' => {
                index += text[index..].find('\n').unwrap_or(text.len() - index);
            }
            b'"' => {
                index += 1;
                while index < bytes.len() && at(index) != b'"' {
                    index += if at(index) == b'\\' { 2 } else { 1 };
                }
            }
            b'r' if matches!(at(index + 1), b'"' | b'#')
                && !at(index.wrapping_sub(1)).is_ascii_alphanumeric()
                && at(index.wrapping_sub(1)) != b'_' =>
            {
                let hashes = text[index + 1..].bytes().take_while(|b| *b == b'#').count();
                if at(index + 1 + hashes) == b'"' {
                    let close = format!("\"{}", "#".repeat(hashes));
                    let body = index + 2 + hashes;
                    index = body + text[body..].find(close.as_str())? + close.len() - 1;
                }
            }
            b'\'' if at(index + 1) == b'\\' => {
                index += 3 + text.get(index + 3..)?.find('\'')?;
            }
            b'\'' if at(index + 2) == b'\'' => index += 2,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=index]);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Every path `route_table` registers, read from `server.rs` itself: the
/// first string literal after each `.route(` outside a comment.
fn registered_routes() -> Vec<String> {
    let server = source("server.rs");
    let table =
        body_of(&server, "route_table").expect("server.rs builds its routes in route_table");
    let code: String = table
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    code.match_indices(".route(")
        .filter_map(|(at, call)| {
            let after = code[at + call.len()..].trim_start();
            let literal = after.strip_prefix('"')?;
            Some(literal[..literal.find('"')?].to_owned())
        })
        .collect()
}

/// **EVERY STORE ROUTE NAMES ITS POOL** (R3-08, NEW-04). Fails for a route
/// the table does not name, a row whose route is gone, a route named twice,
/// and a pooled row whose handler no longer takes the pool it names.
#[test]
fn every_store_route_names_its_pool() {
    let registered = registered_routes();
    assert!(
        registered.len() >= 80,
        "the route table was not read: {registered:?}"
    );
    let named: Vec<&str> = ROUTES.iter().map(|(route, _)| *route).collect();
    let mut seen = std::collections::HashSet::new();
    for route in &named {
        assert!(
            seen.insert(*route),
            "{route} is named twice in the pool table"
        );
    }
    let unnamed: Vec<&String> = registered
        .iter()
        .filter(|route| !named.contains(&route.as_str()))
        .collect();
    assert!(
        unnamed.is_empty(),
        "a registered route names no pool; add it to ROUTES: {unnamed:?}"
    );
    let gone: Vec<&&str> = named
        .iter()
        .filter(|route| !registered.iter().any(|held| held == *route))
        .collect();
    assert!(
        gone.is_empty(),
        "the pool table names routes no longer registered: {gone:?}"
    );

    let mut sources = std::collections::HashMap::new();
    for (route, runs) in ROUTES {
        match *runs {
            Runs::Pool {
                named,
                file,
                function,
                takes,
            } => {
                let text = sources.entry(file).or_insert_with(|| source(file));
                let body = body_of(text, function)
                    .unwrap_or_else(|| panic!("{route}: {file} has no fn {function}"));
                assert!(
                    body.contains(takes),
                    "{route} is named in the {} pool, and {file}'s fn {function} no longer \
                     calls {takes}",
                    named.pool().name()
                );
            }
            // A ROW OUTSIDE A POOL SAYS WHY.
            Runs::Memory(why) | Runs::Write(why) => {
                assert!(
                    why.split_whitespace().count() >= 3,
                    "{route}: say why: {why:?}"
                );
            }
        }
    }
    // THE POOLS THE HELPERS TAKE, so a row naming a helper names the right
    // pool: `cancellable` takes the scan pool for a heavy window and the
    // store-read pool otherwise; the verify helper takes the pool of one.
    let bars_route = source("bars_route.rs");
    let cancellable = body_of(&bars_route, "cancellable").expect("bars_route::cancellable");
    assert!(cancellable.contains("crate::detail::Named::Scan"));
    assert!(cancellable.contains("crate::detail::Named::StoreRead"));
    assert!(cancellable.contains("crate::detail::cancellable_in(named, work)"));
    let recovery = source("recovery.rs");
    for handler in ["recent", "page"] {
        let body = body_of(&recovery, handler).expect("a recovery handler");
        assert!(body.contains("journal_view(site).await"), "{handler}");
        assert!(
            !body.contains("latest_view("),
            "{handler} reads the journal on the worker"
        );
    }
    // A POOL IS NAMED BY NAME: no two pools share one.
    let pool_names: std::collections::HashSet<&str> =
        Named::ALL.iter().map(|named| named.pool().name()).collect();
    assert_eq!(pool_names.len(), Named::ALL.len());
    assert_eq!(
        Named::Verify.pool().max(),
        1,
        "the whole-vendor check runs one at a time"
    );
}

/// One HTTP/1.1 GET on a fresh connection, on its own thread; the answer and
/// how long it took.
async fn get(addr: SocketAddr, path: &str) -> (String, std::time::Duration) {
    let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    tokio::task::spawn_blocking(move || {
        use std::io::{Read as _, Write as _};
        let began = std::time::Instant::now();
        let mut socket = std::net::TcpStream::connect(addr).expect("connect");
        socket.write_all(request.as_bytes()).expect("write");
        let mut answer = String::new();
        socket.read_to_string(&mut answer).expect("read");
        (answer, began.elapsed())
    })
    .await
    .expect("the client thread must not panic")
}

/// The status line's code.
fn status_of(answer: &str) -> &str {
    answer.split(' ').nth(1).unwrap_or("")
}

/// The first 600 characters of an answer, for a failure message.
fn head_text(answer: &str) -> String {
    answer.chars().take(600).collect()
}

/// The response head, lower-cased.
fn head_of(answer: &str) -> String {
    answer
        .split_once("\r\n\r\n")
        .map_or(answer, |(head, _)| head)
        .to_ascii_lowercase()
}

/// Serves the production router over `root` on a fresh loopback port while
/// `body` runs, then stops.
async fn live<F, Fut>(root: &Path, body: F)
where
    F: FnOnce(SocketAddr) -> Fut,
    Fut: Future<Output = ()>,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let site = Loaded::new(Site::load(&root.join("masters"), root));
    let router = router_serving(
        site,
        std::sync::Arc::new(assets::Assets::new(&root.join("web"))),
        addr,
    );
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let served = tokio::spawn(serve(
        listener,
        router,
        Box::pin(async move {
            let _ = stopped.await;
            Ok::<(), std::io::Error>(())
        }),
    ));
    body(addr).await;
    let _ = stop.send(());
    served
        .await
        .expect("task")
        .expect("a graceful shutdown is not a failure");
}

/// A private store holding one invented month: Zerodha NSE NIFTY 1min 2024-01,
/// five bars at prices no market printed.
fn one_month(name: &str) -> (PathBuf, store::path::StorePath<'static>, u32) {
    let root = crate::scratch::path(&format!("pool-table-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("masters")).expect("an owned root");
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: store::path::Timeframe::MINUTE_1,
        month: store::path::YearMonth::new(2024, 1).expect("a legal month"),
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id is the cross-check `open` folds; any 32 bits serve"
    )]
    let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let mut file =
        store::file::BarFile::open_or_create(&root, path, symbol_id).expect("a bar file");
    // 2024-01-02 09:15 IST onward, one invented bar a minute.
    let open = (19_724 * 86_400 - 19_800 + 9 * 3_600 + 15 * 60) * 1_000_000;
    let rows: Vec<store::format::Bar> = (0..5)
        .map(|n| store::format::Bar {
            ts_micros: open + n * 60_000_000,
            open: 1_000 + n,
            high: 1_010 + n,
            low: 990 + n,
            close: 1_005 + n,
            volume: 0,
            open_interest: i64::MIN,
        })
        .collect();
    file.append(&rows).expect("five legal bars");
    drop(file);
    (root, path, symbol_id)
}

const MONTH: &str = "/bars.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY\
                     &timeframe=1min&month=2024-01";
const WINDOW: &str = "/bars/window.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY\
                      &timeframe=1min&from=2024-01&to=2024-01&limit=5&sort=ts";

/// **A READER BESIDE A WRITER WAITS, THEN SAYS "BEING WRITTEN"** (R3-08,
/// NEW-21, RA-117). A writer holds the month — the exclusive lock an import
/// worker, a month repair or an index build takes — on a thread of this
/// process. The page read waits [`crate::detail::READ_PATIENCE_MS`], then
/// answers 503 with `Retry-After`, the words "being written, ask again", the
/// holder (this pid, the writer's thread) and never "unreadable"; the window
/// over the same month names it once. When the writer lets go the same read
/// is answered whole.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reader_beside_a_writer_waits_then_says_being_written() {
    let (root, path, symbol_id) = one_month("beside-writer");
    let (holding_tx, holding_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let writer_root = root.clone();
    let writer = std::thread::Builder::new()
        .name("r3-08-test-writer".to_owned())
        .spawn(move || {
            let held = store::file::BarFile::open_or_create(&writer_root, path, symbol_id)
                .expect("the writer takes the month");
            holding_tx.send(()).expect("the test waits for the writer");
            let _ = release_rx.recv();
            drop(held);
        })
        .expect("a writer thread");
    holding_rx.recv().expect("the writer holds the month");
    let pid = std::process::id().to_string();
    let store_named = root.display().to_string();
    live(&root, |addr| async move {
        let written_before = crate::detail::route_tally("/bars.json").being_written;
        let (answer, took) = get(addr, MONTH).await;
        assert_eq!(status_of(&answer), "503", "{answer}");
        let head = head_of(&answer);
        assert!(head.contains("retry-after: 1"), "{head}");
        assert!(answer.contains("being written, ask again"), "{answer}");
        assert!(answer.contains("\"being_written\":true"), "{answer}");
        assert!(
            answer.contains("r3-08-test-writer"),
            "names the holder: {answer}"
        );
        assert!(
            answer.contains(&format!("pid {pid}")),
            "names the process: {answer}"
        );
        assert!(answer.contains(&store_named), "names the store: {answer}");
        assert!(
            !answer.to_ascii_lowercase().contains("unreadable"),
            "{answer}"
        );
        assert!(
            took >= std::time::Duration::from_millis(crate::detail::READ_PATIENCE_MS),
            "the read waited for the writer before it answered: {took:?}"
        );
        assert!(
            crate::detail::route_tally("/bars.json").being_written > written_before,
            "counted as being written, not as busy"
        );

        // THE WINDOW OVER THE SAME MONTH: never 200, never "unreadable", and
        // the month is named once however many of its rows were asked for.
        let (window, _) = get(addr, WINDOW).await;
        assert_ne!(status_of(&window), "200", "{window}");
        assert_eq!(
            window.matches("being written, ask again").count(),
            1,
            "{window}"
        );
        assert!(
            !window.to_ascii_lowercase().contains("unreadable"),
            "{window}"
        );

        release_tx.send(()).expect("the writer lets go");
        writer.join().expect("the writer thread");
        let (answer, _) = get(addr, MONTH).await;
        assert_eq!(status_of(&answer), "200", "{answer}");
        assert_eq!(answer.matches("\"t\":").count(), 5, "{answer}");
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// The first invented bar of [`one_month`], in seconds: 2024-01-02 09:15 IST.
const FIRST_BAR_S: i64 = 19_724 * 86_400 - 19_800 + 9 * 3_600 + 15 * 60;

/// **A SPOT OR GREEKS READ BESIDE A WRITER SAYS "BEING WRITTEN", NEVER
/// "UNREADABLE"** (R3-08, NEW-21, RA-117). The per-instant routes open the
/// month themselves and used to fold a writer's refusal into their faults:
/// 206, every instant `"why":"unreadable"` (the word a damaged file earns),
/// no `Retry-After`, so the page never asked again — measured on a copy of
/// gdfl NIFTY 1s 2019-11 held by `lockf(1)`. With the underlying's month
/// held, `/spot.json` and `/greeks.json` (whose option month is absent here,
/// so the writer is its only fault) answer 503, `Retry-After: 1`,
/// `"being_written":true` and the holder; when the writer lets go both are
/// answered, and the spot is the invented close.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_spot_or_greeks_read_beside_a_writer_says_being_written() {
    let (root, path, symbol_id) = one_month("spot-beside-writer");
    let (holding_tx, holding_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let writer_root = root.clone();
    let writer = std::thread::Builder::new()
        .name("r3-08-spot-writer".to_owned())
        .spawn(move || {
            let held = store::file::BarFile::open_or_create(&writer_root, path, symbol_id)
                .expect("the writer takes the month");
            holding_tx.send(()).expect("the test waits for the writer");
            let _ = release_rx.recv();
            drop(held);
        })
        .expect("a writer thread");
    holding_rx.recv().expect("the writer holds the month");
    let spot = format!(
        "/spot.json?feed=zerodha&exchange=NSE&symbol=NIFTY&timeframe=1min&month=2024-01\
         &t={FIRST_BAR_S}"
    );
    let greeks = format!(
        "/greeks.json?feed=zerodha&exchange=NSE&segment=FNO&symbol=NIFTY\
         &contract=2024-01-25-2150000-CE&timeframe=1min&month=2024-01&t={FIRST_BAR_S}"
    );
    live(&root, |addr| async move {
        for (route, asked) in [("/spot.json", &spot), ("/greeks.json", &greeks)] {
            let before = crate::detail::route_tally(route).being_written;
            let (answer, took) = get(addr, asked).await;
            assert_eq!(status_of(&answer), "503", "{route}: {}", head_text(&answer));
            assert!(
                head_of(&answer).contains("retry-after: 1"),
                "{route}: {answer}"
            );
            assert!(
                answer.contains("\"being_written\":true"),
                "{route}: {answer}"
            );
            assert!(
                answer.contains("being written, ask again"),
                "{route}: {answer}"
            );
            assert!(
                answer.contains("r3-08-spot-writer"),
                "{route} names the holder: {answer}"
            );
            assert!(
                !answer.to_ascii_lowercase().contains("unreadable"),
                "{route}: {answer}"
            );
            assert!(
                took >= std::time::Duration::from_millis(crate::detail::READ_PATIENCE_MS),
                "{route} waited for the writer first: {took:?}"
            );
            assert!(
                crate::detail::route_tally(route).being_written > before,
                "{route} is counted as being written"
            );
        }
        release_tx.send(()).expect("the writer lets go");
        writer.join().expect("the writer thread");
        let (answer, _) = get(addr, &spot).await;
        assert_eq!(status_of(&answer), "200", "{answer}");
        assert!(
            answer.contains("\"close\":1005"),
            "the invented close: {answer}"
        );
        let (answer, _) = get(addr, &greeks).await;
        assert_ne!(status_of(&answer), "503", "{answer}");
        assert!(!answer.contains("being_written"), "{answer}");
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// Appends `n` invented bars to `vendor`'s NSE NIFTY `timeframe` month
/// `year-month` under `root`, from `first_s` (epoch seconds), `step_s` apart
/// and 300 a day on consecutive days, at prices no market printed; the
/// month's path and symbol id, for a writer to hold.
fn add_month(
    root: &Path,
    (vendor, timeframe): (Vendor, store::path::Timeframe),
    (year, month): (u16, u8),
    (first_s, step_s, n): (i64, i64, i64),
) -> (store::path::StorePath<'static>, u32) {
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: store::path::YearMonth::new(year, month).expect("a legal month"),
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id is the cross-check `open` folds; any 32 bits serve"
    )]
    let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let mut file = store::file::BarFile::open_or_create(root, path, symbol_id).expect("a bar file");
    let rows: Vec<store::format::Bar> = (0..n)
        .map(|at| store::format::Bar {
            ts_micros: (first_s + (at / 300) * 86_400 + (at % 300) * step_s) * 1_000_000,
            open: 1_000 + at,
            high: 1_010 + at,
            low: 990 + at,
            close: 1_005 + at,
            volume: 0,
            open_interest: i64::MIN,
        })
        .collect();
    file.append(&rows).expect("legal bars");
    (path, symbol_id)
}

/// A writer on its own thread named `name`, holding `path` until the sender
/// it returns is sent to (or dropped).
fn hold(
    root: &Path,
    path: store::path::StorePath<'static>,
    symbol_id: u32,
    name: &str,
) -> (std::thread::JoinHandle<()>, std::sync::mpsc::Sender<()>) {
    let (holding_tx, holding_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let writer_root = root.to_path_buf();
    let writer = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            let held = store::file::BarFile::open_or_create(&writer_root, path, symbol_id)
                .expect("the writer takes the month");
            holding_tx.send(()).expect("the test waits for the writer");
            let _ = release_rx.recv();
            drop(held);
        })
        .expect("a writer thread");
    holding_rx.recv().expect("the writer holds the month");
    (writer, release_tx)
}

/// The asserts every being-written answer meets: 503, `Retry-After: 1`,
/// `"being_written":true`, the holder named in `error`, no "unreadable", no
/// rows, and the body under what `count_route` reads; counted as being
/// written on `route`, never as failed.
fn says_being_written(route: &str, answer: &str, holder: &str, before: RouteTallyNow) {
    assert_eq!(status_of(answer), "503", "{route}: {}", head_text(answer));
    assert!(
        head_of(answer).contains("retry-after: 1"),
        "{route}: {}",
        head_text(answer)
    );
    let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body);
    // A `Connection: close` answer may be chunked: the JSON is the part
    // between the first `{` and the last `}`.
    let json = body
        .find('{')
        .zip(body.rfind('}'))
        .and_then(|(from, to)| body.get(from..=to))
        .unwrap_or("");
    let parsed: serde_json::Value =
        serde_json::from_str(json).unwrap_or_else(|why| panic!("{route}: {why}: {json}"));
    assert_eq!(parsed["being_written"], true, "{route}: {json}");
    let error = parsed["error"].as_str().unwrap_or("");
    assert!(
        error.contains("being written, ask again"),
        "{route}: {json}"
    );
    assert!(error.contains(holder), "{route} names the holder: {json}");
    assert!(
        parsed.get("bars").is_none(),
        "{route} sends no rows: {json}"
    );
    assert!(
        !json.to_ascii_lowercase().contains("unreadable"),
        "{route}: {json}"
    );
    assert!(
        json.len() <= crate::detail::BEING_WRITTEN_BODY_MAX,
        "{route}: {}",
        json.len()
    );
    let after = crate::detail::route_tally(route);
    assert!(
        after.being_written > before.being_written,
        "{route} is counted as being written"
    );
    assert_eq!(
        after.failed, before.failed,
        "{route} is never counted failed"
    );
}

/// A route's being-written and failed counts now.
#[derive(Clone, Copy)]
struct RouteTallyNow {
    being_written: u64,
    failed: u64,
}

fn tally_now(route: &str) -> RouteTallyNow {
    let row = crate::detail::route_tally(route);
    RouteTallyNow {
        being_written: row.being_written,
        failed: row.failed,
    }
}

/// **A WINDOW PAGE OVER 16 KiB ACROSS A HELD MONTH AND AN UNHELD ONE SAYS
/// "BEING WRITTEN", WITH `Retry-After`, AND IS COUNTED SO** (R3-08 repair,
/// verify-R3-08-r1 blocking 1 and 4). The window used to answer its
/// being-written 503 with every row it had read from the unheld month —
/// 126,649 bytes measured on a shadow server — which is past what
/// `count_route` reads: no `Retry-After`, counted "failed", and the sentence
/// only under `faults`, so the /db grid's last failure read "HTTP 503". The
/// same page without the writer is proved over the bound first, so the case
/// is the one the verifier measured.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_large_window_beside_a_writer_says_being_written_with_retry_after() {
    let (root, path, symbol_id) = one_month("large-window-beside-writer");
    // 2023-12-01 09:15 IST onward: 600 invented bars over two days.
    let _ = add_month(
        &root,
        (Vendor::Zerodha, store::path::Timeframe::MINUTE_1),
        (2023, 12),
        (19_692 * 86_400 - 19_800 + 9 * 3_600 + 15 * 60, 60, 600),
    );
    // THE PAGE SPANS BOTH MONTHS: 500 rows of the unheld December, then the
    // held January's five.
    let page = "/bars/window.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY\
                &timeframe=1min&from=2023-12&to=2024-01&sort=ts&dir=asc&offset=100&limit=600";
    live(&root, |addr| async move {
        let (whole, _) = get(addr, page).await;
        assert_eq!(status_of(&whole), "200", "{}", head_text(&whole));
        assert!(
            whole.len() > crate::detail::BEING_WRITTEN_BODY_MAX,
            "the page is over the bound count_route reads: {}",
            whole.len()
        );
    })
    .await;
    // A FRESH SERVER, so no total of the held month is remembered.
    let (writer, release) = hold(&root, path, symbol_id, "r3-08-window-writer");
    live(&root, |addr| async move {
        let before = tally_now("/bars/window.json");
        let (answer, _) = get(addr, page).await;
        says_being_written("/bars/window.json", &answer, "r3-08-window-writer", before);
        release.send(()).expect("the writer lets go");
        writer.join().expect("the writer thread");
        let (answer, _) = get(addr, page).await;
        assert_eq!(status_of(&answer), "200", "{}", head_text(&answer));
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE CHART'S FOLD, THE DAY FLAGS AND THE DAY TABLE BESIDE A WRITER SAY
/// "BEING WRITTEN", NEVER "UNREADABLE" OR A BARE 503** (R3-08 repair,
/// verify-R3-08-r1 blocking 2 and 3; RA-117). Before the repair
/// `/sources/bars.json` answered `{"error":"being written, ask again: ..."}`
/// with no field and no `Retry-After` (counted failed; the chart never asked
/// again), `/dayflags.json` answered 200 with every day's cell
/// `"unreadable"`, and `/sources/day.json` answered 200 with a note nothing
/// asked again on. Each now answers the one being-written 503, and when the
/// writer lets go each is answered and no cell says "unreadable".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_fold_day_flags_and_day_table_beside_a_writer_say_being_written() {
    let (root, path, symbol_id) = one_month("fold-flags-beside-writer");
    // GDFL's one-second month, which the fold reads for its 1min rung:
    // 2024-01-02 09:15 IST onward, 300 invented bars a second apart.
    let (seconds, seconds_id) = add_month(
        &root,
        (Vendor::Gdfl, store::path::Timeframe::SECOND_1),
        (2024, 1),
        (FIRST_BAR_S, 1, 300),
    );
    let fold = "/sources/bars.json?exchange=NSE&segment=INDEX&symbol=NIFTY&rung=1min\
                &from=2024-01-02&to=2024-01-02&dir=desc&limit=100";
    let flags = "/dayflags.json?feed=zerodha&instrument=NSE-INDEX-NIFTY\
                 &from=2024-01-01&to=2024-01-31&order=asc";
    let day = "/sources/day.json?exchange=NSE&segment=INDEX&symbol=NIFTY\
               &from=2024-01-02&to=2024-01-02";
    let (writer, release) = hold(&root, path, symbol_id, "r3-08-hold-zerodha");
    let (second_writer, second_release) = hold(&root, seconds, seconds_id, "r3-08-hold-gdfl");
    // THE CELLS, read directly: a writer-held month is `being_written` in
    // every cell, its own and its peer's, and once each in `month_faults`,
    // never "unreadable".
    let asked = crate::server::dayflags::asked(
        "feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-01-01&to=2024-01-31&order=asc",
    )
    .expect("a legal ask");
    let body = crate::server::dayflags::answer(&root, &asked);
    let days = body["days"].as_array().expect("days");
    assert_eq!(days.len(), 31);
    assert!(
        days.iter()
            .all(|day| day["held"]["why"] == "being_written"
                && day["peer"]["why"] == "being_written"),
        "{}",
        body["days"]
    );
    assert!(!body.to_string().contains("\"unreadable\""), "{body}");
    assert_eq!(
        body["month_faults"].as_array().map(Vec::len),
        Some(2),
        "{}",
        body["month_faults"]
    );
    live(&root, |addr| async move {
        for (route, asked) in [
            ("/sources/bars.json", fold),
            ("/dayflags.json", flags),
            ("/sources/day.json", day),
        ] {
            let before = tally_now(route);
            let (answer, _) = get(addr, asked).await;
            says_being_written(route, &answer, "r3-08-hold-", before);
        }
        release.send(()).expect("the writer lets go");
        writer.join().expect("the writer thread");
        second_release.send(()).expect("the second writer lets go");
        second_writer.join().expect("the second writer thread");
        for (route, asked) in [
            ("/sources/bars.json", fold),
            ("/dayflags.json", flags),
            ("/sources/day.json", day),
        ] {
            let (answer, _) = get(addr, asked).await;
            assert_eq!(status_of(&answer), "200", "{route}: {}", head_text(&answer));
            assert!(
                !answer.contains("being written") && !answer.contains("\"unreadable\""),
                "{route}: {}",
                head_text(&answer)
            );
        }
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **REAL DAMAGE BESIDE A WRITER-HELD MONTH IS ANSWERED, NEVER HELD BACK**
/// (R3-08 repair 2, verify-R3-08-r2 follow-up on RA-117). The day flags'
/// being-written 503 looked at `month_faults` only, so a page whose gaps
/// ledger would not read, beside a month a writer held, answered the 503 and
/// hid the ledger's failure until the writer let go. The invented store here
/// holds Zerodha NIFTY 1min 2024-01 under a writer and an `audit/zerodha.gaps`
/// that is a directory (a read of it fails `EISDIR`): the answer lists the
/// ledger in `source_faults`, the held month's cells still say
/// `being_written`, and the route answers it rather than the 503.
#[test]
fn day_flags_beside_a_writer_still_answer_a_source_that_would_not_read() {
    let (root, path, symbol_id) = one_month("flags-source-fault-beside-writer");
    let ledger = root.join("audit").join("zerodha.gaps");
    std::fs::create_dir_all(&ledger).expect("a ledger path that is a directory");
    let (writer, release) = hold(&root, path, symbol_id, "r3-08-hold-source-fault");
    let asked = crate::server::dayflags::asked(
        "feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-01-01&to=2024-01-31&order=asc",
    )
    .expect("a legal ask");
    let body = crate::server::dayflags::answer(&root, &asked);
    release.send(()).expect("the writer lets go");
    writer.join().expect("the writer thread");
    assert!(
        crate::server::dayflags::being_written(&body).is_none(),
        "a source that would not read is answered beside the writer, not held back: {body}"
    );
    let sources = body["source_faults"].as_array().expect("source_faults");
    assert_eq!(sources.len(), 1, "{}", body["source_faults"]);
    assert!(
        sources[0]
            .as_str()
            .is_some_and(|said| said.contains("zerodha.gaps")),
        "{}",
        body["source_faults"]
    );
    let days = body["days"].as_array().expect("days");
    assert!(
        days.iter().all(|day| day["held"]["why"] == "being_written"),
        "{}",
        body["days"]
    );
    // THE SAME BODY WITH NO SOURCE FAULT IS STILL THE 503: the gate is the
    // source fault, not a weakened rule.
    let mut quiet = body.clone();
    quiet["source_faults"] = serde_json::json!([]);
    let (status, said) =
        crate::server::dayflags::being_written(&quiet).expect("a writer-only page is the 503");
    assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(said.contains(crate::detail::BEING_WRITTEN_FIELD), "{said}");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A GAPS AUDIT BESIDE A WRITER NEVER COUNTS THE HELD MONTH ABSENT OR ITS
/// MINUTES LOST** (R3-08 repair 2, verify-R3-08-r2 blocking 2; NEW-21). The
/// audit folded the open's refusal into `absent_file`, and the calendar then
/// owed the held month's every minute: a NIFTY 1s 2019-11 held by `lockf(1)`
/// answered 200 with `months_absent` 1 and 7,560 minutes lost, no
/// `Retry-After`, so the gaps page reported damage that was only a busy month
/// and never asked again. Now:
///
/// * the writer as the audit's only fault: the being-written 503, counted so;
/// * a real absence beside it (2023-11 has no file): 200, that month absent,
///   the held one `being_written` with the holder's sentence, owing, holding
///   and losing nothing, and counted in `months_being_written`, never in
///   `months_absent`;
/// * the writer gone: 200, nothing being written.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_gaps_audit_beside_a_writer_never_counts_the_held_month_absent() {
    let (root, path, symbol_id) = one_month("gaps-beside-writer");
    // 2023-12-01 09:15 IST onward: 600 invented bars over two days.
    let _ = add_month(
        &root,
        (Vendor::Zerodha, store::path::Timeframe::MINUTE_1),
        (2023, 12),
        (19_692 * 86_400 - 19_800 + 9 * 3_600 + 15 * 60, 60, 600),
    );
    let held_only = "/gaps.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY\
                     &timeframe=1min&month=2023-12&to=2024-01";
    let beside_absent = "/gaps.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY\
                         &timeframe=1min&month=2023-11&to=2024-01";
    let (writer, release) = hold(&root, path, symbol_id, "r3-08-gaps-writer");
    live(&root, |addr| async move {
        let before = tally_now("/gaps.json");
        let (answer, _) = get(addr, held_only).await;
        says_being_written("/gaps.json", &answer, "r3-08-gaps-writer", before);

        let (answer, _) = get(addr, beside_absent).await;
        assert_eq!(status_of(&answer), "200", "{}", head_text(&answer));
        let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        let json = body
            .find('{')
            .zip(body.rfind('}'))
            .and_then(|(from, to)| body.get(from..=to))
            .unwrap_or("");
        let audit: serde_json::Value = serde_json::from_str(json).expect("gaps JSON");
        assert_eq!(audit["months"], 3, "{json}");
        assert_eq!(audit["months_absent"], 1, "only November is absent: {json}");
        assert_eq!(audit["months_being_written"], 1, "{json}");
        let january = &audit["month"][2];
        assert_eq!(january["month"], "2024-01", "{january}");
        assert!(january["absent_file"].is_null(), "{january}");
        let why = january["being_written"]
            .as_str()
            .expect("the holder's sentence");
        assert!(why.contains(store::flock::BEING_WRITTEN), "{january}");
        assert!(why.contains("r3-08-gaps-writer"), "{january}");
        assert_eq!(january["summary"], "being_written", "{january}");
        assert_eq!(january["expected"], 0, "{january}");
        assert_eq!(january["lost_minutes"], 0, "{january}");
        assert_eq!(january["absent_minutes"], 0, "{january}");
        assert!(
            audit["month"][0]["absent_file"].is_string(),
            "November's absence is still found: {json}"
        );
        assert!(audit["month"][0]["being_written"].is_null(), "{json}");

        release.send(()).expect("the writer lets go");
        writer.join().expect("the writer thread");
        let (answer, _) = get(addr, held_only).await;
        assert_eq!(status_of(&answer), "200", "{}", head_text(&answer));
        assert!(
            answer.contains(r#""months_being_written":0"#),
            "{}",
            head_text(&answer)
        );
        assert!(
            !answer.contains("being written, ask again"),
            "{}",
            head_text(&answer)
        );
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **A DAMAGED MONTH BESIDE A WRITER IS ANSWERED, NOT ASKED AGAIN** (R3-08
/// repair 3, verify-R3-08-r3 blocking; NEW-21). `writers_only` collected only
/// the months that would not open and the session evidence that would not
/// read, so a month that OPENED but held records that would not read was no
/// fault to it: on a shadow of the real store, gdfl NIFTY 1s 2019-10..2020-01
/// with one byte of 2020-01 flipped answered 200 naming 73 unreadable records
/// and a lost minute, and the same request with 2019-11 held by `lockf(1)`
/// answered the being-written 503, naming the writer only. The damage stayed
/// hidden for as long as the writer held its month. Now:
///
/// * free: 200, December's unreadable records named;
/// * January held: still 200, December's unreadable records still named,
///   January `being_written` with the holder's sentence, owing, holding and
///   losing nothing, counted in `months_being_written`, never absent;
/// * the writer gone: 200, nothing being written, the damage still named.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_gaps_audit_beside_a_writer_still_names_a_damaged_month() {
    let (root, path, symbol_id) = one_month("gaps-writer-beside-damage");
    // 2023-12-01 09:15 IST onward: 600 invented bars over two days.
    let (december, _) = add_month(
        &root,
        (Vendor::Zerodha, store::path::Timeframe::MINUTE_1),
        (2023, 12),
        (19_692 * 86_400 - 19_800 + 9 * 3_600 + 15 * 60, 60, 600),
    );
    // ONE BYTE FLIPPED INSIDE RECORD 100, in this test's own scratch file:
    // the checksum block that covers it (records 73..146) no longer verifies.
    let file = root.join(december.to_string());
    let mut bytes = std::fs::read(&file).expect("December's bars");
    let at = usize::try_from(store::format::HEADER_LEN + 100 * store::format::RECORD_STRIDE + 8)
        .expect("an offset inside the file");
    bytes[at] ^= 0xff;
    std::fs::write(&file, &bytes).expect("the damaged copy");
    let range = "/gaps.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY\
                 &timeframe=1min&month=2023-12&to=2024-01";
    let audit_of = |answer: &str| -> serde_json::Value {
        let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        let json = body
            .find('{')
            .zip(body.rfind('}'))
            .and_then(|(from, to)| body.get(from..=to))
            .unwrap_or("");
        serde_json::from_str(json).unwrap_or_else(|why| panic!("gaps JSON: {why}: {json}"))
    };
    let damage_named = |audit: &serde_json::Value, when: &str| {
        let december = &audit["month"][0];
        assert_eq!(december["month"], "2023-12", "{when}: {december}");
        let unreadable = december["unreadable_records"]
            .as_u64()
            .unwrap_or_else(|| panic!("{when}: December's records were read: {december}"));
        assert!(unreadable > 0, "{when}: the damage is named: {december}");
        assert!(december["being_written"].is_null(), "{when}: {december}");
        assert!(december["absent_file"].is_null(), "{when}: {december}");
    };
    let writer_root = root.clone();
    live(&root, |addr| async move {
        let (answer, _) = get(addr, range).await;
        assert_eq!(status_of(&answer), "200", "free: {}", head_text(&answer));
        let free = audit_of(&answer);
        damage_named(&free, "free");
        assert_eq!(free["months_being_written"], 0, "{free}");

        let (writer, release) = hold(&writer_root, path, symbol_id, "r3-08-gaps-damage-writer");
        let (answer, _) = get(addr, range).await;
        assert_eq!(
            status_of(&answer),
            "200",
            "damage beside a writer is answered, not asked again: {}",
            head_text(&answer)
        );
        assert!(
            !head_of(&answer).contains("retry-after"),
            "{}",
            head_text(&answer)
        );
        let held = audit_of(&answer);
        damage_named(&held, "held");
        assert_eq!(held["months"], 2, "{held}");
        assert_eq!(held["months_being_written"], 1, "{held}");
        assert_eq!(held["months_absent"], 0, "{held}");
        let january = &held["month"][1];
        assert_eq!(january["month"], "2024-01", "{january}");
        let why = january["being_written"]
            .as_str()
            .expect("the holder's sentence");
        assert!(why.contains(store::flock::BEING_WRITTEN), "{january}");
        assert!(why.contains("r3-08-gaps-damage-writer"), "{january}");
        assert!(january["absent_file"].is_null(), "{january}");
        assert_eq!(january["expected"], 0, "{january}");
        assert_eq!(january["held"], 0, "{january}");
        assert_eq!(january["lost_minutes"], 0, "{january}");

        release.send(()).expect("the writer lets go");
        writer.join().expect("the writer thread");
        let (answer, _) = get(addr, range).await;
        assert_eq!(
            status_of(&answer),
            "200",
            "released: {}",
            head_text(&answer)
        );
        let released = audit_of(&answer);
        damage_named(&released, "released");
        assert_eq!(released["months_being_written"], 0, "{released}");
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// The being-written answer is for a read whose EVERY fault is a writer:
/// none at all, or any other fault beside one, is no "ask again"; past
/// [`crate::detail::being_written_answer`]'s naming bound the rest are
/// counted, and the body stays small enough for `Retry-After` to be added.
#[test]
fn only_a_read_whose_every_fault_is_a_writer_is_asked_again() {
    let busy = format!(
        "/s/bars/x.lock: {}: another writer holds /s/bars/x.lock",
        store::flock::BEING_WRITTEN
    );
    assert!(crate::detail::being_written_answer(&[]).is_none());
    assert!(
        crate::detail::being_written_answer(&[busy.clone(), "a torn record".to_owned()]).is_none(),
        "damage beside a busy month is answered as damage"
    );
    let (status, body) =
        crate::detail::being_written_answer(std::slice::from_ref(&busy)).expect("one writer");
    assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains(crate::detail::BEING_WRITTEN_FIELD), "{body}");
    assert!(
        body.contains("another writer holds /s/bars/x.lock"),
        "{body}"
    );
    let many = vec![busy.repeat(40); 9];
    let (_, body) = crate::detail::being_written_answer(&many).expect("nine writers");
    assert!(body.contains("and 5 more being written"), "{body}");
    assert!(body.len() <= 16 * 1024, "{}", body.len());
    serde_json::from_str::<serde_json::Value>(&body).expect("the body is JSON");
    // THE BOUND HOLDS WHATEVER THE SENTENCES HOLD (R3-08 repair): a holder's
    // note is written by whoever took the lock, and an escape writes up to six
    // bytes for one character, so sentences of `<` and control characters
    // are the worst case. Before the repair the cap was 4,096 characters,
    // not bytes — up to six times that once escaped, past what `count_route`
    // reads — so such a body lost `Retry-After` and was counted "failed".
    let hostile = format!("{busy} {}{}", "<".repeat(20_000), "\u{1}".repeat(20_000));
    let (_, body) = crate::detail::being_written_answer(&vec![hostile; 9]).expect("nine writers");
    assert!(
        body.len() <= crate::detail::BEING_WRITTEN_BODY_MAX,
        "a being-written body is never past what count_route reads: {}",
        body.len()
    );
    assert!(body.contains(crate::detail::BEING_WRITTEN_FIELD));
    serde_json::from_str::<serde_json::Value>(&body).expect("the body is JSON");
}

/// **A FULL POOL WAITS, THEN SAYS BUSY WITH `Retry-After`, AND IS COUNTED**
/// (R3-08, NEW-04, RA-009). The whole-vendor check has a pool of one: with
/// it full the route waits the admission wait, then answers 429 with
/// `Retry-After`, a bar page beside it is still answered, and `/pools.json`
/// counts it by pool and by route. (The month repair plan and the recovery
/// journal, RA-088, are proved in the store-read pool by
/// `tests/pool_load.rs`.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_full_pool_waits_then_says_busy_and_is_counted() {
    let (root, _, _) = one_month("full-pool");
    let apart = crate::detail::apart_from_slot_owners().await;
    live(&root, |addr| async move {
        {
            let _crowd = crate::detail::crowd(Named::Verify);
            let (answer, took) = get(addr, "/verify.json?feed=zerodha").await;
            assert_eq!(status_of(&answer), "429", "{answer}");
            assert!(head_of(&answer).contains("retry-after: 1"), "{answer}");
            assert!(answer.contains("vendor scrub not admitted"), "{answer}");
            assert!(answer.contains("at most 1 run at once"), "{answer}");
            assert!(
                took >= std::time::Duration::from_millis(crate::detail::ADMISSION_WAIT_MS),
                "a full pool waits before it refuses: {took:?}"
            );
            // A BAR PAGE IS NOT IN THE VERIFY POOL, so it is still answered.
            let (bars, _) = get(addr, MONTH).await;
            assert_ne!(status_of(&bars), "429", "{bars}");
        }
        // THE REPAIR PLAN AND THE RECOVERY JOURNAL in the store-read pool:
        // proved under real load in `tests/pool_load.rs`, its own process,
        // because crowding that pool here would refuse every other test's
        // bar read in this process for as long as the crowd stood.
        let (pools, _) = get(addr, "/pools.json").await;
        assert_eq!(status_of(&pools), "200", "{pools}");
        let body: serde_json::Value =
            serde_json::from_str(pools.split_once("\r\n\r\n").map_or("", |(_, body)| body))
                .expect("pools JSON");
        let verify = body["pools"]
            .as_array()
            .expect("pools")
            .iter()
            .find(|pool| pool["pool"] == "verify")
            .expect("the verify pool is listed");
        assert_eq!(verify["slots"], 1);
        assert!(verify["busy"].as_u64() >= Some(1), "{verify}");
        let routes = body["routes"].as_array().expect("routes");
        let verify_route = routes
            .iter()
            .find(|row| row["route"] == "/verify.json")
            .expect("the route is counted by its registered path");
        assert!(verify_route["busy"].as_u64() >= Some(1), "{verify_route}");
    })
    .await;
    drop(apart);
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE FEED LIST AND THE SETTINGS READ RUN IN THE LIST POOL** (R3-08,
/// NEW-04, RA-020). R3-05 moved both off the async worker onto Tokio's own
/// blocking pool: no name, no bound, no busy answer and no count on any
/// page. They now run in the named list pool, apart from every page read's:
/// both are answered, `/pools.json` serves the pool with its slots from the
/// core count, and counts what it answered. (Before this the pool did not
/// exist, so `pool_count("list", ..)` panics.) The pool is not crowded here
/// on purpose: every `feeds_route` test in this process reads the feed list,
/// and a crowd would refuse them; the busy path is the same `Pool::take` the
/// verify test above proves full.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_feed_list_and_the_settings_read_run_in_the_list_pool() {
    let (root, _, _) = one_month("list-pool");
    let before = pool_count("list", "answered").await;
    let route_before = crate::detail::route_tally("/feeds.json").answered;
    live(&root, |addr| async move {
        for path in ["/feeds.json", "/settings.json"] {
            let (answer, _) = get(addr, path).await;
            assert_eq!(status_of(&answer), "200", "{path}: {answer}");
        }
    })
    .await;
    assert!(
        pool_count("list", "answered").await >= before + 2,
        "the list pool answered both reads"
    );
    assert!(crate::detail::route_tally("/feeds.json").answered > route_before);
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    assert_eq!(
        usize::try_from(pool_count("list", "slots").await).expect("a slot count"),
        cores.clamp(1, crate::detail::MAX_LIST_CONCURRENT),
        "the list pool's slots come from the core count, under its ceiling"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A READ STOPS WHEN ITS READER LEAVES** (R3-08, RA-130). The whole-vendor
/// check looks at the flag between months: set, it stops before the next
/// month and is refused as cancelled, verifying nothing. And a reader that
/// leaves while its request waits for a slot is counted cancelled, by the
/// pool and by the route.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_stops_when_its_reader_leaves() {
    use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
    let root = crate::scratch::path("pool-table-reader-leaves");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("masters")).expect("an owned root");
    let mut manifest = Manifest::open_image(Vendor::Zerodha, &[]).expect("an empty counter");
    for symbol in ["NIFTY", "BANKNIFTY", "FINNIFTY"] {
        let ts = 1_746_144_000_000_000;
        manifest
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: brutex_core::instrument::Exchange::Nse,
                    segment: brutex_core::instrument::Segment::Index,
                    symbol: brutex_core::symbol::Symbol::new(symbol).expect("a symbol"),
                    timeframe: store::path::Timeframe::DAY_1,
                    month: store::path::YearMonth::new(2025, 5).expect("a month"),
                },
                rows: 1,
                first_ts_micros: ts,
                last_ts_micros: ts,
            })
            .expect("an invented census entry");
    }
    let counter = manifest_path(&root, Vendor::Zerodha);
    std::fs::create_dir_all(counter.parent().expect("a parent")).expect("the vendor root");
    std::fs::write(&counter, manifest.image()).expect("the invented counter");
    let site = Loaded::new(Site::load(&root.join("masters"), &root));
    let (_stamps, now) = census_cache::vendor_census(&site, Vendor::Zerodha);
    let census = now.census().expect("the counter is read").clone();

    let whole = crate::verify::vendor_until(&root, &census, &crate::bars::Cancel::never());
    assert_eq!(
        whole.tally.seen(),
        3,
        "a check nobody left walks every month"
    );
    let (cancel, guard) = crate::bars::Cancel::pair();
    drop(guard);
    let stopped = crate::verify::vendor_until(&root, &census, &cancel);
    assert_eq!(
        stopped.tally.seen(),
        0,
        "a check whose reader left reads no month"
    );
    assert!(!stopped.verified());
    assert!(
        stopped
            .refused
            .as_deref()
            .is_some_and(|why| why.starts_with("cancelled: the reader left after 0 of zerodha")),
        "{:?}",
        stopped.refused
    );

    // A READER THAT LEAVES WHILE IT WAITS FOR A SLOT: the client closes its
    // connection, the server drops the request's future, and the route and
    // the pool each count one reader that left.
    let apart = crate::detail::apart_from_slot_owners().await;
    let crowd = crate::detail::crowd(Named::Verify);
    let pool_before = pool_count("verify", "cancelled").await;
    let route_before = crate::detail::route_tally("/verify.json").cancelled;
    live(&root, |addr| async move {
        tokio::task::spawn_blocking(move || {
            use std::io::Write as _;
            let mut socket = std::net::TcpStream::connect(addr).expect("connect");
            socket
                .write_all(
                    format!("GET /verify.json?feed=zerodha HTTP/1.1\r\nHost: {addr}\r\n\r\n")
                        .as_bytes(),
                )
                .expect("write");
            std::thread::sleep(std::time::Duration::from_millis(50));
            // The reader leaves before the pool's wait could end.
        })
        .await
        .expect("the client thread");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while crate::detail::route_tally("/verify.json").cancelled == route_before
            && std::time::Instant::now() < deadline
        {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await;
    drop(crowd);
    drop(apart);
    assert_eq!(
        crate::detail::route_tally("/verify.json").cancelled,
        route_before + 1,
        "the route counts the reader that left"
    );
    assert!(
        pool_count("verify", "cancelled").await > pool_before,
        "the pool counts the reader that left its queue"
    );
    drop(site);
    let _ = std::fs::remove_dir_all(&root);
}

/// One counter of one pool, as `/pools.json` serves it.
async fn pool_count(pool: &str, counter: &str) -> u64 {
    let (_, _, body) = crate::detail::pools_json().await;
    let body: serde_json::Value = serde_json::from_str(&body).expect("pools JSON");
    body["pools"]
        .as_array()
        .expect("pools")
        .iter()
        .find(|row| row["pool"] == pool)
        .and_then(|row| row[counter].as_u64())
        .expect("the pool and counter are served")
}

/// **EVERY ROUTE ROW `/pools.json` SERVES IS A PATH OR THE FRONT END'S ROW
/// IT NAMES** (R3-08 repair 2, verify-R3-08-r2 blocking 1). A browser loads
/// the page before its panel asks, so the fallback has already counted a row
/// under [`crate::detail::FRONT_END`]; the page reader refused any row not
/// starting with `/`, and the panel showed a fault and no table. The body now
/// says which name is the front end's (`front_end`), and this is the contract
/// `web/src/lib/read-pools.js` reads.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_pools_answer_names_the_front_end_row_it_counts() {
    let (root, _, _) = one_month("pools-front-end");
    live(&root, |addr| async move {
        let (page, _) = get(addr, "/ingest").await;
        assert!(!status_of(&page).is_empty(), "{}", head_text(&page));
        let (answer, _) = get(addr, "/pools.json").await;
        assert_eq!(status_of(&answer), "200", "{}", head_text(&answer));
        let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        let json = body
            .find('{')
            .zip(body.rfind('}'))
            .and_then(|(from, to)| body.get(from..=to))
            .unwrap_or("");
        let parsed: serde_json::Value = serde_json::from_str(json).expect("pools JSON");
        assert_eq!(parsed["front_end"], crate::detail::FRONT_END, "{json}");
        // A POOL'S `answered` IS WORK RUN, NOT WHAT THE ROUTE SAID: a
        // being-written 503 is answered in the pool and being_written in its
        // route's row, and the answer says so rather than leaving two
        // `answered` columns to disagree silently (verifier follow-up).
        let say = parsed["say"].as_str().unwrap_or("");
        assert!(
            say.contains("A pool's answered is work it ran to its end")
                && say.contains("the two answered columns count different things"),
            "{json}"
        );
        let routes = parsed["routes"].as_array().expect("routes");
        assert!(
            routes
                .iter()
                .any(|row| row["route"] == crate::detail::FRONT_END),
            "the page's own request is counted under the front end: {json}"
        );
        for row in routes {
            let route = row["route"].as_str().expect("a route name");
            assert!(
                route.starts_with('/') || route == crate::detail::FRONT_END,
                "{route} is neither a path nor the named front end: {json}"
            );
        }
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

// THE LOAD TESTS — `the_feed_list_stays_fast_while_32_clients_loop_each_route`
// and the closed loop at 16, 64 and 256 clients — are in `tests/pool_load.rs`:
// they fill the process-wide pools for seconds, which in this process would
// refuse the store reads of every test running beside them.

/// **A WHOLE MONTH OF EXACTLY THE ROW CAP IS SERVED WHOLE** (R3-08 repair 3,
/// RA-060; verify-R3-08-r3 follow-up "no test serves exactly 60,000 rows").
/// The cap is the most one answer serves, not one past it: an invented GDFL
/// NIFTY one-second month of exactly
/// [`crate::server::bars_route::MAX_MONTH_ANSWER_ROWS`] rows is a 200 with
/// every row, and one row more is the named refusal
/// (`a_whole_month_past_the_row_cap_is_refused_by_name`). A `>=` in the cap's
/// comparison refuses this month, and this test is what says so.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_whole_month_of_exactly_the_row_cap_is_served_whole() {
    /// One NSE session in seconds, 09:15:00 to 15:30:00 IST.
    const SESSION: i64 = 22_500;
    let root = crate::scratch::path("pool-table-row-cap-exact");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("masters")).expect("an owned root");
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: store::path::Timeframe::SECOND_1,
        month: store::path::YearMonth::new(2024, 1).expect("a legal month"),
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id is the cross-check `open` folds; any 32 bits serve"
    )]
    let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let mut file =
        store::file::BarFile::open_or_create(&root, path, symbol_id).expect("a bar file");
    let cap = i64::try_from(crate::server::bars_route::MAX_MONTH_ANSWER_ROWS).expect("small");
    // 2, 3 and 4 January 2024, from 09:15:00 IST, one invented bar a second:
    // `0..cap` is exactly the cap, where the refusal's test writes `0..=cap`.
    let rows: Vec<store::format::Bar> = (0..cap)
        .map(|n| {
            let day = 19_724 + n / SESSION;
            let open = (day * 86_400 - 19_800 + 9 * 3_600 + 15 * 60) * 1_000_000;
            store::format::Bar {
                ts_micros: open + (n % SESSION) * 1_000_000,
                open: 1_000,
                high: 1_010,
                low: 990,
                close: 1_005,
                volume: 0,
                open_interest: i64::MIN,
            }
        })
        .collect();
    file.append(&rows).expect("legal bars");
    drop(file);
    let month = "/bars.json?feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s\
                 &month=2024-01";
    live(&root, |addr| async move {
        let (whole, _) = get(addr, month).await;
        assert_eq!(status_of(&whole), "200", "{}", head_text(&whole));
        assert!(!whole.contains("too_many_rows"), "{}", head_text(&whole));
        assert_eq!(
            whole.matches("\"t\":").count(),
            crate::server::bars_route::MAX_MONTH_ANSWER_ROWS,
            "every row of a month at the cap is served"
        );
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

/// **A WHOLE MONTH PAST THE ROW CAP IS REFUSED BY NAME, BEFORE A RECORD IS
/// READ, AND POINTED AT THE PAGED ROUTE** (R3-08, RA-060). An invented GDFL
/// NIFTY one-second month of 60,001 rows over three sessions: the whole month
/// is one row past [`crate::server::bars_route::MAX_MONTH_ANSWER_ROWS`] and
/// is refused; one day of it (22,500 rows) is served whole; the paged route
/// serves its first page.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_whole_month_past_the_row_cap_is_refused_by_name() {
    /// One NSE session in seconds, 09:15:00 to 15:30:00 IST.
    const SESSION: i64 = 22_500;
    let root = crate::scratch::path("pool-table-row-cap");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("masters")).expect("an owned root");
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: store::path::Timeframe::SECOND_1,
        month: store::path::YearMonth::new(2024, 1).expect("a legal month"),
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id is the cross-check `open` folds; any 32 bits serve"
    )]
    let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let mut file =
        store::file::BarFile::open_or_create(&root, path, symbol_id).expect("a bar file");
    let cap = i64::try_from(crate::server::bars_route::MAX_MONTH_ANSWER_ROWS).expect("small");
    // 2, 3 and 4 January 2024, from 09:15:00 IST, one invented bar a second.
    let rows: Vec<store::format::Bar> = (0..=cap)
        .map(|n| {
            let day = 19_724 + n / SESSION;
            let open = (day * 86_400 - 19_800 + 9 * 3_600 + 15 * 60) * 1_000_000;
            store::format::Bar {
                ts_micros: open + (n % SESSION) * 1_000_000,
                open: 1_000,
                high: 1_010,
                low: 990,
                close: 1_005,
                volume: 0,
                open_interest: i64::MIN,
            }
        })
        .collect();
    file.append(&rows).expect("legal bars");
    drop(file);
    let month = "/bars.json?feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s\
                 &month=2024-01";
    live(&root, |addr| async move {
        let (whole, _) = get(addr, month).await;
        assert_eq!(status_of(&whole), "400", "{}", head_text(&whole));
        assert!(whole.contains("too_many_rows"), "{whole}");
        assert!(whole.contains("60001 rows"), "{whole}");
        assert!(
            whole.contains("/bars/window.json"),
            "it names the paged route: {whole}"
        );
        let (day, _) = get(addr, &format!("{month}&from=2024-01-03&to=2024-01-03")).await;
        assert_eq!(status_of(&day), "200", "{}", head_text(&day));
        assert_eq!(day.matches("\"t\":").count(), 22_500, "one whole session");
        let (page, _) = get(
            addr,
            "/bars/window.json?feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s\
             &from=2024-01&to=2024-01&limit=1000&sort=ts",
        )
        .await;
        assert_eq!(status_of(&page), "200", "{}", head_text(&page));
    })
    .await;
    let _ = std::fs::remove_dir_all(&root);
}

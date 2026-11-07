//! The app's own speed table (R3-14): the buckets, the outcomes, the minute
//! freeze, the hour, the cap, the cancelled request, and the minute written at
//! shutdown.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes"
)]

use super::*;

/// Every time lands in a bucket whose top is at or above it and whose
/// predecessor's top is below it, the buckets climb, and a bucket is at most
/// an eighth of its time wide.
#[test]
fn every_time_lands_in_one_bucket_at_most_an_eighth_wide() {
    let mut previous_top = None;
    for bucket in 0..BUCKETS - 1 {
        let top = bucket_top(bucket);
        if let Some(previous) = previous_top {
            assert!(top > previous, "bucket {bucket} climbs");
            let low = previous + 1;
            assert_eq!(bucket_of(low), bucket, "{low} opens bucket {bucket}");
            assert!(
                (top - low + 1) * 8 <= low.max(8),
                "bucket {bucket} [{low}, {top}] is at most an eighth wide"
            );
        }
        assert_eq!(bucket_of(top), bucket, "{top} closes bucket {bucket}");
        previous_top = Some(top);
    }
    for t in [
        0,
        1,
        7,
        8,
        9,
        15,
        16,
        17,
        100,
        999,
        1_000,
        65_535,
        1 << 20,
        (1 << 36) - 1,
    ] {
        let b = bucket_of(t);
        assert!(bucket_top(b) >= t, "{t} is under its bucket's top");
    }
    for bucket in 1..BUCKETS {
        assert_eq!(
            bucket_low(bucket),
            bucket_top(bucket - 1) + 1,
            "bucket {bucket} opens where the last closed"
        );
        assert_eq!(
            bucket_of(bucket_low(bucket)),
            bucket,
            "the low of {bucket} lands in it"
        );
    }
    assert_eq!(bucket_low(0), 0);
    assert_eq!(
        bucket_of(1 << 36),
        BUCKETS - 1,
        "past 2^36 µs shares the top"
    );
    assert_eq!(bucket_of(u64::MAX), BUCKETS - 1);
    assert_eq!(bucket_top(BUCKETS - 1), u64::MAX);
    assert_eq!(BUCKETS, 272);
}

/// A percentile is the top of its bucket, clamped to the max; empty is zero;
/// the rank is a ceiling, so one slow request in a hundred IS the p99's
/// neighbour and not the p99.
#[test]
fn a_percentile_is_its_buckets_top_never_above_the_max() {
    let mut buckets = vec![0_u64; BUCKETS];
    buckets[bucket_of(100)] += 99;
    buckets[bucket_of(50_000)] += 1;
    let p50 = percentile(&buckets, 100, 50, 50_000);
    let p99 = percentile(&buckets, 100, 99, 50_000);
    assert!((100..=112).contains(&p50), "{p50}");
    assert_eq!(p50, p99, "99 of 100 at 100 µs: the 99th is one of them");
    assert_eq!(
        percentile(&buckets, 100, 100, 50_000),
        50_000,
        "clamped to the max"
    );
    assert_eq!(percentile(&buckets, 0, 99, 0), 0, "empty");
    let mut two = vec![0_u64; BUCKETS];
    two[bucket_of(10)] += 1;
    two[bucket_of(5_000)] += 1;
    assert_eq!(
        percentile(&two, 2, 99, 5_000),
        5_000,
        "the 99th of two is the second"
    );
    assert_eq!(percentile(&two, 2, 50, 5_000), 10);
}

/// A status maps to one outcome: below 400 answered, 4xx refused but a 429
/// busy, a 503 busy only with `Retry-After`, other 5xx failed.
#[test]
fn a_status_maps_to_one_outcome() {
    assert_eq!(Outcome::of(200, false), Outcome::Answered);
    assert_eq!(Outcome::of(304, false), Outcome::Answered);
    assert_eq!(Outcome::of(404, false), Outcome::Refused);
    assert_eq!(Outcome::of(429, false), Outcome::Busy);
    assert_eq!(Outcome::of(503, true), Outcome::Busy);
    assert_eq!(Outcome::of(503, false), Outcome::Failed);
    assert_eq!(Outcome::of(500, true), Outcome::Failed);
    assert_eq!(Outcome::of(501, false), Outcome::Failed);
}

/// The minute freeze writes one line per row asked in the minute, with its
/// counts by outcome and its p50, p99 and max; a quiet row writes nothing;
/// the next minute starts from zero; the last hour adds the minutes.
#[test]
fn each_minute_writes_one_line_per_route_asked_and_the_hour_adds_them() {
    let speed = Speed::new(0, 16);
    for i in 0..98 {
        speed.record(
            "/bars.json",
            Kind::Route,
            100 + i % 3,
            Outcome::Answered,
            10,
        );
    }
    speed.record("/bars.json", Kind::Route, 40_000, Outcome::Busy, 10);
    speed.record("/bars.json", Kind::Route, 7, Outcome::Refused, 10);
    speed.record("/spot.json", Kind::Route, 900, Outcome::Failed, 10);
    let _quiet = speed.row("/chain.json", Kind::Route, 10);
    let lines = speed.roll(MINUTE_MS, false);
    assert_eq!(
        lines.len(),
        2,
        "two rows were asked; the quiet one writes nothing: {lines:?}"
    );
    let bars = lines.iter().find(|l| l.route == "/bars.json").unwrap();
    assert_eq!(
        (
            bars.figures.n,
            bars.figures.answered,
            bars.figures.busy,
            bars.figures.refused
        ),
        (100, 98, 1, 1)
    );
    assert_eq!(bars.figures.max_micros, 40_000);
    assert!(
        (100..=112).contains(&bars.figures.p50_micros),
        "{:?}",
        bars.figures
    );
    assert!(!bars.partial);
    assert_eq!(bars.window_ms, MINUTE_MS - 10);
    let spot = lines.iter().find(|l| l.route == "/spot.json").unwrap();
    assert_eq!((spot.figures.n, spot.figures.failed), (1, 1));
    assert!(
        speed.roll(2 * MINUTE_MS, false).is_empty(),
        "a quiet minute writes nothing"
    );
    speed.record(
        "/bars.json",
        Kind::Route,
        3,
        Outcome::Answered,
        2 * MINUTE_MS + 1,
    );
    let third = speed.roll(3 * MINUTE_MS, false);
    assert_eq!(third.len(), 1);
    assert_eq!(
        (third[0].figures.n, third[0].figures.max_micros),
        (1, 3),
        "a minute starts from zero"
    );
    let report = speed.report(3 * MINUTE_MS + 5);
    let row = report
        .rows
        .iter()
        .find(|r| r.route == "/bars.json")
        .unwrap();
    assert_eq!(row.since_start.n, 101);
    assert_eq!(row.last_minute.n, 1);
    assert_eq!(row.last_minute.max_micros, 3);
    assert_eq!(row.last_hour.n, 101);
    assert_eq!(row.last_hour.max_micros, 40_000);
    assert_eq!(row.last_hour_minutes, 3);
    assert_eq!(
        row.first_after_start_micros,
        Some(100),
        "the cold request is kept apart"
    );
    let chain = report
        .rows
        .iter()
        .find(|r| r.route == "/chain.json")
        .unwrap();
    assert_eq!(
        (chain.since_start.n, chain.first_after_start_micros),
        (0, None)
    );
}

/// The last hour is the last sixty minutes and no more: a request older than
/// that leaves the hour and stays in since-start.
#[test]
fn the_hour_forgets_the_sixty_first_minute() {
    let speed = Speed::new(0, 4);
    speed.record("/gaps.json", Kind::Route, 50, Outcome::Answered, 1);
    for minute in 1..=HOUR_ROLLS as u64 {
        let _lines = speed.roll(minute * MINUTE_MS, false);
    }
    let held = speed.report(HOUR_ROLLS as u64 * MINUTE_MS);
    assert_eq!(
        held.rows[0].last_hour.n, 1,
        "sixty minutes back still holds it"
    );
    let _lines = speed.roll((HOUR_ROLLS as u64 + 1) * MINUTE_MS, false);
    let gone = speed.report((HOUR_ROLLS as u64 + 1) * MINUTE_MS);
    assert_eq!(
        gone.rows[0].last_hour.n, 0,
        "the sixty-first minute is out of the hour"
    );
    assert_eq!(gone.rows[0].last_hour_minutes, HOUR_ROLLS as u64);
    assert_eq!(gone.rows[0].since_start.n, 1);
}

/// Past the cap a new row is counted under the over-cap row, which the report
/// names with its count: never dropped, never a new row.
#[test]
fn a_row_past_the_cap_is_counted_and_named() {
    let speed = Speed::new(0, 2);
    speed.record("/a", Kind::Route, 1, Outcome::Answered, 0);
    speed.record("/b", Kind::Route, 1, Outcome::Answered, 0);
    speed.record("/c", Kind::Route, 1, Outcome::Answered, 0);
    speed.record("/d", Kind::Route, 1, Outcome::Answered, 0);
    let report = speed.report(1);
    assert_eq!(report.over_cap, 2);
    let names: Vec<&str> = report.rows.iter().map(|r| r.route.as_str()).collect();
    assert_eq!(names, ["/a", "/b", OVER_CAP]);
    assert_eq!(speed.roll(MINUTE_MS, false).len(), 3);
}

/// A request dropped before its answer counts as cancelled, once; a finished
/// one does not.
#[test]
fn a_request_dropped_unanswered_counts_as_cancelled() {
    let speed = Speed::new(0, 4);
    let row = speed.row("/chain.json", Kind::Route, 0);
    drop(Pending::start(row.clone()));
    Pending::start(row).finish(12, Outcome::Answered);
    let report = speed.report(1);
    let figures = report.rows[0].since_start;
    assert_eq!((figures.n, figures.cancelled, figures.answered), (2, 1, 1));
}

/// Stages sit beside routes in the same buckets, after them in the report.
#[test]
fn a_stage_has_the_same_buckets_as_a_route() {
    let speed = Speed::new(0, 4);
    speed.record("stage:land", Kind::Stage, 2_000_000, Outcome::Answered, 0);
    speed.record("/z", Kind::Route, 5, Outcome::Answered, 0);
    let report = speed.report(1);
    assert_eq!(report.rows[0].route, "/z");
    assert_eq!(report.rows[1].kind, Kind::Stage);
    assert_eq!(report.rows[1].since_start.max_micros, 2_000_000);
}

/// **THE LAST MINUTE OF COUNTS SURVIVES A SHUTDOWN** (R3-14): the freeze at
/// shutdown writes the open minute marked partial, and through the process
/// table the line reaches the log, where the audit stream keeps it. Before
/// this package a stop lost every count since the last window closed.
#[test]
fn the_last_minute_of_counts_survives_a_shutdown() {
    let speed = Speed::new(0, 4);
    speed.record("/dayflags.json", Kind::Route, 321, Outcome::Answered, 5);
    speed.record("/dayflags.json", Kind::Route, 9, Outcome::Refused, 6);
    let at_stop = speed.roll(30_000, true);
    assert_eq!(at_stop.len(), 1);
    assert!(at_stop[0].partial);
    assert_eq!(
        (at_stop[0].figures.n, at_stop[0].figures.max_micros),
        (2, 321)
    );
    assert_eq!(at_stop[0].window_ms, 30_000 - 5);

    // THROUGH THE PROCESS TABLE AND THE LOG. A route name no other test uses;
    // the minute thread may freeze it first, so every line for it is summed.
    let mark = crate::emitted::mark();
    let route = "/speed-test/the-last-minute-survives";
    for _ in 0..3 {
        global().record(route, Kind::Route, 77, Outcome::Answered, now_ms());
    }
    let written = flush_at_shutdown();
    assert!(written >= 1, "the shutdown freeze wrote the open minute");
    assert!(telemetry::is_audited(
        telemetry::Level::Info,
        "api.speed",
        "minute ended"
    ));
    let lines: Vec<_> = crate::emitted::landed(mark, "api.speed", "minute ended")
        .into_iter()
        .filter(|record| crate::emitted::says(record, "route", route))
        .collect();
    let n: u64 = lines
        .iter()
        .map(|record| {
            (0..=3)
                .find(|n| crate::emitted::counts(record, "n", *n))
                .unwrap_or(0)
        })
        .sum();
    assert_eq!(n, 3, "every count reached the log: {lines:?}");
}

/// The report `/speed.json` serves: one row per name asked, a `route=`
/// filter that answers that row alone, and the resolution sentence.
#[test]
fn the_route_answers_the_report_and_filters_by_route() {
    let speed = Speed::new(0, 8);
    speed.record("/bars.json", Kind::Route, 10, Outcome::Answered, 0);
    speed.record("/spot.json", Kind::Route, 20, Outcome::Answered, 0);
    let json = |query: &str| {
        let (status, body) = crate::speed_route::answer(&speed, query, 5);
        (
            status,
            serde_json::from_str::<serde_json::Value>(&body).unwrap(),
        )
    };
    let (status, all) = json("");
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(all["rows"].as_array().unwrap().len(), 2);
    assert_eq!(all["resolution"], RESOLUTION);
    assert_eq!(all["now_ms"], 5);
    let (status, one) = json("route=%2Fspot.json");
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(one["rows"].as_array().unwrap().len(), 1);
    assert_eq!(one["rows"][0]["route"], "/spot.json");
    assert_eq!(one["rows"][0]["since_start"]["max_micros"], 20);
    assert_eq!(one["rows"][0]["kind"], "route");
}

/// **A NAME WITH NO ROW IS A 404 THAT SAYS SO** (R3-14 repair 4). It was a
/// `200` with an empty row list, which reads exactly like a route asked and
/// measured as nothing. The body names the row asked for, the three ways a
/// name has no row, the over-cap count, and every name the table keeps.
#[test]
fn a_route_with_no_row_is_refused_by_name_with_the_rows_kept() {
    let speed = Speed::new(0, 8);
    speed.record("/bars.json", Kind::Route, 10, Outcome::Answered, 0);
    speed.record(
        "stage:zerodha landing",
        Kind::Stage,
        30,
        Outcome::Answered,
        0,
    );
    for (query, asked) in [
        ("route=%2Fnever", "/never"),
        ("route=%2Fbars.json%20", "/bars.json "),
        ("route=bars.json", "bars.json"),
        ("route=%2FBARS.JSON", "/BARS.JSON"),
        ("route=stage%3Azerodha", "stage:zerodha"),
    ] {
        let (status, body) = crate::speed_route::answer(&speed, query, 5);
        assert_eq!(status, axum::http::StatusCode::NOT_FOUND, "{query}: {body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["error"], format!("no speed row is named '{asked}'"));
        let why = json["why"].as_str().unwrap();
        for part in ["has not been since started_ms", OVER_CAP, "is not a route"] {
            assert!(why.contains(part), "{query}: the why names {part}: {why}");
        }
        assert_eq!(json["over_cap"], 0);
        assert_eq!(json["started_ms"], 0);
        assert_eq!(json["now_ms"], 5);
        assert_eq!(
            json["rows_kept"],
            serde_json::json!(["/bars.json", "stage:zerodha landing"]),
            "{query}"
        );
        assert!(json.get("rows").is_none(), "no empty report rides along");
    }
    let (status, body) = crate::speed_route::answer(&speed, "route=stage%3Azerodha%20landing", 5);
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let (status, _) = crate::speed_route::answer(&speed, "route=", 5);
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "an empty route= is no filter"
    );
}

/// A FULL TABLE: a name past the cap has no row of its own, and the 404 says
/// how many requests were counted under the cap row, so the caller learns
/// that the name may be among them.
#[test]
fn a_route_past_the_cap_is_refused_with_the_over_cap_count() {
    let speed = Speed::new(0, 1);
    speed.record("/bars.json", Kind::Route, 10, Outcome::Answered, 0);
    speed.record("/spot.json", Kind::Route, 10, Outcome::Answered, 0);
    speed.record("/spot.json", Kind::Route, 10, Outcome::Answered, 0);
    let (status, body) = crate::speed_route::answer(&speed, "route=%2Fspot.json", 5);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let over = json["over_cap"].as_u64().unwrap();
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND, "{body}");
    assert_eq!(
        over, 2,
        "both /spot.json requests were counted over the cap: {body}"
    );
    assert!(
        json["why"]
            .as_str()
            .unwrap()
            .contains(&format!("({over} requests there)")),
        "{body}"
    );
}

/// **EVERY ROUTE IS TIMED THROUGH THE SERVER'S OWN LAYERS**, under its
/// matched path and not its query, the front end as one row, and
/// `/speed.json` is registered in the route table and answers its own
/// figures.
#[tokio::test]
async fn a_served_request_is_counted_under_its_matched_path() {
    let masters = crate::scratch::path("speed-route-masters");
    let store = crate::scratch::path("speed-route-store");
    let web = crate::scratch::path("speed-route-web");
    for dir in [&masters, &store, &web] {
        let _ = std::fs::create_dir_all(dir);
    }
    let site = std::sync::Arc::new(crate::server::Site::load(&masters, &store));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = crate::server::router_serving(
        site,
        std::sync::Arc::new(crate::assets::Assets::new(&web)),
        addr,
    );
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let served = tokio::spawn(
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _closed = stopped.await;
            })
            .into_future(),
    );
    let get = |path: &'static str| {
        tokio::task::spawn_blocking(move || {
            use std::io::{Read as _, Write as _};
            let mut s = std::net::TcpStream::connect(addr).unwrap();
            write!(
                s,
                "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
                addr.port()
            )
            .unwrap();
            let mut buf = String::new();
            s.read_to_string(&mut buf).unwrap();
            buf
        })
    };
    let before = global()
        .report(now_ms())
        .rows
        .into_iter()
        .find(|r| r.route == "/speed.json")
        .map_or(0, |r| r.since_start.n);
    let first = get("/speed.json?route=%2Fspeed.json").await.unwrap();
    assert!(first.starts_with("HTTP/1.1 200 "), "{first}");
    let second = get("/speed.json?route=%2Fspeed.json&x=1").await.unwrap();
    let body = second.split("\r\n\r\n").nth(1).unwrap();
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    let n = json["rows"][0]["since_start"]["n"].as_u64().unwrap();
    assert!(
        n > before,
        "the first request was counted under /speed.json, query or not: {body}"
    );
    let unknown = get("/speed.json?route=%2Fno-such-route-anywhere")
        .await
        .unwrap();
    assert!(
        unknown.starts_with("HTTP/1.1 404 ")
            && unknown.contains("no speed row is named '/no-such-route-anywhere'"),
        "a name with no row is a 404 that says so, through the served router: {unknown}"
    );
    let _front = get("/no-such-page-at-all").await.unwrap();
    let page = get("/page-report").await.unwrap();
    assert!(
        page.starts_with("HTTP/1.1 405 "),
        "the report route is registered, POST only: {page}"
    );
    page_report_through_the_served_router(addr).await;
    let rows = global().report(now_ms()).rows;
    assert!(
        rows.iter().any(|r| r.route == FRONT_END),
        "the fallback is one row"
    );
    assert!(
        !rows.iter().any(|r| r.route.contains("no-such-page")),
        "an unmatched path is never a row of its own"
    );
    let _gone = stop.send(());
    served.await.unwrap().unwrap();
}

/// `POST /page-report` THROUGH THE SERVED ROUTER at `addr` (R3-14 repair 4:
/// it had a handler test only). A same-origin report reaches the handler,
/// which names the bad field; a report past 1 KiB is the handler's own JSON
/// `413`; a cross-site report is the write gate's `403`, before the handler.
/// Each body is refused before the ration is spent, so this cannot starve
/// `a_page_report_is_rationed_and_audited` of its window, and none depends on
/// whether a sink is installed.
async fn page_report_through_the_served_router(addr: std::net::SocketAddr) {
    let post = |sec_fetch: &'static str, body: String| {
        tokio::task::spawn_blocking(move || {
            use std::io::{Read as _, Write as _};
            let mut s = std::net::TcpStream::connect(addr).unwrap();
            write!(
                s,
                "POST /page-report HTTP/1.1\r\nHost: {addr}\r\nOrigin: http://{addr}\r\n\
                 Sec-Fetch-Site: {sec_fetch}\r\n\
                 Content-Type: application/x-www-form-urlencoded\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let mut buf = String::new();
            s.read_to_string(&mut buf).unwrap();
            buf
        })
    };
    let empty_what = post("same-origin", "page=%2Fdb&what=".to_owned())
        .await
        .unwrap();
    assert!(
        empty_what.starts_with("HTTP/1.1 400 ")
            && empty_what.contains("\"written\":false")
            && empty_what.contains("what="),
        "a same-origin report reaches the handler, which names the bad field: {empty_what}"
    );
    let long = format!(
        "page=%2Fdb&what={}",
        "x".repeat(crate::page_report_route::MAX_REPORT_BYTES)
    );
    let too_long = post("same-origin", long).await.unwrap();
    assert!(
        too_long.starts_with("HTTP/1.1 413 ") && too_long.contains("\"written\":false"),
        "a report past 1 KiB is the handler's own JSON 413: {too_long}"
    );
    let cross = post("cross-site", "page=%2Fdb&what=boom".to_owned())
        .await
        .unwrap();
    assert!(
        cross.starts_with("HTTP/1.1 403 ") && !cross.contains("\"written\""),
        "a cross-site report is refused before the handler: {cross}"
    );
}

/// **A WINDOW'S MAX IS NEVER BELOW ITS SLOWEST REQUEST** (R3-14). A request
/// finishing while its minute is frozen can count its bucket in one minute
/// and its maximum in the next. Before this rule the minute read its p99 and
/// its max as the stale 120 µs although two of its requests took 5 ms: a p99
/// below the bucket its rank is in, and a max below a request it counted.
#[test]
fn a_windows_max_is_never_below_its_slowest_request() {
    let mut buckets = vec![0_u64; BUCKETS];
    buckets[bucket_of(100)] += 98;
    buckets[bucket_of(5_000)] += 2;
    let slow = bucket_of(5_000);
    let figures = Figures::of(&buckets, [100, 0, 0, 0, 0], 120);
    assert!(figures.max_micros >= bucket_low(slow), "{figures:?}");
    assert!(
        (bucket_low(slow)..=bucket_top(slow)).contains(&figures.p99_micros),
        "the p99 stays inside the bucket its rank falls in: {figures:?}"
    );
    assert!(figures.p50_micros <= figures.p99_micros && figures.p99_micros <= figures.max_micros);
    // A max that is right is kept as it is.
    let exact = Figures::of(&buckets, [100, 0, 0, 0, 0], 5_003);
    assert_eq!(exact.max_micros, 5_003);
    assert_eq!(
        Figures::of(&vec![0; BUCKETS], [0; OUTCOMES], 0),
        Figures::default()
    );
}

/// **TWO FREEZES AT ONCE NEVER COUNT BACKWARDS** (R3-14). The minute thread
/// and the shutdown flush can roll together. Without `Speed::rolling` two
/// rolls could interleave on a row: the first freezes its counts and waits
/// for the row's copies, the second freezes LATER counts and pushes them
/// first, and the first then takes its difference against the later copy,
/// backwards, which wraps to about four billion requests on a row that had
/// six. With the lock the second roll cannot start until the first is done.
///
/// Driven here as far as a test can: the test holds the row's copies while
/// the first roll waits on them and the second is started after five more
/// requests, twenty rounds, and every request must be counted once. A GUARD,
/// said plainly: on this Mac a waiter that queued first took the lock first
/// in twenty rounds of twenty with the one-roll lock removed, so the bad
/// order needs a preemption between a freeze and its lock that no test can
/// force, and this test's failure without the lock was NOT demonstrated
/// (R3-14's report). The lock is correct by construction: a roll holds it
/// for its whole walk.
#[test]
fn two_freezes_at_once_never_count_backwards() {
    let speed = Speed::new(0, 4);
    let row = speed.row("/race", Kind::Route, 0);
    let table = &speed;
    let mut clock = 0_u64;
    for _round in 0..20 {
        speed.record("/race", Kind::Route, 3, Outcome::Answered, 0);
        let held = row.0.frozen.lock().unwrap();
        let lines = std::thread::scope(|scope| {
            clock += 2;
            let (first_at, second_at) = (clock - 1, clock);
            let first = scope.spawn(move || table.roll(first_at, false));
            std::thread::sleep(std::time::Duration::from_millis(30));
            for _ in 0..5 {
                speed.record("/race", Kind::Route, 3, Outcome::Answered, 0);
            }
            let second = scope.spawn(move || table.roll(second_at, false));
            std::thread::sleep(std::time::Duration::from_millis(30));
            drop(held);
            let mut lines = first.join().unwrap();
            lines.extend(second.join().unwrap());
            lines
        });
        let counted: u64 = lines.iter().map(|line| line.figures.n).sum();
        assert_eq!(
            counted, 6,
            "each request counted once, never backwards: {lines:?}"
        );
    }
    let report = speed.report(clock);
    assert_eq!(report.rows[0].since_start.n, 120);
    assert_eq!(report.rows[0].last_hour.n, 120);
}

/// **A `kill -TERM` STOPS THE SERVER GRACEFULLY, SO THE OPEN MINUTE IS
/// WRITTEN** (R3-14). The binary's stop was Ctrl-C alone, and every stop
/// script and redeploy sends `SIGTERM`, which ended the process before
/// `serve_limited` could reach `flush_at_shutdown`: each redeploy lost up to
/// a minute of every route's counts. [`stop_asked`] races Ctrl-C against a
/// `SIGTERM` stream through [`first_stop`]; the binary passes it, and the
/// server flushes after its graceful stop.
///
/// A test may not start `kill` (gate 0, D-1603) and this crate forbids the
/// unsafe call that raises a signal, so the race is driven here with futures
/// standing for the two signals, and the OS half is shown end to end on a
/// shadow server stopped by `kill -TERM` (R3-14's report: the partial minute
/// lines in its log).
#[tokio::test]
async fn a_sigterm_stops_the_server_gracefully_and_the_open_minute_is_written() {
    let terminated = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        first_stop(std::future::pending(), async {}),
    )
    .await
    .expect("a terminate request stops at once");
    assert!(
        terminated.is_ok(),
        "a terminate request is a stop, not an error"
    );
    let ctrl_c_failed = first_stop(
        async { Err(std::io::Error::other("no ctrl-c here")) },
        std::future::pending(),
    )
    .await;
    assert_eq!(
        ctrl_c_failed.expect_err("Ctrl-C's own error").to_string(),
        "no ctrl-c here"
    );

    // AND STOP_ASKED RACES THE REAL SIGNALS through it.
    let speed = include_str!("speed.rs");
    // BOTH ARE REGISTERED BY THE CALL, before the future it returns (R3-14
    // repair 4): registered inside `async`, a TERM before the first poll
    // ended the process with no line.
    let body = speed
        .split_once(concat!("\npub fn stop_", "asked("))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    let interrupt = body.find(concat!("signal(SignalKind::", "interrupt())"));
    let registered = body.find(concat!("signal(SignalKind::", "terminate())"));
    let deferred = body.find(concat!("async ", "move {"));
    let raced = body.find(concat!("first_", "stop(ctrl_c,"));
    assert!(
        matches!(
            (interrupt, registered, deferred, raced),
            (Some(i), Some(t), Some(d), Some(r)) if i < d && t < d && d < r
        ),
        "stop_asked registers SIGINT and SIGTERM before its future, and races them: {body}"
    );

    // AND THE PATH IS WIRED: the binary makes the call before its runtime
    // runs anything and passes the future, and the server writes the open
    // minute after its graceful stop. The needles are split so this file
    // cannot match them.
    let main = include_str!("main.rs");
    // ONLY FOR A SERVE (R3-14 repair 5): the call is behind
    // `catches_stops`, and the future it returns is the one `run_held` gets.
    let gated = main.find(concat!("if api::server::", "catches_stops(words)"));
    let asked = main.find(concat!("Box::pin(api::speed::", "stop_asked())"));
    let entered = main.find(concat!("runtime.", "enter()"));
    let blocked = main.find(concat!("runtime.", "block_on("));
    assert!(
        matches!(
            (gated, entered, asked, blocked),
            (Some(g), Some(e), Some(a), Some(b)) if g < e && e < a && a < b
        ) && main.contains(concat!("run_held(&args, ", "stop, &mut held)")),
        "the binary registers its stop inside the runtime before it serves, and stops on SIGTERM too"
    );
    // AND ENDS THROUGH `end_served`, which counts what the drain cut and
    // only then frees the store and the port (R3-14 repair 1).
    let ran = main.find(concat!("api::server::run_", "held(&args"));
    let ended = main.find(concat!("api::server::end_", "served(runtime"));
    assert!(
        matches!((ran, ended), (Some(r), Some(e)) if r < e),
        "the binary keeps what the serve held and ends through end_served"
    );
    // AND ITS EXIT LINE IS SAID INSIDE `end_served`, before the log is closed
    // and the port freed (R3-14 repair 3): said after, it would be dropped.
    let call = ended
        .and_then(|at| main.get(at..))
        .and_then(|rest| rest.split_once("});"))
        .map_or("", |(call, _)| call);
    assert!(
        call.contains(concat!("note_", "exit(code, count, outlived)")),
        "the exit line is end_served's last word: {call}"
    );
    let server = include_str!("server.rs");
    let body = server
        .split_once(concat!("\npub async fn serve_", "limited("))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    let graceful = body.find(".with_graceful_shutdown(");
    let flushed = body.find(concat!("crate::speed::flush_", "at_shutdown()"));
    assert!(
        matches!((graceful, flushed), (Some(g), Some(f)) if g < f),
        "serve_limited flushes the open minute after the graceful stop: {body}"
    );
}

/// **A STOP WAITS FOR OPEN REQUESTS NO LONGER THAN ITS GRACE, AND SAYS SO**
/// (R3-14). With `SIGTERM` a graceful stop, a read still being answered held
/// the stop for as long as that read took; a redeploy's `kill` then waited on
/// it. Once the stop is asked, `drain_bound` resolves after the grace with the
/// cut written at `warn`, and `serve_limited` races it against the server's
/// graceful stop and flushes the open minute after both.
#[tokio::test]
async fn a_stop_waits_for_open_requests_no_longer_than_its_grace() {
    let mark = crate::emitted::mark();
    // ONE REQUEST OPEN AT THE BOUND, on a row no other test uses: the cut
    // line must say how many and where (R3-14 repair 1).
    let route = "/speed-test/open-at-the-bound";
    let open = Pending::start(global().row(route, Kind::Route, now_ms()));
    let (stopping, asked) = tokio::sync::oneshot::channel();
    stopping.send(()).expect("the receiver is held");
    let started = std::time::Instant::now();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        drain_bound(asked, std::time::Duration::from_millis(50)),
    )
    .await
    .expect("the bound resolved, not the test's own timeout");
    assert!(started.elapsed() >= std::time::Duration::from_millis(50));
    let cut = crate::emitted::landed(
        mark,
        "api.serve",
        "requests still open at the stop were cut short",
    );
    assert!(
        cut.iter()
            .any(|record| crate::emitted::counts(record, "grace_ms", 50)),
        "the cut is said with its grace: {cut:?}"
    );
    let ours = cut
        .iter()
        .find(|record| crate::emitted::says(record, "routes", &format!("{route}=1")))
        .unwrap_or_else(|| panic!("the cut names the open request's route: {cut:?}"));
    assert!(
        ours.field("cut")
            .and_then(telemetry::OwnedValue::as_u64)
            .is_some_and(|n| n >= 1),
        "and how many were cut: {ours:?}"
    );
    drop(open);
    assert_eq!(
        crate::server::ConnectionLimits::SERVED.drain_grace,
        DRAIN_GRACE,
        "the operator's server drains for DRAIN_GRACE"
    );
    assert_eq!(
        STOP_BOUND,
        DRAIN_GRACE + crate::server::SHUTDOWN_GRACE,
        "the whole stop is the drain plus the runtime's end"
    );

    // AND THE SERVER RACES IT: the needles are split so this file cannot
    // match them.
    let server = include_str!("server.rs");
    let body = server
        .split_once(concat!("\npub async fn serve_", "limited("))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    let armed = body.find(concat!("stopping.", "send(())"));
    let raced = body.find(concat!(
        "crate::speed::drain_",
        "bound(stop_asked, limits.drain_grace)"
    ));
    let flushed = body.find(concat!("crate::speed::flush_", "at_shutdown()"));
    assert!(
        matches!((armed, raced, flushed), (Some(a), Some(r), Some(f)) if a < r && r < f),
        "serve_limited arms the bound at the signal, races it, then flushes: {body}"
    );
}

/// A stop never asked (its sender gone unsent, as when the server ended on
/// its own) arms no bound: `drain_bound` never resolves, so it cannot end a
/// server nobody asked to stop.
#[tokio::test]
async fn an_unasked_stop_arms_no_bound() {
    let (stopping, asked) = tokio::sync::oneshot::channel::<()>();
    drop(stopping);
    let waited = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        drain_bound(asked, std::time::Duration::ZERO),
    )
    .await;
    assert!(waited.is_err(), "an unasked stop resolved the bound");
}

/// **IMPORT AND CHECKING STAGES ARE TIMED IN THE SAME BUCKETS** (R3-14,
/// RA-014). The vendor scrub, driven for real (an unreadable census, refused
/// inside the pool), lands one more count on `stage:vendor scrub`; the
/// Zerodha landing of every pulled member is wrapped the same way, its
/// outcome failed when the member named a failure; and a stage dropped
/// unfinished counts cancelled.
#[tokio::test]
async fn a_landing_and_a_scrub_are_timed_as_stages() {
    let before = global()
        .report(now_ms())
        .rows
        .iter()
        .find(|row| row.route == "stage:vendor scrub")
        .map_or(0, |row| row.since_start.n);
    let unread = crate::census::VendorCensus {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        path: std::path::PathBuf::from("/nowhere/manifest"),
        state: crate::census::Census::Unreadable {
            reason: "a fixture counter".to_owned(),
            fault: crate::census::Fault::Refused,
        },
    };
    let report = crate::verify::vendor_off_worker(std::path::PathBuf::from("/nowhere"), unread)
        .await
        .expect("admitted");
    assert!(report.refused.is_some());
    let row = global()
        .report(now_ms())
        .rows
        .into_iter()
        .find(|row| row.route == "stage:vendor scrub")
        .expect("the scrub has its stage row");
    assert_eq!(row.kind, Kind::Stage);
    assert!(row.since_start.n > before, "{row:?}");

    let dropped = stage_start("speed-test/a stage that never finished");
    drop(dropped);
    let row = global()
        .report(now_ms())
        .rows
        .into_iter()
        .find(|row| row.route == "stage:speed-test/a stage that never finished")
        .expect("the dropped stage has its row");
    assert_eq!((row.since_start.n, row.since_start.cancelled), (1, 1));

    // AND THE LANDING IS WRAPPED: the needles are split so this file cannot
    // match them.
    let server = include_str!("server.rs");
    let body = server
        .split_once(concat!("\nasync fn land_broker_", "member("))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    let started = body.find(concat!("speed::stage_", "start(\"zerodha landing\")"));
    let landed = body.find("land_spot(");
    let finished = body.find(concat!("stage.finish_", "now("));
    assert!(
        matches!((started, landed, finished), (Some(s), Some(l), Some(f)) if s < l && l < f),
        "each member's landing is timed around land_spot: {body}"
    );
}

/// **A STAGE TIMES ITS WORK, NOT THE REQUEST THAT STARTED IT** (R3-14 repair
/// 2). The scrub's stage was a `Pending` held by the request future, so a
/// client that left at one second counted the stage cancelled at one second
/// while the scrub ran on for three minutes on the blocking pool (second
/// agent, 15 Oct). A [`SharedStage`] is counted by whichever side finishes
/// first, once: here the request side is dropped while the work runs, and the
/// stage is counted only when the work ends, answered, with the work's time.
#[test]
fn a_stage_times_its_work_not_the_request_that_started_it() {
    let name = "speed-test/work that outlives its request";
    let row_of = || {
        global()
            .report(now_ms())
            .rows
            .into_iter()
            .find(|row| row.route == format!("stage:{name}"))
    };
    let stage = SharedStage::start(name);
    let in_work = stage.clone();
    let (go, wait) = std::sync::mpsc::channel::<()>();
    let work = std::thread::spawn(move || {
        wait.recv().expect("the test says when the work ends");
        std::thread::sleep(std::time::Duration::from_millis(30));
        in_work.finish(Outcome::Answered)
    });
    // THE CLIENT LEAVES: the request's side is dropped while the work runs.
    drop(stage);
    assert!(
        row_of().is_none_or(|row| row.since_start.n == 0),
        "nothing is counted while the work runs: {:?}",
        row_of()
    );
    go.send(()).expect("the work waits");
    assert!(work.join().expect("the work ends"), "the work counted it");
    let row = row_of().expect("the stage has its row");
    assert_eq!(
        (
            row.since_start.n,
            row.since_start.answered,
            row.since_start.cancelled
        ),
        (1, 1, 0),
        "counted once, answered, when the work ended: {row:?}"
    );
    assert!(
        row.since_start.max_micros >= 30_000,
        "with the work's time: {row:?}"
    );

    // EITHER SIDE COUNTS IT ONCE: the second finish finds it counted.
    let twice = SharedStage::start(name);
    assert!(twice.clone().finish(Outcome::Failed));
    assert!(!twice.finish(Outcome::Answered), "counted once only");
    let row = row_of().expect("the row");
    assert_eq!(
        (row.since_start.n, row.since_start.failed),
        (2, 1),
        "{row:?}"
    );

    // AND THE SCRUB USES IT, finishing inside the blocking closure: the
    // needles are split so this file cannot match them.
    let verify = include_str!("verify.rs");
    let body = verify
        .split_once(concat!("\npub(crate) async fn vendor_off_", "worker("))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    let shared = body.find(concat!("speed::SharedStage::", "start(\"vendor scrub\")"));
    // R3-08: the scrub is `vendor_until` with the reader's flag, and it
    // finishes its stage answered or, when its reader left, cancelled.
    let scrub = body.find(concat!(
        "let report = ",
        "vendor_until(&root, &census, cancel);"
    ));
    let counted = body.find(concat!("in_scrub.", "finish(outcome)"));
    assert!(
        matches!((shared, scrub, counted), (Some(a), Some(b), Some(c)) if a < b && b < c),
        "the scrub's stage is finished by the scrub itself: {body}"
    );
}

/// **WHAT A STOP CUT IS NAMED BY ROW AND COUNT** (R3-14 repair 1). The cut
/// line said only its grace; a reader could not tell whether one request was
/// cut or fifty, or on which route. Every row with a request open is listed,
/// the over-cap row included before it has counted anything, and a request
/// counted (finished or dropped) leaves the list.
#[test]
fn what_a_stop_cut_is_named_by_row_and_count() {
    let speed = Speed::new(0, 2);
    let a = speed.row("/a", Kind::Route, 0);
    let b = speed.row("/b", Kind::Route, 0);
    let over = speed.row("/c", Kind::Route, 0);
    let first = Pending::start(a.clone());
    let second = Pending::start(a);
    let third = Pending::start(b);
    let fourth = Pending::start(over);
    let open = speed.open_now();
    assert_eq!(
        open,
        [
            ("/a".to_owned(), 2),
            ("/b".to_owned(), 1),
            (OVER_CAP.to_owned(), 1)
        ]
    );
    assert_eq!(
        open_summary(&open),
        (4, format!("/a=2, /b=1, {OVER_CAP}=1"))
    );
    first.finish(5, Outcome::Answered);
    drop(second);
    third.finish_now(Outcome::Failed);
    drop(fourth);
    assert!(speed.open_now().is_empty(), "{:?}", speed.open_now());
    assert_eq!(open_summary(&[]), (0, String::new()));
    let report = speed.report(1);
    let a = &report.rows[0].since_start;
    assert_eq!((a.n, a.answered, a.cancelled), (2, 1, 1));
}

/// **A REQUEST STILL OPEN WHEN THE PROCESS ENDS IS NAMED, NOT DROPPED**
/// (R3-14 repair 1). `flush_at_exit` runs after the runtime has dropped every
/// task, so each cut request has been counted by then; one whose future
/// outlived the runtime cannot be, and is said at `error` with its row.
#[test]
fn a_request_still_open_at_exit_is_named_not_dropped() {
    let mark = crate::emitted::mark();
    let route = "/speed-test/open-past-the-exit";
    let open = Pending::start(global().row(route, Kind::Route, now_ms()));
    let _lines = flush_at_exit();
    let named = crate::emitted::landed(
        mark,
        "api.speed",
        "requests still open at exit were not counted",
    );
    assert!(
        named
            .iter()
            .any(|record| crate::emitted::says(record, "routes", &format!("{route}=1"))),
        "the request open past the exit is named: {named:?}"
    );
    assert!(telemetry::is_audited(
        telemetry::Level::Error,
        "api.speed",
        "requests still open at exit were not counted"
    ));
    drop(open);
    let still: Vec<_> = global()
        .open_now()
        .into_iter()
        .filter(|(name, _)| name == route)
        .collect();
    assert!(still.is_empty(), "{still:?}");
}

/// **THE WHOLE STOP ENDS INSIDE WHAT A REDEPLOY WAITS FOR** (R3-14 repair 3).
///
/// Fails before the repair: the stop was bounded at twenty seconds (ten to
/// drain, ten for blocking work) while the operator's redeploy waits about ten
/// for the port, so a redeploy during a long read started the new build on a
/// port still bound; it was refused, and when the old process ended nothing
/// listened (measured: deploy loop ended at +13.4 s, the old process at +20 s).
#[test]
fn the_whole_stop_ends_inside_what_a_redeploy_waits_for() {
    let spare = STOP_BOUND / 2;
    assert!(
        STOP_BOUND + spare <= REDEPLOY_WAIT,
        "a stop bounded at {STOP_BOUND:?} leaves less than half of itself again ({spare:?}) \
         inside the {REDEPLOY_WAIT:?} a redeploy waits for the port"
    );
    assert_eq!(
        crate::server::ConnectionLimits::SERVED.drain_grace + crate::server::SHUTDOWN_GRACE,
        STOP_BOUND,
        "the served process's drain plus its runtime's end is the bound the stop line says"
    );
}

/// **THE STOP'S SIGNALS ARE REGISTERED BY THE CALL, NOT BY THE FIRST POLL**
/// (R3-14 repair 4). `stop_asked` was an `async fn`, so nothing was
/// registered until the serve first polled it, after the load and the bind:
/// a `SIGTERM` in the 1-2.5 s before ended the process with no line.
///
/// Called with no runtime, the eager form panics in tokio's registration
/// (`signal` needs a runtime's driver); the old form only built a future and
/// returned, so this half fails on it. Inside a runtime the call registers and
/// its future stays pending until a signal comes. That half runs in a child
/// of this test binary, because a registration changes the process's
/// `SIGINT` and `SIGTERM` for life and this binary's own must stay default.
///
/// No signal is sent: a test may not start `kill` (gate 0, D-1603) and this
/// crate forbids the unsafe call that raises one. That a signal landing
/// between the call and the first poll is kept is tokio's `watch` receiver,
/// made at registration; the OS half is shown on a shadow server stopped by
/// `kill -TERM` at once after its start (R3-14 repair 4's report).
#[test]
fn stop_asked_registers_its_signals_when_called_not_when_polled() {
    const CHILD: &str = "BRUTEX_STOP_REGISTERED_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let stop = {
            let _inside = runtime.enter();
            stop_asked()
        };
        let waited = runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_millis(50), stop).await
        });
        assert!(waited.is_err(), "no signal came, so no stop: {waited:?}");
        println!("STOP REGISTERED: pending with no signal");
        return;
    }
    let outside = std::panic::catch_unwind(|| {
        let _never_polled = stop_asked();
    });
    assert!(
        outside.is_err(),
        "with no runtime the call itself reaches tokio's registration and panics"
    );
    let out = crate::isolated::rerun(
        "speed::tests::stop_asked_registers_its_signals_when_called_not_when_polled",
        &[(CHILD, std::ffi::OsStr::new("1"))],
    );
    assert!(
        out.contains("STOP REGISTERED: pending with no signal"),
        "{out}"
    );
}

/// **THE TABLE AND A ROW NAME THEMSELVES AND NOTHING THEY HOLD, A
/// PERCENTILE NEVER PASSES ITS MAX, AND AN ACCESS WINDOW WHOSE CLOCK STEPPED
/// BACK IS CLOSED** (BASE-COV-API-AS2). Extremes: (1) the table's `Debug`
/// names its start and cap and no row; (2) a row's `Debug` names its route
/// alone; (3) a count larger than its buckets hold puts the percentile past
/// every bucket: it is the max; (4) the same at the 50th; (5) a count of
/// zero is zero whatever the max; (6) a clock that stepped back closes the
/// window it was in and opens one at the new time; (7) the closed window is
/// said when it held more than its one sampled read; (8) the first read of
/// the new window is sampled again. Combined: (9) a count past its buckets
/// AND a max of zero: zero, never a bucket's top; (10) a clock that stepped
/// back AND a refused read: the old window closes, the refusal is counted
/// in the new one and nothing is sampled. Out of the box: (11) a clock that
/// stepped back to zero from `u64::MAX`.
#[test]
fn the_table_names_itself_a_percentile_keeps_its_max_and_a_clock_step_back_closes() {
    let speed = Speed::new(5, 3);
    let _row = speed.row("/bars.json", Kind::Route, 6);
    let said = format!("{speed:?}");
    assert!(said.starts_with("Speed {"), "{said}");
    assert!(
        said.contains("started_ms: 5") && said.contains("cap: 3"),
        "{said}"
    );
    assert!(!said.contains("/bars.json"), "{said}");
    assert_eq!(
        format!("{:?}", speed.row("/spot.json", Kind::Route, 7)),
        "Handle(\"/spot.json\")"
    );
    // (3), (4), (5), (9): a count past its buckets.
    assert_eq!(percentile(&[1, 0, 0], 5, 99, 77), 77);
    assert_eq!(percentile(&[1, 0, 0], 5, 50, 77), 77);
    assert_eq!(percentile(&[1, 0, 0], 0, 99, 77), 0);
    assert_eq!(percentile(&[1, 0, 0], 5, 99, 0), 0);
    // (6)-(8): a clock that stepped back.
    let mut windows = AccessWindows::default();
    assert!(windows.account("/bars.json", false, 10, 1_000).sample);
    assert!(!windows.account("/bars.json", false, 20, 1_001).sample);
    let back = windows.account("/bars.json", false, 30, 500);
    let closed = back.closed.expect("the window the clock left is said");
    assert_eq!((closed.served, closed.opened_ms), (2, 1_000));
    assert!(back.sample, "the new window samples its first read");
    // (10): back again, refused.
    let refused = windows.account("/bars.json", true, 40, 100);
    assert!(refused.closed.is_none(), "one sampled read is its own line");
    assert!(!refused.sample);
    assert_eq!(windows.routes["/bars.json"].refused, 1);
    assert_eq!(windows.routes["/bars.json"].opened_ms, 100);
    // (11): from u64::MAX back to zero.
    let mut far = AccessWindows::default();
    let _ = far.account("/spot.json", false, 1, u64::MAX);
    let _ = far.account("/spot.json", false, 1, u64::MAX);
    let zero = far.account("/spot.json", false, 1, 0);
    assert_eq!(zero.closed.map(|window| window.served), Some(2));
    assert_eq!(far.routes["/spot.json"].opened_ms, 0);
}

/// **A ROW WITH NO KEPT COPY SAYS NO MINUTE UNTIL IT HAS ONE, AND THE NEXT
/// MINUTE COUNTS FROM THE FREEZE IT KEPT** (BASE-COV-API-AS2). A row is made
/// with one zero copy and `Speed::roll` never empties it, so a roll that
/// finds none has nothing to set its minute against: it says no line for
/// that row and keeps its freeze as the next base. Extremes: (1) a row whose
/// kept copies were emptied: a roll says no line for it though it counted a
/// request; (2) that roll keeps its freeze as the next base, one copy; (3)
/// emptied again and quiet: still nothing said; (4) the next minute's line
/// counts only what came after the base, one request, refused; (5) its window
/// is the minute since the base; (6) its max is that minute's 7 µs, not the
/// 40 µs before the base; (7) since-start keeps both requests; (8) the last
/// hour starts at the kept base: one request over one minute. Combined: (9)
/// emptied copies AND a shutdown roll: nothing said for that row; (10) a
/// sibling row with its copies in the same roll is said, partial, as usual.
/// Out of the box: (11) the cold first time survives the emptying: 40 µs.
#[test]
fn a_row_with_no_kept_copy_says_no_minute_until_it_has_one() {
    let speed = Speed::new(0, 4);
    let bare = speed.row("/speed-test/no-copy", Kind::Route, 0);
    let kept = speed.row("/speed-test/kept-copy", Kind::Route, 0);
    bare.add(40, Outcome::Answered);
    kept.add(9, Outcome::Answered);
    bare.0.frozen.lock().unwrap().clear();
    // (1), (2), (9), (10).
    let at_stop = speed.roll(MINUTE_MS, true);
    let said: Vec<(&str, u64, bool)> = at_stop
        .iter()
        .map(|line| (line.route.as_str(), line.figures.n, line.partial))
        .collect();
    assert_eq!(said, [("/speed-test/kept-copy", 1, true)]);
    assert_eq!(
        bare.0.frozen.lock().unwrap().len(),
        1,
        "the freeze is kept as the next minute's base"
    );
    // (3).
    bare.0.frozen.lock().unwrap().clear();
    assert!(speed.roll(2 * MINUTE_MS, false).is_empty());
    // (4)-(6).
    bare.add(7, Outcome::Refused);
    let next = speed.roll(3 * MINUTE_MS, false);
    assert_eq!(next.len(), 1, "{next:?}");
    let line = &next[0];
    assert_eq!(line.route, "/speed-test/no-copy");
    assert_eq!(
        (
            line.figures.n,
            line.figures.refused,
            line.figures.max_micros,
            line.window_ms
        ),
        (1, 1, 7, MINUTE_MS)
    );
    // (7), (8), (11).
    let report = speed.report(3 * MINUTE_MS);
    let row = report
        .rows
        .iter()
        .find(|row| row.route == "/speed-test/no-copy")
        .unwrap();
    assert_eq!(row.since_start.n, 2);
    assert_eq!((row.last_hour.n, row.last_hour_minutes), (1, 1));
    assert_eq!(row.first_after_start_micros, Some(40));
}

/// **A STAGE COUNTED DIRECTLY IS A STAGE ROW, NEVER A ROUTE ROW**
/// (BASE-COV-API-AS2). `stage` has no caller in the workspace; its contract
/// is driven here on the process table, under names no other test uses.
/// Extremes: (1) a stage counted by `stage` is the row `stage:<name>`, kind
/// stage; (2) its time is its max and its first; (3) a failed outcome is
/// counted failed; (4) a stage of zero microseconds counts with max 0; (5) a
/// cancelled outcome is counted cancelled; (6) the longest time, `u64::MAX`,
/// is the max and the p99 (the top bucket, clamped); (7) the first time stays
/// the first one; (8) one name twice is one row of two. Combined: (9) zero
/// time AND cancelled, twice: one row, two, both cancelled, max 0; (10) a
/// stage named like a route path stays a stage row and makes no route row.
/// Out of the box: (11) an empty stage name is the row `stage:`.
#[test]
fn a_stage_counted_directly_is_a_stage_row_and_never_a_route_row() {
    let named = |what: &str| format!("speed-test/as2 direct {what}");
    let row_of = |route: &str| {
        global()
            .report(now_ms())
            .rows
            .into_iter()
            .find(|row| row.route == route)
    };
    // (1)-(3).
    stage(&named("failed"), 1_500, Outcome::Failed);
    let failed = row_of(&format!("stage:{}", named("failed"))).expect("the stage's row");
    assert_eq!(failed.kind, Kind::Stage);
    let once = failed.since_start;
    assert_eq!((once.n, once.failed, once.max_micros), (1, 1, 1_500));
    assert_eq!(failed.first_after_start_micros, Some(1_500));
    // (4), (5), (8), (9).
    stage(&named("instant"), 0, Outcome::Cancelled);
    stage(&named("instant"), 0, Outcome::Cancelled);
    let instant = row_of(&format!("stage:{}", named("instant"))).expect("one row for both");
    let twice = instant.since_start;
    assert_eq!(
        (twice.n, twice.cancelled, twice.max_micros, twice.p99_micros),
        (2, 2, 0, 0)
    );
    // (6), (7).
    stage(&named("longest"), 5, Outcome::Answered);
    stage(&named("longest"), u64::MAX, Outcome::Answered);
    let longest = row_of(&format!("stage:{}", named("longest"))).expect("the row");
    let ends = longest.since_start;
    assert_eq!(
        (ends.max_micros, ends.p99_micros, ends.p50_micros),
        (u64::MAX, u64::MAX, 5)
    );
    assert_eq!(longest.first_after_start_micros, Some(5));
    // (10).
    let shaped = "/speed-test/as2-route-shaped";
    stage(shaped, 3, Outcome::Answered);
    assert_eq!(
        row_of(&format!("stage:{shaped}")).map(|row| row.kind),
        Some(Kind::Stage)
    );
    assert!(row_of(shaped).is_none(), "a stage makes no route row");
    // (11).
    stage("", 4, Outcome::Refused);
    let bare = row_of("stage:").expect("an empty name is the row `stage:`");
    assert_eq!(bare.kind, Kind::Stage);
    assert!(bare.since_start.refused >= 1, "{bare:?}");
}

/// **A STOP ASKED UNDER A RUNTIME THAT IS GONE ANSWERS CTRL-C'S OWN ERROR AND
/// SAYS `SIGTERM` CANNOT BE CAUGHT** (BASE-COV-API-AS2). tokio refuses to
/// register a signal on a runtime whose driver was dropped ("signal driver
/// gone"), before it touches the process's handlers, so `stop_asked` called
/// with such a runtime's handle entered takes the arm that names the
/// uncatchable `SIGTERM` and answers Ctrl-C's registration error, as its
/// `# Errors` section says. It runs in a child all the same, as the
/// registering test beside it does; the child proves it ran by a file its
/// parent reads. Extremes: (1) a runtime built and dropped before the call:
/// its handle, entered, still lets the call return a future without a panic;
/// (2) the future resolves at once, waiting for no signal; (3) to Ctrl-C's
/// own registration error, never a stop; (4) in tokio's words, "signal
/// driver gone"; (5) "SIGTERM cannot be caught" is said, at error level; (6)
/// with the same reason; (7) a second call on the same dead handle answers
/// the same; (8) and says the line again: two lines for two polls. Combined:
/// (9) a dead runtime's handle AND a poller with no signal driver: the same
/// answer; (10) two futures from one dead handle polled one after the other:
/// two errors, two lines. Out of the box: (11) a runtime that never ran a
/// task before it died.
#[test]
fn a_stop_asked_of_a_runtime_that_is_gone_answers_ctrl_cs_error_and_says_so() {
    const CHILD: &str = "BRUTEX_TEST_STOP_DRIVER_GONE_CHILD";
    if let Some(proof) = std::env::var_os(CHILD) {
        let mark = crate::emitted::mark();
        let gone = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        let handle = gone.handle().clone();
        drop(gone);
        let ask = || {
            let _inside = handle.enter();
            stop_asked()
        };
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(ask))
            .expect("a dead runtime's handle is entered without a panic");
        let second = ask();
        let poller = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("a runtime with a clock and no signal driver");
        let answers: Vec<String> = [first, second]
            .into_iter()
            .map(|asked| {
                poller
                    .block_on(async {
                        tokio::time::timeout(std::time::Duration::from_secs(10), asked).await
                    })
                    .expect("answered at once: no signal is waited for")
                    .expect_err("Ctrl-C's own registration error, never a stop")
                    .to_string()
            })
            .collect();
        assert_eq!(answers, ["signal driver gone", "signal driver gone"]);
        let said = crate::emitted::landed(mark, "api.speed", "SIGTERM cannot be caught");
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(
            said.iter()
                .all(|record| record.level == telemetry::Level::Error
                    && crate::emitted::says(record, "why", "signal driver gone")),
            "{said:?}"
        );
        std::fs::write(
            std::path::Path::new(&proof).join("child-ran"),
            answers.join("; "),
        )
        .expect("the child's proof");
        return;
    }
    let proof = crate::scratch::path("speed-stop-driver-gone");
    let _ = std::fs::remove_dir_all(&proof);
    std::fs::create_dir_all(&proof).expect("the child's proof folder");
    let _ = crate::isolated::rerun(
        "speed::tests::a_stop_asked_of_a_runtime_that_is_gone_answers_ctrl_cs_error_and_says_so",
        &[(CHILD, proof.as_os_str())],
    );
    assert_eq!(
        std::fs::read_to_string(proof.join("child-ran"))
            .ok()
            .as_deref(),
        Some("signal driver gone; signal driver gone"),
        "the child asked twice under a dead runtime and heard Ctrl-C's error twice"
    );
    let _ = std::fs::remove_dir_all(&proof);
}

/// **A MINUTE THREAD THAT CANNOT START IS SAID, AND THE TABLE STILL COUNTS**
/// (BASE-COV-API-AS2). `global()` makes the table even when the OS refuses
/// its minute thread, says so once at `error`, and the minutes are then
/// written only at shutdown. The refusal is arranged in a child whose
/// `RUST_MIN_STACK` asks every new thread for 2^60 bytes of stack, which no
/// address space holds, so `pthread_create` answers `EAGAIN`. The child
/// proves it ran by a file its parent reads. Extremes: (1) the minute thread
/// that cannot start is said once, at error level; (2) with the OS's reason;
/// (3) the process table is made all the same; (4) a stage counted on it is
/// kept, once, 42 µs; (5) its first time is 42 µs; (6) a route counted beside
/// it is kept too; (7) the report lists the route before the stage; (8) with
/// no minute thread ever rolling, the shutdown freeze writes both rows'
/// minute. Combined: (9) the thread refused AND a stage counted before any
/// route: the route still leads the report; (10) the thread refused AND the
/// shutdown freeze: two lines. Out of the box: (11) a stack request of one
/// exbibyte.
#[test]
fn a_minute_thread_that_cannot_start_is_said_and_the_table_still_counts() {
    const CHILD: &str = "BRUTEX_TEST_SPEED_NO_MINUTE_THREAD_CHILD";
    if let Some(proof) = std::env::var_os(CHILD) {
        let mark = crate::emitted::mark();
        assert!(TABLE.get().is_none(), "this child has made no table yet");
        stage("as2 first count", 42, Outcome::Answered);
        assert!(TABLE.get().is_some(), "the table is made all the same");
        let said = crate::emitted::landed(mark, "api.speed", "the minute thread did not start");
        assert_eq!(said.len(), 1, "{said:?}");
        let record = said.first().expect("the line");
        assert_eq!(record.level, telemetry::Level::Error);
        let why = record
            .field("why")
            .and_then(telemetry::OwnedValue::as_str)
            .unwrap_or("");
        assert!(why.contains("os error"), "{said:?}");
        global().record(
            "/speed-test/as2-beside",
            Kind::Route,
            9,
            Outcome::Refused,
            now_ms(),
        );
        let report = global().report(now_ms());
        let names: Vec<&str> = report.rows.iter().map(|row| row.route.as_str()).collect();
        assert_eq!(names, ["/speed-test/as2-beside", "stage:as2 first count"]);
        let counted = report
            .rows
            .iter()
            .find(|row| row.kind == Kind::Stage)
            .expect("the stage row");
        assert_eq!(
            (
                counted.since_start.n,
                counted.since_start.max_micros,
                counted.first_after_start_micros
            ),
            (1, 42, Some(42))
        );
        assert_eq!(
            flush_at_shutdown(),
            2,
            "no minute thread rolled: both rows' minute"
        );
        std::fs::write(std::path::Path::new(&proof).join("child-ran"), b"no thread")
            .expect("the child's proof");
        return;
    }
    let proof = crate::scratch::path("speed-no-minute-thread");
    let _ = std::fs::remove_dir_all(&proof);
    std::fs::create_dir_all(&proof).expect("the child's proof folder");
    let _ = crate::isolated::rerun(
        "speed::tests::a_minute_thread_that_cannot_start_is_said_and_the_table_still_counts",
        &[
            (CHILD, proof.as_os_str()),
            (
                "RUST_MIN_STACK",
                std::ffi::OsStr::new("1152921504606846976"),
            ),
        ],
    );
    assert_eq!(
        std::fs::read(proof.join("child-ran")).ok().as_deref(),
        Some(&b"no thread"[..]),
        "the child's minute thread was refused and its table still counted"
    );
    let _ = std::fs::remove_dir_all(&proof);
}

/// **THE MINUTE THREAD WRITES NOTHING WHILE NO TABLE IS MADE, AND KEEPS
/// RUNNING** (BASE-COV-API-AS2). `global()` makes the table as it starts the
/// thread, whose first wake is the next minute's end, so a served process
/// never shows the thread an unmade table; started here in a child that makes
/// none, it wakes to an empty slot. SLOW: it waits for one wall-clock minute
/// boundary. Extremes: (1) no table is made before the thread starts; (2) the
/// thread announces its start before its first wait; (3) the wall clock
/// passes the boundary it waited for; (4) it is still running after it woke;
/// (5) it made no table; (6) no minute line was written; (7) the shutdown
/// freeze finds no table: zero lines; (8) the exit flush likewise, naming
/// nothing open. Combined: (9) no table AND no line after the wake; (10) both
/// flushes at once: zero and zero. Out of the box: (11) a minute thread
/// started with no table at all, which `global()` never does.
#[test]
fn the_minute_thread_writes_nothing_while_no_table_is_made() {
    const CHILD: &str = "BRUTEX_TEST_MINUTE_THREAD_NO_TABLE_CHILD";
    if let Some(proof) = std::env::var_os(CHILD) {
        let mark = crate::emitted::mark();
        assert!(TABLE.get().is_none(), "this child makes no table");
        let (started, heard) = std::sync::mpsc::channel::<u64>();
        let thread = std::thread::Builder::new()
            .name("speed-minute-under-test".into())
            .spawn(move || {
                let _sent = started.send(now_ms());
                minute_thread();
            })
            .expect("the thread starts");
        let at = heard.recv().expect("the thread announced its start");
        let boundary = (at / MINUTE_MS + 1) * MINUTE_MS;
        let woken = if boundary - at < 2_000 {
            boundary + MINUTE_MS
        } else {
            boundary
        };
        while now_ms() < woken + 2_000 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(now_ms() >= woken);
        assert!(
            !thread.is_finished(),
            "an unmade table does not end the thread"
        );
        assert!(
            TABLE.get().is_none(),
            "the minute thread never makes the table"
        );
        assert!(crate::emitted::landed(mark, "api.speed", "minute ended").is_empty());
        assert_eq!(flush_at_shutdown(), 0);
        assert_eq!(flush_at_exit(), 0);
        std::fs::write(
            std::path::Path::new(&proof).join("child-ran"),
            b"woke to no table",
        )
        .expect("the child's proof");
        return;
    }
    let proof = crate::scratch::path("speed-minute-thread-no-table");
    let _ = std::fs::remove_dir_all(&proof);
    std::fs::create_dir_all(&proof).expect("the child's proof folder");
    let _ = crate::isolated::rerun(
        "speed::tests::the_minute_thread_writes_nothing_while_no_table_is_made",
        &[(CHILD, proof.as_os_str())],
    );
    assert_eq!(
        std::fs::read(proof.join("child-ran")).ok().as_deref(),
        Some(&b"woke to no table"[..])
    );
    let _ = std::fs::remove_dir_all(&proof);
}

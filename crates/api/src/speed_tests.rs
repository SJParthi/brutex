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
    let all: serde_json::Value =
        serde_json::from_str(&crate::speed_route::answer(&speed, "", 5).unwrap()).unwrap();
    assert_eq!(all["rows"].as_array().unwrap().len(), 2);
    assert_eq!(all["resolution"], RESOLUTION);
    assert_eq!(all["now_ms"], 5);
    let one: serde_json::Value =
        serde_json::from_str(&crate::speed_route::answer(&speed, "route=%2Fspot.json", 5).unwrap())
            .unwrap();
    assert_eq!(one["rows"].as_array().unwrap().len(), 1);
    assert_eq!(one["rows"][0]["route"], "/spot.json");
    assert_eq!(one["rows"][0]["since_start"]["max_micros"], 20);
    assert_eq!(one["rows"][0]["kind"], "route");
    let none: serde_json::Value =
        serde_json::from_str(&crate::speed_route::answer(&speed, "route=%2Fnever", 5).unwrap())
            .unwrap();
    assert_eq!(none["rows"].as_array().unwrap().len(), 0);
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
    let _front = get("/no-such-page-at-all").await.unwrap();
    let page = get("/page-report").await.unwrap();
    assert!(
        page.starts_with("HTTP/1.1 405 "),
        "the report route is registered, POST only: {page}"
    );
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
    let body = speed
        .split_once(concat!("\npub async fn stop_", "asked("))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map_or("", |(body, _)| body);
    let registered = body.find(concat!("SignalKind::", "terminate()"));
    let raced = body.find(concat!("first_", "stop(tokio::signal::ctrl_c()"));
    assert!(
        matches!((registered, raced), (Some(t), Some(r)) if t < r),
        "stop_asked registers SIGTERM and races it against Ctrl-C: {body}"
    );

    // AND THE PATH IS WIRED: the binary passes this future, and the server
    // writes the open minute after its graceful stop. The needles are split
    // so this file cannot match them.
    let main = include_str!("main.rs");
    assert!(
        main.contains(concat!("Box::pin(api::speed::", "stop_asked())")),
        "the binary stops on SIGTERM too"
    );
    // AND ENDS THROUGH `end_served`, which counts what the drain cut and
    // only then frees the store and the port (R3-14 repair 1).
    let ran = main.find(concat!("api::server::run_", "held(&args"));
    let ended = main.find(concat!("api::server::end_", "served(runtime"));
    assert!(
        matches!((ran, ended), (Some(r), Some(e)) if r < e),
        "the binary keeps what the serve held and ends through end_served"
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
    let scrub = body.find(concat!("let report = ", "vendor(&root, &census);"));
    let counted = body.find(concat!(
        "in_scrub.",
        "finish(crate::speed::Outcome::Answered)"
    ));
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

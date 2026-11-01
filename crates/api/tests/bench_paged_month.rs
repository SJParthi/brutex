//! The ratio bench's whole-month read, proved against the real router
//! (BASE-BENCH-1028, D-7400).
//!
//! Gate 8's api bench (`crates/api/benches/ratio.rs`, its `reads` module)
//! read a whole one-second month in one `/bars.json` answer at 1,024, 10,240
//! and 102,400 bars. Since R3-08 that route serves at most 60,000 rows and
//! answers `400 too_many_rows` past it, so the 100x case measured nothing and
//! the bench exited 1. The bench now reads the month the way a client must:
//! page by page through `/bars/window.json`, with the reader in
//! `crates/api/benches/ratio/paged.rs`. This file mounts that same module by
//! `#[path]` (a bench is `test = false`, so its own tests never run) and
//! proves, against months served by the production router, that it reads
//! every row exactly once at every size the bench times and at every extreme
//! around them, that the 60,000-row cap is kept, and that a server answering
//! anything wrong is refused by name rather than timed.
//!
//! No row here is market data: every bar is generated, at an invented level
//! (12,345.00) no real index printed in June 2024, in a scratch store under
//! the system temporary directory that each test removes.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

#[path = "../benches/ratio/paged.rs"]
mod paged;

use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use paged::{MONTH_ANSWER_CAP_ROWS, MonthRead, Page, read_month};
use pull::manifest::{Entry, EntryKey, Manifest};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The month sizes the bench times, `MONTH_SIZES` in `benches/ratio.rs`.
const BENCH_MONTH_SIZES: [usize; 3] = [1_024, 10_240, 102_400];

/// 2024-06-01 00:00 IST in epoch seconds, where every June fixture starts.
const JUNE_START_S: i64 = 1_717_180_200;

/// 2024-07-01 00:00 IST in epoch seconds.
const JULY_START_S: i64 = JUNE_START_S + 30 * 86_400;

/// 2024-08-01 00:00 IST in epoch seconds.
const AUGUST_START_S: i64 = JULY_START_S + 31 * 86_400;

/// The series every fixture month is filed under, without its months.
const SERIES: &str = "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s";

/// The window query over `from` to `to`.
fn range(from: &str, to: &str) -> String {
    format!("{SERIES}&from={from}&to={to}")
}

/// June 2024 alone, the bench's own month.
fn june() -> String {
    range("2024-06", "2024-06")
}

/// One month a store holds: its year-month, its first second, its rows.
struct Month {
    year: u16,
    number: u8,
    start_s: i64,
    rows: usize,
}

/// June 2024 of `rows` one-second bars from its first instant.
fn june_of(rows: usize) -> Month {
    Month {
        year: 2024,
        number: 6,
        start_s: JUNE_START_S,
        rows,
    }
}

/// A scratch store served by the production router on a loopback port.
struct Served {
    root: PathBuf,
    addr: SocketAddr,
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The low 32 bits of the symbol's FNV-1a, the id `api::bars::open` checks.
fn symbol_id() -> u32 {
    u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).expect("32 bits")
}

/// Writes one month of NIFTY index one-second bars under `root`.
fn write_month(root: &Path, month: &Month) -> Entry {
    let ym = YearMonth::new(month.year, month.number).expect("a real month");
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: ym,
        file: FileKind::Bars,
    })
    .expect("a legal path");
    let mut file =
        store::file::BarFile::open_or_create(root, path, symbol_id()).expect("a fixture month");
    let bars: Vec<Bar> = (0..month.rows)
        .map(|i| {
            let wiggle = i64::try_from(i % 97).expect("small");
            let at = month.start_s + i64::try_from(i).expect("small");
            Bar {
                ts_micros: at * 1_000_000,
                open: 1_234_500 + wiggle,
                high: 1_234_505 + wiggle,
                low: 1_234_495 + wiggle,
                close: 1_234_500 + wiggle,
                volume: 1 + wiggle,
                open_interest: OI_NULL,
            }
        })
        .collect();
    if !bars.is_empty() {
        file.append(&bars).expect("the fixture fills");
    }
    let last = month.start_s + i64::try_from(month.rows.saturating_sub(1)).expect("small");
    Entry {
        key: EntryKey {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new("NIFTY").expect("a symbol"),
            timeframe: Timeframe::SECOND_1,
            month: ym,
        },
        rows: u64::try_from(month.rows).expect("small"),
        first_ts_micros: month.start_s * 1_000_000,
        last_ts_micros: last * 1_000_000,
    }
}

/// Builds a store of `months` and serves it from a runtime of its own.
fn served(tag: &str, months: &[Month]) -> Served {
    let root =
        std::env::temp_dir().join(format!("brutex-bench-paged-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let mut held = Vec::with_capacity(months.len());
    for month in months {
        let entry = write_month(&root, month);
        // THE CENSUS HOLDS NO EMPTY MONTH (`EmptyEntry`): an empty month is
        // its file alone, as a month created and never filled is.
        if month.rows > 0 {
            held.push(entry);
        }
    }
    paged::write_census(&root, &[], held).expect("the census writes");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = listener.local_addr().expect("its address");
    listener.set_nonblocking(true).expect("non-blocking");
    let site = api::server::Site::load(&root.join("masters"), &root);
    let router = api::server::router_serving(
        Arc::new(site),
        Arc::new(api::assets::Assets::new(&root.join("web"))),
        addr,
    );
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime");
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).expect("a listener");
            axum::serve(listener, router)
                .await
                .expect("the server runs");
        });
    });
    Served { root, addr }
}

/// The first answer from `once` that is not busy: a full pool (`429`) or a
/// route still building (`503`) is asked again, at most two seconds. Any
/// other answer, a refusal included, is the answer.
fn patient(path: &str, mut once: impl FnMut() -> Page) -> Page {
    for _ in 0..400 {
        let page = once();
        if page.status != 429 && page.status != 503 {
            return page;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("{path} stayed busy for two seconds");
}

impl Served {
    /// One GET on fresh connections, patiently.
    fn get(&self, path: &str) -> Page {
        patient(path, || self.once(path))
    }

    /// One GET on a fresh connection, read to its close (`Connection: close`,
    /// so the close ends the body: no route here answers chunked).
    fn once(&self, path: &str) -> Page {
        let mut socket = TcpStream::connect(self.addr).expect("connect");
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.addr
        );
        socket.write_all(request.as_bytes()).expect("write");
        let mut raw = Vec::new();
        socket.read_to_end(&mut raw).expect("read");
        let split = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("a head");
        let head = String::from_utf8_lossy(&raw[..split]);
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("a status");
        Page {
            status,
            body: String::from_utf8(raw[split + 4..].to_vec()).expect("UTF-8"),
        }
    }

    /// The whole month as the bench reads it.
    fn read(&self, query: &str) -> Result<MonthRead, String> {
        read_month(query, |path| self.get(path))
    }
}

/// What reading `rows` one-second rows from `start_s` must give.
fn whole(rows: usize, start_s: i64) -> MonthRead {
    let pages = rows.div_ceil(api::bars::MAX_WINDOW_LIMIT).max(1);
    let last = start_s + i64::try_from(rows).expect("small") - 1;
    MonthRead {
        rows,
        pages,
        first_t: (rows > 0).then_some(start_s),
        last_t: (rows > 0).then_some(last),
    }
}

/// THE RED THIS PACKAGE CLOSES: at each month size the bench times, its
/// whole-month read answers every row, once, in order. At 102,400 rows the
/// one-answer read it replaced was refused `400 too_many_rows`.
#[test]
fn the_benchs_whole_month_read_answers_every_size_it_times() {
    // LARGEST FIRST: the 100x month is the one the one-answer read lost.
    for rows in BENCH_MONTH_SIZES.into_iter().rev() {
        let store = served(&format!("bench-{rows}"), &[june_of(rows)]);
        assert_eq!(
            store.read(&june()),
            Ok(whole(rows, JUNE_START_S)),
            "{rows} rows"
        );
    }
}

/// Page edges: one row, one short of a page, a page, one past it, two
/// pages, and two pages and one row (a last page of exactly one row).
#[test]
fn every_row_is_read_once_across_each_page_edge() {
    for rows in [1, 999, 1_000, 1_001, 2_000, 2_001] {
        let store = served(&format!("edge-{rows}"), &[june_of(rows)]);
        assert_eq!(
            store.read(&june()),
            Ok(whole(rows, JUNE_START_S)),
            "{rows} rows"
        );
    }
}

/// An empty month (its file holds a header and no bar) reads as no rows in
/// one page: one request, never a loop.
#[test]
fn an_empty_month_reads_as_no_rows_in_one_page() {
    let store = served("empty", &[june_of(0)]);
    assert_eq!(store.read(&june()), Ok(whole(0, JUNE_START_S)));
}

/// TWO EXTREMES AT ONCE: the cap's own edge and a one-row last page. At
/// 60,000 rows `/bars.json` still answers the whole month; at 60,001 it
/// refuses by name, naming the 60,000 it keeps, while the pager reads all
/// 60,001 in 61 pages, the last of one row. A raised or removed cap fails
/// here, as does a cap that slipped to refuse the 60,000th row.
#[test]
fn the_cap_is_kept_at_60_000_rows_and_the_pager_reads_past_it() {
    let at_cap = served("cap", &[june_of(MONTH_ANSWER_CAP_ROWS)]);
    let one_answer = at_cap.get(&format!("/bars.json?{SERIES}&month=2024-06"));
    assert_eq!(one_answer.status, 200, "{}", &one_answer.body[..200]);
    assert_eq!(
        one_answer.body.matches("{\"t\":").count(),
        MONTH_ANSWER_CAP_ROWS
    );
    assert_eq!(
        at_cap.read(&june()),
        Ok(whole(MONTH_ANSWER_CAP_ROWS, JUNE_START_S))
    );
    drop(at_cap);

    let past = served("past-cap", &[june_of(MONTH_ANSWER_CAP_ROWS + 1)]);
    let refused = past.get(&format!("/bars.json?{SERIES}&month=2024-06"));
    assert_eq!(refused.status, 400);
    assert!(
        refused.body.contains("too_many_rows")
            && refused
                .body
                .contains(&paged::cap_sentence(MONTH_ANSWER_CAP_ROWS)),
        "{}",
        refused.body
    );
    let read = past.read(&june()).expect("the pager reads past the cap");
    assert_eq!(read, whole(MONTH_ANSWER_CAP_ROWS + 1, JUNE_START_S));
    assert_eq!(read.pages, 61);
}

/// TWO EXTREMES AT ONCE: a window across a month edge where the edge falls
/// exactly on a page edge (June holds 1,000 rows) and July holds one row;
/// then an edge inside a page (June 1,500, July 700).
#[test]
fn a_window_across_a_month_edge_pages_on_through_it() {
    for (june_rows, july_rows) in [(1_000, 1), (1_500, 700)] {
        let store = served(
            &format!("edge-{june_rows}-{july_rows}"),
            &[
                june_of(june_rows),
                Month {
                    year: 2024,
                    number: 7,
                    start_s: JULY_START_S,
                    rows: july_rows,
                },
            ],
        );
        let read = store
            .read(&range("2024-06", "2024-07"))
            .expect("a window across the edge");
        let rows = june_rows + july_rows;
        assert_eq!(read.rows, rows);
        assert_eq!(read.pages, rows.div_ceil(1_000));
        assert_eq!(read.first_t, Some(JUNE_START_S));
        assert_eq!(
            read.last_t,
            Some(JULY_START_S + i64::try_from(july_rows).unwrap() - 1)
        );
    }
}

/// A month missing in the middle of the range asked is refused by name,
/// never read as a month with no rows.
#[test]
fn a_missing_month_mid_range_is_refused_not_read_as_empty() {
    let store = served(
        "gap",
        &[
            june_of(10),
            Month {
                year: 2024,
                number: 8,
                start_s: AUGUST_START_S,
                rows: 10,
            },
        ],
    );
    let refused = store.read(&range("2024-06", "2024-08"));
    let why = refused.expect_err("a missing month is not an empty month");
    assert!(why.contains("months_missing"), "{why}");
}

/// Two readers at once over one month both read every row: the reader holds
/// no state between reads and the route serves concurrent pagers.
#[test]
fn two_readers_at_once_both_read_the_whole_month() {
    let store = Arc::new(served("two", &[june_of(10_240)]));
    let readers: Vec<_> = (0..2)
        .map(|_| {
            let store = Arc::clone(&store);
            std::thread::spawn(move || store.read(&june()))
        })
        .collect();
    for reader in readers {
        assert_eq!(
            reader.join().expect("a reader"),
            Ok(whole(10_240, JUNE_START_S))
        );
    }
}

/// What [`read_month`] did with `query`, a server answering an empty month:
/// its result, and how many requests it made.
fn asked_of(query: &str) -> (Result<MonthRead, String>, usize) {
    let mut asked = 0;
    let read = read_month(query, |_| {
        asked += 1;
        ok(0, &[])
    });
    (read, asked)
}

/// The paging keys belong to the reader: a query that brings its own is
/// refused before a request is made, by the key's name, wherever it stands
/// and with or without a value. A key merely spelled like one is another
/// key: it is asked, exactly once.
#[test]
fn a_query_carrying_its_own_paging_keys_is_refused_before_asking() {
    for key in ["dir", "limit", "offset", "sort"] {
        for query in [
            format!("{}&{key}=1", june()),
            format!("{key}=1&{}", june()),
            format!("{}&{key}", june()),
        ] {
            let (refused, asked) = asked_of(&query);
            let why = refused.expect_err("its own paging key");
            assert!(why.contains(&format!("{key}=")), "{why}");
            assert_eq!(asked, 0, "{query}: nothing is asked");
        }
        for other in [format!("{key}s"), format!("x{key}")] {
            let (read, asked) = asked_of(&format!("{}&{other}=1", june()));
            assert_eq!(read, Ok(whole(0, JUNE_START_S)), "{other}");
            assert_eq!(asked, 1, "{other}");
        }
    }
}

/// `patient` asks a busy route again and stops at the first other answer: a
/// full pool and a route still building are waited through, in turn.
#[test]
fn a_busy_answer_is_asked_again_until_it_is_not() {
    let mut statuses = [429, 503, 429, 200].into_iter();
    let mut asked = 0;
    let page = patient("/busy", || {
        asked += 1;
        answer(statuses.next().expect("asked past the last"), "done")
    });
    assert_eq!((page.status, page.body.as_str(), asked), (200, "done", 4));
}

/// A refusal or a fault is an answer, never a wait: asked once, handed back.
#[test]
fn a_refusal_or_a_fault_is_never_asked_again() {
    for status in [400, 404, 428, 430, 500, 502, 504] {
        let mut asked = 0;
        let page = patient("/refused", || {
            asked += 1;
            answer(status, "no")
        });
        assert_eq!((page.status, asked), (status, 1));
    }
}

/// A route busy for the whole two seconds stops the test by name: a wait
/// that never ends would hang the suite and prove nothing.
#[test]
#[should_panic(expected = "/always-busy stayed busy for two seconds")]
fn a_route_busy_for_two_seconds_stops_the_test() {
    let _never = patient("/always-busy", || answer(503, "building"));
}

/// A sample of a read that took every row answers `200`, shows the FIRST
/// page's first 220 characters and says what the read took; it asks every
/// page to get there.
#[test]
fn a_whole_paged_sample_answers_200_and_shows_its_first_page() {
    let limit = api::bars::MAX_WINDOW_LIMIT;
    let shown: String = body(1_500, &run(0, limit), 0, 0)
        .chars()
        .take(220)
        .collect();
    let mut pages = vec![ok(1_500, &run(0, limit)), ok(1_500, &run(1_000, 500))].into_iter();
    let sampled = paged::sample_month(&june(), |_| pages.next().expect("two pages"));
    assert_eq!(sampled.status, 200);
    assert_eq!(sampled.body, shown);
    assert_eq!(
        sampled.read,
        Some(MonthRead {
            rows: 1_500,
            pages: 2,
            first_t: Some(0),
            last_t: Some(1_499),
        })
    );
    assert!(pages.next().is_none(), "every page was asked");
}

/// An empty month's body is shorter than what a sample shows: all of it.
#[test]
fn a_sample_shorter_than_what_it_shows_shows_all_of_it() {
    let sampled = paged::sample_month(&june(), |_| ok(0, &[]));
    assert_eq!(sampled.status, 200);
    assert_eq!(sampled.body, body(0, &[], 0, 0));
    assert_eq!(sampled.read, Some(whole(0, JUNE_START_S)));
}

/// OUT OF THE BOX: a first page whose 220th character is two bytes wide. A
/// cut by bytes would split it and panic; the sample cuts by characters.
#[test]
fn a_sample_cuts_its_first_page_on_a_whole_character() {
    let wide = "é".repeat(300);
    let page = format!(
        "{{\"total\":0,\"months_missing\":0,\"faulted_records\":0,\"note\":\"{wide}\",\"bars\":[]}}"
    );
    let sampled = paged::sample_month(&june(), |_| answer(200, &page));
    assert_eq!(sampled.status, 200);
    assert_eq!(sampled.body.chars().count(), 220);
    assert!(sampled.body.ends_with('é'), "{}", sampled.body);
}

/// A read refused on its first page, or on a later one after the first was
/// fine, answers status 0 with the refusal's sentence as its body and no
/// read: the bench's status check stops on it, never timing it as a read,
/// and the good first page never stands in for the refusal.
#[test]
fn a_refused_paged_sample_answers_status_0_with_its_sentence() {
    let limit = api::bars::MAX_WINDOW_LIMIT;
    let cases = [
        (vec![answer(400, "{\"error\":\"no\"}")], "answered 400"),
        (
            vec![ok(2_000, &run(0, limit)), answer(500, "a fault")],
            "answered 500",
        ),
        (
            vec![ok(2_000, &run(0, limit)), ok(2_001, &run(1_000, limit))],
            "changed under the read",
        ),
    ];
    for (pages, says) in cases {
        let mut pages = pages.into_iter();
        let sampled = paged::sample_month(&june(), |_| pages.next().expect("a page"));
        assert_eq!(sampled.status, 0, "{says}");
        assert!(sampled.body.contains(says), "{says}: {}", sampled.body);
        assert!(!sampled.body.contains("\"total\":2000"), "{}", sampled.body);
        assert_eq!(sampled.read, None, "{says}");
    }
}

/// Every way an answer can be, or not be, the full one a target wants. Two
/// extremes at once: a building answer with the wanted status and needle
/// (only its sign of building is wrong), and an answer wrong in every way.
#[test]
fn an_answer_is_in_full_only_with_its_status_its_needle_and_no_sign_of_building() {
    let total = "\"total\":";
    let cases: [(&str, u16, &str, &str, u16, &str, bool); 11] = [
        (
            "the plain full answer",
            200,
            "",
            "{\"total\":1}",
            200,
            total,
            true,
        ),
        (
            "a 304 wants an empty body",
            304,
            "etag: x",
            "",
            304,
            "",
            true,
        ),
        (
            "a wanted refusal",
            400,
            "",
            "too_many_rows",
            400,
            "too_many_rows",
            true,
        ),
        (
            "the word without its quotes",
            200,
            "",
            "a building, \"total\":1",
            200,
            total,
            true,
        ),
        (
            "a refused paged sample",
            0,
            "",
            "answered 400",
            200,
            total,
            false,
        ),
        (
            "the wrong status",
            500,
            "",
            "{\"total\":1}",
            200,
            total,
            false,
        ),
        ("no needle", 200, "", "{\"other\":1}", 200, total, false),
        (
            "still building, in its body",
            200,
            "",
            "{\"building\":true,\"total\":1}",
            200,
            total,
            false,
        ),
        (
            "still building, in its head",
            200,
            "retry-after: 1",
            "{\"total\":1}",
            200,
            total,
            false,
        ),
        (
            "building, all else right",
            503,
            "retry-after: 1",
            "\"building\"",
            503,
            "\"building\"",
            false,
        ),
        (
            "wrong in every way",
            0,
            "retry-after: 9",
            "\"building\"",
            200,
            total,
            false,
        ),
    ];
    for (what, status, head, text, wanted, needle, full) in cases {
        assert_eq!(
            paged::answered_in_full(status, head, text, wanted, needle),
            full,
            "{what}"
        );
    }
}

/// A long patience that is never used.
const NO_HURRY: std::time::Duration = std::time::Duration::from_secs(3_600);

/// A pause too short to slow a test.
const A_MOMENT: std::time::Duration = std::time::Duration::from_millis(1);

/// An answer in full at once is one ask and no pause, however long the pause.
#[test]
fn an_answer_in_full_at_once_is_one_ask_and_no_pause() {
    let started = std::time::Instant::now();
    let mut asked = 0;
    let waited = paged::until_in_full(
        NO_HURRY,
        NO_HURRY,
        || {
            asked += 1;
            asked
        },
        |_| true,
    );
    assert_eq!((waited.last, waited.in_full), (1, true));
    assert_eq!(asked, 1);
    assert!(started.elapsed() < NO_HURRY, "it never paused");
}

/// A route still building is asked again until it is not, with the pause
/// between asks, and the cold figure stays the FIRST answer's: the first
/// ask here is the only slow one, so a figure taken from a later one fails.
#[test]
fn a_route_still_building_is_waited_for_and_its_first_answer_is_the_cold_one() {
    const SLOW: std::time::Duration = std::time::Duration::from_millis(20);
    const PAUSE: std::time::Duration = std::time::Duration::from_millis(10);
    let started = std::time::Instant::now();
    let mut asked = 0;
    let waited = paged::until_in_full(
        NO_HURRY,
        PAUSE,
        || {
            asked += 1;
            if asked == 1 {
                std::thread::sleep(SLOW);
            }
            asked
        },
        |&answer| answer == 3,
    );
    assert_eq!((waited.last, waited.in_full), (3, true));
    assert!(
        waited.first_micros >= SLOW.as_micros(),
        "{}",
        waited.first_micros
    );
    assert!(
        started.elapsed() >= SLOW + 2 * PAUSE,
        "the pause was skipped"
    );
}

/// With no patience at all it still asks, then gives up and says so,
/// handing back the last answer for the caller's sentence.
#[test]
fn a_route_that_never_answers_in_full_ends_the_wait_not_in_full() {
    let mut asked = 0;
    let waited = paged::until_in_full(
        std::time::Duration::ZERO,
        A_MOMENT,
        || {
            asked += 1;
            asked
        },
        |_| false,
    );
    assert!(!waited.in_full);
    assert!(waited.last >= 1);
    assert_eq!(waited.last, asked, "the last answer is the last asked");
}

/// TWO EXTREMES AT ONCE: no patience and an answer in full on the very ask
/// that used it up. A full answer is taken, never thrown away for its timing.
#[test]
fn an_answer_in_full_is_taken_even_past_the_patience() {
    let waited = paged::until_in_full(
        std::time::Duration::ZERO,
        A_MOMENT,
        || std::thread::sleep(A_MOMENT),
        |()| true,
    );
    assert!(waited.in_full);
}

/// The stop a test hands [`paged::or_stop`] in place of the bench's `refuse`,
/// which ends the process.
fn stopped(why: &str) -> ! {
    panic!("stopped: {why}");
}

/// A set-up that worked is its value; the stop is never called.
#[test]
fn a_set_up_that_worked_hands_back_its_value() {
    assert_eq!(paged::or_stop(Ok(7_u8), stopped), 7);
    paged::or_stop(Ok(()), stopped);
}

/// A set-up that failed reaches the stop with its own sentence.
#[test]
#[should_panic(expected = "stopped: the census at /x would not write")]
fn a_set_up_that_failed_stops_on_its_sentence() {
    let _never: u8 = paged::or_stop(Err("the census at /x would not write".to_owned()), stopped);
}

/// A scratch directory of this test's own, empty, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("brutex-bench-census-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A census entry for NIFTY's one-second month `number` of 2024, of `rows`.
fn entry(number: u8, rows: u64) -> Entry {
    let start_s = JUNE_START_S + i64::from(number - 6) * 31 * 86_400;
    Entry {
        key: EntryKey {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new("NIFTY").expect("a symbol"),
            timeframe: Timeframe::SECOND_1,
            month: YearMonth::new(2024, number).expect("a real month"),
        },
        rows,
        first_ts_micros: start_s * 1_000_000,
        last_ts_micros: (start_s + 86_400) * 1_000_000,
    }
}

/// The census written under `root`, as a reader opens it.
fn census_at(root: &Path) -> Manifest {
    let path = pull::manifest::manifest_path(root, Vendor::Gdfl);
    let image = std::fs::read(path).expect("the census is there");
    Manifest::open_image(Vendor::Gdfl, &image).expect("it reads back")
}

/// A census of nothing, of one month, and of one more added to an image
/// already held each read back as exactly those months.
#[test]
fn a_census_written_reads_back_as_the_months_it_holds() {
    let scratch = Scratch::new("reads-back");

    let none = scratch.0.join("none");
    paged::write_census(&none, &[], []).expect("an empty census");
    assert_eq!(census_at(&none).held_keys().count(), 0);

    let one = scratch.0.join("one");
    paged::write_census(&one, &[], [entry(6, 10)]).expect("one month");
    let read = census_at(&one);
    assert_eq!(read.held_keys().count(), 1);
    assert_eq!(read.held(&entry(6, 10).key).expect("June").entry.rows, 10);

    let image = read.image();
    let two = scratch.0.join("two");
    paged::write_census(&two, &image, [entry(7, 20)]).expect("one more");
    let read = census_at(&two);
    assert_eq!(read.held_keys().count(), 2);
    assert_eq!(read.held(&entry(6, 10).key).expect("June").entry.rows, 10);
    assert_eq!(read.held(&entry(7, 20).key).expect("July").entry.rows, 20);
}

/// OUT OF THE BOX: a root whose name holds a space and a wide character is a
/// root like any other, and a census written twice over one root is the
/// second one, not a mix of the two.
#[test]
fn a_census_under_a_strange_root_written_twice_is_its_second_writing() {
    let scratch = Scratch::new("strange");
    let root = scratch.0.join("a réal root");
    paged::write_census(&root, &[], [entry(6, 10), entry(7, 20)]).expect("the first");
    paged::write_census(&root, &[], [entry(8, 30)]).expect("the second");
    let read = census_at(&root);
    assert_eq!(read.held_keys().count(), 1);
    assert_eq!(read.held(&entry(8, 30).key).expect("August").entry.rows, 30);
}

/// Each step's refusal names its step, from the input that really reaches
/// it, and leaves no census behind where it stopped before the write.
#[test]
fn each_census_refusal_names_its_step() {
    let scratch = Scratch::new("refusals");

    // THE DIRECTORY: the root is a file, so no directory can go under it.
    let a_file = scratch.0.join("a-file");
    std::fs::write(&a_file, b"not a directory").expect("a file");
    let why = paged::write_census(&a_file, &[], []).expect_err("a file for a root");
    assert!(why.contains("the census directory under"), "{why}");

    // THE START: bytes that are no census.
    let junk = scratch.0.join("junk");
    let why = paged::write_census(&junk, b"junk", []).expect_err("junk to start from");
    assert!(why.contains("the census to start from"), "{why}");
    assert!(!pull::manifest::manifest_path(&junk, Vendor::Gdfl).exists());

    // AN ENTRY: the census holds no empty month.
    let empty = scratch.0.join("empty");
    let why = paged::write_census(&empty, &[], [entry(6, 0)]).expect_err("an empty month");
    assert!(why.contains("a census entry"), "{why}");
    assert!(!pull::manifest::manifest_path(&empty, Vendor::Gdfl).exists());

    // THE WRITE: a directory stands where the census file goes.
    let blocked = scratch.0.join("blocked");
    std::fs::create_dir_all(pull::manifest::manifest_path(&blocked, Vendor::Gdfl))
        .expect("a directory in the way");
    let why = paged::write_census(&blocked, &[], []).expect_err("a directory in the way");
    assert!(why.contains("would not write"), "{why}");
}

/// TWO EXTREMES AT ONCE: every step would refuse, and the first one speaks.
/// A good entry after an empty one does not rescue it either.
#[test]
fn a_census_wrong_in_every_way_is_refused_at_its_first_step() {
    let scratch = Scratch::new("every-way");
    let a_file = scratch.0.join("a-file");
    std::fs::write(&a_file, b"not a directory").expect("a file");
    let why = paged::write_census(&a_file, b"junk", [entry(6, 0)]).expect_err("every way");
    assert!(why.contains("the census directory under"), "{why}");

    let root = scratch.0.join("empty-first");
    let why =
        paged::write_census(&root, &[], [entry(6, 0), entry(7, 20)]).expect_err("an empty first");
    assert!(why.contains("a census entry"), "{why}");
    assert!(!pull::manifest::manifest_path(&root, Vendor::Gdfl).exists());
}

/// A page body the reader is handed, `total` rows held, `stamps` on it.
fn body(total: usize, stamps: &[i64], missing: usize, faulted: usize) -> String {
    let rows: Vec<String> = stamps
        .iter()
        .map(|t| format!("{{\"t\":{t},\"o\":1234500}}"))
        .collect();
    format!(
        "{{\"total\":{total},\"months_missing\":{missing},\"faulted_records\":{faulted},\"bars\":[{}]}}",
        rows.join(",")
    )
}

/// The `n` stamps from `from` on, one second apart.
fn run(from: i64, n: usize) -> Vec<i64> {
    (0..n).map(|i| from + i64::try_from(i).unwrap()).collect()
}

/// A `200` page of `total` rows held, carrying `stamps`.
fn ok(total: usize, stamps: &[i64]) -> Page {
    Page {
        status: 200,
        body: body(total, stamps, 0, 0),
    }
}

/// A page answering `status` with `body`.
fn answer(status: u16, body: &str) -> Page {
    Page {
        status,
        body: body.to_owned(),
    }
}

/// OUT OF THE BOX: a server that answers wrong is caught, never timed as a
/// read. Each fake below answers page by page; what the reader must say is
/// named beside it.
#[test]
fn a_server_answering_wrong_is_refused_by_name() {
    let limit = api::bars::MAX_WINDOW_LIMIT;
    let not_a_number =
        "{\"total\":1,\"months_missing\":0,\"faulted_records\":0,\"bars\":[{\"t\":x}]}";
    let cases: Vec<(&str, Vec<Page>, &str)> = vec![
        (
            "a short page mid-month",
            vec![ok(2_000, &run(0, 10))],
            "short page",
        ),
        (
            "a page repeating the last row of the one before",
            vec![ok(2_000, &run(0, limit)), ok(2_000, &run(999, limit))],
            "repeated or out of order",
        ),
        (
            "rows appended while the month is paged",
            vec![ok(2_000, &run(0, limit)), ok(2_001, &run(1_000, limit))],
            "changed under the read",
        ),
        (
            "more rows than the page asked",
            vec![ok(5_000, &run(0, limit + 1))],
            "more rows than",
        ),
        (
            "more rows than the month holds",
            vec![ok(5, &run(0, 6))],
            "past the 5 rows",
        ),
        (
            "a refusal",
            vec![answer(400, "{\"error\":\"no\"}")],
            "answered 400",
        ),
        (
            "an answer with no total",
            vec![answer(200, "{\"bars\":[]}")],
            "\"total\":",
        ),
        (
            "a total that is not a number",
            vec![answer(200, "{\"total\":-1,\"bars\":[]}")],
            "not a whole number",
        ),
        (
            "a missing month",
            vec![answer(200, &body(0, &[], 1, 0))],
            "months_missing",
        ),
        (
            "a faulted record",
            vec![answer(200, &body(1, &[7], 0, 1))],
            "faulted_records",
        ),
        (
            "a stamp that is not a number",
            vec![answer(200, not_a_number)],
            "a stamp",
        ),
    ];
    for (what, pages, says) in cases {
        let mut pages = pages.into_iter();
        let mut asked = Vec::new();
        let refused = read_month(&june(), |path| {
            asked.push(path.to_owned());
            pages
                .next()
                .expect("the reader asked past the fake's pages")
        });
        let why = refused.expect_err(what);
        assert!(why.contains(says), "{what}: {why}");
        assert!(
            asked[0].starts_with("/bars/window.json?")
                && asked[0].ends_with("&dir=asc&limit=1000&offset=0"),
            "{what}: {asked:?}"
        );
        if let Some(second) = asked.get(1) {
            assert!(second.ends_with("&offset=1000"), "{what}: {asked:?}");
        }
    }
}

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
    month: u8,
    start_s: i64,
    rows: usize,
}

/// June 2024 of `rows` one-second bars from its first instant.
fn june_of(rows: usize) -> Month {
    Month {
        year: 2024,
        month: 6,
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
    let ym = YearMonth::new(month.year, month.month).expect("a real month");
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
    let root = std::env::temp_dir().join(format!(
        "brutex-bench-paged-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("manifest")).expect("a scratch root");
    let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    for month in months {
        let entry = write_month(&root, month);
        // THE CENSUS HOLDS NO EMPTY MONTH (`EmptyEntry`): an empty month is
        // its file alone, as a month created and never filled is.
        if month.rows > 0 {
            manifest.record(entry).expect("a census entry");
        }
    }
    std::fs::write(
        pull::manifest::manifest_path(&root, Vendor::Gdfl),
        manifest.image(),
    )
    .expect("the census writes");
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
            axum::serve(listener, router).await.expect("the server runs");
        });
    });
    Served { root, addr }
}

impl Served {
    /// One GET on a fresh connection, read to its close; a full pool (`429`)
    /// or a route still building is asked again, at most two seconds.
    fn get(&self, path: &str) -> Page {
        for _ in 0..400 {
            let page = self.once(path);
            if page.status != 429 && page.status != 503 {
                return page;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("{path} stayed busy for two seconds");
    }

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
        let head = String::from_utf8_lossy(&raw[..split]).to_ascii_lowercase();
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("a status");
        let mut body = raw[split + 4..].to_vec();
        if head.contains("transfer-encoding: chunked") {
            body = dechunk(&body);
        }
        Page {
            status,
            body: String::from_utf8(body).expect("UTF-8"),
        }
    }

    /// The whole month as the bench reads it.
    fn read(&self, query: &str) -> Result<MonthRead, String> {
        read_month(query, |path| self.get(path))
    }
}

/// A chunked body, joined.
fn dechunk(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    loop {
        let end = at + raw[at..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .expect("a chunk size line");
        let text = std::str::from_utf8(&raw[at..end]).expect("ASCII");
        let size = usize::from_str_radix(text.split(';').next().unwrap().trim(), 16)
            .expect("a hex size");
        at = end + 2;
        if size == 0 {
            return out;
        }
        out.extend_from_slice(&raw[at..at + size]);
        at += size + 2;
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
            && refused.body.contains(&paged::cap_sentence(MONTH_ANSWER_CAP_ROWS)),
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
                    month: 7,
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
                month: 8,
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

/// The paging keys belong to the reader: a query that brings its own is
/// refused before a request is made, by the key's name.
#[test]
fn a_query_carrying_its_own_paging_keys_is_refused_before_asking() {
    for key in ["dir", "limit", "offset", "sort"] {
        let mut asked = 0;
        let refused = read_month(&format!("{}&{key}=1", june()), |_| {
            asked += 1;
            Page {
                status: 200,
                body: String::new(),
            }
        });
        let why = refused.expect_err("its own paging key");
        assert!(why.contains(&format!("{key}=")), "{why}");
        assert_eq!(asked, 0, "{key}: nothing is asked");
    }
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

/// OUT OF THE BOX: a server that answers wrong is caught, never timed as a
/// read. Each fake below answers page by page; what the reader must say is
/// named beside it.
#[test]
fn a_server_answering_wrong_is_refused_by_name() {
    let limit = api::bars::MAX_WINDOW_LIMIT;
    let cases: Vec<(&str, Vec<Page>, &str)> = vec![
        (
            "a short page mid-month",
            vec![Page {
                status: 200,
                body: body(2_000, &run(0, 10), 0, 0),
            }],
            "short page",
        ),
        (
            "a page repeating the last row of the one before",
            vec![
                Page {
                    status: 200,
                    body: body(2_000, &run(0, limit), 0, 0),
                },
                Page {
                    status: 200,
                    body: body(2_000, &run(999, limit), 0, 0),
                },
            ],
            "repeated or out of order",
        ),
        (
            "rows appended while the month is paged",
            vec![
                Page {
                    status: 200,
                    body: body(2_000, &run(0, limit), 0, 0),
                },
                Page {
                    status: 200,
                    body: body(2_001, &run(1_000, limit), 0, 0),
                },
            ],
            "changed under the read",
        ),
        (
            "more rows than the page asked",
            vec![Page {
                status: 200,
                body: body(5_000, &run(0, limit + 1), 0, 0),
            }],
            "more rows than",
        ),
        (
            "more rows than the month holds",
            vec![Page {
                status: 200,
                body: body(5, &run(0, 6), 0, 0),
            }],
            "past the 5 rows",
        ),
        (
            "a refusal",
            vec![Page {
                status: 400,
                body: "{\"error\":\"no\"}".to_owned(),
            }],
            "answered 400",
        ),
        (
            "an answer with no total",
            vec![Page {
                status: 200,
                body: "{\"bars\":[]}".to_owned(),
            }],
            "\"total\":",
        ),
        (
            "a missing month",
            vec![Page {
                status: 200,
                body: body(0, &[], 1, 0),
            }],
            "months_missing",
        ),
        (
            "a faulted record",
            vec![Page {
                status: 200,
                body: body(1, &[7], 0, 1),
            }],
            "faulted_records",
        ),
        (
            "a stamp that is not a number",
            vec![Page {
                status: 200,
                body: "{\"total\":1,\"months_missing\":0,\"faulted_records\":0,\"bars\":[{\"t\":x}]}"
                    .to_owned(),
            }],
            "a stamp",
        ),
    ];
    for (what, pages, says) in cases {
        let mut pages = pages.into_iter();
        let mut asked = Vec::new();
        let refused = read_month(&june(), |path| {
            asked.push(path.to_owned());
            pages.next().expect("the reader asked past the fake's pages")
        });
        let why = refused.expect_err(what);
        assert!(why.contains(says), "{what}: {why}");
        assert!(
            asked[0].starts_with("/bars/window.json?") && asked[0].contains("&offset=0"),
            "{what}: {asked:?}"
        );
    }
}

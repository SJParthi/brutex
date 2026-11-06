//! `/greeks.json`'s refusals and faults, over generated records and an
//! invented rate table (BASE-COV-API-AS3). No row here is market data and no
//! rate here is RBI's. Declared in `optiongreeks.rs`; the route's own suite is
//! `optiongreeks_route_tests.rs`.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use brutex_core::vendor::Vendor;
use pull::session::Day;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, StorePath};

/// `hh:mm` IST on `y-m-d`, in UTC seconds.
fn ist(y: u16, m: u8, d: u8, hh: i64, mm: i64) -> i64 {
    i64::from(
        Day::new(y, m, d)
            .expect("a generated day")
            .days_from_epoch(),
    ) * 86_400
        - 19_800
        + hh * 3_600
        + mm * 60
}

/// 09:15 IST on 2026-08-20, the instant the bars below open at.
fn t0() -> i64 {
    ist(2026, 8, 20, 9, 15)
}

/// An invented weekly table, one price/yield pair the parser's own check
/// accepts, released on each `day` of August 2026.
fn table_text(days: &[u8]) -> String {
    let mut text = String::from(pull::riskfree::HEADER);
    for day in days {
        let _ = write!(
            text,
            "\n\t\t2026-08-{day:02}\t98.5000\t6.1081\thttps://www.rbi.org.in/invented\t\t"
        );
    }
    text
}

fn rate() -> RateTable {
    RateTable::Loaded(Series::parse(&table_text(&[12, 19])).expect("the invented table"))
}

/// The store path of `symbol`'s `segment` file (an option's when `contract`
/// is named) at one minute in `month`, on Dhan.
fn path_of(segment: &'static str, contract: Option<&str>, month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment,
        symbol: "NIFTY",
        contract: contract.map(|c| Contract::parse(c).expect("a generated contract")),
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a generated path")
}

fn id() -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the same low 32 bits `api::bars::open_classified` checks"
    )]
    let id = brutex_core::universe::fnv1a("NIFTY") as u32;
    id
}

/// Bars `(UTC second, close)` at `path`.
fn write(root: &Path, path: StorePath, bars: &[(i64, i64)]) {
    let mut file = BarFile::open_or_create(root, path, id()).expect("a generated writer");
    let rows: Vec<Bar> = bars
        .iter()
        .map(|&(secs, close)| Bar {
            ts_micros: secs * 1_000_000,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect();
    file.append(&rows).expect("generated records");
}

const ATM: &str = "2026-08-27-2500000-CE";

fn august() -> YearMonth {
    YearMonth::new(2026, 8).expect("a month")
}

/// A scratch store root, removed when dropped.
struct Root(PathBuf);

impl Root {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        Self(root)
    }

    /// The index at `t0` and `t0 + 60`, and the option `contract` at the
    /// instants `bars` names.
    fn with_option(name: &str, contract: &str, bars: &[(i64, i64)]) -> Self {
        let root = Self::new(name);
        let t = t0();
        write(
            &root.0,
            path_of("INDEX", None, august()),
            &[(t, 2_500_000), (t + 60, 2_501_000)],
        );
        write(&root.0, path_of("FNO", Some(contract), august()), bars);
        root
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn query(contract: &str, month: &str, times: &[i64]) -> String {
    let t: Vec<String> = times.iter().map(i64::to_string).collect();
    format!(
        "feed=dhan&exchange=NSE&segment=FNO&symbol=NIFTY&contract={contract}&timeframe=1min&month={month}&t={}",
        t.join(",")
    )
}

fn json(answered: Answered) -> (axum::http::StatusCode, serde_json::Value) {
    let (status, _, body) = answered;
    (status, serde_json::from_str(&body).expect("greeks JSON"))
}

/// **A CONTRACT DATE OF THE WRONG WIDTHS IS NOT AN OPTION SEGMENT**
/// (BASE-COV-API-AS3). The store's grammar is `YYYY-MM-DD-<paisa>-CE|PE`, so
/// each field's width is checked before any number is read. Extremes: a
/// one-digit month; a one-digit day; a two-digit year; a three-digit month;
/// and, at once, a one-digit month and a one-digit day, and a five-digit year
/// with a one-digit day; out of the box, the right widths for a day the
/// month does not have (refused by the date, not the widths). The control: the
/// right widths of a real day parse.
#[test]
fn a_contract_date_of_the_wrong_widths_is_not_an_option_segment() {
    for word in [
        "2026-8-27-2500000-CE",
        "2026-08-7-2500000-PE",
        "26-08-27-2500000-CE",
        "2026-008-27-2500000-CE",
        "2026-8-7-2500000-CE",
        "20260-08-7-2500000-PE",
        "2026-09-31-2500000-CE",
    ] {
        assert!(option_parts(word).is_none(), "{word}");
    }
    assert!(option_parts(ATM).is_some(), "the control parses");
}

/// **EXCHANGE, SEGMENT AND SYMBOL ARE EACH REQUIRED, BY NAME**
/// (BASE-COV-API-AS3). Each one empty is the same refusal, before the
/// contract is read. Extremes: the exchange empty; the segment empty; the
/// symbol empty; and, at once, all three empty, and the exchange and symbol
/// empty with the segment named; out of the box, none of the three keys
/// given at all.
#[test]
fn exchange_segment_and_symbol_are_each_required_by_name() {
    let tail = format!("contract={ATM}&timeframe=1min&month=2026-08&t={}", t0());
    for head in [
        "feed=dhan&exchange=&segment=FNO&symbol=NIFTY",
        "feed=dhan&exchange=NSE&segment=&symbol=NIFTY",
        "feed=dhan&exchange=NSE&segment=FNO&symbol=",
        "feed=dhan&exchange=&segment=&symbol=",
        "feed=dhan&exchange=&segment=FNO&symbol=",
        "feed=dhan",
    ] {
        let why = GreeksAsk::parse(&format!("{head}&{tail}")).expect_err(head);
        assert!(
            why.contains("exchange, segment and symbol"),
            "{head}: {why}"
        );
    }
}

/// **THE PLAIN ANSWER IS THE CANCELLABLE ONE, NEVER CANCELLED**
/// (BASE-COV-API-AS3). `GreeksAsk::answer` is `answer_with` under
/// `Cancel::never()`, cell for cell and fault for fault. Extremes: one priced
/// instant; an instant outside the asked month; an instant with no option
/// bar; and, at once, all three in one request, and the same instant asked
/// twice; out of the box, the last second before the month opens.
///
#[test]
fn the_plain_answer_is_the_cancellable_one_never_cancelled() {
    let t = t0();
    let root = Root::with_option("greeks-cover-plain", ATM, &[(t, 25_000)]);
    let before = ist(2026, 8, 1, 0, 0) - 1;
    for times in [
        vec![t],
        vec![ist(2026, 9, 1, 10, 0)],
        vec![t + 60],
        vec![t, ist(2026, 9, 1, 10, 0), t + 60],
        vec![t, t],
        vec![before],
    ] {
        let asked = GreeksAsk::parse(&query(ATM, "2026-08", &times)).expect("a question");
        let table = rate();
        let plain = asked.answer(&root.0, &table).expect("an answer");
        let never = asked
            .answer_with(&root.0, &table, &crate::bars::Cancel::never())
            .expect("an answer");
        assert_eq!(plain, never, "{times:?}");
        assert_eq!(plain.0.len(), times.len(), "one cell an instant: {times:?}");
    }
}

/// **AN OPTION MONTH THAT IS NOT A BAR FILE IS A NAMED FAULT AND A 206**
/// (BASE-COV-API-AS3). The open refuses it as unreadable: every instant of
/// the month is `option_unreadable` with the reader's own sentence, the
/// fault is carried in `faults`, and the answer is the partial 206, never a
/// 200 that reads as whole. Extremes: a file of one byte; an empty file; a
/// file of text the length of a header; and, at once, a damaged option month
/// with an instant outside the month beside it, and with two instants in it;
/// out of the box, a directory where the option's month file goes.
#[test]
fn an_option_month_that_is_not_a_bar_file_is_a_named_fault_and_a_206() {
    let t = t0();
    let header_len = usize::try_from(store::format::HEADER_LEN).expect("a header length");
    let damage: [&dyn Fn(&Path); 4] = [
        &|at: &Path| std::fs::write(at, b"x").expect("one byte"),
        &|at: &Path| std::fs::write(at, b"").expect("an empty file"),
        &|at: &Path| std::fs::write(at, "n".repeat(header_len)).expect("a header of text"),
        &|at: &Path| std::fs::create_dir_all(at).expect("a directory"),
    ];
    for (case, harm) in damage.iter().enumerate() {
        let root = Root::new(&format!("greeks-cover-damaged-{case}"));
        write(
            &root.0,
            path_of("INDEX", None, august()),
            &[(t, 2_500_000), (t + 60, 2_501_000)],
        );
        let option = path_of("FNO", Some(ATM), august()).to_path_buf(&root.0);
        std::fs::create_dir_all(option.parent().expect("a parent")).expect("its folder");
        harm(&option);
        for times in [vec![t], vec![t, ist(2026, 9, 1, 10, 0)], vec![t, t + 60]] {
            let (status, body) = json(respond(&root.0, &query(ATM, "2026-08", &times), &rate()));
            assert_eq!(
                status,
                axum::http::StatusCode::PARTIAL_CONTENT,
                "{case}: {body}"
            );
            let faults = body["faults"].as_str().expect("the fault, named");
            assert!(!faults.is_empty(), "{case}: {body}");
            let first = &body["rows"][0];
            assert_eq!(first["why"], "option_unreadable", "{case}: {body}");
            assert!(first["detail"].is_string(), "the reader's sentence: {body}");
        }
    }
}

/// **AN OPTION MONTH A WRITER HOLDS IS BEING WRITTEN, NEVER UNREADABLE**
/// (BASE-COV-API-AS3, R3-08's rule for the option's own file). With the
/// option's month held by a writer past the read's patience, the cell names
/// `option_being_written`, and the whole answer is the being-written 503 the
/// page asks again on. Extremes: one instant; the same instant twice; and, at
/// once, a held option month with an instant outside the month; out of the
/// box, the writer gone, after which the same question is priced.
#[test]
fn an_option_month_a_writer_holds_is_being_written_never_unreadable() {
    let t = t0();
    let root = Root::with_option("greeks-cover-held", ATM, &[(t, 25_000)]);
    let held = path_of("FNO", Some(ATM), august());
    let writer_root = root.0.clone();
    let (holding_tx, holding_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let writer = std::thread::Builder::new()
        .name("as3-greeks-writer".to_owned())
        .spawn(move || {
            let file = BarFile::open_or_create(&writer_root, held, id()).expect("the writer");
            holding_tx.send(()).expect("the test waits for the writer");
            let _ = release_rx.recv();
            drop(file);
        })
        .expect("a writer thread");
    holding_rx.recv().expect("the writer holds the month");
    for times in [vec![t], vec![t, t], vec![t, ist(2026, 9, 1, 10, 0)]] {
        let (status, body) = json(store::flock::with_patience(
            std::time::Duration::from_millis(crate::detail::READ_PATIENCE_MS),
            || respond(&root.0, &query(ATM, "2026-08", &times), &rate()),
        ));
        assert_eq!(
            status,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "{body}"
        );
        assert_eq!(body["being_written"], true, "{body}");
        assert!(!body.to_string().contains("unreadable"), "{body}");
    }
    release_tx.send(()).expect("the writer lets go");
    writer.join().expect("the writer thread");
    let (status, body) = json(respond(&root.0, &query(ATM, "2026-08", &[t]), &rate()));
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert!(
        body["rows"][0]["iv"].is_number(),
        "priced once the writer left: {body}"
    );
}

/// Flips one byte inside record `index` of the file at `path`, so its
/// checksum block no longer verifies (the store's own lazy check).
fn corrupt_record(path: &Path, index: u64) {
    use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
    let at = store::format::HEADER_LEN + index * store::format::RECORD_STRIDE + 9;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("the bar file exists");
    let mut byte = [0_u8; 1];
    file.seek(SeekFrom::Start(at)).expect("seek");
    file.read_exact(&mut byte).expect("read");
    byte[0] ^= 0x5A;
    file.seek(SeekFrom::Start(at)).expect("seek");
    file.write_all(&byte).expect("write");
}

/// **A RECORD THAT WILL NOT READ IS A FAULT ON ITS OWN CELL**
/// (BASE-COV-API-AS3). The option's month opens (the open checks the header
/// and no block), and the first read of a damaged block refuses: the cell is
/// `option_unreadable`, the fault names the file and the instant, and the
/// answer is the partial 206. Extremes: the first record damaged; the last
/// record damaged; a middle one; and, at once, a damaged block asked at two
/// instants, and asked beside an instant outside the month; out of the box,
/// every record of the block damaged.
#[test]
fn a_record_that_will_not_read_is_a_fault_on_its_own_cell() {
    let t = t0();
    let bars = [(t, 25_000), (t + 60, 25_100), (t + 120, 25_200)];
    let outside = ist(2026, 9, 1, 10, 0);
    for (case, (damaged, times)) in [
        (vec![0], vec![t]),
        (vec![2], vec![t]),
        (vec![1], vec![t]),
        (vec![1], vec![t, t + 120]),
        (vec![1], vec![t, outside]),
        (vec![0, 1, 2], vec![t]),
    ]
    .into_iter()
    .enumerate()
    {
        let root = Root::with_option(&format!("greeks-cover-record-{case}"), ATM, &bars);
        let option = path_of("FNO", Some(ATM), august()).to_path_buf(&root.0);
        for index in damaged {
            corrupt_record(&option, index);
        }
        let (status, body) = json(respond(&root.0, &query(ATM, "2026-08", &times), &rate()));
        assert_eq!(
            status,
            axum::http::StatusCode::PARTIAL_CONTENT,
            "{case}: {body}"
        );
        assert_eq!(
            body["rows"][0]["why"], "option_unreadable",
            "{case}: {body}"
        );
        let faults = body["faults"].as_str().expect("the faults");
        assert!(
            faults.contains(&format!("t={t}")),
            "the instant is named: {faults}"
        );
    }
}

/// **A STAMP THE RATE CALENDAR DOES NOT HOLD IS A TENOR REFUSAL, NOT A
/// RATE** (BASE-COV-API-AS3). A bar stamped before 1990, the rate calendar's
/// first year (`costs::day::FIRST_YEAR`), for a contract that settles inside
/// the session table: its tenor is measured, and the rate lookup refuses the
/// stamp's day by name. Extremes: the last minute of 1989; its first day;
/// and, at once, a 1989 stamp beside a priced one from another month in one
/// request (refused as outside the month); out of the box, a stamp in
/// January 1970, the first month the store files.
#[test]
fn a_stamp_the_rate_calendar_does_not_hold_is_a_tenor_refusal() {
    const LONG: &str = "2018-01-25-2500000-CE";
    for (case, (month, at)) in [
        (YearMonth::new(1989, 12), ist(1989, 12, 29, 15, 29)),
        (YearMonth::new(1989, 1), ist(1989, 1, 2, 9, 15)),
        (YearMonth::new(1970, 1), ist(1970, 1, 2, 0, 0)),
    ]
    .into_iter()
    .enumerate()
    {
        let month = month.expect("a month");
        let root = Root::new(&format!("greeks-cover-pre-1990-{case}"));
        write(&root.0, path_of("INDEX", None, month), &[(at, 2_500_000)]);
        write(&root.0, path_of("FNO", Some(LONG), month), &[(at, 25_000)]);
        let word = month.to_string();
        let (status, body) = json(respond(&root.0, &query(LONG, &word, &[at, t0()]), &rate()));
        assert_eq!(status, axum::http::StatusCode::OK, "{case}: {body}");
        let row = &body["rows"][0];
        assert_eq!(row["why"], "tenor_refused", "{case}: {body}");
        let detail = row["detail"].as_str().expect("the sentence");
        assert!(
            detail.contains("is not a trade day the rate calendar holds"),
            "{case}: {detail}"
        );
        assert!(row["tenor_s"].is_number(), "the tenor was measured: {body}");
        assert_eq!(body["rows"][1]["why"], "t_outside_month", "{case}: {body}");
    }
}

/// **EVERY EXPIRY THE INSTRUMENT MODEL ADMITS IS SETTLED** (BASE-COV-API-AS3).
/// The proof beside `render`'s `unsettled` arm, which no question reaches:
/// `settled_of` asks `pull::expiry_alias::settle` with no trade day, whose only
/// refusal is a closed stated day walked back to an unmeasured one, and the
/// sourced calendar opens on a trading day (2018-01-01). Walked over every day
/// `Expiry` admits (1990..=2100) for an underlying with alias rows and one
/// without. Extremes: the first admitted day; the last; the sourced
/// calendar's first day and the day before it; a weekend inside it; and, at
/// once, every day for two underlyings; out of the box, the leap day of 2000,
/// a century leap year.
#[test]
fn every_expiry_the_instrument_model_admits_is_settled() {
    let mut day = Day::new(1990, 1, 1).expect("the first admitted day");
    let last = Day::new(2100, 12, 31).expect("the last admitted day");
    let mut walked = 0_u32;
    loop {
        let expiry = Expiry::new(day.year(), day.month(), day.day()).expect("an admitted day");
        for symbol in ["NIFTY", "MIDCPNIFTY", "RELIANCE"] {
            assert!(settled_of(symbol, expiry).is_ok(), "{symbol} {day}");
        }
        walked += 1;
        if day == last {
            break;
        }
        day = day.succ().expect("a next day before the last");
    }
    assert_eq!(
        walked, 40_542,
        "every day of 1990..=2100: 111 years, 27 of them leap"
    );
    for (y, m, d) in [(2018, 1, 1), (2017, 12, 31), (2018, 1, 6), (2000, 2, 29)] {
        let expiry = Expiry::new(y, m, d).expect("an admitted day");
        assert!(settled_of("NIFTY", expiry).is_ok(), "{expiry}");
    }
}

/// The child of [`a_greeks_read_the_store_pool_cannot_admit_is_a_429`].
const POOL_CHILD: &str = "BRUTEX_TEST_GREEKS_POOL_CHILD";

/// **A GREEKS READ THE STORE-READ POOL CANNOT ADMIT IS A 429 THAT NAMES IT**
/// (BASE-COV-API-AS3). In a child of this binary, so the crowd starves no
/// other test: every store-read slot counted taken, the handler's own
/// admission refuses before any file is opened, with `Retry-After: 1` and
/// the read's name. Extremes: one instant; the most instants a question may
/// carry; and, at once, a crowded pool and a question that would itself be
/// refused (malformed), answered at the pool first; out of the box, no store
/// root files at all.
#[test]
fn a_greeks_read_the_store_pool_cannot_admit_is_a_429() {
    const NAME: &str =
        "server::optiongreeks::cover_tests::a_greeks_read_the_store_pool_cannot_admit_is_a_429";
    if let Some(root) = std::env::var_os(POOL_CHILD) {
        let root = PathBuf::from(root);
        let site: Loaded = Arc::new(super::super::Site::load(&root, &root));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        let _crowd = crate::detail::crowd(crate::detail::Named::StoreRead);
        let many: Vec<i64> = (0..MAX_GREEK_TIMES)
            .map(|n| t0() + 60 * i64::try_from(n).expect("a small count"))
            .collect();
        for asked in [
            query(ATM, "2026-08", &[t0()]),
            query(ATM, "2026-08", &many),
            "feed=nowhere".to_owned(),
        ] {
            let uri: axum::http::Uri = format!("/greeks.json?{asked}").parse().expect("a URI");
            let (status, headers, body) =
                runtime.block_on(greeks_json(axum::extract::State(Arc::clone(&site)), uri));
            assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS, "{body}");
            assert_eq!(headers[axum::http::header::RETRY_AFTER], "1", "{body}");
            assert!(body.contains("a greeks read"), "{body}");
        }
        std::fs::write(root.join("child-ran"), b"crowded").expect("the child's proof");
        return;
    }
    let root = Root::new("greeks-cover-pool");
    let _ = crate::isolated::rerun(NAME, &[(POOL_CHILD, root.0.as_os_str())]);
    assert_eq!(
        std::fs::read(root.0.join("child-ran")).ok().as_deref(),
        Some(&b"crowded"[..])
    );
}

/// The child of [`a_rate_table_named_by_another_path_is_read_from_it`].
const RATE_CHILD: &str = "BRUTEX_TEST_GREEKS_RATE_CHILD";

/// **A RATE TABLE NAMED BY ANOTHER PATH IS READ FROM THAT PATH**
/// (BASE-COV-API-AS3, DB-067). The kept table answers only the path and
/// stamp it was read under: a second path, even with a stamp of its own, is
/// a fresh read, and so is the first path again. In a child of this binary,
/// so the process's one kept table is this test's alone. Extremes: two paths
/// with different tables; two paths with byte-identical tables (still two
/// reads: the path is the key); and, at once, the first path asked again
/// after the second; out of the box, a path that is a directory.
#[test]
fn a_rate_table_named_by_another_path_is_read_from_it() {
    const NAME: &str =
        "server::optiongreeks::cover_tests::a_rate_table_named_by_another_path_is_read_from_it";
    if let Some(root) = std::env::var_os(RATE_CHILD) {
        let root = PathBuf::from(root);
        let (a, b, c) = (root.join("a.tsv"), root.join("b.tsv"), root.join("c.tsv"));
        std::fs::write(&a, table_text(&[12])).expect("table a");
        std::fs::write(&b, table_text(&[12, 19])).expect("table b");
        std::fs::write(&c, table_text(&[12])).expect("table c, a's bytes");
        let named = |path: &Path| Some(path.as_os_str().to_owned());
        let (first, one) = rate_loads(named(&a));
        let (second, two) = rate_loads(named(&b));
        assert!(
            !Arc::ptr_eq(&first, &second),
            "another path is another read"
        );
        assert_eq!(two, one + 1);
        assert_ne!(first.sha256_hex(), second.sha256_hex());
        let (same_bytes, three) = rate_loads(named(&c));
        assert_eq!(three, two + 1, "the path is the key, not the bytes");
        assert_eq!(same_bytes.sha256_hex(), first.sha256_hex());
        let (again, four) = rate_loads(named(&a));
        assert_eq!(four, three + 1, "the first path again is read again");
        assert_eq!(again.sha256_hex(), first.sha256_hex());
        let (folder, _) = rate_loads(named(&root));
        assert_eq!(folder.state(), "unreadable", "a directory is no table");
        std::fs::write(root.join("child-ran"), b"read").expect("the child's proof");
        return;
    }
    let root = Root::new("greeks-cover-rate-paths");
    let _ = crate::isolated::rerun(NAME, &[(RATE_CHILD, root.0.as_os_str())]);
    assert_eq!(
        std::fs::read(root.0.join("child-ran")).ok().as_deref(),
        Some(&b"read"[..])
    );
}

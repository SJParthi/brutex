//! The `/db` routes timed AS SERVED, at 1x, 10x and 100x (R3-14; RA-064).
//!
//! # Why through the server, and not through a function
//!
//! Most `/db` routes answer from a private module of `api::server` (`chain`,
//! `dayflags`, `provenance`, the calendar), so a bench can reach them only
//! the way a page does: a `GET` to the router. That is also the honest
//! figure: what is timed is the request the operator's page makes, through
//! the admission layer, the request log and the speed table every route
//! passes ([`api::server::router_serving`]), on one kept-alive loopback
//! connection, so the connection is not in the figure.
//!
//! # What grows, and what does not
//!
//! Three stores, built once each by [`stores`]. Between them two things grow
//! a hundredfold and nothing else does:
//!
//! * THE MONTH: the asked series' June 2024 holds 1,024, 10,240 and 102,400
//!   one-second bars ([`super::MONTH_SIZES`], every size indexed), laid in
//!   whole sessions from 2024-06-03 09:15 IST, 22,500 seconds a day;
//! * THE CENSUS: 1,000, 10,000 and 100,000 other series beside it, invented
//!   symbols the routes are never asked about: half cash one-minute months,
//!   half one-second option contracts of OTHER underlyings over four expiries
//!   ([`OTHER_EXPIRIES`]) and fifty strikes each (R3-14 repair 2: the census
//!   was cash alone, so a chain, facet or calendar read that walked every
//!   option contract in the store read flat). The proof is this bench,
//!   `crates/api/benches/ratio.rs`, whose `served` module this is (gate 8).
//!
//! What a page asks stays the same size at every scale: NIFTY's index month,
//! one option month, and a chain of [`STRIKES`] strikes on one expiry. A
//! route whose cost follows the month or the census breaches its ratio here.
//! The ledgers some routes merge (`gdfl.dayflags`, `zerodha.gaps`, the import
//! journal, the lineage ledger) are absent from these stores and are answered
//! as absent; each is read by a tail reader with its own bound, timed in
//! `logs_dirs` for the journal. Said here so no row reads as covering them.
//!
//! # What is printed and what is gated
//!
//! Every target is first asked until it answers in full (a route that is
//! still building says so, and is asked again; that first answer's time is
//! printed as `cold`). It must answer `200` and carry the needle its module
//! names, so a refusal or a miss is never what gets timed. Then n = 200
//! sequential requests: p50, p99 and max are printed for every size, and the
//! p50 ratio 1x -> 10x and 1x -> 100x is GATED at
//! [`super::RATIO_CEILING_PERMILLE`]: per request, or, for the two routes
//! whose answer or whose read IS the month by design (`/bars.json`'s whole
//! month, the gap audit's month), per row of the answer or per bar of the
//! month ([`Per`]); the census's whole bodies (`/store.json`,
//! `/instruments.json`) are gated as the `304` a page's revalidation is,
//! so a sort, a second pass or a walk of what was not asked still breaches. The p99 ratio is printed, not gated:
//! the 99th of 200 requests on a machine twelve agents share is two
//! requests, and a gate on it would fail on a neighbour's build.
//!
//! Invented prices: the index at 12,345.00 and options near 43.21 are no
//! print anyone could mistake for a real NIFTY one of June 2024.

use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::manifest::{Entry, EntryKey, Manifest};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::{MONTH_SIZES, RATIO_CEILING_PERMILLE, bench_month, ratio, refuse};

/// The census sizes beside the asked series: 1x / 10x / 100x.
pub(crate) const CENSUS_SIZES: [usize; 3] = [1_000, 10_000, 100_000];

/// The expiries the census's other underlyings' contracts name: the asked
/// month's own and the three after it, as a real June chain holds.
pub(crate) const OTHER_EXPIRIES: [&str; 4] =
    ["2024-06-27", "2024-07-25", "2024-08-29", "2024-09-26"];

/// Strikes on the chain's expiry, each a call and a put.
pub(crate) const STRIKES: i64 = 10;

/// The expiry every fixture option names: the month's last Thursday.
pub(crate) const EXPIRY: &str = "2024-06-27";

/// The option month the routes are asked about.
pub(crate) const CONTRACT: &str = "2024-06-27-1230000-CE";

/// 2024-06-03 09:15 IST in epoch seconds: the first fixture bar.
const FIRST_OPEN_S: i64 = 1_717_386_300;

/// Seconds in one regular NSE session, 09:15 to 15:30 IST.
const SESSION_S: i64 = 22_500;

/// Requests timed for each target at each size.
const N: usize = 200;

/// The stamp of fixture bar `i`, in epoch seconds: whole sessions from
/// 2024-06-03, one second apart, the next weekday at 09:15 after 15:30.
pub(crate) fn session_second(i: usize) -> i64 {
    let i = i64::try_from(i).unwrap_or(i64::MAX);
    FIRST_OPEN_S
        .saturating_add((i / SESSION_S).saturating_mul(86_400))
        .saturating_add(i % SESSION_S)
}

/// One served store and the connection the bench asks it on.
pub(crate) struct Served {
    /// Bars in the asked months.
    pub(crate) bars: usize,
    /// Other series in the census.
    pub(crate) census: usize,
    /// The store root, kept so it outlives the server.
    pub(crate) root: PathBuf,
    addr: SocketAddr,
    conn: TcpStream,
}

/// One answer: status, the headers lower-cased, the body.
pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) head: String,
    pub(crate) body: String,
}

/// A symbol, or the bench stops.
fn symbol(text: &str) -> Symbol {
    match Symbol::new(text) {
        Ok(symbol) => symbol,
        Err(_) => refuse(&format!("the fixture symbol {text}")),
    }
}

/// A contract, or the bench stops.
fn contract(text: &str) -> Contract {
    match Contract::parse(text) {
        Some(contract) => contract,
        None => refuse(&format!("the fixture contract {text}")),
    }
}

/// Appends `n` one-second bars of `segment` NIFTY (`contract` below it) to
/// June 2024 under `root`, closing at `close` paisa, and refuses a month the
/// store did not index (D-4933), as `month_file` does.
fn month(root: &Path, segment: &str, below: Option<Contract>, n: usize, close: i64) {
    let Ok(path) = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol: "NIFTY",
        contract: below,
        timeframe: Timeframe::SECOND_1,
        month: bench_month(),
        file: FileKind::Bars,
    }) else {
        refuse("the served bench path is a legal one")
    };
    let Ok(id) = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)) else {
        refuse("the low 32 bits of a hash fit 32 bits")
    };
    let mut file = match store::file::BarFile::open_or_create(root, path, id) {
        Ok(file) => file,
        Err(why) => refuse(&format!("the served bench month would not open: {why}")),
    };
    let bars: Vec<Bar> = (0..n)
        .map(|i| {
            let wiggle = i64::try_from(i % 97).unwrap_or(0);
            Bar {
                ts_micros: session_second(i).saturating_mul(1_000_000),
                open: close + wiggle,
                high: close + wiggle + 5,
                low: close + wiggle - 5,
                close: close + wiggle,
                volume: 1 + wiggle,
                open_interest: if below.is_some() {
                    1_000 + wiggle
                } else {
                    OI_NULL
                },
            }
        })
        .collect();
    if let Err(why) = file.append(&bars) {
        refuse(&format!("the served bench month would not fill: {why}"));
    }
    if n >= MONTH_SIZES[0] && file.time_lookup() != store::file::TimeLookup::Indexed {
        refuse(&format!("a {n}-bar served month with its .tix"));
    }
}

/// The census entry of one month of `n` rows.
fn entry(key: EntryKey, n: usize) -> Entry {
    let last = n.saturating_sub(1);
    Entry {
        key,
        rows: u64::try_from(n).unwrap_or(u64::MAX),
        first_ts_micros: session_second(0).saturating_mul(1_000_000),
        last_ts_micros: session_second(last).saturating_mul(1_000_000),
    }
}

/// One NIFTY key at one second.
fn nifty(segment: Segment, below: Option<Contract>) -> EntryKey {
    EntryKey {
        contract: below,
        exchange: Exchange::Nse,
        segment,
        symbol: symbol("NIFTY"),
        timeframe: Timeframe::SECOND_1,
        month: bench_month(),
    }
}

/// The store of one size: NIFTY's index month and option months of `bars`
/// bars each (the chain's other strikes 1,024), and `census` other series in
/// the census beside them.
fn store_of(bars: usize, census: usize) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-served-{bars}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(root.join("manifest")) {
        refuse(&format!("the served bench root would not create: {why}"));
    }
    let Ok(mut manifest) = Manifest::open(Vendor::Gdfl, &[], &[]) else {
        refuse("a genesis census");
    };
    let mut record = |entry: Entry| {
        if manifest.record(entry).is_err() {
            refuse("a served census entry");
        }
    };
    month(&root, "INDEX", None, bars, 1_234_500);
    record(entry(nifty(Segment::Index, None), bars));
    for k in 0..STRIKES {
        for side in ["CE", "PE"] {
            let text = format!("{EXPIRY}-{}-{side}", 1_200_000 + k * 10_000);
            let below = contract(&text);
            let n = if text == CONTRACT {
                bars
            } else {
                MONTH_SIZES[0]
            };
            month(&root, "FNO", Some(below), n, 4_321 + k);
            record(entry(nifty(Segment::Fno, Some(below)), n));
        }
    }
    for i in 0..census {
        // HALF CASH, HALF OPTIONS OF OTHER UNDERLYINGS: every 400 contracts
        // (four expiries, fifty strikes, a call and a put) share a symbol.
        let key = if i % 2 == 0 {
            EntryKey {
                contract: None,
                exchange: Exchange::Nse,
                segment: Segment::Cash,
                symbol: symbol(&format!("S{i:07}")),
                timeframe: Timeframe::MINUTE_1,
                month: bench_month(),
            }
        } else {
            let k = i / 2;
            let expiry = OTHER_EXPIRIES
                .get(k % OTHER_EXPIRIES.len())
                .copied()
                .unwrap_or(EXPIRY);
            let strike = 1_000_000 + i64::try_from((k / 4) % 50).unwrap_or(0) * 5_000;
            let side = if (k / 200) % 2 == 0 { "CE" } else { "PE" };
            EntryKey {
                contract: Some(contract(&format!("{expiry}-{strike}-{side}"))),
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                symbol: symbol(&format!("U{:06}", k / 400)),
                timeframe: Timeframe::SECOND_1,
                month: bench_month(),
            }
        };
        record(entry(key, 375));
    }
    let path = pull::manifest::manifest_path(&root, Vendor::Gdfl);
    if let Err(why) = std::fs::write(&path, manifest.image()) {
        refuse(&format!("the served census would not write: {why}"));
    }
    root
}

/// Serves `root` on a loopback port from a runtime of its own, and connects.
fn serve(bars: usize, census: usize, root: PathBuf) -> Served {
    let site = api::server::Site::load(&root.join("masters"), &root);
    let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(why) => refuse(&format!("a loopback port for the served bench: {why}")),
    };
    let Ok(addr) = listener.local_addr() else {
        refuse("the served bench's own address")
    };
    if listener.set_nonblocking(true).is_err() {
        refuse("a non-blocking listener");
    }
    let router = api::server::router_serving(
        std::sync::Arc::new(site),
        std::sync::Arc::new(api::assets::Assets::new(&root.join("web"))),
        addr,
    );
    let spawned = std::thread::Builder::new()
        .name(format!("served-{bars}"))
        .spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            else {
                refuse("a runtime for the served bench")
            };
            runtime.block_on(async move {
                let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
                    refuse("the served bench's listener")
                };
                if let Err(why) = axum::serve(listener, router).await {
                    refuse(&format!("the served bench's server stopped: {why}"));
                }
            });
        });
    if spawned.is_err() {
        refuse("the served bench's server thread");
    }
    let conn = match TcpStream::connect(addr) {
        Ok(conn) => conn,
        Err(why) => refuse(&format!("the served bench's connection: {why}")),
    };
    let _no_delay = conn.set_nodelay(true);
    let _timeout = conn.set_read_timeout(Some(Duration::from_mins(2)));
    Served {
        bars,
        census,
        root,
        addr,
        conn,
    }
}

/// The three served stores, 1x / 10x / 100x.
pub(crate) fn stores() -> Vec<Served> {
    MONTH_SIZES
        .iter()
        .zip(CENSUS_SIZES)
        .map(|(&bars, census)| {
            let started = Instant::now();
            let root = store_of(bars, census);
            let served = serve(bars, census, root);
            println!(
                "  served store: {bars} bars a month, {census} other series, built and loaded in {} ms",
                started.elapsed().as_millis()
            );
            served
        })
        .collect()
}

/// The position of `needle` in `hay`, case-insensitively for ASCII.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
}

impl Served {
    /// Reads one line ending `\r\n` off `buf` from `at`, reading more as
    /// needed; returns where it ends.
    fn line_end(&mut self, buf: &mut Vec<u8>, at: usize) -> usize {
        loop {
            if let Some(end) = find(buf.get(at..).unwrap_or(&[]), b"\r\n") {
                return at + end;
            }
            self.more(buf);
        }
    }

    /// Reads more of the answer into `buf`.
    fn more(&mut self, buf: &mut Vec<u8>) {
        let mut chunk = vec![0_u8; 65_536];
        match self.conn.read(&mut chunk) {
            Ok(0) => refuse("the served bench's server closed the connection"),
            Ok(n) => buf.extend_from_slice(chunk.get(..n).unwrap_or(&[])),
            Err(why) => refuse(&format!("the served bench's read: {why}")),
        }
    }

    /// One request, `extra` headers added, on the kept connection.
    pub(crate) fn ask(&mut self, target: &str, extra: &str) -> Answer {
        let request = format!(
            "GET {target} HTTP/1.1\r\nHost: {}\r\n{extra}\r\n",
            self.addr
        );
        if let Err(why) = self.conn.write_all(request.as_bytes()) {
            refuse(&format!("the served bench's write: {why}"));
        }
        let mut buf = Vec::with_capacity(65_536);
        let head_end = loop {
            if let Some(end) = find(&buf, b"\r\n\r\n") {
                break end;
            }
            self.more(&mut buf);
        };
        let head = String::from_utf8_lossy(buf.get(..head_end).unwrap_or(&[])).to_ascii_lowercase();
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse::<u16>().ok())
            .unwrap_or(0);
        let mut at = head_end + 4;
        let header = |name: &str| {
            head.lines()
                .find_map(|line| line.strip_prefix(name))
                .map(str::trim)
                .map(str::to_owned)
        };
        let mut body = Vec::new();
        if let Some(length) = header("content-length:") {
            let Ok(length) = length.parse::<usize>() else {
                refuse("a content length that is a number")
            };
            while buf.len() < at + length {
                self.more(&mut buf);
            }
            body.extend_from_slice(buf.get(at..at + length).unwrap_or(&[]));
        } else if header("transfer-encoding:").is_some_and(|te| te.contains("chunked")) {
            loop {
                let end = self.line_end(&mut buf, at);
                let size_text =
                    String::from_utf8_lossy(buf.get(at..end).unwrap_or(&[])).to_string();
                let Ok(size) =
                    usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
                else {
                    refuse(&format!("a chunk size, not {size_text:?}"))
                };
                at = end + 2;
                while buf.len() < at + size + 2 {
                    self.more(&mut buf);
                }
                body.extend_from_slice(buf.get(at..at + size).unwrap_or(&[]));
                at += size + 2;
                if size == 0 {
                    break;
                }
            }
        } else if status != 304 && status != 204 {
            refuse(&format!(
                "an answer to {target} with no length and no chunks: {head}"
            ));
        }
        Answer {
            status,
            head,
            body: String::from_utf8_lossy(&body).into_owned(),
        }
    }
}

/// One target a route family asks: its label, the request, any extra header,
/// and the needle a full answer carries.
pub(crate) struct Target {
    pub(crate) label: String,
    pub(crate) path: String,
    pub(crate) extra: String,
    pub(crate) status: u16,
    pub(crate) needle: String,
    /// What the gated figure is divided by.
    pub(crate) per: Per,
}

/// What one target's gated figure is per.
#[derive(Clone, Copy)]
pub(crate) enum Per {
    /// The request: its cost must not grow at all.
    Request,
    /// Each occurrence of this needle in the answer, one a row: a route whose
    /// answer IS the rows asked (a whole month, the whole census) by design.
    /// Its cost per row must not grow, which catches a sort, a second pass
    /// or a walk of what was not asked; its growth per request is the
    /// answer's and is named for its limits row.
    Row(&'static str),
    /// Each bar of the asked month: a route that reads the whole month by
    /// design (the gap audit). Its cost per bar must not grow.
    MonthBar,
}

/// p50, p99 and max of `times`, ns.
fn spread(times: &mut [u128]) -> (u128, u128, u128) {
    times.sort_unstable();
    let at = |q: usize| {
        let k = (times.len() * q).div_ceil(100).max(1) - 1;
        times.get(k).copied().unwrap_or(0)
    };
    (at(50), at(99), times.last().copied().unwrap_or(0))
}

/// Asks `target` until it answers in full, at most a minute; the first
/// answer's time in µs, and what the gated figure is divided by.
fn warmed(served: &mut Served, target: &Target) -> (u128, u128) {
    let started = Instant::now();
    let mut first = None;
    loop {
        let asked = Instant::now();
        let answer = served.ask(&target.path, &target.extra);
        let took = asked.elapsed().as_micros();
        let _ = first.get_or_insert(took);
        let building = answer.body.contains("\"building\"") || answer.head.contains("retry-after");
        if answer.status == target.status && !building && answer.body.contains(&target.needle) {
            let per = match target.per {
                Per::Request => 1,
                Per::Row(row) => answer.body.matches(row).count(),
                Per::MonthBar => served.bars,
            };
            if per == 0 {
                refuse(&format!("{} answered no row to divide by", target.label));
            }
            if served.census == CENSUS_SIZES[0] {
                // WHAT WAS TIMED, SHOWN: the answer's first bytes at 1x, so a
                // reader sees a full answer and not a refusal wearing its
                // status.
                let shown: String = answer.body.chars().take(220).collect();
                println!(
                    "  {} answers {} ({} bytes): {shown}",
                    target.label,
                    answer.status,
                    answer.body.len()
                );
            }
            return (first.unwrap_or(took), u128::try_from(per).unwrap_or(1));
        }
        if started.elapsed() > Duration::from_mins(1) {
            let shown: String = answer.body.chars().take(600).collect();
            refuse(&format!(
                "{} at {} bars answered {} without {:?} after a minute: {shown}",
                target.label, served.bars, answer.status, target.needle
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Times every target of a family at every size and gates each p50 ratio.
pub(crate) fn flat(
    family: &str,
    stores: &mut [Served],
    mut targets: impl FnMut(&mut Served) -> Vec<Target>,
) -> bool {
    let mut ok = true;
    let per_store: Vec<Vec<Target>> = stores.iter_mut().map(&mut targets).collect();
    let Some(count) = per_store.first().map(Vec::len) else {
        refuse("a family with targets")
    };
    for index in 0..count {
        let mut p50s = Vec::with_capacity(stores.len());
        let mut p99s = Vec::with_capacity(stores.len());
        let mut label = String::new();
        for (served, asked) in stores.iter_mut().zip(&per_store) {
            let Some(target) = asked.get(index) else {
                refuse("one target list per store")
            };
            label.clone_from(&target.label);
            let (cold, per) = warmed(served, target);
            let mut times = Vec::with_capacity(N);
            for _ in 0..N {
                let asked = Instant::now();
                let answer = served.ask(&target.path, &target.extra);
                times.push(asked.elapsed().as_nanos());
                if answer.status != target.status {
                    refuse(&format!(
                        "{} answered {} while timed at {} bars",
                        target.label, answer.status, served.bars
                    ));
                }
            }
            let (p50, p99, max) = spread(&mut times);
            println!(
                "  {family} {label:<34} {:>7} bars {:>7} series  p50 {p50:>9} ns  p99 {p99:>9} ns  max {max:>9} ns  n={N}  cold {cold} µs{}",
                served.bars,
                served.census,
                if per > 1 {
                    format!("  per {per} rows")
                } else {
                    String::new()
                }
            );
            p50s.push(p50 * 1_000 / per);
            p99s.push(p99 * 1_000 / per);
        }
        let (Some(&base), Some(&mid), Some(&big)) = (p50s.first(), p50s.get(1), p50s.get(2)) else {
            refuse("three sizes measured");
        };
        let unit = match per_store
            .first()
            .and_then(|asked| asked.get(index))
            .map(|t| t.per)
        {
            Some(Per::Row(_)) => " per row",
            Some(Per::MonthBar) => " per bar",
            _ => "",
        };
        ok &= ratio(
            &format!("{family} {label} p50{unit} 1x->10x"),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        );
        ok &= ratio(
            &format!("{family} {label} p50{unit} 1x->100x"),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
        if let (Some(&low), Some(&high)) = (p99s.first(), p99s.get(2)) {
            let permille = (high * 1_000).checked_div(low).unwrap_or(0);
            println!(
                "  {family} {label:<34} p99{unit} 1x->100x ratio {}.{:03}x (printed, not gated)",
                permille / 1_000,
                permille % 1_000
            );
        }
    }
    ok
}

/// A target answering `200` with `needle`.
pub(crate) fn ok_with(label: &str, path: String, needle: &str) -> Target {
    Target {
        label: label.to_owned(),
        path,
        extra: String::new(),
        status: 200,
        needle: needle.to_owned(),
        per: Per::Request,
    }
}

/// [`ok_with`], gated per `per` rather than per request.
pub(crate) fn ok_per(label: &str, path: String, needle: &str, per: Per) -> Target {
    Target {
        per,
        ..ok_with(label, path, needle)
    }
}

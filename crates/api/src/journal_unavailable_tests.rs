//! PULL RUNS NEVER START UNJOURNALLED, AND NO TOKIO WORKER WAITS ON THE
//! JOURNAL LOCK (AG-06b, AG06B-1 and AG06B-2; D-7100, D-7101).
//!
//! AG-06 made every journal append wait a bounded time for the append lock
//! (`audit::APPEND_LOCK_WAIT`, one second) and left two halves in this file
//! NOT DONE (`docs/06-limits.md` L-AG06-03, L-AG06-07): a run whose opening
//! record was refused after the bound still ran, fetching and landing with
//! nothing on the journal, so the restart scan read it as clean if it then
//! died; and the wait ran on the tokio worker of the async caller, so one
//! contended append stalled every task queued on that worker.
//!
//! Every fixture is invented: a loopback vendor that counts the requests it
//! is sent and answers each `401`, a scripted credential, two index rows.
//! Nothing here reaches a real vendor, a real credential or a real store.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::credential_law::{Credentials, Script};
use std::io::{Read as _, Write as _};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// The time a bound may overrun by on a machine shared by other builds.
const SLACK: Duration = Duration::from_millis(750);

/// What the test's own holder of the append lock says it is doing, so a
/// refusal that names the holder can be told from any other.
const HOLDER: &str = "the AG-06b test holder of the append lock";

/// A loopback vendor that counts every connection it is sent and answers
/// each with `401`. On its own thread, outside every tokio runtime, so it
/// counts a request even while a runtime's workers are stalled.
struct CountingVendor {
    base: &'static str,
    asked: Arc<AtomicUsize>,
}

impl CountingVendor {
    fn rejecting() -> Self {
        let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let addr = socket.local_addr().expect("its address");
        let asked = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&asked);
        std::thread::spawn(move || {
            while let Ok((mut stream, _)) = socket.accept() {
                count.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 4096];
                drop(stream.read(&mut buf));
                drop(stream.write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\n\
                      Content-Length: 2\r\nConnection: close\r\n\r\n{}",
                ));
                drop(stream.flush());
            }
        });
        Self {
            base: Box::leak(format!("http://{addr}").into_boxed_str()),
            asked,
        }
    }

    /// How many requests reached it.
    fn asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }
}

/// A serving site over two invented index rows Dhan names, its credential
/// scripted against `vendor`, its journal file present (so a holder can lock
/// it) and empty; with a finished daily window over both.
fn spot_site(name: &str, vendor: &CountingVendor) -> (Site, ingest::SpotRequest, PathBuf) {
    let dir = crate::scratch::path(&format!("ag06b-masters-{name}"));
    std::fs::create_dir_all(&dir).expect("masters dir");
    std::fs::write(
        dir.join("groww_instruments.csv"),
        "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
         expiry_date,strike_price,groww_symbol\n\
         NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n\
         NSE,CASH,,BANKNIFTY,IDX,,BANKNIFTY,,,NSE-BANKNIFTY\n",
    )
    .expect("groww master");
    std::fs::write(
        dir.join("dhan_scrip.csv"),
        "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,\
         SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n\
         NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,13\n\
         NSE,I,NA,INDEX,BANKNIFTY,BANKNIFTY,INDEX,NA,0001-01-01,,,25\n",
    )
    .expect("dhan master");
    let root = crate::scratch::path(&format!("ag06b-store-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("manifest")).expect("store root");
    let mut site = Site::serving(&dir, &root);
    site.credentials = Credentials::Scripted(Arc::new(Script::new(
        vendor.base,
        vec![Ok("ag06b-invented-token".to_owned())],
    )));
    let journal = site.journal();
    std::fs::create_dir_all(journal.path.parent().expect("audit/")).expect("audit dir");
    std::fs::File::create(&journal.path).expect("an empty journal");
    let asked = ingest::parse_spot(
        "target=swept&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05",
        Day::new(2026, 8, 10).expect("a day"),
    )
    .expect("a finished daily window");
    (site, asked, root)
}

/// Another holder of the journal's append lock, registered in this process
/// under [`HOLDER`], so the refusal names it in `holder_note`'s words.
fn hold(journal: &audit::Journal) -> store::flock::Flock<std::fs::File> {
    store::flock::Flock::try_lock_patiently(
        std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(&journal.path)
            .expect("the journal opens"),
        journal.path.clone(),
        Duration::ZERO,
        HOLDER,
    )
    .expect("nothing else holds it")
}

/// Every file under `root` with its bytes, and every directory, by path.
fn snapshot(root: &Path) -> std::collections::BTreeMap<String, Option<Vec<u8>>> {
    let mut out = std::collections::BTreeMap::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(&dir).expect("a readable directory") {
            let path = entry.expect("an entry").path();
            let name = path.display().to_string();
            if path.is_dir() {
                out.insert(name, None);
                todo.push(path);
            } else {
                out.insert(name, Some(std::fs::read(&path).expect("a readable file")));
            }
        }
    }
    out
}

/// The `pull.run` `refused` events since `from` that name `path`.
fn refused_naming(from: u64, path: &Path) -> Vec<telemetry::Record> {
    let path = path.display().to_string();
    crate::emitted::landed(from, "pull.run", "refused")
        .into_iter()
        .filter(|record| crate::emitted::says(record, "why", &path))
        .collect()
}

/// The `api.audit` `record not appended` events since `from` for `path`.
fn not_appended_at(from: u64, path: &Path) -> Vec<telemetry::Record> {
    let path = path.display().to_string();
    crate::emitted::landed(from, "api.audit", "record not appended")
        .into_iter()
        .filter(|record| crate::emitted::says(record, "path", &path))
        .collect()
}

/// One GET on its own connection from a plain thread, outside every tokio
/// runtime, answered whole.
fn blocking_get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut s = std::net::TcpStream::connect(addr).expect("connect");
    s.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .expect("write");
    let mut buf = String::new();
    s.read_to_string(&mut buf).expect("read");
    buf
}

/// `text` with every byte outside the unreserved set as `%XX`, as a form
/// field carries it.
fn percent_encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The HTTP surface over `site`, served on the current runtime, and the
/// address of a stopper that ends it when connected to.
async fn serving(site: &Loaded, name: &str) -> (std::net::SocketAddr, std::net::SocketAddr) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let stop_addr = stopper.local_addr().expect("addr");
    let _served = tokio::spawn(serve(
        listener,
        router_serving(Loaded::clone(site), super::tests::front(name), addr),
        Box::pin(async move { stopper.accept().await.map(|_| ()) }),
    ));
    (addr, stop_addr)
}

// ------------------------------------------------------- AG06B-1, failing first

/// **A RUN WHOSE OPENING IS REFUSED AFTER THE BOUND ASKS THE VENDOR NOTHING
/// AND LANDS NOTHING** (AG06B-1; L-AG06-03, L-AG06-04, FM-629).
///
/// The append lock is held past `APPEND_LOCK_WAIT` by a holder registered in
/// this process. Before AG-06b the run kept `opened_refused` and went on: it
/// asked the vendor, and a run that then died left nothing on the journal.
/// Now it is refused by name before its first vendor request: the fixture
/// vendor counts 0 requests, the store is byte for byte what it was, and the
/// only writes are the `api.audit` Error event of the refused append and one
/// `pull.run` `refused` event, at `Error`, which `/logs` audits.
#[tokio::test]
async fn a_run_whose_opening_is_refused_after_the_bound_asks_the_vendor_nothing_and_lands_nothing()
{
    let _sink = crate::emitted::sink();
    let vendor = CountingVendor::rejecting();
    let (site, asked, root) = spot_site("refused-opening", &vendor);
    let journal = site.journal();
    let before = snapshot(&root);
    let holder = hold(&journal);
    let from = crate::emitted::mark();
    let started = Instant::now();
    let out = broker_run(&asked, &site, &[]).await;
    let took = started.elapsed();
    holder.release().expect("released");

    let blocked = out
        .blocked
        .as_ref()
        .expect("a run whose opening the journal refused is itself refused");
    assert_eq!(
        blocked.code,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{blocked:?}"
    );
    let why = &blocked.why;
    assert!(why.starts_with("journal_unavailable: "), "{why}");
    assert!(
        why.contains("for a run record (OPENED)")
            && why.contains("within its 1000 ms bound (APPEND_LOCK_WAIT)"),
        "the record kind and the bound: {why}"
    );
    assert!(
        why.contains("attempt(s)") && why.contains(HOLDER),
        "the attempts and the holder in holder_note's words: {why}"
    );
    assert!(
        took >= audit::APPEND_LOCK_WAIT && took < audit::APPEND_LOCK_WAIT + SLACK,
        "refused after the bound, not before it and not long after: {took:?}"
    );
    assert_eq!(vendor.asked(), 0, "nothing was asked of the vendor");
    assert_eq!(
        (out.attempted, out.reached, out.touched_wire),
        (0, 0, false),
        "nothing attempted: {out:?}"
    );
    assert!(
        !out.ledger.opened() && out.ledger.opened_refused.is_some(),
        "the ledger keeps the journal's words: {:?}",
        out.ledger
    );
    assert_eq!(snapshot(&root), before, "the store is byte-identical");

    let refused = refused_naming(from, &journal.path);
    assert_eq!(refused.len(), 1, "one pull.run refused event: {refused:?}");
    assert_eq!(refused[0].level, telemetry::Level::Error);
    assert!(
        crate::emitted::says(&refused[0], "scope", "spot")
            && crate::emitted::says(&refused[0], "feed", "dhan")
            && crate::emitted::says(&refused[0], "from", "2026-08-03")
            && crate::emitted::says(&refused[0], "why", "journal_unavailable"),
        "{:?}",
        refused[0]
    );
    assert!(telemetry::is_audited(
        telemetry::Level::Error,
        "pull.run",
        "refused"
    ));
    assert_eq!(
        not_appended_at(from, &journal.path).len(),
        1,
        "the refused opening's own api.audit Error event, and no second append"
    );
    let _ = std::fs::remove_dir_all(&root);
}

// ------------------------------------------------------- AG06B-2, failing first

/// **A CONTENDED JOURNAL APPEND NEVER STALLS THE HTTP SURFACE** (AG06B-2;
/// L-AG06-07, FM-630).
///
/// A MULTI-THREAD runtime with ONE worker: with two, the second keeps
/// answering while the first waits, and the test would pass before the fix.
/// The append lock is held 900 ms while a run opens, inside the bound, so
/// the opening lands; meanwhile 20 GET `/feeds.json`, sent at once from plain
/// threads, must each be answered within 100 ms. Before AG-06b the run's
/// opening waited on the one worker and every GET waited behind it.
#[test]
fn a_contended_journal_append_never_stalls_the_http_surface() {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a one-worker runtime")
        .block_on(async {
            let vendor = CountingVendor::rejecting();
            let (site, asked, root) = spot_site("stall", &vendor);
            let journal = site.journal();
            let site: Loaded = Arc::new(site);
            let (addr, _stop) = serving(&site, "ag06b-stall").await;
            let warm = blocking_get(addr, "/feeds.json");
            assert!(warm.starts_with("HTTP/1.1 200"), "{warm}");

            let holder = hold(&journal);
            let held = Instant::now();
            let run = {
                let site = Loaded::clone(&site);
                tokio::spawn(async move {
                    broker_run_by(&asked, &site, &[], crate::autopilot::Driver::Hand).await
                })
            };
            // THE RUN REACHES ITS OPENING AND WAITS THERE. A plain sleep: this
            // thread is not a worker, and a tokio timer would need the worker.
            std::thread::sleep(Duration::from_millis(150));
            let clients: Vec<_> = (0..20)
                .map(|_| {
                    std::thread::spawn(move || {
                        let asked_at = Instant::now();
                        let answer = blocking_get(addr, "/feeds.json");
                        (asked_at.elapsed(), answer)
                    })
                })
                .collect();
            let answers: Vec<(Duration, String)> = clients
                .into_iter()
                .map(|client| client.join().expect("a client"))
                .collect();
            std::thread::sleep(Duration::from_millis(900).saturating_sub(held.elapsed()));
            holder.release().expect("released");
            let out = run.await.expect("the run's task");

            for (took, answer) in &answers {
                assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
                assert!(
                    *took < Duration::from_millis(100),
                    "a GET waited {took:?} behind the run's contended opening"
                );
            }
            assert!(
                out.blocked.is_none() && out.ledger.opened(),
                "900 ms is inside the bound, so the opening landed and the run ran: {out:?}"
            );
            let _ = std::fs::remove_dir_all(&root);
        });
}

/// **EVERY JOURNAL CALL ON THE PULL PATHS GOES OFF THE WORKERS** (AG06B-2).
///
/// A source-shape check in the `split_once` pattern of
/// `the_sweep_withholds_per_instrument_probes_the_store_and_says_the_horizon_once`.
/// `broker_run_by` reaches the journal only through `RunLedger`, whose every
/// journal call sits inside `off_the_workers`; `fno_walk` reaches it through
/// `fno_opened` and `FnoPage::say_counted`, each inside `off_the_workers`;
/// and none of them adds a `spawn_blocking` (the stop's bounded wait, D-1582,
/// is why: a `spawn_blocking` task can hold a stopping process).
#[test]
fn every_journal_call_on_the_pull_paths_goes_off_the_workers() {
    let src = include_str!("server.rs");
    let body_of = |head: &str| {
        let at = src.split_once(head).expect(head).1;
        at[..at.find("\n}\n").expect("it has an end")].to_owned()
    };
    // Code only: comment lines name the journal in prose.
    let code = |body: &str| {
        body.lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let run = code(&body_of("pub(crate) async fn broker_run_by("));
    assert!(!run.contains("journal."), "no direct journal call:\n{run}");
    assert_eq!(
        run.matches("&journal").count(),
        run.matches("BrokerRun::opened(&journal,").count()
            + run.matches("out.ledger.flush(&journal,").count(),
        "the run hands its journal only to the ledger:\n{run}"
    );

    let ledger = code(&body_of("impl RunLedger {"));
    for method in ledger
        .split("\n    fn ")
        .chain(ledger.split("\n    pub(crate) fn "))
    {
        let method = method.split("\n    fn ").next().unwrap_or(method);
        if let Some(call) = method.find("journal.") {
            let off = method.find("off_the_workers(").unwrap_or_else(|| {
                panic!("a ledger method calls the journal on a worker:\n{method}")
            });
            assert!(off < call, "the journal call is inside it:\n{method}");
        }
    }
    for wrapped in [
        "match off_the_workers(|| journal.open_run(&head))",
        "off_the_workers(|| match (self.thread.as_mut(), self.head.as_ref())",
    ] {
        assert!(ledger.contains(wrapped), "{wrapped}");
    }
    assert_eq!(
        ledger.matches("off_the_workers(").count(),
        4,
        "open, close, flush and rest"
    );

    let walk = code(&body_of("async fn fno_walk("));
    assert!(
        !walk.contains("journal."),
        "no direct journal call:\n{walk}"
    );
    assert!(
        walk.contains("match fno_opened(journal, asked, now)"),
        "{walk}"
    );
    assert!(
        code(&body_of("fn fno_opened(")).contains("off_the_workers(|| journal.append(&head))"),
        "the walk's opening"
    );
    assert!(
        code(&body_of("    fn say_counted(").replace("\n    }\n", "\n}\n"))
            .contains("off_the_workers(|| recorded_fact(self.journal, &record))"),
        "the walk's close"
    );
    for (name, body) in [
        ("broker_run_by", &run),
        ("RunLedger", &ledger),
        ("fno_walk", &walk),
        ("off_the_workers", &code(&body_of("fn off_the_workers<T>("))),
    ] {
        assert!(
            !body.contains("spawn_blocking"),
            "{name} adds no spawn_blocking"
        );
    }
}

// ------------------------------------------------------------------ extremes

/// **REFUSED AT ITS FIRST ATTEMPT, AT ONCE** (extreme: the zero wait). The
/// journal's path is a directory, so the opening fails at its first `open`
/// with no lock asked for and no wait: the run is refused by name at once,
/// far inside the bound, and nothing is fetched. A zero LOCK bound itself is
/// `audit::lock_wait_tests::a_zero_bound_refuses_at_once`'s: a run's bound is
/// the constant `APPEND_LOCK_WAIT`, and `audit.rs` is not this part's file.
#[tokio::test]
async fn an_opening_refused_at_its_first_attempt_refuses_the_run_at_once_and_asks_nothing() {
    let vendor = CountingVendor::rejecting();
    let (site, asked, root) = spot_site("first-attempt", &vendor);
    let journal = site.journal();
    std::fs::remove_file(&journal.path).expect("the empty journal goes");
    std::fs::create_dir(&journal.path).expect("a directory where the journal goes");
    let started = Instant::now();
    let out = broker_run(&asked, &site, &[]).await;
    let took = started.elapsed();
    let why = &out.blocked.as_ref().expect("refused").why;
    assert!(
        why.starts_with("journal_unavailable: ") && why.contains("cannot open the journal"),
        "{why}"
    );
    assert!(
        took < SLACK,
        "refused at once, not after the bound: {took:?}"
    );
    assert_eq!(vendor.asked(), 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// **REFUSED AT ITS OPENING, ASKED AGAIN AT ONCE, IT LANDS WHEN THE HOLDER
/// LETS GO** (extreme). The refusal is not sticky: nothing remembers it, and
/// the next run opens, is on the journal, and asks the vendor.
#[tokio::test]
async fn a_run_refused_at_its_opening_and_asked_again_at_once_lands_when_the_holder_lets_go() {
    let vendor = CountingVendor::rejecting();
    let (site, asked, root) = spot_site("again", &vendor);
    let journal = site.journal();
    let holder = hold(&journal);
    let first = broker_run(&asked, &site, &[]).await;
    holder.release().expect("released");
    assert!(first.blocked.is_some(), "{first:?}");
    assert_eq!((vendor.asked(), journal.look().records()), (0, 0));

    let second = broker_run(&asked, &site, &[]).await;
    assert!(
        second.blocked.is_none() && second.ledger.opened(),
        "{second:?}"
    );
    assert!(vendor.asked() >= 1, "the second run asked the vendor");
    let opening = journal
        .page(journal.look().records(), journal.look().records() - 1, 1)
        .expect("a page")[0]
        .decoded
        .clone()
        .expect("decodes");
    assert_eq!(
        (opening.kind, opening.outcome),
        (audit::Kind::Run, audit::Outcome::Opened),
        "the journal's first record is the second run's opening"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A JOURNAL THAT CANNOT BE MADE REFUSES THE RUN BY NAME** (extreme: the
/// journal missing). `audit/` is a regular file, so the journal is missing
/// and cannot be created: the run is refused, nothing is fetched, and no
/// directory is made in its place.
#[tokio::test]
async fn a_journal_that_cannot_be_made_refuses_the_run_by_name_and_asks_nothing() {
    let vendor = CountingVendor::rejecting();
    let (site, asked, root) = spot_site("missing", &vendor);
    let journal = site.journal();
    let audit_dir = journal.path.parent().expect("audit/").to_path_buf();
    std::fs::remove_dir_all(&audit_dir).expect("the audit dir goes");
    std::fs::write(&audit_dir, b"not a directory").expect("a file where audit/ goes");
    let before = snapshot(&root);
    let out = broker_run(&asked, &site, &[]).await;
    let why = &out.blocked.as_ref().expect("refused").why;
    assert!(
        why.starts_with("journal_unavailable: ") && why.contains("audit directory"),
        "{why}"
    );
    assert_eq!(vendor.asked(), 0);
    assert_eq!(snapshot(&root), before, "nothing made in its place");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A READ-ONLY JOURNAL REFUSES THE RUN BY NAME** (extreme). Mode bits do
/// not bind root, so the body runs where they do (`crate::isolated`, D-0995).
#[test]
fn a_read_only_journal_refuses_the_run_by_name_and_asks_nothing() {
    crate::isolated::where_permission_binds(
        "server::journal_unavailable_tests::a_read_only_journal_refuses_the_run_by_name_and_asks_nothing",
        || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a current-thread runtime")
                .block_on(async {
                    use std::os::unix::fs::PermissionsExt as _;
                    let vendor = CountingVendor::rejecting();
                    let (site, asked, root) = spot_site("read-only", &vendor);
                    let journal = site.journal();
                    std::fs::set_permissions(&journal.path, std::fs::Permissions::from_mode(0o444))
                        .expect("chmod");
                    let out = broker_run(&asked, &site, &[]).await;
                    let why = &out.blocked.as_ref().expect("refused").why;
                    assert!(
                        why.starts_with("journal_unavailable: ")
                            && why.contains("cannot open the journal"),
                        "{why}"
                    );
                    assert_eq!(vendor.asked(), 0);
                    assert_eq!(std::fs::metadata(&journal.path).expect("kept").len(), 0);
                    let _ = std::fs::remove_dir_all(&root);
                });
        },
    );
}

/// **ON A CURRENT-THREAD RUNTIME THE LEDGER'S JOURNAL CALLS RUN INLINE, AND
/// A REFUSED OPENING STILL REFUSES** (extreme). `block_in_place` panics on a
/// current-thread runtime, which is why `off_the_workers` runs the work where
/// it is there; every ledger call (open, flush, close, rest) is driven on
/// one, with the opening first refused and then landed.
#[test]
fn on_a_current_thread_runtime_the_ledger_runs_inline_and_a_refused_opening_still_refuses() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a current-thread runtime")
        .block_on(async {
            assert_eq!(
                off_the_workers(|| tokio::runtime::Handle::current().runtime_flavor()),
                tokio::runtime::RuntimeFlavor::CurrentThread
            );
            assert!(
                std::panic::catch_unwind(|| tokio::task::block_in_place(|| ())).is_err(),
                "the premise: block_in_place is refused here, so the inline arm is needed"
            );
            let vendor = CountingVendor::rejecting();
            let (site, asked, root) = spot_site("current-thread", &vendor);
            let journal = site.journal();
            let holder = hold(&journal);
            let refused = broker_run(&asked, &site, &[]).await;
            holder.release().expect("released");
            assert!(refused.blocked.is_some(), "{refused:?}");

            let mut ledger = RunLedger::open(&journal, &asked, 2, crate::autopilot::Driver::Hand);
            assert!(ledger.opened(), "{ledger:?}");
            let failure = [pull::ingest::Failure::new(
                "NIFTY",
                "NIFTY: an invented gap",
            )];
            ledger.flush(&journal, &failure);
            let record = audit::Record::of_run(
                audit::Scope::Spot,
                std::time::SystemTime::now(),
                1,
                ledger.source().expect("opened"),
                asked.window,
                &pull::ingest::Ingested::default(),
            );
            ledger.close(&journal, &record).expect("the close lands");
            ledger.rest(&journal, std::time::SystemTime::now(), &failure);
            assert_eq!(
                (
                    journal.look().records(),
                    ledger.written,
                    ledger.refused.as_deref()
                ),
                (3, 1, None),
                "opening, failure, close"
            );
            let _ = std::fs::remove_dir_all(&root);
        });
}

/// **COMBINATION: REFUSED PAST THE BOUND ON ONE WORKER, WHILE THE HTTP
/// SURFACE IS ASKED.** The lock is held past the bound on a one-worker
/// multi-thread runtime: the run is refused by name and asks the vendor
/// nothing, AND the 20 GETs sent during its second-long wait are each
/// answered within 100 ms. Before AG-06b both halves failed at once: the run
/// ran unjournalled after stalling the worker for the whole bound.
#[test]
fn a_run_refused_past_the_bound_on_one_worker_neither_stalls_the_http_surface_nor_asks_the_vendor()
{
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a one-worker runtime")
        .block_on(async {
            let vendor = CountingVendor::rejecting();
            let (site, asked, root) = spot_site("stall-refused", &vendor);
            let journal = site.journal();
            let site: Loaded = Arc::new(site);
            let (addr, _stop) = serving(&site, "ag06b-stall-refused").await;
            assert!(blocking_get(addr, "/feeds.json").starts_with("HTTP/1.1 200"));
            let holder = hold(&journal);
            let run = {
                let site = Loaded::clone(&site);
                tokio::spawn(async move {
                    broker_run_by(&asked, &site, &[], crate::autopilot::Driver::Hand).await
                })
            };
            std::thread::sleep(Duration::from_millis(150));
            let clients: Vec<_> = (0..20)
                .map(|_| {
                    std::thread::spawn(move || {
                        let asked_at = Instant::now();
                        let answer = blocking_get(addr, "/feeds.json");
                        (asked_at.elapsed(), answer)
                    })
                })
                .collect();
            let answers: Vec<(Duration, String)> = clients
                .into_iter()
                .map(|client| client.join().expect("a client"))
                .collect();
            let out = run.await.expect("the run's task");
            holder.release().expect("released");
            for (took, answer) in &answers {
                assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
                assert!(*took < Duration::from_millis(100), "a GET waited {took:?}");
            }
            assert!(
                out.blocked
                    .as_ref()
                    .is_some_and(|b| b.why.starts_with("journal_unavailable: ")),
                "{out:?}"
            );
            assert_eq!(vendor.asked(), 0);
            let _ = std::fs::remove_dir_all(&root);
        });
}

/// **COMBINATION: THE AUTOPILOT'S RUN, REFUSED AT ITS OPENING, READS AS A
/// JOURNAL ERROR AND A TRANSIENT FAULT.** `autopilot.rs` stays as merged, so
/// the refused run must leave `BrokerRun` in the shape its unchanged code
/// reads: blocked (so the tick appends a `Refused` record, which closes no
/// run), `opened_refused` set (so `journal_error` carries the journal's words
/// to `/autopilot.json`), and a reason `classify` files as transport, never
/// as a dead credential or a full disk, so the month backs off and retries.
#[tokio::test]
async fn the_autopilots_run_refused_at_its_opening_reads_as_a_journal_error_and_a_transient_fault()
{
    let vendor = CountingVendor::rejecting();
    let (site, asked, root) = spot_site("autopilot", &vendor);
    let journal = site.journal();
    let holder = hold(&journal);
    let run = broker_run_by(&asked, &site, &[], crate::autopilot::Driver::Autopilot).await;
    holder.release().expect("released");
    assert_eq!(vendor.asked(), 0);
    let blocked = run.blocked.as_ref().expect("refused");
    // THE AUTOPILOT'S OWN READING, verbatim from `autopilot::tick`.
    let journal_error = run
        .ledger
        .refused
        .clone()
        .or_else(|| run.ledger.opened_refused.clone());
    assert!(
        journal_error
            .as_deref()
            .is_some_and(|why| why.contains("within its 1000 ms bound")),
        "{journal_error:?}"
    );
    assert!(
        run.ledger
            .source()
            .is_some_and(|s| s.contains("autopilot ")),
        "{:?}",
        run.ledger
    );
    let outcome = crate::autopilot::outcome_of(&run, false, journal_error);
    assert_eq!(outcome.reason.as_deref(), Some(blocked.why.as_str()));
    assert_eq!(
        crate::autopilot::classify(&blocked.why),
        crate::autopilot::Trouble::Transport
    );
    assert!(outcome.credential.is_none() && !outcome.stopped);
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE F&O WALK TAKES THE SAME PATH** (extreme: one shared code path).
/// An expired-F&O walk whose opening is refused after the bound is refused
/// with the same sentence and the same receipt lines, before its credential
/// read and its first vendor request, and appends nothing.
#[tokio::test]
async fn an_fno_walk_whose_opening_is_refused_asks_nothing_and_appends_nothing() {
    let _sink = crate::emitted::sink();
    let vendor = CountingVendor::rejecting();
    let (site, _spot, root) = spot_site("fno", &vendor);
    let journal = site.journal();
    let holder = hold(&journal);
    let from = crate::emitted::mark();
    let (code, page) = fno_answer(
        "underlying=NIFTY&series=fut&expiry=2026-07-30&from=2026-07-01&to=2026-07-30&vendor=dhan",
        Day::new(2026, 8, 7).expect("a day"),
        std::time::SystemTime::now(),
        &journal,
        site.broker,
        &site,
    )
    .await;
    holder.release().expect("released");
    assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE, "{page}");
    assert!(page.contains("journal_unavailable: "), "{page}");
    assert!(page.contains(HOLDER), "{page}");
    assert!(
        page.contains("<th>Recorded</th><td>NO — this run is NOT in the journal"),
        "{page}"
    );
    assert!(!page.contains("Opened in the journal"), "{page}");
    assert_eq!(vendor.asked(), 0);
    assert_eq!(journal.look().records(), 0, "nothing appended");
    let refused = refused_naming(from, &journal.path);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(crate::emitted::says(&refused[0], "scope", "fno"));
    let _ = std::fs::remove_dir_all(&root);
}

/// **OUT OF THE BOX: THE JOURNAL REPLACED WHILE THE OPENING WAITS.** The
/// opening opened the journal, met the held lock and waited; meanwhile the
/// journal was moved aside and a new one put at its path, and only then did
/// the holder let go. The opening takes the lock on a file no reader opens,
/// so audit refuses it by name; the run is refused, asks nothing, and
/// neither the old file nor the new one gains a record.
#[tokio::test]
async fn a_journal_replaced_while_the_opening_waits_refuses_the_run_and_neither_file_gains_a_record()
 {
    let vendor = CountingVendor::rejecting();
    let (site, asked, root) = spot_site("replaced", &vendor);
    let journal = site.journal();
    let aside = journal.path.with_extension("aside");
    let holder = hold(&journal);
    let mover = {
        let (path, aside) = (journal.path.clone(), aside.clone());
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            std::fs::rename(&path, &aside).expect("moved aside");
            std::fs::File::create(&path).expect("a new journal");
            std::thread::sleep(Duration::from_millis(200));
            holder.release().expect("released");
        })
    };
    let out = broker_run(&asked, &site, &[]).await;
    mover.join().expect("the mover");
    let why = &out.blocked.as_ref().expect("refused").why;
    assert!(
        why.starts_with("journal_unavailable: ") && why.contains("replaced or moved"),
        "{why}"
    );
    assert_eq!(vendor.asked(), 0);
    assert_eq!(std::fs::metadata(&aside).expect("the old").len(), 0);
    assert_eq!(std::fs::metadata(&journal.path).expect("the new").len(), 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE RUN KEY A REFUSED RUN CLAIMED IS RELEASED.** A refused run never
/// reaches `note_run_finished`, the only other release, so without this every
/// later event of the process would be stamped with a run that never ran. The
/// key is a process global shared with every other test in this binary, so a
/// fresh id is claimed once nobody else holds it, then handed to the refusal.
#[test]
fn the_run_key_a_refused_run_claimed_is_released() {
    let sink = crate::emitted::sink();
    let id = u64::MAX - 0x0a06b;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !sink.claim_run(id) {
        assert!(
            Instant::now() < deadline,
            "the run key stayed claimed by another test"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(sink.run(), id, "the premise: this test holds the key");
    let day = Day::new(2026, 8, 3).expect("a day");
    note_run_refused(
        "spot",
        "dhan",
        pull::session::Window::new(day, day).expect("forwards"),
        &journal_unavailable("an invented refusal"),
        Some(id),
    );
    assert_ne!(sink.run(), id, "released");
}

/// **THE REFUSAL IS ON THE RECEIPT AND ON `/pull/run.json`.** A `/pull/run`
/// press of one spot leg whose opening the journal refuses: the leg's receipt
/// says `journal_unavailable`, that nothing was recorded, and a transient
/// retry class; the conductor files it as the feed's `lastError`, which
/// `/pull/run.json` serves.
#[tokio::test]
async fn the_refusal_is_on_the_receipt_and_on_pull_run_json() {
    let vendor = CountingVendor::rejecting();
    let (site, _asked, root) = spot_site("pull-run-json", &vendor);
    let journal = site.journal();
    let site: Loaded = Arc::new(site);
    let form = "target=swept&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05";

    let holder = hold(&journal);
    let (code, _receipt, page) =
        pull_spot(axum::extract::State(Loaded::clone(&site)), form.to_owned()).await;
    assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let page = page.0;
    assert!(page.contains("journal_unavailable: "), "{page}");
    assert!(
        page.contains("<th>Recorded</th><td>NO — this run is NOT in the journal"),
        "{page}"
    );
    assert_eq!(
        crate::pullrun::RetryClass::of_receipt(&page),
        Some(crate::pullrun::RetryClass::Transient),
        "{page}"
    );

    let leg = format!("/pull/spot|dhan|1day|AG06b invented leg|{form}");
    let body = format!("leg={}", percent_encoded(&leg));
    let (started, _, said) = pull_run(axum::extract::State(Loaded::clone(&site)), body).await;
    assert_eq!(started, axum::http::StatusCode::ACCEPTED, "{said}");
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        let (_, _, status) = pull_run_json(axum::extract::State(Loaded::clone(&site))).await;
        if status.contains("journal_unavailable") || Instant::now() > deadline {
            break status;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let _stopped = pull_run_stop(axum::extract::State(Loaded::clone(&site))).await;
    holder.release().expect("released");
    assert!(
        status.contains("answered HTTP 503: journal_unavailable: "),
        "{status}"
    );
    assert_eq!(vendor.asked(), 0);
    assert_eq!(journal.look().records(), 0, "nothing appended");
    let _ = std::fs::remove_dir_all(&root);
}

// -------------------------------------------------------------- measurements

/// Nearest-rank percentile `p` of sorted microseconds.
fn rank(sorted: &[u128], p: usize) -> u128 {
    let n = sorted.len();
    let at = (n * p).div_ceil(100).max(1) - 1;
    sorted[at.min(n - 1)]
}

/// One line of p50 / p99 / max in microseconds over `micros`, sorted here.
fn summary(name: &str, mut micros: Vec<u128>) -> String {
    micros.sort_unstable();
    format!(
        "AG06B op={name} n={} p50_us={} p99_us={} max_us={}",
        micros.len(),
        rank(&micros, 50),
        rank(&micros, 99),
        micros.last().copied().unwrap_or(0)
    )
}

/// **MEASURED, BY HAND: THE LEDGER'S JOURNAL CALLS, UNCONTENDED** (AG-06b,
/// the ratio gate). `RunLedger::open`, `flush` of one failure and `close`,
/// 200 runs each, timed with `Instant` per call, on a task of a multi-thread
/// runtime (so `off_the_workers` takes its `block_in_place` arm) over a
/// fresh temp journal that grows to 800 records. The same file runs on the
/// build before AG-06b for the other column. Run:
/// `cargo test -p api --locked --lib measure_the_ledger_calls_uncontended -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, run by hand with --ignored --nocapture"]
fn measure_the_ledger_calls_uncontended() {
    const N: usize = 200;
    let vendor = CountingVendor::rejecting();
    let (site, asked, root) = spot_site("measure-ledger", &vendor);
    let journal = site.journal();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    let measured = runtime.spawn(async move {
        let (mut opens, mut flushes, mut closes) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..N {
            let failure = [pull::ingest::Failure::new(
                "NIFTY",
                format!("NIFTY: invented gap {i}"),
            )];
            let t = Instant::now();
            let mut ledger = RunLedger::open(&journal, &asked, 2, crate::autopilot::Driver::Hand);
            opens.push(t.elapsed().as_micros());
            assert!(ledger.opened(), "{ledger:?}");
            let t = Instant::now();
            ledger.flush(&journal, &failure);
            flushes.push(t.elapsed().as_micros());
            let record = audit::Record::of_run(
                audit::Scope::Spot,
                std::time::SystemTime::now(),
                1,
                ledger.source().expect("opened"),
                asked.window,
                &pull::ingest::Ingested::default(),
            );
            let t = Instant::now();
            ledger.close(&journal, &record).expect("the close lands");
            closes.push(t.elapsed().as_micros());
        }
        assert_eq!(
            journal.look().records(),
            u64::try_from(N * 3).expect("small"),
            "opening, failure, close"
        );
        [
            summary("RunLedger::open", opens),
            summary("RunLedger::flush", flushes),
            summary("RunLedger::close", closes),
        ]
    });
    let lines = runtime.block_on(measured).expect("the measuring task");
    for line in lines {
        println!("{line}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// **MEASURED, BY HAND: GET `/feeds.json` WHILE A HOLDER KEEPS THE APPEND
/// LOCK** (AG-06b). On a one-worker multi-thread runtime (the failing-first
/// test's shape), 200 sequential GETs from a plain thread with nobody holding
/// the lock; then 200 more while a holder keeps the lock for good and spot
/// runs open against it one after another (each waits the bound). The same
/// file runs on the build before AG-06b for the other column. Run:
/// `cargo test -p api --locked --lib measure_feeds_json_while_a_holder_keeps_the_append_lock -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, run by hand with --ignored --nocapture"]
fn measure_feeds_json_while_a_holder_keeps_the_append_lock() {
    const N: usize = 200;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a one-worker runtime")
        .block_on(async {
            let vendor = CountingVendor::rejecting();
            let (site, asked, root) = spot_site("measure-feeds", &vendor);
            let journal = site.journal();
            let site: Loaded = Arc::new(site);
            let (addr, _stop) = serving(&site, "ag06b-measure-feeds").await;
            let sequential = move || {
                std::thread::spawn(move || {
                    (0..N)
                        .map(|_| {
                            let t = Instant::now();
                            let answer = blocking_get(addr, "/feeds.json");
                            assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
                            t.elapsed().as_micros()
                        })
                        .collect::<Vec<u128>>()
                })
                .join()
                .expect("the client")
            };
            let free = sequential();
            let holder = hold(&journal);
            let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let runs = {
                let (site, done) = (Loaded::clone(&site), Arc::clone(&done));
                tokio::spawn(async move {
                    let mut runs = 0_usize;
                    while !done.load(Ordering::SeqCst) {
                        let _out =
                            broker_run_by(&asked, &site, &[], crate::autopilot::Driver::Hand).await;
                        runs += 1;
                    }
                    runs
                })
            };
            std::thread::sleep(Duration::from_millis(150));
            let held = sequential();
            done.store(true, Ordering::SeqCst);
            let runs = runs.await.expect("the runs");
            holder.release().expect("released");
            println!("{}", summary("GET /feeds.json, no holder", free));
            println!(
                "{} runs_opened_meanwhile={runs}",
                summary("GET /feeds.json, holder and runs opening", held)
            );
            let _ = std::fs::remove_dir_all(&root);
        });
}

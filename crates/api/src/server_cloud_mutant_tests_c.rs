//! Cloud mutation pass C over `server.rs`'s spot-pull, broker, recovery and
//! pull-run half: the broker receipt, the lanes, the run ledger, the pull
//! order, the recovery unit and the pull-run routes.
//!
//! The broker path is driven end to end against a LOOPBACK fake vendor
//! (`127.0.0.1:0`) with the credential taken from
//! [`crate::credential_law::Script`], the `cfg(test)` source the production
//! `Credentials` enum carries for exactly this. No AWS, no real vendor, no
//! browser: every bar here is generated, and none of it is market data.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::credential_law::{CredentialStop, Credentials, Script, Unreadable};
use std::sync::{Arc, Mutex};

// ------------------------------------------------------------------ fixtures

/// A scratch path for this module, pid-stamped by [`crate::scratch::path`].
fn scratch(name: &str) -> PathBuf {
    crate::scratch::path(&format!("cloudc-{name}"))
}

/// One generated daily bar for Dhan's parallel-array shape, stamped `ts`.
fn dhan_bar(ts: i64) -> String {
    format!(
        "{{\"open\":[24500.75],\"high\":[24501.50],\"low\":[24499.25],\
         \"close\":[24500.50],\"volume\":[250],\"timestamp\":[{ts}]}}"
    )
}

/// 2026-08-03 09:15 IST, in UTC epoch seconds.
const AUG_03: i64 = 1_785_728_700;
/// 2026-07-01 09:15 IST, in UTC epoch seconds.
const JUL_01: i64 = 1_782_877_500;

/// A masters directory naming NIFTY (13) and BANKNIFTY (25) at Dhan and Groww,
/// and an empty store root with a manifest directory. Both are removed on drop.
struct Fixture {
    masters: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let masters = scratch(&format!("{name}-masters"));
        let root = scratch(&format!("{name}-store"));
        let _ = std::fs::remove_dir_all(&masters);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&masters).expect("masters dir");
        std::fs::write(
            masters.join("groww_instruments.csv"),
            "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
             expiry_date,strike_price,groww_symbol\n\
             NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n\
             NSE,CASH,,BANKNIFTY,IDX,,BANKNIFTY,,,NSE-BANKNIFTY\n",
        )
        .expect("groww master");
        std::fs::write(
            masters.join("dhan_scrip.csv"),
            "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,\
             SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n\
             NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,13\n\
             NSE,I,NA,INDEX,BANKNIFTY,BANKNIFTY,INDEX,NA,0001-01-01,,,25\n",
        )
        .expect("dhan master");
        std::fs::create_dir_all(root.join("manifest")).expect("store root");
        Self { masters, root }
    }

    /// [`Fixture::new`] with Dhan's master naming four indices and no Groww
    /// master: two of them swept, all four in the index target.
    fn four_indices(name: &str) -> Self {
        let fx = Self::new(name);
        std::fs::remove_file(fx.masters.join("groww_instruments.csv")).expect("groww gone");
        std::fs::write(
            fx.masters.join("dhan_scrip.csv"),
            "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,\
             SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n\
             NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,13\n\
             NSE,I,NA,INDEX,BANKNIFTY,BANKNIFTY,INDEX,NA,0001-01-01,,,25\n\
             NSE,I,NA,INDEX,FINNIFTY,FINNIFTY,INDEX,NA,0001-01-01,,,27\n\
             NSE,I,NA,INDEX,MIDCPNIFTY,MIDCPNIFTY,INDEX,NA,0001-01-01,,,442\n",
        )
        .expect("dhan master");
        fx
    }

    /// A site that refuses the broker, as every ordinary test site does.
    fn refused(&self) -> Site {
        Site::load(&self.masters, &self.root)
    }

    /// A site permitted to reach a broker, whose credential is `script`'s.
    fn serving(&self, script: &Arc<Script>) -> Site {
        let mut site = Site::serving(&self.masters, &self.root);
        site.credentials = Credentials::Scripted(Arc::clone(script));
        site
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.masters);
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A loopback vendor answering each request with `answer(nth, body)`, and
/// keeping every request body it received, in order.
struct FakeVendor {
    base: &'static str,
    seen: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

type Answer = dyn Fn(usize, &str) -> (u16, String) + Send + Sync;

impl FakeVendor {
    async fn answering(
        answer: impl Fn(usize, &str) -> (u16, String) + Send + Sync + 'static,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        let answer: Arc<Answer> = Arc::new(answer);
        let app = axum::Router::new().fallback(move |body: String| {
            let log = Arc::clone(&log);
            let answer = Arc::clone(&answer);
            async move {
                let nth = {
                    let mut log = log.lock().expect("the fake's log");
                    log.push(body.clone());
                    log.len() - 1
                };
                let (code, text) = answer(nth, &body);
                (
                    axum::http::StatusCode::from_u16(code).expect("a status"),
                    text,
                )
            }
        });
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("the fake serves");
        });
        Self {
            base: Box::leak(format!("http://{address}").into_boxed_str()),
            seen,
            task,
        }
    }

    fn seen(&self) -> Vec<String> {
        self.seen.lock().expect("the fake's log").clone()
    }
}

impl Drop for FakeVendor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A script that hands out `value` on every read.
fn live(vendor: &FakeVendor, value: &str) -> Arc<Script> {
    Arc::new(Script::new(vendor.base, vec![Ok(value.to_owned())]))
}

/// Every value the receipt shows under `label`, in order, as rendered.
fn facts(page: &str, label: &str) -> Vec<String> {
    let open = format!("<tr><th>{}</th><td>", render::escape(label));
    page.match_indices(&open)
        .map(|(at, _)| {
            let rest = &page[at + open.len()..];
            rest[..rest.find("</td>").expect("a closed cell")].to_owned()
        })
        .collect()
}

/// Every record in the journal under `root`, newest first.
fn journal_records(root: &Path) -> Vec<audit::Record> {
    let journal = audit::Journal::at(root);
    let audit::Log::Held { records, .. } = journal.look() else {
        return Vec::new();
    };
    journal
        .page(records, 0, records)
        .expect("the journal reads")
        .into_iter()
        .map(|entry| entry.decoded.expect("each record decodes"))
        .collect()
}

fn aug_10() -> Day {
    Day::new(2026, 8, 10).expect("a day")
}

fn spot(body: &str) -> ingest::SpotRequest {
    ingest::parse_spot(body, aug_10()).expect("a valid spot request")
}

const DHAN_DAY: &str = "target=swept&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05";

// ------------------------------------------------------- the spot receipt

/// A repair request says so on its receipt, with the reason it was given,
/// before the broker path is asked anything (`spot_answer`'s repair fact).
#[tokio::test]
async fn a_repair_request_names_its_versioned_landing_on_the_receipt() {
    let fx = Fixture::new("repair-fact");
    let site = fx.refused();
    let body = format!("{DHAN_DAY}&repair_reason=bad%20open");
    let (code, page) = spot_answer(&body, aug_10(), std::time::SystemTime::now(), &site).await;
    assert_eq!(
        code,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "a refused broker blocks the run: {page}"
    );
    assert_eq!(
        facts(&page, "Landing"),
        [render::escape(
            "VERSIONED REPAIR (D-3350): a month the append-only store refuses is published as a \
             new generation of its held and offered bars, a held bar never changed; reason: bad open"
        )]
    );
    // An ordinary request carries no such line.
    let (_, plain) = spot_answer(DHAN_DAY, aug_10(), std::time::SystemTime::now(), &site).await;
    assert!(facts(&plain, "Landing").is_empty(), "{plain}");
}

/// The NSE-checked Zerodha policy names the NSE identity file its assurance
/// rests on; the other policies do not.
#[tokio::test]
async fn the_nse_checked_policy_names_the_nse_identity_file() {
    let fx = Fixture::new("nse-file");
    let site = fx.refused();
    let body = "target=swept&vendor=zerodha&granularity=1day&from=2026-08-03&to=2026-08-05\
                &cash_identity=zerodha_nse_checked";
    let (_, page) = spot_answer(body, aug_10(), std::time::SystemTime::now(), &site).await;
    let expected = render::escape(&site.universe().read.nse_reference.provenance());
    assert_eq!(facts(&page, "NSE identity file"), [expected], "{page}");
    let symbol = "target=swept&vendor=zerodha&granularity=1day&from=2026-08-03&to=2026-08-05\
                  &cash_identity=zerodha_symbol";
    let (_, other) = spot_answer(symbol, aug_10(), std::time::SystemTime::now(), &site).await;
    assert!(facts(&other, "NSE identity file").is_empty(), "{other}");
}

/// A broker run that lands bars answers the stored receipt with every count,
/// names the one member it was asked for, and closes its run in the journal.
/// Covers `broker_answer` past the block, `broker_window`'s `Ok`,
/// `land_broker_member` and `landed_answer`'s ledger arm.
#[tokio::test]
async fn a_broker_run_that_lands_bars_answers_a_stored_receipt() {
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("landed");
    let site = fx.serving(&script);

    let body = format!("{DHAN_DAY}&member=NIFTY");
    let (code, page) = spot_answer(&body, aug_10(), std::time::SystemTime::now(), &site).await;

    assert_eq!(code, axum::http::StatusCode::OK, "{page}");
    assert_eq!(facts(&page, "Source"), ["broker · HTTPS"]);
    assert_eq!(facts(&page, "Instruments asked for"), ["1 named: NIFTY"]);
    assert_eq!(facts(&page, "Instruments expected"), ["1"]);
    assert_eq!(facts(&page, "Instruments attempted"), ["1"]);
    assert_eq!(facts(&page, "Instruments reached"), ["1"]);
    assert!(facts(&page, "Instruments refused").is_empty(), "{page}");
    assert!(facts(&page, "Stopped").is_empty(), "{page}");
    assert!(facts(&page, "Credential").is_empty(), "{page}");
    // NO FAILURE, SO NO FAILURES LINE (kills `delete !` at landed_answer:9382).
    assert!(facts(&page, "Failures recorded").is_empty(), "{page}");
    assert!(facts(&page, "Before first bar").is_empty(), "{page}");
    assert!(
        page.contains("The run finished and every row is accounted for."),
        "a clean run reads clean (kills `delete !` at landed_answer:9395): {page}"
    );
    assert_eq!(facts(&page, "Rows read"), ["1"], "{page}");
    assert_eq!(facts(&page, "Bars written to the file"), ["1"], "{page}");
    let journal = site.journal();
    assert_eq!(
        facts(&page, "Recorded"),
        [render::escape(&format!(
            "yes — appended to {}",
            journal.path.display()
        ))]
    );
    let seen = vendor.seen();
    assert_eq!(seen.len(), 1, "one instrument, one request: {seen:?}");
    assert!(
        seen[0].contains("\"13\""),
        "NIFTY's own id went out: {seen:?}"
    );
    // THE RUN IS OPENED AND CLOSED UNDER ONE SOURCE.
    let records = journal_records(&fx.root);
    assert_eq!(records.len(), 2, "{records:?}");
    assert_eq!(records[0].outcome, audit::Outcome::Stored);
    assert_eq!(records[0].source, records[1].source);
}

/// Naming no member is read as every instrument in the target, and the
/// receipt says so in words; one refused instrument is named first.
#[tokio::test]
async fn an_unnamed_member_set_reads_as_every_instrument_and_names_the_refused_one() {
    let vendor = FakeVendor::answering(|_, body| {
        if body.contains("\"25\"") {
            (
                400,
                "{\"errorType\":\"Data_API_Error\",\"errorCode\":\"DH-905\",\
                 \"errorMessage\":\"missing field\"}"
                    .to_owned(),
            )
        } else {
            (200, dhan_bar(AUG_03))
        }
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("one-refused");
    let site = fx.serving(&script);
    let asked = spot(DHAN_DAY);

    let (code, page) = spot_answer(DHAN_DAY, aug_10(), std::time::SystemTime::now(), &site).await;

    assert_eq!(code, axum::http::StatusCode::OK, "{page}");
    assert_eq!(
        facts(&page, "Instruments asked for"),
        [render::escape(&format!(
            "every instrument in {} — no member was named, which this route reads as all of them",
            asked.target.label()
        ))]
    );
    assert_eq!(facts(&page, "Instruments expected"), ["2"]);
    assert_eq!(facts(&page, "Instruments attempted"), ["2"]);
    assert_eq!(facts(&page, "Instruments reached"), ["1"]);
    let refused = facts(&page, "Instruments refused");
    assert_eq!(refused.len(), 1, "{page}");
    assert!(
        refused[0].starts_with("1 — first: BANKNIFTY: "),
        "{refused:?}"
    );
    // THE FAILURE IS COUNTED AND IT LANDED BEHIND THE RUN'S OPENING RECORD.
    assert_eq!(
        facts(&page, "Failures recorded"),
        [render::escape(
            "1 of 1 — each appended as its member landed, behind this run's opening record."
        )],
        "{page}"
    );
    assert_eq!(vendor.seen().len(), 2);
}

/// A run whose every instrument was refused before any socket answers 400,
/// not 502, and closes its own opened run `NOT STARTED` under its source.
/// Covers `broker_answer`'s reached-none arm and `close_reaching_none`.
#[tokio::test]
async fn a_run_refused_before_the_wire_answers_bad_request_and_closes_its_run() {
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = Arc::new(Script::new(
        vendor.base,
        vec![Err(Unreadable::transport(
            "parameter store timed out".to_owned(),
        ))],
    ));
    let fx = Fixture::new("none-reached");
    let site = fx.serving(&script);

    let (code, page) = spot_answer(DHAN_DAY, aug_10(), std::time::SystemTime::now(), &site).await;

    assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{page}");
    assert!(vendor.seen().is_empty(), "no socket was opened");
    assert_eq!(script.reads(), 2, "each instrument read its own credential");
    assert_eq!(facts(&page, "Instruments reached"), ["0"]);
    assert_eq!(
        facts(&page, "Instruments refused"),
        ["2 — first: BANKNIFTY: parameter store timed out"]
    );
    assert_eq!(
        facts(&page, "Refused because"),
        ["BANKNIFTY: parameter store timed out"]
    );
    assert!(
        facts(&page, "Stopped").is_empty(),
        "a transport fault does not stop"
    );
    let records = journal_records(&fx.root);
    let close = records.first().expect("a closing record");
    assert_eq!(close.outcome, audit::Outcome::NotStarted);
    assert_eq!(close.source, records[records.len() - 1].source);
}

/// A configuration fault stops the run at once; the receipt says Stopped and
/// still answers 400, because no vendor was asked.
#[tokio::test]
async fn a_configuration_fault_is_a_stopped_bad_request() {
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = Arc::new(Script::new(
        vendor.base,
        vec![Err(Unreadable::configuration(
            "no credentials file".to_owned(),
        ))],
    ));
    let fx = Fixture::new("config-stop");
    let site = fx.serving(&script);

    let (code, page) = spot_answer(DHAN_DAY, aug_10(), std::time::SystemTime::now(), &site).await;

    assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{page}");
    let stopped = facts(&page, "Stopped");
    assert_eq!(stopped.len(), 1, "{page}");
    assert!(
        stopped[0].ends_with("Stopped after 1 of 2 instruments; the 1 after it were not asked."),
        "{stopped:?}"
    );
    assert!(
        facts(&page, "Credential").is_empty(),
        "nothing was rejected"
    );
}

/// A vendor that rejects the token: the run reached the wire, so 502, and the
/// receipt carries the credential verdict as its own fact.
#[tokio::test]
async fn a_rejected_token_is_a_bad_gateway_with_the_credential_fact() {
    let vendor = FakeVendor::answering(|_, _| (401, "{}".to_owned())).await;
    let script = live(&vendor, "stale");
    let fx = Fixture::new("dead-token");
    let site = fx.serving(&script);

    let (code, page) = spot_answer(DHAN_DAY, aug_10(), std::time::SystemTime::now(), &site).await;

    assert_eq!(code, axum::http::StatusCode::BAD_GATEWAY, "{page}");
    assert_eq!(
        facts(&page, "Credential"),
        [render::escape(CREDENTIAL_FACT)]
    );
    assert_eq!(facts(&page, "Instruments reached"), ["0"]);
    assert_eq!(vendor.seen().len(), 1, "the dead token was sent once");
}

/// A window whose first chunk landed and whose second was rejected keeps the
/// prefix, lifts the credential death out of `unfetched`, and halts the run.
/// Covers `BrokerRun::lift_credential_death` on the live path.
#[tokio::test]
async fn a_death_after_the_first_chunk_is_lifted_and_halts_the_run() {
    let vendor = FakeVendor::answering(|nth, _| {
        if nth == 0 {
            (200, dhan_bar(JUL_01))
        } else {
            (401, "{}".to_owned())
        }
    })
    .await;
    let script = live(&vendor, "stale");
    let fx = Fixture::new("late-death");
    let site = fx.serving(&script);
    let asked = spot(
        "target=swept&vendor=dhan&granularity=1day&from=2026-07-01&to=2026-08-05&member=NIFTY",
    );

    let run = broker_run(&asked, &site, &[]).await;

    assert_eq!(vendor.seen().len(), 2, "July answered, August rejected");
    assert_eq!(run.reached, 1, "{run:?}");
    assert!(run.credential_dead, "the verdict was lifted: {run:?}");
    assert_eq!(run.credential_stop, Some(CredentialStop::SameValue));
    assert_eq!(
        run.total.bars_committed, 1,
        "the July prefix landed: {run:?}"
    );
    assert!(
        run.total
            .failures
            .iter()
            .all(|failure| !failure.why.contains(CREDENTIAL_DEAD)),
        "the marker never reaches a reason: {:?}",
        run.total.failures
    );
}

/// A Dhan spec, for windows built by hand.
fn dhan_spec() -> pull::vendor::HttpSpec {
    match pull::vendor::Feed::Dhan.descriptor().transport {
        pull::vendor::Transport::Http(spec) => spec,
        pull::vendor::Transport::LocalArchive(_) => panic!("Dhan is HTTP"),
    }
}

/// A hand-built window of `instrument`, with no bodies.
fn window_of(
    instrument: &str,
    segment: &'static str,
    granularity: pull::vendor::Granularity,
) -> BrokerWindow {
    BrokerWindow {
        listing: pull::vendor::Listing::Index,
        contract: None,
        bodies: Vec::new(),
        unfetched: None,
        before_first_bar: None,
        exchange: "NSE",
        segment,
        window: spot(DHAN_DAY).window,
        granularity,
        instrument: instrument.to_owned(),
        origin: String::new(),
        spec: dhan_spec(),
        store_vendor: Vendor::Dhan,
    }
}

/// `lift_credential_death` answers false for a whole window and for a short
/// one cut for another reason, and true, stripped, for a marked one.
#[test]
fn lift_credential_death_reads_only_the_marker() {
    let mut window = window_of("NIFTY", "INDEX", pull::vendor::Granularity::Day1);
    let mut run = BrokerRun::default();
    assert!(!run.lift_credential_death(&mut window));
    window.unfetched = Some("the vendor cut it short".to_owned());
    assert!(!run.lift_credential_death(&mut window));
    assert_eq!(window.unfetched.as_deref(), Some("the vendor cut it short"));
    assert!(!run.credential_dead);
    window.unfetched = Some(format!("{CREDENTIAL_DEAD}token rejected"));
    assert!(run.lift_credential_death(&mut window));
    assert_eq!(window.unfetched.as_deref(), Some("token rejected"));
    assert!(run.credential_dead);
}

/// A run that never opened closes its refusal under the TARGET's label, as a
/// plain append (`close_reaching_none`'s fallback source).
#[test]
fn close_reaching_none_without_a_ledger_source_files_under_the_target() {
    let fx = Fixture::new("close-none");
    let journal = audit::Journal::at(&fx.root);
    let asked = spot(DHAN_DAY);
    let mut run = BrokerRun::default();
    let fact = close_reaching_none(
        &mut run,
        &asked,
        std::time::SystemTime::now(),
        &journal,
        "nothing reached",
    );
    assert_eq!(
        fact,
        (
            "Recorded",
            format!("yes — appended to {}", journal.path.display())
        )
    );
    let records = journal_records(&fx.root);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].outcome, audit::Outcome::NotStarted);
    assert_eq!(records[0].source, asked.target.label());
}

// ------------------------------------------------------------ the recovery

/// A recovery unit refused before its run opened appends a `Refused` record
/// under the target and still answers `Ok` with the block on the run.
#[tokio::test]
async fn a_blocked_recovery_unit_records_a_refusal_and_returns_the_run() {
    let fx = Fixture::new("recovery-blocked");
    let site = fx.refused();
    let asked = spot(DHAN_DAY);
    let seat = site.autopilot.take_seat(asked.feed).expect("a free seat");

    let run = recovery_spot(&site, &asked, seat).await.expect("recorded");

    let blocked = run.blocked.as_ref().expect("a refused broker blocks");
    assert_eq!(blocked.code, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let records = journal_records(&fx.root);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].outcome, audit::Outcome::Refused);
    assert_eq!(records[0].source, asked.target.label());
    assert!(
        site.autopilot.take_seat(asked.feed).is_some(),
        "the seat was released with the unit"
    );
}

/// A recovery unit whose journal cannot be written refuses the unit.
#[tokio::test]
async fn a_recovery_unit_whose_journal_refuses_is_an_error() {
    let fx = Fixture::new("recovery-nojournal");
    std::fs::write(fx.root.join("audit"), b"not a directory").expect("a file where audit/ goes");
    let site = fx.refused();
    let asked = spot(DHAN_DAY);
    let seat = site.autopilot.take_seat(asked.feed).expect("a free seat");

    let refused = recovery_spot(&site, &asked, seat)
        .await
        .expect_err("an unwritable journal refuses the unit");
    assert!(refused.contains("audit"), "{refused}");
}

/// A recovery unit that ran and landed closes its run under the opening
/// record's source and answers the run.
#[tokio::test]
async fn a_recovery_unit_that_landed_closes_its_run() {
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("recovery-landed");
    let site = fx.serving(&script);
    let asked = spot(&format!("{DHAN_DAY}&member=NIFTY"));
    let seat = site.autopilot.take_seat(asked.feed).expect("a free seat");

    let run = recovery_spot(&site, &asked, seat).await.expect("closed");

    assert!(run.blocked.is_none(), "{run:?}");
    assert_eq!(run.reached, 1);
    let records = journal_records(&fx.root);
    assert_eq!(records.len(), 2, "opened, then closed: {records:?}");
    assert_eq!(records[0].outcome, audit::Outcome::Stored);
    assert_eq!(records[0].source, records[1].source);
}

/// A recovery unit that opened a run over a journal that refuses every
/// append refuses the unit at its close.
#[tokio::test]
async fn a_recovery_unit_whose_close_is_refused_is_an_error() {
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("recovery-noclose");
    std::fs::write(fx.root.join("audit"), b"not a directory").expect("a file where audit/ goes");
    let site = fx.serving(&script);
    let asked = spot(&format!("{DHAN_DAY}&member=NIFTY"));
    let seat = site.autopilot.take_seat(asked.feed).expect("a free seat");

    let refused = recovery_spot(&site, &asked, seat)
        .await
        .expect_err("a close that cannot land refuses the unit");
    assert!(refused.contains("audit"), "{refused}");
}

/// Mapping evidence that names every target is `Ok`.
#[test]
fn recovery_mapping_is_ok_when_every_target_maps() {
    let fx = Fixture::new("mapping-ok");
    let site = fx.refused();
    assert_eq!(recovery_mapping(&site, &spot(DHAN_DAY)), Ok(()));
}

// ---------------------------------------------------------- the pull routes

/// A named feed this build cannot read is refused by name, seat untaken.
#[tokio::test]
async fn a_hand_pull_naming_an_unknown_feed_is_refused_by_name() {
    let fx = Fixture::new("unknown-feed");
    let site = fx.refused();
    let (code, header, axum::response::Html(page)) = spot_pull_held(
        &site,
        "target=swept&vendor=dahn&from=2026-08-03&to=2026-08-05",
    )
    .await;
    assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(header[0].1, SPOT_RECEIPT);
    let said = facts(&page, "Refused because");
    assert_eq!(said.len(), 1, "{page}");
    assert!(
        said[0].starts_with(&render::escape(
            "\"dahn\" is not a feed this build has a descriptor for, so there is no seat to take"
        )),
        "{said:?}"
    );
    assert!(site.autopilot.take_seat(pull::vendor::Feed::Dhan).is_some());
}

/// A repair with a valid reason runs through the hand pull's path, detached,
/// and its receipt carries the repair line.
#[tokio::test]
async fn a_repair_with_a_reason_runs_the_hand_pull_path() {
    let fx = Fixture::new("repair-route");
    let site: Loaded = Arc::new(fx.refused());
    let body = format!("{DHAN_DAY}&repair_reason=wrong%20close");
    let (code, header, axum::response::Html(page)) =
        pull_repair(axum::extract::State(Loaded::clone(&site)), body).await;
    assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE, "{page}");
    assert_eq!(header[0].0.as_str(), RECEIPT_HEADER);
    assert_eq!(header[0].1, SPOT_RECEIPT);
    let landing = facts(&page, "Landing");
    assert_eq!(landing.len(), 1, "{page}");
    assert!(landing[0].ends_with("reason: wrong close"), "{landing:?}");
}

/// Percent-encodes every byte outside the unreserved set.
fn enc(raw: &str) -> String {
    raw.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// `POST /pull/run` claims the slot and answers 202 with the leg count; a
/// second press while it runs is refused 409, and the slot is not replaced.
#[tokio::test]
async fn a_pull_run_is_accepted_once_and_refused_while_it_runs() {
    let fx = Fixture::new("pull-run");
    let site: Loaded = Arc::new(fx.refused());
    let joined = format!("/pull/spot|dhan|1day|label|{}", enc(DHAN_DAY));
    let body = format!("leg={}", enc(&joined));

    let (code, headers, json) =
        pull_run(axum::extract::State(Loaded::clone(&site)), body.clone()).await;
    assert_eq!(code, axum::http::StatusCode::ACCEPTED, "{json}");
    assert_eq!(json, "{\"started\":true,\"legs\":1}");
    assert_eq!(
        headers
            .get(axum::http::header::CONTENT_TYPE)
            .map(|value| value.to_str().expect("ascii")),
        Some("application/json; charset=utf-8")
    );
    let generation = site
        .run
        .lock()
        .expect("the slot")
        .as_ref()
        .map(|progress| progress.generation)
        .expect("the slot is claimed");

    let (again, _, refusal) = pull_run(axum::extract::State(Loaded::clone(&site)), body).await;
    assert_eq!(again, axum::http::StatusCode::CONFLICT, "{refusal}");
    assert_eq!(
        refusal,
        run_refusal_json(&crate::pullrun::Refusal::AlreadyRunning.why())
    );
    assert_eq!(
        site.run
            .lock()
            .expect("the slot")
            .as_ref()
            .map(|progress| progress.generation),
        Some(generation),
        "the running claim is untouched"
    );
}

// ------------------------------------------------------------ small pieces

/// The vendor-down sentence names the streak, the place and the total.
#[test]
fn the_vendor_down_sentence_names_streak_place_and_total() {
    let said = vendor_down_sentence(3, 4, 9);
    assert!(
        said.starts_with(
            "the vendor's own side answered a 5xx on 3 consecutive instruments, so the run \
             stopped after 4 of 9 rather than asking the rest"
        ),
        "{said}"
    );
}

/// Which control stops which driver's run.
#[test]
fn the_stop_bit_is_named_per_driver() {
    assert_eq!(
        stop_bit_by(crate::autopilot::Driver::Hand),
        "Stop (POST /pull/run/stop) bit"
    );
    assert_eq!(
        stop_bit_by(crate::autopilot::Driver::Recovery),
        "the autopilot's pause bit"
    );
    assert_eq!(
        stop_bit_by(crate::autopilot::Driver::Autopilot),
        "the autopilot's pause bit"
    );
}

/// The recovery line for nothing in flight, and for an unreadable one.
#[test]
fn the_recovery_grounded_line_names_none_and_unreadable() {
    assert_eq!(
        recovery_grounded_line("read-only", Ok(None)),
        "recovery not resumed: read-only; no plan is in flight"
    );
    assert_eq!(
        recovery_grounded_line("read-only", Err("torn")),
        "recovery not resumed: read-only; what is in flight could NOT be read (torn) — \
         nothing was claimed or written"
    );
}

/// An empty active pointer cannot be read: a grounded start says so on its
/// banner line, and a start that may write refuses with the reason.
#[tokio::test]
async fn an_unreadable_recovery_pointer_grounds_or_refuses_the_start() {
    let fx = Fixture::new("recovery-start");
    let dir = fx.root.join("audit/recovery-v1");
    std::fs::create_dir_all(&dir).expect("recovery dir");
    std::fs::write(dir.join("active.bin"), b"").expect("an empty pointer");
    let site: Loaded = Arc::new(fx.refused());
    let why = "active recovery pointer is empty; prior activation is unverified";

    let grounded = recovery_at_start(
        &site,
        WriteGate {
            read_only: true,
            unstamped: None,
        },
    );
    assert_eq!(
        grounded,
        RecoveryStart::Grounded(format!(
            "recovery not resumed: read-only (BRUTEX_READ_ONLY=1); what is in flight could NOT \
             be read ({why}) — nothing was claimed or written"
        ))
    );
    let refused = recovery_at_start(
        &site,
        WriteGate {
            read_only: false,
            unstamped: None,
        },
    );
    assert_eq!(refused, RecoveryStart::Refused(why.to_owned()));
}

/// A store with no recorded recovery starts silently.
#[tokio::test]
async fn no_recorded_recovery_is_a_silent_start() {
    let fx = Fixture::new("recovery-silent");
    let site: Loaded = Arc::new(fx.refused());
    assert_eq!(
        recovery_at_start(
            &site,
            WriteGate {
                read_only: false,
                unstamped: None,
            },
        ),
        RecoveryStart::Silent
    );
}

// ------------------------------------------------------------ batch two

/// A hand Stop pressed while the first instrument is on the wire stops the
/// run at the next instrument, and the stop names the hand's control.
/// Covers `broker_run_by`'s stop arm and `stop_bit_by(Hand)` on the live path.
#[tokio::test]
async fn a_hand_stop_mid_run_stops_at_the_next_instrument() {
    let cell: Arc<std::sync::OnceLock<Loaded>> = Arc::new(std::sync::OnceLock::new());
    let pressed = Arc::clone(&cell);
    let vendor = FakeVendor::answering(move |_, _| {
        if let Some(site) = pressed.get() {
            let _epoch = site.autopilot.stop_hand();
        }
        (200, dhan_bar(AUG_03))
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("hand-stop");
    let site: Loaded = Arc::new(fx.serving(&script));
    assert!(cell.set(Loaded::clone(&site)).is_ok());

    let run = broker_run(&spot(DHAN_DAY), &site, &[]).await;

    assert_eq!(
        vendor.seen().len(),
        1,
        "the second instrument was never asked"
    );
    assert_eq!(run.reached, 1);
    assert_eq!(
        run.stopped.as_deref(),
        Some(
            "stopped by the operator — Stop (POST /pull/run/stop) bit after 1 of 2 instruments. \
             The partial month is refilled on resume, because the resume point is the store's own."
        )
    );
    assert!(
        run.total
            .failures
            .iter()
            .any(|failure| failure.instrument == "requested basket"),
        "a stopped basket is never certified: {:?}",
        run.total.failures
    );
}

/// A Pause while an autopilot run's first instrument is on the wire stops it
/// at the next instrument, naming the pause bit.
#[tokio::test]
async fn a_pause_mid_run_stops_an_autopilot_run_at_the_next_instrument() {
    let cell: Arc<std::sync::OnceLock<Loaded>> = Arc::new(std::sync::OnceLock::new());
    let pressed = Arc::clone(&cell);
    let vendor = FakeVendor::answering(move |_, _| {
        if let Some(site) = pressed.get() {
            site.autopilot.pause();
        }
        (200, dhan_bar(AUG_03))
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("pause-stop");
    let site: Loaded = Arc::new(fx.serving(&script));
    assert!(cell.set(Loaded::clone(&site)).is_ok());

    let run = broker_run_by(
        &spot(DHAN_DAY),
        &site,
        &[],
        crate::autopilot::Driver::Autopilot,
    )
    .await;

    assert_eq!(vendor.seen().len(), 1);
    assert_eq!(
        run.stopped.as_deref(),
        Some(
            "stopped by the operator — the autopilot's pause bit after 1 of 2 instruments. \
             The partial month is refilled on resume, because the resume point is the store's own."
        )
    );
}

/// A refusal the vendor NAMES as not-entitled is reported as a live
/// credential on an unentitled key, and is not retried (`with_retry`'s
/// `NotEntitled` arm).
#[tokio::test]
async fn a_named_not_entitled_refusal_is_alive_and_not_retried() {
    let vendor = FakeVendor::answering(|_, _| {
        (
            400,
            "{\"errorType\":\"Data_API_Error\",\"errorCode\":\"DH-902\",\
             \"errorMessage\":\"User has not subscribed to Data APIs\"}"
                .to_owned(),
        )
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("not-entitled");
    let site = fx.serving(&script);

    let run = broker_run(&spot(&format!("{DHAN_DAY}&member=NIFTY")), &site, &[]).await;

    assert_eq!(vendor.seen().len(), 1, "nothing was retried");
    assert!(
        !run.credential_dead,
        "an unentitled key is not a dead token"
    );
    assert_eq!(run.credential_stop, None);
    assert_eq!(run.refused.len(), 1, "{run:?}");
    assert!(
        run.refused[0].contains(
            "the credential is ALIVE and this API key is not entitled to this call. This is not \
             an expired session and re-running later cannot fix it"
        ),
        "{:?}",
        run.refused
    );
    assert!(run.touched_wire);
}

/// A throttle the vendor names under a status the transport does not read
/// (400) is recorded against the shared governor exactly once, and the chunk
/// is re-asked and lands (`with_retry`'s named-throttle decrease).
#[tokio::test]
async fn a_named_throttle_under_a_400_halves_the_shared_governor_once() {
    let vendor = FakeVendor::answering(|nth, _| {
        if nth == 0 {
            (
                400,
                "{\"errorType\":\"Data_API_Error\",\"errorCode\":\"DH-904\",\
                 \"errorMessage\":\"Too many requests\"}"
                    .to_owned(),
            )
        } else {
            (200, dhan_bar(AUG_03))
        }
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("named-throttle");
    let site = fx.serving(&script);
    let day = |site: &Site| {
        site.budgets.lock().expect("budgets")[pull::vendor::Feed::Dhan as usize]
            .as_ref()
            .expect("Dhan has a budget")
            .lock()
            .expect("the governor")
            .permitted(pull::rate::WindowSpan::Day)
    };
    assert_eq!(
        day(&site),
        Some(100_000),
        "the premise: a full day allowance"
    );

    let run = broker_run(&spot(&format!("{DHAN_DAY}&member=NIFTY")), &site, &[]).await;

    assert_eq!(vendor.seen().len(), 2, "throttled once, then answered");
    assert_eq!(run.reached, 1, "{run:?}");
    assert_eq!(run.total.bars_committed, 1);
    // THE SAME GOVERNOR, REPLAYED BY HAND: one decrease, then one success.
    let expected = {
        let fresh = feed_budgets();
        let mut governor = fresh[pull::vendor::Feed::Dhan as usize]
            .as_ref()
            .expect("Dhan has a budget")
            .lock()
            .expect("the governor");
        governor.record_throttled();
        governor.record_success();
        governor.permitted(pull::rate::WindowSpan::Day)
    };
    assert_ne!(expected, Some(100_000), "the premise: a decrease shows");
    assert_eq!(
        day(&site),
        expected,
        "decreased once by the named throttle, then one success added back"
    );
}

/// `laddered` reports a refusal NAMED not-entitled as a live credential, and
/// asks once.
#[tokio::test]
async fn laddered_reports_not_entitled_alive_and_asks_once() {
    let fx = Fixture::new("ladder-entitled");
    let site = fx.refused();
    let asks = std::sync::atomic::AtomicUsize::new(0);
    let refused = laddered(pull::vendor::Feed::Dhan, &site, "discovery call", || {
        let _ = asks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async {
            Err(pull::chain::Refusal {
                status: Some(403),
                detail: "the vendor answered 403".to_owned(),
                credential_dead: false,
                named: Some(pull::refusal::Disposition::NotEntitled),
            })
        }
    })
    .await
    .expect_err("an unentitled key is refused");
    assert_eq!(asks.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(!refused.credential_dead);
    assert_eq!(refused.status, Some(403));
    assert_eq!(refused.named, Some(pull::refusal::Disposition::NotEntitled));
    assert_eq!(
        refused.detail,
        "the vendor answered 403 — the credential is ALIVE and this API key is not entitled to \
         this call. Re-running later cannot fix it; only a change to the key's subscription \
         changes the answer. Nothing was retried."
    );
}

/// `laddered` marks a refusal the vendor names as its own backend failure
/// VENDOR DOWN once the ladder is spent, and says how many 5xx it counted.
#[tokio::test]
async fn laddered_marks_a_spent_backend_ladder_vendor_down() {
    let fx = Fixture::new("ladder-down");
    let site = fx.refused();
    let asks = std::sync::atomic::AtomicUsize::new(0);
    let started = std::time::Instant::now();
    let refused = laddered(pull::vendor::Feed::Dhan, &site, "discovery call", || {
        let _ = asks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async {
            Err(pull::chain::Refusal {
                status: Some(400),
                detail: "the vendor answered 400".to_owned(),
                credential_dead: false,
                named: Some(pull::refusal::Disposition::RetryBounded),
            })
        }
    })
    .await
    .expect_err("a backend that never answers is refused");
    // SIX PERMITS AGAINST FIVE A SECOND: the sixth is WAITED for, so the
    // ladder cannot finish at once (kills `> -> ==` and `> -> <` at
    // `await_budget`:7174, which would never sleep).
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(100),
        "the governor's named wait was slept: {:?}",
        started.elapsed()
    );
    assert_eq!(asks.load(std::sync::atomic::Ordering::SeqCst), 6);
    assert!(!refused.credential_dead);
    assert_eq!(refused.status, Some(400));
    assert_eq!(
        refused.detail,
        format!(
            "{VENDOR_DOWN}the vendor answered 400 — and its own side has now failed 0 time(s) on \
             this discovery call, out of 5 allowed. The vendor is reachable and failing, which is \
             not a blip this walk can wait out."
        )
    );
}

/// A ledger with no thread and no opening record closes with a plain append;
/// `flush` does nothing without a window or a thread; `rest` on an unopened
/// run writes each failure as its own member record.
#[test]
fn an_unopened_ledger_closes_and_rests_by_plain_append() {
    let fx = Fixture::new("ledger-plain");
    let journal = audit::Journal::at(&fx.root);
    let window = spot(DHAN_DAY).window;
    let now = std::time::SystemTime::now();
    let failures = [pull::ingest::Failure::new("NIFTY", "the vendor refused it")];

    let mut bare = RunLedger::default();
    let close = audit::Record::refused(
        audit::Scope::Spot,
        audit::Outcome::NotStarted,
        now,
        "src",
        "why",
    );
    assert_eq!(bare.close(&journal, &close), Ok(()));
    assert_eq!(journal_records(&fx.root).len(), 1);
    bare.flush(&journal, &failures);
    assert_eq!(bare.written, 0, "no window, nothing flushed");

    let mut unopened = RunLedger {
        window: Some(window),
        ..RunLedger::default()
    };
    unopened.flush(&journal, &failures);
    assert_eq!(unopened.written, 0, "no thread, nothing flushed");
    unopened.rest(&journal, now, &failures);
    assert_eq!(unopened.written, 1);
    assert_eq!(unopened.refused, None);
    let records = journal_records(&fx.root);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].kind, audit::Kind::MemberFailure);
    assert_eq!(records[0].source, "NIFTY");
    assert_eq!(
        unopened.fact(1),
        (
            "Failures recorded",
            "1 of 1 — each on its own record after this run's.".to_owned()
        )
    );
    let mut bare_rest = RunLedger::default();
    bare_rest.rest(&journal, now, &failures);
    assert_eq!(bare_rest.written, 0, "no window, nothing rested");
}

/// The failures line says when the opening record was refused, and when the
/// rest are not on disk.
#[test]
fn the_failures_line_names_a_refused_opening_and_a_short_write() {
    let refused_open = RunLedger {
        opened_refused: Some("disk full".to_owned()),
        written: 1,
        refused: Some("busy".to_owned()),
        ..RunLedger::default()
    };
    assert_eq!(
        refused_open.fact(3),
        (
            "Failures recorded",
            "1 of 3 — the rest are NOT on disk: busy. The run's opening record was refused, so \
             none was written during the run: disk full"
                .to_owned()
        )
    );
    // A refusal still waiting with everything written is not a short write.
    let whole = RunLedger {
        written: 2,
        refused: Some("busy".to_owned()),
        ..RunLedger::default()
    };
    assert_eq!(
        whole.fact(2),
        (
            "Failures recorded",
            "2 of 2 — each on its own record after this run's.".to_owned()
        )
    );
}

/// The ask of a repair differs from a pull's by exactly the word `repair`.
#[test]
fn a_repair_asks_a_different_digest_than_a_pull() {
    let pull = spot(DHAN_DAY);
    let repair = spot(&format!("{DHAN_DAY}&repair_reason=fix"));
    let parts = |word: &'static str| {
        audit::ask_digest(&["spot", "Dhan", "Isin", "Swept", "1day", word, "0"])
    };
    assert_eq!(spot_ask(&pull), parts("pull"));
    assert_eq!(spot_ask(&repair), parts("repair"));
    assert_ne!(spot_ask(&pull), spot_ask(&repair));
}

/// An archive feed has no endpoint to plan.
#[test]
fn an_archive_feed_plans_no_origin() {
    let asked = spot("target=swept&vendor=gdfl&from=2026-08-03&to=2026-08-05");
    assert!(matches!(
        asked.feed.descriptor().transport,
        pull::vendor::Transport::LocalArchive(_)
    ));
    assert_eq!(planned_origin(&asked), "");
    assert!(
        planned_origin(&spot(DHAN_DAY)).starts_with("https://api.dhan.co/v2/charts/historical")
    );
}

/// A failure about a month is filed under that month's days inside the
/// window: clamped at the window's start and at the month's own end.
#[test]
fn a_month_failure_is_filed_under_its_days_inside_the_window() {
    let window = pull::session::Window::new(
        Day::new(2026, 7, 15).expect("a day"),
        Day::new(2026, 8, 20).expect("a day"),
    )
    .expect("a window");
    let failure = pull::ingest::Failure {
        instrument: "NIFTY".to_owned(),
        why: "short".to_owned(),
        month: Some(store::path::YearMonth::new(2026, 7).expect("a month")),
        day: None,
    };
    assert_eq!(
        failure_window(&failure, window),
        pull::session::Window::new(
            Day::new(2026, 7, 15).expect("a day"),
            Day::new(2026, 7, 31).expect("a day"),
        )
        .expect("a window")
    );
    // A MONTH THAT BEGINS INSIDE THE WINDOW starts at its own first day and
    // ends at the window's end (kills `> -> ==` at 4460 and `< -> >` at 4466).
    let august = pull::ingest::Failure {
        month: Some(store::path::YearMonth::new(2026, 8).expect("a month")),
        ..failure
    };
    assert_eq!(
        failure_window(&august, window),
        pull::session::Window::new(
            Day::new(2026, 8, 1).expect("a day"),
            Day::new(2026, 8, 20).expect("a day"),
        )
        .expect("a window")
    );
}

/// The receipt names how many instruments the pull order withheld, and adds
/// the retry class only when a failure exists.
#[test]
fn the_order_and_retry_facts_name_the_withheld_first() {
    let mut run = BrokerRun {
        withheld: 2,
        ..BrokerRun::default()
    };
    let facts = run.order_and_retry_facts();
    assert_eq!(
        facts,
        vec![(
            "Withheld by the pull order",
            "2 instrument(s) owe months at the rung below and were not asked for; each is named \
             under Failed and in the journal. The rest were pulled."
                .to_owned()
        )]
    );
    run.record_refusal("NIFTY", "refused".to_owned());
    let facts = run.order_and_retry_facts();
    assert_eq!(facts.len(), 2);
    assert_eq!(facts[1].0, crate::pullrun::RETRY_CLASS_FACT);
    assert!(BrokerRun::default().order_and_retry_facts().is_empty());
}

/// A futures instrument asked before spot has finished is refused by the leg
/// order, naming spot and the day rung.
#[test]
fn a_futures_target_before_spot_finishes_is_refused_by_the_leg_order() {
    let asked = spot(DHAN_DAY);
    let future = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Fno,
        underlying: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
        kind: brutex_core::instrument::Kind::Future {
            expiry: brutex_core::instrument::Expiry::new(2026, 8, 27).expect("an expiry"),
        },
    };
    let said = ladder_refusal(&asked, &[future], &[]).expect("the leg order refuses");
    assert_eq!(
        said,
        format!(
            "spot comes first, and it has to FINISH: 2 of 2 instrument-months have no spot bars \
             behind them at {}. Nothing was asked for. The order is spot, then expired futures, \
             then expired options — and each of the three completes BOTH its day pass and its \
             minute pass before the next begins, because a leg that is only half pulled cannot \
             be checked against anything.",
            store::path::Timeframe::DAY_1.as_str()
        )
    );
}

/// The pull order withholds nothing for an archive feed, or for a rung the
/// store has no directory for.
#[test]
fn the_pull_order_withholds_nothing_ungated_or_unfiled() {
    let archive = spot("target=swept&vendor=gdfl&from=2026-08-03&to=2026-08-05");
    let nifty = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Index,
        underlying: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
        kind: brutex_core::instrument::Kind::Index,
    };
    assert!(ladder_withheld(&archive, &[nifty], &[]).is_empty());
    let mut seconds = spot(DHAN_DAY);
    seconds.granularity = pull::vendor::Granularity::Hour1;
    assert_eq!(seconds.granularity.store_timeframe(), None);
    assert!(ladder_withheld(&seconds, &[nifty], &[]).is_empty());
}

/// A floor that names no day narrows nothing, and a window wholly below a
/// fixed floor is refused by name.
#[test]
fn clamping_to_a_floor_passes_unbounded_and_refuses_below() {
    let window = spot(DHAN_DAY).window;
    for floor in [
        pull::vendor::HistoryFloor::Unstated,
        pull::vendor::HistoryFloor::Unbounded,
    ] {
        assert_eq!(clamp_to_floor(window, floor, aug_10()), Ok(window));
    }
    assert_eq!(
        clamp_to_floor(
            window,
            pull::vendor::HistoryFloor::Fixed {
                year: 2026,
                month: 9,
                day: 1
            },
            aug_10()
        ),
        Err(
            "this feed's history starts at 2026-09-01 and the window ends 2026-08-05. Every day \
             asked for is older than the vendor holds — it would answer empty, which reads \
             exactly like a market holiday and would be recorded as nothing to store rather than \
             nothing available."
                .to_owned()
        )
    );
}

/// A window that ends after today is refused before any socket, with the
/// refusal's own sentence.
#[test]
fn a_window_past_today_is_refused_before_the_socket() {
    let asked = ingest::parse_spot(
        "target=swept&vendor=dhan&granularity=1day&from=2099-12-01&to=2099-12-31",
        Day::new(2100, 1, 10).expect("a day"),
    )
    .expect("a window that is past from 2100's view");
    let today = ingest::ist_day(std::time::SystemTime::now()).expect("a clock");
    assert_eq!(
        finished_day_only(&asked),
        Err(ingest::Refusal::WindowReachesToday {
            to: asked.window.to(),
            today,
        }
        .to_string())
    );
}

/// A budget lock poisoned by a panic refuses the request on an unknown
/// allowance instead of issuing it.
#[tokio::test]
async fn a_poisoned_budget_refuses_rather_than_issues() {
    let fx = Fixture::new("poisoned-budget");
    let site = fx.refused();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _held = site.budgets.lock().expect("not yet poisoned");
        panic!("poison the budget lock");
    }));
    assert!(poisoned.is_err());
    assert_eq!(
        await_budget(pull::vendor::Feed::Dhan, &site).await,
        Err(
            "the rate budget for Dhan cannot be read: another request panicked while holding \
             it, so the allowance already spent is unknown. Nothing is issued on an unknown \
             budget."
                .to_owned()
        )
    );
}

/// No lane answering for an instrument is a named refusal, not a panic; a
/// fetched-ahead refusal reached the wire only when it carries the marker.
#[tokio::test]
async fn the_lanes_name_a_missing_answer_and_read_the_wire_marker() {
    let fx = Fixture::new("lanes");
    let site = fx.refused();
    let mut lanes = Lanes::default();
    let missing = lanes.next(&spot(DHAN_DAY), &[], 0, &site).await;
    assert_eq!(
        missing.err().as_deref(),
        Some("instrument 1 of 0: no lane answered for it")
    );
    assert!(!lanes.wide, "a refusal before the wire proves nothing");
    assert!(!lanes.reached_wire(), "nothing fetched ahead");
    lanes
        .ahead
        .push_back(Err("refused before the socket".to_owned()));
    assert!(!lanes.reached_wire());
    lanes
        .ahead
        .push_back(Err(format!("{WIRE_REACHED}the vendor said no")));
    assert!(lanes.reached_wire());
}

/// An archive feed that reaches the broker window is a routing error, named.
#[tokio::test]
async fn an_archive_feed_at_the_broker_window_is_a_routing_error() {
    let fx = Fixture::new("archive-window");
    let site = fx.refused();
    let asked = spot("target=swept&vendor=gdfl&from=2026-08-03&to=2026-08-05");
    let watch = std::sync::Mutex::new(crate::credential_law::Watch::default());
    let refused = broker_window(
        &asked,
        &brutex_core::instrument::InstrumentKey {
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Index,
            underlying: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
            kind: brutex_core::instrument::Kind::Index,
        },
        &site,
        &watch,
    )
    .await
    .err()
    .expect("refused");
    assert_eq!(
        refused,
        format!(
            "{} declares a local-archive transport, not an HTTP one. Reaching this function at \
             all is a routing error rather than an operator one — the transport is what chooses \
             the path.",
            asked.feed.display()
        )
    );
}

/// A journal that refuses every append: the run's line says NO and the
/// failures line says none of them is on disk.
#[test]
fn recorded_with_failures_names_the_ones_not_on_disk() {
    let fx = Fixture::new("failures-refused");
    std::fs::write(fx.root.join("audit"), b"not a directory").expect("a file where audit/ goes");
    let journal = audit::Journal::at(&fx.root);
    let window = spot(DHAN_DAY).window;
    let now = std::time::SystemTime::now();
    let record = audit::Record::refused(
        audit::Scope::Spot,
        audit::Outcome::Failed,
        now,
        "src",
        "why",
    );
    let failures = [
        pull::ingest::Failure::new("NIFTY", "one"),
        pull::ingest::Failure::new("BANKNIFTY", "two"),
    ];
    let facts = recorded_with_failures(
        &journal,
        &record,
        audit::Scope::Spot,
        now,
        window,
        &failures,
    );
    assert_eq!(facts.len(), 2);
    assert!(
        facts[0]
            .1
            .starts_with("NO — this run is NOT in the journal."),
        "{facts:?}"
    );
    assert_eq!(facts[1].0, "Failures recorded");
    assert!(
        facts[1]
            .1
            .starts_with("0 of 2 — the rest are NOT on disk: "),
        "{facts:?}"
    );
}

/// An HTTP feed handed to the archive reader is refused by name.
#[test]
fn an_http_feed_has_no_archive_layout_to_read() {
    let fx = Fixture::new("run-local-http");
    let window = spot(DHAN_DAY).window;
    assert_eq!(
        run_local(
            pull::vendor::Feed::Dhan,
            "/nowhere",
            window,
            pull::vendor::Granularity::Minute1,
            &fx.root,
        )
        .err()
        .as_deref(),
        Some("Dhan declares no archive, so there is no column layout to read one with.")
    );
}

/// A run whose rows are not all accounted for, with no failure, says the
/// books do not balance and gives the figures.
#[test]
fn an_unbalanced_run_says_the_books_do_not_balance() {
    let fx = Fixture::new("unbalanced");
    let journal = audit::Journal::at(&fx.root);
    let done = pull::ingest::Ingested {
        rows_read: 5,
        ..pull::ingest::Ingested::default()
    };
    let (code, page) = landed_answer(
        &done,
        spot(DHAN_DAY).window,
        std::time::SystemTime::now(),
        "src",
        &journal,
        Vec::new(),
        0,
        None,
    );
    assert_eq!(code, axum::http::StatusCode::OK);
    assert!(
        page.contains(&render::escape(
            "THE RUN FINISHED AND THE BOOKS DO NOT BALANCE. Rows in does not equal bars out plus \
             rows folded plus rows dropped, or a member failed."
        )),
        "{page}"
    );
    assert_eq!(
        facts(&page, "Balances"),
        [render::escape(
            "NO — 5 rows read, 0 stored, 0 folded, 0 dropped, 0 failure diagnostics"
        )]
    );
}

/// BANKNIFTY's spot minute window reads BANKNIFTY's calendar, and an empty
/// store holds no census to read one from.
#[test]
fn a_banknifty_minute_window_with_no_census_observes_nothing() {
    let fx = Fixture::new("observe-banknifty");
    let site = fx.refused();
    let landed = window_of("BANKNIFTY", "INDEX", pull::vendor::Granularity::Minute1);
    assert!(ingestion_observations(&landed, &site).is_none());
}

/// A store root whose `audit/` cannot be created is the store's fault, named.
#[test]
fn a_store_whose_audit_dir_cannot_be_made_is_at_fault() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = scratch("store-fault-ro");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o555)).expect("read-only");
    let fault = store_fault(&root);
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).expect("writable");
    let _ = std::fs::remove_dir_all(&root);
    let fault = fault.expect("a read-only root takes no audit dir");
    assert!(
        fault.starts_with(&format!("{}: ", root.join("audit").display())),
        "{fault}"
    );
    assert!(fault.contains("Permission denied"), "{fault}");
}

/// A cash schedule is refused for a window whose identity is not an NSE cash
/// source, and is not needed for a rung with no committed-day reread and no
/// observed day.
#[tokio::test]
async fn the_cash_schedule_refuses_a_non_cash_identity_and_skips_an_empty_one() {
    let fx = Fixture::new("cash-schedule");
    let site = fx.refused();
    let reliance = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Cash,
        underlying: brutex_core::symbol::Symbol::new("RELIANCE").expect("a symbol"),
        kind: brutex_core::instrument::Kind::Equity,
    };
    let mut dated = std::collections::HashMap::new();
    let mut wrong = window_of("RELIANCE", "INDEX", pull::vendor::Granularity::Minute1);
    wrong.listing = pull::vendor::Listing::Equity;
    assert_eq!(
        prepare_cash_schedule(&wrong, &[], &reliance, &site, &mut dated)
            .await
            .err()
            .as_deref(),
        Some("cash schedule requires the exact NSE cash source identity")
    );
    let mut five = window_of("RELIANCE", "CASH", pull::vendor::Granularity::Minute5);
    five.listing = pull::vendor::Listing::Equity;
    assert_eq!(
        prepare_cash_schedule(&five, &[], &reliance, &site, &mut dated)
            .await
            .map(|schedule| schedule.is_none()),
        Ok(true)
    );
}

/// A spec whose feed-wide floor is `floor`, Dhan's in every other field.
fn spec_with_floor(floor: pull::vendor::HistoryFloor) -> pull::vendor::HttpSpec {
    pull::vendor::HttpSpec {
        history_floor: floor,
        ..dhan_spec()
    }
}

/// A rung the feed's own table leaves unstated falls back to the spec's
/// feed-wide floor (`rung_floor`'s `Unstated` arm). Only the archive feeds
/// leave a served rung unstated, so one of them is asked.
#[test]
fn an_unstated_rung_floor_falls_back_to_the_specs() {
    let asked = spot("target=swept&vendor=truedata&granularity=1day&from=2026-08-03&to=2026-08-05");
    assert_eq!(
        asked.feed.descriptor().history_floor(asked.granularity),
        pull::vendor::HistoryFloor::Unstated,
        "the premise"
    );
    let floor = pull::vendor::HistoryFloor::Fixed {
        year: 2019,
        month: 4,
        day: 1,
    };
    assert_eq!(rung_floor(&asked, &spec_with_floor(floor)), floor);
    // A STATED rung keeps its own floor whatever the spec says.
    let dhan = spot(DHAN_DAY);
    assert_eq!(
        rung_floor(&dhan, &spec_with_floor(floor)),
        dhan.feed.descriptor().history_floor(dhan.granularity)
    );
}

/// With no dated floor nothing bounds how far back an earlier bar could be
/// held, so the empty chunks before the first row stay a failure.
#[test]
fn no_dated_floor_cannot_show_nothing_is_held_earlier() {
    let fx = Fixture::new("no-floor");
    let site = fx.refused();
    let asked = spot("target=swept&vendor=truedata&granularity=1day&from=2026-07-01&to=2026-08-05");
    let chunks = Chunks {
        bodies: Vec::new(),
        unfetched: None,
        hollow: vec![
            pull::session::Window::new(
                Day::new(2026, 7, 1).expect("a day"),
                Day::new(2026, 7, 31).expect("a day"),
            )
            .expect("a window"),
        ],
        leading: 1,
        total: 2,
    };
    assert_eq!(
        nothing_held_before_first_bar(
            &asked,
            &site,
            &spec_with_floor(pull::vendor::HistoryFloor::Unbounded),
            &brutex_core::instrument::InstrumentKey {
                exchange: brutex_core::instrument::Exchange::Nse,
                segment: brutex_core::instrument::Segment::Index,
                underlying: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
                kind: brutex_core::instrument::Kind::Index,
            },
            Vendor::Dhan,
            &chunks,
        ),
        Err(
            "the feed states no dated history floor for this rung, so no bound limits how far \
             back an earlier bar could be held"
                .to_owned()
        )
    );
}

/// A census recovered from damage cannot show that nothing is held, because
/// a month it lost could hold an earlier bar.
#[test]
fn a_damaged_census_cannot_show_nothing_is_held() {
    let fx = Fixture::new("damaged-census");
    let mut damaged = pull::manifest::ManifestHeader::genesis(Vendor::Dhan).image();
    damaged[16] ^= 0xFF;
    let good = pull::manifest::ManifestHeader {
        generation: 1,
        ..pull::manifest::ManifestHeader::genesis(Vendor::Dhan)
    }
    .image();
    let mut bytes = vec![0u8; 32_768];
    bytes[..64].copy_from_slice(&damaged);
    bytes[16_384..16_448].copy_from_slice(&good);
    std::fs::write(
        pull::manifest::manifest_path(&fx.root, Vendor::Dhan),
        &bytes,
    )
    .expect("a damaged manifest");
    let census = census::read_vendor(&fx.root, Vendor::Dhan);
    let census::Census::Held { ref manifest } = census.state else {
        panic!("a recovered census still loads: {:?}", census.state.name());
    };
    let why = manifest.degraded_reason().expect("the premise: degraded");
    let month = store::path::YearMonth::new(2026, 7).expect("a month");
    let held = store_holds_nothing_in(&census, &[month], |month| pull::manifest::EntryKey {
        contract: None,
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Index,
        symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
        timeframe: store::path::Timeframe::DAY_1,
        month,
    });
    assert_eq!(
        held,
        Err(format!(
            "the store's census was recovered from damage ({why}), so a month it lost could \
             hold an earlier bar"
        ))
    );
}

// ---------------------------------------------------------- batch three

/// A 5xx that the next attempt answers leaves the shared governor where it
/// was: a backend failure names no budget (kills `&& -> ||` at
/// `with_retry`:8433, which would record a decrease for an unnamed 5xx).
#[tokio::test]
async fn a_5xx_then_an_answer_leaves_the_governor_undecreased() {
    let vendor = FakeVendor::answering(|nth, _| {
        if nth == 0 {
            (503, "busy".to_owned())
        } else {
            (200, dhan_bar(AUG_03))
        }
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("5xx-then-ok");
    let site = fx.serving(&script);

    let run = broker_run(&spot(&format!("{DHAN_DAY}&member=NIFTY")), &site, &[]).await;

    assert_eq!(vendor.seen().len(), 2, "re-asked once after the 5xx");
    assert_eq!(run.reached, 1, "{run:?}");
    assert_eq!(run.total.bars_committed, 1);
    let day = site.budgets.lock().expect("budgets")[pull::vendor::Feed::Dhan as usize]
        .as_ref()
        .expect("Dhan has a budget")
        .lock()
        .expect("the governor")
        .permitted(pull::rate::WindowSpan::Day);
    assert_eq!(day, Some(100_000), "no decrease for a backend failure");
}

/// A leading chunk that answered with no rows, on a store whose census
/// cannot show nothing is held earlier, stays a failure and is not taken as
/// the span before the first bar (`broker_window`'s leading-hollow arm).
#[tokio::test]
async fn a_hollow_leading_chunk_with_no_census_stays_a_failure() {
    let vendor = FakeVendor::answering(|nth, _| {
        if nth == 0 {
            (
                200,
                "{\"open\":[],\"high\":[],\"low\":[],\"close\":[],\"volume\":[],\
                 \"timestamp\":[]}"
                    .to_owned(),
            )
        } else {
            (200, dhan_bar(AUG_03))
        }
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("hollow-lead");
    let site = fx.serving(&script);
    let asked = spot(
        "target=swept&vendor=dhan&granularity=1day&from=2026-07-01&to=2026-08-05&member=NIFTY",
    );

    let run = broker_run(&asked, &site, &[]).await;

    assert_eq!(vendor.seen().len(), 2);
    assert_eq!(run.reached, 1, "{run:?}");
    assert_eq!(run.total.bars_committed, 1, "August landed");
    assert!(
        run.before_first_bar.is_empty(),
        "{:?}",
        run.before_first_bar
    );
    let short: Vec<&str> = run
        .total
        .failures
        .iter()
        .map(|failure| failure.why.as_str())
        .filter(|why| why.contains("NO ROWS AT ALL"))
        .collect();
    assert_eq!(short.len(), 1, "{:?}", run.total.failures);
    assert!(
        short[0].contains(
            "The leading empty chunks precede the vendor's first row, but are not taken as the \
             span before the first bar: the store's census is "
        ),
        "{short:?}"
    );
}

/// An instrument the universe cannot name at the fetch boundary is refused
/// there, after the credential and before any socket.
#[tokio::test]
async fn a_target_missing_from_the_universe_is_refused_at_the_fetch_boundary() {
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("unmapped-target");
    let site = fx.serving(&script);
    let watch = std::sync::Mutex::new(crate::credential_law::Watch::default());
    let refused = broker_window(
        &spot(DHAN_DAY),
        &brutex_core::instrument::InstrumentKey {
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Index,
            underlying: brutex_core::symbol::Symbol::new("FINNIFTY").expect("a symbol"),
            kind: brutex_core::instrument::Kind::Index,
        },
        &site,
        &watch,
    )
    .await
    .err()
    .expect("refused");
    assert_eq!(refused, "FINNIFTY: dhan has no unambiguous vendor ID");
    assert!(vendor.seen().is_empty(), "no socket");
    assert_eq!(script.reads(), 1, "the credential was read first");
}

/// The past-record line needs a window to name; with none, nothing is said.
#[test]
fn the_past_record_line_needs_a_window() {
    let mut run = BrokerRun {
        past_record: 2,
        ..BrokerRun::default()
    };
    run.record_past_record();
    assert!(run.total.failures.is_empty());
    assert_eq!(run.retry.standing, 0);
}

/// A grounded start with nothing in flight says so, and resumes nothing.
#[tokio::test]
async fn a_grounded_start_with_nothing_in_flight_says_so() {
    let fx = Fixture::new("grounded-none");
    let site: Loaded = Arc::new(fx.refused());
    assert_eq!(
        recovery_at_start(
            &site,
            WriteGate {
                read_only: true,
                unstamped: None,
            },
        ),
        RecoveryStart::Grounded(
            "recovery not resumed: read-only (BRUTEX_READ_ONLY=1); no plan is in flight".to_owned()
        )
    );
}

/// A budget that becomes unreadable between two chunks keeps the chunk
/// already fetched and names why the rest is missing (`fetch_chunks`' budget
/// discard site).
#[tokio::test]
async fn a_budget_lost_between_chunks_keeps_the_prefix() {
    let cell: Arc<std::sync::OnceLock<Loaded>> = Arc::new(std::sync::OnceLock::new());
    let held = Arc::clone(&cell);
    let vendor = FakeVendor::answering(move |_, _| {
        if let Some(site) = held.get() {
            std::thread::scope(|scope| {
                let poisoner = scope.spawn(|| {
                    let _budgets = site.budgets.lock().expect("not yet poisoned");
                    panic!("poison the budget lock mid-window");
                });
                assert!(poisoner.join().is_err());
            });
        }
        (200, dhan_bar(JUL_01))
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("budget-lost");
    let site: Loaded = Arc::new(fx.serving(&script));
    assert!(cell.set(Loaded::clone(&site)).is_ok());
    let asked = spot(
        "target=swept&vendor=dhan&granularity=1day&from=2026-07-01&to=2026-08-05&member=NIFTY",
    );

    let run = broker_run(&asked, &site, &[]).await;

    assert_eq!(vendor.seen().len(), 1, "the second chunk was never sent");
    assert_eq!(run.reached, 1, "{run:?}");
    assert_eq!(run.total.bars_committed, 1, "the July prefix landed");
    assert!(
        run.total.failures.iter().any(|failure| failure
            .why
            .contains("the rate budget for Dhan cannot be read")),
        "{:?}",
        run.total.failures
    );
}

/// The seat's holder is named from the run slot: the run's generation, and
/// its leg when it is on one.
#[test]
fn the_seat_holder_names_the_hand_run_and_its_leg() {
    let fx = Fixture::new("seat-holder");
    let site = fx.refused();
    let mut progress = crate::pullrun::Progress::claimed();
    let generation = progress.generation;
    progress.feeds.push(crate::pullrun::FeedReport {
        vendor: "dhan".to_owned(),
        ..crate::pullrun::FeedReport::default()
    });
    *site.run.lock().expect("the slot") = Some(progress);
    assert_eq!(
        seat_holder(&site, pull::vendor::Feed::Dhan),
        format!(
            "It is held by the hand run started on /ingest (POST /pull/run, run {generation}). \
             Its Stop (POST /pull/run/stop) stops it at its next instrument; pausing the \
             autopilot does not touch it."
        )
    );
    site.run
        .lock()
        .expect("the slot")
        .as_mut()
        .expect("a run")
        .feeds[0]
        .doing = "spot 1day".to_owned();
    assert_eq!(
        seat_holder(&site, pull::vendor::Feed::Dhan),
        format!(
            "It is held by the hand run started on /ingest (POST /pull/run, run {generation}), \
             now on its leg \"spot 1day\". Its Stop (POST /pull/run/stop) stops it at its next \
             instrument; pausing the autopilot does not touch it."
        )
    );
}

/// Under the NSE-checked policy, a member with no NSE ISIN (an index) keeps
/// the generic unresolved sentence.
#[test]
fn an_index_member_keeps_the_generic_unresolved_sentence() {
    let fx = Fixture::new("unresolved-index");
    let site = fx.refused();
    let asked = spot(
        "target=swept&vendor=zerodha&granularity=1day&from=2026-08-03&to=2026-08-05\
         &cash_identity=zerodha_nse_checked",
    );
    assert_eq!(
        unresolved_member(&asked, &site.universe().read, "NIFTY"),
        "NIFTY: no unambiguous vendor ID in the requested target"
    );
}

/// The pull order withholds, by name, an instrument whose day months are
/// owed, and pulls the rest (DB-B27): the run is opened with the withheld
/// instrument already on it as a standing failure.
#[tokio::test]
async fn the_pull_order_withholds_an_instrument_that_owes_its_day_pass() {
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("withheld");
    let site = fx.serving(&script);
    // NIFTY's day pass first, so only BANKNIFTY owes.
    let day = broker_run(&spot(&format!("{DHAN_DAY}&member=NIFTY")), &site, &[]).await;
    assert_eq!(day.total.bars_committed, 1, "the premise: {day:?}");
    let censuses = census::read_all(&fx.root);
    let minute = spot("target=swept&vendor=dhan&granularity=1min&from=2026-08-03&to=2026-08-05");

    let run = broker_run(&minute, &site, &censuses).await;

    assert!(run.blocked.is_none(), "{run:?}");
    assert_eq!(run.withheld, 1, "{run:?}");
    assert_eq!(run.attempted, 2, "the withheld one is still expected");
    let withheld: Vec<&pull::ingest::Failure> = run
        .total
        .failures
        .iter()
        .filter(|failure| failure.why.starts_with(WITHHELD_BY_ORDER))
        .collect();
    assert_eq!(withheld.len(), 1, "{:?}", run.total.failures);
    assert_eq!(withheld[0].instrument, "BANKNIFTY");
    assert_eq!(
        withheld[0].month,
        Some(store::path::YearMonth::new(2026, 8).expect("a month")),
        "filed under the one month the window lies in"
    );
    assert_eq!(
        vendor.seen().len(),
        2,
        "one day request, one minute request"
    );
    let facts = run.order_and_retry_facts();
    assert_eq!(facts[0].0, "Withheld by the pull order");
}

/// Three instruments whose bars the store will not take, with the probe write
/// refused too, trip the store breaker: the run stops asking the vendor and
/// says why (`broker_run_by`'s store-breaker exit).
#[tokio::test]
async fn a_store_that_takes_no_write_trips_the_store_breaker() {
    use std::os::unix::fs::PermissionsExt as _;
    let vendor = FakeVendor::answering(|_, _| (200, dhan_bar(AUG_03))).await;
    let script = live(&vendor, "live");
    let fx = Fixture::four_indices("store-breaker");
    // THE BARS HAVE NOWHERE TO GO, AND THE PROBE HAS NOWHERE TO WRITE.
    std::fs::write(fx.root.join("bars"), b"not a directory").expect("a file where bars/ goes");
    let audit_dir = fx.root.join("audit");
    std::fs::create_dir_all(&audit_dir).expect("audit dir");
    std::fs::set_permissions(&audit_dir, std::fs::Permissions::from_mode(0o555))
        .expect("read-only audit");
    let site = fx.serving(&script);
    let asked = ingest::parse_spot(
        "target=indices&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05",
        aug_10(),
    )
    .expect("asked");

    let run = broker_run(&asked, &site, &[]).await;
    std::fs::set_permissions(&audit_dir, std::fs::Permissions::from_mode(0o755))
        .expect("writable again");

    assert_eq!(run.attempted, 4, "{run:?}");
    assert_eq!(run.reached, 3, "the fourth was not landed: {run:?}");
    assert_eq!(run.total.bars_committed, 0);
    let fault = run
        .store_unavailable
        .as_deref()
        .expect("the breaker tripped");
    assert!(fault.contains(".store-probe"), "{fault}");
    assert_eq!(
        run.stopped.as_deref(),
        Some(store_unavailable_sentence(fault, 3, 3, 4).as_str())
    );
}

// ----------------------------------------------------------- batch four

/// The receipt counts the instruments of the target it was asked for, read
/// from that target's own slot (kills `== -> !=` at `spot_answer`:3134,
/// which would read the first OTHER target's count).
#[tokio::test]
async fn the_receipt_counts_the_asked_targets_own_universe() {
    let fx = Fixture::four_indices("covered");
    let site = fx.refused();
    let body = "target=indices&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05";
    let (_, page) = spot_answer(body, aug_10(), std::time::SystemTime::now(), &site).await;
    let targets = site.universe().targets;
    assert_ne!(
        targets[0], targets[1],
        "the premise: swept and indices differ"
    );
    assert_eq!(
        facts(&page, "Instruments covered"),
        [format!("{} in the merged universe", targets[1])]
    );
    assert_eq!(targets[1], 4, "four indices: {targets:?}");
}

/// The retry class, by what another attempt could change: transient when
/// something did not answer or the store refused, standing otherwise, and
/// nothing to class with no failure.
#[test]
fn the_retry_fact_classes_transient_store_and_standing() {
    use crate::pullrun::RetryClass;
    let line = |class: RetryClass, transient: usize, store: usize, standing: usize| {
        (
            crate::pullrun::RETRY_CLASS_FACT,
            format!(
                "{} — {transient} failure(s) another attempt could change (an instrument that \
                 did not answer, a window cut short, a stop), {store} of the store and not the \
                 vendor (a write the store refused, or a census another writer held or that \
                 would not open), which another attempt can change too, and {standing} it \
                 would repeat (what the vendor holds for an instrument that answered, an \
                 instrument the pull order withheld, the end of the calendar of record)",
                class.word()
            ),
        )
    };
    assert_eq!(BrokerRun::default().retry_fact(), None);
    let mut refused = BrokerRun::default();
    refused.record_refusal("NIFTY", "no answer".to_owned());
    assert_eq!(
        refused.retry_fact(),
        Some(line(RetryClass::Transient, 1, 0, 0))
    );
    let mut stored = BrokerRun::default();
    stored
        .total
        .failures
        .push(pull::ingest::Failure::new("/store/x", "refused"));
    stored.retry.store = 1;
    assert_eq!(
        stored.retry_fact(),
        Some(line(RetryClass::Transient, 0, 1, 0))
    );
    let mut withheld = BrokerRun::default();
    withheld.record_withheld("NIFTY", "owes".to_owned());
    assert_eq!(
        withheld.retry_fact(),
        Some(line(RetryClass::Standing, 0, 0, 1))
    );
}

/// The cash schedule's gates, one condition at a time: a daily rung needs
/// none whatever the identity, and every identity field is checked.
#[tokio::test]
async fn the_cash_schedule_checks_every_identity_field() {
    let fx = Fixture::new("cash-identity");
    let site = fx.refused();
    let reliance = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Cash,
        underlying: brutex_core::symbol::Symbol::new("RELIANCE").expect("a symbol"),
        kind: brutex_core::instrument::Kind::Equity,
    };
    let mut dated = std::collections::HashMap::new();
    let equity = |instrument: &str, segment: &'static str, rung| {
        let mut window = window_of(instrument, segment, rung);
        window.listing = pull::vendor::Listing::Equity;
        window
    };
    // A DAILY RUNG: no schedule, even for an identity that would be refused.
    let daily = equity("RELIANCE", "INDEX", pull::vendor::Granularity::Day1);
    assert_eq!(
        prepare_cash_schedule(&daily, &[], &reliance, &site, &mut dated)
            .await
            .map(|schedule| schedule.is_none()),
        Ok(true)
    );
    // ANOTHER INSTRUMENT'S WINDOW is refused even on the right venue.
    let other = equity("TCS", "CASH", pull::vendor::Granularity::Minute1);
    assert_eq!(
        prepare_cash_schedule(&other, &[], &reliance, &site, &mut dated)
            .await
            .err()
            .as_deref(),
        Some("cash schedule requires the exact NSE cash source identity")
    );
    // THE EXACT IDENTITY with nothing observed needs no schedule.
    let exact = equity("RELIANCE", "CASH", pull::vendor::Granularity::Minute1);
    assert_eq!(
        prepare_cash_schedule(&exact, &[], &reliance, &site, &mut dated)
            .await
            .map(|schedule| schedule.is_none()),
        Ok(true)
    );
}

/// The cash days a body was observed on: inside the window, inside the cash
/// session, once each, in order.
#[test]
fn observed_cash_days_are_the_windows_in_session_days() {
    let at = |day: u8, hour: i64, minute: i64| {
        let days = i64::from(Day::new(2026, 8, day).expect("a day").days_from_epoch());
        days * 86_400 + hour * 3_600 + minute * 60 - 19_800
    };
    let row = |timestamp: i64| pull::fetch::RawRow {
        timestamp,
        open: 1,
        high: 1,
        low: 1,
        close: 1,
        volume: 1,
        open_interest: None,
    };
    let window = pull::session::Window::new(
        Day::new(2026, 8, 3).expect("a day"),
        Day::new(2026, 8, 5).expect("a day"),
    )
    .expect("a window");
    let body = pull::fetch::RawWindow {
        rows: vec![
            row(at(2, 10, 0)),
            row(at(3, 10, 0)),
            row(at(3, 11, 0)),
            row(at(4, 16, 0)),
            row(at(5, 10, 0)),
            row(at(6, 10, 0)),
        ],
    };
    assert_eq!(
        observed_cash_source_days(
            &[(window, body)],
            pull::vendor::TimestampEncoding::EpochSecondsUtc
        ),
        Ok(vec![
            Day::new(2026, 8, 3).expect("a day"),
            Day::new(2026, 8, 5).expect("a day"),
        ])
    );
}

/// The floor's edges: a window ending ON the floor keeps that one day, a
/// window wholly above it is untouched, and a rolling floor is recomputed
/// from the caller's day at 365.25 days a year.
#[test]
fn the_floor_clamp_at_its_edges_and_rolling() {
    let window = spot(DHAN_DAY).window;
    let day = |y, m, d| Day::new(y, m, d).expect("a day");
    assert_eq!(
        clamp_to_floor(
            window,
            pull::vendor::HistoryFloor::Fixed {
                year: 2026,
                month: 8,
                day: 5
            },
            aug_10()
        ),
        Ok(pull::session::Window::new(day(2026, 8, 5), day(2026, 8, 5)).expect("a window"))
    );
    assert_eq!(
        clamp_to_floor(
            window,
            pull::vendor::HistoryFloor::Fixed {
                year: 2026,
                month: 1,
                day: 1
            },
            aug_10()
        ),
        Ok(window)
    );
    let long = pull::session::Window::new(day(2020, 1, 1), day(2026, 8, 5)).expect("a window");
    let oldest = Day::from_days(aug_10().days_from_epoch() - 1_826).expect("a day");
    assert_eq!(
        clamp_to_floor(
            long,
            pull::vendor::HistoryFloor::Rolling { years: 5 },
            aug_10()
        ),
        Ok(pull::session::Window::new(oldest, day(2026, 8, 5)).expect("a window"))
    );
}

/// A minute cash window whose month already has a committed file the store
/// cannot open is refused, naming the instrument and month, before any
/// dated eligibility is asked for (`prepare_cash_schedule`'s committed-days
/// read).
#[tokio::test]
async fn an_unreadable_committed_cash_month_refuses_the_schedule() {
    let fx = Fixture::new("cash-committed");
    let site = fx.refused();
    let month = store::path::YearMonth::new(2026, 7).expect("a month");
    let file = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "CASH",
        symbol: "RELIANCE",
        contract: None,
        timeframe: store::path::Timeframe::MINUTE_1,
        month,
        file: store::path::FileKind::Bars,
    })
    .expect("a store path")
    .to_path_buf(&fx.root);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("month dir");
    std::fs::write(&file, b"not a bar file").expect("a damaged bar file");
    let reliance = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Cash,
        underlying: brutex_core::symbol::Symbol::new("RELIANCE").expect("a symbol"),
        kind: brutex_core::instrument::Kind::Equity,
    };
    let mut landed = window_of("RELIANCE", "CASH", pull::vendor::Granularity::Minute1);
    landed.listing = pull::vendor::Listing::Equity;
    let july = pull::session::Window::new(
        Day::new(2026, 7, 1).expect("a day"),
        Day::new(2026, 7, 31).expect("a day"),
    )
    .expect("a window");
    // 2026-07-01 10:00 IST, inside the cash session.
    let ten = i64::from(Day::new(2026, 7, 1).expect("a day").days_from_epoch()) * 86_400
        + 10 * 3_600
        - 19_800;
    let body = pull::fetch::RawWindow {
        rows: vec![pull::fetch::RawRow {
            timestamp: ten,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 1,
            open_interest: None,
        }],
    };
    let mut dated = std::collections::HashMap::new();
    let refused = prepare_cash_schedule(&landed, &[(july, body)], &reliance, &site, &mut dated)
        .await
        .expect_err("an unreadable committed month refuses");
    assert!(
        refused.starts_with(&format!("RELIANCE {month}: committed cash source: ")),
        "{refused}"
    );
    assert!(dated.is_empty(), "no eligibility was acquired");
}

// ------------------------------------------------- the spot breaker (fixed)

/// THE SPOT RUN'S VENDOR-DOWN BREAKER TRIPS. Every request is answered with
/// Dhan's own backend refusal (`DH-908`, `Disposition::RetryBounded`), so each
/// instrument spends its retry ladder and is refused as the vendor's side
/// failing. The third such instrument in a row stops the run with
/// `vendor_down_sentence(3, 3, 4)` and the fourth is never refused or filed.
///
/// Before the fix the run ended with `stopped = None` and all four refused:
/// `with_retry`, the spot path's ladder, wrote no `VENDOR_DOWN` marker, and
/// `fetch_chunks` would have buried one mid-sentence anyway. No refusal an
/// operator reads carries either control character.
#[tokio::test]
async fn three_instruments_lost_to_the_vendors_own_side_stop_a_spot_run() {
    let vendor = FakeVendor::answering(|_, _| {
        (
            400,
            "{\"errorType\":\"Data_API_Error\",\"errorCode\":\"DH-908\",\
             \"errorMessage\":\"Server was not able to process request\"}"
                .to_owned(),
        )
    })
    .await;
    let script = live(&vendor, "live");
    let fx = Fixture::four_indices("spot-vendor-down");
    let site = fx.serving(&script);
    let asked = ingest::parse_spot(
        "target=indices&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05",
        aug_10(),
    )
    .expect("asked");

    let run = broker_run(&asked, &site, &[]).await;

    assert_eq!(
        run.stopped.as_deref(),
        Some(vendor_down_sentence(3, 3, 4).as_str()),
        "{run:?}"
    );
    assert_eq!(run.reached, 0);
    assert_eq!(
        run.refused.len(),
        3,
        "the fourth is not filed: {:?}",
        run.refused
    );
    for why in &run.refused {
        assert!(
            why.contains("its own side has now failed 0 time(s) on this chunk"),
            "{why}"
        );
        assert!(
            !why.contains(VENDOR_DOWN) && !why.contains(WIRE_REACHED),
            "a marker reached the operator: {why:?}"
        );
    }
    assert!(run.touched_wire);
    assert!(!run.credential_dead);
    // Lanes widen to three after the first answer, so the fourth instrument
    // was fetched ahead; its six requests are the only cost past the breaker.
    assert_eq!(vendor.seen().len(), 4 * THROTTLE_ATTEMPTS as usize);
}

/// A server-down refusal on the F&O contract path is stripped of the
/// `VENDOR_DOWN` marker before it is filed: that path has no breaker to read
/// it, and an operator must never see a control character.
#[test]
fn the_fno_contract_refusal_carries_no_vendor_down_marker() {
    let marked = format!("{VENDOR_DOWN}status 503: busy");
    let refusal = marked.trim_start_matches(VENDOR_DOWN);
    assert_eq!(refusal, "status 503: busy");
    let source = include_str!("server.rs");
    let path = source
        .split_once("Err(refusal) => {\n                // THE MARKER STAYS AT THE HEAD.")
        .expect("the F&O contract refusal arm exists")
        .1;
    let arm = &path[..path
        .find("result: Fetched::ContractRefused")
        .expect("it files")];
    assert!(
        arm.contains("refusal.trim_start_matches(VENDOR_DOWN)"),
        "the arm strips VENDOR_DOWN before filing the refusal"
    );
}

// ------------------------------------------- the autopilot, flying (loopback)

/// AN UNPAUSED AUTOPILOT THAT MAY REACH A BROKER FLIES ITS LOOP AND JOURNALS
/// A TICK. Every feed's credential is the script's, so any request would go to
/// the loopback fake vendor, which refuses each one with a plain 400. After
/// the twenty-second grace the loop chooses the day rung's oldest month (a
/// Zerodha month: no instrument in these masters maps to Zerodha), runs one
/// tick and journals it: no instrument was reached and the run was not
/// blocked, so the record is NOT STARTED with the tick's own default reason,
/// and it closes the run the tick opened (`autopilot::fly`'s loop and
/// `tick`'s unblocked arms).
#[tokio::test]
async fn an_unpaused_live_autopilot_journals_a_tick_whose_every_instrument_was_refused() {
    let vendor = FakeVendor::answering(|_, _| (400, "{\"reason\":\"no\"}".to_owned())).await;
    let script = live(&vendor, "live");
    let fx = Fixture::new("fly-live-tick");
    let site = Loaded::new(fx.serving(&script));
    assert_eq!(site.broker, Broker::Live);
    site.autopilot.resume();
    let flying = tokio::spawn(crate::autopilot::fly(Loaded::clone(&site)));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(150);
    let records = loop {
        let records = journal_records(&fx.root);
        // THE RECORD LANDS BEFORE THE STATUS COUNTS THE TICK, so both are
        // waited for.
        if records
            .iter()
            .any(|record| record.outcome == audit::Outcome::NotStarted)
            && !site.autopilot.json().contains(r#""ticks":0,"#)
        {
            break records;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no tick was journalled: {}",
            site.autopilot.json()
        );
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    };
    site.autopilot.pause();
    flying.abort();
    let _aborted = flying.await;

    let closed = records
        .iter()
        .find(|record| record.outcome == audit::Outcome::NotStarted)
        .expect("the tick's record");
    assert_eq!(closed.kind, audit::Kind::Run);
    assert!(closed.source.contains("autopilot"), "{closed:?}");
    assert_eq!(
        closed.note, "no instrument in the universe could be reached",
        "{closed:?}"
    );
    assert!(
        records
            .iter()
            .any(|record| record.outcome == audit::Outcome::Opened),
        "the tick opened its run before its first instrument: {records:?}"
    );
    let json = site.autopilot.json();
    assert!(json.contains(r#""journal_error":"""#), "{json}");
}

/// A run the pull order refused whole (409) carries the STANDING class fact,
/// under the label the conductor reads; any other refusal carries none
/// (kills `order_class_fact -> None` and `-> Some(("", String::new()))`).
#[test]
fn a_run_the_pull_order_refused_carries_the_standing_class_fact() {
    let (label, value) =
        order_class_fact(axum::http::StatusCode::CONFLICT).expect("a 409 is the order's");
    assert_eq!(label, crate::pullrun::RETRY_CLASS_FACT);
    assert_eq!(
        value,
        format!(
            "{} — the pull order withholds every instrument asked for; asking again returns \
             the same until the pass it names has landed",
            crate::pullrun::RetryClass::Standing.word()
        )
    );
    for code in [
        axum::http::StatusCode::BAD_REQUEST,
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        axum::http::StatusCode::BAD_GATEWAY,
    ] {
        assert_eq!(order_class_fact(code), None, "{code}");
    }
}

/// A member handed NO rows landed nothing to ask the store about, whatever it
/// failed with: the probe is for a member the vendor fed and the store did
/// not take (kills `rows_read > 0` turned to `>=`).
#[test]
fn a_member_handed_no_rows_did_not_land_nothing() {
    let failed = pull::ingest::Ingested {
        rows_read: 0,
        bars_committed: 0,
        failures: vec![pull::ingest::Failure {
            instrument: "NIFTY".to_owned(),
            why: "the vendor answered nothing".to_owned(),
            month: None,
            day: None,
        }],
        ..pull::ingest::Ingested::default()
    };
    assert!(!landed_nothing(&failed));
    let fed = pull::ingest::Ingested {
        rows_read: 1,
        ..failed
    };
    assert!(landed_nothing(&fed));
}

/// THE RUN'S CLOSING LINE IS `Info` ONLY WHEN THE BOOKS BALANCE AND NO MEMBER
/// FAILED: a balanced run with a failure, and an unbalanced run with none, are
/// both `Warn` (kills `&&` turned to `||` in `note_run_finished`). Each run
/// carries an `expected` figure no other test uses, so its line is found among
/// the parallel suite's.
#[test]
fn the_run_finished_line_is_info_only_when_balanced_and_nothing_failed() {
    let failure = pull::ingest::Failure {
        instrument: "NIFTY".to_owned(),
        why: "the vendor answered nothing".to_owned(),
        month: None,
        day: None,
    };
    for (expected, balanced, failed, level, message) in [
        (9_190_311, true, false, telemetry::Level::Info, "finished"),
        (9_190_312, true, true, telemetry::Level::Warn, "finished"),
        (
            9_190_313,
            false,
            false,
            telemetry::Level::Warn,
            "finished — BOOKS DO NOT BALANCE",
        ),
    ] {
        let mut out = BrokerRun::unreachable_broker();
        out.attempted = expected;
        if failed {
            out.total.failures.push(failure.clone());
        }
        let from = crate::emitted::mark();
        note_run_finished(&out, balanced, None);
        let mine: Vec<telemetry::Record> = crate::emitted::landed(from, "pull.run", message)
            .into_iter()
            .filter(|record| crate::emitted::counts(record, "expected", expected as u64))
            .collect();
        assert_eq!(mine.len(), 1, "{expected}: {mine:?}");
        assert_eq!(mine[0].level, level, "{expected}: {mine:?}");
    }
}

/// A RUN'S TARGETS ARE TRACKED **AND** NAMED BY THE TARGET ASKED FOR. Four
/// tracked indices are in the masters; the swept target names two, and only
/// those two are run, while the index target names all four (kills `&&`
/// turned to `||` in `spot_targets`, which would run every tracked name
/// whatever the target).
#[test]
fn a_runs_targets_are_tracked_and_named_by_the_target_asked_for() {
    let fx = Fixture::four_indices("targets-named");
    let site = fx.refused();
    let names = |body: &str| {
        let mut named: Vec<String> = spot_targets(&spot(body), &site)
            .iter()
            .map(|key| key.underlying.as_str().to_owned())
            .collect();
        named.sort_unstable();
        named
    };
    assert_eq!(names(DHAN_DAY), ["BANKNIFTY", "NIFTY"]);
    assert_eq!(
        names("target=indices&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05"),
        ["BANKNIFTY", "FINNIFTY", "MIDCPNIFTY", "NIFTY"]
    );
}

/// THE EPOCH'S FIRST DAY IS A DAY LIKE ANY OTHER. A window opening on
/// 1970-01-01 (day 0) is read like every other window: its in-session minute is
/// observed and its pre-open minute is not (kills `-` turned to `/` in the
/// window's day count, which divides by that zero).
#[test]
fn a_window_opening_on_the_epochs_first_day_is_read_like_any_other() {
    let row = |timestamp: i64| pull::fetch::RawRow {
        timestamp,
        open: 1,
        high: 1,
        low: 1,
        close: 1,
        volume: 1,
        open_interest: None,
    };
    let first = Day::from_days(0).expect("1970-01-01");
    let second = Day::from_days(1).expect("1970-01-02");
    let window = pull::session::Window::new(first, second).expect("a window");
    // 10:00 IST on each day (04:30 UTC), and 08:00 IST on the first, before
    // the open.
    let body = pull::fetch::RawWindow {
        rows: vec![row(16_200), row(9_000), row(86_400 + 16_200)],
    };
    assert_eq!(
        observed_cash_source_days(
            &[(window, body)],
            pull::vendor::TimestampEncoding::EpochSecondsUtc
        ),
        Ok(vec![first, second])
    );
}

/// A BODY WHOSE CASH SCHEDULE IS REFUSED IS STILL A MEMBER THE RUN WAS HANDED.
/// Two equity minute bodies under an identity that is not an NSE cash source
/// are each refused before anything lands, and the landing counts both as
/// members, every row as read, and one failure per body naming its month
/// (kills `delete field members` in `land_spot`'s refused arm).
#[tokio::test]
async fn a_body_whose_cash_schedule_is_refused_is_still_a_member() {
    let fx = Fixture::new("cash-refused-members");
    let site = fx.refused();
    let reliance = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Cash,
        underlying: brutex_core::symbol::Symbol::new("RELIANCE").expect("a symbol"),
        kind: brutex_core::instrument::Kind::Equity,
    };
    let row = |timestamp: i64| pull::fetch::RawRow {
        timestamp,
        open: 1,
        high: 1,
        low: 1,
        close: 1,
        volume: 1,
        open_interest: None,
    };
    let window = spot(DHAN_DAY).window;
    let mut landed = window_of("RELIANCE", "INDEX", pull::vendor::Granularity::Minute1);
    landed.listing = pull::vendor::Listing::Equity;
    landed.bodies = vec![
        (
            window,
            pull::fetch::RawWindow {
                rows: vec![row(AUG_03)],
            },
        ),
        (
            window,
            pull::fetch::RawWindow {
                rows: vec![row(AUG_03 + 60), row(AUG_03 + 120)],
            },
        ),
    ];
    let mut dated = std::collections::HashMap::new();
    let done = land_spot(&landed, &reliance, &site, &mut dated, None).await;
    assert_eq!(
        (done.members, done.rows_read, done.bars_committed),
        (2, 3, 0),
        "{done:?}"
    );
    let august = store::path::YearMonth::new(2026, 8).expect("a month");
    assert_eq!(done.failures.len(), 2, "{done:?}");
    for failure in &done.failures {
        assert_eq!(failure.instrument, "RELIANCE");
        assert_eq!(
            failure.why,
            "cash schedule requires the exact NSE cash source identity"
        );
        assert_eq!((failure.month, failure.day), (Some(august), None));
    }
}

/// A RECOVERY IS REFUSED ON THE MAPPING A RUN WOULD BE REFUSED ON, before any
/// audit or wire request: a member no feed id names is refused in the run's
/// own words, and a fully mapped target is not (kills `recovery_mapping ->
/// Ok(())`).
#[test]
fn a_recovery_is_refused_on_the_mapping_a_run_would_be_refused_on() {
    let fx = Fixture::new("recovery-mapping");
    let site = fx.refused();
    assert_eq!(recovery_mapping(&site, &spot(DHAN_DAY)), Ok(()));
    let asked = spot(&format!("{DHAN_DAY}&member=NOSUCH"));
    let why = recovery_mapping(&site, &asked).expect_err("an unmapped member is refused");
    assert_eq!(
        Some(why.as_str()),
        spot_mapping_refusal(&asked, &site.universe().read, &spot_targets(&asked, &site))
            .as_deref()
    );
    assert!(
        why.starts_with("Instrument mapping is not ready for Dhan: NOSUCH: "),
        "{why}"
    );
}

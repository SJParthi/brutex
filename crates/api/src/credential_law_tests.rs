#![cfg(test)]
//! `CLAUDE.md` §8 on the shipped pull loops, driven against a scripted
//! credential source and a loopback vendor that rejects. No AWS, no network.
//!
//! GAP2-36, GAP2-37, D-0948. Each test drives a production loop:
//! `broker_run` for spot, `roll_every` for the offset-addressed F&O cells and
//! `fno_land` for the named contracts. Each one counts the requests the fake
//! vendor actually received, because "zero further requests" is a claim about
//! the wire and cannot be checked anywhere else.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::credential_law::{CredentialStop, Credentials, Script, Unread, Unreadable};
use std::sync::{Arc, Mutex};

/// A loopback vendor that rejects every request with 401 unless the request
/// carries `accept`, which it answers with 200 and a body nobody can read.
/// It keeps the header values of every request it received, in order.
struct FakeVendor {
    base: &'static str,
    seen: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl FakeVendor {
    async fn rejecting_all_but(accept: Option<&'static str>) -> Self {
        Self::refusing_with(accept, axum::http::StatusCode::UNAUTHORIZED).await
    }

    /// A vendor whose own side is down: every request is answered `503`.
    async fn down() -> Self {
        Self::refusing_with(None, axum::http::StatusCode::SERVICE_UNAVAILABLE).await
    }

    /// A vendor that takes `delay` to answer every request with `refusal`.
    async fn slow(refusal: axum::http::StatusCode, delay: std::time::Duration) -> Self {
        Self::serving(None, refusal, delay).await
    }

    async fn refusing_with(accept: Option<&'static str>, refusal: axum::http::StatusCode) -> Self {
        Self::serving(accept, refusal, std::time::Duration::ZERO).await
    }

    async fn serving(
        accept: Option<&'static str>,
        refusal: axum::http::StatusCode,
        delay: std::time::Duration,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        let app = axum::Router::new().fallback(move |headers: axum::http::HeaderMap| {
            let carried = headers
                .values()
                .filter_map(|value| value.to_str().ok())
                .collect::<Vec<_>>()
                .join(" ");
            let ok = accept.is_some_and(|token| carried.contains(token));
            log.lock().expect("the fake's log").push(carried);
            async move {
                tokio::time::sleep(delay).await;
                if ok {
                    (axum::http::StatusCode::OK, "{}")
                } else {
                    (refusal, "{}")
                }
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

    /// The header values of every request received, in order.
    fn seen(&self) -> Vec<String> {
        self.seen.lock().expect("the fake's log").clone()
    }
}

impl Drop for FakeVendor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a script answer is a Result, and the scripts read as one list of answers"
)]
fn token(value: &str) -> Result<String, Unreadable> {
    Ok(value.to_owned())
}

fn ssm_timed_out() -> Result<String, Unreadable> {
    Err(Unreadable::transport(
        "this feed's credential field \"access-token\" could not be read: \
         ssm.ap-south-1.amazonaws.com was not reached: operation timed out"
            .to_owned(),
    ))
}

fn scripted(vendor: &FakeVendor, answers: Vec<Result<String, Unreadable>>) -> Arc<Script> {
    Arc::new(Script::new(vendor.base, answers))
}

// ------------------------------------------------------------------ spot

/// A serving site over two indices that Dhan names, with its credential
/// taken from `script`.
fn spot_site(name: &str, script: &Arc<Script>) -> (Site, ingest::SpotRequest) {
    let dir = crate::scratch::path(&format!("credential-law-{name}"));
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
    let root = crate::scratch::path(&format!("credential-law-store-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("manifest")).expect("store root");
    let mut site = Site::serving(&dir, &root);
    site.credentials = Credentials::Scripted(Arc::clone(script));
    let asked = ingest::parse_spot(
        "target=swept&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05",
        Day::new(2026, 8, 10).expect("a day"),
    )
    .expect("a finished daily window");
    (site, asked)
}

/// **(a) Same value on re-read: the run halts, and the vendor hears nothing
/// more.**
///
/// The first instrument's request is rejected. The run re-reads exactly once
/// and gets the same token, so it stops. The second instrument is never sent,
/// and the stop names the feed, the field and §8. The vendor received ONE
/// request. Before D-0948 it received one per instrument, each carrying the
/// dead token.
///
/// **(e)** The same run, carried to the autopilot through `outcome_of`, halts
/// the feed as [`autopilot::Halt::Credential`].
#[tokio::test]
async fn a_rejected_token_whose_re_read_is_unchanged_halts_the_spot_run_with_no_further_request() {
    let _shared = crate::emitted::sink();
    let from = crate::emitted::mark();
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(&vendor, vec![token("stale")]);
    let (site, asked) = spot_site("same", &script);

    let run = broker_run(&asked, &site, &[]).await;

    assert!(run.blocked.is_none(), "the run reached its loop: {run:?}");
    assert_eq!(run.attempted, 2, "two instruments were named");
    let seen = vendor.seen();
    assert_eq!(
        seen.len(),
        1,
        "ONE request reached the vendor; the dead token was not sent again: {seen:?}"
    );
    assert!(seen[0].contains("stale"));
    assert_eq!(
        script.reads(),
        2,
        "one read for the request and exactly ONE re-read after the rejection"
    );
    assert!(run.credential_dead, "the vendor's verdict is recorded");
    assert_eq!(run.credential_stop, Some(CredentialStop::SameValue));
    let stopped = run.stopped.as_deref().expect("the halt is loud");
    for named in [
        "Dhan",
        "access-token",
        "same value",
        "never mints",
        "1 of 2",
    ] {
        assert!(stopped.contains(named), "{named:?} is named: {stopped}");
    }
    assert!(
        !stopped.contains("stale"),
        "the credential value itself never reaches the reason: {stopped}"
    );

    // (e) THE AUTOPILOT READS THE STRUCTURAL VERDICT AND HALTS.
    let mut state = autopilot::FeedState::new(
        pull::vendor::Feed::Dhan,
        Vendor::Dhan,
        store::path::YearMonth::new(2026, 8).expect("a month"),
    );
    let outcome = autopilot::outcome_of(&run, false, None);
    assert_eq!(outcome.credential, Some(CredentialStop::SameValue));
    let autopilot::Next::Halt { reason } = state.observe(&outcome) else {
        panic!("a same-value verdict halts the feed");
    };
    assert_eq!(state.halt_kind, Some(autopilot::Halt::Credential));
    assert!(reason.contains("returned the same value"), "{reason}");

    // THE RE-READ IS ON THE ROLLING LOG, with the field named and the value
    // absent. This is the proof `emitted` counts for `pull.credential`.
    let said = crate::emitted::landed(
        from,
        "pull.credential",
        "re-read returned the SAME value; the token is dead and the pull halted",
    );
    assert!(
        said.iter().any(|record| {
            record.level == telemetry::Level::Warn
                && crate::emitted::says(record, "field", "access-token")
                && !format!("{record:?}").contains("stale")
        }),
        "the same-value verdict is logged by field name and never by value: {said:?}"
    );
}

/// **(b) A rotated value: the run continues, and the next request carries
/// the NEW token.**
#[tokio::test]
async fn a_rejected_token_whose_re_read_rotated_continues_with_the_new_value() {
    let vendor = FakeVendor::rejecting_all_but(Some("fresh")).await;
    let script = scripted(&vendor, vec![token("stale"), token("fresh")]);
    let (site, asked) = spot_site("rotated", &script);

    let run = broker_run(&asked, &site, &[]).await;

    let seen = vendor.seen();
    assert_eq!(seen.len(), 2, "both instruments were asked: {seen:?}");
    assert!(seen[0].contains("stale") && !seen[0].contains("fresh"));
    assert!(
        seen[1].contains("fresh") && !seen[1].contains("stale"),
        "the second instrument carried the rotated token: {seen:?}"
    );
    assert_eq!(run.credential_stop, None, "a rotation is not a stop");
    assert!(run.stopped.is_none(), "{run:?}");
    assert_eq!(
        script.reads(),
        3,
        "read, ONE re-read, and the next instrument's own read"
    );
}

/// **A rotation that is undone is caught before the socket.**
///
/// The vendor rejected `stale` and the re-read returned `fresh`. If the next
/// read hands `stale` back, the run already knows that value is dead, so it is
/// not sent. The run stops with nothing more on the wire.
#[tokio::test]
async fn a_read_that_returns_a_value_already_rejected_is_never_sent() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(
        &vendor,
        vec![token("stale"), token("fresh"), token("stale")],
    );
    let (site, asked) = spot_site("flapped", &script);

    let run = broker_run(&asked, &site, &[]).await;

    assert_eq!(vendor.seen().len(), 1, "{:?}", vendor.seen());
    assert_eq!(run.credential_stop, Some(CredentialStop::SameValue));
    let stopped = run.stopped.as_deref().expect("loud");
    assert!(stopped.contains("already rejected"), "{stopped}");
    assert!(stopped.contains("2 of 2"), "{stopped}");
}

/// **(c) The re-read itself fails: the run halts loudly, and does not claim
/// a comparison.**
///
/// GAP2-37 from the other side: the halt is real, because the run cannot
/// tell whether the token rotated and must not send the rejected value
/// again. But no comparison was made, so nothing may say "same value", and
/// an unreachable Parameter Store is transport to the autopilot, not a dead
/// token.
#[tokio::test]
async fn a_failed_re_read_halts_the_run_and_the_autopilot_backs_off() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(&vendor, vec![token("stale"), ssm_timed_out()]);
    let (site, asked) = spot_site("reread-failed", &script);

    let run = broker_run(&asked, &site, &[]).await;

    assert_eq!(vendor.seen().len(), 1, "{:?}", vendor.seen());
    assert_eq!(
        run.credential_stop,
        Some(CredentialStop::Unreadable(Unread::Transport))
    );
    let stopped = run.stopped.as_deref().expect("the halt is loud");
    assert!(stopped.contains("re-read after it failed"), "{stopped}");
    assert!(stopped.contains("operation timed out"), "{stopped}");
    assert!(!stopped.contains("same value"), "{stopped}");

    let mut state = autopilot::FeedState::new(
        pull::vendor::Feed::Dhan,
        Vendor::Dhan,
        store::path::YearMonth::new(2026, 8).expect("a month"),
    );
    let next = state.observe(&autopilot::outcome_of(&run, false, None));
    assert!(
        matches!(next, autopilot::Next::Wait { .. }),
        "transport backs off: {next:?}"
    );
    assert_eq!(state.halt_kind, None);
}

/// **A configuration fault stops the run before any request, and halts the
/// feed as configuration.**
///
/// The first read fails as configuration. Every later instrument would fail
/// the same way, so the run stops at once, sends nothing, and makes no
/// re-read: nothing was rejected.
#[tokio::test]
async fn a_configuration_fault_stops_the_run_unsent_and_halts_as_configuration() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(
        &vendor,
        vec![Err(Unreadable::configuration(
            "the credential configuration at ~/.brutex/credentials.toml is not usable".to_owned(),
        ))],
    );
    let (site, asked) = spot_site("config", &script);

    let run = broker_run(&asked, &site, &[]).await;

    assert!(vendor.seen().is_empty(), "{:?}", vendor.seen());
    assert_eq!(script.reads(), 1, "no re-read: nothing was rejected");
    assert_eq!(
        run.credential_stop,
        Some(CredentialStop::Unreadable(Unread::Configuration))
    );
    assert!(!run.credential_dead, "no vendor rejected anything");

    let mut state = autopilot::FeedState::new(
        pull::vendor::Feed::Dhan,
        Vendor::Dhan,
        store::path::YearMonth::new(2026, 8).expect("a month"),
    );
    let autopilot::Next::Halt { reason } = state.observe(&autopilot::outcome_of(&run, false, None))
    else {
        panic!("an unusable configuration halts");
    };
    assert_eq!(state.halt_kind, Some(autopilot::Halt::Configuration));
    assert!(!reason.contains("same value"), "{reason}");
}

/// A serving site over FOUR indices that Dhan names, every one in the
/// `indices` target, with its credential taken from `script`.
fn four_index_site(name: &str, script: &Arc<Script>) -> (Site, ingest::SpotRequest) {
    let dir = crate::scratch::path(&format!("credential-law-{name}"));
    std::fs::create_dir_all(&dir).expect("masters dir");
    std::fs::write(
        dir.join("groww_instruments.csv"),
        "exchange,segment,underlying_symbol,trading_symbol,instrument_type,series,isin,\
         expiry_date,strike_price,groww_symbol\n\
         NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n\
         NSE,CASH,,BANKNIFTY,IDX,,BANKNIFTY,,,NSE-BANKNIFTY\n\
         NSE,CASH,,FINNIFTY,IDX,,FINNIFTY,,,NSE-FINNIFTY\n\
         NSE,CASH,,MIDCPNIFTY,IDX,,MIDCPNIFTY,,,NSE-MIDCPNIFTY\n",
    )
    .expect("groww master");
    std::fs::write(
        dir.join("dhan_scrip.csv"),
        "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,\
         SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n\
         NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,13\n\
         NSE,I,NA,INDEX,BANKNIFTY,BANKNIFTY,INDEX,NA,0001-01-01,,,25\n\
         NSE,I,NA,INDEX,FINNIFTY,FINNIFTY,INDEX,NA,0001-01-01,,,9001\n\
         NSE,I,NA,INDEX,MIDCPNIFTY,MIDCPNIFTY,INDEX,NA,0001-01-01,,,9002\n",
    )
    .expect("dhan master");
    let root = crate::scratch::path(&format!("credential-law-store-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("manifest")).expect("store root");
    let mut site = Site::serving(&dir, &root);
    site.credentials = Credentials::Scripted(Arc::clone(script));
    let asked = ingest::parse_spot(
        "target=indices&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05",
        Day::new(2026, 8, 10).expect("a day"),
    )
    .expect("a finished daily window");
    (site, asked)
}

/// **conc8-1: a vendor whose own side is down stops the spot run after
/// [`VENDOR_DOWN_INSTRUMENTS`] instruments, and the autopilot backs off.**
///
/// `with_retry` is the only ladder under `broker_run`, and it never wrote
/// `VENDOR_DOWN`, so the breaker's streak stayed at zero and every instrument
/// paid the whole 5xx ladder. Four instruments against an all-503 vendor:
/// three ladders are spent (five requests each), the fourth instrument is
/// never asked, and the stop names the vendor's side. No control marker
/// reaches the operator's text.
///
/// autopilot-2 rides on it: the breaker's stop is not an operator's pause, so
/// the tick counts an attempt and waits rather than retrying at once.
#[tokio::test]
async fn a_vendor_answering_5xx_stops_the_spot_run_after_the_breaker_count() {
    let vendor = FakeVendor::down().await;
    let script = scripted(&vendor, vec![token("live")]);
    let (site, asked) = four_index_site("vendor-down", &script);

    let run = broker_run(&asked, &site, &[]).await;

    assert!(run.blocked.is_none(), "the run reached its loop: {run:?}");
    assert_eq!(run.attempted, 4, "four instruments were named: {run:?}");
    let per_instrument = SERVER_ERROR_ATTEMPTS as usize;
    let breaker = VENDOR_DOWN_INSTRUMENTS as usize;
    assert_eq!(
        vendor.seen().len(),
        per_instrument * breaker,
        "{breaker} ladders of {per_instrument} and nothing for the fourth instrument"
    );
    assert!(
        !run.cancelled,
        "the breaker's stop is not the operator's pause"
    );
    let stopped = run.stopped.as_deref().expect("the stop is loud");
    assert!(stopped.contains("consecutive"), "{stopped}");
    assert!(stopped.contains("3 of 4"), "{stopped}");
    for said in run
        .refused
        .iter()
        .chain(std::iter::once(&stopped.to_owned()))
    {
        assert!(
            !said.chars().any(char::is_control),
            "no marker reaches operator text: {said:?}"
        );
    }

    let outcome = autopilot::outcome_of(&run, false, None);
    assert!(
        !outcome.stopped,
        "a vendor outage is not an operator's pause"
    );
    let mut state = autopilot::FeedState::new(
        pull::vendor::Feed::Dhan,
        Vendor::Dhan,
        store::path::YearMonth::new(2026, 8).expect("a month"),
    );
    let next = state.observe(&outcome);
    assert!(
        matches!(next, autopilot::Next::Wait { .. }),
        "the outage is backed off and counted: {next:?}"
    );
    assert_eq!(state.attempts, 1, "the attempt is counted");
}

/// **conc6-2: the autopilot's Stop does not cut a hand-made walk short.**
///
/// `broker_run` is shared by the autopilot tick, a hand `/pull/spot`, every
/// `/pull/run` press leg and recovery, and every one of them captured the
/// autopilot's stop generation. So a Stop pressed on the autopilot page cut
/// an operator's own pull at its next instrument and journalled it FAILED as
/// "stopped by the operator", while the Stop's answer spoke only of the
/// autopilot. A hand walk now runs to its end; the autopilot's own walk
/// still stops.
#[tokio::test]
async fn an_autopilot_stop_does_not_cut_a_hand_walk_short() {
    let vendor = FakeVendor::slow(
        axum::http::StatusCode::NOT_FOUND,
        std::time::Duration::from_millis(300),
    )
    .await;
    let script = scripted(&vendor, vec![token("live")]);
    let (site, asked) = spot_site("hand-walk-stop", &script);

    // THE STOP LANDS WHILE THE FIRST INSTRUMENT IS ON THE WIRE.
    let press_stop = async {
        while vendor.seen().is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        site.autopilot.pause();
    };
    let (run, ()) = tokio::join!(broker_run(&asked, &site, &[]), press_stop);

    assert_eq!(vendor.seen().len(), 2, "both instruments were asked");
    assert!(
        run.stopped.is_none(),
        "the hand walk was not stopped: {run:?}"
    );

    // THE AUTOPILOT'S OWN WALK STILL OBEYS IT.
    let (site, asked) = spot_site("autopilot-walk-stop", &script);
    let before = vendor.seen().len();
    let press_stop = async {
        while vendor.seen().len() == before {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        site.autopilot.pause();
    };
    let (run, ()) = tokio::join!(
        broker_run_at(&asked, &site, &[], Some(site.autopilot.epoch())),
        press_stop
    );
    assert_eq!(vendor.seen().len(), before + 1, "the second was not asked");
    let stopped = run.stopped.as_deref().expect("the autopilot walk stopped");
    // `stopped after` counts instruments REACHED, and the first was refused.
    assert!(stopped.contains("stopped after 0 of 2"), "{stopped}");
}

/// **conc8-2: the F&O ladder's 5xx refusal carries no control marker.**
///
/// `laddered` wrote `VENDOR_DOWN` at the head of its `detail`, and nothing on
/// the discovery or rolling walks ever stripped it, so `\u{2}` landed after
/// `{label}: ` in the receipt, the rolling log and the journal. The verdict
/// is the refusal's own `status`, which the walk already carries.
#[tokio::test]
async fn the_f_and_o_ladder_refusal_on_a_5xx_carries_no_control_marker() {
    let vendor = FakeVendor::down().await;
    let script = scripted(&vendor, vec![token("live")]);
    let (site, _) = spot_site("laddered-down", &script);
    let asked = std::sync::atomic::AtomicU32::new(0);

    let refused = laddered(pull::vendor::Feed::Dhan, &site, "rolling request", || {
        asked.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        core::future::ready(Err(pull::chain::Refusal {
            credential_dead: false,
            status: Some(503),
            named: None,
            detail: "503 Service Unavailable".to_owned(),
        }))
    })
    .await
    .expect_err("an all-503 vendor is refused");

    assert_eq!(
        asked.into_inner(),
        SERVER_ERROR_ATTEMPTS,
        "the whole 5xx ladder"
    );
    assert_eq!(refused.status, Some(503), "the verdict is the status");
    let said = format!("NIFTY: {refused}");
    assert!(said.contains("its own side has now failed"), "{said}");
    assert!(
        !said.chars().any(char::is_control),
        "no marker reaches operator text: {said:?}"
    );
}

/// **conc8-1 on the named F&O walk: the marker `with_retry` now writes is
/// lifted there too.** The named walk keeps no breaker; it must still not
/// carry the marker into the contract's reason.
#[tokio::test]
async fn the_named_walk_strips_the_vendor_down_marker() {
    let vendor = FakeVendor::down().await;
    let script = scripted(&vendor, vec![token("live")]);

    let landed = named_walk("vendor-down", &vendor, &script, &[24_000]).await;

    assert_eq!(vendor.seen().len(), SERVER_ERROR_ATTEMPTS as usize);
    assert_eq!(landed.failed, 1, "{:?}", landed.why);
    assert!(
        landed
            .why
            .iter()
            .all(|why| !why.chars().any(char::is_control) && why.contains("own side")),
        "{:?}",
        landed.why
    );
}

/// A local-archive feed has no credential, and asking for one is a
/// configuration refusal, not a panic and not a silent empty value.
#[tokio::test]
async fn a_local_archive_feed_has_no_credential_to_read() {
    let archive = pull::vendor::Feed::ALL
        .into_iter()
        .find(|feed| {
            matches!(
                feed.descriptor().transport,
                pull::vendor::Transport::LocalArchive(_)
            )
        })
        .expect("a local-archive feed exists");
    let refused = Credentials::Ssm
        .source(archive)
        .await
        .expect_err("no credential for an archive");
    assert_eq!(refused.class, Unread::Configuration);
    assert!(refused.to_string().contains("local archive"), "{refused}");
}

// ------------------------------------------------------------- F&O cells

/// The offset-addressed walk against the fake, narrowed to one ordinal and
/// two strikes so its cross product is small.
struct Rolling {
    site: Site,
    wire: Wire,
    asked: ingest::FnoRequest,
    spec: pull::vendor::RollingSpec,
    today: Day,
}

impl Rolling {
    fn new(name: &str, vendor: &FakeVendor, script: &Arc<Script>) -> Self {
        let root = crate::scratch::path(&format!("credential-law-roll-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("masters")).expect("masters");
        let mut site = Site::load(&root.join("masters"), &root);
        site.credentials = Credentials::Scripted(Arc::clone(script));
        let today = Day::new(2025, 8, 2).expect("a day");
        let asked = ingest::parse_fno(
            "underlying=NIFTY&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01",
            today,
        )
        .expect("a rolling request");
        let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
            panic!("Dhan is HTTP");
        };
        let spec = pull::vendor::HttpSpec {
            base_url: vendor.base,
            ..shipped
        };
        let rolling = pull::vendor::RollingSpec {
            expiry_codes: &["1"],
            ..spec.fno.by_offset().expect("a rolling descriptor")
        };
        let source =
            pull::http::HttpSource::new(spec, pull::http::Credential::token("stale".to_owned()))
                .expect("a loopback source")
                .sharing(shared_governor(&site, asked.feed));
        Self {
            site,
            wire: Wire {
                source,
                spec,
                store_vendor: Vendor::Dhan,
            },
            asked,
            spec: rolling,
            today,
        }
    }

    async fn walk(&self) -> RollingWalk {
        roll_every(
            &self.asked,
            &self.site,
            &self.wire,
            self.spec,
            "13",
            "OPTIDX",
            &["ATM", "ATM+1"],
            ingest::last_settled_day(self.today).expect("a settled day"),
        )
        .await
    }
}

/// **(d) A rejection mid-walk stops every remaining cell.**
///
/// The walk was built with one source and used to send its token to every
/// cell of the cross product, including every cell after the vendor rejected
/// it. Now the first rejection is re-read once. The value is unchanged, so the
/// walk ends: one request on the wire, the rest of the plan not asked, and the
/// receipt says why.
#[tokio::test]
async fn a_rejection_mid_rolling_walk_stops_every_remaining_cell() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(&vendor, vec![token("stale")]);
    let fixture = Rolling::new("same", &vendor, &script);

    let walk = fixture.walk().await;

    assert!(walk.planned > 1, "the premise: a cross product: {walk:?}");
    assert_eq!(
        vendor.seen().len(),
        1,
        "{} cells were planned and ONE was sent: {:?}",
        walk.planned,
        vendor.seen()
    );
    assert_eq!(script.reads(), 1, "exactly one re-read");
    assert_eq!(walk.credential_stop, Some(CredentialStop::SameValue));
    let said = walk.done.why.join(" | ");
    assert!(
        said.contains("same value") && said.contains("Dhan"),
        "{said}"
    );
    assert!(
        !said.contains('\u{3}'),
        "the marker never reaches a reason: {said:?}"
    );

    // AND THE RECEIPT STATES IT AS A FACT A READER CAN MATCH EXACTLY.
    let mut facts = Vec::new();
    credential_stop_facts(&mut facts, walk.credential_stop);
    assert!(
        facts
            .iter()
            .any(|(label, value)| *label == "Credential" && value == CREDENTIAL_FACT),
        "{facts:?}"
    );
}

/// **A rotation mid-walk: every remaining cell carries the new token.**
#[tokio::test]
async fn a_rotation_mid_rolling_walk_sends_the_remaining_cells_with_the_new_token() {
    let vendor = FakeVendor::rejecting_all_but(Some("fresh")).await;
    let script = scripted(&vendor, vec![token("fresh")]);
    let fixture = Rolling::new("rotated", &vendor, &script);

    let walk = fixture.walk().await;

    let seen = vendor.seen();
    assert_eq!(seen.len(), walk.planned, "every planned cell was asked");
    assert!(seen[0].contains("stale"));
    assert!(
        seen.iter().skip(1).all(|carried| carried.contains("fresh")),
        "{seen:?}"
    );
    assert_eq!(walk.credential_stop, None);
    assert_eq!(script.reads(), 1, "one rejection, one re-read");
    let mut facts = Vec::new();
    credential_stop_facts(&mut facts, walk.credential_stop);
    assert!(facts.is_empty(), "no stop, no stop facts: {facts:?}");
}

/// **A rotation that is undone mid-walk is caught before the socket.** v3a-1,
/// D-1482.
///
/// The walk sends `stale`, which is rejected; the re-read returns `fresh`,
/// which is rejected too; the next re-read hands `stale` back. The watch held
/// only the LAST dead value and only the spot loop's `admit` read it, so the
/// walk took `stale` for a rotation and sent it to the remaining cells. Now the
/// re-read itself refuses any value this run already saw rejected: two
/// requests on the wire, two re-reads, and the stop names the reason.
#[tokio::test]
async fn an_undone_rotation_mid_rolling_walk_never_resends_a_rejected_token() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(&vendor, vec![token("fresh"), token("stale")]);
    let fixture = Rolling::new("flapped", &vendor, &script);

    let walk = fixture.walk().await;

    assert!(
        walk.planned > 2,
        "the premise: cells remain after two: {walk:?}"
    );
    let seen = vendor.seen();
    assert_eq!(
        seen.len(),
        2,
        "stale once, fresh once, never stale again: {seen:?}"
    );
    assert!(
        seen[0].contains("stale") && seen[1].contains("fresh"),
        "{seen:?}"
    );
    assert_eq!(script.reads(), 2, "one re-read per rejection");
    assert_eq!(walk.credential_stop, Some(CredentialStop::SameValue));
    let said = walk.done.why.join(" | ");
    assert!(said.contains("already rejected in this run"), "{said}");
}

/// **A failed re-read mid-walk ends the walk too, and says so without
/// claiming a comparison.**
#[tokio::test]
async fn a_failed_re_read_mid_rolling_walk_stops_the_walk() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(&vendor, vec![ssm_timed_out()]);
    let fixture = Rolling::new("reread-failed", &vendor, &script);

    let walk = fixture.walk().await;

    assert_eq!(vendor.seen().len(), 1);
    assert_eq!(
        walk.credential_stop,
        Some(CredentialStop::Unreadable(Unread::Transport))
    );
    let mut facts = Vec::new();
    credential_stop_facts(&mut facts, walk.credential_stop);
    assert!(facts.iter().any(|(label, _)| *label == "Stopped"));
    assert!(
        !facts.iter().any(|(label, _)| *label == "Credential"),
        "no comparison, so no dead-token fact: {facts:?}"
    );
}

// ------------------------------------------------------- named contracts

/// The named-contract walk over three contracts, against the fake.
/// The three contracts the credential walks ask for.
const THREE_STRIKES: &[u32] = &[24_000, 24_050, 24_100];

async fn named_walk(
    name: &str,
    vendor: &FakeVendor,
    script: &Arc<Script>,
    strikes: &[u32],
) -> FnoLanded {
    let root = crate::scratch::path(&format!("credential-law-named-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("masters")).expect("masters");
    let mut site = Site::load(&root.join("masters"), &root);
    site.credentials = Credentials::Scripted(Arc::clone(script));
    let today = Day::new(2025, 8, 2).expect("a day");
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01",
        today,
    )
    .expect("a named-chain request");
    let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
        panic!("Groww is HTTP");
    };
    let spec = pull::vendor::HttpSpec {
        base_url: vendor.base,
        ..shipped
    };
    let wire = Wire {
        source: pull::http::HttpSource::new(
            spec,
            pull::http::Credential::token("stale".to_owned()),
        )
        .expect("a loopback source")
        .sharing(shared_governor(&site, asked.feed)),
        spec,
        store_vendor: Vendor::Groww,
    };
    let expiry = brutex_core::instrument::Expiry::new(2025, 7, 31).expect("an expiry");
    let wanted: Vec<pull::fno::Found> = strikes
        .iter()
        .map(|strike| {
            pull::fno::read_contract(&format!("NSE-NIFTY-31Jul25-{strike}-CE"), expiry)
                .expect("a named contract")
        })
        .collect();
    fno_land(&wanted, &asked, &site, &wire).await
}

/// **The named-contract walk stops on the first rejection as well.**
///
/// `fetch_chain_chunks` prefixed the contract's symbol IN FRONT of the
/// marker, which buried the verdict where no reader looks, and `fno_land`
/// sent the dead token to every remaining contract. Three contracts, one
/// request.
#[tokio::test]
async fn a_rejection_mid_contract_walk_stops_every_remaining_contract() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(&vendor, vec![token("stale")]);

    let landed = named_walk("same", &vendor, &script, THREE_STRIKES).await;

    assert_eq!(vendor.seen().len(), 1, "{:?}", vendor.seen());
    assert_eq!(landed.credential_stop, Some(CredentialStop::SameValue));
    assert_eq!(
        landed.failed, 3,
        "the rejected contract and the two not asked"
    );
    assert!(landed.why[0].contains("same value"), "{:?}", landed.why);
    assert!(
        landed.why.iter().all(|why| !why.contains('\u{3}')),
        "{:?}",
        landed.why
    );
}

/// **A rotation mid-contract-walk sends the remaining contracts with the new
/// token.**
#[tokio::test]
async fn a_rotation_mid_contract_walk_sends_the_rest_with_the_new_token() {
    let vendor = FakeVendor::rejecting_all_but(Some("fresh")).await;
    let script = scripted(&vendor, vec![token("fresh")]);

    let landed = named_walk("rotated", &vendor, &script, THREE_STRIKES).await;

    let seen = vendor.seen();
    assert_eq!(seen.len(), 3, "every contract was asked: {seen:?}");
    assert!(seen[0].contains("stale"));
    assert!(
        seen[1].contains("fresh") && seen[2].contains("fresh"),
        "{seen:?}"
    );
    assert_eq!(landed.credential_stop, None);
    assert_eq!(script.reads(), 1);
}

/// **An undone rotation mid-contract-walk is caught before the socket.**
/// v3a-1, D-1482. `stale` rejected, `fresh` read and rejected, `stale` read
/// again: the third contract is never asked.
#[tokio::test]
async fn an_undone_rotation_mid_contract_walk_never_resends_a_rejected_token() {
    let vendor = FakeVendor::rejecting_all_but(None).await;
    let script = scripted(&vendor, vec![token("fresh"), token("stale")]);

    let landed = named_walk("flapped", &vendor, &script, THREE_STRIKES).await;

    let seen = vendor.seen();
    assert_eq!(seen.len(), 2, "stale once, fresh once: {seen:?}");
    assert!(
        seen[0].contains("stale") && seen[1].contains("fresh"),
        "{seen:?}"
    );
    assert_eq!(script.reads(), 2);
    assert_eq!(landed.credential_stop, Some(CredentialStop::SameValue));
    assert_eq!(landed.failed, 3, "two rejected and one not asked");
    assert!(
        landed
            .why
            .iter()
            .any(|why| why.contains("already rejected in this run")),
        "{:?}",
        landed.why
    );
}

// ------------------------------------------------------------ the lanes

/// A loopback vendor that accepts every request after `hold`, and records the
/// most requests it ever held open at once.
struct SlowVendor {
    base: &'static str,
    most: Arc<std::sync::atomic::AtomicUsize>,
    seen: Arc<std::sync::atomic::AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl SlowVendor {
    async fn holding(hold: std::time::Duration) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        let open = Arc::new(AtomicUsize::new(0));
        let most = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(AtomicUsize::new(0));
        let (open_in, most_in, seen_in) = (Arc::clone(&open), Arc::clone(&most), Arc::clone(&seen));
        let app = axum::Router::new().fallback(move || {
            let (open, most, seen) = (
                Arc::clone(&open_in),
                Arc::clone(&most_in),
                Arc::clone(&seen_in),
            );
            async move {
                let now = open.fetch_add(1, Ordering::SeqCst) + 1;
                let _ = seen.fetch_add(1, Ordering::SeqCst);
                let _ = most.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(hold).await;
                let _ = open.fetch_sub(1, Ordering::SeqCst);
                (axum::http::StatusCode::OK, "{}")
            }
        });
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("the fake serves");
        });
        Self {
            base: Box::leak(format!("http://{address}").into_boxed_str()),
            most,
            seen,
            task,
        }
    }
}

impl Drop for SlowVendor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// **A spot run keeps up to `BROKER_LANES` instruments on the wire at once,
/// and never more.**
///
/// It was serial: one instrument's whole round trip before the next was
/// asked, so a feed's rate was set by latency rather than by its budget.
/// Seven indices against a vendor that holds every request open: the vendor
/// sees all seven, and at most three of them were ever open together, and at
/// least two were. The first instrument goes out alone, because a run widens
/// only after the vendor answered without rejecting the token. D-3002.
#[tokio::test]
async fn a_spot_run_keeps_up_to_three_instruments_on_the_wire_and_never_more() {
    let vendor = SlowVendor::holding(std::time::Duration::from_millis(300)).await;
    let script = Arc::new(Script::new(vendor.base, vec![token("live"); 7]));
    let dir = crate::scratch::path("credential-law-lanes");
    std::fs::create_dir_all(&dir).expect("masters dir");
    std::fs::write(
        dir.join("dhan_scrip.csv"),
        "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,INSTRUMENT_TYPE,\
         SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n\
         NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,13\n\
         NSE,I,NA,INDEX,BANKNIFTY,BANKNIFTY,INDEX,NA,0001-01-01,,,25\n\
         NSE,I,NA,INDEX,FINNIFTY,FINNIFTY,INDEX,NA,0001-01-01,,,27\n\
         NSE,I,NA,INDEX,MIDCPNIFTY,MIDCPNIFTY,INDEX,NA,0001-01-01,,,442\n\
         NSE,I,NA,INDEX,NIFTYIT,NIFTYIT,INDEX,NA,0001-01-01,,,29\n\
         NSE,I,NA,INDEX,NIFTYMETAL,NIFTYMETAL,INDEX,NA,0001-01-01,,,31\n\
         NSE,I,NA,INDEX,NIFTYPHARMA,NIFTYPHARMA,INDEX,NA,0001-01-01,,,32\n",
    )
    .expect("dhan master");
    let root = crate::scratch::path("credential-law-store-lanes");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("manifest")).expect("store root");
    let mut site = Site::serving(&dir, &root);
    site.credentials = Credentials::Scripted(Arc::clone(&script));
    let asked = ingest::parse_spot(
        "target=indices&vendor=dhan&granularity=1day&from=2026-08-03&to=2026-08-05",
        Day::new(2026, 8, 10).expect("a day"),
    )
    .expect("a finished daily window");

    let run = broker_run(&asked, &site, &[]).await;

    assert!(run.blocked.is_none(), "the run reached its loop: {run:?}");
    assert_eq!(run.attempted, 7, "seven indices were named: {run:?}");
    let seen = vendor.seen.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(seen, 7, "every instrument was asked once: {run:?}");
    let most = vendor.most.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        most >= 2,
        "the run is no longer serial: at most {most} request was ever open"
    );
    assert!(
        most <= BROKER_LANES,
        "never more than {BROKER_LANES} at once, the strictest per-second cap: {most}"
    );
    assert_eq!(script.reads(), 7, "one credential read per instrument");
    assert!(run.touched_wire);
    assert_eq!(run.credential_stop, None);
}

//! Cloud mutation pass B (2026-10-16): the F&O half of `server.rs`.
//!
//! Covers the expired-derivative walk and its helpers -- `fno_facts`,
//! `months_to_walk`, `walk_months`, `owed_chunks`, `fno_land`, `chain_quotes`,
//! `price_chain_month`, `fno_walk`, `fno_roll`, `fno_report`, the rolling
//! helpers and the receipt builders -- with exact assertions on what each
//! returns. Every vendor in this file is a loopback double or a scripted
//! `pull::chain::Discovery`; no test reaches a vendor, AWS or a browser.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

fn day(year: u16, month: u8, of: u8) -> Day {
    Day::new(year, month, of).expect("a real day")
}

fn ym(year: u16, month: u8) -> store::path::YearMonth {
    store::path::YearMonth::new(year, month).expect("a real month")
}

/// The value of the first fact labelled `label`.
fn fact<'a>(facts: &'a [(&'static str, String)], label: &str) -> &'a str {
    facts.iter().find(|(name, _)| *name == label).map_or_else(
        || panic!("no {label:?} fact in {facts:?}"),
        |(_, value)| value.as_str(),
    )
}

// ------------------------------------------------------------- fno_facts

/// `fno_facts` with no expiry names the whole month against today, and a
/// window pulled back to the last settled day says so in its own fact.
/// Kills the `fno_facts -> vec![...]` replacements at server.rs:11272.
#[test]
fn fno_facts_for_a_month_request_name_the_month_and_the_narrowing() {
    let today = day(2026, 8, 19);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2026-07-01&to=2026-08-19",
        today,
    )
    .expect("a month request");
    assert_eq!(
        asked.clamped_from,
        Some(today),
        "the premise: it was clamped"
    );

    let facts = fno_facts(&asked, today);

    let labels: Vec<&str> = facts.iter().map(|(label, _)| *label).collect();
    assert_eq!(
        labels,
        [
            "Underlying",
            "Series",
            "Expiry",
            "Window",
            "From",
            "To",
            "Calendar days",
            "toDate on the wire",
            "Timeframe",
            "Window narrowed",
            "Feed",
        ]
    );
    assert_eq!(fact(&facts, "Underlying"), "NIFTY");
    assert_eq!(fact(&facts, "Series"), "Option chain");
    assert_eq!(
        fact(&facts, "Expiry"),
        "every expiry 2026-07-31 held — the whole month, checked against 2026-08-19"
    );
    assert_eq!(
        fact(&facts, "Window narrowed"),
        "you asked to 2026-08-19, and an expired series never touches the CURRENT MONTH — \
         its contracts have not all expired yet. Taken to 2026-07-31 instead, the last day \
         of the last closed month. Spot is not narrowed this way: its bound is the \
         session's, not the month's"
    );
    assert_eq!(fact(&facts, "Feed"), asked.feed.display());
}

/// A named expiry is stated as expired, and an unclamped window carries no
/// narrowing fact.
#[test]
fn fno_facts_for_one_expiry_carry_no_narrowing_fact() {
    let today = day(2026, 8, 19);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=fut&vendor=groww&expiry=2026-07-31&from=2026-07-01&to=2026-07-31",
        today,
    )
    .expect("a one-expiry request");

    let facts = fno_facts(&asked, today);

    assert_eq!(fact(&facts, "Series"), "Future");
    assert_eq!(
        fact(&facts, "Expiry"),
        "2026-07-31 — expired, checked against 2026-08-19"
    );
    assert!(
        facts.iter().all(|(label, _)| *label != "Window narrowed"),
        "{facts:?}"
    );
    assert_eq!(facts.len(), 10, "{facts:?}");
}

// ------------------------------------------------------------- fixtures

use crate::credential_law::{Credentials, Script, Unreadable};
use axum::http::StatusCode;
use std::sync::{Arc, Mutex};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

/// A claimed scratch root with an empty `masters` directory, removed on drop.
struct Root(PathBuf);

impl Root {
    fn new(name: &str) -> Self {
        let path = crate::scratch::path(&format!("cloudb-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("masters")).expect("a scratch root");
        Self(path)
    }

    fn masters(&self) -> PathBuf {
        self.0.join("masters")
    }

    /// A Dhan scrip master naming NIFTY as security id 13.
    fn with_dhan_nifty(self) -> Self {
        std::fs::write(
            self.masters().join("dhan_scrip.csv"),
            "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,\
             INSTRUMENT_TYPE,SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n\
             NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,13\n",
        )
        .expect("a generated dhan master");
        self
    }

    fn site(&self) -> Site {
        Site::load(&self.masters(), &self.0)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A loopback vendor answering every request through `reply`, keeping the
/// path and query of every request it received, in order.
struct Loopback {
    base: &'static str,
    seen: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Loopback {
    async fn answering(reply: fn(&str) -> (StatusCode, String)) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        let app = axum::Router::new().fallback(move |uri: axum::http::Uri| {
            let asked = uri.to_string();
            log.lock().expect("the loopback log").push(asked.clone());
            let (status, body) = reply(&asked);
            async move {
                (
                    status,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    body,
                )
            }
        });
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("the loopback serves");
        });
        Self {
            base: Box::leak(format!("http://{address}").into_boxed_str()),
            seen,
            task,
        }
    }

    fn seen(&self) -> Vec<String> {
        self.seen.lock().expect("the loopback log").clone()
    }
}

impl Drop for Loopback {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// The shipped HTTP descriptor of `feed`, pointed at `base`.
fn spec_at(feed: pull::vendor::Feed, base: &'static str) -> pull::vendor::HttpSpec {
    let pull::vendor::Transport::Http(shipped) = feed.descriptor().transport else {
        panic!("{} is an HTTP feed", feed.display());
    };
    pull::vendor::HttpSpec {
        base_url: base,
        ..shipped
    }
}

/// A wire to `feed` at `base`. Constructing it opens no socket.
fn wire_at(feed: pull::vendor::Feed, base: &'static str) -> Wire {
    let spec = spec_at(feed, base);
    Wire {
        source: pull::http::HttpSource::new(
            spec,
            pull::http::Credential::token("cloudb-fixture".to_owned()),
        )
        .expect("a loopback source"),
        spec,
        store_vendor: feed.store_vendor().expect("a store prefix"),
    }
}

fn found(name: &str, expiry: brutex_core::instrument::Expiry) -> pull::fno::Found {
    pull::fno::read_contract(name, expiry).expect("a readable generated contract")
}

fn expiry(year: u16, month: u8, of: u8) -> brutex_core::instrument::Expiry {
    brutex_core::instrument::Expiry::new(year, month, of).expect("a real expiry")
}

/// 09:15 IST plus `minute` minutes on `on`, in microseconds.
fn session_micros(on: Day, minute: i64) -> i64 {
    (i64::from(on.days_from_epoch()) * 86_400 + 13_500 + minute * 60) * 1_000_000
}

fn bar_at(ts_micros: i64, close: i64) -> Bar {
    Bar {
        ts_micros,
        open: close,
        high: close,
        low: close,
        close,
        volume: 1,
        open_interest: OI_NULL,
    }
}

fn nifty_slot() -> pull::pricing::SweptSlot {
    pull::pricing::slot_of(brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"))
        .expect("NIFTY has a recorded regime")
}

fn operator_rate() -> pull::pricing::Rate {
    pull::pricing::Rate::measured(
        0.065,
        pull::tenor::YearBasis::Calendar365,
        pull::pricing::RateSource::Operator,
    )
    .expect("a plausible rate")
}

fn inputs(at: brutex_core::instrument::Expiry, rate: pull::pricing::Rate) -> PriceInputs {
    PriceInputs {
        strike: 2_400_000,
        side: brutex_core::instrument::OptionSide::Call,
        expiry: at,
        slot: nifty_slot(),
        rate,
        vendor: Vendor::Groww,
    }
}

fn one_day(on: Day) -> pull::session::Window {
    pull::session::Window::new(on, on).expect("a one-day window")
}

/// Files `bars` under the store path `parts` names.
fn write_bars(root: &Path, parts: PathParts<'_>, bars: &[Bar]) {
    let path = StorePath::new(parts).expect("a generated store path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the same fold the store stamps into the header"
    )]
    let symbol_id = brutex_core::universe::fnv1a(parts.symbol) as u32;
    let mut writer =
        store::file::BarFile::open_or_create(root, path, symbol_id).expect("a generated writer");
    writer.append(bars).expect("generated bars append");
}

fn nifty_parts(
    vendor: Vendor,
    segment: &'static str,
    contract: Option<brutex_core::instrument::Contract>,
    month: store::path::YearMonth,
    file: FileKind,
) -> PathParts<'static> {
    PathParts {
        vendor,
        exchange: "NSE",
        segment,
        symbol: "NIFTY",
        contract,
        timeframe: Timeframe::MINUTE_1,
        month,
        file,
    }
}

fn fixture_now() -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_mins(29_234_895)
}

// ------------------------------------------------------------ walk_months

/// A scripted `Discovery`: answers in order, keeping every URL asked.
struct Scripted {
    answers: Mutex<std::collections::VecDeque<Result<String, pull::chain::Refusal>>>,
    asked: Mutex<Vec<String>>,
}

impl Scripted {
    fn new(answers: Vec<Result<String, pull::chain::Refusal>>) -> Self {
        Self {
            answers: Mutex::new(answers.into()),
            asked: Mutex::new(Vec::new()),
        }
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("the asked log").clone()
    }
}

impl pull::chain::Discovery for Scripted {
    async fn get(&self, url: &str) -> Result<String, pull::chain::Refusal> {
        self.asked
            .lock()
            .expect("the asked log")
            .push(url.to_owned());
        self.answers
            .lock()
            .expect("the script")
            .pop_front()
            .unwrap_or_else(|| {
                Err(pull::chain::Refusal::transport(
                    "script exhausted".to_owned(),
                ))
            })
    }
}

/// `walk_months` keeps walking past a month whose expiry list refuses: the
/// refusal is named in `unreadable` and the later month's contracts arrive.
/// Kills `walk_months -> Ok(Default::default())` and `== with !=` at
/// server.rs:11730.
#[tokio::test]
async fn walk_months_names_a_refused_month_and_keeps_the_next_ones_contracts() {
    let from = Scripted::new(vec![
        Err(pull::chain::Refusal::answered(
            503,
            "the vendor answered 503 Service Unavailable".to_owned(),
        )),
        Ok(r#"{"expiries":["2025-07-31"]}"#.to_owned()),
        Ok(r#"{"contracts":["NSE-NIFTY-31Jul25-24000-CE","NOT A CONTRACT"]}"#.to_owned()),
    ]);

    let chain = walk_months(
        pull::vendor::Feed::Groww,
        "NIFTY",
        &[ym(2025, 6), ym(2025, 7)],
        &from,
    )
    .await
    .expect("one month walked");

    let asked = from.asked();
    assert_eq!(asked.len(), 3, "{asked:?}");
    assert_eq!(chain.expiries, ["2025-07-31"]);
    assert_eq!(
        chain.contracts,
        [found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31))]
    );
    assert_eq!(
        chain.unreadable,
        [
            format!(
                "2025-06: the expiry list could not be walked — {} was not reached: \
                 the vendor answered 503 Service Unavailable",
                asked[0]
            ),
            "NOT A CONTRACT".to_owned(),
        ]
    );
}

/// A walk that discovered nothing anywhere answers the LAST refusal, and a
/// walk over no months at all answers `NoExpiry` rather than an empty chain.
#[tokio::test]
async fn walk_months_with_nothing_discovered_is_an_error() {
    let from = Scripted::new(vec![
        Err(pull::chain::Refusal::transport("first refusal".to_owned())),
        Err(pull::chain::Refusal::transport("second refusal".to_owned())),
    ]);
    let refused = walk_months(
        pull::vendor::Feed::Groww,
        "NIFTY",
        &[ym(2025, 6), ym(2025, 7)],
        &from,
    )
    .await
    .expect_err("nothing was discovered");
    let asked = from.asked();
    assert_eq!(
        refused,
        pull::chain::ChainError::Transport {
            url: asked[1].clone(),
            why: "second refusal".to_owned(),
        }
    );

    let none = walk_months(
        pull::vendor::Feed::Groww,
        "NIFTY",
        &[],
        &Scripted::new(Vec::new()),
    )
    .await
    .expect_err("no month was walked");
    assert_eq!(
        none,
        pull::chain::ChainError::Lookup(pull::fno::FnoError::NoExpiry)
    );
}

// --------------------------------------------------------- months_to_walk

/// The lookahead stops AT the last settled month, inclusive: a window
/// ending in June walked on 2025-08-02 reaches July (the ceiling) and not
/// August. Kills `> with >=`, `> with ==` and `> with <` at server.rs:11636
/// and the `vec![...]` replacements at server.rs:11619.
#[test]
fn months_to_walk_reaches_the_last_settled_month_and_no_further() {
    let today = day(2025, 8, 2);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-05-20&to=2025-06-10",
        today,
    )
    .expect("a closed window");
    assert_eq!(
        months_to_walk(&asked, today),
        [ym(2025, 5), ym(2025, 6), ym(2025, 7)]
    );

    // Far from the ceiling, all three lookahead months are walked.
    let early = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-01-02&to=2025-01-03",
        today,
    )
    .expect("a closed window");
    assert_eq!(
        months_to_walk(&early, today),
        [ym(2025, 1), ym(2025, 2), ym(2025, 3), ym(2025, 4)]
    );

    // A named expiry walks its own month only.
    let named = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&expiry=2025-06-26&from=2025-06-02&to=2025-06-20",
        today,
    )
    .expect("a named expiry");
    assert_eq!(months_to_walk(&named, today), [ym(2025, 6)]);
}

/// A window in the last month the store can name stops the lookahead at
/// once: there is no month after 9999-12 to ask about.
#[test]
fn months_to_walk_stops_at_the_last_nameable_month() {
    let asked = ingest::FnoRequest {
        underlying: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
        series: ingest::Series::Options,
        expiry: None,
        window: pull::session::Window::new(day(9999, 12, 1), day(9999, 12, 2))
            .expect("a window at the end of the calendar"),
        rate: None,
        clamped_from: None,
        feed: pull::vendor::Feed::Groww,
        granularity: pull::vendor::Granularity::Minute1,
    };
    assert_eq!(months_to_walk(&asked, day(2025, 8, 2)), [ym(9999, 12)]);
}

// ------------------------------------------------------------ owed_chunks

/// A rung with no store directory asks for the whole window, split to the
/// cap, and claims nothing as held. Kills the four `owed_chunks -> (vec![..],
/// n)` replacements at server.rs:10567.
#[test]
fn owed_chunks_without_a_store_rung_asks_for_the_whole_window() {
    let contract = found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31));
    let window = pull::session::Window::new(day(2025, 7, 1), day(2025, 7, 3)).expect("a window");
    let (chunks, done) = owed_chunks(&contract, &[ym(2025, 7)], None, window, Some(1), |_| {
        panic!("no store rung, so nothing is probed")
    });
    assert_eq!(
        chunks,
        [
            one_day(day(2025, 7, 1)),
            one_day(day(2025, 7, 2)),
            one_day(day(2025, 7, 3)),
        ]
    );
    assert_eq!(done, 0);

    // A zero cap cannot be split, so the operator's window is asked whole.
    let (whole, none) = owed_chunks(&contract, &[ym(2025, 7)], None, window, Some(0), |_| None);
    assert_eq!(whole, [window]);
    assert_eq!(none, 0);
}

// --------------------------------------------------------------- fno_land

/// A window that cannot be split to the feed's cap fails EVERY contract
/// before a request is sent, and says why. Kills `delete field failed` and
/// `delete field why` at server.rs:10654-10655 and `fno_land ->
/// Default::default()` at server.rs:10614.
#[tokio::test]
async fn fno_land_with_an_unsplittable_window_fails_every_contract_unsent() {
    let root = Root::new("land-zero-cap");
    let site = root.site();
    let today = day(2025, 8, 2);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01",
        today,
    )
    .expect("a named-chain request");
    let mut wire = wire_at(pull::vendor::Feed::Groww, "http://127.0.0.1:0");
    wire.spec.window_caps = &[(pull::vendor::Granularity::Minute1, 0)];
    let wanted = [
        found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31)),
        found("NSE-NIFTY-31Jul25-24050-CE", expiry(2025, 7, 31)),
    ];

    let landed = fno_land(&wanted, &asked, &site, &wire).await;

    let refusal =
        pull::session::split_window(asked.window, Some(0)).expect_err("a zero cap cannot be split");
    assert_eq!(landed.failed, 2);
    assert_eq!(
        landed.why,
        [format!(
            "the window could not be split to this feed's cap, so no contract was \
             asked for: {refusal}"
        )]
    );
    assert_eq!(landed.stored, 0);
    assert_eq!(landed.rows_read, 0);
    assert_eq!(landed.settled, 0);
    assert_eq!(landed.credential_stop, None);
}

// ----------------------------------------------------------- chain_quotes

/// `chain_quotes` keeps only the chunk's own days and refuses, by counted
/// reason, a stamp no calendar places and a bar with no index bar behind it.
/// Kills every mutant on `day < from || day > to` at server.rs:11059 (the
/// out-of-chunk bars have index bars, so a mutant would quote them) and the
/// `vec![...]` replacements at server.rs:11043.
#[test]
fn chain_quotes_quote_only_the_chunk_and_count_each_refusal() {
    let on = day(2025, 7, 1);
    let contract = found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31));
    let inside = session_micros(on, 0);
    let before = session_micros(day(2025, 6, 30), 0);
    let after = session_micros(day(2025, 7, 2), 0);
    let unbooked = session_micros(on, 1);
    let book = pull::pricing::SpotBook::of(&[
        bar_at(inside, 2_400_000),
        bar_at(before, 2_400_000),
        bar_at(after, 2_400_000),
    ]);
    let month_of = ChainMonth {
        found: &contract,
        chunk: one_day(on),
        timeframe: Timeframe::MINUTE_1,
        inputs: inputs(expiry(2025, 7, 31), operator_rate()),
    };
    let bars = [
        bar_at(i64::MAX, 1),
        bar_at(before, 7_000),
        bar_at(inside, 8_000),
        bar_at(unbooked, 9_000),
        bar_at(after, 7_500),
    ];
    let mut out = pull::pricing::PricedAll::default();

    let quotes = chain_quotes(&bars, month_of, &book, &mut out);

    assert_eq!(quotes.len(), 1, "{quotes:?}");
    let quote = quotes[0];
    assert_eq!(quote.ts_micros, inside);
    assert_eq!(quote.spot, 2_400_000);
    assert_eq!(quote.strike, 2_400_000);
    assert_eq!(quote.premium, 8_000);
    assert_eq!(quote.side, brutex_core::instrument::OptionSide::Call);
    assert_eq!(quote.vendor, Vendor::Groww);
    assert_eq!(out.refused, 2);
    assert_eq!(
        out.why,
        [
            "a stored bar's stamp is not a moment any calendar places",
            "no index bar is stored at this option bar's stamp, so it has no underlying \
             level to price against. Nothing was borrowed from a neighbouring minute",
        ]
    );
}

/// A bar at or past the contract's expiry, and a bar on a day before the
/// cost calendar begins, are each refused with their own reason.
#[test]
fn chain_quotes_refuse_an_expired_bar_and_a_day_outside_the_cost_calendar() {
    let on = day(2025, 7, 1);
    let contract = found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31));
    let at = session_micros(on, 0);
    let book = pull::pricing::SpotBook::of(&[bar_at(at, 2_400_000)]);
    let mut out = pull::pricing::PricedAll::default();
    let expired = ChainMonth {
        found: &contract,
        chunk: one_day(on),
        timeframe: Timeframe::MINUTE_1,
        inputs: inputs(expiry(2025, 6, 26), operator_rate()),
    };
    assert!(chain_quotes(&[bar_at(at, 8_000)], expired, &book, &mut out).is_empty());
    assert_eq!(out.refused, 1);
    assert_eq!(
        out.why,
        ["the bar is at or past the moment the contract stopped trading"]
    );

    let early = day(1989, 12, 28);
    let then = session_micros(early, 0);
    let early_book = pull::pricing::SpotBook::of(&[bar_at(then, 100_000)]);
    let mut before_costs = pull::pricing::PricedAll::default();
    let ancient = ChainMonth {
        found: &contract,
        chunk: one_day(early),
        timeframe: Timeframe::MINUTE_1,
        inputs: inputs(expiry(1990, 1, 25), operator_rate()),
    };
    assert!(
        chain_quotes(
            &[bar_at(then, 8_000)],
            ancient,
            &early_book,
            &mut before_costs
        )
        .is_empty()
    );
    assert_eq!(before_costs.refused, 1);
    assert_eq!(
        before_costs.why,
        ["the bar's own day is outside the cost calendar"]
    );
}

// ------------------------------------------------------ price_chain_month

/// A month whose index bars are on disk but whose option bars are not is
/// refused with the read-back reason, and nothing is priced or filed.
#[test]
fn price_chain_month_names_option_bars_that_cannot_be_read_back() {
    let root = Root::new("chain-month-readback");
    let site = root.site();
    let today = day(2025, 8, 2);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01&rate=0.065",
        today,
    )
    .expect("a priced request");
    let wire = wire_at(pull::vendor::Feed::Groww, "http://127.0.0.1:0");
    let on = day(2025, 7, 1);
    write_bars(
        &root.0,
        nifty_parts(
            Vendor::Groww,
            brutex_core::instrument::Segment::Index.as_str(),
            None,
            ym(2025, 7),
            FileKind::Bars,
        ),
        &[bar_at(session_micros(on, 0), 2_400_000)],
    );
    let contract = found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31));
    let expected = read_month_bars(
        &site,
        &wire,
        "NIFTY",
        brutex_core::instrument::Segment::Fno.as_str(),
        Some(contract.contract),
        Timeframe::MINUTE_1,
        ym(2025, 7),
    )
    .expect_err("no option bars were written");
    let mut books = SpotBooks::new();

    let out = price_chain_month(
        ChainMonth {
            found: &contract,
            chunk: one_day(on),
            timeframe: Timeframe::MINUTE_1,
            inputs: inputs(expiry(2025, 7, 31), operator_rate()),
        },
        &mut books,
        &asked,
        &site,
        &wire,
    );

    assert!(out.rows.is_empty());
    assert_eq!(out.refused, 0);
    assert_eq!(
        out.why,
        [format!(
            "the bars just written cannot be read back: {expected}"
        )]
    );
    assert_eq!(
        books.len(),
        1,
        "the index month is cached for the next contract"
    );
    assert!(books[&ym(2025, 7)].is_ok());
}

/// Option bars that the model refuses to price are counted, and the model's
/// own reason reaches the month's notes; nothing is filed.
#[test]
fn price_chain_month_carries_the_models_refusals() {
    let root = Root::new("chain-month-model");
    let site = root.site();
    let today = day(2025, 8, 2);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01&rate=0.065",
        today,
    )
    .expect("a priced request");
    let wire = wire_at(pull::vendor::Feed::Groww, "http://127.0.0.1:0");
    let on = day(2025, 7, 1);
    let at = session_micros(on, 0);
    write_bars(
        &root.0,
        nifty_parts(
            Vendor::Groww,
            brutex_core::instrument::Segment::Index.as_str(),
            None,
            ym(2025, 7),
            FileKind::Bars,
        ),
        &[bar_at(at, 2_400_000)],
    );
    let contract = found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31));
    // A call premium twenty times the spot: no volatility prices it.
    write_bars(
        &root.0,
        nifty_parts(
            Vendor::Groww,
            brutex_core::instrument::Segment::Fno.as_str(),
            Some(contract.contract),
            ym(2025, 7),
            FileKind::Bars,
        ),
        &[bar_at(at, 48_000_000)],
    );
    let mut books = SpotBooks::new();

    let out = price_chain_month(
        ChainMonth {
            found: &contract,
            chunk: one_day(on),
            timeframe: Timeframe::MINUTE_1,
            inputs: inputs(expiry(2025, 7, 31), operator_rate()),
        },
        &mut books,
        &asked,
        &site,
        &wire,
    );

    assert!(out.rows.is_empty(), "{out:?}");
    assert_eq!(out.refused, 1, "{out:?}");
    assert_eq!(out.why, [MODEL_SUPREMUM], "the model's own reason: {out:?}");
    let greeks = StorePath::new(nifty_parts(
        Vendor::Groww,
        brutex_core::instrument::Segment::Fno.as_str(),
        Some(contract.contract),
        ym(2025, 7),
        FileKind::Greeks,
    ))
    .expect("a greek path")
    .to_path_buf(&root.0);
    assert!(!greeks.exists(), "nothing priced, nothing filed");
}

// ------------------------------------------------------------ price_group

fn rolling_row(ts_micros: i64, close: i64, spot: i64, iv_micros: i64) -> pull::rolling::Row {
    pull::rolling::Row {
        bar: bar_at(ts_micros, close),
        overlay: store::format::Overlay {
            ts_micros,
            spot,
            iv_micros,
        },
        strike: Some(2_400_000),
    }
}

/// `price_group` refuses a row with no vendor spot, skips a row outside the
/// window, prices a row with a vendor volatility, and `greek_records` files
/// its rate provenance as the CHARTER class when the rate came from there.
#[test]
fn price_group_refuses_a_spotless_row_and_files_charter_provenance() {
    let on = day(2025, 7, 1);
    let rate = pull::pricing::Rate::measured(
        0.065,
        pull::tenor::YearBasis::Calendar365,
        pull::pricing::RateSource::Charter("cloud pass B generated citation"),
    )
    .expect("a cited rate");
    let group = [
        rolling_row(session_micros(on, 0), 10_000, OI_NULL, 200_000),
        rolling_row(session_micros(on, 1), 10_000, 2_400_000, 200_000),
        rolling_row(
            session_micros(day(2025, 6, 30), 0),
            10_000,
            2_400_000,
            200_000,
        ),
    ];

    let done = price_group(
        &group,
        inputs(expiry(2025, 7, 31), rate),
        one_day(on),
        pull::vendor::Granularity::Minute1.cadence(),
    );

    assert_eq!(done.refused, 1, "{done:?}");
    // COMPARED WORD BY WORD: the production literal carries a run of 18
    // spaces where a line continuation was lost (server.rs:12558, reported).
    assert_eq!(done.why.len(), 1, "{done:?}");
    assert_eq!(
        done.why[0].split_whitespace().collect::<Vec<_>>().join(" "),
        "the vendor sent no underlying level beside this bar, so there is nothing to \
         price it against"
    );
    assert_eq!(done.rows.len(), 1, "{done:?}");
    let row = &done.rows[0];
    assert_eq!(row.ts_micros, session_micros(on, 1));
    assert_eq!(row.volatility.to_bits(), 0.2_f64.to_bits());
    assert!(matches!(row.vol_from, pull::pricing::VolSource::Vendor(_)));

    let records = greek_records(&done);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].ts_micros, row.ts_micros);
    assert_eq!(records[0].spot, 2_400_000);
    assert_eq!(records[0].rate.to_bits(), 0.065_f64.to_bits());
    assert_eq!(
        records[0].provenance,
        store::format::Greek::provenance_of(
            store::format::VOL_FROM_VENDOR,
            store::format::RATE_FROM_CHARTER,
            row.below_validated_band,
            row.moneyness.steps,
        )
    );
}

/// A rate solved against a vendor's volatility is filed as the SOLVED class,
/// and a volatility this build solved is filed as such.
#[test]
fn greek_records_file_a_solved_rate_and_a_solved_volatility() {
    let on = day(2025, 7, 1);
    let rate = pull::pricing::Rate::measured(
        0.065,
        pull::tenor::YearBasis::Calendar365,
        pull::pricing::RateSource::SolvedAgainst(Vendor::Dhan),
    )
    .expect("a solved rate");
    let group = [rolling_row(
        session_micros(on, 0),
        50_000,
        2_400_000,
        OI_NULL,
    )];

    let done = price_group(
        &group,
        inputs(expiry(2025, 7, 31), rate),
        one_day(on),
        pull::vendor::Granularity::Minute1.cadence(),
    );

    assert_eq!(done.refused, 0, "{done:?}");
    assert_eq!(done.rows.len(), 1, "{done:?}");
    let row = &done.rows[0];
    assert!(matches!(
        row.vol_from,
        pull::pricing::VolSource::Solved { .. }
    ));
    let records = greek_records(&done);
    assert_eq!(
        records[0].provenance,
        store::format::Greek::provenance_of(
            store::format::VOL_FROM_SOLVED,
            store::format::RATE_FROM_SOLVED,
            row.below_validated_band,
            row.moneyness.steps,
        )
    );
}

/// An expired bar, a pre-calendar day and an unpriceable premium are each
/// refused, and the model's refusal reaches the notes.
#[test]
fn price_group_refuses_expired_ancient_and_unpriceable_rows() {
    let on = day(2025, 7, 1);
    let cadence = pull::vendor::Granularity::Minute1.cadence();
    let expired = price_group(
        &[rolling_row(
            session_micros(on, 0),
            10_000,
            2_400_000,
            200_000,
        )],
        inputs(expiry(2025, 6, 26), operator_rate()),
        one_day(on),
        cadence,
    );
    let tenor = pull::tenor::Tenor::between(session_micros(on, 0), expiry(2025, 6, 26))
        .expect_err("the bar is after the expiry");
    assert_eq!(expired.refused, 1);
    assert_eq!(expired.why, [tenor.to_string()]);
    assert!(expired.rows.is_empty());

    let early = day(1989, 12, 28);
    let ancient = price_group(
        &[rolling_row(
            session_micros(early, 0),
            10_000,
            100_000,
            200_000,
        )],
        inputs(expiry(1990, 1, 25), operator_rate()),
        one_day(early),
        cadence,
    );
    assert_eq!(ancient.refused, 1, "{ancient:?}");
    assert_eq!(
        ancient.why,
        ["the bar's own day is outside the cost calendar"]
    );

    let unpriceable = price_group(
        &[rolling_row(
            session_micros(on, 0),
            48_000_000,
            2_400_000,
            OI_NULL,
        )],
        inputs(expiry(2025, 7, 31), operator_rate()),
        one_day(on),
        cadence,
    );
    assert!(unpriceable.rows.is_empty(), "{unpriceable:?}");
    assert_eq!(unpriceable.refused, 1, "{unpriceable:?}");
    assert_eq!(unpriceable.why, [MODEL_SUPREMUM], "{unpriceable:?}");
}

/// `millionths_to_decimal` divides by a million. Kills the `0.0`, `1.0`,
/// `-1.0` replacements and `/ with %`, `/ with *` at server.rs:12621-12630.
#[test]
fn millionths_to_decimal_is_a_division_by_a_million() {
    assert_eq!(
        millionths_to_decimal(125_000).to_bits(),
        0.125_f64.to_bits()
    );
    assert_eq!(
        millionths_to_decimal(2_500_000).to_bits(),
        2.5_f64.to_bits()
    );
    assert_eq!(
        millionths_to_decimal(-500_000).to_bits(),
        (-0.5_f64).to_bits()
    );
}

/// `note_price_refusal` keeps each distinct reason once and at most
/// `REASONS_KEPT` of them. Kills the mutants at server.rs:12640.
#[test]
fn note_price_refusal_dedups_and_caps() {
    let mut out = pull::pricing::PricedAll::default();
    note_price_refusal(&mut out, "a");
    note_price_refusal(&mut out, "a");
    assert_eq!(out.why, ["a"]);
    for why in ["b", "c", "d", "e", "f", "g"] {
        note_price_refusal(&mut out, why);
    }
    assert_eq!(out.why, ["a", "b", "c", "d", "e"]);
    assert_eq!(out.why.len(), pull::pricing::REASONS_KEPT);
}

// ------------------------------------------------------------ greek_facts

fn counted(rows: usize, solved: usize, refused: usize, below_band: usize) -> PricedCount {
    PricedCount {
        rows,
        filed: rows,
        refused,
        solved,
        below_band,
        why: Vec::new(),
    }
}

/// Every priced row below the validated band is stated, and the filed count
/// is stated beside it. Kills the `> with >=/==/<` mutants at
/// server.rs:12088 and the `vec![...]` replacements at server.rs:12069.
#[test]
fn greek_facts_state_rows_below_the_validated_band() {
    let facts = greek_facts(&counted(4, 1, 0, 1), true);
    assert_eq!(
        facts,
        [
            (
                "Rows priced",
                "4 (1 solved here, 3 sent by the vendor)".to_owned()
            ),
            (
                "Rows below the validated band",
                "1 of 4 — under 0.02 years, the maturity crates/greeks does not check \
                 its own derivatives at"
                    .to_owned()
            ),
            (
                "Where the greeks were stored",
                "4 confirmed stored alongside their option bars in .grk files \
                 (including exact replays)"
                    .to_owned()
            ),
        ]
    );

    // None below the band, so no such line.
    let clear = greek_facts(&counted(4, 1, 0, 0), true);
    assert_eq!(clear.len(), 2, "{clear:?}");
    assert!(
        clear
            .iter()
            .all(|(label, _)| *label != "Rows below the validated band")
    );
}

/// One refused row is stated with its reasons; reasons with nothing refused
/// are notes; nothing filed is said so. Kills `> with >=/==/<` at
/// server.rs:12080, `delete !` at server.rs:12085 and `== with !=` at
/// server.rs:12107.
#[test]
fn greek_facts_state_refusals_notes_and_an_empty_filing() {
    let mut refused = counted(0, 0, 1, 0);
    refused.why = vec!["first".to_owned(), "second".to_owned()];
    assert_eq!(
        greek_facts(&refused, false),
        [
            (
                "Rows priced",
                "0 (0 solved here, 0 sent by the vendor)".to_owned()
            ),
            (
                "Rows that could not be priced",
                "1 — first · second".to_owned()
            ),
            (
                "Where the greeks were stored",
                "no Greek rows were confirmed written".to_owned()
            ),
        ]
    );

    let mut noted = counted(0, 0, 0, 0);
    noted.why = vec!["the index month could not be read".to_owned()];
    assert!(
        !noted.ran(),
        "the premise: nothing priced and nothing refused"
    );
    assert_eq!(
        greek_facts(&noted, true),
        [
            (
                "Rows priced",
                "0 (0 solved here, 0 sent by the vendor)".to_owned()
            ),
            (
                "Pricing notes",
                "the index month could not be read".to_owned()
            ),
            (
                "Where the greeks were stored",
                "no Greek rows were confirmed written".to_owned()
            ),
        ]
    );
}

/// Nothing priced and no rate: the receipt says why there are no greeks;
/// nothing priced WITH a rate says nothing at all. Kills `|| with &&` at
/// server.rs:12070 and `delete !` at server.rs:12070.
#[test]
fn greek_facts_without_a_run_explain_only_a_missing_rate() {
    let idle = PricedCount::default();
    let facts = greek_facts(&idle, true);
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].0, "Greeks");
    assert!(
        facts[0]
            .1
            .starts_with("not computed — no risk-free rate was supplied"),
        "{facts:?}"
    );
    assert!(greek_facts(&idle, false).is_empty());
}

/// `ran` is true when anything priced OR anything refused, and false for
/// zero of both. Kills every mutant at server.rs:12779.
#[test]
fn priced_count_ran_reads_both_counts() {
    assert!(counted(1, 0, 0, 0).ran());
    assert!(counted(0, 0, 1, 0).ran());
    assert!(!counted(0, 0, 0, 0).ran());
}

/// `absorb_count` adds every count and merges reasons distinct and capped.
/// Kills the mutants at server.rs:12765-12771.
#[test]
fn priced_count_absorb_count_sums_and_merges_reasons() {
    let mut into = PricedCount {
        rows: 1,
        filed: 2,
        refused: 3,
        solved: 4,
        below_band: 5,
        why: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
    };
    let from = PricedCount {
        rows: 10,
        filed: 20,
        refused: 30,
        solved: 40,
        below_band: 50,
        why: vec![
            "b".to_owned(),
            "d".to_owned(),
            "e".to_owned(),
            "f".to_owned(),
        ],
    };
    into.absorb_count(&from);
    assert_eq!(
        into,
        PricedCount {
            rows: 11,
            filed: 22,
            refused: 33,
            solved: 44,
            below_band: 55,
            why: ["a", "b", "c", "d", "e"].map(str::to_owned).to_vec(),
        }
    );
}

/// `absorb` folds a priced month in, reasons distinct and capped. Kills the
/// mutants at server.rs:12752-12757.
#[test]
fn priced_count_absorb_folds_a_month() {
    let mut into = PricedCount {
        why: ["a", "b", "c", "d"].map(str::to_owned).to_vec(),
        ..PricedCount::default()
    };
    let month = pull::pricing::PricedAll {
        rows: Vec::new(),
        refused: 7,
        why: ["a", "x", "y"].map(str::to_owned).to_vec(),
    };
    into.absorb(&month);
    assert_eq!(into.refused, 7);
    assert_eq!(into.rows, 0);
    assert_eq!(into.why, ["a", "b", "c", "d", "x"]);
}

/// `Rolled::absorb` adds an `Ok` cell's counts and keeps at most
/// `REASONS_KEPT` reasons; an `Err` cell is one failure, and a
/// dead-credential marker is reported and stripped. Kills the mutants at
/// server.rs:12676 and server.rs:12694.
#[test]
fn rolled_absorb_counts_cells_and_reports_a_dead_credential() {
    let mut walk = Rolled {
        why: ["a", "b", "c", "d"].map(str::to_owned).to_vec(),
        ..Rolled::default()
    };
    let dead = walk.absorb(Ok(Rolled {
        rows_read: 5,
        stored: 4,
        declined: 3,
        failed: 2,
        why: vec!["e".to_owned(), "f".to_owned()],
        priced: counted(1, 1, 0, 0),
    }));
    assert!(!dead);
    assert_eq!(walk.rows_read, 5);
    assert_eq!(walk.stored, 4);
    assert_eq!(walk.declined, 3);
    assert_eq!(walk.failed, 2);
    assert_eq!(walk.why, ["a", "b", "c", "d", "e"]);
    assert_eq!(walk.priced.rows, 1);

    let mut fresh = Rolled::default();
    assert!(fresh.absorb(Err(format!("{CREDENTIAL_DEAD}cell: rejected"))));
    assert!(!fresh.absorb(Err("cell: refused".to_owned())));
    assert_eq!(fresh.failed, 2);
    assert_eq!(fresh.why, ["cell: rejected", "cell: refused"]);
}

/// `declined_note` states the count and that declining is the rule.
#[test]
fn declined_note_states_the_count() {
    assert_eq!(
        declined_note(3),
        "3 — the cadence and ordinal resolved to a contract that has not settled. \
         Nothing was filed for them, and that is the rule rather than a fault"
    );
}

// --------------------------------------------------- below_history_floor

fn dhan_rolling() -> pull::vendor::RollingSpec {
    spec_at(pull::vendor::Feed::Dhan, "http://127.0.0.1:0")
        .fno
        .by_offset()
        .expect("Dhan is addressed by offset")
}

/// A window starting below the feed's depth is refused with the depth and
/// the year named; the floor year itself is served. Kills the mutants at
/// server.rs:12834-12835.
#[test]
fn below_history_floor_refuses_only_years_before_the_floor() {
    let today = day(2025, 8, 2);
    let rolling = dhan_rolling();
    assert_eq!(rolling.years_back, 5, "the premise: a five-year floor");
    let ask = |from: &str| {
        ingest::parse_fno(
            &format!("underlying=NIFTY&series=opt&vendor=dhan&from={from}&to={from}"),
            today,
        )
        .expect("a rolling request")
    };
    assert_eq!(
        below_history_floor(&ask("2019-12-31"), rolling, today).as_deref(),
        Some(
            "this feed serves expired options for the last 5 years, so it holds nothing \
             for 2019. Nothing was sent: an out-of-range answer and an empty one are the \
             same bytes, and a run that cannot tell them apart reports a month as swept \
             that was never held"
        )
    );
    assert_eq!(
        below_history_floor(&ask("2020-01-02"), rolling, today),
        None
    );
    assert_eq!(
        below_history_floor(&ask("2024-01-02"), rolling, today),
        None
    );
}

// ------------------------------------------------ rolling identity helpers

/// No master row for the underlying is a refusal that says so; an index row
/// is its id and `true`. Kills `rolling_security_id -> Ok((Default, _))` at
/// server.rs:12885 and `== with !=` at server.rs:12917.
#[test]
fn rolling_security_id_reads_the_index_row_or_refuses() {
    let today = day(2025, 8, 2);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01",
        today,
    )
    .expect("a rolling request");
    let wire = wire_at(pull::vendor::Feed::Dhan, "http://127.0.0.1:0");

    let empty = Root::new("security-id-empty");
    assert_eq!(
        rolling_security_id(&asked, &empty.site(), &wire),
        Err(
            "this vendor's instrument master lists no id for the underlying, as either \
             an index or an equity, so there is nothing to name it by. Refused rather \
             than sending another vendor's id, which would ask for a different \
             instrument and be answered"
                .to_owned()
        )
    );

    let named = Root::new("security-id-named").with_dhan_nifty();
    let (id, is_index) = rolling_security_id(&asked, &named.site(), &wire).expect("NIFTY is 13");
    assert_eq!(id.as_str(), "13");
    assert!(is_index);
}

/// A cadence the descriptor does not name is refused by the pre-flight with
/// the label and the vendor table's own reason. Kills `rolling_preflight ->
/// Ok(())` at server.rs:13485.
#[test]
fn rolling_preflight_names_an_unknown_cadence() {
    let today = day(2025, 8, 2);
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01",
        today,
    )
    .expect("a rolling request");
    let rolling = dhan_rolling();
    let window = one_day(day(2025, 7, 1));
    let why = pull::rolling::listing_of("NIFTY", &rolling, "YEAR", window.from())
        .expect_err("no such cadence");
    assert_eq!(
        rolling_preflight(&asked, &rolling, "YEAR", window, "cell"),
        Err(format!("cell: {why}"))
    );
    assert_eq!(
        rolling_preflight(&asked, &rolling, "MONTH", window, "cell"),
        Ok(())
    );
}

/// A strike wider than the store can render is refused by name; a put is a
/// put. Kills `name_the_contract -> Ok(Default...)` and `== with !=` at
/// server.rs:12161-12167.
#[test]
fn name_the_contract_renders_both_sides_and_refuses_an_unrenderable_strike() {
    let on = day(2025, 7, 31);
    let (contract, side, named) =
        name_the_contract(on, 2_400_000, "PUT", "cell").expect("a nameable put");
    assert_eq!(side, brutex_core::instrument::OptionSide::Put);
    assert_eq!(named, expiry(2025, 7, 31));
    assert_eq!(contract.to_string(), "2025-07-31-2400000-PE");

    let (call, call_side, _) =
        name_the_contract(on, 2_400_000, "CALL", "cell").expect("a nameable call");
    assert_eq!(call_side, brutex_core::instrument::OptionSide::Call);
    assert_eq!(call.to_string(), "2025-07-31-2400000-CE");

    assert_eq!(
        name_the_contract(on, 10_000_000_000, "CALL", "cell"),
        Err("cell: this store cannot name that contract".to_owned())
    );
}

// -------------------------------------------------------- receipt helpers

fn page_for<'a>(
    asked: &'a ingest::FnoRequest,
    today: Day,
    journal: &'a audit::Journal,
) -> FnoPage<'a> {
    FnoPage {
        asked,
        now: fixture_now(),
        today,
        journal,
        broker: Broker::Refused,
    }
}

fn newest(journal: &audit::Journal) -> audit::Record {
    newest_record(journal, &journal.look()).expect("a durable record")
}

fn groww_request(today: Day) -> ingest::FnoRequest {
    ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01",
        today,
    )
    .expect("a named-chain request")
}

/// A rolling walk that declined runs says so on its own line and still
/// completes; one that declined none carries no such line. Kills the three
/// `>` mutants at server.rs:13805 and the `==` mutants at
/// server.rs:13816-13825.
#[test]
fn rolling_receipt_states_declined_runs_and_completes() {
    let root = Root::new("rolling-receipt");
    let site = root.site();
    let journal = site.journal();
    let today = day(2025, 8, 2);
    let asked = groww_request(today);
    let page = page_for(&asked, today, &journal);

    let (status, body) = rolling_receipt(
        &page,
        Vec::new(),
        RollingWalk {
            planned: 2,
            done: Rolled {
                rows_read: 10,
                declined: 1,
                ..Rolled::default()
            },
            credential_stop: None,
        },
    );
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("Contract runs declined"),
        "{}",
        visible(&body)
    );
    assert!(
        body.contains(&render::escape(&declined_note(1))),
        "{}",
        visible(&body)
    );
    assert!(
        body.contains("the rolling walk completed; no new source bars were committed"),
        "{}",
        visible(&body)
    );
    let record = newest(&journal);
    assert_eq!(record.outcome, audit::Outcome::Empty);
    assert_eq!(record.members, 2);
    assert_eq!(record.rows_read, 10);
    assert_eq!(record.bars_stored, 0);
    assert_eq!(record.failures, 0);

    let (status, body) = rolling_receipt(
        &page,
        Vec::new(),
        RollingWalk {
            planned: 1,
            done: Rolled {
                rows_read: 3,
                stored: 3,
                ..Rolled::default()
            },
            credential_stop: None,
        },
    );
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.contains("Contract runs declined"),
        "{}",
        visible(&body)
    );
    assert!(
        body.contains(
            "the rolling walk completed; new source bars were committed under their own \
             expiry and strike"
        ),
        "{}",
        visible(&body)
    );
    assert_eq!(newest(&journal).outcome, audit::Outcome::Stored);
}

/// A failed rolling walk with no kept reason carries no "First reasons"
/// line; with one, it does. Kills `delete !` at server.rs:13837.
#[test]
fn rolling_receipt_failures_carry_reasons_only_when_kept() {
    let root = Root::new("rolling-receipt-failed");
    let site = root.site();
    let journal = site.journal();
    let today = day(2025, 8, 2);
    let asked = groww_request(today);
    let page = page_for(&asked, today, &journal);

    let (status, body) = rolling_receipt(
        &page,
        Vec::new(),
        RollingWalk {
            planned: 4,
            done: Rolled {
                failed: 1,
                ..Rolled::default()
            },
            credential_stop: None,
        },
    );
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(
        body.contains("Failures during rolling requests"),
        "{}",
        visible(&body)
    );
    assert!(!body.contains("First reasons"), "{}", visible(&body));
    let record = newest(&journal);
    assert_eq!(record.outcome, audit::Outcome::Failed);
    assert_eq!(record.failures, 1);
    assert_eq!(record.members, 4);

    let (_, body) = rolling_receipt(
        &page,
        Vec::new(),
        RollingWalk {
            planned: 4,
            done: Rolled {
                failed: 2,
                why: vec!["cloudb first reason".to_owned()],
                ..Rolled::default()
            },
            credential_stop: None,
        },
    );
    assert!(body.contains("First reasons"), "{}", visible(&body));
    assert!(body.contains("cloudb first reason"), "{}", visible(&body));
}

/// `fno_complete` picks Stored for any stored bar, and otherwise one of
/// three Empty sentences by what was read and what was already held. Kills
/// every mutant at server.rs:14070-14078.
#[test]
fn fno_complete_chooses_its_outcome_by_bars_rows_and_held_months() {
    let root = Root::new("fno-complete");
    let site = root.site();
    let journal = site.journal();
    let today = day(2025, 8, 2);
    let asked = groww_request(today);
    let page = page_for(&asked, today, &journal);
    let counts = |rows_read, bars_stored| FnoCounts {
        contracts: 1,
        rows_read,
        bars_stored,
        failures: 0,
    };

    let (status, body) = fno_complete(&page, Vec::new(), counts(5, 1), 0);
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("every owed contract window completed; new source bars were committed"),
        "{}",
        visible(&body)
    );
    assert_eq!(newest(&journal).outcome, audit::Outcome::Stored);
    assert_eq!(newest(&journal).bars_stored, 1);

    let (_, body) = fno_complete(&page, Vec::new(), counts(0, 0), 1);
    assert!(
        body.contains("every contract-month with bars owed"),
        "{}",
        visible(&body)
    );
    assert_eq!(newest(&journal).outcome, audit::Outcome::Empty);

    let (_, body) = fno_complete(&page, Vec::new(), counts(0, 0), 0);
    assert!(
        body.contains("no contract has bars owed within this window"),
        "{}",
        visible(&body)
    );
    assert_eq!(newest(&journal).outcome, audit::Outcome::Empty);

    let (_, body) = fno_complete(&page, Vec::new(), counts(1, 0), 0);
    assert!(
        body.contains("the fetched bars were already stored byte for byte"),
        "{}",
        visible(&body)
    );
    assert_eq!(newest(&journal).outcome, audit::Outcome::Empty);
}

/// A clock in January 1970 has no settled month before it: the name walk's
/// report refuses before touching a contract, and says why.
#[tokio::test]
async fn fno_report_with_no_settled_month_refuses_before_any_fetch() {
    let root = Root::new("report-1970");
    let site = root.site();
    let journal = site.journal();
    let asked = groww_request(day(2025, 8, 2));
    let early = day(1970, 1, 15);
    let why = ingest::last_settled_day(early).expect_err("no month before January 1970");
    let page = page_for(&asked, early, &journal);
    let wire = wire_at(pull::vendor::Feed::Groww, "http://127.0.0.1:0");
    let chain = pull::chain::Chain {
        expiries: vec!["2025-07-31".to_owned()],
        contracts: vec![found("NSE-NIFTY-31Jul25-24000-CE", expiry(2025, 7, 31))],
        unreadable: Vec::new(),
    };

    let (status, body) = fno_report(&page, Vec::new(), &chain, &asked, &site, &wire).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        body.contains(&render::escape(&why.to_string())),
        "{}",
        visible(&body)
    );
    assert!(body.contains("Contracts discovered"), "{}", visible(&body));
    assert!(!body.contains("Contracts asked for"), "{}", visible(&body));
    // No unreadable name, so no such line (kills `delete !` at server.rs:13869).
    assert!(!body.contains("Unreadable names"), "{}", visible(&body));
    let record = newest(&journal);
    assert_eq!(record.outcome, audit::Outcome::NotStarted);
    assert_eq!(record.members, 0);
}

/// The same clock on the offset walk refuses after naming the offsets and
/// before planning a single request.
#[tokio::test]
async fn fno_roll_with_no_settled_month_refuses_before_any_request() {
    let root = Root::new("roll-1970").with_dhan_nifty();
    let site = root.site();
    let journal = site.journal();
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01",
        day(2025, 8, 2),
    )
    .expect("a rolling request");
    let early = day(1970, 1, 15);
    let why = ingest::last_settled_day(early).expect_err("no month before January 1970");
    let page = page_for(&asked, early, &journal);
    let rolling = dhan_rolling();
    let offsets = rolling.offsets_for(rolling.index_word).len();

    let (status, body) = fno_roll(
        &page,
        Vec::new(),
        &asked,
        &site,
        &wire_at(pull::vendor::Feed::Dhan, "http://127.0.0.1:0"),
        rolling,
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        body.contains(&render::escape(&why.to_string())),
        "{}",
        visible(&body)
    );
    assert!(
        body.contains(&format!("strike offset — {offsets} offsets, ATM-relative")),
        "{}",
        visible(&body)
    );
    assert_eq!(newest(&journal).outcome, audit::Outcome::NotStarted);
}

// ---------------------------------------------------------- fno_pull_held

/// A feed this build has no descriptor for is refused by name before any
/// seat is taken.
#[tokio::test]
async fn fno_pull_held_refuses_an_unknown_feed() {
    let root = Root::new("pull-held-unknown");
    let site = root.site();
    let (status, axum::response::Html(body)) =
        fno_pull_held(&site, "underlying=NIFTY&series=opt&vendor=nonsense").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body.contains(&render::escape(
            "\"nonsense\" is not a feed this build has a descriptor for, so there is no \
             seat to take and no vendor to ask."
        )),
        "{}",
        visible(&body)
    );
    assert!(!site.autopilot.feed_seat_held(pull::vendor::Feed::Dhan));
}

/// A feed whose seat another pull holds is refused with 409, naming the
/// feed, and nothing is journaled.
#[tokio::test]
async fn fno_pull_held_refuses_a_held_seat() {
    let root = Root::new("pull-held-seat");
    let site = root.site();
    let journal = site.journal();
    let before = journal.look().records();
    let seat = site
        .autopilot
        .take_seat(pull::vendor::Feed::Groww)
        .expect("the seat is free");

    let (status, axum::response::Html(body)) = fno_pull_held(
        &site,
        "underlying=NIFTY&series=opt&vendor=groww&from=2025-07-01&to=2025-07-01",
    )
    .await;
    drop(seat);

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body.contains(&render::escape(
            "another pull already holds Groww's seat, so this walk was refused BEFORE it \
             asked the vendor for anything"
        )),
        "{}",
        visible(&body)
    );
    assert_eq!(journal.look().records(), before);
}

// --------------------------------------------------------------- fno_walk

fn scripted_site(
    root: &Root,
    base: &'static str,
    answers: Vec<Result<String, Unreadable>>,
) -> Site {
    let mut site = root.site();
    site.credentials = Credentials::Scripted(Arc::new(Script::new(base, answers)));
    site
}

async fn walk(asked: &ingest::FnoRequest, today: Day, site: &Site) -> (StatusCode, String) {
    let journal = site.journal();
    fno_walk(asked, today, fixture_now(), &journal, Broker::Refused, site).await
}

/// A credential that will not read stops the walk before discovery, with
/// the reason the read gave. Covers the `Err` arm at server.rs:11427.
#[tokio::test]
async fn fno_walk_with_an_unreadable_credential_is_not_started() {
    let root = Root::new("walk-no-credential");
    let site = scripted_site(
        &root,
        "http://127.0.0.1:0",
        vec![Err(Unreadable::configuration(
            "cloud pass B: no credential is configured".to_owned(),
        ))],
    );
    let today = day(2025, 8, 2);
    let before = site.journal().look().records();

    let (status, body) = walk(&groww_request(today), today, &site).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        body.contains("cloud pass B: no credential is configured"),
        "{}",
        visible(&body)
    );
    let journal = site.journal();
    assert_eq!(journal.look().records(), before + 2, "opened, then closed");
    let record = newest(&journal);
    assert_eq!(record.outcome, audit::Outcome::NotStarted);
    assert_eq!(record.note, "cloud pass B: no credential is configured");
    // THE OPENING RECORD AND ITS FACT (kills the `fno_opened -> (..)`
    // replacements at server.rs:11346).
    assert!(body.contains("Opened in the journal"), "{}", visible(&body));
    assert!(
        body.contains(
            "yes — its outcome record (Recorded) closes it, so a restart during the walk \
             is seen at the next start"
        ),
        "{}",
        visible(&body)
    );
}

/// The offset feed is routed to `fno_roll`, which refuses futures, a window
/// below its depth and an underlying its master does not name -- each
/// before a request is sent.
#[tokio::test]
async fn fno_walk_routes_the_offset_feed_to_its_refusals() {
    let today = day(2025, 8, 2);
    let cases = [
        (
            "series=fut&from=2025-07-01&to=2025-07-01",
            true,
            "this feed serves expired OPTIONS only, addressed by strike offset — it \
             publishes no expired-futures history, and nothing was sent rather than options \
             being filed under a futures request",
        ),
        (
            "series=opt&from=2019-07-01&to=2019-07-01",
            true,
            "this feed serves expired options for the last 5 years, so it holds nothing for \
             2019.",
        ),
        (
            "series=opt&from=2025-07-01&to=2025-07-01",
            false,
            "this vendor's instrument master lists no id for the underlying",
        ),
    ];
    for (index, (form, named, said)) in cases.into_iter().enumerate() {
        let root = Root::new(&format!("walk-dhan-{index}"));
        let root = if named { root.with_dhan_nifty() } else { root };
        let site = scripted_site(&root, "http://127.0.0.1:0", vec![Ok("cloudb".to_owned())]);
        let asked = ingest::parse_fno(&format!("underlying=NIFTY&vendor=dhan&{form}"), today)
            .expect("a rolling request");

        let (status, body) = walk(&asked, today, &site).await;

        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "{form}: {}",
            visible(&body)
        );
        assert!(
            body.contains(&render::escape(said)),
            "{form}: {}",
            visible(&body)
        );
        let record = newest(&site.journal());
        assert_eq!(record.outcome, audit::Outcome::NotStarted, "{form}");
    }
}

// --------------------------------------------------------------- roll_one

fn rolling_answer(strike_rupees: i64) -> String {
    let first = i64::from(day(2025, 7, 1).days_from_epoch()) * 86_400 + 13_500;
    let stamps: Vec<i64> = (0..3).map(|minute| first + minute * 60).collect();
    serde_json::json!({"data": {"ce": {
        "timestamp": stamps,
        "open": [100, 100, 100], "high": [110, 110, 110],
        "low": [90, 90, 90], "close": [105, 105, 105],
        "volume": [1, 1, 1], "oi": [0, 0, 0],
        "iv": [0.2, 0.2, 0.2], "spot": [24_000, 24_000, 24_000],
        "strike": [strike_rupees, strike_rupees, strike_rupees]
    }, "pe": null}})
    .to_string()
}

fn rolling_plain(_uri: &str) -> (StatusCode, String) {
    (StatusCode::OK, rolling_answer(24_000))
}

fn rolling_unnameable(_uri: &str) -> (StatusCode, String) {
    (StatusCode::OK, rolling_answer(100_000_000))
}

/// The expiry the descriptor resolves MONTH/near to on 2025-07-01.
fn july_expiry() -> Day {
    let e = pull::rolling::expiry_of("NIFTY", &dhan_rolling(), "MONTH", "1", day(2025, 7, 1))
        .expect("a July monthly");
    day(e.year(), e.month(), e.day())
}

async fn roll_once(site: &Site, base: &'static str, last_settled: Day) -> Rolled {
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01",
        day(2025, 8, 2),
    )
    .expect("a rolling request");
    let wire = wire_at(pull::vendor::Feed::Dhan, base);
    roll_one(
        &asked,
        site,
        &wire,
        "13",
        "OPTIDX",
        "MONTH",
        "1",
        "ATM",
        "CALL",
        &format!("{base}/cloudb-rolling"),
        dhan_rolling(),
        asked.window,
        last_settled,
    )
    .await
    .expect("a readable rolling answer")
}

/// A run that resolves to a contract expiring after the last settled day is
/// declined -- neither stored nor failed -- and one settled exactly on that
/// day is filed. Kills `> with >=/==/<` at server.rs:11911.
#[tokio::test]
async fn roll_one_declines_an_unsettled_contract_and_files_a_settled_one() {
    let vendor = Loopback::answering(rolling_plain).await;
    let expiry_day = july_expiry();
    let previous = Day::from_days(expiry_day.days_from_epoch() - 1).expect("the day before");

    let root = Root::new("roll-declined");
    let declined = roll_once(&root.site(), vendor.base, previous).await;
    assert_eq!(
        declined,
        Rolled {
            rows_read: 3,
            declined: 1,
            ..Rolled::default()
        }
    );

    let filed_root = Root::new("roll-settled");
    let filed = roll_once(&filed_root.site(), vendor.base, expiry_day).await;
    assert_eq!(
        filed,
        Rolled {
            rows_read: 3,
            stored: 3,
            ..Rolled::default()
        }
    );
    assert_eq!(vendor.seen().len(), 2);
}

/// A run whose resolved strike the store cannot render is ONE counted
/// failure with the label and the reason, not the end of the answer.
#[tokio::test]
async fn roll_one_counts_an_unnameable_strike_as_one_failure() {
    let vendor = Loopback::answering(rolling_unnameable).await;
    let root = Root::new("roll-unnameable");

    let rolled = roll_once(&root.site(), vendor.base, july_expiry()).await;

    assert_eq!(
        rolled,
        Rolled {
            rows_read: 3,
            failed: 1,
            why: vec!["OPTIDX MONTH/1 ATM CALL: this store cannot name that contract".to_owned()],
            ..Rolled::default()
        }
    );
}

/// A run whose bars land but whose overlay cannot be written keeps the bars
/// and counts the overlay's refusal as a failure of that run.
#[tokio::test]
async fn roll_one_keeps_landed_bars_and_counts_an_overlay_refusal() {
    let vendor = Loopback::answering(rolling_plain).await;
    let root = Root::new("roll-overlay");
    let expiry_day = july_expiry();
    let (contract, _, _) =
        name_the_contract(expiry_day, 2_400_000, "CALL", "fixture").expect("a nameable call");
    let overlay = StorePath::new(nifty_parts(
        Vendor::Dhan,
        brutex_core::instrument::Segment::Fno.as_str(),
        Some(contract),
        ym(2025, 7),
        FileKind::Overlay,
    ))
    .expect("an overlay path")
    .to_path_buf(&root.0);
    std::fs::create_dir_all(&overlay).expect("obstruct only the overlay file");

    let rolled = roll_once(&root.site(), vendor.base, expiry_day).await;

    assert_eq!(rolled.rows_read, 3, "{rolled:?}");
    assert_eq!(rolled.stored, 3, "{rolled:?}");
    assert_eq!(rolled.failed, 1, "{rolled:?}");
    assert_eq!(rolled.why.len(), 1, "{rolled:?}");
    assert!(
        rolled.why[0].starts_with("OPTIDX MONTH/1 ATM CALL: "),
        "{rolled:?}"
    );
    assert!(overlay.is_dir(), "the obstruction is untouched");
}

/// The visible part of a rendered receipt, for assertion messages: the
/// page from its first content section on, without the stylesheet.
fn visible(body: &str) -> &str {
    body.find("<section class=\"wrap\">")
        .and_then(|at| body.get(at..))
        .unwrap_or(body)
}

// ------------------------------------------------- rolling walk internals

/// `note_run_failure` counts every failure and keeps the first
/// `REASONS_KEPT` reasons. Kills the mutants at server.rs:12342-12343.
#[test]
fn note_run_failure_counts_all_and_keeps_the_first_reasons() {
    let mut failed = 0usize;
    let mut why = Vec::new();
    for n in 0..7 {
        note_run_failure(&mut failed, &mut why, format!("reason {n}"));
    }
    assert_eq!(failed, 7);
    assert_eq!(
        why,
        ["reason 0", "reason 1", "reason 2", "reason 3", "reason 4"]
    );
}

fn keyed(strike: Option<i64>) -> pull::rolling::Row {
    pull::rolling::Row {
        strike,
        ..rolling_row(session_micros(day(2025, 7, 1), 0), 100, 2_400_000, OI_NULL)
    }
}

/// `next_group` ends a run at the first row with a different key, and at
/// the first row with no key; a first row with no key is the error; a row
/// past the end is named. Kills the `next_group -> Ok(..)` replacements at
/// server.rs:12380 and `!= with ==` at server.rs:12390.
#[test]
fn next_group_splits_runs_on_the_key() {
    let on = day(2025, 7, 31);
    let key_at = |row: &pull::rolling::Row| {
        row.strike
            .map(|strike| (on, strike))
            .ok_or_else(|| "no strike".to_owned())
    };
    let rows = [keyed(Some(1)), keyed(Some(1)), keyed(Some(2))];
    assert_eq!(next_group(&rows, 0, "l", key_at), Ok((2, (on, 1))));
    assert_eq!(next_group(&rows, 2, "l", key_at), Ok((3, (on, 2))));
    assert_eq!(
        next_group(&rows, 3, "l", key_at),
        Err("l: row vanished".to_owned())
    );

    let unnamed_first = [keyed(None), keyed(Some(1))];
    assert_eq!(
        next_group(&unnamed_first, 0, "l", key_at),
        Err("no strike".to_owned())
    );
    let unnamed_later = [keyed(Some(1)), keyed(None), keyed(Some(1))];
    assert_eq!(next_group(&unnamed_later, 0, "l", key_at), Ok((1, (on, 1))));
}

/// `rolling_key` names a row's contract by the expiry its own day resolves
/// to and the strike the vendor sent, and refuses a row with no strike or no
/// calendar day. Kills the `rolling_key -> Ok((Default, n))` replacements at
/// server.rs:13068.
#[test]
fn rolling_key_names_the_expiry_and_strike_or_refuses() {
    let rolling = dhan_rolling();
    assert_eq!(
        rolling_key(
            &keyed(Some(2_400_000)),
            "NIFTY",
            "MONTH",
            "1",
            &rolling,
            "l"
        ),
        Ok((july_expiry(), 2_400_000))
    );
    assert_eq!(
        rolling_key(&keyed(None), "NIFTY", "MONTH", "1", &rolling, "l"),
        Err(
            "l: the vendor sent no strike for a bar, so the contract it resolved to has \
             no name — nothing was filed rather than bars being written under a guessed \
             strike"
                .to_owned()
        )
    );
    let mut lost = keyed(Some(2_400_000));
    lost.bar.ts_micros = i64::MAX;
    assert_eq!(
        rolling_key(&lost, "NIFTY", "MONTH", "1", &rolling, "l"),
        Err("l: a bar stamp outside the calendar".to_owned())
    );
}

/// The plan is the full cross product. Kills `planned_rolling_requests -> 0`
/// and `-> 1` at server.rs:13554.
#[test]
fn planned_rolling_requests_is_the_cross_product() {
    let rolling = dhan_rolling();
    assert_eq!(
        planned_rolling_requests(2, rolling, 3, 4),
        2 * rolling.expiry_codes.len() * 3 * rolling.sides.len() * 4
    );
    assert_eq!(
        rolling.expiry_codes.len(),
        3,
        "the premise: near, next, far"
    );
    assert_eq!(rolling.sides.len(), 2, "the premise: call and put");
}

/// Only a WITHDRAWN cadence is skipped: BANKNIFTY weeklies after their
/// withdrawal are, before it they are not, and a cadence the table cannot
/// answer is still asked. Kills the `true`, `false` and `delete !` mutants
/// at server.rs:13453 and server.rs:13527.
#[test]
fn cadence_has_contracts_skips_only_a_withdrawn_cadence() {
    let rolling = dhan_rolling();
    let ask = |underlying: &str| {
        ingest::parse_fno(
            &format!(
                "underlying={underlying}&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01"
            ),
            day(2025, 8, 2),
        )
        .expect("a rolling request")
    };
    let nifty = ask("NIFTY");
    let banknifty = ask("BANKNIFTY");
    assert!(cadence_has_contracts(&nifty, &rolling, "MONTH"));
    assert!(cadence_has_contracts(&nifty, &rolling, "WEEK"));
    assert!(!cadence_has_contracts(&banknifty, &rolling, "WEEK"));
    assert!(cadence_has_contracts(&banknifty, &rolling, "MONTH"));
    assert!(cadence_has_contracts_on(
        &banknifty,
        &rolling,
        "WEEK",
        day(2024, 11, 4)
    ));
    assert!(cadence_has_contracts(&banknifty, &rolling, "YEAR"));
}

/// The index month a chain is priced against reads back exactly as filed.
/// Kills `read_month_bars -> Ok(vec![..])` at server.rs:12257 and
/// `spot_book_for -> Ok(Default::default())` at server.rs:12319.
#[test]
fn spot_book_for_and_read_month_bars_read_what_was_filed() {
    let root = Root::new("spot-book");
    let site = root.site();
    let wire = wire_at(pull::vendor::Feed::Groww, "http://127.0.0.1:0");
    let on = day(2025, 7, 1);
    let bars = [
        bar_at(session_micros(on, 0), 2_400_000),
        bar_at(session_micros(on, 1), 2_400_500),
    ];
    write_bars(
        &root.0,
        nifty_parts(
            Vendor::Groww,
            brutex_core::instrument::Segment::Index.as_str(),
            None,
            ym(2025, 7),
            FileKind::Bars,
        ),
        &bars,
    );

    let read = read_month_bars(
        &site,
        &wire,
        "NIFTY",
        brutex_core::instrument::Segment::Index.as_str(),
        None,
        Timeframe::MINUTE_1,
        ym(2025, 7),
    )
    .expect("the filed month");
    assert_eq!(read, bars);

    let book = spot_book_for(&site, &wire, "NIFTY", Timeframe::MINUTE_1, ym(2025, 7))
        .expect("the filed month");
    assert_eq!(book.len(), 2);
    assert_eq!(book.at(session_micros(on, 1)), Some(2_400_500));
    assert_eq!(book.at(session_micros(on, 2)), None);
}

/// The rolling walk's three progress events land on the log with the
/// numbers they were given. Kills `say_walk_starting`, `say_group_starting`
/// and `say_walk_finished -> ()` at server.rs:13584-13637.
#[test]
fn the_rolling_progress_events_carry_their_counts() {
    let _shared = crate::emitted::sink();
    let from = crate::emitted::mark();
    let asked = ingest::parse_fno(
        "underlying=NIFTY&series=opt&vendor=dhan&from=2025-07-01&to=2025-07-01",
        day(2025, 8, 2),
    )
    .expect("a rolling request");

    say_walk_starting(&asked, "cloudb-endpoint", 7, 21, 2, 918_273);
    say_group_starting(&asked, "MONTH", "1", 918_274, 3, 4);
    say_walk_finished(&asked, 918_275, 5, 6, 918_273);

    let started = crate::emitted::landed(from, "pull.roll", "walk starting");
    assert!(
        started.iter().any(|record| {
            crate::emitted::says(record, "endpoint", "cloudb-endpoint")
                && crate::emitted::says(record, "underlying", "NIFTY")
                && crate::emitted::counts(record, "chunks", 7)
                && crate::emitted::counts(record, "offsets", 21)
                && crate::emitted::counts(record, "cadences", 2)
                && crate::emitted::counts(record, "planned_requests", 918_273)
        }),
        "{started:?}"
    );
    let group = crate::emitted::landed(from, "pull.roll", "group starting");
    assert!(
        group.iter().any(|record| {
            crate::emitted::says(record, "expiry_flag", "MONTH")
                && crate::emitted::counts(record, "stored_so_far", 918_274)
                && crate::emitted::counts(record, "failed_so_far", 3)
                && crate::emitted::counts(record, "declined_so_far", 4)
        }),
        "{group:?}"
    );
    let finished = crate::emitted::landed(from, "pull.roll", "walk finished");
    assert!(
        finished.iter().any(|record| {
            crate::emitted::counts(record, "stored", 918_275)
                && crate::emitted::counts(record, "failed", 5)
                && crate::emitted::counts(record, "declined", 6)
                && crate::emitted::counts(record, "planned_requests", 918_273)
        }),
        "{finished:?}"
    );
}

// --------------------------------------------------------- route handlers

/// `POST /pull/fno` answers with the detached walk's own status and page.
/// Kills the `pull_fno -> (Default, Html(..))` replacements at
/// server.rs:14104.
#[tokio::test]
async fn pull_fno_answers_with_the_walks_refusal() {
    let root = Root::new("pull-fno-route");
    let site: Loaded = Arc::new(root.site());
    let (status, axum::response::Html(body)) = pull_fno(
        axum::extract::State(site),
        "underlying=NIFTY&series=opt&vendor=nonsense".to_owned(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Expired F&amp;O pull"), "{}", visible(&body));
    assert!(
        body.contains(&render::escape(
            "\"nonsense\" is not a feed this build has a descriptor for"
        )),
        "{}",
        visible(&body)
    );
}

fn audit_record(n: u32) -> audit::Record {
    audit::Record::refused(
        audit::Scope::Spot,
        audit::Outcome::Stored,
        fixture_now(),
        &format!("cloudb-run-{n}"),
        "",
    )
}

/// `GET /audit/page` renders the journal of the site it was given.
/// Kills the `audit_get -> (Default, Html(..))` replacements at
/// server.rs:14224.
#[tokio::test]
async fn audit_get_renders_the_journal() {
    let root = Root::new("audit-get");
    let site = root.site();
    site.journal()
        .append(&audit_record(1))
        .expect("a journal record");
    let (status, axum::response::Html(body)) = audit_get(
        axum::extract::State(Arc::new(site)),
        "/audit/page?page=0".parse().expect("a uri"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("cloudb-run-1"), "{}", visible(&body));
    assert!(body.contains("1 record(s)"), "{}", visible(&body));
}

/// A journal one record longer than a page has exactly two pages: a page
/// past the end clamps to the second, which holds only the oldest record,
/// and a healthy journal carries no UNAVAILABLE note. Kills `/ with %` and
/// `/ with *` at server.rs:14248 and `delete !` at server.rs:14255.
#[test]
fn audit_html_clamps_to_the_last_page_of_a_two_page_journal() {
    let root = Root::new("audit-pages");
    let site = root.site();
    let journal = site.journal();
    let per_page = u32::try_from(PAGE_ROWS).expect("a small page");
    for n in 0..=per_page {
        journal.append(&audit_record(n)).expect("a journal record");
    }
    let today = day(2026, 8, 7);

    let first = audit_html(&site, today, 0);
    let second = audit_html(&site, today, 1);
    let far = audit_html(&site, today, 9_999);

    assert_eq!(far, second, "a page past the end clamps to the last one");
    assert_ne!(first, second);
    assert!(second.contains("cloudb-run-0<"), "{}", visible(&second));
    assert!(!second.contains("cloudb-run-1<"), "{}", visible(&second));
    assert!(!first.contains("cloudb-run-0<"), "{}", visible(&first));
    assert!(!first.contains("UNAVAILABLE"), "{}", visible(&first));
}

/// What the model says of a call premium twenty times its spot.
const MODEL_SUPREMUM: &str = "the model refused this quote: price 48000000 is at or above the \
                              model's supremum 2400000; no volatility produces it";

/// A FEED THAT SERVES EXPIRED DERIVATIVES GOES ON TO DISCOVERY; it is not
/// refused as one that declares none. Groww serves them by name, so the walk
/// asks for the month's expiries. With the site's budget unreadable, that ask
/// is refused by the budget before a request is built (`laddered` charges
/// before it asks, on every attempt), so no socket opens, and the answer is
/// the budget's refusal rather than the "declares no expired-derivative
/// history" arm (kills `delete !` at `!spec.fno.serves()` in `fno_walk`).
#[tokio::test]
async fn a_feed_that_serves_expired_derivatives_goes_on_to_discovery() {
    let root = Root::new("walk-groww-serves");
    let site = scripted_site(&root, "http://127.0.0.1:1", vec![Ok("cloudb".to_owned())]);
    std::thread::scope(|scope| {
        let poisoner = scope.spawn(|| {
            let _budgets = site.budgets.lock().expect("not yet poisoned");
            panic!("poison the budget lock before discovery");
        });
        assert!(poisoner.join().is_err());
    });
    assert!(
        site.budgets.lock().is_err(),
        "the premise: no budget can be read"
    );
    let today = day(2025, 8, 2);
    let (status, body) = walk(&groww_request(today), today, &site).await;
    assert!(
        !body.contains("declares no expired-derivative history"),
        "{}",
        visible(&body)
    );
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{}", visible(&body));
    assert!(
        body.contains(
            "was not reached: the rate budget for Groww cannot be read: another request \
             panicked while holding it, so the allowance already spent is unknown. Nothing is \
             issued on an unknown budget."
        ),
        "{}",
        visible(&body)
    );
}

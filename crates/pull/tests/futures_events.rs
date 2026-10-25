//! R4-28 repair 4 (D-7190; `docs/04-invariants.md` R428-18): EACH
//! `pull.futures` SERIES EVENT COUNTS THE BARS WRITTEN, beside the bars
//! offered, the rows folded and the rows dropped by reason, and carries the
//! receipt's own line for that series, so `/logs` and the receipt cannot
//! disagree about one series.
//!
//! The futures pull used to emit `stored` for the bars it OFFERED the store,
//! so a re-pull that wrote nothing logged every bar as stored. Nothing read
//! the event back: the receipt tests read `FuturesLanded`, and a futures site
//! is not in `emit_sites`. This binary drives the shipped
//! `fnowork::pull_futures_day` and reads the lines off the file the sink
//! wrote.
//!
//! Its own test binary because `telemetry::install` is a process singleton:
//! no sibling test can write a `pull.futures` line into this log.
//!
//! Every candle here is invented: the underlying `ZQIDX`, a token in the eight
//! millions and prices near ₹140, which no vendor publishes for a real future.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Expiry, Kind};
use pull::chain::{Discovery, Refusal};
use pull::fnowork::{FuturesAsk, FuturesLanded, FuturesPlan, FuturesSeries};
use pull::session::{Day, Window};

/// A new, empty folder of this test's own. Left for the main session's
/// clean-up: nothing in this binary deletes a file.
fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "brutex-r428-futures-events-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

/// The vendor, answering every request with `body`.
struct Answer(String);

impl Discovery for Answer {
    async fn get(&self, _url: &str) -> Result<String, Refusal> {
        Ok(self.0.clone())
    }
}

/// Six invented day candles for the window 2023-11-27..30: one before it
/// (11-20), the exchange holiday 2023-11-27, three sessions, and one past the
/// contract's expiry and the window (12-01).
fn six_candles() -> String {
    let candles: Vec<String> = [(11, 20), (11, 27), (11, 28), (11, 29), (11, 30), (12, 1)]
        .iter()
        .zip(140_u32..)
        .map(|(&(m, d), price)| {
            format!(
                "[\"{}T00:00:00+0530\",{price}.5,{}.0,{}.25,{price}.75,1200,{}]",
                day(2023, m, d),
                price + 1,
                price - 1,
                u64::from(price) * 10
            )
        })
        .collect();
    format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        candles.join(",")
    )
}

/// One pull of the November contract over 2023-11-27..30, answered `body`.
fn pull_november(store_root: &Path, body: String) -> FuturesLanded {
    let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
    else {
        panic!("Zerodha is an HTTP feed");
    };
    let expiry = day(2023, 11, 30);
    let plan = FuturesPlan {
        underlying: "ZQIDX".to_owned(),
        newest: pull::master_snapshots::Stamp {
            day: day(2023, 11, 1),
            ordinal: 1,
        },
        asks: vec![FuturesAsk {
            token: 8_400_001,
            series: FuturesSeries::Contract {
                contract: Contract::of(Kind::Future {
                    expiry: Expiry::new(2023, 11, 30).expect("an expiry"),
                })
                .expect("a future renders"),
                expiry,
            },
            window: Window::new(day(2023, 11, 27), expiry).expect("an ordered window"),
        }],
        refused: Vec::new(),
        rebuilt: None,
        incomplete: Vec::new(),
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(pull::fnowork::pull_futures_day(
            &Answer(body),
            &spec,
            &plan,
            store_root,
            brutex_core::vendor::Vendor::Zerodha,
            "test://futures-events",
        ))
}

/// The `pull.futures` lines of the log, oldest first.
fn futures_lines(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|line| line.contains("\"target\":\"pull.futures\""))
        .collect()
}

#[test]
fn each_futures_series_event_counts_bars_written_beside_offered_and_its_drops() {
    let logs = scratch("logs");
    let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
    let sink = telemetry::install(&config).expect("the only install in this binary");
    let store_root = scratch("store");

    let first = pull_november(&store_root, six_candles());
    let again = pull_november(&store_root, six_candles());
    assert_eq!((first.bars_committed, again.bars_committed), (3, 0));
    let empty = pull_november(
        &scratch("store-empty"),
        "{\"status\":\"success\",\"data\":{\"candles\":[]}}".to_owned(),
    );
    assert_eq!(empty.failed.len(), 1, "{:?}", empty.failed);

    sink.sync().expect("the log is durable before it is read");
    let log = std::fs::read_to_string(telemetry::current_path(&logs)).expect("the log");
    let lines = futures_lines(&log);
    assert_eq!(lines.len(), 3, "one line per series pulled: {lines:#?}");

    // THE FIRST PULL: three written, three offered, three dropped by reason.
    let one = lines[0];
    for fragment in [
        "\"level\":\"info\"",
        "\"msg\":\"a futures series landed\"",
        "\"series\":\"ZQIDX 2023-11-30-FUT\"",
        "\"rows\":6",
        "\"offered\":3",
        "\"written\":3",
        "\"folded\":0",
        "\"dropped\":3",
        "before the requested window 1",
        "after the requested window 1",
        "on a day the exchange calendar records closed 1",
        "3 bars written, 3 offered from 6 rows over 2023-11-27..2023-11-30",
    ] {
        assert!(one.contains(fragment), "{fragment} in {one}");
    }
    assert!(
        !one.contains("\"stored\""),
        "no offered count as stored: {one}"
    );

    // THE IDENTICAL RE-PULL: offered again, written never, and the line says
    // they were already held.
    let two = lines[1];
    for fragment in [
        "\"level\":\"info\"",
        "\"offered\":3",
        "\"written\":0",
        "\"dropped\":3",
        "0 bars written, 3 offered (3 already held) from 6 rows",
    ] {
        assert!(two.contains(fragment), "{fragment} in {two}");
    }

    // A SERIES THAT STOPPED is an error line naming why, with nothing written.
    let three = lines[2];
    for fragment in [
        "\"level\":\"error\"",
        "\"msg\":\"a futures series stopped by name\"",
        "\"written\":0",
        "history_ends",
    ] {
        assert!(three.contains(fragment), "{fragment} in {three}");
    }
    // WHOLE LINES: no text cut at its ceiling, and the last two fields kept,
    // so none was counted away past `telemetry::MAX_FIELDS`.
    for line in &lines {
        assert!(!line.contains("\"cut\":true"), "no field cut: {line}");
        assert!(
            line.contains("\"line\":\"ZQIDX 2023-11-30-FUT: ") && line.contains("\"detail\":\""),
            "every field kept: {line}"
        );
    }
    assert_eq!(sink.health().dropped, 0, "nothing dropped by the sink");
}

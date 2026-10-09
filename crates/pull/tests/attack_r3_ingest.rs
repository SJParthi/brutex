//! ATTACK ROUND 3, INGEST: the decoder-skip accounting of D-3122 on the door
//! round 1 did not walk — a local archive's CSV members.
//!
//! D-3122 carried every candle the HTTP decoders skip into
//! `Ingested::decoder_skips` and `rows_read`, so `balances` cannot say every
//! offered candle is on the receipt while one is nowhere. `csv::decode` skips a
//! row whose volume parses negative too, and counted it only into one log
//! event: the member reached `from_members` short, `rows_read` was the decoded
//! count, and the run balanced over a row that is in no column of the receipt.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use brutex_core::vendor::Vendor;
use pull::csv::Columns;
use pull::fetch::BarRequest;
use pull::ingest::Plan;
use pull::session::{Day, Window};
use pull::vendor::{Granularity, Listing, PriceScale, TimestampEncoding};

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-attack-r3-ingest-{tag}-{}",
            std::process::id()
        ));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(dir.join("ARCHIVE")).expect("an archive folder");
        fs::create_dir_all(dir.join("STORE")).expect("a store folder");
        Self(dir)
    }

    fn archive(&self) -> PathBuf {
        self.0.join("ARCHIVE")
    }

    fn store(&self) -> PathBuf {
        self.0.join("STORE")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn request() -> BarRequest {
    BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: Window::new(
            Day::new(2022, 10, 3).expect("day"),
            Day::new(2022, 10, 3).expect("day"),
        )
        .expect("a one-day window"),
        granularity: Granularity::Minute1,
    }
}

fn plan(request: &BarRequest) -> Plan<'_> {
    Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: Columns::TrueDataIndex,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        scale: PriceScale::Paisa,
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// A whole 2022-10-03 session of one-minute rows, volume 7, then `negative`
/// more rows after the close carrying volume `-5`, which the decoder skips
/// before the session filter ever sees them.
fn session(negative: usize) -> String {
    let mut body = String::new();
    for minute in 0..375 + negative {
        let clock = 9 * 60 + 15 + minute;
        let volume = if minute < 375 { 7 } else { -5 };
        writeln!(
            body,
            "20221003,{:02}:{:02}:00,38600.00,{volume},0",
            clock / 60,
            clock % 60
        )
        .expect("a fixture row");
    }
    body
}

fn land(tag: &str, negative: usize) -> pull::ingest::Ingested {
    let scratch = Scratch::new(tag);
    fs::write(scratch.archive().join("NIFTY.csv"), session(negative)).expect("a member");
    let request = request();
    pull::ingest::from_dir(&scratch.archive(), &scratch.store(), plan(&request))
        .expect("the folder and the column shape are fine")
}

/// ROUND 3, D-3125: a CSV row the decoder skips for a negative volume is the
/// vendor's row too. It is in `rows_read`, named under `decoder_skips`, and
/// `balances` accounts for it — exactly as on the HTTP doors since D-3122.
#[test]
fn a_csv_row_the_decoder_skips_is_still_on_the_receipt() {
    let clean = land("clean", 0);
    assert!(clean.failures.is_empty(), "{:?}", clean.failures);
    assert_eq!(clean.rows_read, 375);
    assert_eq!(clean.decoder_skips.total(), 0);
    assert!(clean.balances());

    assert_eq!(session(3).lines().count(), 378, "the archive offers 378");
    let skipped = land("skipped", 3);
    assert!(skipped.failures.is_empty(), "{:?}", skipped.failures);
    assert_eq!(skipped.bars_stored, 375);
    assert_eq!(
        skipped.rows_read, 378,
        "the archive offered 378 rows; three were skipped, not unsent: {skipped:?}"
    );
    assert_eq!(
        skipped.decoder_skips.negative_volume, 3,
        "and they are named by reason"
    );
    assert!(
        skipped.balances(),
        "375 stored + 3 skipped = 378 read: {skipped:?}"
    );
}

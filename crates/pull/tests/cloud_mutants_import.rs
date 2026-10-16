//! Mutant-killing tests for the public items of the GDFL import's readers,
//! the fold, the scope ledger and the scrub (cloud mutation round,
//! 2026-10-16; `reports/cloud-mut-pull-20261016.md`).
//!
//! Each test names the behaviour one or more surviving `cargo mutants`
//! mutants broke and asserts exactly that behaviour, so the mutant fails it.
//! Private items are tested beside their code, in the `cloud_mutant_tests`
//! module at the end of each source file.
//!
//! # Every value here is invented
//!
//! The repository is public and the vendor's licence is UNVERIFIED: no row,
//! price or file of any vendor appears here. Tickers are real index and
//! share names only where the code under test admits nothing else (the swept
//! surface); every price, quantity and stamp is made up.

// The same exceptions every test module in this workspace takes: a test that
// cannot panic cannot fail.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use brutex_core::instrument::{Contract, Exchange, Expiry, Kind, OptionSide, Segment};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey};
use store::path::{Timeframe, YearMonth};

/// A census entry of `contract` with one bar.
fn entry_of(contract: Option<Contract>) -> Entry {
    Entry {
        key: EntryKey {
            contract,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new("NIFTY").unwrap(),
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2031, 3).unwrap(),
        },
        rows: 1,
        first_ts_micros: 1,
        last_ts_micros: 1,
    }
}

// ═══ scrub ════════════════════════════════════════════════════════════════

/// `scrub::contract_of` hands back the entry's own contract, and `None` for
/// spot (kills `-> None` and, where it compiles, `-> Some(Default)`).
#[test]
fn scrub_contract_of_is_the_entrys_own_contract() {
    let contract = Contract::of(Kind::Option {
        expiry: Expiry::new(2031, 3, 27).unwrap(),
        strike: Paisa::from_raw(1_234_500),
        side: OptionSide::Put,
    })
    .unwrap();
    assert_eq!(
        pull::scrub::contract_of(&entry_of(Some(contract))),
        Some(contract)
    );
    assert_eq!(pull::scrub::contract_of(&entry_of(None)), None);
}

// ═══ shared fixtures ══════════════════════════════════════════════════════

/// A scratch directory unique to `tag` and this process, empty.
fn scratch(tag: &str) -> std::path::PathBuf {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "brutex-cloud-mut-{tag}-{}-{serial}",
        std::process::id()
    ));
    let _absent = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The one telemetry sink this binary installs (`telemetry::install` is a
/// process singleton), at `Trace`, and its log directory.
fn sink() -> &'static (std::path::PathBuf, &'static telemetry::Sink) {
    static SINK: std::sync::OnceLock<(std::path::PathBuf, &'static telemetry::Sink)> =
        std::sync::OnceLock::new();
    SINK.get_or_init(|| {
        let logs = scratch("logs");
        let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
        let sink = telemetry::install(&config).expect("the only install in this binary");
        (logs, sink)
    })
}

/// Every log line written so far that holds `marker`, synced first.
fn logged(marker: &str) -> Vec<String> {
    let (logs, sink) = sink();
    sink.sync().unwrap();
    std::fs::read_to_string(telemetry::current_path(logs))
        .unwrap()
        .lines()
        .filter(|line| line.contains(marker))
        .map(str::to_owned)
        .collect()
}

fn day(y: u16, m: u8, d: u8) -> pull::session::Day {
    pull::session::Day::new(y, m, d).unwrap()
}

// ═══ gdfl_cm::auction ═════════════════════════════════════════════════════

mod auction_sidecar {
    use super::{day, logged, scratch, sink};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use pull::gdfl_cm::auction::{self, Print, RECORD_LEN, Record};

    fn print(sod: u32, price: i64, quantity: u64) -> Print {
        Print {
            sod,
            price,
            quantity,
            rows: 1,
            line: 2,
        }
    }

    /// A record is exactly 32 bytes with a zero reserved word: a slice one
    /// byte longer or shorter is no record, whatever its reserved word says.
    /// Kills the length-and-reserved test's `&&` read as `||`.
    #[test]
    fn a_record_is_exactly_thirty_two_bytes() {
        let record = Record {
            t_micros: -5,
            price_paisa: 12_345,
            quantity: 6,
            rows: 7,
        };
        let bytes = record.to_bytes();
        assert_eq!(Record::from_bytes(&bytes), Some(record));
        let mut long = bytes.to_vec();
        long.push(0);
        assert_eq!(Record::from_bytes(&long), None, "33 bytes");
        assert_eq!(
            Record::from_bytes(&bytes[..RECORD_LEN - 1]),
            None,
            "31 bytes"
        );
        assert_eq!(Record::from_bytes(&[]), None, "zero bytes");
        let mut reserved = bytes;
        reserved[31] = 1;
        assert_eq!(Record::from_bytes(&reserved), None, "a reserved bit set");
    }

    /// A print stamped at exactly IST midnight belongs to its own day, never
    /// to the day before: re-offering the earlier day after it is still the
    /// same day's records and writes nothing. Kills the day window's `<`
    /// read as `<=`.
    #[test]
    fn a_print_at_ist_midnight_is_its_own_days_and_not_the_day_befores() {
        let root = scratch("AucMidnight");
        let key = InstrumentKey::cash(Exchange::Nse, "ZZMID").unwrap();
        let (d1, d2) = (day(2031, 3, 4), day(2031, 3, 5));
        let late = [print(15 * 3_600 + 28 * 60 + 7, 50_000, 3)];
        let midnight = [print(0, 50_100, 4)];
        assert_eq!(auction::append(&root, &key, d1, &late).unwrap().written, 1);
        assert_eq!(
            auction::append(&root, &key, d2, &midnight).unwrap().written,
            1
        );
        let again = auction::append(&root, &key, d1, &late).unwrap();
        assert_eq!((again.offered, again.written), (1, 0));
        let records =
            auction::parse(&std::fs::read(auction::path(&root, &key, d1).unwrap()).unwrap())
                .unwrap();
        assert_eq!(
            records,
            vec![Record::of(d1, &late[0]), Record::of(d2, &midnight[0])]
        );
        // 00:00:00 IST of d2 is 18:30:00 UTC of d1.
        assert_eq!(
            records[1].t_micros,
            (i64::from(d1.days_from_epoch()) * 86_400 + 18 * 3_600 + 30 * 60) * 1_000_000
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Every filed day is one `info` line naming the share (an `&` kept as
    /// written), the day, the prints offered and written, and the sidecar;
    /// the idempotent re-offer is a line too, with `written` 0. Kills
    /// `note_filed` read as `()`.
    #[test]
    fn every_filed_day_says_what_it_filed() {
        sink();
        let root = scratch("AucLog");
        let key = InstrumentKey::cash(Exchange::Nse, "Z&Z").unwrap();
        let d = day(2031, 3, 6);
        let prints = [
            print(15 * 3_600 + 28 * 60 + 7, 50_000, 3),
            print(15 * 3_600 + 30 * 60, 50_005, 1),
        ];
        assert_eq!(auction::append(&root, &key, d, &prints).unwrap().written, 2);
        assert_eq!(auction::append(&root, &key, d, &prints).unwrap().written, 0);
        let sidecar = auction::path(&root, &key, d).unwrap();
        let lines = logged(&sidecar.display().to_string());
        assert_eq!(lines.len(), 2, "{lines:#?}");
        for (line, written) in lines.iter().zip(["\"written\":2", "\"written\":0"]) {
            for want in [
                "\"auction prints filed\"",
                "\"gdfl.import\"",
                "\"info\"",
                "\"symbol\":\"Z&Z\"",
                "\"day\":\"2031-03-06\"",
                "\"prints\":2",
                written,
            ] {
                assert!(line.contains(want), "{want} in {line}");
            }
        }
        // Nothing offered: nothing filed, nothing said, no file made.
        let empty = auction::append(&root, &key, day(2031, 3, 7), &[]).unwrap();
        assert_eq!((empty.offered, empty.written), (0, 0));
        assert_eq!(logged("2031-03-07").len(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}

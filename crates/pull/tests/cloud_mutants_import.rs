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

// ═══ scrub: a record that fails its checksum, a month born without them ═══

mod scrub_reads {
    use super::{day, scratch};
    use brutex_core::instrument::{Exchange, Segment};
    use brutex_core::symbol::Symbol;
    use brutex_core::vendor::Vendor;
    use pull::manifest::{Entry, EntryKey};
    use pull::scrub::{self, Finding};
    use store::file::BarFile;
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

    fn path() -> StorePath<'static> {
        StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2031, 3).unwrap(),
            file: FileKind::Bars,
        })
        .unwrap()
    }

    /// A month whose one record no longer matches its block checksum: the
    /// counts agree, so only reading the record can tell, and the scrub says
    /// `Unreadable` with the path and the store's own words — never
    /// `Agrees`, never a `Bounds` built from a value it could not read.
    #[test]
    fn a_record_that_fails_its_checksum_is_unreadable_not_agreement() {
        let root = scratch("ScrubRot");
        // 09:15 IST on 2031-03-03.
        let ts = i64::from(day(2031, 3, 3).days_from_epoch()) * 86_400_000_000 + 13_500_000_000;
        let mut writer = BarFile::open_or_create(&root, path(), 7).unwrap();
        writer
            .append(&[Bar {
                ts_micros: ts,
                open: 101,
                high: 109,
                low: 99,
                close: 105,
                volume: 3,
                open_interest: OI_NULL,
            }])
            .unwrap();
        drop(writer);
        let entry = Entry {
            key: EntryKey {
                contract: None,
                exchange: Exchange::Nse,
                segment: Segment::Index,
                symbol: Symbol::new("NIFTY").unwrap(),
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2031, 3).unwrap(),
            },
            rows: 1,
            first_ts_micros: ts,
            last_ts_micros: ts,
        };
        assert_eq!(scrub::one(&entry, &root, Vendor::Dhan, 7), Finding::Agrees);
        let file = BarFile::open_existing(&root, path(), 7).unwrap();
        let at = usize::try_from(file.layout().offset_of(0).unwrap()).unwrap();
        let bin = file.path().to_path_buf();
        drop(file);
        let mut bytes = std::fs::read(&bin).unwrap();
        bytes[at + 8] ^= 0x5a;
        std::fs::write(&bin, bytes).unwrap();
        let shown = path().to_path_buf(&root).display().to_string();
        match scrub::one(&entry, &root, Vendor::Dhan, 7) {
            Finding::Unreadable { path, why } => {
                assert_eq!(path, shown);
                assert!(!why.is_empty());
            }
            other => panic!("a rotted record read as {other:?}"),
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A month born without block checksums (a version-2 header, written
    /// here by hand as every build before the writer's fix wrote one) has
    /// nothing to verify against: the block pass says so, by the file's
    /// name, and checks nothing.
    #[test]
    fn a_month_born_without_checksums_is_unverifiable_by_the_block_pass() {
        let root = scratch("ScrubUnsealed");
        let bin = path().to_path_buf(&root);
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        let layout = store::layout::Layout::V2;
        let genesis = store::header::Header::genesis_at(layout, 7, 60, 0);
        let commit = genesis.commit().unwrap();
        let mut bytes = vec![0_u8; usize::try_from(layout.header_len()).unwrap()];
        let at = usize::try_from(commit.offset).unwrap();
        bytes[at..at + commit.bytes.len()].copy_from_slice(&commit.bytes);
        std::fs::write(&bin, bytes).unwrap();
        let file = BarFile::open_existing(&root, path(), 7).unwrap();
        assert!(!file.header().checksums_present(), "the premise");
        let pass = scrub::blocks(&file, 0, u64::MAX);
        assert_eq!((pass.blocks, pass.checked, pass.next), (0, 0, None));
        assert!(pass.bad.is_empty());
        let why = pass.unverifiable.unwrap();
        assert!(
            why.contains("without block checksums") && why.contains(&bin.display().to_string()),
            "{why}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ═══ gdfl_cm::auction: the sidecar's directory and lock refusals ═══════════

mod auction_refusals {
    use super::{day, scratch};
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use pull::gdfl_cm::CmRefusal;
    use pull::gdfl_cm::auction::{self, Print};

    const PRINT: Print = Print {
        sod: 15 * 3_600 + 28 * 60 + 7,
        price: 50_000,
        quantity: 3,
        rows: 1,
        line: 2,
    };

    /// A sidecar whose directory cannot be made (a file stands where the
    /// share's directory goes) is refused naming that directory.
    #[test]
    fn a_sidecar_whose_directory_cannot_be_made_is_refused_by_name() {
        let root = scratch("AucNoDir");
        let key = InstrumentKey::cash(Exchange::Nse, "ZZDIR").unwrap();
        let d = day(2031, 3, 4);
        let sidecar = auction::path(&root, &key, d).unwrap();
        let blocker = sidecar.parent().unwrap().parent().unwrap().to_path_buf();
        std::fs::create_dir_all(blocker.parent().unwrap()).unwrap();
        std::fs::write(&blocker, b"in the way").unwrap();
        match auction::append(&root, &key, d, &[PRINT]) {
            Err(CmRefusal::AuctionSidecar { why }) => {
                assert!(why.contains(&blocker.display().to_string()), "{why}");
            }
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A sidecar another writer holds is refused as busy, by name, and is
    /// left byte for byte as it was; once the writer lets go the append
    /// goes through.
    #[test]
    fn a_sidecar_another_writer_holds_is_refused_as_busy() {
        let root = scratch("AucBusy");
        let key = InstrumentKey::cash(Exchange::Nse, "ZZBUSY").unwrap();
        let (d1, d2) = (day(2031, 3, 4), day(2031, 3, 5));
        assert_eq!(
            auction::append(&root, &key, d1, &[PRINT]).unwrap().written,
            1
        );
        let sidecar = auction::path(&root, &key, d1).unwrap();
        let before = std::fs::read(&sidecar).unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&sidecar)
            .unwrap();
        let held = store::flock::Flock::try_lock(file, sidecar.clone()).unwrap();
        match auction::append(&root, &key, d2, &[PRINT]) {
            Err(CmRefusal::AuctionSidecar { why }) => {
                assert!(why.contains("is busy or cannot be locked"), "{why}");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(std::fs::read(&sidecar).unwrap(), before);
        held.release().unwrap();
        assert_eq!(
            auction::append(&root, &key, d2, &[PRINT]).unwrap().written,
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ═══ cash_session_cache: the two public loaders over a warm cache ══════════

mod cash_loaders {
    use super::{day, scratch};
    use std::collections::HashMap;
    use std::io::Write as _;

    const CSV: &str = "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n1363,HINDALCO,EQ,INE038A01020,1\n16921,20MICRONS,EQ,INE144J01027,0\n";

    fn gzip() -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(CSV.as_bytes()).unwrap();
        encoder.finish().unwrap()
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// `prepare` over a window whose every required master is held answers
    /// each, keyed by IST day number, without asking any host; the weekend
    /// inside it owes nothing. Kills `prepare` answering an empty or a
    /// one-entry map.
    #[test]
    fn prepare_over_a_held_window_answers_every_required_day() {
        let root = scratch("CashPrepare");
        let (mon, tue) = (day(2026, 8, 3), day(2026, 8, 4));
        for held in [mon, tue] {
            pull::cash_session_cache::install(&root, held, &gzip()).unwrap();
        }
        let window = pull::session::Window::new(day(2026, 8, 1), tue).unwrap();
        let map = runtime()
            .block_on(pull::cash_session_cache::prepare(&root, window))
            .unwrap();
        assert_eq!(map.len(), 2);
        let flags = map.get(&mon.days_from_epoch()).unwrap();
        assert!(flags.eligibility("HINDALCO", "INE038A01020").unwrap());
        assert!(!flags.eligibility("20MICRONS", "INE144J01027").unwrap());
        assert!(map.contains_key(&tue.days_from_epoch()));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// `prepare_observed` fills the caller's cache with every observed day's
    /// held master, deduplicated, without asking any host. Kills
    /// `prepare_observed` read as `Ok(())`.
    #[test]
    fn prepare_observed_fills_the_cache_from_held_masters() {
        let root = scratch("CashObserved");
        let (mon, tue) = (day(2026, 8, 3), day(2026, 8, 4));
        for held in [mon, tue] {
            pull::cash_session_cache::install(&root, held, &gzip()).unwrap();
        }
        let mut cache = HashMap::new();
        runtime()
            .block_on(pull::cash_session_cache::prepare_observed(
                &root,
                &[tue, mon, tue],
                &mut cache,
            ))
            .unwrap();
        assert_eq!(cache.len(), 2);
        assert!(
            cache
                .get(&tue.days_from_epoch())
                .unwrap()
                .eligibility("HINDALCO", "INE038A01020")
                .unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

// ═══ fold ═════════════════════════════════════════════════════════════════

/// The anchor refusal names the stamp it could not place and why it was
/// refused rather than saturated.
#[test]
fn fold_an_anchor_overflow_names_its_stamp() {
    let said = pull::fold::FoldError::AnchorOverflow { ts_micros: -42 }.to_string();
    assert!(
        said.starts_with("the snapshot stamped -42 cannot be shifted onto the IST day grid")
            && said.contains("Refused rather than saturated"),
        "{said}"
    );
}

// ═══ gdfl_tickstore: a socket where a day file belongs ═════════════════════

/// A socket at a day file's name is refused as not a regular file, by
/// name, never opened and never taken for an absent day.
#[test]
fn tickstore_a_socket_at_a_day_name_is_refused_by_name() {
    use pull::gdfl_cm::{CmKind, CmRefusal, CmSource as _};
    let root = scratch("TickSocket");
    let store = pull::gdfl_tickstore::TickStore::new(&root);
    let d = day(2024, 4, 1);
    let path = store.day_path(CmKind::Indices, d);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    match store.day(CmKind::Indices, d) {
        Err(CmRefusal::TickStoreMalformed { what }) => {
            assert!(what.contains("a socket or a device"), "{what}");
        }
        other => panic!("{other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}
